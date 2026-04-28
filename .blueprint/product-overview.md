# Product Overview

## Purpose

QMeter is a Windows-focused usage monitor for Claude Code and Codex. It exposes the same core usage snapshot through two user-facing entry points:

- A CLI command, `qmeter`, for scripts and terminal use.
- A Windows tray app that shows a popup UI, background refresh, notifications, and packaged-app update checks.

The repository currently carries the legacy TypeScript/Electron app and an in-progress Rust workspace that is the forward path for native binaries.

## Supported Runtime

- Rust stable for the native workspace
- Node.js 20+ for the legacy TypeScript/Electron implementation
- Windows 11 for the tray app
- Electron for packaged tray builds

The CLI can still run outside the tray app, but several provider assumptions are Windows-oriented:

- Rust Claude collection expects Claude Code OAuth credentials and calls the Anthropic OAuth usage endpoint directly.
- Legacy TypeScript Claude collection expects a PTY-capable environment and, on Windows, defaults to Git Bash.
- Codex collection relies on the Codex CLI/app-server path being locally available.

## Core User Flows

### CLI

The CLI is implemented in [`src/cli.ts`](D:\Code\Vibe\QMeter\src\cli.ts) and [`src/main.ts`](D:\Code\Vibe\QMeter\src\main.ts).

It supports:

- Table output by default
- Graph output with `--view graph`
- JSON output with `--json`
- Cache bypass with `--refresh`
- Debug diagnostics with `--debug`
- Provider selection with `--providers claude,codex,all`

Exit codes are meaningful:

- `0`: full success
- `1`: partial success
- `2`: argument or usage error
- `3`: total failure

### Tray App

The tray app is implemented from [`src/tray/main.ts`](D:\Code\Vibe\QMeter\src\tray\main.ts).

It provides:

- Tray icon with popup window
- Manual refresh
- Configurable refresh interval
- Provider visibility toggles
- Threshold-based notifications
- Auto-update checks for packaged builds only

## What The Snapshot Represents

The shared data contract is defined in [`src/types.ts`](D:\Code\Vibe\QMeter\src\types.ts).

Each usage row includes:

- `provider`
- `window`
- `used`, `limit`, `usedPercent`
- `resetAt`
- `source`
- `confidence`
- `stale`
- `notes`

This contract is the common language between providers, cache, CLI output, tests, and tray UI.

## Non-Goals And Constraints

Based on the current code and `.sisyphus` plans, this repository intentionally does not do the following:

- Manage authentication or sign-in flows for Claude/Codex
- Store secrets or tokens in cache/settings files
- Depend on a remote backend service
- Provide long-term usage history or dashboard analytics

The design favors local-only collection, explicit partial-failure handling, and compact operational tooling.
