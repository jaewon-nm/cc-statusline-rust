# ccstatusline-rs (beta)

Rust port of [`ccstatusline`](https://github.com/sirmalloc/ccstatusline) — Claude Code의 상태 줄을 그리는 작은 프로그램. 색상이 들어간 두 줄짜리 기본 테마와 함께, **AI 에이전트가 직접 설정·설치·삭제까지 할 수 있도록** 디자인되어 있습니다.

> ⚠️ **Beta** — API/CLI는 안정적이지만 v0.1.0 태그 전이라 일부 동작이 바뀔 수 있습니다. 사용 중 문제 발견하면 [Issues](https://github.com/jaewon-nm/cc-statusline-rust/issues)에 남겨주세요.

## 기본 화면

```
✦ [Opus 4.7 (1M context)] | 📂 F:\Works\naya\cc-statusline-rust | 🔋 [..........] 80.0K/1.0M(8%) | 📊 85.3K | 💰 $2.55
⏱ 5h [##........](21%) ↻ 12:00 | 📅 7d [##........](20%) ↻ 5/19 06:00
```

- 모델, 작업 폴더, 컨텍스트 사용량, 세션 토큰, 비용, 5시간 / 7일 리밋이 한눈에.
- 프로그레스 바는 사용률에 따라 **초록(여유) → 노랑(주의) → 빨강(위험)** 으로 자동 변색.
- 기본 색상은 켜져 있고, `NO_COLOR=1` 환경 변수로 끌 수 있음.

기본 테마 전체 사양: [`docs/design-docs/default-theme.md`](docs/design-docs/default-theme.md).

---

## 에이전트한테 시키기 (권장)

설치·설정·되돌리기 모두 한 명령으로 끝납니다. Claude Code(또는 다른 에이전트)에게 다음처럼 부탁하면 알아서 처리해요:

> "ccstatusline-rs를 빌드해서 ~/bin에 설치하고 Claude Code statusLine에 연결해줘."

> "ccstatusline-rs config color session_cost를 yellow로 바꿔줘."

> "ccstatusline-rs를 깔끔히 제거하고 이전 statusLine으로 되돌려줘."

에이전트는 아래 정리된 CLI 명령을 그대로 실행할 수 있고, 각 명령은 JSON 결과를 돌려주기 때문에 에이전트가 다음 행동을 결정하기에 충분합니다.

`ccstatusline-rs --help`로 사람이 읽기 좋은 도움말도 볼 수 있어요. 모든 서브커맨드에 `--help`가 달려 있습니다.

---

## 직접 하기 — 설치

```powershell
# 1. 소스에서 빌드
cargo build --release

# 2. 한 명령으로 설치 — 바이너리 복사 + 래퍼 생성 + Claude Code 설정 연결
./target/release/ccstatusline-rs install
```

`install`은 자동으로 다음을 합니다:

- 바이너리를 `~/bin`(Windows) / `~/.local/bin`(macOS·Linux)에 복사.
- Windows 한정: 작은 Node 래퍼(`ccstatusline-rs.mjs`)를 같이 만들어요. Claude Code Windows 빌드가 statusLine 명령을 `node "..."` 꼴로만 받아주는 버그 때문입니다.
- `~/.claude/settings.json`을 **백업**한 뒤 `statusLine` 항목만 우리 명령으로 교체. 다른 설정은 손대지 않음.

설치 후 **Claude Code를 완전 종료하고 다시 실행**해야 반영됩니다.

### neo-mem과 함께 쓰는 경우

`tokenwatch-statusline.mjs`가 이미 등록돼 있으면 그게 우리 명령을 wrapper로 호출하도록 자동 설정됩니다(자세한 흐름은 본 문서 마지막 "통합 다이어그램" 참고).

---

## 직접 하기 — 사용 / 변경

### 설정 확인
```powershell
ccstatusline-rs config show --pretty
ccstatusline-rs widgets        # 사용 가능한 위젯 목록
ccstatusline-rs schema         # 설정 파일의 JSON Schema
```

### 위젯 추가/제거
```powershell
ccstatusline-rs config add git_branch                  # 마지막 줄에 추가
ccstatusline-rs config add cwd --line 0 --position 1   # 0번 줄 1번 자리에 끼워넣기
ccstatusline-rs config remove --line 1 --position 0
```

### 색상 바꾸기
```powershell
ccstatusline-rs config color model --fg red --bold
ccstatusline-rs config color session_cost --fg "#ff8800"
ccstatusline-rs config color session_cost --clear      # 사용자 색 지우고 기본 테마로
```

사용자 색은 위젯 전체를 **덮어쓰기**라서 프로그레스 바 단계별 색까지 그 색 한 가지로 보입니다(예: red로 지정하면 8%여도 빨강). 단계별 색을 그대로 두고 싶으면 색 지정을 하지 않으면 됩니다.

### 미리 보기 / 차이 비교
```powershell
ccstatusline-rs preview --payload sample.json
ccstatusline-rs preview --payload sample.json --config candidate.json --diff
```

`--diff`는 현재 설정 vs 후보 설정을 같은 페이로드로 렌더링해서 두 결과와 `identical: true/false`를 JSON으로 알려줍니다. 적용 전에 안전하게 확인용.

### 적용 / 검증
```powershell
ccstatusline-rs config apply --file new-layout.json   # 통째로 교체
ccstatusline-rs config validate --file new-layout.json
```

---

## 직접 하기 — 제거 / 되돌리기

```powershell
ccstatusline-rs uninstall                  # 직전 install 시점의 settings.json으로 복구
ccstatusline-rs uninstall --purge-binary   # 위에 더해 ~/bin의 바이너리·래퍼도 삭제
```

`uninstall`은 가장 최근 백업(`settings.json.ccstatusline-rs-bak-...`)을 자동으로 골라 원자적으로 되돌립니다. 더 오래된 시점으로 가고 싶으면 `--backup <경로>`로 지정 가능. 우리가 만들지 않은 파일은 절대 지우지 않습니다.

---

## 명령 요약

| 명령 | 용도 |
|---|---|
| (없음) | stdin으로 받은 Claude Code payload를 렌더링 |
| `install` | 바이너리 배치 + Claude Code 연결 |
| `uninstall` | 직전 install 되돌리기 |
| `config show` | 현재 설정 JSON 출력 |
| `config add` | 위젯 추가 |
| `config remove` | 위젯 제거 |
| `config color` | 위젯 색 설정/지우기 |
| `config apply` | 설정 통째로 교체 |
| `config validate` | 설정 검증 |
| `preview` | 페이로드로 미리 렌더링 |
| `widgets` | 위젯 종류 목록 |
| `schema` | 설정 JSON Schema |

모든 비-렌더링 명령은 **JSON을 stdout으로** 돌려주고, `--pretty` 플래그를 주면 사람이 읽기 좋게 들여쓰기까지 해줍니다. 에러는 stderr로 가고 종료 코드는 0이 아닙니다.

---

## 환경 변수

| 변수 | 효과 |
|---|---|
| `NO_COLOR=<아무 값>` | 모든 색 끔 (최우선) |
| `CLICOLOR_FORCE=1` / `FORCE_COLOR=1` | 색 강제 켬 |
| `CCSTATUSLINE_RS_CONFIG=<경로>` | 설정 파일 위치 override (테스트·여러 프로필 운영용) |

---

## 빌드 환경

- Rust 1.94 (저장소의 `rust-toolchain.toml`이 자동으로 잡아줍니다)
- `git` (선택적, `git_*` 위젯 사용 시)
- Windows / macOS / Linux 모두 지원, 최종 산출물은 단일 정적 바이너리(~1.9MB)

---

## 통합 다이어그램

neo-mem과 함께 쓰는 경우의 흐름:

```
Claude Code statusLine
   ↓ stdin (payload + rate_limits)
tokenwatch-statusline.mjs (neo-mem)
   ├── ~/.claude/.tw-statusline-cache.json 에 캐시 기록  (neo-mem worker가 읽음)
   └── ccstatusline-rs.mjs 실행 (stdin 그대로 전달)
        ↓
   ccstatusline-rs.exe ← 색깔 입혀서 stdout
        ↓
   Claude Code 입력창 표시
```

자체 단독으로 쓰는 경우는 `tokenwatch-statusline.mjs` 단계가 빠지고 `ccstatusline-rs.mjs`(또는 POSIX에선 `ccstatusline-rs` 바이너리 직결)이 직접 호출됩니다.

---

## 더 자세한 설치 옵션

[`INSTALL.md`](INSTALL.md) — 사전 빌드된 아카이브·`cargo install`·수동(zero-trust) 설치 경로 + 트러블슈팅.

## 프로젝트 거버넌스

- Rust 1.94 핀 고정 (`rust-toolchain.toml`).
- `thiserror`만 사용, `anyhow`는 `main.rs` 한 곳에서만.
- 100% 테스트 커버리지(현재 172 tests).
- 한국 시간(KST) 기본; `tz` 설정으로 IANA 이름이나 `"system"`으로 변경 가능.
- 코드 주석은 WHY만, 작업/이슈 번호는 코드에 박지 않음.

전체 규칙: [`CLAUDE.md`](CLAUDE.md). 문서 워크플로우: [`docs/GOVERNANCE.md`](docs/GOVERNANCE.md). 변경 이력: [`CHANGELOG.md`](CHANGELOG.md).

## 업스트림 참고

업스트림 TS/Bun 구현은 별도로 클론해서 두면 비교 작업이 쉽습니다(저장소에는 포함하지 않음):

```bash
git clone https://github.com/sirmalloc/ccstatusline references/ccstatusline
```

## License

MIT — 업스트림과 동일.
