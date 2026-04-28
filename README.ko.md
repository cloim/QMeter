# QMeter

[English](./README.md) | 한국어

QMeter는 Claude Code와 Codex 사용량, 초기화 시간, 캐시 상태, provider 부분 실패를 확인하는 Rust native Windows 트레이 앱 + CLI입니다.

## 주요 기능

- Claude/Codex 사용량 통합 조회
- table, graph, JSON CLI 출력
- Windows tray icon, 수동 새로고침, native popup
- 저장된 tray 설정 기반 백그라운드 새로고침
- cooldown, hysteresis, quiet hours를 반영한 threshold 알림
- live 조회 실패 시 provider별 cache fallback

## 요구사항

- Rust stable
- 트레이 앱은 Windows 11 기준
- Claude 사용량 조회에는 Claude Code 로그인 필요
- Codex 사용량 조회에는 Codex CLI 로그인 필요

## 빌드/실행

```powershell
cargo test --workspace
cargo run -p qmeter-cli -- --json
cargo run -p qmeter-cli -- --view table
cargo run -p qmeter-cli -- --view graph
cargo check -p qmeter-tray
cargo run -p qmeter-tray
```

결정적 출력 확인은 fixture mode를 사용합니다.

```powershell
$env:USAGE_STATUS_FIXTURE='demo'
cargo run -p qmeter-cli -- --json
cargo run -p qmeter-cli -- --view table
cargo run -p qmeter-cli -- --view graph
```

## CLI

```powershell
cargo run -p qmeter-cli -- --json --providers claude,codex --refresh --debug
```

옵션:

- `--json`: normalized JSON 출력
- `--refresh`: fresh cache 무시
- `--debug`: 민감정보 없는 provider 진단 출력
- `--view table|graph`: 터미널 출력 형식 선택
- `--providers claude,codex,all`: provider 선택

종료 코드:

- `0`: 전체 성공
- `1`: 부분 성공
- `2`: 인자 또는 사용 오류
- `3`: 전체 provider 실패

## Tray

```powershell
cargo run -p qmeter-tray
```

트레이 앱 동작:

- 설정: `%APPDATA%\qmeter\tray-settings.v1.json`
- 런타임 로그: `%LOCALAPPDATA%\qmeter\tray-runtime.log`
- 알림 상태: `%LOCALAPPDATA%\qmeter\notification-state.v1.json`
- 메뉴: `Open QMeter`, `Refresh`, `Settings`, `Quit`

## Provider 참고

Claude 사용량은 Claude Code OAuth credential과 Anthropic usage endpoint로 조회합니다. Windows/Linux에서는 `~/.claude/.credentials.json`을 읽고, macOS에서는 `Claude Code-credentials` Keychain 항목을 먼저 시도합니다.

Codex 사용량은 Codex app-server JSON-RPC integration으로 조회합니다.

## Release Binaries

```powershell
cargo build --release --workspace
```

산출물:

- `target/release/qmeter.exe`
- `target/release/qmeter-tray.exe`

태그 기반 GitHub Actions가 이 바이너리를 빌드하고 해당 GitHub release에 업로드합니다.

## Troubleshooting

- Claude row가 없으면 Claude Code 로그인 또는 OAuth credential 파일 상태를 확인하세요.
- Codex row가 없으면 Codex CLI 설치, 로그인, PATH 상태를 확인하세요.
- live provider 없이 렌더링만 확인하려면 `USAGE_STATUS_FIXTURE=demo`를 사용하세요.
