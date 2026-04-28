# Rust Native Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace QMeter's shipped Node/Electron implementation with a Rust-native CLI and Windows tray application.

**Architecture:** Build a Rust workspace with focused crates for core contracts, providers, CLI, and tray. Keep the current TypeScript implementation as the parity oracle until the Rust CLI and tray have equivalent tests and smoke coverage.

**Tech Stack:** Rust stable, Cargo workspace, serde, clap, anyhow/thiserror, chrono, dirs, tokio or std process as needed, windows-rs, tray-icon, assert_cmd, insta or similar snapshot testing.

---

## File Structure

- Create: `Cargo.toml`
- Create: `crates/qmeter-core/Cargo.toml`
- Create: `crates/qmeter-core/src/lib.rs`
- Create: `crates/qmeter-core/src/types.rs`
- Create: `crates/qmeter-core/src/cache.rs`
- Create: `crates/qmeter-core/src/output.rs`
- Create: `crates/qmeter-core/src/snapshot.rs`
- Create: `crates/qmeter-core/src/notification_policy.rs`
- Create: `crates/qmeter-core/src/settings.rs`
- Create: `crates/qmeter-providers/Cargo.toml`
- Create: `crates/qmeter-providers/src/lib.rs`
- Create: `crates/qmeter-providers/src/provider.rs`
- Create: `crates/qmeter-providers/src/codex.rs`
- Create: `crates/qmeter-providers/src/claude.rs`
- Create: `crates/qmeter-providers/src/claude_usage.rs`
- Create: `crates/qmeter-cli/Cargo.toml`
- Create: `crates/qmeter-cli/src/main.rs`
- Create: `crates/qmeter-tray/Cargo.toml`
- Create: `crates/qmeter-tray/src/main.rs`
- Create: `crates/qmeter-tray/src/runtime_log.rs`
- Create: `crates/qmeter-tray/src/tray_app.rs`
- Create: `tests/fixtures/claude_usage_screen.txt`
- Modify later: `.blueprint/architecture.md`
- Modify later: `.blueprint/development-workflow.md`
- Modify later: `.blueprint/tray-and-settings.md`
- Modify later: `README.md`
- Modify later: `README.ko.md`
- Remove in final cleanup: Node/Electron build files after parity is verified

## Task 1: Workspace Skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `crates/qmeter-core/Cargo.toml`
- Create: `crates/qmeter-core/src/lib.rs`
- Create: `crates/qmeter-cli/Cargo.toml`
- Create: `crates/qmeter-cli/src/main.rs`

- [ ] **Step 1: Write failing workspace check**

Run:

```powershell
cargo test --workspace
```

Expected: FAIL because no Cargo workspace exists.

- [ ] **Step 2: Add minimal workspace and placeholder crates**

Create root workspace and minimal `qmeter-core` and `qmeter-cli` crates.

- [ ] **Step 3: Verify workspace compiles**

Run:

```powershell
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 4: Commit**

```powershell
git add Cargo.toml crates/qmeter-core crates/qmeter-cli
git commit -m "build: add rust workspace skeleton"
```

## Task 2: Core Types And Serialization

**Files:**
- Create: `crates/qmeter-core/src/types.rs`
- Modify: `crates/qmeter-core/src/lib.rs`

- [ ] **Step 1: Write failing tests for normalized snapshot JSON**

Add tests that serialize a fixture snapshot with the same field names as current TypeScript JSON.

- [ ] **Step 2: Run tests and verify failure**

```powershell
cargo test -p qmeter-core types
```

Expected: FAIL because types do not exist.

- [ ] **Step 3: Implement `ProviderId`, `SourceKind`, `Confidence`, `NormalizedRow`, `NormalizedError`, `NormalizedSnapshot`**

Use serde rename rules to preserve camelCase fields such as `usedPercent`, `resetAt`, and `fetchedAt`.

- [ ] **Step 4: Verify tests pass**

```powershell
cargo test -p qmeter-core types
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add crates/qmeter-core
git commit -m "feat: add rust normalized snapshot types"
```

## Task 3: Fixture Snapshot And Orchestration

**Files:**
- Create: `crates/qmeter-core/src/snapshot.rs`
- Modify: `crates/qmeter-core/src/lib.rs`

- [ ] **Step 1: Write failing tests for `USAGE_STATUS_FIXTURE=demo`**

Expected rows:

- `claude:session`, 79 percent
- `claude:week(all-models)`, 22 percent
- `codex:5h`, 81 percent
- `codex:weekly`, 30 percent

- [ ] **Step 2: Run failing test**

```powershell
cargo test -p qmeter-core fixture
```

Expected: FAIL because snapshot orchestration is absent.

- [ ] **Step 3: Implement fixture snapshot path**

Keep the same fixed timestamp behavior from `src/core/snapshot.ts`.

- [ ] **Step 4: Verify**

```powershell
cargo test -p qmeter-core fixture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add crates/qmeter-core
git commit -m "feat: add rust fixture snapshot"
```

## Task 4: CLI Contract

**Files:**
- Modify: `crates/qmeter-cli/src/main.rs`
- Create: `crates/qmeter-core/src/output.rs`

- [ ] **Step 1: Write failing CLI tests**

Cover:

- `--help`
- unknown argument exits `2`
- `USAGE_STATUS_FIXTURE=demo --json`
- `--view table`
- `--view graph`
- `--providers claude`

- [ ] **Step 2: Verify failures**

```powershell
cargo test -p qmeter-cli
```

Expected: FAIL because CLI behavior is missing.

- [ ] **Step 3: Implement clap-based CLI and output rendering**

Preserve existing help semantics where possible, exit codes, table headers, graph bars, and error filtering.

- [ ] **Step 4: Verify**

```powershell
cargo test -p qmeter-cli
cargo run -p qmeter-cli -- --help
```

Expected: tests PASS and help prints.

- [ ] **Step 5: Commit**

```powershell
git add crates/qmeter-cli crates/qmeter-core
git commit -m "feat: port qmeter cli contract to rust"
```

## Task 5: Cache

**Files:**
- Create: `crates/qmeter-core/src/cache.rs`
- Modify: `crates/qmeter-core/src/snapshot.rs`

- [ ] **Step 1: Write failing cache tests**

Cover:

- default Windows path shape
- `USAGE_STATUS_CACHE_PATH`
- `USAGE_STATUS_CACHE_TTL_SECS`
- cache freshness
- stale cache row fallback
- cache rows rewritten with `source=cache`

- [ ] **Step 2: Run failing tests**

```powershell
cargo test -p qmeter-core cache
```

- [ ] **Step 3: Implement cache behavior**

Use JSON version `1`, same provider layout, and no credentials.

- [ ] **Step 4: Verify**

```powershell
cargo test -p qmeter-core cache
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add crates/qmeter-core
git commit -m "feat: port cache behavior to rust"
```

## Task 6: Provider Trait And Codex Provider

**Files:**
- Create: `crates/qmeter-providers/Cargo.toml`
- Create: `crates/qmeter-providers/src/lib.rs`
- Create: `crates/qmeter-providers/src/provider.rs`
- Create: `crates/qmeter-providers/src/codex.rs`
- Modify: `crates/qmeter-core/src/snapshot.rs`

- [ ] **Step 1: Write failing JSON-RPC parsing tests**

Use fixture JSON that includes `rateLimits`, `rateLimitsByLimitId.codex`, primary, and secondary windows.

- [ ] **Step 2: Run failing tests**

```powershell
cargo test -p qmeter-providers codex
```

- [ ] **Step 3: Implement Codex parsing and process boundary**

Keep process spawning behind a function that can be tested without launching live `codex`.

- [ ] **Step 4: Verify**

```powershell
cargo test -p qmeter-providers codex
```

- [ ] **Step 5: Commit**

```powershell
git add crates/qmeter-providers crates/qmeter-core
git commit -m "feat: port codex provider to rust"
```

## Task 7: Claude Parser And Driver Boundary

**Files:**
- Create: `tests/fixtures/claude_usage_screen.txt`
- Create: `crates/qmeter-providers/src/claude_usage.rs`
- Create: `crates/qmeter-providers/src/claude.rs`

- [ ] **Step 1: Write failing parser tests**

Cover ANSI stripping, whitespace normalization, session percent, week percent, reset labels, and parse failure.

- [ ] **Step 2: Run failing tests**

```powershell
cargo test -p qmeter-providers claude
```

- [ ] **Step 3: Implement parser**

Port `src/parsers/claudeUsage.ts` behavior first, without semantic improvements.

- [ ] **Step 4: Add driver boundary tests**

Test timeout/error mapping with a fake command runner.

- [ ] **Step 5: Implement driver boundary**

Use a small abstraction so real PTY/process handling is replaceable.

- [ ] **Step 6: Verify**

```powershell
cargo test -p qmeter-providers claude
```

- [ ] **Step 7: Commit**

```powershell
git add crates/qmeter-providers tests/fixtures
git commit -m "feat: port claude usage parsing to rust"
```

## Task 8: Notification, Settings, And Scheduler

**Files:**
- Create: `crates/qmeter-core/src/notification_policy.rs`
- Create: `crates/qmeter-core/src/settings.rs`
- Modify: `crates/qmeter-core/src/lib.rs`

- [ ] **Step 1: Write failing tests from existing Vitest behavior**

Port the cases from:

- `test/notificationPolicy.test.ts`
- `test/notificationState.test.ts`
- `test/scheduler.test.ts`
- `test/notificationState.test.ts`

- [ ] **Step 2: Run failing tests**

```powershell
cargo test -p qmeter-core notification
cargo test -p qmeter-core settings
```

- [ ] **Step 3: Implement Rust equivalents**

Preserve thresholds, hysteresis, cooldown, quiet hours, refresh interval bounds, visible provider defaults, and settings path override.

- [ ] **Step 4: Verify**

```powershell
cargo test -p qmeter-core
```

- [ ] **Step 5: Commit**

```powershell
git add crates/qmeter-core
git commit -m "feat: port tray settings and notifications to rust"
```

## Task 9: Native Tray Shell

**Files:**
- Create: `crates/qmeter-tray/Cargo.toml`
- Create: `crates/qmeter-tray/src/main.rs`
- Create: `crates/qmeter-tray/src/runtime_log.rs`
- Create: `crates/qmeter-tray/src/tray_app.rs`

- [ ] **Step 1: Write failing runtime log tests**

Cover log path override/default and basic telemetry line formatting.

- [ ] **Step 2: Run failing tests**

```powershell
cargo test -p qmeter-tray runtime_log
```

- [ ] **Step 3: Implement runtime logging**

Preserve `%LOCALAPPDATA%\qmeter\tray-runtime.log`.

- [ ] **Step 4: Implement tray shell**

Add single-instance guard, tray icon, menu, refresh command, quit command, and popup placeholder.

- [ ] **Step 5: Manual smoke**

```powershell
cargo run -p qmeter-tray
```

Expected: tray icon appears, menu opens, quit exits cleanly, runtime log is written.

- [ ] **Step 6: Commit**

```powershell
git add crates/qmeter-tray
git commit -m "feat: add rust native tray shell"
```

## Task 10: Native Popup And Refresh Loop

**Files:**
- Modify: `crates/qmeter-tray/src/tray_app.rs`
- Modify: `crates/qmeter-tray/src/main.rs`

- [ ] **Step 1: Add tests for tray state model**

Test refresh state transitions separately from Win32 UI.

- [ ] **Step 2: Implement popup model**

Show provider rows, progress bars, reset labels, cache/stale status, last checked time, manual refresh, and settings entry.

- [ ] **Step 3: Implement background refresh**

Use settings refresh interval, manual refresh trigger, and safe error logging.

- [ ] **Step 4: Manual smoke**

```powershell
$env:USAGE_STATUS_FIXTURE='demo'
cargo run -p qmeter-tray
```

Expected: popup shows fixture Claude/Codex rows and refresh works.

- [ ] **Step 5: Commit**

```powershell
git add crates/qmeter-tray
git commit -m "feat: add rust tray popup and refresh loop"
```

## Task 11: Packaging And Documentation

**Files:**
- Modify: `.blueprint/architecture.md`
- Modify: `.blueprint/development-workflow.md`
- Modify: `.blueprint/tray-and-settings.md`
- Modify: `README.md`
- Modify: `README.ko.md`
- Create or modify: `.github/workflows/*` as needed

- [ ] **Step 1: Write or update docs after behavior is real**

Document Rust commands:

```powershell
cargo test --workspace
cargo run -p qmeter-cli -- --json
cargo run -p qmeter-tray
cargo build --release --workspace
```

- [ ] **Step 2: Add packaging workflow**

Start with release binaries before adding installer complexity.

- [ ] **Step 3: Verify docs commands**

Run every documented command that is practical locally.

- [ ] **Step 4: Commit**

```powershell
git add .blueprint README.md README.ko.md .github
git commit -m "docs: document rust native qmeter workflow"
```

## Task 12: Legacy Removal

**Files:**
- Remove after parity verification: `src/`
- Remove after parity verification: `test/`
- Remove after parity verification: `package.json`
- Remove after parity verification: `package-lock.json`
- Remove after parity verification: `tsconfig.json`
- Remove after parity verification: `tsup.config.ts`
- Remove after parity verification: `scripts/copy-resources.mjs`
- Keep or relocate: `resources/`

- [ ] **Step 1: Verify Rust parity**

Run:

```powershell
cargo test --workspace
$env:USAGE_STATUS_FIXTURE='demo'; cargo run -p qmeter-cli -- --json
$env:USAGE_STATUS_FIXTURE='demo'; cargo run -p qmeter-cli -- --view table
$env:USAGE_STATUS_FIXTURE='demo'; cargo run -p qmeter-cli -- --view graph
```

- [ ] **Step 2: Manually smoke tray**

Run:

```powershell
$env:USAGE_STATUS_FIXTURE='demo'
cargo run -p qmeter-tray
```

- [ ] **Step 3: Remove legacy Node/Electron implementation**

Only remove after all parity checks pass.

- [ ] **Step 4: Final verification**

```powershell
cargo test --workspace
cargo build --release --workspace
git status --short
```

- [ ] **Step 5: Commit**

```powershell
git add -A
git commit -m "refactor: remove legacy electron implementation"
```
