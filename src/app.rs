use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::Backend, layout::Rect, Terminal};
use std::io::Read as _;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::fs::ops::FileEntry;
use crate::ui::layout::AppLayout;
use crate::ui::status_bar::StatusMessage;

/// 포커스된 패널
#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPanel {
    FileList,
    Bookmarks,
    PathClipboard,
}

/// 앱의 전체 모드 상태
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// 3-패널 파일 탐색 모드
    FileList,
    /// 전체화면 뷰어 모드
    Viewer,
    /// 열기 프로그램 선택 오버레이
    OpenWith,
    /// 설정 화면 모드
    Settings,
    /// 명령어 팔레트 모드
    CommandPalette,
    /// 도움말 오버레이
    Help,
    /// Git 관리 모드
    Git,
    /// 파일 관리 오버레이 (복사/이동/삭제/이름변경)
    FileManager,
    /// 경로 클립보드 선택 오버레이
    PathClipboard,
}

/// Git 모드에서 포커스된 섹션
#[derive(Debug, Clone, PartialEq)]
pub enum GitSection {
    Staged,
    Unstaged,
}

/// 파일 관리 작업 종류
#[derive(Debug, Clone, PartialEq)]
pub enum FmOp {
    Copy,
    Move,
    Rename,
    Delete,
    NewDir,
}

/// 파일 타입 분류
#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Text,
    Markdown,
    Code(String), // 언어 이름
    Image,
    Pdf,
    Csv,
    Archive,
    Unknown,
}

/// 앱 전체 상태
pub struct App {
    /// 현재 디렉토리 경로
    pub current_dir: PathBuf,
    /// 파일 목록
    pub file_entries: Vec<FileEntry>,
    /// 선택된 파일 인덱스
    pub selected_index: usize,
    /// 현재 앱 모드
    pub mode: AppMode,
    /// 포커스된 패널
    pub focused_panel: FocusedPanel,
    /// 즐겨찾기 패널에서 선택된 항목 인덱스
    pub bookmark_index: usize,
    /// 즐겨찾기 패널 영역 (마우스 클릭 처리용)
    pub bookmarks_area: Option<Rect>,
    /// 설정
    pub config: Config,
    /// 종료 플래그
    pub should_quit: bool,
    /// 미리보기 수직 스크롤 오프셋
    pub preview_scroll: u16,
    /// 미리보기 수평 스크롤 오프셋
    pub preview_h_scroll: u16,
    /// 미리보기 자동 줄바꿈 여부
    pub preview_wrap: bool,
    /// 편집기 열기 대기 플래그
    pub pending_editor_open: bool,
    /// 터미널 프로그램으로 열기 대기 (command, args)
    pub pending_terminal_opener: Option<(String, Vec<String>)>,
    /// OpenWith 메뉴에서 선택된 인덱스
    pub open_with_index: usize,
    /// 검색 쿼리
    pub search_query: String,
    /// 검색 모드 여부
    pub is_searching: bool,
    /// 필터링된 파일 인덱스 목록 (file_entries 기준)
    pub filtered_indices: Vec<usize>,
    /// 뷰어 검색 쿼리
    pub viewer_search_query: String,
    /// 뷰어 검색 입력창 열림 여부
    pub viewer_is_searching: bool,
    /// 뷰어 검색 매칭 줄 번호 목록 (0-based)
    pub viewer_search_matches: Vec<usize>,
    /// 현재 포커스된 매칭 인덱스
    pub viewer_search_idx: usize,
    /// 뷰어 줄이동 입력
    pub viewer_goto_input: String,
    /// 뷰어 줄이동 입력창 열림 여부
    pub viewer_is_goto: bool,
    /// gg 감지용 직전 키
    pub viewer_prev_key: Option<KeyCode>,
    /// Git 상태 (GitState로 묶임)
    pub git: crate::state::GitState,
    /// 파일 관리: 메뉴 선택 인덱스
    pub fm_menu_idx: usize,
    /// 파일 관리: 텍스트 입력 버퍼 (이름/경로)
    pub fm_input: String,
    /// 파일 관리: 입력 커서 위치 (바이트 오프셋)
    pub fm_cursor: usize,
    /// 파일 관리: 현재 진행 중인 작업
    pub fm_operation: Option<FmOp>,
    /// 파일 관리: 마지막 오류 메시지
    pub fm_error: Option<String>,
    /// 파일 관리: 덮어쓰기 확인 대기 중인 대상 경로
    pub fm_overwrite_target: Option<PathBuf>,
    /// 경로 클립보드 목록
    pub path_clipboard: Vec<PathBuf>,
    /// 경로 클립보드 선택 인덱스
    pub path_clipboard_idx: usize,
    /// 파일 목록 패널의 실제 내부 높이 (렌더링 시 기록)
    pub file_list_height: u16,
    /// 뷰어 콘텐츠 영역의 실제 내부 높이 (렌더링 시 기록)
    pub viewer_height: u16,
    /// 상태 메시지 (작업 결과 피드백, TTL 만료 시 None)
    pub status: Option<StatusMessage>,
    /// 마우스 드래그 시작 위치 (column, row)
    pub drag_start: Option<(u16, u16)>,
    /// 마우스 드래그 현재 위치 (column, row)
    pub drag_end: Option<(u16, u16)>,
    /// 드래그 가능한 컨텐츠 영역 (렌더링 시 갱신)
    pub drag_content_area: Rect,
}

impl App {
    /// 새 앱 인스턴스 생성
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let current_dir = std::env::current_dir()?;
        let file_entries = crate::fs::ops::list_dir(&current_dir)?;
        let preview_wrap = config.preview.wrap;

        let filtered_indices = (0..file_entries.len()).collect();
        let git = crate::state::GitState::new(&current_dir);
        Ok(Self {
            current_dir,
            file_entries,
            selected_index: 0,
            mode: AppMode::FileList,
            focused_panel: FocusedPanel::FileList,
            bookmark_index: 0,
            bookmarks_area: None,
            config,
            should_quit: false,
            preview_scroll: 0,
            preview_h_scroll: 0,
            preview_wrap,
            pending_editor_open: false,
            pending_terminal_opener: None,
            open_with_index: 0,
            search_query: String::new(),
            is_searching: false,
            filtered_indices,
            viewer_search_query: String::new(),
            viewer_is_searching: false,
            viewer_search_matches: Vec::new(),
            viewer_search_idx: 0,
            viewer_goto_input: String::new(),
            viewer_is_goto: false,
            viewer_prev_key: None,
            git,
            fm_menu_idx: 0,
            fm_input: String::new(),
            fm_cursor: 0,
            fm_operation: None,
            fm_error: None,
            fm_overwrite_target: None,
            path_clipboard: Vec::new(),
            path_clipboard_idx: 0,
            file_list_height: 0,
            viewer_height: 0,
            status: None,
            drag_start: None,
            drag_end: None,
            drag_content_area: Rect::default(),
        })
    }

    pub fn set_status_success(&mut self, msg: impl Into<String>) {
        self.status = Some(StatusMessage::success(msg));
    }

    pub fn set_status_error(&mut self, msg: impl Into<String>) {
        self.status = Some(StatusMessage::error(msg));
    }

    /// 메인 이벤트 루프
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // 만료된 상태 메시지 제거
            if let Some(ref s) = self.status {
                if s.is_expired() {
                    self.status = None;
                }
            }
            // UI 렌더링
            terminal.draw(|f| {
                AppLayout::render(f, self);
            })?;

            // 이벤트 처리 (100ms 타임아웃)
            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key)?,
                    Event::Mouse(mouse) => self.handle_mouse(mouse)?,
                    Event::Resize(_, _) => {} // 자동 재렌더링
                    _ => {}
                }
            }

            // 비동기 git 명령 완료 폴링 (push/pull/fetch)
            self.poll_git_async();

            if self.pending_editor_open {
                self.pending_editor_open = false;
                self.open_external_editor_with_terminal(terminal)?;
            }

            if let Some((cmd, args)) = self.pending_terminal_opener.take() {
                self.run_terminal_program(terminal, &cmd, &args)?;
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    /// 키보드 이벤트 처리
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.mode == AppMode::FileList && self.focused_panel == FocusedPanel::Bookmarks {
            return self.handle_key_bookmarks(key);
        }
        if self.mode == AppMode::FileList && self.focused_panel == FocusedPanel::PathClipboard {
            return self.handle_key_clipboard_panel(key);
        }
        match self.mode {
            AppMode::FileList => self.handle_key_file_list(key),
            AppMode::Viewer => self.handle_key_viewer(key),
            AppMode::OpenWith => self.handle_key_open_with(key),
            AppMode::Settings => self.handle_key_settings(key),
            AppMode::CommandPalette => self.handle_key_palette(key),
            AppMode::Help => self.handle_key_help(key),
            AppMode::Git => self.handle_key_git(key),
            AppMode::FileManager => self.handle_key_file_manager(key),
            AppMode::PathClipboard => self.handle_key_path_clipboard(key),
        }
    }

    /// 파일 목록 모드 키 처리
    fn handle_key_file_list(&mut self, key: KeyEvent) -> Result<()> {
        // 검색 모드 입력 처리
        if self.is_searching {
            match key.code {
                KeyCode::Esc => {
                    self.is_searching = false;
                    self.search_query.clear();
                    self.update_filter();
                }
                KeyCode::Enter => {
                    self.is_searching = false;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.update_filter();
                }
                KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                KeyCode::PageUp => self.jump_up(10),
                KeyCode::PageDown => self.jump_down(10),
                KeyCode::Right | KeyCode::Char('l') => self.enter_or_open()?,
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.update_filter();
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::PageUp => {
                let n = self.file_list_height.max(1) as usize;
                self.jump_up(n);
            }
            KeyCode::PageDown => {
                let n = self.file_list_height.max(1) as usize;
                self.jump_down(n);
            }
            KeyCode::Left | KeyCode::Char('h') => self.go_parent(),
            KeyCode::Right | KeyCode::Char('l') => self.enter_or_open()?,
            KeyCode::Enter => self.open_with_or_navigate()?,
            KeyCode::Char(' ') => {
                self.clear_viewer_search();
                self.mode = AppMode::Viewer;
            }
            KeyCode::Char('?') => self.mode = AppMode::Help,
            KeyCode::Char('p') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.mode = AppMode::CommandPalette;
            }
            KeyCode::Char(',') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.mode = AppMode::Settings;
            }
            KeyCode::Char('/') => {
                self.is_searching = true;
                self.search_query.clear();
                self.update_filter();
            }
            KeyCode::Char('b') => self.toggle_bookmark(),
            KeyCode::Tab => {
                if !self.config.bookmarks.is_empty() {
                    self.focused_panel = FocusedPanel::Bookmarks;
                } else if !self.path_clipboard.is_empty() {
                    self.focused_panel = FocusedPanel::PathClipboard;
                }
            }
            KeyCode::Char('r') => {
                self.refresh_file_list();
            }
            KeyCode::Char('y') => {
                self.toggle_path_clipboard();
            }
            KeyCode::Char('m') => {
                if self.selected_path().is_some() {
                    self.fm_menu_idx = 0;
                    self.fm_operation = None;
                    self.fm_error = None;
                    self.fm_input.clear();
                    self.mode = AppMode::FileManager;
                }
            }
            KeyCode::Char('n') => {
                self.fm_start(FmOp::NewDir);
                self.mode = AppMode::FileManager;
            }
            KeyCode::Char('g') => {
                { let root = self.current_dir.clone(); self.git.refresh(&root); }
                self.git.staged_idx = 0;
                self.git.unstaged_idx = 0;
                self.git.is_committing = false;
                self.git.commit_input.clear();
                self.git.show_log = false;
                self.git.log.clear();
                self.git.log_focused = false;
                self.git.log_idx = 0;
                self.git.log_file_focused = false;
                self.git.commit_files.clear();
                self.git.commit_file_idx = 0;
                self.git.commit_show.clear();
                self.git.commit_show_scroll = 0;
                self.git.diff_h_scroll = 0;
                self.git.diff_wrap = false;
                self.git.diff_fullscreen = false;
                self.git.section = GitSection::Unstaged;
                self.mode = AppMode::Git;
                self.git.load_diff();
            }
            _ => {}
        }
        Ok(())
    }

    /// 검색 쿼리에 따라 filtered_indices 갱신
    fn update_filter(&mut self) {
        let q = self.search_query.to_lowercase();
        self.filtered_indices = if q.is_empty() {
            (0..self.file_entries.len()).collect()
        } else {
            self.file_entries.iter().enumerate()
                .filter(|(_, e)| e.name.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect()
        };
        self.selected_index = 0;
        self.preview_scroll = 0;
        self.preview_h_scroll = 0;
    }

    /// 전체화면 뷰어 모드 키 처리
    fn handle_key_viewer(&mut self, key: KeyEvent) -> Result<()> {
        // 검색 입력 모드
        if self.viewer_is_searching {
            match key.code {
                KeyCode::Esc => { self.viewer_is_searching = false; }
                KeyCode::Enter => {
                    self.viewer_is_searching = false;
                    self.viewer_do_search();
                    self.viewer_jump_to_match();
                }
                KeyCode::Backspace => { self.viewer_search_query.pop(); }
                KeyCode::Char(c) => { self.viewer_search_query.push(c); }
                _ => {}
            }
            return Ok(());
        }
        // 줄이동 입력 모드
        if self.viewer_is_goto {
            match key.code {
                KeyCode::Esc => {
                    self.viewer_is_goto = false;
                    self.viewer_goto_input.clear();
                }
                KeyCode::Enter => {
                    self.viewer_is_goto = false;
                    if let Ok(n) = self.viewer_goto_input.parse::<usize>() {
                        self.preview_scroll = n.saturating_sub(1) as u16;
                    }
                    self.viewer_goto_input.clear();
                }
                KeyCode::Backspace => { self.viewer_goto_input.pop(); }
                KeyCode::Char(c) if c.is_ascii_digit() => { self.viewer_goto_input.push(c); }
                _ => {}
            }
            return Ok(());
        }
        // 일반 키: prev_key 꺼내기 (gg 감지)
        let prev = self.viewer_prev_key.take();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.mode = AppMode::FileList;
                self.preview_scroll = 0;
                self.preview_h_scroll = 0;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                let n = self.viewer_height.max(1);
                self.preview_scroll = self.preview_scroll.saturating_sub(n);
            }
            KeyCode::PageDown | KeyCode::Char(' ') => {
                let n = self.viewer_height.max(1);
                self.preview_scroll = self.preview_scroll.saturating_add(n);
            }
            KeyCode::Left => self.preview_h_scroll = self.preview_h_scroll.saturating_sub(4),
            KeyCode::Right => self.preview_h_scroll = self.preview_h_scroll.saturating_add(4),
            KeyCode::Char('w') => {
                self.preview_wrap = !self.preview_wrap;
                if self.preview_wrap { self.preview_h_scroll = 0; }
                self.config.preview.wrap = self.preview_wrap;
                self.config.save().ok();
            }
            KeyCode::Char('e') => { self.pending_editor_open = true; }
            // 검색
            KeyCode::Char('/') => {
                self.viewer_search_query.clear();
                self.viewer_search_matches.clear();
                self.viewer_is_searching = true;
            }
            KeyCode::Char('n') => self.viewer_next_match(),
            KeyCode::Char('N') => self.viewer_prev_match(),
            // 줄이동
            KeyCode::Char(':') => {
                self.viewer_goto_input.clear();
                self.viewer_is_goto = true;
            }
            // G = 파일 끝으로
            KeyCode::Char('G') => {
                self.preview_scroll = u16::MAX / 2;
            }
            // g g = 파일 처음으로
            KeyCode::Char('g') => {
                if prev == Some(KeyCode::Char('g')) {
                    self.preview_scroll = 0;
                    self.preview_h_scroll = 0;
                } else {
                    self.viewer_prev_key = Some(KeyCode::Char('g'));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// 즐겨찾기 패널 포커스 상태의 키 처리
    fn handle_key_bookmarks(&mut self, key: KeyEvent) -> Result<()> {
        let len = self.config.bookmarks.len();
        match key.code {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                self.focused_panel = FocusedPanel::FileList;
            }
            KeyCode::Tab => {
                if !self.path_clipboard.is_empty() {
                    self.focused_panel = FocusedPanel::PathClipboard;
                } else {
                    self.focused_panel = FocusedPanel::FileList;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if len > 0 && self.bookmark_index > 0 {
                    self.bookmark_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 && self.bookmark_index + 1 < len {
                    self.bookmark_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.navigate_to_bookmark()?;
            }
            KeyCode::Delete | KeyCode::Char('d') => {
                self.bookmark_delete();
            }
            _ => {}
        }
        Ok(())
    }

    /// 경로 클립보드 사이드바 패널 포커스 상태의 키 처리
    fn handle_key_clipboard_panel(&mut self, key: KeyEvent) -> Result<()> {
        let len = self.path_clipboard.len();
        match key.code {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                self.focused_panel = FocusedPanel::FileList;
            }
            KeyCode::Tab => {
                self.focused_panel = FocusedPanel::FileList;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.path_clipboard_idx > 0 {
                    self.path_clipboard_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 && self.path_clipboard_idx + 1 < len {
                    self.path_clipboard_idx += 1;
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(path) = self.path_clipboard.get(self.path_clipboard_idx).cloned() {
                    let dir = if path.is_dir() {
                        path
                    } else {
                        path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| path.clone())
                    };
                    if dir.is_dir() {
                        self.navigate_to(dir)?;
                    }
                    self.focused_panel = FocusedPanel::FileList;
                }
            }
            KeyCode::Char('d') => {
                if !self.path_clipboard.is_empty() {
                    self.path_clipboard.remove(self.path_clipboard_idx);
                    if self.path_clipboard_idx > 0
                        && self.path_clipboard_idx >= self.path_clipboard.len()
                    {
                        self.path_clipboard_idx = self.path_clipboard.len().saturating_sub(1);
                    }
                    if self.path_clipboard.is_empty() {
                        self.focused_panel = FocusedPanel::FileList;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// OpenWith 오버레이 키 처리
    fn handle_key_open_with(&mut self, key: KeyEvent) -> Result<()> {
        let total = 1 + self.config.openers.len(); // 기본 앱 + 등록된 프로그램
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = AppMode::FileList;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.open_with_index > 0 {
                    self.open_with_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.open_with_index + 1 < total {
                    self.open_with_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.execute_open_with()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_settings(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
            self.mode = AppMode::FileList;
        }
        Ok(())
    }

    fn handle_key_palette(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Esc {
            self.mode = AppMode::FileList;
        }
        Ok(())
    }

    fn handle_key_help(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('?') || key.code == KeyCode::Char('q') {
            self.mode = AppMode::FileList;
        }
        Ok(())
    }

    /// Git 관리 모드 키 처리
    fn handle_key_git(&mut self, key: KeyEvent) -> Result<()> {
        // ORDER MATTERS — modals absorb keys first.
        // 1) async progress modal — only Esc (kill child) is meaningful
        if self.git.async_kind.is_some() { return self.handle_git_async_progress(key); }
        // 2) confirm yes/no modal
        if self.git.confirm.is_some() { return self.handle_git_confirm(key); }
        // 3) branch creation input modal
        if self.git.branch_input_active { return self.handle_git_branch_input(key); }
        // 4) commit message input modal
        if self.git.is_committing { return self.handle_git_commit_input(key); }
        // 5) branch panel (when open and focused)
        if self.git.branch_panel_open { return self.handle_git_branch_panel(key); }
        // 6) git-wide common keys ([ ] w f) — early return if handled
        if self.handle_git_common_keys(key)? { return Ok(()); }
        // 7) submode dispatch
        if self.git.diff_fullscreen { return self.handle_git_fullscreen(key); }
        if self.git.log_focused && self.git.log_file_focused { return self.handle_git_log_files(key); }
        if self.git.log_focused { return self.handle_git_log(key); }
        self.handle_git_files(key)
    }

    fn handle_git_async_progress(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Esc {
            if let Some(mut child) = self.git.async_child.take() {
                let _ = child.kill();
            }
            self.git.async_kind = None;
            self.git.async_started_at = None;
            self.set_status_error("취소됨");
        }
        Ok(())
    }

    fn handle_git_confirm(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(kind) = self.git.confirm.take() {
                    self.execute_confirmed(kind);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.git.confirm = None;
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_confirmed(&mut self, kind: crate::state::ConfirmKind) {
        use crate::state::ConfirmKind;
        match kind {
            ConfirmKind::DeleteBranchSoft(name) => {
                if let Some(ref status) = self.git.status {
                    let root = status.root.clone();
                    match crate::git::delete_branch(&root, &name, false) {
                        Ok(()) => self.set_status_success(format!("브랜치 삭제됨: {name}")),
                        Err(e) => self.set_status_error(e),
                    }
                    self.git.branches = crate::git::list_branches(&root);
                    let root2 = root.clone();
                    self.git.refresh(&root2);
                } else {
                    let root = self.current_dir.clone();
                    self.git.refresh(&root);
                }
            }
            ConfirmKind::DeleteBranchForce(name) => {
                if let Some(ref status) = self.git.status {
                    let root = status.root.clone();
                    match crate::git::delete_branch(&root, &name, true) {
                        Ok(()) => self.set_status_success(format!("브랜치 강제 삭제됨: {name}")),
                        Err(e) => self.set_status_error(e),
                    }
                    self.git.branches = crate::git::list_branches(&root);
                    let root2 = root.clone();
                    self.git.refresh(&root2);
                } else {
                    let root = self.current_dir.clone();
                    self.git.refresh(&root);
                }
            }
            ConfirmKind::CheckoutFile(path) => {
                if let Some(ref status) = self.git.status {
                    let root = status.root.clone();
                    match crate::git::restore_file(&root, &path) {
                        Ok(()) => self.set_status_success(format!("되돌림: {path}")),
                        Err(e) => self.set_status_error(e),
                    }
                    let root2 = root.clone();
                    self.git.refresh(&root2);
                    self.git.load_diff();
                } else {
                    let root = self.current_dir.clone();
                    self.git.refresh(&root);
                    self.git.load_diff();
                }
            }
            ConfirmKind::ForcePush(branch) => {
                self.start_git_async(crate::state::AsyncKind::Push { force: true, branch });
            }
        }
    }

    fn handle_git_branch_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.git.branch_input_active = false;
                self.git.branch_input.clear();
            }
            KeyCode::Enter => {
                let name = self.git.branch_input.trim().to_string();
                if !name.is_empty() {
                    if let Some(ref status) = self.git.status {
                        let root = status.root.clone();
                        match crate::git::create_branch(&root, &name) {
                            Ok(()) => self.set_status_success(format!("브랜치 생성: {name}")),
                            Err(e) => self.set_status_error(e),
                        }
                        self.git.branches = crate::git::list_branches(&root);
                        let current = self.git.branches.iter()
                            .position(|b| b.is_current)
                            .unwrap_or(0);
                        self.git.branch_idx = current;
                        let root2 = root.clone();
                        self.git.refresh(&root2);
                    }
                }
                self.git.branch_input_active = false;
                self.git.branch_input.clear();
            }
            KeyCode::Backspace => { self.git.branch_input.pop(); }
            KeyCode::Char(c) => { self.git.branch_input.push(c); }
            _ => {}
        }
        Ok(())
    }

    fn handle_git_branch_panel(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('b') | KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                self.git.branch_panel_open = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.git.branch_idx > 0 {
                    self.git.branch_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.git.branch_idx + 1 < self.git.branches.len() {
                    self.git.branch_idx += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('o') => {
                let branches = &self.git.branches;
                if let Some(branch) = branches.get(self.git.branch_idx) {
                    if branch.is_current {
                        return Ok(());
                    }
                    let name = branch.name.clone();
                    if let Some(ref status) = self.git.status {
                        let root = status.root.clone();
                        match crate::git::switch_branch(&root, &name) {
                            Ok(()) => self.set_status_success(format!("전환됨: {name}")),
                            Err(e) => self.set_status_error(e),
                        }
                        self.git.branches = crate::git::list_branches(&root);
                        let current = self.git.branches.iter()
                            .position(|b| b.is_current)
                            .unwrap_or(0);
                        self.git.branch_idx = current;
                        let root2 = root.clone();
                        self.git.refresh(&root2);
                        self.git.load_diff();
                    }
                }
            }
            KeyCode::Char('n') => {
                self.git.branch_input_active = true;
                self.git.branch_input.clear();
            }
            KeyCode::Char('d') => {
                if let Some(branch) = self.git.branches.get(self.git.branch_idx) {
                    if !branch.is_current && !branch.is_remote {
                        let name = branch.name.clone();
                        self.git.confirm = Some(crate::state::ConfirmKind::DeleteBranchSoft(name));
                    }
                }
            }
            KeyCode::Char('D') => {
                if let Some(branch) = self.git.branches.get(self.git.branch_idx) {
                    if !branch.is_current && !branch.is_remote {
                        let name = branch.name.clone();
                        self.git.confirm = Some(crate::state::ConfirmKind::DeleteBranchForce(name));
                    }
                }
            }
            KeyCode::Char('r') => {
                if let Some(ref status) = self.git.status {
                    let root = status.root.clone();
                    self.git.branches = crate::git::list_branches(&root);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn start_git_async(&mut self, kind: crate::state::AsyncKind) {
        if self.git.async_kind.is_some() {
            self.set_status_error("이미 실행 중인 명령이 있습니다");
            return;
        }
        let root = match self.git.status.as_ref() {
            Some(s) => s.root.clone(),
            None => return,
        };
        let root_str = match root.to_str() {
            Some(s) => s.to_string(),
            None => return,
        };
        let args: Vec<String> = match &kind {
            crate::state::AsyncKind::Push { force, branch } => {
                crate::git::push_args(branch, *force)
            }
            crate::state::AsyncKind::Pull => {
                crate::git::pull_args().iter().map(|s| s.to_string()).collect()
            }
            crate::state::AsyncKind::Fetch => {
                crate::git::fetch_args().iter().map(|s| s.to_string()).collect()
            }
        };
        match std::process::Command::new("git")
            .args(["-C", &root_str])
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => {
                self.git.async_child = Some(child);
                self.git.async_kind = Some(kind);
                self.git.async_started_at = Some(Instant::now());
                self.git.spinner_tick = 0;
            }
            Err(e) => self.set_status_error(format!("실행 실패: {e}")),
        }
    }

    pub fn poll_git_async(&mut self) {
        if self.git.async_kind.is_none() {
            return;
        }
        self.git.spinner_tick = self.git.spinner_tick.wrapping_add(1);
        let done = match self.git.async_child.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(Some(_status)) => Some(_status),
                Ok(None) => None,
                Err(_) => {
                    self.git.async_child = None;
                    self.git.async_kind = None;
                    self.git.async_started_at = None;
                    self.set_status_error("프로세스 오류");
                    return;
                }
            },
            None => {
                self.git.async_kind = None;
                return;
            }
        };
        if let Some(exit_status) = done {
            let mut child = self.git.async_child.take().unwrap();
            let mut err_buf = String::new();
            if let Some(mut stderr) = child.stderr.take() {
                let _ = stderr.read_to_string(&mut err_buf);
            }
            let kind = self.git.async_kind.take().unwrap();
            self.git.async_started_at = None;
            let label = match &kind {
                crate::state::AsyncKind::Push { force: true, .. } => "force push",
                crate::state::AsyncKind::Push { .. } => "push",
                crate::state::AsyncKind::Pull => "pull",
                crate::state::AsyncKind::Fetch => "fetch",
            };
            if exit_status.success() {
                self.set_status_success(format!("{label} 완료"));
            } else {
                let msg = err_buf.trim().to_string();
                let short = if msg.len() > 80 { msg[..80].to_string() } else { msg };
                self.set_status_error(format!("{label} 실패: {short}"));
            }
            let root = self.current_dir.clone();
            self.git.refresh(&root);
            if let Some(ref status) = self.git.status {
                let r = status.root.clone();
                self.git.branches = crate::git::list_branches(&r);
            }
            self.git.load_diff();
        }
    }

    fn handle_git_commit_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.git.is_committing = false;
                self.git.commit_input.clear();
            }
            KeyCode::Enter => {
                if !self.git.commit_input.is_empty() {
                    if let Some(status) = &self.git.status {
                        let root = status.root.clone();
                        let msg = self.git.commit_input.clone();
                        match crate::git::commit_changes(&root, &msg) {
                            Ok(()) => self.set_status_success("커밋됨"),
                            Err(e) => self.set_status_error(e),
                        }
                    }
                    self.git.is_committing = false;
                    self.git.commit_input.clear();
                    { let root = self.current_dir.clone(); self.git.refresh(&root); }
                    self.git.load_diff();
                }
            }
            KeyCode::Backspace => { self.git.commit_input.pop(); }
            KeyCode::Char(c) => { self.git.commit_input.push(c); }
            _ => {}
        }
        Ok(())
    }

    fn handle_git_common_keys(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('[') => {
                if !self.git.diff_wrap {
                    self.git.diff_h_scroll = self.git.diff_h_scroll.saturating_sub(4);
                }
                Ok(true)
            }
            KeyCode::Char(']') => {
                if !self.git.diff_wrap {
                    self.git.diff_h_scroll += 4;
                }
                Ok(true)
            }
            KeyCode::Char('w') => {
                self.git.diff_wrap = !self.git.diff_wrap;
                if self.git.diff_wrap {
                    self.git.diff_h_scroll = 0;
                }
                Ok(true)
            }
            KeyCode::Char('f') => {
                let has_diff = !self.git.diff.is_empty() || !self.git.commit_show.is_empty();
                if has_diff {
                    self.git.diff_fullscreen = true;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn handle_git_fullscreen(&mut self, key: KeyEvent) -> Result<()> {
        let has_commit_diff = !self.git.commit_show.is_empty();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('f') => {
                self.git.diff_fullscreen = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if has_commit_diff {
                    self.git.commit_show_scroll = self.git.commit_show_scroll.saturating_sub(1);
                } else {
                    self.git.diff_scroll = self.git.diff_scroll.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if has_commit_diff {
                    self.git.commit_show_scroll += 1;
                } else {
                    self.git.diff_scroll += 1;
                }
            }
            KeyCode::PageUp => {
                let n = self.git.diff_panel_height.max(1);
                if has_commit_diff {
                    self.git.commit_show_scroll = self.git.commit_show_scroll.saturating_sub(n);
                } else {
                    self.git.diff_scroll = self.git.diff_scroll.saturating_sub(n);
                }
            }
            KeyCode::PageDown | KeyCode::Char(' ') => {
                let n = self.git.diff_panel_height.max(1);
                if has_commit_diff {
                    self.git.commit_show_scroll += n;
                } else {
                    self.git.diff_scroll += n;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_git_log_files(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                self.git.log_file_focused = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.git.commit_file_idx > 0 {
                    self.git.commit_file_idx -= 1;
                    self.git.load_commit_file_diff();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.git.commit_file_idx + 1 < self.git.commit_files.len() {
                    self.git.commit_file_idx += 1;
                    self.git.load_commit_file_diff();
                }
            }
            KeyCode::Enter | KeyCode::Char('d') => {
                self.git.load_commit_file_diff();
            }
            KeyCode::PageUp => {
                let n = self.git.diff_panel_height.max(1);
                self.git.commit_show_scroll = self.git.commit_show_scroll.saturating_sub(n);
            }
            KeyCode::PageDown => {
                let n = self.git.diff_panel_height.max(1);
                self.git.commit_show_scroll = self.git.commit_show_scroll.saturating_add(n);
            }
            KeyCode::Char('L') => {
                self.git.show_log = false;
                self.git.log_focused = false;
                self.git.log_file_focused = false;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_git_log(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                self.git.log_focused = false;
                self.git.log_file_focused = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.git.log_idx > 0 {
                    self.git.log_idx -= 1;
                    self.git.log_file_focused = false;
                    self.git.load_commit_show();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.git.log_idx + 1 < self.git.log.len() {
                    self.git.log_idx += 1;
                    self.git.log_file_focused = false;
                    self.git.load_commit_show();
                }
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter | KeyCode::Char('d') => {
                if !self.git.commit_files.is_empty() {
                    self.git.log_file_focused = true;
                    self.git.commit_file_idx = 0;
                    self.git.load_commit_file_diff();
                } else {
                    self.git.load_commit_show();
                }
            }
            KeyCode::PageUp => {
                let n = self.git.diff_panel_height.max(1);
                self.git.commit_show_scroll = self.git.commit_show_scroll.saturating_sub(n);
            }
            KeyCode::PageDown => {
                let n = self.git.diff_panel_height.max(1);
                self.git.commit_show_scroll = self.git.commit_show_scroll.saturating_add(n);
            }
            KeyCode::Char('L') => {
                self.git.show_log = false;
                self.git.log_focused = false;
                self.git.log_file_focused = false;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_git_files(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = AppMode::FileList;
            }
            KeyCode::Tab => {
                self.git.section = match self.git.section {
                    GitSection::Staged => GitSection::Unstaged,
                    GitSection::Unstaged => GitSection::Staged,
                };
                self.git.load_diff();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.git.show_log && !self.git.log.is_empty() {
                    self.git.log_focused = true;
                    self.git.log_file_focused = false;
                    self.git.load_commit_show();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                match self.git.section {
                    GitSection::Staged => {
                        if self.git.staged_idx > 0 { self.git.staged_idx -= 1; }
                    }
                    GitSection::Unstaged => {
                        if self.git.unstaged_idx > 0 { self.git.unstaged_idx -= 1; }
                    }
                }
                self.git.load_diff();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref status) = self.git.status {
                    match self.git.section {
                        GitSection::Staged => {
                            if self.git.staged_idx + 1 < status.staged.len() {
                                self.git.staged_idx += 1;
                            }
                        }
                        GitSection::Unstaged => {
                            if self.git.unstaged_idx + 1 < status.unstaged.len() {
                                self.git.unstaged_idx += 1;
                            }
                        }
                    }
                }
                self.git.load_diff();
            }
            KeyCode::Char('a') => {
                if self.git.section == GitSection::Unstaged {
                    if let Some(ref status) = self.git.status {
                        if let Some(file) = status.unstaged.get(self.git.unstaged_idx) {
                            let root = status.root.clone();
                            let path = file.path.clone();
                            match crate::git::stage_file(&root, &path) {
                                Ok(()) => self.set_status_success(format!("스테이지됨: {path}")),
                                Err(e) => self.set_status_error(e),
                            }
                        }
                    }
                    { let root = self.current_dir.clone(); self.git.refresh(&root); }
                    self.git.load_diff();
                }
            }
            KeyCode::Char('u') => {
                if self.git.section == GitSection::Staged {
                    if let Some(ref status) = self.git.status {
                        if let Some(file) = status.staged.get(self.git.staged_idx) {
                            let root = status.root.clone();
                            let path = file.path.clone();
                            match crate::git::unstage_file(&root, &path) {
                                Ok(()) => self.set_status_success(format!("언스테이지됨: {path}")),
                                Err(e) => self.set_status_error(e),
                            }
                        }
                    }
                    { let root = self.current_dir.clone(); self.git.refresh(&root); }
                    self.git.load_diff();
                }
            }
            KeyCode::Enter | KeyCode::Char('d') => {
                self.git.show_log = false;
                self.git.log_focused = false;
                self.git.load_diff();
            }
            KeyCode::Char('c') => {
                let has_staged = self.git.status.as_ref()
                    .map(|s| !s.staged.is_empty())
                    .unwrap_or(false);
                if has_staged {
                    self.git.is_committing = true;
                    self.git.commit_input.clear();
                }
            }
            KeyCode::Char('L') => {
                self.git.show_log = !self.git.show_log;
                if self.git.show_log {
                    if let Some(ref status) = self.git.status {
                        let root = status.root.clone();
                        self.git.log = crate::git::get_log(&root);
                    }
                    self.git.log_idx = 0;
                    self.git.log_focused = false;
                    self.git.log_file_focused = false;
                    self.git.commit_files.clear();
                    self.git.commit_file_idx = 0;
                    self.git.commit_show.clear();
                    self.git.commit_show_scroll = 0;
                } else {
                    self.git.log_focused = false;
                }
            }
            KeyCode::Char('r') => {
                { let root = self.current_dir.clone(); self.git.refresh(&root); }
                self.git.load_diff();
            }
            KeyCode::Char('b') => {
                if let Some(ref status) = self.git.status {
                    let root = status.root.clone();
                    self.git.branches = crate::git::list_branches(&root);
                }
                self.git.branch_idx = self.git.branches.iter()
                    .position(|b| b.is_current)
                    .unwrap_or(0);
                self.git.branch_panel_open = true;
            }
            KeyCode::Char('X') => {
                if self.git.section == crate::app::GitSection::Unstaged {
                    if let Some(ref status) = self.git.status {
                        if let Some(file) = status.unstaged.get(self.git.unstaged_idx) {
                            let path = file.path.clone();
                            self.git.confirm = Some(crate::state::ConfirmKind::CheckoutFile(path));
                        }
                    }
                }
            }
            KeyCode::Char('P') => {
                let branch = self.git.status.as_ref()
                    .map(|s| s.branch.clone())
                    .unwrap_or_default();
                if branch == "HEAD" || branch.is_empty() {
                    self.set_status_error("detached HEAD: push 불가");
                } else {
                    self.start_git_async(crate::state::AsyncKind::Push { force: false, branch });
                }
            }
            KeyCode::Char('!') => {
                let branch = self.git.status.as_ref()
                    .map(|s| s.branch.clone())
                    .unwrap_or_default();
                if branch == "HEAD" || branch.is_empty() {
                    self.set_status_error("detached HEAD: push 불가");
                } else {
                    self.git.confirm = Some(crate::state::ConfirmKind::ForcePush(branch));
                }
            }
            KeyCode::Char('p') => {
                let branch = self.git.status.as_ref()
                    .map(|s| s.branch.clone())
                    .unwrap_or_default();
                if branch == "HEAD" || branch.is_empty() {
                    self.set_status_error("detached HEAD: pull 불가");
                } else {
                    self.start_git_async(crate::state::AsyncKind::Pull);
                }
            }
            KeyCode::Char('F') => {
                self.start_git_async(crate::state::AsyncKind::Fetch);
            }
            KeyCode::PageUp => {
                let n = self.git.diff_panel_height.max(1);
                self.git.diff_scroll = self.git.diff_scroll.saturating_sub(n);
            }
            KeyCode::PageDown => {
                let n = self.git.diff_panel_height.max(1);
                self.git.diff_scroll = self.git.diff_scroll.saturating_add(n);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_file_manager(&mut self, key: KeyEvent) -> Result<()> {
        if self.fm_overwrite_target.is_some() {
            return self.handle_fm_overwrite_confirm(key);
        }
        match self.fm_operation.clone() {
            None => self.handle_fm_menu(key),
            Some(FmOp::Delete) => self.handle_fm_delete(key),
            Some(_) => self.handle_fm_input(key),
        }
    }

    fn handle_fm_overwrite_confirm(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') => {
                let dst = match self.fm_overwrite_target.take() {
                    Some(d) => d,
                    None => return Ok(()),
                };
                let src = match self.selected_path().cloned() {
                    Some(p) => p,
                    None => return Ok(()),
                };
                let input = dst.display().to_string();
                let result = match self.fm_operation {
                    Some(FmOp::Copy) => crate::fs::ops::copy_file(&src, &dst),
                    Some(FmOp::Move) => crate::fs::ops::move_file(&src, &dst),
                    _ => return Ok(()),
                };
                match result {
                    Ok(_) => {
                        let op_label = if self.fm_operation == Some(FmOp::Copy) { "복사됨" } else { "이동됨" };
                        self.set_status_success(format!("{op_label}: {input}"));
                        self.fm_operation = None;
                        self.fm_error = None;
                        self.mode = AppMode::FileList;
                        self.fm_refresh_file_list();
                    }
                    Err(e) => {
                        self.fm_error = Some(e.to_string());
                        self.set_status_error(format!("실패: {e}"));
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.fm_overwrite_target = None;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_fm_menu(&mut self, key: KeyEvent) -> Result<()> {
        const MENU_LEN: usize = 5;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = AppMode::FileList;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.fm_menu_idx > 0 { self.fm_menu_idx -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.fm_menu_idx + 1 < MENU_LEN { self.fm_menu_idx += 1; }
            }
            KeyCode::Char('c') => self.fm_start(FmOp::Copy),
            KeyCode::Char('v') => self.fm_start(FmOp::Move),
            KeyCode::Char('r') => self.fm_start(FmOp::Rename),
            KeyCode::Char('d') => self.fm_start(FmOp::Delete),
            KeyCode::Char('n') => self.fm_start(FmOp::NewDir),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                let op = match self.fm_menu_idx {
                    0 => FmOp::Copy,
                    1 => FmOp::Move,
                    2 => FmOp::Rename,
                    3 => FmOp::Delete,
                    _ => FmOp::NewDir,
                };
                self.fm_start(op);
            }
            _ => {}
        }
        Ok(())
    }

    fn fm_start(&mut self, op: FmOp) {
        self.fm_error = None;
        self.fm_menu_idx = match op {
            FmOp::Copy => 0,
            FmOp::Move => 1,
            FmOp::Rename => 2,
            FmOp::Delete => 3,
            FmOp::NewDir => 4,
        };
        let input = match op {
            FmOp::Rename => self
                .selected_path()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string(),
            FmOp::Copy | FmOp::Move => {
                format!("{}/", self.current_dir.display())
            }
            FmOp::Delete | FmOp::NewDir => String::new(),
        };
        self.fm_cursor = input.len();
        self.fm_input = input;
        self.fm_operation = Some(op);
    }

    fn handle_fm_delete(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(path) = self.selected_path().cloned() {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("(알 수 없음)")
                        .to_string();
                    match crate::fs::ops::delete_file(&path) {
                        Ok(_) => {
                            self.fm_operation = None;
                            self.mode = AppMode::FileList;
                            self.fm_refresh_file_list();
                            self.set_status_success(format!("삭제됨: {name}"));
                        }
                        Err(e) => {
                            self.fm_error = Some(e.to_string());
                            self.set_status_error(format!("삭제 실패: {e}"));
                        }
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.fm_operation = None;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_fm_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.fm_operation = None;
                self.fm_error = None;
            }
            KeyCode::Left => {
                self.fm_cursor = Self::prev_char_boundary(&self.fm_input, self.fm_cursor);
            }
            KeyCode::Right => {
                self.fm_cursor = Self::next_char_boundary(&self.fm_input, self.fm_cursor);
            }
            KeyCode::Home => {
                self.fm_cursor = 0;
            }
            KeyCode::End => {
                self.fm_cursor = self.fm_input.len();
            }
            KeyCode::Backspace => {
                if self.fm_cursor > 0 {
                    let prev = Self::prev_char_boundary(&self.fm_input, self.fm_cursor);
                    self.fm_input.drain(prev..self.fm_cursor);
                    self.fm_cursor = prev;
                }
            }
            KeyCode::Delete => {
                if self.fm_cursor < self.fm_input.len() {
                    let next = Self::next_char_boundary(&self.fm_input, self.fm_cursor);
                    self.fm_input.drain(self.fm_cursor..next);
                }
            }
            KeyCode::Tab => {
                // Copy/Move 에서만 경로 클립보드 오버레이 열기
                let is_path_op = matches!(self.fm_operation, Some(FmOp::Copy) | Some(FmOp::Move));
                if is_path_op && !self.path_clipboard.is_empty() {
                    self.path_clipboard_idx = 0;
                    self.mode = AppMode::PathClipboard;
                }
            }
            KeyCode::Char(c) => {
                self.fm_input.insert(self.fm_cursor, c);
                self.fm_cursor += c.len_utf8();
            }
            KeyCode::Enter => self.execute_fm_operation()?,
            _ => {}
        }
        Ok(())
    }

    fn prev_char_boundary(s: &str, pos: usize) -> usize {
        if pos == 0 { return 0; }
        let mut i = pos - 1;
        while i > 0 && !s.is_char_boundary(i) {
            i -= 1;
        }
        i
    }

    fn next_char_boundary(s: &str, pos: usize) -> usize {
        if pos >= s.len() { return s.len(); }
        let mut i = pos + 1;
        while i < s.len() && !s.is_char_boundary(i) {
            i += 1;
        }
        i
    }

    fn execute_fm_operation(&mut self) -> Result<()> {
        let src = match self.selected_path().cloned() {
            Some(p) => p,
            None => return Ok(()),
        };
        let input = self.fm_input.trim().to_string();
        if input.is_empty() {
            self.fm_error = Some("경로를 입력하세요.".to_string());
            return Ok(());
        }

        let op = self.fm_operation.clone();

        // Copy/Move: 대상이 디렉토리면 원본 파일명을 붙여 하위 경로로 해소
        let resolved = if matches!(op, Some(FmOp::Copy) | Some(FmOp::Move)) {
            let dst = std::path::PathBuf::from(&input);
            if dst.is_dir() {
                if let Some(name) = src.file_name() {
                    dst.join(name)
                } else {
                    dst
                }
            } else {
                dst
            }
        } else {
            std::path::PathBuf::from(&input)
        };

        // 해소된 경로가 이미 존재하면 덮어쓰기 확인
        if matches!(op, Some(FmOp::Copy) | Some(FmOp::Move)) && resolved.exists() {
            self.fm_overwrite_target = Some(resolved);
            return Ok(());
        }

        let result = match op {
            Some(FmOp::Rename) => crate::fs::ops::rename_file(&src, &input).map(|_| ()),
            Some(FmOp::Copy) => crate::fs::ops::copy_file(&src, &resolved),
            Some(FmOp::Move) => crate::fs::ops::move_file(&src, &resolved),
            Some(FmOp::NewDir) => {
                crate::fs::ops::create_dir(&self.current_dir, &input).map(|_| ())
            }
            _ => return Ok(()),
        };

        let display_name = resolved.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&input)
            .to_string();

        match result {
            Ok(_) => {
                let op_label = match self.fm_operation {
                    Some(FmOp::Rename) => "이름 변경됨",
                    Some(FmOp::Copy) => "복사됨",
                    Some(FmOp::Move) => "이동됨",
                    Some(FmOp::NewDir) => "생성됨",
                    _ => "완료됨",
                };
                self.set_status_success(format!("{op_label}: {display_name}"));
                self.fm_operation = None;
                self.fm_error = None;
                self.mode = AppMode::FileList;
                self.fm_refresh_file_list();
            }
            Err(e) => {
                self.fm_error = Some(e.to_string());
                self.set_status_error(format!("실패: {e}"));
            }
        }
        Ok(())
    }

    fn fm_refresh_file_list(&mut self) {
        self.file_entries = crate::fs::ops::list_dir(&self.current_dir).unwrap_or_default();
        self.search_query.clear();
        self.is_searching = false;
        self.filtered_indices = (0..self.file_entries.len()).collect();
        self.selected_index = self.selected_index.min(
            self.filtered_indices.len().saturating_sub(1)
        );
        self.preview_scroll = 0;
        self.preview_h_scroll = 0;
        { let root = self.current_dir.clone(); self.git.status = crate::git::get_status(&root); }
    }

    fn refresh_file_list(&mut self) {
        let prev_selected_path = self.selected_path().cloned();
        self.file_entries = crate::fs::ops::list_dir(&self.current_dir).unwrap_or_default();
        { let root = self.current_dir.clone(); self.git.status = crate::git::get_status(&root); }
        self.update_filter();
        // 이전에 선택된 파일이 여전히 존재하면 해당 위치로 커서 복원
        if let Some(prev) = prev_selected_path {
            if let Some(pos) = self.filtered_indices.iter().position(|&i| {
                self.file_entries.get(i).map(|e| e.path == prev).unwrap_or(false)
            }) {
                self.selected_index = pos;
            }
        }
    }

    /// 마우스 이벤트 처리
    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(3);
            }
            MouseEventKind::ScrollDown => {
                self.preview_scroll = self.preview_scroll.saturating_add(3);
            }
            MouseEventKind::Down(btn) => {
                if btn == MouseButton::Left {
                    let ca = self.drag_content_area;
                    if ca.width > 0 && ca.height > 0
                        && mouse.column >= ca.x && mouse.column < ca.x + ca.width
                        && mouse.row >= ca.y && mouse.row < ca.y + ca.height
                    {
                        self.drag_start = Some((mouse.column, mouse.row));
                        self.drag_end = None;
                    } else {
                        self.drag_start = None;
                        self.drag_end = None;
                    }
                }
                // 즐겨찾기 패널 클릭
                if let Some(area) = self.bookmarks_area {
                    if mouse.column >= area.x && mouse.column < area.x + area.width
                        && mouse.row >= area.y && mouse.row < area.y + area.height
                    {
                        let row = mouse.row.saturating_sub(area.y + 1) as usize;
                        if row < self.config.bookmarks.len() {
                            self.bookmark_index = row;
                            self.navigate_to_bookmark()?;
                        }
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.drag_start.is_some() {
                    self.drag_end = Some((mouse.column, mouse.row));
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.drag_start.is_some() && self.drag_end.is_some() {
                    self.copy_drag_selection();
                } else {
                    self.drag_start = None;
                    self.drag_end = None;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// 드래그 선택 범위의 텍스트를 클립보드에 복사
    fn copy_drag_selection(&mut self) {
        let (start, end) = match (self.drag_start.take(), self.drag_end.take()) {
            (Some(s), Some(e)) => (s, e),
            _ => return,
        };
        let ca = self.drag_content_area;
        if ca.height == 0 { return; }

        let start_line = (start.1.saturating_sub(ca.y) as usize) + self.preview_scroll as usize;
        let end_line = (end.1.saturating_sub(ca.y) as usize) + self.preview_scroll as usize;
        let (s, e) = if start_line <= end_line { (start_line, end_line) } else { (end_line, start_line) };

        let path = match self.selected_path().cloned() {
            Some(p) if p.is_file() => p,
            _ => return,
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let selected: String = content.lines()
            .enumerate()
            .filter(|(i, _)| *i >= s && *i <= e)
            .map(|(_, l)| l)
            .collect::<Vec<_>>()
            .join("\n");

        if selected.is_empty() { return; }

        self.write_to_clipboard(&selected);
        let count = e - s + 1;
        self.set_status_success(format!("{count}줄 복사됨"));
    }

    /// 텍스트를 시스템 클립보드에 기록
    fn write_to_clipboard(&self, text: &str) {
        use std::io::Write as _;
        #[cfg(target_os = "macos")]
        {
            if let Ok(mut child) = std::process::Command::new("pbcopy")
                .stdin(Stdio::piped())
                .spawn()
            {
                if let Some(stdin) = child.stdin.as_mut() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
            }
        }
        #[cfg(target_os = "linux")]
        {
            let ok = std::process::Command::new("xclip")
                .args(["-selection", "clipboard"])
                .stdin(Stdio::piped())
                .spawn()
                .ok()
                .and_then(|mut c| {
                    if let Some(stdin) = c.stdin.as_mut() { let _ = stdin.write_all(text.as_bytes()); }
                    c.wait().ok()
                })
                .map(|s| s.success())
                .unwrap_or(false);
            if !ok {
                if let Ok(mut child) = std::process::Command::new("xsel")
                    .args(["--clipboard", "--input"])
                    .stdin(Stdio::piped())
                    .spawn()
                {
                    if let Some(stdin) = child.stdin.as_mut() {
                        let _ = stdin.write_all(text.as_bytes());
                    }
                    let _ = child.wait();
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Ok(mut child) = std::process::Command::new("clip")
                .stdin(Stdio::piped())
                .spawn()
            {
                if let Some(stdin) = child.stdin.as_mut() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
            }
        }
    }

    /// 위로 이동
    fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.preview_scroll = 0;
            self.preview_h_scroll = 0;
        }
    }

    /// 아래로 이동
    fn move_down(&mut self) {
        if self.selected_index + 1 < self.filtered_indices.len() {
            self.selected_index += 1;
            self.preview_scroll = 0;
            self.preview_h_scroll = 0;
        }
    }

    /// n칸 위로 점프
    fn jump_up(&mut self, n: usize) {
        self.selected_index = self.selected_index.saturating_sub(n);
        self.preview_scroll = 0;
        self.preview_h_scroll = 0;
    }

    /// n칸 아래로 점프
    fn jump_down(&mut self, n: usize) {
        let max = self.filtered_indices.len().saturating_sub(1);
        self.selected_index = (self.selected_index + n).min(max);
        self.preview_scroll = 0;
        self.preview_h_scroll = 0;
    }

    /// 뷰어 검색 상태 초기화
    fn clear_viewer_search(&mut self) {
        self.viewer_search_query.clear();
        self.viewer_search_matches.clear();
        self.viewer_search_idx = 0;
        self.viewer_prev_key = None;
    }

    /// 현재 파일에서 viewer_search_query로 검색해 매칭 줄 목록 갱신
    fn viewer_do_search(&mut self) {
        let q = self.viewer_search_query.to_lowercase();
        if q.is_empty() {
            self.viewer_search_matches.clear();
            return;
        }
        if let Some(path) = self.selected_path().cloned() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.viewer_search_matches = content
                    .lines()
                    .enumerate()
                    .filter(|(_, l)| l.to_lowercase().contains(&q))
                    .map(|(i, _)| i)
                    .collect();
            }
        }
        self.viewer_search_idx = 0;
    }

    /// 현재 매칭 줄로 스크롤
    fn viewer_jump_to_match(&mut self) {
        if let Some(&line) = self.viewer_search_matches.get(self.viewer_search_idx) {
            self.preview_scroll = line as u16;
        }
    }

    /// 다음 매칭으로 이동
    fn viewer_next_match(&mut self) {
        if self.viewer_search_matches.is_empty() { return; }
        self.viewer_search_idx = (self.viewer_search_idx + 1) % self.viewer_search_matches.len();
        self.viewer_jump_to_match();
    }

    /// 이전 매칭으로 이동
    fn viewer_prev_match(&mut self) {
        if self.viewer_search_matches.is_empty() { return; }
        let len = self.viewer_search_matches.len();
        self.viewer_search_idx = (self.viewer_search_idx + len - 1) % len;
        self.viewer_jump_to_match();
    }

    /// 현재 디렉토리를 즐겨찾기에 추가/제거 (토글)
    fn toggle_bookmark(&mut self) {
        let dir = self.current_dir.clone();
        if let Some(pos) = self.config.bookmarks.iter().position(|b| b == &dir) {
            self.config.bookmarks.remove(pos);
            if self.bookmark_index > 0 && self.bookmark_index >= self.config.bookmarks.len() {
                self.bookmark_index = self.config.bookmarks.len().saturating_sub(1);
            }
        } else {
            self.config.bookmarks.push(dir);
        }
        self.config.save().ok();
    }

    /// 선택된 즐겨찾기 항목 삭제
    fn bookmark_delete(&mut self) {
        if self.config.bookmarks.is_empty() {
            return;
        }
        self.config.bookmarks.remove(self.bookmark_index);
        if self.bookmark_index > 0 && self.bookmark_index >= self.config.bookmarks.len() {
            self.bookmark_index = self.config.bookmarks.len().saturating_sub(1);
        }
        if self.config.bookmarks.is_empty() {
            self.focused_panel = FocusedPanel::FileList;
        }
        self.config.save().ok();
    }

    /// 선택된 즐겨찾기로 이동
    fn navigate_to_bookmark(&mut self) -> Result<()> {
        if let Some(path) = self.config.bookmarks.get(self.bookmark_index).cloned() {
            if path.is_dir() {
                self.navigate_to(path)?;
                self.focused_panel = FocusedPanel::FileList;
            }
        }
        Ok(())
    }

    /// Enter 키: 파일/디렉토리 모두 OpenWith 메뉴 표시
    fn open_with_or_navigate(&mut self) -> Result<()> {
        let actual = match self.filtered_indices.get(self.selected_index) {
            Some(&i) => i,
            None => return Ok(()),
        };
        if self.file_entries.get(actual).is_some() {
            self.open_with_index = 0;
            self.mode = AppMode::OpenWith;
        }
        Ok(())
    }

    /// OpenWith 메뉴에서 선택된 항목으로 파일 열기
    fn execute_open_with(&mut self) -> Result<()> {
        let path = match self.selected_path() {
            Some(p) => p.clone(),
            None => {
                self.mode = AppMode::FileList;
                return Ok(());
            }
        };
        self.mode = AppMode::FileList;

        if self.open_with_index == 0 {
            // 기본 앱으로 열기
            self.open_with_default_app(&path);
        } else if let Some(opener) = self.config.openers.get(self.open_with_index - 1).cloned() {
            if opener.terminal {
                self.pending_terminal_opener = Some((opener.command, opener.args));
            } else {
                let mut cmd = std::process::Command::new(&opener.command);
                for arg in &opener.args {
                    cmd.arg(arg);
                }
                cmd.arg(&path);
                cmd.spawn().ok();
            }
        }
        Ok(())
    }

    /// OS 기본 앱으로 파일 열기
    fn open_with_default_app(&self, path: &std::path::Path) {
        #[cfg(target_os = "macos")]
        { std::process::Command::new("open").arg(path).spawn().ok(); }
        #[cfg(target_os = "linux")]
        { std::process::Command::new("xdg-open").arg(path).spawn().ok(); }
        #[cfg(target_os = "windows")]
        { std::process::Command::new("cmd").args(["/c", "start", ""]).arg(path).spawn().ok(); }
    }

    /// TUI를 일시 해제하고 터미널 프로그램 실행 후 복원
    fn run_terminal_program<B: Backend>(
        &self,
        terminal: &mut Terminal<B>,
        command: &str,
        args: &[String],
    ) -> Result<()> {
        disable_raw_mode()?;
        execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        terminal.show_cursor()?;

        let path = self.selected_path().map(|p| p.as_os_str().to_owned());
        let mut cmd = std::process::Command::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        if let Some(p) = &path {
            cmd.arg(p);
        }
        cmd.status().ok();

        enable_raw_mode()?;
        execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        terminal.hide_cursor()?;
        terminal.clear()?;
        Ok(())
    }

    /// 상위 디렉토리로 이동
    fn go_parent(&mut self) {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            let prev_dir = self.current_dir.clone();
            if self.navigate_to(parent).is_ok() {
                if let Some(pos) = self.filtered_indices.iter().position(|&i| {
                    self.file_entries.get(i).map(|e| e.path == prev_dir).unwrap_or(false)
                }) {
                    self.selected_index = pos;
                }
            }
        }
    }

    /// 파일/디렉토리 진입 또는 열기
    fn enter_or_open(&mut self) -> Result<()> {
        let actual = match self.filtered_indices.get(self.selected_index) {
            Some(&i) => i,
            None => return Ok(()),
        };
        if let Some(entry) = self.file_entries.get(actual) {
            if entry.is_dir {
                let path = entry.path.clone();
                self.navigate_to(path)?;
            } else {
                self.clear_viewer_search();
                self.mode = AppMode::Viewer;
            }
        }
        Ok(())
    }

    /// 지정 경로로 이동
    pub fn navigate_to(&mut self, path: PathBuf) -> Result<()> {
        self.file_entries = crate::fs::ops::list_dir(&path)?;
        self.current_dir = path;
        self.selected_index = 0;
        self.preview_scroll = 0;
        self.preview_h_scroll = 0;
        self.search_query.clear();
        self.is_searching = false;
        self.filtered_indices = (0..self.file_entries.len()).collect();
        { let root = self.current_dir.clone(); self.git.status = crate::git::get_status(&root); }
        Ok(())
    }

    /// 터미널을 일시 해제하고 외부 편집기로 파일 열기 후 복원
    fn open_external_editor_with_terminal<B: Backend>(&self, terminal: &mut Terminal<B>) -> Result<()> {
        let actual = match self.filtered_indices.get(self.selected_index) {
            Some(&i) => i,
            None => return Ok(()),
        };
        let Some(entry) = self.file_entries.get(actual) else {
            return Ok(());
        };

        // 터미널 해제
        disable_raw_mode()?;
        execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        terminal.show_cursor()?;

        // 편집기 실행 (종료까지 대기)
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
        std::process::Command::new(&editor)
            .arg(&entry.path)
            .status()?;

        // 터미널 복원
        enable_raw_mode()?;
        execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        terminal.hide_cursor()?;
        terminal.clear()?;

        Ok(())
    }

    /// 경로를 클립보드에 추가 (중복 제거)
    fn toggle_path_clipboard(&mut self) {
        let dir = self.current_dir.clone();
        if let Some(pos) = self.path_clipboard.iter().position(|p| p == &dir) {
            self.path_clipboard.remove(pos);
            if self.path_clipboard_idx > 0 && self.path_clipboard_idx >= self.path_clipboard.len() {
                self.path_clipboard_idx = self.path_clipboard.len().saturating_sub(1);
            }
        } else {
            self.path_clipboard.push(dir);
        }
    }

    /// 경로 클립보드 선택 오버레이 키 처리
    fn handle_key_path_clipboard(&mut self, key: KeyEvent) -> Result<()> {
        let len = self.path_clipboard.len();
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::FileManager;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.path_clipboard_idx > 0 {
                    self.path_clipboard_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 && self.path_clipboard_idx + 1 < len {
                    self.path_clipboard_idx += 1;
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(path) = self.path_clipboard.get(self.path_clipboard_idx).cloned() {
                    self.fm_input = if path.is_dir() {
                        format!("{}/", path.display())
                    } else {
                        path.display().to_string()
                    };
                    self.fm_cursor = self.fm_input.len();
                }
                self.mode = AppMode::FileManager;
            }
            KeyCode::Char('d') => {
                if !self.path_clipboard.is_empty() {
                    self.path_clipboard.remove(self.path_clipboard_idx);
                    if self.path_clipboard_idx > 0
                        && self.path_clipboard_idx >= self.path_clipboard.len()
                    {
                        self.path_clipboard_idx = self.path_clipboard.len().saturating_sub(1);
                    }
                    if self.path_clipboard.is_empty() {
                        self.mode = AppMode::FileManager;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// 현재 선택된 파일 경로
    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.filtered_indices.get(self.selected_index)
            .and_then(|&i| self.file_entries.get(i))
            .map(|e| &e.path)
    }

    /// 파일 타입 감지
    pub fn detect_file_type(path: &PathBuf) -> FileType {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "md" | "markdown" => FileType::Markdown,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" => FileType::Image,
            "pdf" => FileType::Pdf,
            "csv" | "tsv" => FileType::Csv,
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => FileType::Archive,
            "rs" => FileType::Code("rust".to_string()),
            "py" => FileType::Code("python".to_string()),
            "js" | "mjs" => FileType::Code("javascript".to_string()),
            "ts" | "tsx" => FileType::Code("typescript".to_string()),
            "go" => FileType::Code("go".to_string()),
            "c" | "h" => FileType::Code("c".to_string()),
            "cpp" | "cc" | "cxx" => FileType::Code("cpp".to_string()),
            "java" => FileType::Code("java".to_string()),
            "sh" | "bash" | "zsh" => FileType::Code("bash".to_string()),
            "toml" => FileType::Code("toml".to_string()),
            "yaml" | "yml" => FileType::Code("yaml".to_string()),
            "json" => FileType::Code("json".to_string()),
            "html" | "htm" => FileType::Code("html".to_string()),
            "css" => FileType::Code("css".to_string()),
            "txt" | "log" | "conf" | "ini" => FileType::Text,
            _ => FileType::Unknown,
        }
    }
}
