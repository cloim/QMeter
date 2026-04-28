use std::collections::BTreeMap;

use qmeter_core::notification_policy::{
    evaluate_notification_policy, is_in_quiet_hours, AlertLevel, NotificationPolicyConfig,
    NotificationState, NotificationThresholds, QuietHours,
};
use qmeter_core::types::{Confidence, NormalizedRow, ProviderId, SourceKind};

fn cfg() -> NotificationPolicyConfig {
    NotificationPolicyConfig {
        thresholds: NotificationThresholds {
            warning_percent: 80.0,
            critical_percent: 95.0,
        },
        cooldown_ms: 60_000,
        hysteresis_percent: 2.0,
        quiet_hours: QuietHours {
            enabled: false,
            start_hour: 22,
            end_hour: 8,
        },
    }
}

fn row(percent: f64) -> NormalizedRow {
    NormalizedRow {
        provider: ProviderId::Codex,
        window: "codex:5h".to_string(),
        used: None,
        limit: None,
        used_percent: Some(percent),
        reset_at: None,
        source: SourceKind::Structured,
        confidence: Confidence::High,
        stale: false,
        notes: None,
    }
}

#[test]
fn alerts_on_threshold_crossing() {
    let s1 = evaluate_notification_policy(
        &[row(79.0)],
        &BTreeMap::new(),
        &cfg(),
        "2026-02-24T00:00:00.000Z",
    );
    assert_eq!(s1.events.len(), 0);

    let s2 = evaluate_notification_policy(
        &[row(81.0)],
        &s1.next_state,
        &cfg(),
        "2026-02-24T00:00:10.000Z",
    );

    assert_eq!(s2.events.len(), 1);
    assert_eq!(s2.events[0].level, AlertLevel::Warning);
    assert_eq!(s2.events[0].reason, "transition");
}

#[test]
fn suppresses_during_cooldown_and_renotifies_after_cooldown() {
    let mut prev = BTreeMap::new();
    prev.insert(
        "codex:codex:5h".to_string(),
        NotificationState {
            event_key: "codex:codex:5h".to_string(),
            level: AlertLevel::Warning,
            last_notified_at: Some("2026-02-24T00:00:00.000Z".to_string()),
        },
    );

    let early = evaluate_notification_policy(
        &[row(85.0)],
        &prev,
        &cfg(),
        "2026-02-24T00:00:30.000Z",
    );
    assert_eq!(early.events.len(), 0);

    let late = evaluate_notification_policy(
        &[row(85.0)],
        &prev,
        &cfg(),
        "2026-02-24T00:02:00.000Z",
    );
    assert_eq!(late.events.len(), 1);
    assert_eq!(late.events[0].reason, "cooldown");
}

#[test]
fn hysteresis_prevents_warning_drop_near_threshold() {
    let mut prev = BTreeMap::new();
    prev.insert(
        "codex:codex:5h".to_string(),
        NotificationState {
            event_key: "codex:codex:5h".to_string(),
            level: AlertLevel::Warning,
            last_notified_at: None,
        },
    );

    let keep = evaluate_notification_policy(&[row(79.0)], &prev, &cfg(), "2026-02-24T00:00:00Z");
    assert_eq!(
        keep.next_state["codex:codex:5h"].level,
        AlertLevel::Warning
    );

    let drop = evaluate_notification_policy(&[row(77.0)], &prev, &cfg(), "2026-02-24T00:00:00Z");
    assert_eq!(drop.next_state["codex:codex:5h"].level, AlertLevel::Normal);
}

#[test]
fn quiet_hours_across_midnight() {
    let quiet = QuietHours {
        enabled: true,
        start_hour: 22,
        end_hour: 8,
    };

    assert!(is_in_quiet_hours(&quiet, 23));
    assert!(is_in_quiet_hours(&quiet, 7));
    assert!(!is_in_quiet_hours(&quiet, 12));
}
