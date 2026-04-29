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
- tray menu, reusable WebView2 overlay, refresh skeleton, settings UI, refresh loop, runtime log, notification state
- Rust CI workflow plus tag-driven GitHub release upload workflow

## Remaining Improvements

Likely follow-up work after the Rust-native cutover:

- native Windows startup registration for `startupEnabled`
- richer release packaging such as installer or portable directory bundle
- richer tray icon asset loading instead of minimal generated icon

These are enhancements, not blockers for removing the Node/Electron implementation.
