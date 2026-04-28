use qmeter_core::output::render_graph;
use qmeter_core::settings::{TraySettings, default_tray_settings};
use qmeter_core::snapshot::{CollectOptions, collect_fixture_snapshot, is_fixture_mode_from_env};
use qmeter_core::types::{NormalizedSnapshot, ProviderId};
use qmeter_providers::snapshot::collect_live_snapshot;

#[derive(Clone, Debug, PartialEq)]
pub struct TrayState {
    pub snapshot: Option<NormalizedSnapshot>,
    pub last_checked_at: Option<String>,
    pub settings: TraySettings,
}

impl Default for TrayState {
    fn default() -> Self {
        Self::new(default_tray_settings())
    }
}

impl TrayState {
    pub fn new(settings: TraySettings) -> Self {
        Self {
            snapshot: None,
            last_checked_at: None,
            settings,
        }
    }

    pub fn visible_provider_ids(&self) -> Vec<ProviderId> {
        let mut providers = Vec::new();
        if self.settings.visible_providers.claude {
            providers.push(ProviderId::Claude);
        }
        if self.settings.visible_providers.codex {
            providers.push(ProviderId::Codex);
        }
        providers
    }

    #[cfg(test)]
    pub fn refresh_fixture(&mut self) {
        let snapshot = collect_fixture_snapshot(&CollectOptions {
            refresh: false,
            debug: false,
            providers: self.visible_provider_ids(),
        });
        self.apply_snapshot(snapshot);
    }

    pub fn refresh_current_mode(&mut self, refresh: bool) {
        let opts = CollectOptions {
            refresh,
            debug: false,
            providers: self.visible_provider_ids(),
        };
        let snapshot = if is_fixture_mode_from_env() {
            collect_fixture_snapshot(&opts)
        } else {
            collect_live_snapshot(&opts).snapshot
        };
        self.apply_snapshot(snapshot);
    }

    pub fn apply_snapshot(&mut self, snapshot: NormalizedSnapshot) {
        self.last_checked_at = Some(snapshot.fetched_at.clone());
        self.snapshot = Some(snapshot);
    }

    pub fn render_popup_text(&self) -> String {
        let mut lines = Vec::new();
        match &self.snapshot {
            Some(snapshot) => lines.push(render_graph(snapshot)),
            None => lines.push("Usage Snapshot @ pending\n\n(no rows)".to_string()),
        }
        lines.push(format!(
            "Last checked: {}",
            self.last_checked_at.as_deref().unwrap_or("never")
        ));
        if let Some(snapshot) = &self.snapshot {
            if !snapshot.errors.is_empty() {
                lines.push("Errors:".to_string());
                for error in &snapshot.errors {
                    lines.push(format!(
                        "- {} {}: {}",
                        error.provider.as_str(),
                        serde_error_type(&error.error_type),
                        error.message
                    ));
                }
            }
        }
        lines.join("\n")
    }
}

fn serde_error_type(error_type: &qmeter_core::types::NormalizedErrorType) -> String {
    serde_json::to_value(error_type)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("{error_type:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_fixture_populates_snapshot_and_last_checked() {
        let mut state = TrayState::default();

        state.refresh_fixture();

        let snapshot = state.snapshot.as_ref().expect("snapshot");
        assert_eq!(snapshot.rows.len(), 4);
        assert_eq!(
            state.last_checked_at.as_deref(),
            Some("2026-02-24T00:00:00.000Z")
        );
    }

    #[test]
    fn visible_provider_ids_follow_settings() {
        let mut settings = default_tray_settings();
        settings.visible_providers.claude = false;
        settings.visible_providers.codex = true;

        let state = TrayState::new(settings);

        assert_eq!(state.visible_provider_ids(), vec![ProviderId::Codex]);
    }

    #[test]
    fn popup_text_contains_provider_rows() {
        let mut state = TrayState::default();
        state.refresh_fixture();

        let text = state.render_popup_text();

        assert!(text.contains("Claude Session limit"));
        assert!(text.contains("Codex Session limit"));
        assert!(text.contains("Last checked: 2026-02-24T00:00:00.000Z"));
    }
}
