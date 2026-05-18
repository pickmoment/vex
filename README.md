# VEX — Visual EXplorer

> "파일을 보는 새로운 방식 — 터미널 안의 뷰어"

VEX는 Rust로 구현된 TUI(Terminal User Interface) 파일 매니저입니다.
[yazi](https://github.com/sxyazi/yazi)보다 직관적이고, 파일 **보기(View)** 경험에 집중한 새로운 접근 방식을 제공합니다.

---

## 목차

1. [소개 & 철학](#소개--철학)
2. [설치](#설치)
3. [사용법](#사용법)
4. [기능](#기능)
5. [기술 스택](#기술-스택)
6. [설정](#설정)
7. [개발 로드맵](#개발-로드맵)
8. [기여 가이드](#기여-가이드)
9. [라이선스](#라이선스)

---

## 소개 & 철학

### 핵심 철학: "열기 전에 이미 다 보인다"

VEX의 핵심은 미리보기가 1등 시민이라는 원칙입니다. 파일을 열기 전에 내용을 충분히 파악할 수 있어야 합니다.

### yazi와의 차별점

| 기능 | yazi | VEX |
|------|------|-----|
| 미리보기 방식 | 우측 패널 (고정) | **패널 / 전체화면 / 플로팅** 3모드 |
| 마크다운 | 원문 텍스트 표시 | **렌더링된 HTML 스타일 TUI** |
| 이미지 | 터미널 프로토콜 의존 | 자체 픽셀 렌더러 + 프로토콜 폴백 |
| 학습 곡선 | Vim 키맵 중심, 가파름 | **UI 힌트 상시 표시**, 마우스 완전 지원 |
| 설정 | TOML 직접 편집 | **TUI 내 설정 화면** 제공 |

---

## 설치

### 사전 요구사항

- Rust 1.75+ (`rustup` 권장)
- macOS / Linux / Windows

### Cargo로 빌드

```bash
# 저장소 클론
git clone https://github.com/example/vex
cd vex

# 개발 빌드
cargo build

# 릴리즈 빌드 (최적화)
cargo build --release

# 실행
./target/release/vex
```

### 설치 (PATH에 등록)

```bash
cargo install --path .
```

이후 어디서든 `vex` 명령어로 실행할 수 있습니다.

---

## 사용법

### 기본 실행

```bash
# 현재 디렉토리에서 시작
vex

# 특정 디렉토리에서 시작
vex /path/to/directory
```

### UI 레이아웃

```
┌────────────────────────────────────────────────────────────┐
│ VEX | /home/user/Documents            [탭1] [탭2] [+]  [?] │
├──────────┬─────────────────────┬───────────────────────────┤
│ 즐겨찾기  │  파일 목록           │  미리보기                  │
│          │                     │                           │
│  Home   │ ▶  projects/        │  # 제목                   │
│  Docs   │    notes/           │                           │
│  Pics   │    README.md        │  본문 렌더링 텍스트...      │
│  Code   │    plan.pdf         │                           │
│          │    photo.png        │  - 항목 1                 │
│          │    config.toml      │  - 항목 2                 │
│          │                     │                           │
├──────────┴─────────────────────┴───────────────────────────┤
│ ↑↓ 이동  →/Enter 열기  Space 뷰어  ? 도움말  q 종료         │
└────────────────────────────────────────────────────────────┘
```

### 단축키

#### 탐색

| 키 | 동작 |
|----|------|
| `↑` / `k` | 위로 이동 |
| `↓` / `j` | 아래로 이동 |
| `←` / `h` | 상위 폴더로 이동 |
| `→` / `l` / `Enter` | 진입 또는 파일 열기 |
| `Space` | 전체화면 뷰어 모드 |

#### 파일 조작

| 키 | 동작 |
|----|------|
| `c` | 복사 |
| `v` | 붙여넣기 |
| `d` | 삭제 |
| `r` | 이름 변경 |
| `n` | 새 폴더 생성 |
| `/` | 검색 |

#### 뷰어 모드

| 키 | 동작 |
|----|------|
| `q` / `Esc` | 파일 목록으로 돌아가기 |
| `↑↓` | 스크롤 |
| `←` / `→` | 이전/다음 파일 |
| `e` | 외부 편집기로 열기 |
| `PageUp/Down` | 빠른 스크롤 |

#### 기타

| 키 | 동작 |
|----|------|
| `Ctrl+P` | 명령어 팔레트 |
| `Ctrl+,` | TUI 설정 화면 |
| `?` | 단축키 도움말 |
| `q` | 종료 |

---

## 기능

### 마크다운 렌더링

VEX는 마크다운 파일을 파싱하여 TUI 스타일로 렌더링합니다.

지원 요소:
- 제목 (H1~H6) — 색상 계층 구분
- **굵기**, *기울임*, ~~취소선~~
- `인라인 코드`
- 코드 블록 (신택스 하이라이팅 포함)
- 테이블
- 인용구 (blockquote)
- 체크리스트 (`- [ ]` / `- [x]`)
- 수평선

```bash
# 마크다운 파일 선택 후 Space로 전체화면 뷰어
# [Raw] ↔ [Render] 토글로 원문/렌더링 전환
```

### 신택스 하이라이팅

170개 이상의 언어를 지원하는 syntect 기반 코드 하이라이팅.

지원 언어 예시:
- Rust, Python, JavaScript/TypeScript, Go
- C/C++, Java, Shell
- TOML, YAML, JSON, HTML, CSS

### 이미지 미리보기

터미널 환경에 따라 최적의 렌더링 프로토콜을 자동 선택합니다:

1. **Kitty** — `xterm-kitty` 터미널에서 최고 품질
2. **iTerm2** — macOS iTerm2에서 네이티브 지원
3. **Sixel** — WezTerm, xterm 등
4. **Braille 폴백** — 모든 터미널에서 동작하는 픽셀 아트

지원 포맷: PNG, JPEG, GIF, WEBP, SVG, BMP

### CSV 테이블 뷰어

CSV 파일을 정렬 가능한 인터랙티브 테이블로 표시합니다.

```
  ┌─ 테이블 ───────────────────────────────────
  │ 이름     │ 나이 │ 직업       │
  ─────────────────────────────────────────────
  │ 홍길동   │  30  │ 개발자     │
  │ 김철수   │  25  │ 디자이너   │
  └────────────────────────────────────────────
```

### 파일시스템 실시간 감시

`notify` 크레이트를 활용하여 디렉토리 변경을 실시간으로 감지하고 파일 목록을 자동 갱신합니다.

---

## 기술 스택

| 레이어 | 크레이트 / 기술 | 버전 |
|--------|----------------|------|
| TUI 프레임워크 | `ratatui` | 0.28+ |
| 비동기 런타임 | `tokio` | 1.x |
| 터미널 이벤트 | `crossterm` | 0.28 |
| 마크다운 파싱 | `pulldown-cmark` | 0.11 |
| 신택스 하이라이팅 | `syntect` | 5.x |
| 이미지 렌더링 | `ratatui-image` + `image` | 최신 |
| PDF 렌더링 | `pdfium-render` | v0.3 예정 |
| 파일시스템 감시 | `notify` | 6.x |
| CSV 파싱 | `csv` | 1.x |
| 설정 관리 | `serde` + `toml` | 최신 |
| 크로스플랫폼 경로 | `dirs` | 5.x |
| 퍼지 검색 | `nucleo` | 최신 |
| 에러 처리 | `anyhow` + `thiserror` | 최신 |

---

## 설정

설정 파일 위치:
- macOS: `~/Library/Application Support/vex/config.toml`
- Linux: `~/.config/vex/config.toml`
- Windows: `%APPDATA%\vex\config.toml`

### 기본 설정 예시

```toml
[general]
show_hidden = false
sort_by = "Name"
sort_descending = false
onboarding_done = false

[ui]
theme = "default"
show_bookmarks_panel = true
show_preview_panel = true
show_hint_bar = true
show_icons = true  # Nerd Font 필요

[keymap]
vim_keys = true

[preview]
markdown_render = true
image_protocol = "Auto"
syntax_theme = "Solarized (dark)"
max_file_size = 10485760  # 10MB
```

### TUI 설정 화면

`Ctrl+,`로 설정 화면을 열어 파일 직접 편집 없이 UI로 변경할 수 있습니다 (v0.3+).

---

## 개발 로드맵

### v0.1 — Foundation (현재)
- [x] 기본 3-패널 파일 탐색 (ratatui)
- [x] 텍스트 / 코드 미리보기 + 신택스 하이라이팅
- [x] 마크다운 렌더링 미리보기
- [x] 기본 파일 조작 (복사, 이동, 삭제, 이름변경)
- [x] 하단 힌트 바 + 마우스 지원

### v0.2 — Viewer
- [ ] 전체화면 뷰어 모드 (완성)
- [ ] 이미지 미리보기 (Kitty/Sixel/Braille)
- [ ] CSV 테이블 뷰어 (완성)
- [ ] ZIP/tar 내부 탐색
- [ ] 명령어 팔레트 (`Ctrl+P`)

### v0.3 — Polish
- [ ] PDF 미리보기
- [ ] TUI 설정 화면
- [ ] 즐겨찾기 / 북마크
- [ ] 탭 기능
- [ ] 온보딩 튜토리얼

### v1.0 — Stable
- [ ] 플러그인 API (Lua)
- [ ] Git 상태 표시
- [ ] 원격 파일시스템 (SFTP)
- [ ] 테마 마켓플레이스

---

## 기여 가이드

### 환경 설정

```bash
# Rust 설치 (미설치시)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 의존성 확인
cargo check

# 테스트 실행
cargo test

# 코드 포맷
cargo fmt

# 린트
cargo clippy
```

### 기여 방법

1. 이슈 또는 기능 요청 등록
2. 포크(Fork) 후 피처 브랜치 생성
3. 변경사항 구현
4. PR 제출 (테스트 포함)

### 코딩 컨벤션

- `cargo fmt` 포맷 준수
- `cargo clippy` 경고 해소
- 공개 API에 문서 주석(`///`) 필수
- 에러 처리는 `anyhow` 사용 (라이브러리는 `thiserror`)

---

## 라이선스

MIT License — 자세한 내용은 [LICENSE](LICENSE) 파일 참조.
