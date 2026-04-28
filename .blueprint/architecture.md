# Architecture

## High-Level Layout

QMeter is being ported from the original TypeScript/Electron implementation to a Rust workspace. During the migration, both implementations may exist in the tree:

- Rust core: [`crates/qmeter-core`](../crates/qmeter-core)
- Rust CLI: [`crates/qmeter-cli`](../crates/qmeter-cli)
- Rust providers: [`crates/qmeter-providers`](../crates/qmeter-providers)
- Rust tray shell: [`crates/qmeter-tray`](../crates/qmeter-tray)
- Legacy TypeScript/Electron implementation: [`src`](../src)

The Rust workspace is the forward path. The legacy implementation remains a parity oracle until provider acquisition, tray UI, packaging, and smoke checks are complete.

## Legacy TypeScript Layout

The codebase is organized around a shared snapshot pipeline with two adapters layered on top.

- CLI entry: [`src/cli.ts`](D:\Code\Vibe\QMeter\src\cli.ts)
- CLI orchestration/output: [`src/main.ts`](D:\Code\Vibe\QMeter\src\main.ts)
- Shared collection core: [`src/core/snapshot.ts`](D:\Code\Vibe\QMeter\src\core\snapshot.ts)
- Provider implementations: [`src/providers`](D:\Code\Vibe\QMeter\src\providers)
- Tray runtime: [`src/tray`](D:\Code\Vibe\QMeter\src\tray)

The practical flow is:

1. Parse CLI args or tray action.
2. Call `collectSnapshot(...)`.
3. Acquire provider data, or fall back to cache when appropriate.
4. Validate against the normalized schema.
5. Render to terminal output or tray UI.

## Shared Snapshot Collection

[`src/core/snapshot.ts`](D:\Code\Vibe\QMeter\src\core\snapshot.ts) is the main integration seam.

Responsibilities:

- Select provider implementations
- Respect fixture mode via `USAGE_STATUS_FIXTURE=demo`
- Reuse cached provider rows when still fresh
- Persist fresh rows back into cache
- Merge rows and errors into one normalized snapshot
- Return optional debug payloads per provider

Important behavior:

- Cache is disabled in fixture mode.
- Cache is per provider, not an all-or-nothing blob.
- If acquisition returns no rows but cached rows exist, stale cache rows are injected with `source: "cache"`.

## Provider Boundary

The provider contract lives in [`src/providers/provider.ts`](D:\Code\Vibe\QMeter\src\providers\provider.ts).

Every provider must:

- Identify itself with `id`
- Implement `acquire(ctx)`
- Return normalized rows and normalized errors

That keeps the core agnostic to provider internals.

### Claude Provider

The Rust Claude provider in [`crates/qmeter-providers/src/claude.rs`](../crates/qmeter-providers/src/claude.rs) uses the Claude Code OAuth credential to call Anthropic's usage endpoint directly.

Important implementation facts:

- The live Rust path reads `claudeAiOauth.accessToken` from Claude Code credentials.
- On Windows and Linux it reads `~/.claude/.credentials.json`.
- On macOS it first tries the `Claude Code-credentials` Keychain item, then falls back to the credentials file.
- It calls `https://api.anthropic.com/api/oauth/usage` with `Authorization: Bearer <token>` and `anthropic-beta: oauth-2025-04-20`.
- It maps `five_hour`, `seven_day`, and `seven_day_sonnet` windows into structured normalized rows.
- The old `/usage` screen parser remains isolated behind a testable runner boundary, but it is no longer the default live Rust acquisition path.
- Failure modes are mapped into normalized error types such as `auth-required`, `timeout`, `invalid-response`, and `acquire-failed`.

The legacy TypeScript provider still uses `node-pty` to drive the Claude TUI and parse `/usage` output until it is retired.

### Codex Provider

[`src/providers/codexProvider.ts`](D:\Code\Vibe\QMeter\src\providers\codexProvider.ts) delegates to the app-server integration in [`src/providers/codexAppServer.ts`](D:\Code\Vibe\QMeter\src\providers\codexAppServer.ts).

Important implementation facts:

- It prefers a structured acquisition path
- It maps operational failures into normalized error types like `not-installed`, `timeout`, `auth-required`, and `acquire-failed`
- It supports command override through `USAGE_STATUS_CODEX_COMMAND`

## Cache Layer

[`src/cache.ts`](D:\Code\Vibe\QMeter\src\cache.ts) owns cache I/O and freshness logic.

Current rules:

- Default path uses `%LOCALAPPDATA%\\qmeter\\cache.v1.json` on Windows
- TTL defaults to 60 seconds
- `USAGE_STATUS_CACHE_PATH` overrides path
- `USAGE_STATUS_CACHE_TTL_SECS` overrides TTL
- Cached rows are rewritten with `source: "cache"` and may be marked stale

The cache stores rows only. It does not store credentials or provider-side secrets.

## Tray Runtime

[`src/tray/main.ts`](D:\Code\Vibe\QMeter\src\tray\main.ts) is a stateful Electron shell around the same snapshot pipeline.

Main responsibilities:

- Single-instance lock
- Tray icon and menu
- Popup window lifecycle
- Refresh loop and manual refresh
- Notification evaluation
- Updater checks for packaged builds
- IPC handlers for renderer actions

Important runtime detail:

- The popup window is created lazily and destroyed on blur so renderer resources are not kept alive when the tray UI is closed.
- The main process remains responsible for background refresh, updater checks, and notifications even when no popup window exists.

The HTML UI is currently generated inline in the main process rather than being maintained as a separate frontend app. That means UI edits often happen inside the large `renderHtml(...)` function in the same file.

## Notifications

Threshold logic lives in [`src/core/notificationPolicy.ts`](D:\Code\Vibe\QMeter\src\core\notificationPolicy.ts) and state transitions in [`src/core/notificationState.ts`](D:\Code\Vibe\QMeter\src\core\notificationState.ts).

The design is transition-aware:

- warning and critical thresholds
- hysteresis to avoid flapping
- cooldown to avoid duplicate alerts
- optional quiet hours

Persisted notification state is handled separately by [`src/tray/notificationStore.ts`](D:\Code\Vibe\QMeter\src\tray\notificationStore.ts).

## Settings

Tray settings are stored as JSON and validated with Zod in [`src/tray/settings.ts`](D:\Code\Vibe\QMeter\src\tray\settings.ts).

The settings file includes:

- startup toggle
- refresh interval
- visible providers
- notification thresholds and quiet hours

The UI currently exposes refresh interval and visible providers. The stored schema already includes richer notification settings than the current popup exposes directly.
