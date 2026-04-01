# Tray And Settings

## Tray Entry Point

The tray application starts from [`src/tray/main.ts`](D:\Code\Vibe\QMeter\src\tray\main.ts).

This file currently owns:

- Electron app bootstrap
- single-instance lock
- BrowserWindow creation
- Tray creation and context menu
- updater event wiring
- refresh scheduling
- notification dispatch
- inline popup HTML and renderer-side script
- IPC registration

Because all of that is in one file, changes here need extra care. A small UI edit can accidentally affect updater or lifecycle behavior if the file is patched casually.

## Window And UI Model

The popup is a frameless `BrowserWindow` whose HTML is generated inline by `renderHtml(...)`.

Current lifecycle behavior:

- the tray popup window is created lazily on demand
- when the popup loses focus, the window is destroyed rather than kept hidden
- background refresh and notifications continue in the main process even when no popup window exists

Current UI characteristics:

- dark card-based popup
- one provider card per visible provider
- session/week progress bars
- inline settings modal
- last-checked timestamp
- manual refresh button

The UI is not currently a separate React/Vite app. There is no standalone renderer build step.

## IPC Surface

The preload bridge and IPC handlers currently support snapshot/settings operations.

Relevant files:

- [`src/tray/preload.ts`](D:\Code\Vibe\QMeter\src\tray\preload.ts)
- [`src/tray/main.ts`](D:\Code\Vibe\QMeter\src\tray\main.ts)

Handlers include:

- `tray:get-snapshot`
- `tray:refresh`
- `tray:get-settings`
- `tray:save-settings`
- `tray:set-height`

Important caveat: the current BrowserWindow config uses `nodeIntegration: true` and `contextIsolation: false`. That is functional for the current implementation, but it is not a hardened Electron security posture. If security tightening work starts, this is one of the first places to revisit.

## Settings Persistence

Settings are defined and validated in [`src/tray/settings.ts`](D:\Code\Vibe\QMeter\src\tray\settings.ts).

Default persisted fields include:

- `startupEnabled`
- `refreshIntervalMs`
- `visibleProviders`
- notification thresholds
- cooldown
- hysteresis
- quiet hours

Default storage location:

- Windows: `%APPDATA%\\qmeter\\tray-settings.v1.json`
- Override: `USAGE_STATUS_TRAY_SETTINGS_PATH`

The code is designed so settings files should contain preferences only, not provider credentials or tokens.

## Notifications

Notification behavior uses the evaluated policy from [`src/core/notificationPolicy.ts`](D:\Code\Vibe\QMeter\src\core\notificationPolicy.ts) and persistent state from [`src/tray/notificationStore.ts`](D:\Code\Vibe\QMeter\src\tray\notificationStore.ts).

Operational rules:

- Alert only on threshold transitions or cooldown re-eligibility
- Respect quiet hours when enabled
- Persist enough state to suppress duplicate alerts across refreshes

When touching notification behavior, confirm both the policy evaluation and the tray dispatch path.

## Runtime Diagnostics

Tray runtime diagnostics are written to:

- Windows: `%LOCALAPPDATA%\\qmeter\\tray-runtime.log`

The runtime log is intended to capture more than ordinary JS exceptions. Current diagnostics include:

- guarded task failures
- updater state transitions
- refresh summaries with memory usage
- renderer and child-process gone events
- process exit and quit-time memory snapshots

## Updater Behavior

Auto-update uses `electron-updater` inside [`src/tray/main.ts`](D:\Code\Vibe\QMeter\src\tray\main.ts).

Current rules:

- Updater checks run only when `app.isPackaged` is true
- Manual checks show explicit user notifications
- Background checks stay quieter
- Downloaded updates are installed on app quit

`.sisyphus/notepads/windows-tray-full-version/issues.md` notes that development mode cannot exercise real auto-update behavior, so the code must keep a clear manual message for that case.

`.sisyphus/notepads/windows-tray-full-version/learnings.md` records two important implementation lessons:

- Manual checks should rely on `checkForUpdates()` plus event handling so “up to date” is shown explicitly.
- Background checks should suppress noisy notifications even though they share the same updater events.

## Startup And Packaging

The settings schema includes `startupEnabled`, but startup management is only meaningful in packaged Windows app flows.

Packaging expectations are documented in the README and implemented via Electron Builder. Before changing tray boot or updater logic, consider whether the behavior differs between:

- local dev run
- unpacked build
- installed packaged app

Those environments do not behave identically.
