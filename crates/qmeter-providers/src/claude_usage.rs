use std::sync::OnceLock;

use qmeter_core::types::{
    Confidence, NormalizedError, NormalizedErrorType, NormalizedRow, ProviderId, SourceKind,
};
use regex::Regex;

#[derive(Clone, Debug, PartialEq)]
pub struct ClaudeUsageParseResult {
    pub rows: Vec<NormalizedRow>,
    pub errors: Vec<NormalizedError>,
}

pub fn clean_claude_screen_text(raw: &str) -> String {
    normalize_whitespace(&strip_ansi(raw))
}

fn ansi_regex() -> &'static Regex {
    static ANSI: OnceLock<Regex> = OnceLock::new();
    ANSI.get_or_init(|| Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07]*(?:\x07|\x1b\\)").unwrap())
}

fn strip_ansi(raw: &str) -> String {
    ansi_regex().replace_all(raw, "").into_owned()
}

fn normalize_whitespace(raw: &str) -> String {
    let normalized = raw
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{00a0}', " ");

    let mut out = String::with_capacity(normalized.len());
    let mut last_space = false;
    let mut blank_lines = 0usize;

    for ch in normalized.chars() {
        match ch {
            ' ' | '\t' => {
                if !last_space {
                    out.push(' ');
                    last_space = true;
                }
            }
            '\n' => {
                if out.ends_with(' ') {
                    out.pop();
                }
                blank_lines += 1;
                if blank_lines <= 2 {
                    out.push('\n');
                }
                last_space = false;
            }
            _ => {
                blank_lines = 0;
                last_space = false;
                out.push(ch);
            }
        }
    }

    out
}

pub fn parse_claude_usage_from_screen(screen_text: &str) -> ClaudeUsageParseResult {
    let mut rows = Vec::new();
    let mut errors = Vec::new();

    if let Some(row) = parse_section(
        screen_text,
        "Current session",
        "claude:session",
    ) {
        rows.push(row);
    }

    if let Some(row) = parse_section(
        screen_text,
        "Current week (all models)",
        "claude:week(all-models)",
    ) {
        rows.push(row);
    }

    if rows.is_empty() {
        errors.push(NormalizedError {
            provider: ProviderId::Claude,
            error_type: NormalizedErrorType::ParseFailed,
            message: "Failed to extract usage from /usage screen output".to_string(),
            actionable: Some("run `claude`, run /usage, and ensure you are logged in".to_string()),
        });
    }

    ClaudeUsageParseResult { rows, errors }
}

fn parse_section(screen_text: &str, header: &str, window: &str) -> Option<NormalizedRow> {
    let start = find_section_header(screen_text, header)?;
    let slice_end = (start + 1200).min(screen_text.len());
    let section = &screen_text[start..slice_end];
    let used_percent = parse_percent(section)?;
    let notes = parse_reset_line(section);

    Some(NormalizedRow {
        provider: ProviderId::Claude,
        window: window.to_string(),
        used: None,
        limit: None,
        used_percent: Some(used_percent),
        reset_at: None,
        source: SourceKind::Parsed,
        confidence: Confidence::Medium,
        stale: false,
        notes,
    })
}

fn find_section_header(screen_text: &str, header: &str) -> Option<usize> {
    let mut offset = 0usize;
    for line in screen_text.split_inclusive('\n') {
        if line.trim_end_matches('\n').trim() == header {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

fn parse_percent(section: &str) -> Option<f64> {
    static PERCENT: OnceLock<Regex> = OnceLock::new();
    let re = PERCENT.get_or_init(|| Regex::new(r"(?i)(\d{1,3})%\s*used").unwrap());
    let captures = re.captures(section)?;
    let value = captures.get(1)?.as_str().parse::<f64>().ok()?;
    (0.0..=100.0).contains(&value).then_some(value)
}

fn parse_reset_line(section: &str) -> Option<String> {
    section
        .lines()
        .map(str::trim)
        .find(|line| line.to_ascii_lowercase().starts_with("resets "))
        .map(ToOwned::to_owned)
}
