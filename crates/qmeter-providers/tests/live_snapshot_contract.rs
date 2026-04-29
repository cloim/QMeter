use qmeter_core::cache::{CacheConfig, CacheProviderEntry, CacheState, save_cache};
use qmeter_core::snapshot::CollectOptions;
use qmeter_core::types::{
    Confidence, NormalizedError, NormalizedErrorType, NormalizedRow, ProviderId, SourceKind,
};
use qmeter_providers::provider::ProviderResult;
use qmeter_providers::snapshot::collect_live_snapshot_with;

fn row(provider: ProviderId, window: &str, percent: f64) -> NormalizedRow {
    NormalizedRow {
        provider,
        window: window.to_string(),
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

fn opts(provider: ProviderId, refresh: bool) -> CollectOptions {
    CollectOptions {
        refresh,
        debug: false,
        providers: vec![provider],
    }
}

#[test]
fn live_snapshot_uses_fresh_cache_without_acquiring_provider() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache_config = CacheConfig {
        path: dir.path().join("cache.v1.json"),
        ttl_ms: 60_000,
    };
    let mut cache = CacheState::new(cache_config.clone());
    cache.providers.insert(
        ProviderId::Claude,
        CacheProviderEntry {
            fetched_at: "2026-04-28T00:00:00.000Z".to_string(),
            rows: vec![row(ProviderId::Claude, "claude:5h", 11.0)],
        },
    );
    save_cache(&cache).expect("save cache");

    let snapshot = collect_live_snapshot_with(
        &opts(ProviderId::Claude, false),
        cache_config,
        "2026-04-28T00:00:01.000Z",
        |_| panic!("provider should not be acquired while cache is fresh"),
    );

    assert_eq!(snapshot.errors, vec![]);
    assert_eq!(snapshot.rows.len(), 1);
    assert_eq!(snapshot.rows[0].source, SourceKind::Cache);
    assert!(!snapshot.rows[0].stale);
}

#[test]
fn live_snapshot_falls_back_to_stale_cache_when_provider_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache_config = CacheConfig {
        path: dir.path().join("cache.v1.json"),
        ttl_ms: 0,
    };
    let mut cache = CacheState::new(cache_config.clone());
    cache.providers.insert(
        ProviderId::Codex,
        CacheProviderEntry {
            fetched_at: "2026-04-28T00:00:00.000Z".to_string(),
            rows: vec![row(ProviderId::Codex, "codex:5h", 44.0)],
        },
    );
    save_cache(&cache).expect("save cache");

    let snapshot = collect_live_snapshot_with(
        &opts(ProviderId::Codex, false),
        cache_config,
        "2026-04-28T00:00:01.000Z",
        |_| ProviderResult {
            rows: Vec::new(),
            errors: vec![NormalizedError {
                provider: ProviderId::Codex,
                error_type: NormalizedErrorType::NotInstalled,
                message: "missing".to_string(),
                actionable: None,
            }],
            debug: None,
        },
    );

    assert_eq!(snapshot.rows.len(), 1);
    assert_eq!(snapshot.rows[0].source, SourceKind::Cache);
    assert!(snapshot.rows[0].stale);
    assert_eq!(
        snapshot.errors[0].error_type,
        NormalizedErrorType::NotInstalled
    );
}
