# History And Roadmap

## Historical Context

Historical planning notes that previously lived in `.sisyphus` have been removed from the repository workspace.

This file now serves as the lightweight summary of the decisions and caveats that still matter for current work.

## What Was Implemented From The Plans

The current code clearly reflects several plan decisions:

- Shared normalized snapshot schema across CLI and tray
- Distinct Claude and Codex provider implementations
- Cache with stale-row fallback
- Partial-failure aware snapshot output
- Tray refresh loop and notification policy
- Packaged-app-only updater behavior

These outcomes reflect the major design direction that shaped the current codebase.

## Key Recorded Decisions

- Release automation should continue using the existing `npm run tray:pack` path.
- GitHub Actions should inject the token for publishing rather than introducing a new publish script.

These decisions matter when adjusting release automation: preserve the current packaging entry point unless there is a deliberate migration.

## Known Operational Caveats

- Auto-update cannot be validated in ordinary dev mode because `app.isPackaged` is false.
- TypeScript verification relies on `npm run typecheck`, not an installed `typescript-language-server`.

- Manual updater checks need explicit “latest version” handling.
- Background updater checks should avoid noisy notification behavior.

## Likely Future Work Areas

Likely evolution paths:

- Better tray security posture around preload and context isolation
- Richer settings UI for notification options already present in the stored schema
- Cleaner separation inside `src/tray/main.ts`, which currently carries too many responsibilities
- Stronger packaged-build verification for updater and release flows

If new work touches these areas, verify the behavior directly in source and keep this summary up to date as the concise historical reference.
