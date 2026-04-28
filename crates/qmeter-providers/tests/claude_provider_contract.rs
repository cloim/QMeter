use std::time::Duration;

use qmeter_core::types::{NormalizedErrorType, SourceKind};
use qmeter_providers::claude::{
    ClaudeProvider, ClaudeProviderConfig, ClaudeScreenRunner, ClaudeUsageApiClient,
};
use qmeter_providers::provider::AcquireContext;

struct FakeClaudeRunner {
    result: Result<String, String>,
}

impl ClaudeScreenRunner for FakeClaudeRunner {
    fn capture_usage_screen(&self, _config: &ClaudeProviderConfig) -> Result<String, String> {
        self.result.clone()
    }
}

struct FakeClaudeUsageClient {
    result: Result<String, String>,
}

impl ClaudeUsageApiClient for FakeClaudeUsageClient {
    fn fetch_usage_json(&self, _config: &ClaudeProviderConfig) -> Result<String, String> {
        self.result.clone()
    }
}

fn provider() -> ClaudeProvider {
    ClaudeProvider::new(ClaudeProviderConfig {
        bash_command: "bash-test".to_string(),
        timeout: Duration::from_secs(1),
        api_url: "https://example.test/oauth/usage".to_string(),
        user_agent: "qmeter-test".to_string(),
    })
}

#[test]
fn claude_provider_parses_oauth_usage_response() {
    let client = FakeClaudeUsageClient {
        result: Ok(r#"{
                "five_hour": { "utilization": 42.4, "resets_at": "2026-04-28T12:00:00Z" },
                "seven_day": { "utilization": 7.8, "resets_at": "2026-05-05T12:00:00Z" },
                "seven_day_sonnet": { "utilization": 3.2, "resets_at": null }
            }"#
        .to_string()),
    };

    let result = provider().acquire_with_usage_client(
        &client,
        AcquireContext {
            refresh: true,
            debug: true,
        },
    );

    assert_eq!(result.errors, vec![]);
    assert_eq!(result.rows.len(), 3);
    assert_eq!(result.rows[0].window, "claude:5h");
    assert_eq!(result.rows[0].used_percent, Some(42.4));
    assert_eq!(
        result.rows[0].reset_at.as_deref(),
        Some("2026-04-28T12:00:00Z")
    );
    assert_eq!(result.rows[0].source, SourceKind::Structured);
    assert_eq!(result.rows[1].window, "claude:7d");
    assert_eq!(result.rows[2].window, "claude:7d-sonnet");
    assert!(result.debug.is_some());
}

#[test]
fn claude_provider_maps_oauth_client_error_to_auth_required() {
    let client = FakeClaudeUsageClient {
        result: Err("Claude OAuth credentials not found".to_string()),
    };

    let result = provider().acquire_with_usage_client(
        &client,
        AcquireContext {
            refresh: true,
            debug: false,
        },
    );

    assert_eq!(result.rows, vec![]);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(
        result.errors[0].error_type,
        NormalizedErrorType::AuthRequired
    );
}

#[test]
fn claude_provider_parses_runner_screen() {
    let runner = FakeClaudeRunner {
        result: Ok([
            "Settings: Usage",
            "Current session",
            "  90% used",
            "  Resets 3am",
            "",
            "Current week (all models)",
            "  21% used",
            "  Resets Feb 28, 10am",
        ]
        .join("\n")),
    };

    let result = provider().acquire_with_runner(
        &runner,
        AcquireContext {
            refresh: true,
            debug: true,
        },
    );

    assert_eq!(result.errors, vec![]);
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0].window, "claude:session");
    assert_eq!(result.rows[0].used_percent, Some(90.0));
    assert!(result.debug.is_some());
}

#[test]
fn claude_provider_maps_runner_timeout() {
    let runner = FakeClaudeRunner {
        result: Err("claude /usage timed out after 25000ms".to_string()),
    };

    let result = provider().acquire_with_runner(
        &runner,
        AcquireContext {
            refresh: true,
            debug: false,
        },
    );

    assert_eq!(result.rows, vec![]);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].error_type, NormalizedErrorType::Timeout);
}
