use std::path::PathBuf;
use std::{fs, io};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraySettingsConfig {
    pub path: PathBuf,
}

impl TraySettingsConfig {
    pub fn from_env() -> Self {
        Self::from_values(
            std::env::var("USAGE_STATUS_TRAY_SETTINGS_PATH")
                .ok()
                .as_deref(),
            std::env::var("APPDATA").ok().as_deref(),
            std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
            std::env::var("USERPROFILE").ok().map(PathBuf::from),
        )
    }

    pub fn from_values(
        override_path: Option<&str>,
        app_data: Option<&str>,
        xdg_config_home: Option<&str>,
        home_dir: Option<PathBuf>,
    ) -> Self {
        let path = match override_path.filter(|value| !value.trim().is_empty()) {
            Some(path) => PathBuf::from(path),
            None => default_config_dir(app_data, xdg_config_home, home_dir)
                .join("qmeter")
                .join("tray-settings.v1.json"),
        };
        Self { path }
    }
}

fn default_config_dir(
    app_data: Option<&str>,
    xdg_config_home: Option<&str>,
    home_dir: Option<PathBuf>,
) -> PathBuf {
    if let Some(base) = app_data.filter(|value| !value.trim().is_empty()) {
        return PathBuf::from(base);
    }
    if let Some(base) = xdg_config_home.filter(|value| !value.trim().is_empty()) {
        return PathBuf::from(base);
    }
    home_dir
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraySettings {
    pub startup_enabled: bool,
    pub refresh_interval_ms: u64,
    pub visible_providers: VisibleProviders,
    pub notification: TrayNotificationSettings,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleProviders {
    pub claude: bool,
    pub codex: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayNotificationSettings {
    pub warning_percent: f64,
    pub critical_percent: f64,
    pub cooldown_minutes: u64,
    pub hysteresis_percent: f64,
    pub quiet_hours: TrayQuietHours,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayQuietHours {
    pub enabled: bool,
    pub start_hour: u8,
    pub end_hour: u8,
}

pub fn default_tray_settings() -> TraySettings {
    TraySettings {
        startup_enabled: false,
        refresh_interval_ms: 60_000,
        visible_providers: VisibleProviders {
            claude: true,
            codex: true,
        },
        notification: TrayNotificationSettings {
            warning_percent: 80.0,
            critical_percent: 95.0,
            cooldown_minutes: 60,
            hysteresis_percent: 2.0,
            quiet_hours: TrayQuietHours {
                enabled: false,
                start_hour: 22,
                end_hour: 8,
            },
        },
    }
}

pub fn load_tray_settings(config: &TraySettingsConfig) -> io::Result<TraySettings> {
    match fs::read_to_string(&config.path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid tray settings JSON: {err}"),
            )
        }),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            let settings = default_tray_settings();
            save_tray_settings(config, &settings)?;
            Ok(settings)
        }
        Err(err) => Err(err),
    }
}

pub fn save_tray_settings(config: &TraySettingsConfig, settings: &TraySettings) -> io::Result<()> {
    if let Some(parent) = config.path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize tray settings: {err}"),
        )
    })?;
    fs::write(&config.path, format!("{json}\n"))
}
