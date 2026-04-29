use qmeter_core::cache::{
    CacheConfig, CacheProviderEntry, CacheState, as_cache_rows, is_entry_fresh, load_cache,
    save_cache,
};
use qmeter_core::snapshot::CollectOptions;
use qmeter_core::types::{NormalizedSnapshot, ProviderId};

use crate::claude::ClaudeProvider;
use crate::codex::CodexProvider;
use crate::provider::{AcquireContext, Provider, ProviderResult};

#[derive(Clone, Debug, PartialEq)]
pub struct LiveSnapshot {
    pub snapshot: NormalizedSnapshot,
    pub debug_messages: Vec<(ProviderId, serde_json::Value)>,
}

pub fn collect_live_snapshot(opts: &CollectOptions) -> LiveSnapshot {
    let cache_config = CacheConfig::from_env();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    collect_live_snapshot_with_debug(opts, cache_config, &now, |provider| {
        acquire_default_provider(
            provider,
            AcquireContext {
                refresh: opts.refresh,
                debug: opts.debug,
            },
        )
    })
}

pub fn collect_live_snapshot_with<F>(
    opts: &CollectOptions,
    cache_config: CacheConfig,
    now: &str,
    acquire: F,
) -> NormalizedSnapshot
where
    F: FnMut(ProviderId) -> ProviderResult,
{
    collect_live_snapshot_with_debug(opts, cache_config, now, acquire).snapshot
}

fn collect_live_snapshot_with_debug<F>(
    opts: &CollectOptions,
    cache_config: CacheConfig,
    now: &str,
    mut acquire: F,
) -> LiveSnapshot
where
    F: FnMut(ProviderId) -> ProviderResult,
{
    let mut cache =
        load_cache(cache_config.clone()).unwrap_or_else(|_| CacheState::new(cache_config));
    let mut snapshot = NormalizedSnapshot {
        fetched_at: now.to_string(),
        rows: Vec::new(),
        errors: Vec::new(),
    };
    let mut debug_messages = Vec::new();
    let mut cache_dirty = false;

    for provider in &opts.providers {
        let cached = cache.providers.get(provider).cloned();
        if !opts.refresh
            && let Some(entry) = cached.as_ref()
            && is_entry_fresh(entry, cache.config.ttl_ms, now)
        {
            snapshot.rows.extend(as_cache_rows(
                &entry.rows,
                false,
                Some(&format!("cached at {}", entry.fetched_at)),
            ));
            continue;
        }

        let result = acquire(*provider);
        if opts.debug
            && let Some(debug) = result.debug.clone()
        {
            debug_messages.push((*provider, debug));
        }
        snapshot.errors.extend(result.errors);

        if result.rows.is_empty() {
            if let Some(entry) = cached {
                snapshot.rows.extend(as_cache_rows(
                    &entry.rows,
                    true,
                    Some(&format!("stale cache from {}", entry.fetched_at)),
                ));
            }
        } else {
            cache.providers.insert(
                *provider,
                CacheProviderEntry {
                    fetched_at: now.to_string(),
                    rows: result.rows.clone(),
                },
            );
            cache_dirty = true;
            snapshot.rows.extend(result.rows);
        }
    }

    if cache_dirty {
        let _ = save_cache(&cache);
    }

    LiveSnapshot {
        snapshot,
        debug_messages,
    }
}

fn acquire_default_provider(provider: ProviderId, ctx: AcquireContext) -> ProviderResult {
    match provider {
        ProviderId::Claude => ClaudeProvider::default().acquire(ctx),
        ProviderId::Codex => CodexProvider::default().acquire(ctx),
    }
}
