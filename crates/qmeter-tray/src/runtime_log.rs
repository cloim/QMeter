use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;

use chrono::{SecondsFormat, Utc};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeLogConfig {
    pub path: PathBuf,
}

impl RuntimeLogConfig {
    pub fn from_env() -> Self {
        Self::from_values(
            std::env::var("LOCALAPPDATA").ok().as_deref(),
            std::env::var("USERPROFILE").ok().map(PathBuf::from),
        )
    }

    pub fn from_values(local_app_data: Option<&str>, home_dir: Option<PathBuf>) -> Self {
        let base = local_app_data
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .or(home_dir.map(|home| home.join("AppData").join("Local")))
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            path: base.join("qmeter").join("tray-runtime.log"),
        }
    }
}

pub fn format_runtime_log_line(event: &str, detail: &str) -> String {
    format!(
        "{} [{}] {}",
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        event,
        detail
    )
}

pub fn append_runtime_log(config: &RuntimeLogConfig, event: &str, detail: &str) -> io::Result<()> {
    if let Some(parent) = config.path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.path)?;
    writeln!(file, "{}", format_runtime_log_line(event, detail))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_log_path_uses_localappdata() {
        let config = RuntimeLogConfig::from_values(
            Some("C:\\Users\\me\\AppData\\Local"),
            Some(PathBuf::from("C:\\Users\\me")),
        );

        assert_eq!(
            config.path,
            PathBuf::from("C:\\Users\\me\\AppData\\Local")
                .join("qmeter")
                .join("tray-runtime.log")
        );
    }

    #[test]
    fn appends_runtime_log_line() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config = RuntimeLogConfig {
            path: dir.path().join("tray-runtime.log"),
        };

        append_runtime_log(&config, "startup", "ok").expect("append log");
        let raw = std::fs::read_to_string(config.path).expect("read log");

        assert!(raw.contains("[startup] ok"));
    }
}
