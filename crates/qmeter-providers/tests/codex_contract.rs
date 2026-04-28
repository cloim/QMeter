use qmeter_providers::codex::parse_rate_limits_response;

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
