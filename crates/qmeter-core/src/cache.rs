use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{NormalizedRow, ProviderId, SourceKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheConfig {
    pub path: PathBuf,
    pub ttl_ms: u64,
}

impl CacheConfig {
    pub fn from_env() -> Self {
        Self::from_values(
            std::env::var("USAGE_STATUS_CACHE_PATH").ok().as_deref(),
            std::env::var("USAGE_STATUS_CACHE_TTL_SECS").ok().as_deref(),
            std::env::var("LOCALAPPDATA").ok().as_deref(),
            std::env::var("XDG_CACHE_HOME").ok().as_deref(),
            std::env::var("USERPROFILE").ok().map(PathBuf::from),
        )
    }

    pub fn from_values(
        cache_path: Option<&str>,
        ttl_secs: Option<&str>,
        local_app_data: Option<&str>,
        xdg_cache_home: Option<&str>,
        home_dir: Option<PathBuf>,
    ) -> Self {
        let path = match cache_path.filter(|value| !value.trim().is_empty()) {
            Some(path) => PathBuf::from(path),
            None => default_cache_dir(local_app_data, xdg_cache_home, home_dir)
                .join("qmeter")
                .join("cache.v1.json"),
        };

        let ttl_ms = ttl_secs
            .and_then(|raw| raw.trim().parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(|secs| (secs * 1000.0).floor() as u64)
            .unwrap_or(60_000);

        Self { path, ttl_ms }
    }
}

fn default_cache_dir(
    local_app_data: Option<&str>,
    xdg_cache_home: Option<&str>,
    home_dir: Option<PathBuf>,
) -> PathBuf {
    if let Some(base) = local_app_data.filter(|value| !value.trim().is_empty()) {
        return PathBuf::from(base);
    }
    if let Some(base) = xdg_cache_home.filter(|value| !value.trim().is_empty()) {
        return PathBuf::from(base);
    }
    home_dir
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache")
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheProviderEntry {
    pub fetched_at: String,
    pub rows: Vec<NormalizedRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CacheState {
    pub config: CacheConfig,
    pub providers: BTreeMap<ProviderId, CacheProviderEntry>,
}

impl CacheState {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            providers: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CacheFile {
    version: u8,
    saved_at: String,
    providers: BTreeMap<ProviderId, Option<CacheProviderEntry>>,
}

pub fn load_cache(config: CacheConfig) -> io::Result<CacheState> {
    let raw = match fs::read_to_string(&config.path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(CacheState::new(config)),
        Err(err) => return Err(err),
    };

    let parsed = match serde_json::from_str::<CacheFile>(&raw) {
        Ok(parsed) if parsed.version == 1 => parsed,
        _ => return Ok(CacheState::new(config)),
    };

    let providers = parsed
        .providers
        .into_iter()
        .filter_map(|(id, entry)| entry.map(|entry| (id, entry)))
        .collect();

    Ok(CacheState { config, providers })
}

pub fn save_cache(state: &CacheState) -> io::Result<()> {
    if let Some(parent) = state.config.path.parent() {
        fs::create_dir_all(parent)?;
    }

    let providers = [ProviderId::Claude, ProviderId::Codex]
        .into_iter()
        .map(|id| (id, state.providers.get(&id).cloned()))
        .collect();

    let file = CacheFile {
        version: 1,
        saved_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        providers,
    };

    let json = serde_json::to_string_pretty(&file)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(&state.config.path, json)
}

pub fn is_entry_fresh(entry: &CacheProviderEntry, ttl_ms: u64, now: &str) -> bool {
    if ttl_ms == 0 {
        return false;
    }

    let fetched_at = match DateTime::parse_from_rfc3339(&entry.fetched_at) {
        Ok(value) => value.with_timezone(&Utc),
        Err(_) => return false,
    };
    let now = match DateTime::parse_from_rfc3339(now) {
        Ok(value) => value.with_timezone(&Utc),
        Err(_) => return false,
    };
    let elapsed = now.signed_duration_since(fetched_at).num_milliseconds();
    elapsed >= 0 && elapsed as u64 <= ttl_ms
}

pub fn as_cache_rows(rows: &[NormalizedRow], stale: bool, note: Option<&str>) -> Vec<NormalizedRow> {
    rows.iter()
        .map(|row| {
            let mut next = row.clone();
            next.source = SourceKind::Cache;
            next.stale = stale;
            next.notes = match (next.notes.take(), note) {
                (Some(existing), Some(note)) => Some(format!("{existing}; {note}")),
                (None, Some(note)) => Some(note.to_string()),
                (existing, None) => existing,
            };
            next
        })
        .collect()
}

pub fn cache_path(config: &CacheConfig) -> &Path {
    &config.path
}
