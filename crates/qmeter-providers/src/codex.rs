use chrono::{DateTime, SecondsFormat, Utc};
use qmeter_core::types::{Confidence, NormalizedRow, ProviderId, SourceKind};
use serde::Deserialize;
use serde_json::Value;

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
