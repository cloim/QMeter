use std::path::PathBuf;

use qmeter_core::settings::{
    TraySettingsConfig, default_tray_settings, load_tray_settings, save_tray_settings,
};

#[test]
fn default_settings_match_current_tray_defaults() {
    let settings = default_tray_settings();

    assert!(!settings.startup_enabled);
    assert_eq!(settings.refresh_interval_ms, 60_000);
    assert!(settings.visible_providers.claude);
    assert!(settings.visible_providers.codex);
    assert_eq!(settings.notification.warning_percent, 80.0);
    assert_eq!(settings.notification.critical_percent, 95.0);
    assert_eq!(settings.notification.cooldown_minutes, 60);
    assert_eq!(settings.notification.hysteresis_percent, 2.0);
}

#[test]
fn settings_path_uses_override_or_appdata_default() {
    let override_cfg = TraySettingsConfig::from_values(
        Some("D:\\tmp\\settings.json"),
        Some("C:\\Users\\me\\AppData\\Roaming"),
        None,
        Some(PathBuf::from("C:\\Users\\me")),
    );
    assert_eq!(override_cfg.path, PathBuf::from("D:\\tmp\\settings.json"));

    let default_cfg = TraySettingsConfig::from_values(
        None,
        Some("C:\\Users\\me\\AppData\\Roaming"),
        None,
        Some(PathBuf::from("C:\\Users\\me")),
    );
    assert_eq!(
        default_cfg.path,
        PathBuf::from("C:\\Users\\me\\AppData\\Roaming")
            .join("qmeter")
            .join("tray-settings.v1.json")
    );
}

#[test]
fn settings_round_trip_preserves_values() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cfg = TraySettingsConfig {
        path: dir.path().join("tray-settings.v1.json"),
    };
    let mut settings = default_tray_settings();
    settings.refresh_interval_ms = 30_000;
    settings.visible_providers.claude = false;

    save_tray_settings(&cfg, &settings).expect("save settings");
    let loaded = load_tray_settings(&cfg).expect("load settings");

    assert_eq!(loaded.refresh_interval_ms, 30_000);
    assert!(!loaded.visible_providers.claude);
    assert!(loaded.visible_providers.codex);
}

#[test]
fn missing_settings_loads_defaults_and_creates_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cfg = TraySettingsConfig {
        path: dir.path().join("nested").join("tray-settings.v1.json"),
    };

    let loaded = load_tray_settings(&cfg).expect("load settings");

    assert_eq!(loaded, default_tray_settings());
    assert!(cfg.path.exists());
}
