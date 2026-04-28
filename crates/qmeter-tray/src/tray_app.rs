use crate::runtime_log::{RuntimeLogConfig, append_runtime_log};
use crate::tray_state::TrayState;

pub fn run_tray_app() -> Result<(), Box<dyn std::error::Error>> {
    let log_config = RuntimeLogConfig::from_env();
    append_runtime_log(&log_config, "startup", "qmeter tray starting")?;
    let mut state = TrayState::default();
    refresh_state(&mut state, &log_config, false)?;
    run_platform_tray(state, log_config)
}

fn refresh_state(
    state: &mut TrayState,
    log_config: &RuntimeLogConfig,
    force_refresh: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    state.refresh_current_mode(force_refresh);
    let popup_text = state.render_popup_text();
    append_runtime_log(
        log_config,
        "refresh",
        &format!(
            "rows={} errors={} popup_chars={}",
            state.snapshot.as_ref().map_or(0, |s| s.rows.len()),
            state.snapshot.as_ref().map_or(0, |s| s.errors.len()),
            popup_text.len()
        ),
    )?;
    Ok(())
}

#[cfg(windows)]
fn run_platform_tray(
    mut state: TrayState,
    log_config: RuntimeLogConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
        menu::{Menu, MenuEvent, MenuItem},
    };

    let menu = Menu::new();
    let open = MenuItem::new("Open QMeter", true, None);
    let open_id = open.id().clone();
    let refresh = MenuItem::new("Refresh", true, None);
    let refresh_id = refresh.id().clone();
    let settings = MenuItem::new("Settings", true, None);
    let settings_id = settings.id().clone();
    let quit = MenuItem::new("Quit", true, None);
    let quit_id = quit.id().clone();
    menu.append(&open)?;
    menu.append(&refresh)?;
    menu.append(&settings)?;
    menu.append(&quit)?;

    let icon = Icon::from_rgba(vec![0, 0, 0, 0], 1, 1)?;
    let _tray_icon = TrayIconBuilder::new()
        .with_tooltip("QMeter")
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .build()?;

    let mut last_refresh = Instant::now();
    let refresh_interval = Duration::from_millis(state.settings.refresh_interval_ms.max(5_000));

    loop {
        if let Ok(event) = MenuEvent::receiver().recv_timeout(Duration::from_millis(250)) {
            if event.id == quit_id {
                break;
            } else if event.id == open_id {
                show_popup("QMeter", &state.render_popup_text());
            } else if event.id == refresh_id {
                refresh_state(&mut state, &log_config, true)?;
                last_refresh = Instant::now();
                show_popup("QMeter", &state.render_popup_text());
            } else if event.id == settings_id {
                show_popup("QMeter Settings", &render_settings_text(&state));
            }
        }

        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
                | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => show_popup("QMeter", &state.render_popup_text()),
                _ => {}
            }
        }

        if last_refresh.elapsed() >= refresh_interval {
            if let Err(err) = refresh_state(&mut state, &log_config, false) {
                append_runtime_log(&log_config, "refresh-error", &err.to_string())?;
            }
            last_refresh = Instant::now();
        }
    }

    Ok(())
}

#[cfg(windows)]
fn show_popup(title: &str, body: &str) {
    let _ = rfd::MessageDialog::new()
        .set_title(title)
        .set_description(body)
        .set_level(rfd::MessageLevel::Info)
        .show();
}

fn render_settings_text(state: &TrayState) -> String {
    format!(
        "Refresh interval: {} seconds\nClaude visible: {}\nCodex visible: {}\nNotifications: warning {}%, critical {}%",
        state.settings.refresh_interval_ms / 1000,
        state.settings.visible_providers.claude,
        state.settings.visible_providers.codex,
        state.settings.notification.warning_percent,
        state.settings.notification.critical_percent
    )
}

#[cfg(not(windows))]
fn run_platform_tray(
    _state: TrayState,
    _log_config: RuntimeLogConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("qmeter-tray is only supported on Windows".into())
}
