# Development Workflow

## Primary Commands

Project commands are defined in [`package.json`](D:\Code\Vibe\QMeter\package.json).

- `npm run build`
  Builds TypeScript with `tsup` and copies tray resources into `dist/resources`.
- `npm run typecheck`
  Runs `tsc --noEmit`.
- `npm test`
  Runs the Vitest suite once.
- `npm run tray:start`
  Starts the Electron tray app from the built output.
- `npm run tray:smoke`
  Runs the tray smoke entry.
- `npm run tray:pack`
  Builds and packages Windows NSIS and portable artifacts.
- `npm run tray:pack:dir`
  Builds an unpacked directory artifact.

## Expected Verification Order

For most code changes, verify in this order:

1. `npm run typecheck`
2. `npm test`
3. `npm run build`
4. If tray behavior changed, run `npm run tray:smoke`

If a change affects packaging or updater logic, also validate the relevant packaging command.

## Test Layout

Tests live under [`test`](D:\Code\Vibe\QMeter\test).

Current test coverage focuses on core logic:

- cache behavior
- Claude usage parsing
- notification policy/state
- scheduler behavior

When adding behavior in core or providers, keep tests close to the current style: isolated logic tests first, integration only where the runtime behavior matters.

## Fixture And Local Debug Modes

The collection layer has a built-in fixture mode in [`src/core/snapshot.ts`](D:\Code\Vibe\QMeter\src\core\snapshot.ts).

Useful environment variables:

- `USAGE_STATUS_FIXTURE=demo`
  Returns deterministic sample Claude/Codex rows without touching real tools.
- `USAGE_STATUS_BASH_EXE`
  Overrides the Git Bash path used by the Claude provider.
- `USAGE_STATUS_CODEX_COMMAND`
  Overrides the Codex command path.
- `USAGE_STATUS_CACHE_PATH`
  Overrides cache file location.
- `USAGE_STATUS_CACHE_TTL_SECS`
  Overrides cache TTL.
- `USAGE_STATUS_TRAY_SETTINGS_PATH`
  Overrides tray settings file location.

Use fixture mode when working on output, UI, or notification behavior without depending on live provider state.

## Resource And Build Notes

Static assets live in [`resources`](D:\Code\Vibe\QMeter\resources).

The build step depends on [`scripts/copy-resources.mjs`](D:\Code\Vibe\QMeter\scripts\copy-resources.mjs) to copy:

- `QMeter.ico`
- `QMeter.png`
- `Claude.png`
- `Codex.png`

Do not move or rename these assets casually; tray packaging and inline UI rendering assume they exist.

## Packaging And Release

Packaging configuration is inside [`package.json`](D:\Code\Vibe\QMeter\package.json) under `build`.

Current release model:

- Electron Builder targets `nsis` and `portable`
- GitHub is the configured publish provider
- The repository includes a GitHub Actions release workflow referenced in the README

`.sisyphus/notepads/windows-tray-full-version/decisions.md` records that release automation should keep using the existing `npm run tray:pack` path rather than adding a separate publish script.

## Practical Editing Guidance

- Prefer changing shared logic in `src/core/*`, `src/providers/*`, or `src/types.ts` before touching both CLI and tray layers separately.
- Treat `src/tray/main.ts` as high-impact: it mixes lifecycle, updater logic, IPC, and inline HTML.
- Keep public CLI output stable unless the change explicitly updates the contract and tests.
- Use `npm run typecheck` as the baseline verification tool. `.sisyphus/notepads/windows-tray-full-version/issues.md` notes that `typescript-language-server` is not available in the current environment.
