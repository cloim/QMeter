use chrono::{DateTime, SecondsFormat, Utc};
use qmeter_core::types::{
    Confidence, NormalizedError, NormalizedErrorType, NormalizedRow, ProviderId, SourceKind,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
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
    pub codex_command: String,
    pub timeout: Duration,
}

impl Default for CodexProviderConfig {
    fn default() -> Self {
        Self {
            codex_command: std::env::var("USAGE_STATUS_CODEX_COMMAND")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "codex".to_string()),
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

    pub fn acquire_with_runner(
        &self,
        runner: &dyn AppServerRunner,
        ctx: AcquireContext,
    ) -> ProviderResult {
        match runner.exchange(
            &self.config.codex_command,
            &app_server_requests(),
            self.config.timeout,
        ) {
            Ok(value) => match parse_rate_limits_response(value) {
                Ok(parsed) => ProviderResult {
                    rows: parsed.rows,
                    errors: Vec::new(),
                    debug: ctx.debug.then(|| {
                        json!({
                            "spawnCommand": app_server_spawn_command(&self.config.codex_command),
                            "limitId": parsed.debug.limit_id,
                            "limitName": parsed.debug.limit_name,
                            "planType": parsed.debug.plan_type,
                            "hadRateLimitsByLimitId": parsed.debug.had_rate_limits_by_limit_id
                        })
                    }),
                },
                Err(err) => ProviderResult {
                    rows: Vec::new(),
                    errors: vec![normalized_error(
                        NormalizedErrorType::InvalidResponse,
                        err.to_string(),
                    )],
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
        self.acquire_with_runner(&ProcessAppServerRunner, ctx)
    }
}

pub trait AppServerRunner {
    fn exchange(
        &self,
        command: &str,
        requests: &[Value],
        timeout: Duration,
    ) -> Result<Value, String>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessAppServerRunner;

impl AppServerRunner for ProcessAppServerRunner {
    fn exchange(
        &self,
        command: &str,
        requests: &[Value],
        timeout: Duration,
    ) -> Result<Value, String> {
        let mut child = spawn_app_server(command).map_err(|err| err.to_string())?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "codex app-server stdin unavailable".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "codex app-server stdout unavailable".to_string())?;

        for request in requests {
            writeln!(stdin, "{request}").map_err(|err| err.to_string())?;
        }
        stdin.flush().map_err(|err| err.to_string())?;

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = read_rate_limit_result(stdout);
            let _ = tx.send(result);
        });

        let result = rx
            .recv_timeout(timeout)
            .map_err(|_| format!("codex app-server timed out after {}ms", timeout.as_millis()));

        let _ = child.kill();
        let _ = child.wait();

        result?
    }
}

fn spawn_app_server(command: &str) -> std::io::Result<std::process::Child> {
    let command = if cfg!(windows) && command == "codex" {
        "codex.cmd"
    } else {
        command
    };

    Command::new(command)
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

fn read_rate_limit_result(stdout: std::process::ChildStdout) -> Result<Value, String> {
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|err| err.to_string())?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("id") == Some(&json!(1)) && value.get("error").is_some() {
            return Err(format!(
                "codex initialize failed: {}",
                value["error"]["message"].as_str().unwrap_or("unknown error")
            ));
        }
        if value.get("id") == Some(&json!(2)) {
            if value.get("error").is_some() {
                return Err(format!(
                    "codex account/rateLimits/read failed: {}",
                    value["error"]["message"].as_str().unwrap_or("unknown error")
                ));
            }
            return value
                .get("result")
                .cloned()
                .ok_or_else(|| "codex account/rateLimits/read missing result".to_string());
        }
    }

    Err("codex app-server exited before rate limits response".to_string())
}

fn app_server_requests() -> [Value; 3] {
    [
        json!({
            "method": "initialize",
            "id": 1,
            "params": {
                "clientInfo": {
                    "name": "qmeter",
                    "title": "QMeter",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }),
        json!({ "method": "initialized", "params": {} }),
        json!({ "method": "account/rateLimits/read", "id": 2 }),
    ]
}

fn app_server_spawn_command(command: &str) -> String {
    let command = if cfg!(windows) && command == "codex" {
        "codex.cmd"
    } else {
        command
    };
    format!("{command} app-server")
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
    {
        NormalizedErrorType::NotInstalled
    } else if lowered.contains("timed out") || lowered.contains("timeout") {
        NormalizedErrorType::Timeout
    } else if lowered.contains("unauthorized") || lowered.contains("forbidden") {
        NormalizedErrorType::AuthRequired
    } else {
        NormalizedErrorType::AcquireFailed
    }
}

pub fn parse_rate_limits_response(value: Value) -> Result<CodexRateLimitsResult, serde_json::Error> {
    let parsed: GetAccountRateLimitsResponse = serde_json::from_value(value)?;
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
