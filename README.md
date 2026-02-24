# QMeter

English | [한국어](./README.ko.md)

QMeter is a Windows tray app + CLI that lets you check Claude Code/Codex usage and reset timing at a glance.

![QMeter Screenshot](./Screenshot.png)

## Key Features

- Unified usage view for Claude and Codex
- Tray popup UI (card-based)
- JSON output CLI (`qmeter --json`)
- Cache and partial-failure handling
- Settings
  - Refresh interval
  - Card visibility (Claude/Codex)

## Requirements

- Node.js 20+
- Windows 11 (for tray app)

## Installation

```bash
npm install
```

## CLI Quick Start for Installer Users

If you install QMeter via Windows installer (NSIS/Portable), the tray app is available right away,
but the `qmeter` CLI command is not automatically added to PATH.

To use CLI, use one of the following from a Node.js environment.

1) Build and run directly

```bash
npm run build
node dist/cli.js --help
node dist/cli.js --json
```

2) Link globally and use `qmeter`

```bash
npm link
qmeter --help
qmeter --json
```

Common options

- `--json`: Print JSON output
- `--refresh`: Bypass cache and refresh data
- `--debug`: Print debug diagnostics (no secrets)
- `--view table|graph`: Select output mode
- `--providers claude,codex,all`: Select providers

## Development / Run

### Typecheck

```bash
npm run typecheck
```

### Test

```bash
npm test
```

### Build

```bash
npm run build
```

### Run CLI

```bash
node dist/cli.js --json
```

Or global link:

```bash
npm link
qmeter --json
```

### Run Tray App

```bash
npm run tray:start
```

### Tray Smoke Test

```bash
npm run tray:smoke
```

## Settings

You can configure the following in the tray UI settings (gear icon):

- Refresh interval
- Visible cards (Claude/Codex)

Settings are saved in the local user settings directory.

## Resource Files

QMeter uses files in the `resources` folder.

- `resources/QMeter.ico`
- `resources/QMeter.png`
- `resources/Claude.png`
- `resources/Codex.png`

During build, `scripts/copy-resources.mjs` copies them into `dist/resources` automatically.

## Packaging (Distributables)

### Directory Output

```bash
npm run tray:pack:dir
```

### Windows Installer (NSIS + Portable)

```bash
npm run tray:pack
```

Artifacts are generated under electron-builder output directories (per `dist`/`release` settings), not under runtime `dist` assets only.

## Auto Update

- Works only in packaged app builds (not in local dev run).
- App performs background update checks on startup.
- Tray menu includes `Check for Updates` for manual checks.
- Manual check notifications show explicit statuses: checking, update available/downloading, up-to-date, and error.
- After download finishes, update is applied when the app exits.

## GitHub Release Automation

- Workflow: `.github/workflows/release.yml`
- Trigger: push tag matching `v*` (example: `v0.1.1`)
- Pipeline:
  1. Validate tag format (`vMAJOR.MINOR.PATCH`)
  2. Sync `package.json` version from the pushed tag
  3. `npm ci`
  4. `npm run typecheck`
  5. `npm test`
  6. `npm run tray:pack` (electron-builder publishes via GitHub provider)

Tag release example:

```bash
git tag v0.1.1
git push origin v0.1.1
```

## Troubleshooting

- Codex launch failure (e.g. `spawn EINVAL`)
  - This can be a Windows shell/path issue.
  - Verify Codex installation/login status, then retry.
- Card not visible
  - The provider may be missing/not authenticated, or disabled in settings.
