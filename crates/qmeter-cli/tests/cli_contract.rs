use assert_cmd::Command;
use serde_json::Value;
use std::fs;

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

#[test]
fn non_fixture_mode_reports_provider_gap_instead_of_demo_rows() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache_path = dir.path().join("cache.v1.json");
    let output = qmeter()
        .env_remove("USAGE_STATUS_FIXTURE")
        .env("USAGE_STATUS_CACHE_PATH", &cache_path)
        .args(["--json", "--providers", "claude"])
        .assert()
        .code(3)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("stdout should be JSON");
    assert_eq!(value["rows"].as_array().expect("rows array").len(), 0);
    assert_eq!(value["errors"][0]["provider"], "claude");
    assert_eq!(value["errors"][0]["type"], "tty-unavailable");
}

#[test]
fn non_fixture_codex_uses_live_provider_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache_path = dir.path().join("cache.v1.json");
    let output = qmeter()
        .env_remove("USAGE_STATUS_FIXTURE")
        .env("USAGE_STATUS_CACHE_PATH", &cache_path)
        .env(
            "USAGE_STATUS_CODEX_COMMAND",
            "definitely-missing-qmeter-codex-command.exe",
        )
        .args(["--json", "--providers", "codex"])
        .assert()
        .code(3)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("stdout should be JSON");
    assert_eq!(value["rows"].as_array().expect("rows array").len(), 0);
    assert_eq!(value["errors"][0]["provider"], "codex");
    assert_eq!(value["errors"][0]["type"], "not-installed");
}

#[test]
fn live_provider_failure_falls_back_to_stale_cache_rows() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache_path = dir.path().join("cache.v1.json");
    fs::write(
        &cache_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "version": 1,
            "savedAt": "2026-04-28T00:00:00.000Z",
            "providers": {
                "claude": null,
                "codex": {
                    "fetchedAt": "2026-04-28T00:00:00.000Z",
                    "rows": [{
                        "provider": "codex",
                        "window": "codex:5h",
                        "used": null,
                        "limit": null,
                        "usedPercent": 44.0,
                        "resetAt": null,
                        "source": "structured",
                        "confidence": "high",
                        "stale": false,
                        "notes": null
                    }]
                }
            }
        }))
        .expect("cache json"),
    )
    .expect("write cache");

    let output = qmeter()
        .env_remove("USAGE_STATUS_FIXTURE")
        .env("USAGE_STATUS_CACHE_PATH", &cache_path)
        .env("USAGE_STATUS_CACHE_TTL_SECS", "0")
        .env(
            "USAGE_STATUS_CODEX_COMMAND",
            "definitely-missing-qmeter-codex-command.exe",
        )
        .args(["--json", "--providers", "codex"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("stdout should be JSON");
    assert_eq!(value["rows"][0]["source"], "cache");
    assert_eq!(value["rows"][0]["stale"], true);
    assert_eq!(value["errors"][0]["type"], "not-installed");
}
