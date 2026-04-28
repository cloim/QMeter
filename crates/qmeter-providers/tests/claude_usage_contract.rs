use qmeter_core::types::NormalizedErrorType;
use qmeter_providers::claude_usage::{clean_claude_screen_text, parse_claude_usage_from_screen};

#[test]
fn cleans_ansi_and_normalizes_whitespace() {
    let clean = clean_claude_screen_text("\u{1b}[31mCurrent session\u{1b}[0m\r\n  90%   used\r\n");

    assert!(clean.contains("Current session"));
    assert!(clean.contains("90% used"));
    assert!(!clean.contains('\u{1b}'));
}

#[test]
fn extracts_session_and_week_rows_when_present() {
    let screen = clean_claude_screen_text(
        r#"
Settings: Usage
Current session
  90% used
  Resets 3am

Current week (all models)
  21% used
  Resets Feb 28, 10am
"#,
    );

    let parsed = parse_claude_usage_from_screen(&screen);

    assert_eq!(parsed.errors, vec![]);
    let session = parsed
        .rows
        .iter()
        .find(|row| row.window == "claude:session")
        .expect("session row");
    let week = parsed
        .rows
        .iter()
        .find(|row| row.window == "claude:week(all-models)")
        .expect("week row");
    assert_eq!(session.used_percent, Some(90.0));
    assert_eq!(week.used_percent, Some(21.0));
}

#[test]
fn anchors_session_parsing_to_section_header() {
    let filler = "Context line with unrelated details.\n".repeat(40);
    let screen = clean_claude_screen_text(&format!(
        r#"
/subagent-driven-development (superpowers) Use when executing implementation plans.
Choose a command to run.
{filler}

Current session
  7% used
  Resets 12pm (Asia/Seoul)

Current week (all models)
  1% used
  Resets Apr 26, 6pm (Asia/Seoul)
"#
    ));

    let parsed = parse_claude_usage_from_screen(&screen);
    let session = parsed
        .rows
        .iter()
        .find(|row| row.window == "claude:session")
        .expect("session row");
    let week = parsed
        .rows
        .iter()
        .find(|row| row.window == "claude:week(all-models)")
        .expect("week row");

    assert_eq!(parsed.errors, vec![]);
    assert_eq!(session.used_percent, Some(7.0));
    assert_eq!(session.notes.as_deref(), Some("Resets 12pm (Asia/Seoul)"));
    assert_eq!(week.used_percent, Some(1.0));
}

#[test]
fn returns_parse_failed_error_when_no_rows_found() {
    let parsed = parse_claude_usage_from_screen("hello world");

    assert_eq!(parsed.rows.len(), 0);
    assert_eq!(parsed.errors.len(), 1);
    assert_eq!(
        parsed.errors[0].error_type,
        NormalizedErrorType::ParseFailed
    );
}
