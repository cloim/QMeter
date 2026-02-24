# QMeter

QMeter는 Claude Code/ Codex 사용량과 초기화 시간을 한눈에 확인하는 Windows 트레이 앱 + CLI입니다.

## 주요 기능

- Claude / Codex 사용량 통합 조회
- 트레이 팝업 UI (카드 기반)
- JSON 출력 CLI (`qmeter --json`)
- 캐시/부분 실패 처리
- 환경설정
  - 새로고침 주기 변경
  - 표시 카드(Claude/Codex) on/off

## 요구사항

- Node.js 20+
- Windows 11 (트레이 앱 기준)

## 설치

```bash
npm install
```

## 개발/실행

### 타입체크

```bash
npm run typecheck
```

### 테스트

```bash
npm test
```

### 빌드

```bash
npm run build
```

### CLI 실행

```bash
node dist/cli.js --json
```

또는 전역 링크:

```bash
npm link
qmeter --json
```

### 트레이 앱 실행

```bash
npm run tray:start
```

### 트레이 스모크 테스트

```bash
npm run tray:smoke
```

## 환경설정

트레이 UI의 설정 버튼(톱니)에서 아래 항목을 설정할 수 있습니다.

- 새로고침 주기
- 표시할 카드(Claude/Codex)

설정은 로컬 사용자 설정 폴더에 저장됩니다.

## 리소스 파일

`resources` 폴더의 리소스를 사용합니다.

- `resources/QMeter.ico`
- `resources/QMeter.png`
- `resources/Claude.png`
- `resources/Codex.png`

빌드 시 `scripts/copy-resources.mjs`로 `dist/resources`에 자동 복사됩니다.

## 패키징(배포본)

### 디렉터리 아웃풋

```bash
npm run tray:pack:dir
```

### Windows 설치본(NSIS + Portable)

```bash
npm run tray:pack
```

산출물은 `dist` 하위가 아니라 electron-builder 기본 경로(`dist`/`release` 설정에 따름)로 생성됩니다.

## 트러블슈팅

- Codex 실행 실패(`spawn EINVAL` 등)
  - Windows 셸/경로 이슈일 수 있습니다.
  - Codex 설치/로그인 상태 확인 후 재실행하세요.
- 카드가 보이지 않음
  - 해당 provider가 설치/인증되지 않았거나, 설정에서 OFF일 수 있습니다.
