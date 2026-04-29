#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod notification_store;
mod popup_overlay;
mod runtime_log;
mod tray_app;
mod tray_state;

fn main() {
    if let Err(err) = tray_app::run_tray_app() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
