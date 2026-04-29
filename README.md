# QMeter

English | [한국어](./README.ko.md)

QMeter is a Rust-native Windows tray app and CLI for checking Claude Code and Codex usage, reset timing, cache state, and partial provider failures.

## Screenshot

![QMeter tray overlay](./Screenshot.png)

## Features

- Unified Claude/Codex usage snapshot
- CLI output as table, graph, or JSON
- Windows tray icon with manual refresh and native popup
- Background refresh using persisted tray settings
- Threshold notifications with cooldown, hysteresis, and quiet hours
- Per-provider cache fallback when live collection fails

## Requirements

- Rust stable
- Windows 11 for the tray app
- Claude Code login for Claude usage
- Codex CLI login for Codex usage

## Build And Run

```powershell
cargo test --workspace
cargo run -p qmeter-cli -- --json
cargo run -p qmeter-cli -- --view table
cargo run -p qmeter-cli -- --view graph
cargo check -p qmeter-tray
cargo build -p qmeter-tray
cargo run -p qmeter-tray --bin qmeter-tray
```

Use fixture mode for deterministic local output:

```powershell
$env:USAGE_STATUS_FIXTURE='demo'
cargo run -p qmeter-cli -- --json
cargo run -p qmeter-cli -- --view table
cargo run -p qmeter-cli -- --view graph
```

## CLI

```powershell
cargo run -p qmeter-cli -- --json --providers claude,codex --refresh --debug
```

Options:

- `--json`: print normalized JSON
- `--refresh`: bypass fresh cache
- `--debug`: print provider diagnostics without secrets
- `--view table|graph`: choose terminal rendering
- `--providers claude,codex,all`: select providers

Exit codes:

- `0`: full success
- `1`: partial success
- `2`: argument or usage error
- `3`: total provider failure

## Tray

```powershell
cargo build -p qmeter-tray
cargo run -p qmeter-tray --bin qmeter-tray
```

The tray app:

- loads settings from `%APPDATA%\qmeter\tray-settings.v1.json`
- writes runtime logs to `%LOCALAPPDATA%\qmeter\tray-runtime.log`
- stores notification state at `%LOCALAPPDATA%\qmeter\notification-state.v1.json`
- supports `Open QMeter`, `Refresh`, `Settings`, and `Quit` menu actions
- owns a reusable WebView2 overlay for usage cards and manual refresh

## Provider Notes

Claude usage is collected through Claude Code OAuth credentials and Anthropic's usage endpoint. On Windows/Linux, QMeter reads `~/.claude/.credentials.json`; on macOS it tries the `Claude Code-credentials` Keychain item first.

Codex usage is collected through the Codex app-server JSON-RPC integration.

## Release Binaries

```powershell
cargo build --release --workspace
```

Outputs:

- `target/release/qmeter.exe`
- `target/release/qmeter-tray.exe`

Tag-triggered GitHub Actions builds these binaries and uploads them to the matching GitHub release.

## CI/CD

GitHub Actions is Rust-only:

- `CI` runs on pull requests and pushes to `main`
- CI checks `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --release --workspace --locked`
- `Release` runs on `v*` tags, validates the tag format, builds the same Rust release binaries, zips them, and uploads release assets

## Troubleshooting

- Missing Claude rows usually means Claude Code is not logged in or the OAuth credentials file is unavailable.
- Missing Codex rows usually means Codex CLI is not installed, not logged in, or unavailable on PATH.
- Set `USAGE_STATUS_FIXTURE=demo` to test CLI/tray rendering without touching live providers.
