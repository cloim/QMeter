use std::{collections::BTreeMap, path::Path};

use crate::notification_store::{
    NotificationStoreConfig, load_notification_state, save_notification_state,
};
use crate::runtime_log::{RuntimeLogConfig, append_runtime_log};
use crate::tray_state::TrayState;
use qmeter_core::notification_policy::{
    AlertLevel, NotificationEvent, NotificationPolicyConfig, NotificationState,
    NotificationThresholds, QuietHours, evaluate_notification_policy,
};
use qmeter_core::settings::{TraySettingsConfig, load_tray_settings};

pub fn run_tray_app() -> Result<(), Box<dyn std::error::Error>> {
    let log_config = RuntimeLogConfig::from_env();
    append_runtime_log(&log_config, "startup", "qmeter tray starting")?;
    let settings_config = TraySettingsConfig::from_env();
    let settings = load_tray_settings(&settings_config)?;
    append_runtime_log(
        &log_config,
        "settings",
        &format!("path={}", settings_config.path.display()),
    )?;
    let notification_config = NotificationStoreConfig::from_env();
    let notification_state = load_notification_state(&notification_config)?;
    let mut state = TrayState::new(settings);
    let _ = refresh_state(&mut state, &log_config, false, None)?;
    run_platform_tray(state, log_config, notification_config, notification_state)
}

fn refresh_state(
    state: &mut TrayState,
    log_config: &RuntimeLogConfig,
    force_refresh: bool,
    notification_state: Option<&mut BTreeMap<String, NotificationState>>,
) -> Result<Vec<NotificationEvent>, Box<dyn std::error::Error>> {
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

    let Some(notification_state) = notification_state else {
        return Ok(Vec::new());
    };
    let Some(snapshot) = &state.snapshot else {
        return Ok(Vec::new());
    };
    let evaluation = evaluate_notification_policy(
        &snapshot.rows,
        notification_state,
        &notification_policy_config(state),
        &snapshot.fetched_at,
    );
    *notification_state = evaluation.next_state;
    Ok(evaluation.events)
}

#[cfg(windows)]
fn run_platform_tray(
    mut state: TrayState,
    log_config: RuntimeLogConfig,
    notification_config: NotificationStoreConfig,
    mut notification_state: BTreeMap<String, NotificationState>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
        menu::{Menu, MenuEvent, MenuItem},
    };
    use winit::event::{Event, StartCause};
    use winit::event_loop::{ControlFlow, EventLoop};

    #[derive(Clone, Debug)]
    enum UserEvent {
        Tray(TrayIconEvent),
        Menu(MenuEvent),
    }

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Tray(event));
    }));

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

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
    let mut popup_anchor = None;
    let refresh_interval = Duration::from_millis(state.settings.refresh_interval_ms.max(5_000));

    #[allow(deprecated)]
    {
        event_loop.run(move |event, event_loop| {
            event_loop.set_control_flow(ControlFlow::WaitUntil(last_refresh + refresh_interval));
            match event {
                Event::NewEvents(StartCause::ResumeTimeReached { .. }) | Event::AboutToWait => {
                    if last_refresh.elapsed() >= refresh_interval {
                        match refresh_state(
                            &mut state,
                            &log_config,
                            false,
                            Some(&mut notification_state),
                        ) {
                            Ok(events) => {
                                if let Err(err) = save_notification_state(
                                    &notification_config,
                                    &notification_state,
                                ) {
                                    let _ = append_runtime_log(
                                        &log_config,
                                        "notification-state-error",
                                        &err.to_string(),
                                    );
                                }
                                show_notification_events(&events);
                            }
                            Err(err) => {
                                let _ = append_runtime_log(
                                    &log_config,
                                    "refresh-error",
                                    &err.to_string(),
                                );
                            }
                        }
                        last_refresh = Instant::now();
                    }
                }
                Event::UserEvent(UserEvent::Menu(event)) => {
                    if event.id == quit_id {
                        event_loop.exit();
                    } else if event.id == open_id {
                        open_popup_window(&log_config, popup_anchor);
                    } else if event.id == refresh_id {
                        match refresh_state(
                            &mut state,
                            &log_config,
                            true,
                            Some(&mut notification_state),
                        ) {
                            Ok(events) => {
                                if let Err(err) = save_notification_state(
                                    &notification_config,
                                    &notification_state,
                                ) {
                                    let _ = append_runtime_log(
                                        &log_config,
                                        "notification-state-error",
                                        &err.to_string(),
                                    );
                                }
                                show_notification_events(&events);
                                last_refresh = Instant::now();
                                open_popup_window(&log_config, popup_anchor);
                            }
                            Err(err) => {
                                let _ =
                                    append_runtime_log(&log_config, "menu-error", &err.to_string());
                            }
                        }
                    } else if event.id == settings_id {
                        show_popup("QMeter Settings", &render_settings_text(&state));
                    }
                }
                Event::UserEvent(UserEvent::Tray(event)) => {
                    if let Some(anchor) = tray_event_position(&event) {
                        popup_anchor = Some(anchor);
                    }
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            position,
                            ..
                        } => open_popup_window(&log_config, Some((position.x, position.y))),
                        TrayIconEvent::DoubleClick {
                            button: MouseButton::Left,
                            position,
                            ..
                        } => open_popup_window(&log_config, Some((position.x, position.y))),
                        _ => {}
                    }
                }
                _ => {}
            }
        })?;
    }

    Ok(())
}

#[cfg(windows)]
fn open_popup_window(log_config: &RuntimeLogConfig, anchor: Option<(f64, f64)>) {
    let popup_path = match std::env::current_exe() {
        Ok(current_exe) => popup_exe_path(&current_exe),
        Err(err) => {
            let _ = append_runtime_log(log_config, "popup-error", &err.to_string());
            return;
        }
    };

    let mut command = std::process::Command::new(&popup_path);
    if let Some((x, y)) = anchor {
        command
            .env("QMETER_POPUP_ANCHOR_X", x.to_string())
            .env("QMETER_POPUP_ANCHOR_Y", y.to_string());
    }

    if let Err(err) = command.spawn() {
        let _ = append_runtime_log(
            log_config,
            "popup-error",
            &format!("{}: {err}", popup_path.display()),
        );
    }
}

fn popup_exe_path(current_exe: &Path) -> std::path::PathBuf {
    current_exe.with_file_name(format!("qmeter-popup{}", std::env::consts::EXE_SUFFIX))
}

#[cfg(windows)]
fn tray_event_position(event: &tray_icon::TrayIconEvent) -> Option<(f64, f64)> {
    match event {
        tray_icon::TrayIconEvent::Click { position, .. }
        | tray_icon::TrayIconEvent::DoubleClick { position, .. }
        | tray_icon::TrayIconEvent::Enter { position, .. }
        | tray_icon::TrayIconEvent::Move { position, .. }
        | tray_icon::TrayIconEvent::Leave { position, .. } => Some((position.x, position.y)),
        _ => None,
    }
}

#[cfg(windows)]
fn show_notification_events(events: &[NotificationEvent]) {
    for event in events {
        let title = match event.level {
            AlertLevel::Normal => continue,
            AlertLevel::Warning => "QMeter warning",
            AlertLevel::Critical => "QMeter critical",
        };
        let percent = event
            .row
            .used_percent
            .map(|value| format!("{value:.0}%"))
            .unwrap_or_else(|| "?".to_string());
        let reset = event.row.reset_at.as_deref().unwrap_or("unknown reset");
        show_popup(
            title,
            &format!(
                "{} {} reached {} ({})",
                event.row.provider.as_str(),
                event.row.window,
                percent,
                reset
            ),
        );
    }
}

#[cfg(windows)]
fn show_popup(title: &str, body: &str) {
    let _ = rfd::MessageDialog::new()
        .set_title(title)
        .set_description(body)
        .set_level(rfd::MessageLevel::Info)
        .show();
}

fn notification_policy_config(state: &TrayState) -> NotificationPolicyConfig {
    NotificationPolicyConfig {
        thresholds: NotificationThresholds {
            warning_percent: state.settings.notification.warning_percent,
            critical_percent: state.settings.notification.critical_percent,
        },
        cooldown_ms: state.settings.notification.cooldown_minutes * 60_000,
        hysteresis_percent: state.settings.notification.hysteresis_percent,
        quiet_hours: QuietHours {
            enabled: state.settings.notification.quiet_hours.enabled,
            start_hour: state.settings.notification.quiet_hours.start_hour,
            end_hour: state.settings.notification.quiet_hours.end_hour,
        },
    }
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

#[cfg(all(test, windows))]
mod tests {
    use super::popup_exe_path;
    use std::path::Path;

    #[test]
    fn popup_exe_path_uses_sibling_binary() {
        let current = Path::new(r"C:\tools\qmeter-tray.exe");
        assert_eq!(
            popup_exe_path(current),
            Path::new(r"C:\tools\qmeter-popup.exe")
        );
    }
}

#[cfg(not(windows))]
fn run_platform_tray(
    _state: TrayState,
    _log_config: RuntimeLogConfig,
    _notification_config: NotificationStoreConfig,
    _notification_state: BTreeMap<String, NotificationState>,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("qmeter-tray is only supported on Windows".into())
}
