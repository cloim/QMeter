# Rust Native Port Design

## Goal

Port QMeter from the current Node.js, TypeScript, and Electron implementation to a Rust-native Windows application. The final state removes the Node/Electron runtime from the shipped app and provides:

- a Rust CLI with the current `qmeter` command contract
- a Rust-native Windows tray app
- local provider collection for Claude Code and Codex
- cache, settings, notification, and diagnostics behavior equivalent to the current app

## Current Baseline

The current implementation has two user-facing surfaces over one shared snapshot pipeline:

- CLI: `src/cli.ts` and `src/main.ts`
- shared snapshot orchestration: `src/core/snapshot.ts`
- provider contract: `src/providers/provider.ts`
- Claude provider: `src/providers/claudeProvider.ts`
- Codex provider: `src/providers/codexProvider.ts` and `src/providers/codexAppServer.ts`
- cache: `src/cache.ts`
- output rendering: `src/output.ts`
- tray runtime: `src/tray/main.ts`

The tray runtime is the highest-risk area because it currently combines tray lifecycle, popup rendering, refresh scheduling, settings, IPC, updater behavior, notifications, and inline HTML in one Electron main-process file.

## Selected Approach

Use a Rust workspace with focused crates:

- `qmeter-core`
  - normalized data types
  - snapshot orchestration
  - fixture mode
  - cache path and cache freshness logic
  - table and graph output rendering
  - notification policy and state transitions
- `qmeter-providers`
  - provider trait
  - Claude provider
  - Codex provider
  - provider-specific parsers and process integration
- `qmeter`
  - CLI argument parsing
  - output selection
  - debug diagnostics
  - exit-code contract
- `qmeter-tray`
  - Windows tray icon
  - native popup window
  - settings persistence
  - refresh loop
  - notification dispatch
  - runtime telemetry
  - packaged-app startup behavior

The tray app should use `windows-rs` for Windows integration and `tray-icon` for tray surface where practical. Popup UI should be Rust-native, not Electron or Tauri. If a small UI helper crate is needed, it must not reintroduce a Node or Chromium dependency.

## Compatibility Contract

The Rust port must preserve the current public behavior unless a later task explicitly changes it.

CLI options:

- `--json`
- `--refresh`
- `--debug`
- `--view table|graph`
- `--providers claude,codex,all`
- `-h` and `--help`

Exit codes:

- `0`: all selected providers succeeded
- `1`: partial success
- `2`: usage or argument error
- `3`: total failure

Normalized JSON fields:

- `fetchedAt`
- `rows`
- `errors`
- row fields: `provider`, `window`, `used`, `limit`, `usedPercent`, `resetAt`, `source`, `confidence`, `stale`, `notes`
- error fields: `provider`, `type`, `message`, `actionable`

Known environment variables:

- `USAGE_STATUS_FIXTURE=demo`
- `USAGE_STATUS_BASH_EXE`
- `USAGE_STATUS_CODEX_COMMAND`
- `USAGE_STATUS_CACHE_PATH`
- `USAGE_STATUS_CACHE_TTL_SECS`
- `USAGE_STATUS_TRAY_SETTINGS_PATH`

Default persisted paths:

- cache: `%LOCALAPPDATA%\qmeter\cache.v1.json`
- tray settings: `%APPDATA%\qmeter\tray-settings.v1.json`
- tray runtime log: `%LOCALAPPDATA%\qmeter\tray-runtime.log`

## Provider Design

Providers should implement one Rust trait:

```rust
pub trait Provider {
    fn id(&self) -> ProviderId;
    fn acquire(&self, ctx: AcquireContext) -> anyhow::Result<ProviderResult>;
}
```

`ProviderResult` should contain normalized rows, normalized errors, and optional debug data. Provider panics or unexpected errors must be contained by snapshot orchestration and converted into `unexpected` or provider-specific errors.

### Codex

The first Rust implementation should keep the current structured path:

1. Spawn `codex app-server`.
2. Send JSON-RPC `initialize`.
3. Send `initialized`.
4. Send `account/rateLimits/read`.
5. Prefer `rateLimitsByLimitId.codex` when present.
6. Convert primary and secondary windows into normalized rows.

On Windows, command resolution must preserve the current practical behavior of calling `codex.cmd` through a shell when the command is the default `codex`.

### Claude

Claude remains the brittle provider because it depends on driving a TUI and parsing `/usage` output. The Rust port should first preserve the current behavior:

1. Use the configured bash path from `USAGE_STATUS_BASH_EXE`.
2. Default to `C:/Program Files/Git/usr/bin/bash.exe` on Windows.
3. Launch `claude` through bash.
4. Send `/usage`.
5. Capture terminal output.
6. Strip ANSI/control sequences.
7. Parse session and weekly usage sections.
8. Return timeout or parse errors without crashing the snapshot.

The Rust implementation should isolate terminal driving behind a small module so a future structured Claude provider can replace it without changing core snapshot logic.

## Tray Design

The Rust tray app should preserve the current operational model:

- single running instance
- tray icon and context menu
- popup shown from tray interaction
- popup destroyed or hidden when dismissed
- background refresh continues while popup is not visible
- manual refresh
- provider visibility settings
- refresh interval setting
- threshold notifications
- quiet hours, cooldown, and hysteresis state
- runtime telemetry log

The popup should expose the same practical information as the current tray UI:

- provider rows
- session/week usage bars
- reset labels
- source and stale/cache state
- last checked time
- manual refresh action
- settings action

The design should not introduce an embedded browser runtime. If native UI complexity grows too large, reduce visual ambition before adopting a webview.

## Migration Strategy

The port should be implemented in working checkpoints:

1. Add Rust workspace and core type model.
2. Add fixture snapshot and snapshot tests.
3. Add CLI argument parsing, table output, graph output, JSON output, and exit code tests.
4. Add cache load/save/freshness behavior and tests.
5. Add provider trait and Codex provider tests with process fixtures.
6. Add Claude parser tests from current fixture text.
7. Add Claude process driver behind a testable boundary.
8. Add notification policy/state tests.
9. Add settings persistence tests.
10. Add tray shell and runtime telemetry.
11. Add Windows packaging.
12. Update `.blueprint` docs.
13. Remove Electron/Node runtime after Rust CLI and tray pass verification.

During migration, the existing TypeScript implementation remains the comparison oracle. Remove it only after Rust has equivalent tests and manual smoke coverage.

## Testing Strategy

Use test-first implementation for behavior changes.

Required coverage:

- normalized schema serialization
- fixture snapshot
- provider selection
- cache path override and default path
- cache TTL and stale cache fallback
- table rendering
- graph rendering
- CLI argument errors
- CLI exit codes
- Codex JSON-RPC response parsing
- Claude ANSI cleanup
- Claude `/usage` parsing
- notification threshold transitions
- notification cooldown and quiet hours
- settings validation and defaults
- runtime telemetry formatting

Manual smoke checks:

- `qmeter --json`
- `qmeter --view table`
- `qmeter --view graph`
- `qmeter --providers claude`
- `qmeter --providers codex`
- fixture mode with `USAGE_STATUS_FIXTURE=demo`
- tray startup
- tray popup open/close
- manual refresh
- settings save/reload
- notification threshold behavior with fixture data

## Packaging And Release

The final packaging should be Rust-native. Preferred first target:

- `cargo build --release`
- Windows executable artifacts for CLI and tray
- installer or portable package in a later checkpoint

The existing GitHub release workflow should not be deleted until the Rust packaging workflow is available and verified.

Versioning must remain aligned between the CLI, tray app, and release tags.

## Risks

- Claude TUI automation may be harder in Rust than in Node because the current implementation relies on `node-pty`.
- Native tray popup polish may take more work than the Electron popup.
- Auto-update behavior needs a replacement for `electron-updater`.
- Windows installer behavior needs a replacement for Electron Builder.
- Existing docs include absolute paths from an older repository location and should be cleaned during blueprint updates.

## Non-Goals

- Do not add authentication management.
- Do not store Claude or Codex secrets.
- Do not add a remote backend.
- Do not add long-term analytics or history.
- Do not improve Claude provider semantics beyond parity during the first port.
