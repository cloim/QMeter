# AGENTS.md

This file provides guidance to agents working in this repository.

## Read In This Order

1. [Product Overview](./.blueprint/product-overview.md)
   - What QMeter is, who it serves, and the current shipped surface area.

2. [Architecture](./.blueprint/architecture.md)
   - CLI flow, snapshot orchestration, provider boundaries, tray runtime structure.

3. [Development Workflow](./.blueprint/development-workflow.md)
   - Build, test, smoke-test, packaging commands, fixtures, and local verification.

4. [Tray And Settings](./.blueprint/tray-and-settings.md)
   - Rust tray behavior, settings persistence, notifications, runtime diagnostics.

5. [History And Roadmap](./.blueprint/history-and-roadmap.md)
   - Historical context, current caveats, and likely follow-up areas.

## Maintenance Rules

- Keep `AGENTS.md` as an index only. Put detailed guidance in `./.blueprint/*.md`.
- When behavior changes, update the relevant `.blueprint` file instead of expanding this file.
- Prefer documenting current implemented behavior first, then call out planned or pending work separately.
