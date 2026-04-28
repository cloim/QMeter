use crate::runtime_log::{append_runtime_log, RuntimeLogConfig};
use crate::tray_state::TrayState;

pub fn run_tray_app() -> Result<(), Box<dyn std::error::Error>> {
    append_runtime_log(
        &RuntimeLogConfig::from_env(),
        "startup",
        "qmeter tray starting",
    )?;
    let mut state = TrayState::default();
    state.refresh_fixture();
    let popup_text = state.render_popup_text();
    append_runtime_log(
        &RuntimeLogConfig::from_env(),
        "refresh",
        &format!(
            "rows={} popup_chars={}",
            state.snapshot.as_ref().map_or(0, |s| s.rows.len()),
            popup_text.len()
        ),
    )?;
    run_platform_tray()
}

#[cfg(windows)]
fn run_platform_tray() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Duration;
    use tray_icon::{
        menu::{Menu, MenuEvent, MenuItem},
        Icon, TrayIconBuilder,
    };

    let menu = Menu::new();
    let quit = MenuItem::new("Quit", true, None);
    let quit_id = quit.id().clone();
    menu.append(&quit)?;

    let icon = Icon::from_rgba(vec![0, 0, 0, 0], 1, 1)?;
    let _tray_icon = TrayIconBuilder::new()
        .with_tooltip("QMeter")
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .build()?;

    loop {
        if let Ok(event) = MenuEvent::receiver().recv_timeout(Duration::from_millis(250)) {
            if event.id == quit_id {
                break;
            }
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn run_platform_tray() -> Result<(), Box<dyn std::error::Error>> {
    Err("qmeter-tray is only supported on Windows".into())
}
