# History And Roadmap

## Current State

QMeter has been ported to a Rust workspace with native CLI and Windows tray binaries.

Implemented:

- normalized snapshot schema
- fixture snapshots
- cache with stale fallback
- Claude OAuth usage provider
- Codex app-server provider
- CLI table, graph, and JSON output
- tray menu, popup, refresh loop, settings persistence, runtime log, notification state
- Rust release binaries and GitHub release upload workflow

## Remaining Improvements

Likely follow-up work after the Rust-native cutover:

- richer native tray popup UI beyond message dialog
- explicit settings editor instead of settings summary dialog
- native Windows startup registration for `startupEnabled`
- richer release packaging such as installer or portable directory bundle
- richer tray icon asset loading instead of minimal generated icon

These are enhancements, not blockers for removing the Node/Electron implementation.
