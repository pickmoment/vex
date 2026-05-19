use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::Backend, layout::Rect, Terminal};
use std::path::PathBuf;
use std::time::Duration;

use crate::config::Config;
use crate::fs::ops::FileEntry;
use crate::ui::layout::AppLayout;

/// 포커스된 패널
#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPanel {
    FileList,
    Bookmarks,
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
    /// 탭 목록 (디렉토리 경로)
    pub tabs: Vec<PathBuf>,
    /// 현재 탭 인덱스
    pub current_tab: usize,
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
    /// git 상태 캐시
    pub git_status: Option<crate::git::GitStatus>,
    /// git 모드: 포커스된 섹션
    pub git_section: GitSection,
    /// git 모드: 스테이징 영역 선택 인덱스
    pub git_staged_idx: usize,
    /// git 모드: 워킹 트리 선택 인덱스
    pub git_unstaged_idx: usize,
    /// git 모드: 현재 표시 중인 diff 내용
    pub git_diff: Vec<String>,
    /// git 모드: diff 스크롤 오프셋
    pub git_diff_scroll: u16,
    /// git 모드: 커밋 메시지 입력 중 여부
    pub git_is_committing: bool,
    /// git 모드: 커밋 메시지 입력 버퍼
    pub git_commit_input: String,
    /// git 모드: 로그 패널 표시 여부
    pub git_show_log: bool,
    /// git 모드: 커밋 로그 목록
    pub git_log: Vec<String>,
    /// git 모드: 로그 패널 포커스 여부 (커밋 목록)
    pub git_log_focused: bool,
    /// git 모드: 로그 패널에서 선택된 커밋 인덱스
    pub git_log_idx: usize,
    /// git 모드: 커밋의 변경 파일 목록 포커스 여부
    pub git_log_file_focused: bool,
    /// git 모드: 선택된 커밋의 변경 파일 목록 (status_char, path)
    pub git_commit_files: Vec<(char, String)>,
    /// git 모드: 변경 파일 목록에서 선택된 인덱스
    pub git_commit_file_idx: usize,
    /// git 모드: 선택된 파일의 diff 내용
    pub git_commit_show: Vec<String>,
    /// git 모드: diff 스크롤 오프셋
    pub git_commit_show_scroll: u16,
    /// git 모드: diff 수평 스크롤 오프셋 (공통)
    pub git_diff_h_scroll: u16,
    /// git 모드: diff 줄바꿈 여부
    pub git_diff_wrap: bool,
    /// git 모드: diff 전체화면 여부
    pub git_diff_fullscreen: bool,
    /// 파일 관리: 메뉴 선택 인덱스
    pub fm_menu_idx: usize,
    /// 파일 관리: 텍스트 입력 버퍼 (이름/경로)
    pub fm_input: String,
    /// 파일 관리: 현재 진행 중인 작업
    pub fm_operation: Option<FmOp>,
    /// 파일 관리: 마지막 오류 메시지
    pub fm_error: Option<String>,
    /// 파일 목록 패널의 실제 내부 높이 (렌더링 시 기록)
    pub file_list_height: u16,
    /// 뷰어 콘텐츠 영역의 실제 내부 높이 (렌더링 시 기록)
    pub viewer_height: u16,
    /// Git diff 패널의 실제 내부 높이 (렌더링 시 기록)
    pub git_diff_panel_height: u16,
}

impl App {
    /// 새 앱 인스턴스 생성
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let current_dir = std::env::current_dir()?;
        let file_entries = crate::fs::ops::list_dir(&current_dir)?;
        let preview_wrap = config.preview.wrap;

        let filtered_indices = (0..file_entries.len()).collect();
        let git_status = crate::git::get_status(&current_dir);
        Ok(Self {
            current_dir,
            file_entries,
            selected_index: 0,
            mode: AppMode::FileList,
            focused_panel: FocusedPanel::FileList,
            bookmark_index: 0,
            bookmarks_area: None,
            tabs: Vec::new(),
            current_tab: 0,
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
            git_status,
            git_section: GitSection::Unstaged,
            git_staged_idx: 0,
            git_unstaged_idx: 0,
            git_diff: Vec::new(),
            git_diff_scroll: 0,
            git_is_committing: false,
            git_commit_input: String::new(),
            git_show_log: false,
            git_log: Vec::new(),
            git_log_focused: false,
            git_log_idx: 0,
            git_log_file_focused: false,
            git_commit_files: Vec::new(),
            git_commit_file_idx: 0,
            git_commit_show: Vec::new(),
            git_commit_show_scroll: 0,
            git_diff_h_scroll: 0,
            git_diff_wrap: false,
            git_diff_fullscreen: false,
            fm_menu_idx: 0,
            fm_input: String::new(),
            fm_operation: None,
            fm_error: None,
            file_list_height: 0,
            viewer_height: 0,
            git_diff_panel_height: 0,
        })
    }

    /// 메인 이벤트 루프
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
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
        match self.mode {
            AppMode::FileList => self.handle_key_file_list(key),
            AppMode::Viewer => self.handle_key_viewer(key),
            AppMode::OpenWith => self.handle_key_open_with(key),
            AppMode::Settings => self.handle_key_settings(key),
            AppMode::CommandPalette => self.handle_key_palette(key),
            AppMode::Help => self.handle_key_help(key),
            AppMode::Git => self.handle_key_git(key),
            AppMode::FileManager => self.handle_key_file_manager(key),
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
                }
            }
            KeyCode::Char('r') => {
                self.refresh_file_list();
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
            KeyCode::Char('g') => {
                self.refresh_git_status();
                self.git_staged_idx = 0;
                self.git_unstaged_idx = 0;
                self.git_is_committing = false;
                self.git_commit_input.clear();
                self.git_show_log = false;
                self.git_log.clear();
                self.git_log_focused = false;
                self.git_log_idx = 0;
                self.git_log_file_focused = false;
                self.git_commit_files.clear();
                self.git_commit_file_idx = 0;
                self.git_commit_show.clear();
                self.git_commit_show_scroll = 0;
                self.git_diff_h_scroll = 0;
                self.git_diff_wrap = false;
                self.git_diff_fullscreen = false;
                self.git_section = GitSection::Unstaged;
                self.mode = AppMode::Git;
                self.load_git_diff();
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
            KeyCode::Char('q') | KeyCode::Esc => self.mode = AppMode::FileList,
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
            KeyCode::Esc | KeyCode::Tab | KeyCode::Left | KeyCode::Char('h') => {
                self.focused_panel = FocusedPanel::FileList;
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
        // 커밋 메시지 입력 중
        if self.git_is_committing {
            match key.code {
                KeyCode::Esc => {
                    self.git_is_committing = false;
                    self.git_commit_input.clear();
                }
                KeyCode::Enter => {
                    if !self.git_commit_input.is_empty() {
                        if let Some(status) = &self.git_status {
                            let root = status.root.clone();
                            let msg = self.git_commit_input.clone();
                            crate::git::commit_changes(&root, &msg);
                        }
                        self.git_is_committing = false;
                        self.git_commit_input.clear();
                        self.refresh_git_status();
                        self.load_git_diff();
                    }
                }
                KeyCode::Backspace => { self.git_commit_input.pop(); }
                KeyCode::Char(c) => { self.git_commit_input.push(c); }
                _ => {}
            }
            return Ok(());
        }

        // diff 전체화면 모드
        if self.git_diff_fullscreen {
            let has_commit_diff = !self.git_commit_show.is_empty();
            match key.code {
                KeyCode::Char('q') => {
                    self.mode = AppMode::FileList;
                }
                KeyCode::Esc | KeyCode::Char('f') => {
                    self.git_diff_fullscreen = false;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if has_commit_diff {
                        self.git_commit_show_scroll =
                            self.git_commit_show_scroll.saturating_sub(1);
                    } else {
                        self.git_diff_scroll = self.git_diff_scroll.saturating_sub(1);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if has_commit_diff {
                        self.git_commit_show_scroll += 1;
                    } else {
                        self.git_diff_scroll += 1;
                    }
                }
                KeyCode::PageUp => {
                    let n = self.git_diff_panel_height.max(1);
                    if has_commit_diff {
                        self.git_commit_show_scroll =
                            self.git_commit_show_scroll.saturating_sub(n);
                    } else {
                        self.git_diff_scroll = self.git_diff_scroll.saturating_sub(n);
                    }
                }
                KeyCode::PageDown | KeyCode::Char(' ') => {
                    let n = self.git_diff_panel_height.max(1);
                    if has_commit_diff {
                        self.git_commit_show_scroll += n;
                    } else {
                        self.git_diff_scroll += n;
                    }
                }
                KeyCode::Left | KeyCode::Char('[') => {
                    if !self.git_diff_wrap {
                        self.git_diff_h_scroll = self.git_diff_h_scroll.saturating_sub(4);
                    }
                }
                KeyCode::Right | KeyCode::Char(']') => {
                    if !self.git_diff_wrap {
                        self.git_diff_h_scroll += 4;
                    }
                }
                KeyCode::Char('w') => {
                    self.git_diff_wrap = !self.git_diff_wrap;
                    if self.git_diff_wrap {
                        self.git_diff_h_scroll = 0;
                    }
                }
                _ => {}
            }
            return Ok(());
        }

        // 범용 diff 조작키 (모드 무관)
        match key.code {
            KeyCode::Char('[') => {
                if !self.git_diff_wrap {
                    self.git_diff_h_scroll = self.git_diff_h_scroll.saturating_sub(4);
                }
                return Ok(());
            }
            KeyCode::Char(']') => {
                if !self.git_diff_wrap {
                    self.git_diff_h_scroll += 4;
                }
                return Ok(());
            }
            KeyCode::Char('w') => {
                self.git_diff_wrap = !self.git_diff_wrap;
                if self.git_diff_wrap {
                    self.git_diff_h_scroll = 0;
                }
                return Ok(());
            }
            KeyCode::Char('f') => {
                let has_diff = !self.git_diff.is_empty() || !self.git_commit_show.is_empty();
                if has_diff {
                    self.git_diff_fullscreen = true;
                }
                return Ok(());
            }
            _ => {}
        }

        // 커밋 변경 파일 포커스 상태
        if self.git_log_focused && self.git_log_file_focused {
            match key.code {
                KeyCode::Char('q') => {
                    self.mode = AppMode::FileList;
                }
                KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                    self.git_log_file_focused = false;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.git_commit_file_idx > 0 {
                        self.git_commit_file_idx -= 1;
                        self.load_commit_file_diff();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.git_commit_file_idx + 1 < self.git_commit_files.len() {
                        self.git_commit_file_idx += 1;
                        self.load_commit_file_diff();
                    }
                }
                KeyCode::Enter | KeyCode::Char('d') => {
                    self.load_commit_file_diff();
                }
                KeyCode::PageUp => {
                    let n = self.git_diff_panel_height.max(1);
                    self.git_commit_show_scroll =
                        self.git_commit_show_scroll.saturating_sub(n);
                }
                KeyCode::PageDown => {
                    let n = self.git_diff_panel_height.max(1);
                    self.git_commit_show_scroll =
                        self.git_commit_show_scroll.saturating_add(n);
                }
                KeyCode::Char('L') => {
                    self.git_show_log = false;
                    self.git_log_focused = false;
                    self.git_log_file_focused = false;
                }
                _ => {}
            }
            return Ok(());
        }

        // 커밋 목록 포커스 상태
        if self.git_log_focused {
            match key.code {
                KeyCode::Char('q') => {
                    self.mode = AppMode::FileList;
                }
                KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                    self.git_log_focused = false;
                    self.git_log_file_focused = false;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.git_log_idx > 0 {
                        self.git_log_idx -= 1;
                        self.git_log_file_focused = false;
                        self.load_commit_show();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.git_log_idx + 1 < self.git_log.len() {
                        self.git_log_idx += 1;
                        self.git_log_file_focused = false;
                        self.load_commit_show();
                    }
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter | KeyCode::Char('d') => {
                    if !self.git_commit_files.is_empty() {
                        self.git_log_file_focused = true;
                        self.git_commit_file_idx = 0;
                        self.load_commit_file_diff();
                    } else {
                        self.load_commit_show();
                    }
                }
                KeyCode::PageUp => {
                    let n = self.git_diff_panel_height.max(1);
                    self.git_commit_show_scroll =
                        self.git_commit_show_scroll.saturating_sub(n);
                }
                KeyCode::PageDown => {
                    let n = self.git_diff_panel_height.max(1);
                    self.git_commit_show_scroll =
                        self.git_commit_show_scroll.saturating_add(n);
                }
                KeyCode::Char('L') => {
                    self.git_show_log = false;
                    self.git_log_focused = false;
                    self.git_log_file_focused = false;
                }
                _ => {}
            }
            return Ok(());
        }

        // 파일 패널 포커스 상태 (기본)
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = AppMode::FileList;
            }
            KeyCode::Tab => {
                self.git_section = match self.git_section {
                    GitSection::Staged => GitSection::Unstaged,
                    GitSection::Unstaged => GitSection::Staged,
                };
                self.load_git_diff();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.git_show_log && !self.git_log.is_empty() {
                    self.git_log_focused = true;
                    self.git_log_file_focused = false;
                    self.load_commit_show();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                match self.git_section {
                    GitSection::Staged => {
                        if self.git_staged_idx > 0 { self.git_staged_idx -= 1; }
                    }
                    GitSection::Unstaged => {
                        if self.git_unstaged_idx > 0 { self.git_unstaged_idx -= 1; }
                    }
                }
                self.load_git_diff();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref status) = self.git_status {
                    match self.git_section {
                        GitSection::Staged => {
                            if self.git_staged_idx + 1 < status.staged.len() {
                                self.git_staged_idx += 1;
                            }
                        }
                        GitSection::Unstaged => {
                            if self.git_unstaged_idx + 1 < status.unstaged.len() {
                                self.git_unstaged_idx += 1;
                            }
                        }
                    }
                }
                self.load_git_diff();
            }
            KeyCode::Char('a') => {
                if self.git_section == GitSection::Unstaged {
                    if let Some(ref status) = self.git_status {
                        if let Some(file) = status.unstaged.get(self.git_unstaged_idx) {
                            let root = status.root.clone();
                            let path = file.path.clone();
                            crate::git::stage_file(&root, &path);
                        }
                    }
                    self.refresh_git_status();
                    self.load_git_diff();
                }
            }
            KeyCode::Char('u') => {
                if self.git_section == GitSection::Staged {
                    if let Some(ref status) = self.git_status {
                        if let Some(file) = status.staged.get(self.git_staged_idx) {
                            let root = status.root.clone();
                            let path = file.path.clone();
                            crate::git::unstage_file(&root, &path);
                        }
                    }
                    self.refresh_git_status();
                    self.load_git_diff();
                }
            }
            KeyCode::Enter | KeyCode::Char('d') => {
                self.git_show_log = false;
                self.git_log_focused = false;
                self.load_git_diff();
            }
            KeyCode::Char('c') => {
                let has_staged = self.git_status.as_ref()
                    .map(|s| !s.staged.is_empty())
                    .unwrap_or(false);
                if has_staged {
                    self.git_is_committing = true;
                    self.git_commit_input.clear();
                }
            }
            KeyCode::Char('L') => {
                self.git_show_log = !self.git_show_log;
                if self.git_show_log {
                    if let Some(ref status) = self.git_status {
                        let root = status.root.clone();
                        self.git_log = crate::git::get_log(&root);
                    }
                    self.git_log_idx = 0;
                    self.git_log_focused = false;
                    self.git_log_file_focused = false;
                    self.git_commit_files.clear();
                    self.git_commit_file_idx = 0;
                    self.git_commit_show.clear();
                    self.git_commit_show_scroll = 0;
                } else {
                    self.git_log_focused = false;
                }
            }
            KeyCode::Char('r') => {
                self.refresh_git_status();
                self.load_git_diff();
            }
            KeyCode::PageUp => {
                let n = self.git_diff_panel_height.max(1);
                self.git_diff_scroll = self.git_diff_scroll.saturating_sub(n);
            }
            KeyCode::PageDown => {
                let n = self.git_diff_panel_height.max(1);
                self.git_diff_scroll = self.git_diff_scroll.saturating_add(n);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_file_manager(&mut self, key: KeyEvent) -> Result<()> {
        match self.fm_operation.clone() {
            None => self.handle_fm_menu(key),
            Some(FmOp::Delete) => self.handle_fm_delete(key),
            Some(_) => self.handle_fm_input(key),
        }
    }

    fn handle_fm_menu(&mut self, key: KeyEvent) -> Result<()> {
        const MENU_LEN: usize = 4;
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
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                let op = match self.fm_menu_idx {
                    0 => FmOp::Copy,
                    1 => FmOp::Move,
                    2 => FmOp::Rename,
                    _ => FmOp::Delete,
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
            FmOp::Delete => String::new(),
        };
        self.fm_input = input;
        self.fm_operation = Some(op);
    }

    fn handle_fm_delete(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(path) = self.selected_path().cloned() {
                    match crate::fs::ops::delete_file(&path) {
                        Ok(_) => {
                            self.fm_operation = None;
                            self.mode = AppMode::FileList;
                            self.fm_refresh_file_list();
                        }
                        Err(e) => {
                            self.fm_error = Some(e.to_string());
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
            KeyCode::Backspace => { self.fm_input.pop(); }
            KeyCode::Char(c) => { self.fm_input.push(c); }
            KeyCode::Enter => self.execute_fm_operation()?,
            _ => {}
        }
        Ok(())
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
        let result = match op {
            Some(FmOp::Rename) => crate::fs::ops::rename_file(&src, &input).map(|_| ()),
            Some(FmOp::Copy) => {
                crate::fs::ops::copy_file(&src, &std::path::PathBuf::from(&input))
            }
            Some(FmOp::Move) => {
                crate::fs::ops::move_file(&src, &std::path::PathBuf::from(&input))
            }
            _ => return Ok(()),
        };

        match result {
            Ok(_) => {
                self.fm_operation = None;
                self.fm_error = None;
                self.mode = AppMode::FileList;
                self.fm_refresh_file_list();
            }
            Err(e) => {
                self.fm_error = Some(e.to_string());
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
        self.git_status = crate::git::get_status(&self.current_dir);
    }

    fn refresh_file_list(&mut self) {
        let prev_selected_path = self.selected_path().cloned();
        self.file_entries = crate::fs::ops::list_dir(&self.current_dir).unwrap_or_default();
        self.git_status = crate::git::get_status(&self.current_dir);
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

    fn refresh_git_status(&mut self) {
        self.git_status = crate::git::get_status(&self.current_dir);
        if let Some(ref status) = self.git_status {
            if self.git_staged_idx >= status.staged.len().max(1) {
                self.git_staged_idx = status.staged.len().saturating_sub(1);
            }
            if self.git_unstaged_idx >= status.unstaged.len().max(1) {
                self.git_unstaged_idx = status.unstaged.len().saturating_sub(1);
            }
        }
    }

    /// 선택된 커밋의 변경 파일 목록 로드
    fn load_commit_show(&mut self) {
        if let Some(ref status) = self.git_status {
            if let Some(entry) = self.git_log.get(self.git_log_idx) {
                let hash = entry.split_whitespace().next().unwrap_or("").to_string();
                if !hash.is_empty() {
                    let root = status.root.clone();
                    self.git_commit_files = crate::git::get_commit_files(&root, &hash);
                    self.git_commit_file_idx = 0;
                    self.git_commit_show.clear();
                    self.git_commit_show_scroll = 0;
                }
            }
        }
    }

    /// 선택된 커밋의 선택된 파일 diff 로드
    fn load_commit_file_diff(&mut self) {
        if let Some(ref status) = self.git_status {
            if let Some(log_entry) = self.git_log.get(self.git_log_idx) {
                let hash = log_entry.split_whitespace().next().unwrap_or("").to_string();
                if let Some((_, path)) = self.git_commit_files.get(self.git_commit_file_idx) {
                    let root = status.root.clone();
                    let path = path.clone();
                    self.git_commit_show =
                        crate::git::get_commit_file_diff(&root, &hash, &path);
                    self.git_commit_show_scroll = 0;
                    self.git_diff_h_scroll = 0;
                }
            }
        }
    }

    fn load_git_diff(&mut self) {
        self.git_diff_scroll = 0;
        self.git_diff_h_scroll = 0;
        if let Some(ref status) = self.git_status {
            let (file, is_staged) = match self.git_section {
                GitSection::Staged => (status.staged.get(self.git_staged_idx), true),
                GitSection::Unstaged => (status.unstaged.get(self.git_unstaged_idx), false),
            };
            if let Some(f) = file {
                let root = status.root.clone();
                let path = f.path.clone();
                self.git_diff = crate::git::get_diff(&root, &path, is_staged);
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
            MouseEventKind::Down(_) => {
                if let Some(area) = self.bookmarks_area {
                    if mouse.column >= area.x && mouse.column < area.x + area.width
                        && mouse.row >= area.y && mouse.row < area.y + area.height
                    {
                        // 테두리(1행) 제외
                        let row = mouse.row.saturating_sub(area.y + 1) as usize;
                        if row < self.config.bookmarks.len() {
                            self.bookmark_index = row;
                            self.navigate_to_bookmark()?;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
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
            self.navigate_to(parent).ok();
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
        self.git_status = crate::git::get_status(&self.current_dir);
        Ok(())
    }

    /// 이전 파일 (뷰어 모드)
    fn prev_file(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.preview_scroll = 0;
        }
    }

    /// 다음 파일 (뷰어 모드)
    fn next_file(&mut self) {
        if self.selected_index + 1 < self.filtered_indices.len() {
            self.selected_index += 1;
            self.preview_scroll = 0;
        }
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
