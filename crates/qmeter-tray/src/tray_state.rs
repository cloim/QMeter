use qmeter_core::output::render_graph;
use qmeter_core::snapshot::{CollectOptions, collect_fixture_snapshot};
use qmeter_core::types::{NormalizedSnapshot, ProviderId};

#[derive(Clone, Debug, PartialEq)]
pub struct TrayState {
    pub snapshot: Option<NormalizedSnapshot>,
    pub last_checked_at: Option<String>,
    pub visible_providers: Vec<ProviderId>,
}

impl Default for TrayState {
    fn default() -> Self {
        Self {
            snapshot: None,
            last_checked_at: None,
            visible_providers: vec![ProviderId::Claude, ProviderId::Codex],
        }
    }
}

impl TrayState {
    pub fn refresh_fixture(&mut self) {
        let snapshot = collect_fixture_snapshot(&CollectOptions {
            refresh: false,
            debug: false,
            providers: self.visible_providers.clone(),
        });
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
        lines.join("\n")
    }
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
    fn popup_text_contains_provider_rows() {
        let mut state = TrayState::default();
        state.refresh_fixture();

        let text = state.render_popup_text();

        assert!(text.contains("Claude Session limit"));
        assert!(text.contains("Codex Session limit"));
        assert!(text.contains("Last checked: 2026-02-24T00:00:00.000Z"));
    }
}
