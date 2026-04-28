use std::fs;
use std::process::Command;
use std::time::Duration;

use qmeter_core::types::{
    Confidence, NormalizedError, NormalizedErrorType, NormalizedRow, ProviderId, SourceKind,
};
use serde::Deserialize;

use crate::claude_usage::{clean_claude_screen_text, parse_claude_usage_from_screen};
use crate::provider::{AcquireContext, Provider, ProviderResult};

const CLAUDE_USAGE_API_URL: &str = "https://api.anthropic.com/api/oauth/usage";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaudeProviderConfig {
    pub bash_command: String,
    pub timeout: Duration,
    pub api_url: String,
    pub user_agent: String,
}

impl Default for ClaudeProviderConfig {
    fn default() -> Self {
        Self {
            bash_command: std::env::var("USAGE_STATUS_BASH_EXE")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| {
                    if cfg!(windows) {
                        "C:/Program Files/Git/usr/bin/bash.exe".to_string()
                    } else {
                        "bash".to_string()
                    }
                }),
            timeout: Duration::from_secs(25),
            api_url: CLAUDE_USAGE_API_URL.to_string(),
            user_agent: format!("qmeter/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ClaudeProvider {
    config: ClaudeProviderConfig,
}

impl ClaudeProvider {
    pub fn new(config: ClaudeProviderConfig) -> Self {
        Self { config }
    }

    pub fn acquire_with_runner(
        &self,
        runner: &dyn ClaudeScreenRunner,
        ctx: AcquireContext,
    ) -> ProviderResult {
        match runner.capture_usage_screen(&self.config) {
            Ok(raw) => {
                let clean = clean_claude_screen_text(&raw);
                let parsed = parse_claude_usage_from_screen(&clean);
                ProviderResult {
                    rows: parsed.rows,
                    errors: parsed.errors,
                    debug: ctx.debug.then(|| {
                        serde_json::json!({
                            "bashCommand": self.config.bash_command,
                            "timeoutMs": self.config.timeout.as_millis(),
                            "extractedChars": clean.len()
                        })
                    }),
                }
            }
            Err(message) => ProviderResult {
                rows: Vec::new(),
                errors: vec![NormalizedError {
                    provider: ProviderId::Claude,
                    error_type: error_type_for_message(&message),
                    message,
                    actionable: Some(
                        "Rust Claude PTY capture is not available; use the default OAuth usage provider for Claude live data"
                            .to_string(),
                    ),
                }],
                debug: None,
            },
        }
    }

    pub fn acquire_with_usage_client(
        &self,
        client: &dyn ClaudeUsageApiClient,
        ctx: AcquireContext,
    ) -> ProviderResult {
        match client.fetch_usage_json(&self.config) {
            Ok(raw) => match parse_oauth_usage_response(&raw) {
                Ok(rows) => ProviderResult {
                    rows,
                    errors: Vec::new(),
                    debug: ctx.debug.then(|| {
                        serde_json::json!({
                            "apiUrl": self.config.api_url,
                            "source": "oauth-usage",
                            "responseBytes": raw.len()
                        })
                    }),
                },
                Err(message) => ProviderResult {
                    rows: Vec::new(),
                    errors: vec![NormalizedError {
                        provider: ProviderId::Claude,
                        error_type: NormalizedErrorType::InvalidResponse,
                        message,
                        actionable: Some(
                            "refresh Claude Code login and retry QMeter Claude usage".to_string(),
                        ),
                    }],
                    debug: None,
                },
            },
            Err(message) => ProviderResult {
                rows: Vec::new(),
                errors: vec![NormalizedError {
                    provider: ProviderId::Claude,
                    error_type: error_type_for_message(&message),
                    message,
                    actionable: Some(
                        "run `claude` and complete login, then retry QMeter Claude usage"
                            .to_string(),
                    ),
                }],
                debug: None,
            },
        }
    }
}

impl Provider for ClaudeProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Claude
    }

    fn acquire(&self, ctx: AcquireContext) -> ProviderResult {
        self.acquire_with_usage_client(&HttpClaudeUsageApiClient, ctx)
    }
}

pub trait ClaudeUsageApiClient {
    fn fetch_usage_json(&self, config: &ClaudeProviderConfig) -> Result<String, String>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HttpClaudeUsageApiClient;

impl ClaudeUsageApiClient for HttpClaudeUsageApiClient {
    fn fetch_usage_json(&self, config: &ClaudeProviderConfig) -> Result<String, String> {
        let token = load_claude_oauth_token()
            .ok_or_else(|| "Claude OAuth credentials not found".to_string())?;

        let client = reqwest::blocking::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|err| format!("failed to build Claude usage HTTP client: {err}"))?;

        let response = client
            .get(&config.api_url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::USER_AGENT, &config.user_agent)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .header("anthropic-beta", "oauth-2025-04-20")
            .send()
            .map_err(|err| format!("Claude usage API request failed: {err}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "Claude usage API returned HTTP {}",
                status.as_u16()
            ));
        }

        response
            .text()
            .map_err(|err| format!("failed to read Claude usage API response: {err}"))
    }
}

#[derive(Debug, Deserialize)]
struct ClaudeCredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<ClaudeOauthCredentials>,
}

#[derive(Debug, Deserialize)]
struct ClaudeOauthCredentials {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
}

fn load_claude_oauth_token() -> Option<String> {
    if cfg!(target_os = "macos") {
        load_claude_oauth_token_from_keychain().or_else(load_claude_oauth_token_from_file)
    } else {
        load_claude_oauth_token_from_file()
    }
}

fn load_claude_oauth_token_from_file() -> Option<String> {
    let path = home_dir()?.join(".claude").join(".credentials.json");
    let raw = fs::read_to_string(path).ok()?;
    parse_oauth_token(&raw)
}

fn load_claude_oauth_token_from_keychain() -> Option<String> {
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8(output.stdout).ok()?;
    parse_oauth_token(raw.trim())
}

fn parse_oauth_token(raw: &str) -> Option<String> {
    let creds: ClaudeCredentialsFile = serde_json::from_str(raw).ok()?;
    creds
        .claude_ai_oauth?
        .access_token
        .filter(|token| !token.trim().is_empty())
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
}

pub trait ClaudeScreenRunner {
    fn capture_usage_screen(&self, config: &ClaudeProviderConfig) -> Result<String, String>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnsupportedClaudeScreenRunner;

impl ClaudeScreenRunner for UnsupportedClaudeScreenRunner {
    fn capture_usage_screen(&self, _config: &ClaudeProviderConfig) -> Result<String, String> {
        Err("Rust Claude PTY capture is not implemented".to_string())
    }
}

fn error_type_for_message(message: &str) -> NormalizedErrorType {
    let lowered = message.to_ascii_lowercase();
    if lowered.contains("credential")
        || lowered.contains("oauth")
        || lowered.contains("401")
        || lowered.contains("403")
    {
        NormalizedErrorType::AuthRequired
    } else if lowered.contains("not found")
        || lowered.contains("enoent")
        || lowered.contains("cannot find")
        || lowered.contains("os error 2")
    {
        NormalizedErrorType::NotInstalled
    } else if lowered.contains("timed out") || lowered.contains("timeout") {
        NormalizedErrorType::Timeout
    } else if lowered.contains("tty") || lowered.contains("pty") {
        NormalizedErrorType::TtyUnavailable
    } else {
        NormalizedErrorType::AcquireFailed
    }
}

#[derive(Debug, Deserialize)]
struct ClaudeUsageResponse {
    five_hour: Option<ClaudeUsageWindow>,
    seven_day: Option<ClaudeUsageWindow>,
    seven_day_sonnet: Option<ClaudeUsageWindow>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsageWindow {
    utilization: f64,
    resets_at: Option<String>,
}

fn parse_oauth_usage_response(raw: &str) -> Result<Vec<NormalizedRow>, String> {
    let response: ClaudeUsageResponse = serde_json::from_str(raw)
        .map_err(|err| format!("failed to parse Claude usage API response: {err}"))?;

    let mut rows = Vec::new();
    push_usage_row(&mut rows, "claude:5h", response.five_hour);
    push_usage_row(&mut rows, "claude:7d", response.seven_day);
    push_usage_row(&mut rows, "claude:7d-sonnet", response.seven_day_sonnet);

    if rows.is_empty() {
        return Err("Claude usage API response did not include usage windows".to_string());
    }

    Ok(rows)
}

fn push_usage_row(rows: &mut Vec<NormalizedRow>, window: &str, limit: Option<ClaudeUsageWindow>) {
    let Some(limit) = limit else {
        return;
    };
    rows.push(NormalizedRow {
        provider: ProviderId::Claude,
        window: window.to_string(),
        used: None,
        limit: None,
        used_percent: Some(limit.utilization),
        reset_at: limit.resets_at,
        source: SourceKind::Structured,
        confidence: Confidence::High,
        stale: false,
        notes: None,
    });
}
