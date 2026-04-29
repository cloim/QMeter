use std::cell::RefCell;
use std::fs;
use std::time::Duration;

use qmeter_core::types::NormalizedErrorType;
use qmeter_providers::codex::{
    CodexProvider, CodexProviderConfig, UsageApiClient, parse_rate_limits_response,
};
use qmeter_providers::provider::AcquireContext;

struct FakeUsageClient {
    calls: RefCell<Vec<(String, String)>>,
    result: Result<String, String>,
}

impl UsageApiClient for FakeUsageClient {
    fn fetch_usage_json(&self, base_url: &str, access_token: &str) -> Result<String, String> {
        self.calls
            .borrow_mut()
            .push((base_url.to_string(), access_token.to_string()));
        self.result.clone()
    }
}

#[test]
fn parses_codex_rate_limits_by_limit_id_when_available() {
    let payload = serde_json::json!({
        "rateLimits": {
            "limitId": "fallback",
            "limitName": "Fallback",
            "planType": "pro",
            "primary": { "usedPercent": 11, "windowDurationMins": 300, "resetsAt": 1770000000 },
            "secondary": { "usedPercent": 12, "windowDurationMins": 10080, "resetsAt": 1770100000 }
        },
        "rateLimitsByLimitId": {
            "codex": {
                "limitId": "codex",
                "limitName": "Codex",
                "planType": "pro",
                "primary": { "usedPercent": 81, "windowDurationMins": 300, "resetsAt": 1770000000 },
                "secondary": { "usedPercent": 30, "windowDurationMins": 10080, "resetsAt": 1770100000 }
            }
        }
    });

    let result = parse_rate_limits_response(payload).expect("parse result");

    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0].window, "codex:5h");
    assert_eq!(result.rows[0].used_percent, Some(81.0));
    assert_eq!(result.rows[1].window, "codex:weekly");
    assert_eq!(result.rows[1].used_percent, Some(30.0));
    assert_eq!(result.debug.limit_id.as_deref(), Some("codex"));
    assert!(result.debug.had_rate_limits_by_limit_id);
}

#[test]
fn parses_top_level_codex_rate_limits_without_by_limit_id() {
    let payload = serde_json::json!({
        "rateLimits": {
            "limitId": "default",
            "limitName": "Default",
            "planType": null,
            "primary": { "usedPercent": 55, "windowDurationMins": 1440, "resetsAt": null },
            "secondary": null
        }
    });

    let result = parse_rate_limits_response(payload).expect("parse result");

    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].window, "codex:1d");
    assert_eq!(result.rows[0].used_percent, Some(55.0));
    assert_eq!(result.rows[0].reset_at, None);
}

#[test]
fn parses_codex_backend_usage_payload() {
    let payload = serde_json::json!({
        "plan_type": "pro",
        "rate_limit": {
            "allowed": true,
            "limit_reached": false,
            "primary_window": {
                "used_percent": 19,
                "limit_window_seconds": 18000,
                "reset_after_seconds": 60,
                "reset_at": 1770000000
            },
            "secondary_window": {
                "used_percent": 27,
                "limit_window_seconds": 604800,
                "reset_after_seconds": 60,
                "reset_at": 1770100000
            }
        },
        "additional_rate_limits": [
            {
                "limit_name": "Codex Other",
                "metered_feature": "codex_other",
                "rate_limit": {
                    "allowed": true,
                    "limit_reached": false,
                    "primary_window": {
                        "used_percent": 3,
                        "limit_window_seconds": 86400,
                        "reset_after_seconds": 60,
                        "reset_at": 1770200000
                    },
                    "secondary_window": null
                }
            }
        ]
    });

    let result = parse_rate_limits_response(payload).expect("parse result");

    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0].window, "codex:5h");
    assert_eq!(result.rows[0].used_percent, Some(19.0));
    assert_eq!(result.rows[1].window, "codex:weekly");
    assert_eq!(result.rows[1].used_percent, Some(27.0));
    assert_eq!(result.debug.limit_id.as_deref(), Some("codex"));
    assert!(result.debug.had_rate_limits_by_limit_id);
}

#[test]
fn codex_provider_reads_oauth_token_and_calls_usage_api() {
    let temp = tempfile::tempdir().expect("tempdir");
    let auth_path = temp.path().join("auth.json");
    fs::write(
        &auth_path,
        serde_json::json!({
            "tokens": {
                "id_token": "id-token",
                "access_token": "access-token",
                "refresh_token": "refresh-token",
                "account_id": "account-id"
            },
            "last_refresh": "2026-04-29T00:00:00Z"
        })
        .to_string(),
    )
    .expect("write auth");
    let client = FakeUsageClient {
        calls: RefCell::new(Vec::new()),
        result: Ok(serde_json::json!({
            "plan_type": "pro",
            "rate_limit": {
                "allowed": true,
                "limit_reached": false,
                "primary_window": {
                    "used_percent": 81,
                    "limit_window_seconds": 18000,
                    "reset_after_seconds": 60,
                    "reset_at": 1770000000
                },
                "secondary_window": null
            }
        })
        .to_string()),
    };
    let provider = CodexProvider::new(CodexProviderConfig {
        auth_path,
        base_url: "https://chatgpt.example/backend-api".to_string(),
        timeout: Duration::from_secs(1),
    });

    let result = provider.acquire_with_client(
        &client,
        AcquireContext {
            refresh: true,
            debug: true,
        },
    );

    assert_eq!(result.errors, vec![]);
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].window, "codex:5h");

    let calls = client.calls.borrow();
    assert_eq!(
        calls.as_slice(),
        [(
            "https://chatgpt.example/backend-api".to_string(),
            "access-token".to_string()
        )]
    );
}

#[test]
fn codex_provider_maps_runner_failure_to_normalized_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let auth_path = temp.path().join("auth.json");
    fs::write(
        &auth_path,
        serde_json::json!({
            "tokens": {
                "id_token": "id-token",
                "access_token": "access-token",
                "refresh_token": "refresh-token",
                "account_id": "account-id"
            },
            "last_refresh": "2026-04-29T00:00:00Z"
        })
        .to_string(),
    )
    .expect("write auth");
    let client = FakeUsageClient {
        calls: RefCell::new(Vec::new()),
        result: Err("codex usage API timed out after 1000ms".to_string()),
    };
    let provider = CodexProvider::new(CodexProviderConfig {
        auth_path,
        base_url: "https://chatgpt.example/backend-api".to_string(),
        timeout: Duration::from_secs(1),
    });

    let result = provider.acquire_with_client(
        &client,
        AcquireContext {
            refresh: true,
            debug: false,
        },
    );

    assert_eq!(result.rows, vec![]);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].provider.as_str(), "codex");
    assert_eq!(result.errors[0].error_type, NormalizedErrorType::Timeout);
}
