use std::collections::BTreeMap;

use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::types::NormalizedRow;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertLevel {
    Normal,
    Warning,
    Critical,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationThresholds {
    pub warning_percent: f64,
    pub critical_percent: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuietHours {
    pub enabled: bool,
    pub start_hour: u8,
    pub end_hour: u8,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPolicyConfig {
    pub thresholds: NotificationThresholds,
    pub cooldown_ms: u64,
    pub hysteresis_percent: f64,
    pub quiet_hours: QuietHours,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationState {
    pub event_key: String,
    pub level: AlertLevel,
    pub last_notified_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NotificationEvent {
    pub event_key: String,
    pub level: AlertLevel,
    pub row: NormalizedRow,
    pub reason: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NotificationEvaluation {
    pub events: Vec<NotificationEvent>,
    pub next_state: BTreeMap<String, NotificationState>,
}

pub fn make_event_key(row: &NormalizedRow) -> String {
    format!("{}:{}", row.provider.as_str(), row.window)
}

pub fn to_alert_level(row: &NormalizedRow, thresholds: &NotificationThresholds) -> AlertLevel {
    let percent = row.used_percent.unwrap_or(0.0);
    if percent >= thresholds.critical_percent {
        AlertLevel::Critical
    } else if percent >= thresholds.warning_percent {
        AlertLevel::Warning
    } else {
        AlertLevel::Normal
    }
}

pub fn is_in_quiet_hours(q: &QuietHours, hour: u8) -> bool {
    if !q.enabled {
        return false;
    }
    let start = q.start_hour.min(23);
    let end = q.end_hour.min(23);
    let hour = hour.min(23);

    if start == end {
        return true;
    }
    if start < end {
        return hour >= start && hour < end;
    }
    hour >= start || hour < end
}

pub fn evaluate_notification_policy(
    rows: &[NormalizedRow],
    prev_state: &BTreeMap<String, NotificationState>,
    cfg: &NotificationPolicyConfig,
    now: &str,
) -> NotificationEvaluation {
    let now_dt = DateTime::parse_from_rfc3339(now)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let quiet = is_in_quiet_hours(&cfg.quiet_hours, now_dt.hour() as u8);
    let mut next_state = prev_state.clone();
    let mut events = Vec::new();

    for row in rows {
        let Some(used_percent) = row.used_percent else {
            continue;
        };
        let event_key = make_event_key(row);
        let prev = prev_state.get(&event_key);
        let prev_level = prev.map(|state| state.level).unwrap_or(AlertLevel::Normal);
        let level = level_with_hysteresis(used_percent, prev_level, cfg);
        let mut candidate = NotificationState {
            event_key: event_key.clone(),
            level,
            last_notified_at: prev.and_then(|state| state.last_notified_at.clone()),
        };

        let should_notify =
            !quiet && should_notify_transition(prev, &candidate, cfg.cooldown_ms, now_dt);
        if should_notify && level != AlertLevel::Normal {
            let reason = if prev.is_some_and(|state| state.level == level) {
                "cooldown"
            } else {
                "transition"
            };
            events.push(NotificationEvent {
                event_key: event_key.clone(),
                level,
                row: row.clone(),
                reason,
            });
            candidate.last_notified_at =
                Some(now_dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        }

        next_state.insert(event_key, candidate);
    }

    NotificationEvaluation { events, next_state }
}

fn level_with_hysteresis(
    used_percent: f64,
    prev: AlertLevel,
    cfg: &NotificationPolicyConfig,
) -> AlertLevel {
    let percent = used_percent.clamp(0.0, 100.0);
    let warning = cfg.thresholds.warning_percent;
    let critical = cfg.thresholds.critical_percent;
    let h = cfg.hysteresis_percent.max(0.0);

    match prev {
        AlertLevel::Critical => {
            if percent >= critical - h {
                AlertLevel::Critical
            } else if percent >= warning {
                AlertLevel::Warning
            } else {
                AlertLevel::Normal
            }
        }
        AlertLevel::Warning => {
            if percent >= critical {
                AlertLevel::Critical
            } else if percent >= warning - h {
                AlertLevel::Warning
            } else {
                AlertLevel::Normal
            }
        }
        AlertLevel::Normal => {
            if percent >= critical {
                AlertLevel::Critical
            } else if percent >= warning {
                AlertLevel::Warning
            } else {
                AlertLevel::Normal
            }
        }
    }
}

fn should_notify_transition(
    prev: Option<&NotificationState>,
    next: &NotificationState,
    cooldown_ms: u64,
    now: DateTime<Utc>,
) -> bool {
    let Some(prev) = prev else {
        return next.level != AlertLevel::Normal;
    };
    if prev.level != next.level {
        return next.level != AlertLevel::Normal;
    }
    let Some(last_notified_at) = prev.last_notified_at.as_deref() else {
        return false;
    };
    let Ok(last) = DateTime::parse_from_rfc3339(last_notified_at) else {
        return false;
    };
    now.signed_duration_since(last.with_timezone(&Utc))
        .num_milliseconds()
        >= cooldown_ms as i64
}
