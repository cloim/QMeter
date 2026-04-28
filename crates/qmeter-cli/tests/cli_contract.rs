use assert_cmd::Command;
use serde_json::Value;

fn qmeter() -> Command {
    Command::cargo_bin("qmeter").expect("qmeter binary should build")
}

#[test]
fn help_prints_usage() {
    qmeter()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage:"))
        .stdout(predicates::str::contains("--providers"));
}

#[test]
fn unknown_argument_exits_with_usage_error() {
    qmeter().arg("--wat").assert().code(2);
}

#[test]
fn fixture_json_outputs_normalized_snapshot() {
    let output = qmeter()
        .env("USAGE_STATUS_FIXTURE", "demo")
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("stdout should be JSON");
    assert_eq!(value["rows"].as_array().expect("rows array").len(), 4);
    assert_eq!(value["rows"][0]["window"], "claude:session");
    assert_eq!(value["rows"][2]["window"], "codex:5h");
}

#[test]
fn fixture_table_outputs_existing_headers() {
    qmeter()
        .env("USAGE_STATUS_FIXTURE", "demo")
        .args(["--view", "table"])
        .assert()
        .success()
        .stdout(predicates::str::contains("PROV"))
        .stdout(predicates::str::contains("claude:session"));
}

#[test]
fn fixture_graph_outputs_usage_bars() {
    qmeter()
        .env("USAGE_STATUS_FIXTURE", "demo")
        .args(["--view", "graph"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Claude Session limit"))
        .stdout(predicates::str::contains("79% used"));
}

#[test]
fn selected_provider_filters_rows() {
    let output = qmeter()
        .env("USAGE_STATUS_FIXTURE", "demo")
        .args(["--json", "--providers", "codex"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("stdout should be JSON");
    let rows = value["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row["provider"] == "codex"));
}
