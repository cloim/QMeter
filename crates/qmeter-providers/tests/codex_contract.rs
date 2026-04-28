use std::cell::RefCell;
use std::time::Duration;

use qmeter_core::types::NormalizedErrorType;
use qmeter_providers::codex::{
    AppServerRunner, CodexProvider, CodexProviderConfig, parse_rate_limits_response,
};
use qmeter_providers::provider::AcquireContext;
use serde_json::Value;

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

struct FakeRunner {
    requests: RefCell<Vec<Value>>,
    result: Result<Value, String>,
}

impl AppServerRunner for FakeRunner {
    fn exchange(
        &self,
        _command: &str,
        requests: &[Value],
        _timeout: Duration,
    ) -> Result<Value, String> {
        self.requests.borrow_mut().extend_from_slice(requests);
        self.result.clone()
    }
}

#[test]
fn codex_provider_sends_json_rpc_handshake_and_returns_rows() {
    let runner = FakeRunner {
        requests: RefCell::new(Vec::new()),
        result: Ok(serde_json::json!({
            "rateLimits": {
                "limitId": "codex",
                "limitName": "Codex",
                "planType": "pro",
                "primary": { "usedPercent": 81, "windowDurationMins": 300, "resetsAt": null },
                "secondary": null
            }
        })),
    };
    let provider = CodexProvider::new(CodexProviderConfig {
        codex_command: "codex-test".to_string(),
        timeout: Duration::from_secs(1),
    });

    let result = provider.acquire_with_runner(
        &runner,
        AcquireContext {
            refresh: true,
            debug: true,
        },
    );

    assert_eq!(result.errors, vec![]);
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].window, "codex:5h");

    let requests = runner.requests.borrow();
    assert_eq!(requests[0]["method"], "initialize");
    assert_eq!(requests[1]["method"], "initialized");
    assert_eq!(requests[2]["method"], "account/rateLimits/read");
}

#[test]
fn codex_provider_maps_runner_failure_to_normalized_error() {
    let runner = FakeRunner {
        requests: RefCell::new(Vec::new()),
        result: Err("codex app-server timed out after 1000ms".to_string()),
    };
    let provider = CodexProvider::new(CodexProviderConfig {
        codex_command: "codex-test".to_string(),
        timeout: Duration::from_secs(1),
    });

    let result = provider.acquire_with_runner(
        &runner,
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
