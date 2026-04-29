# Product Overview

## Purpose

QMeter is a Rust-native local usage monitor for Claude Code and Codex. It exposes one normalized snapshot through:

- `qmeter`, a CLI for terminal and scripts
- `qmeter-tray`, a Windows tray app for background refresh, native popup display, settings, and notifications

## Supported Runtime

- Rust stable
- Windows 11 for the tray surface
- Claude Code credentials for Claude usage
- Codex CLI/app-server availability for Codex usage

There is no Node/Electron runtime requirement in the Rust-native app.

## Core User Flows

### CLI

The CLI entry point is [`crates/qmeter/src/main.rs`](../crates/qmeter/src/main.rs).

It supports:

- table output by default
- graph output with `--view graph`
- JSON output with `--json`
- cache bypass with `--refresh`
- debug diagnostics with `--debug`
- provider selection with `--providers claude,codex,all`

Exit codes:

- `0`: full success
- `1`: partial success
- `2`: argument or usage error
- `3`: total provider failure

### Tray App

The tray app entry point is [`crates/qmeter-tray/src/main.rs`](../crates/qmeter-tray/src/main.rs).

It provides:

- Windows tray icon and context menu
- native popup display for the current snapshot
- manual refresh
- configurable refresh interval
- provider visibility settings loaded from disk
- threshold notifications with persisted cooldown state

## Snapshot Contract

The shared data contract is defined in [`crates/qmeter-core/src/types.rs`](../crates/qmeter-core/src/types.rs).

Each row includes:

- `provider`
- `window`
- `used`, `limit`, `usedPercent`
- `resetAt`
- `source`
- `confidence`
- `stale`
- `notes`

The contract is shared by providers, cache, CLI output, tray state, tests, and release artifacts.

## Non-Goals

QMeter intentionally does not:

- manage Claude/Codex sign-in
- store provider credentials or tokens in cache/settings files
- depend on a remote backend
- provide long-term usage analytics
