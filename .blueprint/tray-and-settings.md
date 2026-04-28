# Tray And Settings

## Tray Entry Point

The Rust tray app starts at [`crates/qmeter-tray/src/main.rs`](../crates/qmeter-tray/src/main.rs).

Runtime implementation lives in:

- [`tray_app.rs`](../crates/qmeter-tray/src/tray_app.rs)
- [`tray_state.rs`](../crates/qmeter-tray/src/tray_state.rs)
- [`runtime_log.rs`](../crates/qmeter-tray/src/runtime_log.rs)
- [`notification_store.rs`](../crates/qmeter-tray/src/notification_store.rs)

## UI Model

The tray creates a Windows tray icon and context menu with:

- `Open QMeter`
- `Refresh`
- `Settings`
- `Quit`

The current popup surface is a native message dialog rendered from the normalized snapshot graph. This keeps the app Rust-native and avoids Electron, Chromium, and unsafe Win32 UI calls.

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
