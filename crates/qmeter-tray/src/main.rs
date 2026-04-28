mod runtime_log;
mod tray_app;

fn main() {
    if let Err(err) = tray_app::run_tray_app() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

