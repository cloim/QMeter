use std::path::PathBuf;

use qmeter_core::cache::{
    as_cache_rows, is_entry_fresh, load_cache, save_cache, CacheConfig, CacheProviderEntry,
    CacheState,
};
use qmeter_core::types::{Confidence, NormalizedRow, ProviderId, SourceKind};

fn row(provider: ProviderId) -> NormalizedRow {
    NormalizedRow {
        provider,
        window: format!("{}:fixture", provider.as_str()),
        used: None,
        limit: None,
        used_percent: Some(42.0),
        reset_at: None,
        source: SourceKind::Structured,
        confidence: Confidence::High,
        stale: false,
        notes: None,
    }
}

#[test]
fn cache_config_uses_override_path_and_ttl() {
    let cfg = CacheConfig::from_values(
        Some("D:\\tmp\\qmeter-cache.json"),
        Some("2.5"),
        Some("C:\\Users\\me\\AppData\\Local"),
        None,
        Some(PathBuf::from("C:\\Users\\me")),
    );

    assert_eq!(cfg.path, PathBuf::from("D:\\tmp\\qmeter-cache.json"));
    assert_eq!(cfg.ttl_ms, 2_500);
}

#[test]
fn cache_config_defaults_to_localappdata_qmeter_cache() {
    let cfg = CacheConfig::from_values(
        None,
        None,
        Some("C:\\Users\\me\\AppData\\Local"),
        None,
        Some(PathBuf::from("C:\\Users\\me")),
    );

    assert_eq!(
        cfg.path,
        PathBuf::from("C:\\Users\\me\\AppData\\Local")
            .join("qmeter")
            .join("cache.v1.json")
    );
    assert_eq!(cfg.ttl_ms, 60_000);
}

#[test]
fn cache_freshness_respects_ttl() {
    let entry = CacheProviderEntry {
        fetched_at: "2026-04-28T00:00:00.000Z".to_string(),
        rows: vec![row(ProviderId::Codex)],
    };

    let now = "2026-04-28T00:00:30.000Z";
    assert!(is_entry_fresh(&entry, 60_000, now));
    assert!(!is_entry_fresh(&entry, 10_000, now));
}

#[test]
fn cache_rows_are_rewritten_as_cache_source() {
    let rows = as_cache_rows(&[row(ProviderId::Claude)], true, Some("stale cache"));

    assert_eq!(rows[0].source, SourceKind::Cache);
    assert!(rows[0].stale);
    assert_eq!(rows[0].notes.as_deref(), Some("stale cache"));
}

#[test]
fn cache_round_trip_preserves_provider_entries() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cfg = CacheConfig {
        path: dir.path().join("cache.v1.json"),
        ttl_ms: 60_000,
    };
    let mut state = CacheState::new(cfg.clone());
    state.providers.insert(
        ProviderId::Codex,
        CacheProviderEntry {
            fetched_at: "2026-04-28T00:00:00.000Z".to_string(),
            rows: vec![row(ProviderId::Codex)],
        },
    );

    save_cache(&state).expect("save cache");
    let loaded = load_cache(cfg).expect("load cache");

    assert_eq!(loaded.providers[&ProviderId::Codex].rows[0].window, "codex:fixture");
}
