use chrono::{DateTime, SecondsFormat, Utc};
use qmeter_core::types::{
    Confidence, NormalizedError, NormalizedErrorType, NormalizedRow, ProviderId, SourceKind,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::provider::{AcquireContext, Provider, ProviderResult};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitWindow {
    used_percent: i64,
    window_duration_mins: Option<i64>,
    resets_at: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitSnapshot {
    limit_id: Option<String>,
    limit_name: Option<String>,
    plan_type: Option<String>,
    primary: Option<RateLimitWindow>,
    secondary: Option<RateLimitWindow>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetAccountRateLimitsResponse {
    rate_limits: RateLimitSnapshot,
    rate_limits_by_limit_id: Option<std::collections::BTreeMap<String, RateLimitSnapshot>>,
}

#[derive(Clone, Debug, Deserialize)]
struct BackendUsagePayload {
    plan_type: Option<String>,
    rate_limit: Option<BackendRateLimitStatus>,
    additional_rate_limits: Option<Vec<BackendAdditionalRateLimit>>,
}

#[derive(Clone, Debug, Deserialize)]
struct BackendAdditionalRateLimit {
    limit_name: String,
    metered_feature: String,
    rate_limit: Option<BackendRateLimitStatus>,
}

#[derive(Clone, Debug, Deserialize)]
struct BackendRateLimitStatus {
    primary_window: Option<BackendRateLimitWindow>,
    secondary_window: Option<BackendRateLimitWindow>,
}

#[derive(Clone, Debug, Deserialize)]
struct BackendRateLimitWindow {
    used_percent: i64,
    limit_window_seconds: Option<i64>,
    reset_at: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
struct CodexAuthFile {
    tokens: Option<CodexAuthTokens>,
}

#[derive(Clone, Debug, Deserialize)]
struct CodexAuthTokens {
    access_token: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexRateLimitDebug {
    pub limit_id: Option<String>,
    pub limit_name: Option<String>,
    pub plan_type: Option<String>,
    pub had_rate_limits_by_limit_id: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodexRateLimitsResult {
    pub rows: Vec<NormalizedRow>,
    pub debug: CodexRateLimitDebug,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexProviderConfig {
    pub auth_path: PathBuf,
    pub base_url: String,
    pub timeout: Duration,
}

impl Default for CodexProviderConfig {
    fn default() -> Self {
        Self {
            auth_path: codex_auth_path(),
            base_url: std::env::var("USAGE_STATUS_CODEX_BASE_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "https://chatgpt.com/backend-api".to_string()),
            timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CodexProvider {
    config: CodexProviderConfig,
}

impl CodexProvider {
    pub fn new(config: CodexProviderConfig) -> Self {
        Self { config }
    }

    pub fn acquire_with_client(
        &self,
        client: &dyn UsageApiClient,
        ctx: AcquireContext,
    ) -> ProviderResult {
        let token = match read_codex_access_token(&self.config.auth_path) {
            Ok(token) => token,
            Err(message) => {
                return ProviderResult {
                    rows: Vec::new(),
                    errors: vec![normalized_error(error_type_for_message(&message), message)],
                    debug: None,
                };
            }
        };

        match client.fetch_usage_json(&self.config.base_url, &token) {
            Ok(raw) => match serde_json::from_str::<Value>(&raw)
                .map_err(|err| err.to_string())
                .and_then(|value| parse_rate_limits_response(value).map_err(|err| err.to_string()))
            {
                Ok(parsed) => ProviderResult {
                    rows: parsed.rows,
                    errors: Vec::new(),
                    debug: ctx.debug.then(|| {
                        json!({
                            "apiUrl": usage_url(&self.config.base_url),
                            "source": "oauth-usage",
                            "limitId": parsed.debug.limit_id,
                            "limitName": parsed.debug.limit_name,
                            "planType": parsed.debug.plan_type,
                            "hadRateLimitsByLimitId": parsed.debug.had_rate_limits_by_limit_id
                        })
                    }),
                },
                Err(err) => ProviderResult {
                    rows: Vec::new(),
                    errors: vec![normalized_error(NormalizedErrorType::InvalidResponse, err)],
                    debug: None,
                },
            },
            Err(message) => ProviderResult {
                rows: Vec::new(),
                errors: vec![normalized_error(error_type_for_message(&message), message)],
                debug: None,
            },
        }
    }
}

impl Provider for CodexProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Codex
    }

    fn acquire(&self, ctx: AcquireContext) -> ProviderResult {
        self.acquire_with_client(&HttpUsageApiClient::new(self.config.timeout), ctx)
    }
}

pub trait UsageApiClient {
    fn fetch_usage_json(&self, base_url: &str, access_token: &str) -> Result<String, String>;
}

#[derive(Clone, Debug)]
pub struct HttpUsageApiClient {
    timeout: Duration,
}

impl HttpUsageApiClient {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl UsageApiClient for HttpUsageApiClient {
    fn fetch_usage_json(&self, base_url: &str, access_token: &str) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|err| format!("failed to build Codex usage HTTP client: {err}"))?;
        let url = usage_url(base_url);
        let response = client
            .get(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                reqwest::header::USER_AGENT,
                format!("qmeter/{}", env!("CARGO_PKG_VERSION")),
            )
            .bearer_auth(access_token)
            .send()
            .map_err(|err| format!("Codex usage API request failed: {err}"))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(format!(
                "Codex usage API returned HTTP {}: {body}",
                status.as_u16()
            ));
        }

        response
            .text()
            .map_err(|err| format!("failed to read Codex usage API response: {err}"))
    }
}

fn usage_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.contains("/backend-api") {
        format!("{base}/wham/usage")
    } else {
        format!("{base}/api/codex/usage")
    }
}

fn read_codex_access_token(path: &PathBuf) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read Codex auth file {}: {err}", path.display()))?;
    let auth: CodexAuthFile = serde_json::from_str(&raw)
        .map_err(|err| format!("failed to parse Codex auth file {}: {err}", path.display()))?;
    auth.tokens
        .map(|tokens| tokens.access_token)
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| "Codex ChatGPT OAuth token not found; run `codex login`".to_string())
}

fn codex_auth_path() -> PathBuf {
    if let Some(path) = std::env::var_os("USAGE_STATUS_CODEX_AUTH_PATH") {
        let path = PathBuf::from(path);
        if !path.as_os_str().is_empty() {
            return path;
        }
    }
    if let Some(path) = std::env::var_os("CODEX_HOME") {
        let path = PathBuf::from(path);
        if !path.as_os_str().is_empty() {
            return path.join("auth.json");
        }
    }
    if let Some(path) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(path).join(".codex").join("auth.json");
    }
    PathBuf::from(".codex").join("auth.json")
}

fn normalized_error(error_type: NormalizedErrorType, message: String) -> NormalizedError {
    NormalizedError {
        provider: ProviderId::Codex,
        error_type,
        message,
        actionable: Some("run `codex` once and ensure you are logged in".to_string()),
    }
}

fn error_type_for_message(message: &str) -> NormalizedErrorType {
    let lowered = message.to_ascii_lowercase();
    if lowered.contains("not found")
        || lowered.contains("enoent")
        || lowered.contains("cannot find")
        || lowered.contains("os error 2")
        || lowered.contains("failed to read codex auth file")
    {
        NormalizedErrorType::NotInstalled
    } else if lowered.contains("timed out") || lowered.contains("timeout") {
        NormalizedErrorType::Timeout
    } else if lowered.contains("unauthorized")
        || lowered.contains("forbidden")
        || lowered.contains("oauth token not found")
        || lowered.contains("http 401")
        || lowered.contains("http 403")
    {
        NormalizedErrorType::AuthRequired
    } else {
        NormalizedErrorType::AcquireFailed
    }
}

pub fn parse_rate_limits_response(
    value: Value,
) -> Result<CodexRateLimitsResult, serde_json::Error> {
    let parsed: GetAccountRateLimitsResponse = match serde_json::from_value(value.clone()) {
        Ok(parsed) => parsed,
        Err(_) => backend_usage_payload_to_response(serde_json::from_value(value)?),
    };
    let by_limit_id = parsed.rate_limits_by_limit_id;
    let had_rate_limits_by_limit_id = by_limit_id.is_some();
    let snapshot = by_limit_id
        .and_then(|items| items.get("codex").cloned())
        .unwrap_or(parsed.rate_limits);

    Ok(CodexRateLimitsResult {
        rows: snapshot_to_rows(&snapshot),
        debug: CodexRateLimitDebug {
            limit_id: snapshot.limit_id,
            limit_name: snapshot.limit_name,
            plan_type: snapshot.plan_type,
            had_rate_limits_by_limit_id,
        },
    })
}

fn backend_usage_payload_to_response(payload: BackendUsagePayload) -> GetAccountRateLimitsResponse {
    let mut by_limit_id = std::collections::BTreeMap::new();
    let primary = backend_snapshot("codex", None, payload.rate_limit, payload.plan_type.clone());
    by_limit_id.insert("codex".to_string(), primary.clone());

    if let Some(additional) = payload.additional_rate_limits {
        for item in additional {
            by_limit_id.insert(
                item.metered_feature.clone(),
                backend_snapshot(
                    &item.metered_feature,
                    Some(item.limit_name),
                    item.rate_limit,
                    payload.plan_type.clone(),
                ),
            );
        }
    }

    GetAccountRateLimitsResponse {
        rate_limits: primary,
        rate_limits_by_limit_id: Some(by_limit_id),
    }
}

fn backend_snapshot(
    limit_id: &str,
    limit_name: Option<String>,
    rate_limit: Option<BackendRateLimitStatus>,
    plan_type: Option<String>,
) -> RateLimitSnapshot {
    RateLimitSnapshot {
        limit_id: Some(limit_id.to_string()),
        limit_name,
        plan_type,
        primary: rate_limit
            .as_ref()
            .and_then(|rate_limit| backend_window(rate_limit.primary_window.as_ref())),
        secondary: rate_limit
            .as_ref()
            .and_then(|rate_limit| backend_window(rate_limit.secondary_window.as_ref())),
    }
}

fn backend_window(window: Option<&BackendRateLimitWindow>) -> Option<RateLimitWindow> {
    let window = window?;
    Some(RateLimitWindow {
        used_percent: window.used_percent,
        window_duration_mins: window.limit_window_seconds.map(|seconds| seconds / 60),
        resets_at: window.reset_at,
    })
}

fn snapshot_to_rows(snapshot: &RateLimitSnapshot) -> Vec<NormalizedRow> {
    let mut rows = Vec::new();
    push_window(&mut rows, snapshot.primary.as_ref());
    push_window(&mut rows, snapshot.secondary.as_ref());
    rows
}

fn push_window(rows: &mut Vec<NormalizedRow>, window: Option<&RateLimitWindow>) {
    let Some(window) = window else {
        return;
    };

    rows.push(NormalizedRow {
        provider: ProviderId::Codex,
        window: format!("codex:{}", format_window(window.window_duration_mins)),
        used: None,
        limit: None,
        used_percent: Some(window.used_percent as f64),
        reset_at: to_iso_from_epoch_seconds(window.resets_at),
        source: SourceKind::Structured,
        confidence: Confidence::High,
        stale: false,
        notes: None,
    });
}

fn format_window(minutes: Option<i64>) -> String {
    let Some(minutes) = minutes else {
        return "unknown".to_string();
    };
    if minutes <= 0 {
        return "unknown".to_string();
    }
    if (295..=305).contains(&minutes) {
        return "5h".to_string();
    }
    if (10000..=10100).contains(&minutes) {
        return "weekly".to_string();
    }
    if minutes % (60 * 24) == 0 {
        return format!("{}d", minutes / (60 * 24));
    }
    if minutes % 60 == 0 {
        return format!("{}h", minutes / 60);
    }
    format!("{minutes}m")
}

fn to_iso_from_epoch_seconds(epoch_seconds: Option<i64>) -> Option<String> {
    let seconds = epoch_seconds?;
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
}
