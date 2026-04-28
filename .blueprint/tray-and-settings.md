# Tray And Settings

## Tray Entry Point

The Rust tray app starts at [`crates/qmeter-tray/src/main.rs`](../crates/qmeter-tray/src/main.rs).

Runtime implementation lives in:

- [`tray_app.rs`](../crates/qmeter-tray/src/tray_app.rs)
- [`tray_state.rs`](../crates/qmeter-tray/src/tray_state.rs)
- [`popup_main.rs`](../crates/qmeter-tray/src/popup_main.rs)
- [`popup_model.rs`](../crates/qmeter-tray/src/popup_model.rs)
- [`runtime_log.rs`](../crates/qmeter-tray/src/runtime_log.rs)
- [`notification_store.rs`](../crates/qmeter-tray/src/notification_store.rs)

## UI Model

The tray creates a Windows tray icon and context menu with:

- `Open QMeter`
- `Refresh`
- `Settings`
- `Quit`

The usage popup is a Rust-native `qmeter-popup.exe` GUI window launched as a sibling process by the tray. It renders normalized snapshot rows as provider cards with progress bars, reset timing, stale state, errors, and a manual refresh action.

Notification and simple settings summaries can still use native message dialogs. The primary usage surface should stay in `qmeter-popup.exe` so the tray event loop remains small and the GUI can own its own window loop.

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
