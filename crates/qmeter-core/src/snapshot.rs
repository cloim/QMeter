use crate::types::{
    Confidence, NormalizedError, NormalizedErrorType, NormalizedRow, NormalizedSnapshot,
    ProviderId, SourceKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectOptions {
    pub refresh: bool,
    pub debug: bool,
    pub providers: Vec<ProviderId>,
}

const FIXED_NOW: &str = "2026-02-24T00:00:00.000Z";

pub fn collect_fixture_snapshot(opts: &CollectOptions) -> NormalizedSnapshot {
    let mut rows = Vec::new();

    for provider in &opts.providers {
        match provider {
            ProviderId::Claude => rows.extend(claude_fixture_rows()),
            ProviderId::Codex => rows.extend(codex_fixture_rows()),
        }
    }

    NormalizedSnapshot {
        fetched_at: FIXED_NOW.to_string(),
        rows,
        errors: Vec::new(),
    }
}

pub fn collect_unimplemented_snapshot(opts: &CollectOptions) -> NormalizedSnapshot {
    NormalizedSnapshot {
        fetched_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        rows: Vec::new(),
        errors: opts
            .providers
            .iter()
            .map(|provider| NormalizedError {
                provider: *provider,
                error_type: NormalizedErrorType::AcquireFailed,
                message: "Rust provider acquisition is not implemented yet".to_string(),
                actionable: Some(
                    "use USAGE_STATUS_FIXTURE=demo or enable the Rust live provider path"
                        .to_string(),
                ),
            })
            .collect(),
    }
}

pub fn is_fixture_mode_from_env() -> bool {
    std::env::var("USAGE_STATUS_FIXTURE")
        .ok()
        .map(|value| value.trim().eq_ignore_ascii_case("demo"))
        .unwrap_or(false)
}

fn claude_fixture_rows() -> [NormalizedRow; 2] {
    [
        NormalizedRow {
            provider: ProviderId::Claude,
            window: "claude:session".to_string(),
            used: None,
            limit: None,
            used_percent: Some(79.0),
            reset_at: Some(FIXED_NOW.to_string()),
            source: SourceKind::Parsed,
            confidence: Confidence::Medium,
            stale: false,
            notes: Some("fixture".to_string()),
        },
        NormalizedRow {
            provider: ProviderId::Claude,
            window: "claude:week(all-models)".to_string(),
            used: None,
            limit: None,
            used_percent: Some(22.0),
            reset_at: Some(FIXED_NOW.to_string()),
            source: SourceKind::Parsed,
            confidence: Confidence::Medium,
            stale: false,
            notes: Some("fixture".to_string()),
        },
    ]
}

fn codex_fixture_rows() -> [NormalizedRow; 2] {
    [
        NormalizedRow {
            provider: ProviderId::Codex,
            window: "codex:5h".to_string(),
            used: None,
            limit: None,
            used_percent: Some(81.0),
            reset_at: Some(FIXED_NOW.to_string()),
            source: SourceKind::Structured,
            confidence: Confidence::High,
            stale: false,
            notes: Some("fixture".to_string()),
        },
        NormalizedRow {
            provider: ProviderId::Codex,
            window: "codex:weekly".to_string(),
            used: None,
            limit: None,
            used_percent: Some(30.0),
            reset_at: Some(FIXED_NOW.to_string()),
            source: SourceKind::Structured,
            confidence: Confidence::High,
            stale: false,
            notes: Some("fixture".to_string()),
        },
    ]
}
