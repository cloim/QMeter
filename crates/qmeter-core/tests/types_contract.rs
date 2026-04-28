use qmeter_core::types::{
    Confidence, NormalizedError, NormalizedErrorType, NormalizedRow, NormalizedSnapshot,
    ProviderId, SourceKind,
};
use serde_json::json;

#[test]
fn serializes_normalized_snapshot_with_current_json_contract() {
    let snapshot = NormalizedSnapshot {
        fetched_at: "2026-02-24T00:00:00.000Z".to_string(),
        rows: vec![NormalizedRow {
            provider: ProviderId::Claude,
            window: "claude:session".to_string(),
            used: None,
            limit: None,
            used_percent: Some(79.0),
            reset_at: Some("2026-02-24T00:00:00.000Z".to_string()),
            source: SourceKind::Parsed,
            confidence: Confidence::Medium,
            stale: false,
            notes: Some("fixture".to_string()),
        }],
        errors: vec![NormalizedError {
            provider: ProviderId::Codex,
            error_type: NormalizedErrorType::AuthRequired,
            message: "login required".to_string(),
            actionable: Some("run `codex` once".to_string()),
        }],
    };

    let value = serde_json::to_value(snapshot).expect("snapshot should serialize");

    assert_eq!(
        value,
        json!({
            "fetchedAt": "2026-02-24T00:00:00.000Z",
            "rows": [{
                "provider": "claude",
                "window": "claude:session",
                "used": null,
                "limit": null,
                "usedPercent": 79.0,
                "resetAt": "2026-02-24T00:00:00.000Z",
                "source": "parsed",
                "confidence": "medium",
                "stale": false,
                "notes": "fixture"
            }],
            "errors": [{
                "provider": "codex",
                "type": "auth-required",
                "message": "login required",
                "actionable": "run `codex` once"
            }]
        })
    );
}
