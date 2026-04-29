# Development Workflow

## Commands

Run all Rust tests:

```powershell
cargo test --workspace --locked
```

Run CLI:

The release CLI binary is `qmeter.exe`. The Cargo package is `qmeter`, so source runs use `-p qmeter`.

```powershell
cargo run -p qmeter -- --json
cargo run -p qmeter -- --view table
cargo run -p qmeter -- --view graph
```

Check tray:

```powershell
cargo check -p qmeter-tray
```

Run tray:

```powershell
cargo build -p qmeter-tray
cargo run -p qmeter-tray --bin qmeter-tray
```

Build release binaries:

```powershell
cargo build --release --workspace --locked
```

Expected outputs:

- `target/release/qmeter.exe`
- `target/release/qmeter-tray.exe`

## Fixture Mode

Use fixture mode to verify output and tray rendering without live provider access:

```powershell
$env:USAGE_STATUS_FIXTURE='demo'
cargo run -p qmeter -- --json
cargo run -p qmeter -- --view table
cargo run -p qmeter -- --view graph
cargo build -p qmeter-tray
cargo run -p qmeter-tray --bin qmeter-tray
```

## Environment Variables

- `USAGE_STATUS_FIXTURE=demo`: deterministic sample rows
- `USAGE_STATUS_CODEX_COMMAND`: Codex command override
- `USAGE_STATUS_CACHE_PATH`: cache file override
- `USAGE_STATUS_CACHE_TTL_SECS`: cache TTL override
- `USAGE_STATUS_TRAY_SETTINGS_PATH`: tray settings file override
- `USAGE_STATUS_TRAY_NOTIFICATION_STATE_PATH`: notification state file override

## Verification Order

For code changes:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `cargo test --workspace --locked`
4. `cargo check -p qmeter-tray` when tray code changed
5. `cargo build --release --workspace --locked` before release or legacy-removal commits

For CLI/output changes, also smoke fixture output:

```powershell
$env:USAGE_STATUS_FIXTURE='demo'
cargo run -p qmeter -- --json
cargo run -p qmeter -- --view table
cargo run -p qmeter -- --view graph
```

## Release

The tag-triggered workflow in [`.github/workflows/release.yml`](../.github/workflows/release.yml) is the CI/CD path. It runs only for `v*` tag pushes, validates the tag, runs Rust formatting/clippy/tests, builds release binaries with `--locked`, creates a zip, and uploads assets to the matching GitHub release.

Release sequence:

1. Finish code and documentation.
2. Run the verification order above.
3. Commit.
4. Push.
5. Tag with `vMAJOR.MINOR.PATCH`.
6. Push the tag.

Example:

```powershell
git tag v0.1.9
git push origin v0.1.9
```
