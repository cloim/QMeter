use std::path::PathBuf;

use qmeter_core::settings::{default_tray_settings, TraySettingsConfig};

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
