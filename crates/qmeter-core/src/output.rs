use crate::types::{NormalizedError, NormalizedSnapshot};

fn pad_right(value: &str, width: usize) -> String {
    if value.len() >= width {
        return value.to_string();
    }
    format!("{value}{}", " ".repeat(width - value.len()))
}

fn clamp_text(value: &str, width: usize) -> String {
    if value.len() <= width {
        return value.to_string();
    }
    if width <= 1 {
        return value[..width].to_string();
    }
    format!("{}.", &value[..width - 1])
}

fn render_errors(snapshot: &NormalizedSnapshot) -> Vec<String> {
    if snapshot.errors.is_empty() {
        return Vec::new();
    }

    let mut lines = vec![String::new(), "Errors:".to_string()];
    for err in &snapshot.errors {
        lines.push(render_error(err));
    }
    lines
}

fn render_error(err: &NormalizedError) -> String {
    let action = err
        .actionable
        .as_ref()
        .map(|a| format!(" (next: {a})"))
        .unwrap_or_default();
    format!(
        "- {}: {:?}: {}{}",
        err.provider.as_str(),
        err.error_type,
        err.message,
        action
    )
}

fn usage_label(percent: Option<f64>, used: Option<u64>, limit: Option<u64>) -> String {
    if let Some(percent) = percent {
        return format!("{}%", percent.round() as i64);
    }
    match (used, limit) {
        (Some(used), Some(limit)) => format!("{used}/{limit}"),
        _ => "?".to_string(),
    }
}

fn pretty_window_title(provider: &str, window: &str) -> String {
    match (provider, window) {
        ("claude", "claude:session") => "Claude Session limit".to_string(),
        ("claude", "claude:week(all-models)") => "Claude Week limit".to_string(),
        ("codex", "codex:5h") => "Codex Session limit".to_string(),
        ("codex", "codex:weekly") => "Codex Week limit".to_string(),
        _ => window.to_string(),
    }
}

fn make_bar(percent: f64, width: usize) -> String {
    let bounded = percent.round().clamp(0.0, 100.0);
    let filled = ((bounded / 100.0) * width as f64).round() as usize;
    format!("{}{}", "#".repeat(filled), ".".repeat(width - filled))
}

pub fn render_graph(snapshot: &NormalizedSnapshot) -> String {
    let mut lines = vec![
        format!("Usage Snapshot @ {}", snapshot.fetched_at),
        String::new(),
    ];

    if snapshot.rows.is_empty() {
        lines.push("(no rows)".to_string());
    } else {
        for row in &snapshot.rows {
            let provider = row.provider.as_str();
            lines.push(pretty_window_title(provider, &row.window));
            if let Some(percent) = row.used_percent {
                let cache_tag = if row.source.is_cache() || row.stale {
                    " (cache)"
                } else {
                    ""
                };
                lines.push(format!(
                    "  {}  {}% used{}",
                    make_bar(percent, 32),
                    percent.round() as i64,
                    cache_tag
                ));
            } else {
                lines.push(format!(
                    "  {} used",
                    usage_label(row.used_percent, row.used, row.limit)
                ));
            }
            lines.push(format!(
                "  Resets {}",
                row.reset_at.as_deref().unwrap_or("unknown")
            ));
            lines.push(format!(
                "  Source {}/{}{}",
                row.source.as_str(),
                row.confidence.as_str(),
                if row.stale { "/stale" } else { "" }
            ));
            lines.push(String::new());
        }
        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }
    }

    lines.extend(render_errors(snapshot));
    lines.join("\n")
}

pub fn render_table(snapshot: &NormalizedSnapshot) -> String {
    let cols = (6usize, 16usize, 18usize, 25usize, 18usize);
    let header = [
        pad_right("PROV", cols.0),
        pad_right("WINDOW", cols.1),
        pad_right("USAGE", cols.2),
        pad_right("RESET_AT", cols.3),
        pad_right("META", cols.4),
    ]
    .join(" ");
    let mut lines = vec![header.clone(), "-".repeat(header.len())];

    if snapshot.rows.is_empty() {
        lines.push("(no rows)".to_string());
    } else {
        for row in &snapshot.rows {
            let cache_tag = if row.source.is_cache() || row.stale {
                " (cache)"
            } else {
                ""
            };
            let usage = format!(
                "{}{}",
                usage_label(row.used_percent, row.used, row.limit),
                cache_tag
            );
            let meta = format!(
                "{}/{}{}",
                row.source.as_str(),
                row.confidence.as_str(),
                if row.stale { "/stale" } else { "" }
            );
            lines.push(
                [
                    pad_right(&clamp_text(row.provider.as_str(), cols.0), cols.0),
                    pad_right(&clamp_text(&row.window, cols.1), cols.1),
                    pad_right(&clamp_text(&usage, cols.2), cols.2),
                    pad_right(
                        &clamp_text(row.reset_at.as_deref().unwrap_or("?"), cols.3),
                        cols.3,
                    ),
                    pad_right(&clamp_text(&meta, cols.4), cols.4),
                ]
                .join(" "),
            );
        }
    }

    lines.extend(render_errors(snapshot));
    lines.join("\n")
}
