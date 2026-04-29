# Architecture

## Workspace Layout

QMeter is a Rust workspace:

- [`crates/qmeter-core`](../crates/qmeter-core): normalized types, cache, fixture snapshots, output rendering, settings, scheduler, notification policy
- [`crates/qmeter-providers`](../crates/qmeter-providers): Claude and Codex live acquisition plus shared live snapshot orchestration
- [`crates/qmeter`](../crates/qmeter): command-line interface
- [`crates/qmeter-tray`](../crates/qmeter-tray): Windows tray runtime

## Snapshot Flow

Live collection is centered in [`crates/qmeter-providers/src/snapshot.rs`](../crates/qmeter-providers/src/snapshot.rs).

Flow:

1. Parse CLI args or tray settings into `CollectOptions`.
2. Use fixture mode when `USAGE_STATUS_FIXTURE=demo`.
3. Reuse fresh per-provider cache when allowed.
4. Acquire Claude/Codex live data.
5. Fall back to stale cache rows if a provider fails.
6. Save fresh rows back into cache.
7. Return one `NormalizedSnapshot`.

Cache behavior lives in [`crates/qmeter-core/src/cache.rs`](../crates/qmeter-core/src/cache.rs). The cache stores usage rows only and does not store credentials.

## Providers

The provider trait lives in [`crates/qmeter-providers/src/provider.rs`](../crates/qmeter-providers/src/provider.rs). Providers return normalized rows, normalized errors, and optional debug payloads.

### Claude

[`crates/qmeter-providers/src/claude.rs`](../crates/qmeter-providers/src/claude.rs) reads Claude Code OAuth credentials and calls:

```text
https://api.anthropic.com/api/oauth/usage
```

It maps `five_hour`, `seven_day`, and `seven_day_sonnet` into `claude:5h`, `claude:7d`, and `claude:7d-sonnet`.

Credential lookup:

- Windows/Linux: `~/.claude/.credentials.json`
- macOS: `Claude Code-credentials` Keychain item, then file fallback

### Codex

[`crates/qmeter-providers/src/codex.rs`](../crates/qmeter-providers/src/codex.rs) uses the Codex app-server JSON-RPC path and maps primary/secondary rate limits into normalized rows.

`USAGE_STATUS_CODEX_COMMAND` can override the Codex command path.

## Tray Runtime

[`crates/qmeter-tray/src/tray_app.rs`](../crates/qmeter-tray/src/tray_app.rs) owns tray startup, context menu, background refresh, popup display, and notification dispatch.

Related modules:

- [`runtime_log.rs`](../crates/qmeter-tray/src/runtime_log.rs): `%LOCALAPPDATA%\qmeter\tray-runtime.log`
- [`tray_state.rs`](../crates/qmeter-tray/src/tray_state.rs): visible providers, snapshot state, popup text rendering
- [`popup_overlay.rs`](../crates/qmeter-tray/src/popup_overlay.rs): in-process WebView2 usage overlay, refresh skeleton, and settings UI
- [`notification_store.rs`](../crates/qmeter-tray/src/notification_store.rs): persisted notification cooldown state

The current popup is a reusable in-process WebView2 overlay owned by `qmeter-tray.exe`. It avoids the legacy Node/Electron runtime and does not spawn a separate popup process.

## Settings And Notifications

Tray settings are persisted through [`crates/qmeter-core/src/settings.rs`](../crates/qmeter-core/src/settings.rs).

Default settings path:

```text
%APPDATA%\qmeter\tray-settings.v1.json
```

Notification policy is in [`crates/qmeter-core/src/notification_policy.rs`](../crates/qmeter-core/src/notification_policy.rs). It supports warning/critical thresholds, hysteresis, cooldown, and quiet hours.
