# Tray And Settings

## Tray Entry Point

The Rust tray app starts at [`crates/qmeter-tray/src/main.rs`](../crates/qmeter-tray/src/main.rs).

Runtime implementation lives in:

- [`tray_app.rs`](../crates/qmeter-tray/src/tray_app.rs)
- [`tray_state.rs`](../crates/qmeter-tray/src/tray_state.rs)
- [`popup_overlay.rs`](../crates/qmeter-tray/src/popup_overlay.rs)
- [`runtime_log.rs`](../crates/qmeter-tray/src/runtime_log.rs)
- [`notification_store.rs`](../crates/qmeter-tray/src/notification_store.rs)

## UI Model

The tray creates a Windows tray icon and context menu with:

- `Open QMeter`
- `Refresh`
- `Settings`
- `Quit`

The usage popup is a Rust-native WebView2 overlay owned by the `qmeter-tray.exe` process. The tray keeps one overlay instance alive after first creation, hides/shows the same window on tray clicks, and updates existing WebView content with JavaScript when snapshots change. The tray passes the last tray click position so the overlay opens above the tray area instead of centered on the screen. It renders the legacy HTML/CSS provider cards with progress bars, reset timing, a refresh skeleton, a manual refresh action, and the settings modal.

The popup formats timestamps for local display as `YYYY-MM-DD HH:mm:ss`. Repeated tray clicks must not spawn additional processes or recreate the WebView after the first overlay exists.

Notifications can still use native message dialogs. Usage and settings surfaces should stay inside the tray process so overlay display is immediate and cannot leave orphan popup processes.

## Refresh Loop

On startup, the tray loads settings, collects a snapshot, writes runtime telemetry, and then runs a menu/event loop.

Refresh behavior:

- background refresh uses `refreshIntervalMs`
- manual refresh bypasses fresh cache
- fixture mode uses deterministic rows
- live mode uses the same provider snapshot path as CLI
- refresh errors are written to the runtime log

## Settings Persistence

Settings are defined in [`crates/qmeter-core/src/settings.rs`](../crates/qmeter-core/src/settings.rs).

Default path:

```text
%APPDATA%\qmeter\tray-settings.v1.json
```

Override:

```text
USAGE_STATUS_TRAY_SETTINGS_PATH
```

Stored settings include:

- `startupEnabled`
- `refreshIntervalMs`
- `visibleProviders`
- notification thresholds
- notification cooldown
- hysteresis
- quiet hours

Settings must not contain provider credentials or tokens.

## Notifications

Notification policy is evaluated through [`crates/qmeter-core/src/notification_policy.rs`](../crates/qmeter-core/src/notification_policy.rs).

Rules:

- notify on warning/critical transitions
- re-notify only after cooldown
- use hysteresis to avoid threshold flapping
- respect quiet hours

Persisted notification state path:

```text
%LOCALAPPDATA%\qmeter\notification-state.v1.json
```

Override:

```text
USAGE_STATUS_TRAY_NOTIFICATION_STATE_PATH
```

## Runtime Log

Runtime diagnostics path:

```text
%LOCALAPPDATA%\qmeter\tray-runtime.log
```

The log records startup, settings path, refresh summaries, and refresh errors.
