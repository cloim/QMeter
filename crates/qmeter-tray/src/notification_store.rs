use std::collections::BTreeMap;
use std::path::PathBuf;
use std::{fs, io};

use qmeter_core::notification_policy::NotificationState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationStoreConfig {
    pub path: PathBuf,
}

impl NotificationStoreConfig {
    pub fn from_env() -> Self {
        let path = std::env::var("USAGE_STATUS_TRAY_NOTIFICATION_STATE_PATH")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let base = std::env::var_os("LOCALAPPDATA")
                    .or_else(|| std::env::var_os("XDG_STATE_HOME"))
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."));
                base.join("qmeter").join("notification-state.v1.json")
            });
        Self { path }
    }
}

pub fn load_notification_state(
    config: &NotificationStoreConfig,
) -> io::Result<BTreeMap<String, NotificationState>> {
    match fs::read_to_string(&config.path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid notification state JSON: {err}"),
            )
        }),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(err) => Err(err),
    }
}

pub fn save_notification_state(
    config: &NotificationStoreConfig,
    state: &BTreeMap<String, NotificationState>,
) -> io::Result<()> {
    if let Some(parent) = config.path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize notification state: {err}"),
        )
    })?;
    fs::write(&config.path, format!("{json}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use qmeter_core::notification_policy::AlertLevel;

    #[test]
    fn missing_notification_state_returns_empty_map() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cfg = NotificationStoreConfig {
            path: dir.path().join("state.json"),
        };

        let loaded = load_notification_state(&cfg).expect("load state");

        assert!(loaded.is_empty());
    }

    #[test]
    fn notification_state_round_trips() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cfg = NotificationStoreConfig {
            path: dir.path().join("nested").join("state.json"),
        };
        let mut state = BTreeMap::new();
        state.insert(
            "codex:codex:5h".to_string(),
            NotificationState {
                event_key: "codex:codex:5h".to_string(),
                level: AlertLevel::Warning,
                last_notified_at: Some("2026-04-28T00:00:00.000Z".to_string()),
            },
        );

        save_notification_state(&cfg, &state).expect("save state");
        let loaded = load_notification_state(&cfg).expect("load state");

        assert_eq!(loaded, state);
    }
}
