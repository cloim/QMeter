use qmeter_core::types::{Confidence, NormalizedError, NormalizedSnapshot, SourceKind};

#[derive(Clone, Debug, PartialEq)]
pub struct PopupModel {
    pub fetched_at: String,
    pub rows: Vec<PopupRow>,
    pub errors: Vec<NormalizedError>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PopupRow {
    pub provider: String,
    pub title: String,
    pub used_percent: f64,
    pub reset_at: Option<String>,
    pub meta: String,
    pub stale: bool,
}

pub fn popup_model_from_snapshot(snapshot: &NormalizedSnapshot) -> PopupModel {
    PopupModel {
        fetched_at: snapshot.fetched_at.clone(),
        rows: snapshot.rows.iter().map(PopupRow::from).collect(),
        errors: snapshot.errors.clone(),
    }
}

impl From<&qmeter_core::types::NormalizedRow> for PopupRow {
    fn from(row: &qmeter_core::types::NormalizedRow) -> Self {
        Self {
            provider: row.provider.as_str().to_string(),
            title: title_for_window(&row.window),
            used_percent: row.used_percent.unwrap_or(0.0).clamp(0.0, 100.0),
            reset_at: row.reset_at.clone(),
            meta: format!(
                "{} / {}{}",
                source_label(row.source),
                confidence_label(row.confidence),
                if row.stale { " / stale" } else { "" }
            ),
            stale: row.stale,
        }
    }
}

fn title_for_window(window: &str) -> String {
    match window {
        "claude:5h" => "Claude 5h".to_string(),
        "claude:7d" => "Claude 7d".to_string(),
        "claude:7d-sonnet" => "Claude 7d Sonnet".to_string(),
        "claude:session" => "Claude Session".to_string(),
        "claude:week(all-models)" => "Claude Week".to_string(),
        "codex:5h" => "Codex 5h".to_string(),
        "codex:weekly" => "Codex Weekly".to_string(),
        _ => window.to_string(),
    }
}

fn source_label(source: SourceKind) -> &'static str {
    match source {
        SourceKind::Structured => "live",
        SourceKind::Parsed => "parsed",
        SourceKind::Cache => "cache",
    }
}

fn confidence_label(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::High => "high",
        Confidence::Medium => "medium",
        Confidence::Low => "low",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qmeter_core::types::{NormalizedRow, ProviderId};

    #[test]
    fn popup_model_maps_snapshot_rows_to_user_labels() {
        let snapshot = NormalizedSnapshot {
            fetched_at: "2026-04-29T00:00:00.000Z".to_string(),
            rows: vec![NormalizedRow {
                provider: ProviderId::Claude,
                window: "claude:5h".to_string(),
                used: None,
                limit: None,
                used_percent: Some(42.4),
                reset_at: Some("2026-04-29T05:00:00.000Z".to_string()),
                source: SourceKind::Structured,
                confidence: Confidence::High,
                stale: false,
                notes: None,
            }],
            errors: Vec::new(),
        };

        let model = popup_model_from_snapshot(&snapshot);

        assert_eq!(model.rows[0].title, "Claude 5h");
        assert_eq!(model.rows[0].used_percent, 42.4);
        assert_eq!(model.rows[0].meta, "live / high");
    }
}
