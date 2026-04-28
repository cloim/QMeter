use std::time::Duration;

use qmeter_core::types::{NormalizedError, NormalizedErrorType, ProviderId};

use crate::claude_usage::{clean_claude_screen_text, parse_claude_usage_from_screen};
use crate::provider::{AcquireContext, Provider, ProviderResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaudeProviderConfig {
    pub bash_command: String,
    pub timeout: Duration,
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
                        "Rust Claude PTY capture is not available yet; use the legacy Node CLI for Claude live data"
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
        self.acquire_with_runner(&UnsupportedClaudeScreenRunner, ctx)
    }
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
    if lowered.contains("not found")
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

