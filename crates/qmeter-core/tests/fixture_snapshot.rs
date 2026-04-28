use qmeter_core::snapshot::{collect_fixture_snapshot, CollectOptions};
use qmeter_core::types::ProviderId;

#[test]
fn fixture_snapshot_matches_current_demo_rows() {
    let snapshot = collect_fixture_snapshot(&CollectOptions {
        refresh: false,
        debug: false,
        providers: vec![ProviderId::Claude, ProviderId::Codex],
    });

    assert_eq!(snapshot.fetched_at, "2026-02-24T00:00:00.000Z");
    assert_eq!(snapshot.errors, vec![]);
    assert_eq!(snapshot.rows.len(), 4);

    let windows: Vec<_> = snapshot
        .rows
        .iter()
        .map(|row| (row.provider, row.window.as_str(), row.used_percent))
        .collect();

    assert_eq!(
        windows,
        vec![
            (ProviderId::Claude, "claude:session", Some(79.0)),
            (ProviderId::Claude, "claude:week(all-models)", Some(22.0)),
            (ProviderId::Codex, "codex:5h", Some(81.0)),
            (ProviderId::Codex, "codex:weekly", Some(30.0)),
        ]
    );
}

#[test]
fn fixture_snapshot_respects_selected_providers() {
    let snapshot = collect_fixture_snapshot(&CollectOptions {
        refresh: false,
        debug: false,
        providers: vec![ProviderId::Codex],
    });

    assert_eq!(snapshot.rows.len(), 2);
    assert!(snapshot.rows.iter().all(|row| row.provider == ProviderId::Codex));
}
