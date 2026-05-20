use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::app::{App, AppMode, FocusedPanel};
use ratatui::widgets::{List, ListItem, ListState};
use crate::ui::{file_list, file_manager as fm_ui, git as git_ui, hint_bar, viewer};

/// 앱 전체 레이아웃 렌더러
pub struct AppLayout;

impl AppLayout {
    /// 메인 렌더링 진입점
    pub fn render(f: &mut Frame, app: &mut App) {
        match app.mode {
            AppMode::Viewer => {
                viewer::render_fullscreen(f, f.area(), app);
            }
            AppMode::Git => {
                git_ui::render(f, f.area(), app);
            }
            AppMode::Help => {
                Self::render_main_panels(f, app);
                Self::render_help_overlay(f, app);
            }
            AppMode::CommandPalette => {
                Self::render_main_panels(f, app);
                Self::render_palette_overlay(f, app);
            }
            AppMode::OpenWith => {
                Self::render_main_panels(f, app);
                Self::render_open_with_overlay(f, app);
            }
            AppMode::FileManager => {
                Self::render_main_panels(f, app);
                fm_ui::render(f, app);
            }
            AppMode::PathClipboard => {
                Self::render_main_panels(f, app);
                fm_ui::render(f, app);
                Self::render_path_clipboard_overlay(f, app);
            }
            _ => {
                Self::render_main_panels(f, app);
            }
        }
    }

    /// 3-패널 기본 레이아웃 렌더링
    fn render_main_panels(f: &mut Frame, app: &mut App) {
        let area = f.area();
        let has_status = app.status.is_some();

        // 수직 분할: 상단 탭바 + 중앙 패널 + 상태바(선택) + 하단 힌트바
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),                              // 탭 + 경로 바
                Constraint::Min(0),                                 // 메인 패널 영역
                Constraint::Length(if has_status { 1 } else { 0 }), // 상태 메시지
                Constraint::Length(1),                              // 힌트 바
            ])
            .split(area);

        // 탭 + 경로 바 렌더링
        Self::render_tab_bar(f, vertical[0], app);

        // 중앙 패널 수평 분할
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(15), // 즐겨찾기 패널
                Constraint::Percentage(35), // 파일 목록
                Constraint::Percentage(50), // 미리보기
            ])
            .split(vertical[1]);

        // 각 패널 렌더링
        Self::render_bookmarks_panel(f, horizontal[0], app);
        file_list::render(f, horizontal[1], app);
        Self::render_preview_panel(f, horizontal[2], app);

        // 상태 메시지
        if let Some(ref msg) = app.status {
            crate::ui::status_bar::render(f, vertical[2], msg);
        }

        // 힌트 바
        hint_bar::render(f, vertical[3], app);
    }

    /// 탭 + 현재 경로 바
    fn render_tab_bar(f: &mut Frame, area: Rect, app: &App) {
        use ratatui::{
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::Paragraph,
        };

        let path_str = app.current_dir.display().to_string();
        let line = Line::from(vec![
            Span::styled(" VEX ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("| "),
            Span::styled(path_str, Style::default().fg(Color::Yellow)),
            Span::raw("  [?] 도움말"),
        ]);
        f.render_widget(Paragraph::new(line), area);
    }

    /// 즐겨찾기 패널
    fn render_bookmarks_panel(f: &mut Frame, area: Rect, app: &mut App) {
        use ratatui::{
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, List, ListItem, ListState},
        };

        app.bookmarks_area = Some(area);

        let is_focused = app.focused_panel == FocusedPanel::Bookmarks;
        let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };
        let is_current_bookmarked = app.config.bookmarks.contains(&app.current_dir);

        let title = if is_focused {
            " 즐겨찾기 [Tab:나가기] "
        } else if is_current_bookmarked {
            " ★ 즐겨찾기 "
        } else {
            " 즐겨찾기 "
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        if app.config.bookmarks.is_empty() {
            use ratatui::widgets::Paragraph;
            let para = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  (비어있음)",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  b: 현재폴더추가",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(block);
            f.render_widget(para, area);
            return;
        }

        let items: Vec<ListItem> = app
            .config
            .bookmarks
            .iter()
            .map(|p| {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                let is_current = p == &app.current_dir;
                let style = if is_current {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let prefix = if is_current { "★ " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{prefix}{name}"), style),
                ]))
            })
            .collect();

        let mut state = ListState::default();
        if is_focused {
            state.select(Some(app.bookmark_index.min(app.config.bookmarks.len() - 1)));
        }

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶");

        f.render_stateful_widget(list, area, &mut state);
    }

    /// 미리보기 패널
    fn render_preview_panel(f: &mut Frame, area: Rect, app: &App) {
        use ratatui::{
            style::{Color, Style},
            widgets::{Block, Borders},
        };

        let selected_path = app.selected_path().cloned();
        let block = Block::default()
            .title(" 미리보기 ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        if let Some(path) = selected_path {
            if path.is_file() {
                let file_type = App::detect_file_type(&path);
                viewer::render_preview_content(f, area, &path, &file_type, app.preview_scroll, 0, app.preview_wrap, block, "", &[], None);
                return;
            }
        }

        // 파일 미선택 또는 디렉토리인 경우
        use ratatui::{text::Line, widgets::Paragraph};
        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from("  파일을 선택하면"),
            Line::from("  미리보기가 표시됩니다."),
        ])
        .block(block);
        f.render_widget(para, area);
    }

    /// 경로 클립보드 선택 오버레이
    fn render_path_clipboard_overlay(f: &mut Frame, app: &App) {
        use ratatui::{
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
        };

        let count = app.path_clipboard.len();
        let height = (count + 5).min(20) as u16;
        let area = centered_rect_abs(66, height, f.area());
        f.render_widget(Clear, area);

        let block = Block::default()
            .title(" 경로 클립보드 ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));

        let available_width = area.width.saturating_sub(4) as usize;

        let items: Vec<ListItem> = app
            .path_clipboard
            .iter()
            .map(|p| {
                let full = p.display().to_string();
                // 너무 길면 왼쪽을 잘라 마지막 부분만 표시
                let display = if full.len() > available_width {
                    format!("…{}", &full[full.len() - available_width + 1..])
                } else {
                    full
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {display}"), Style::default().fg(Color::White)),
                ]))
            })
            .collect();

        let mut state = ListState::default();
        if count > 0 {
            state.select(Some(app.path_clipboard_idx.min(count - 1)));
        }

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Magenta)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, inner[0], &mut state);

        let hint = Line::from(vec![
            Span::styled(" Enter ", Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::styled(" 선택  ", Style::default().fg(Color::DarkGray)),
            Span::styled(" d ", Style::default().fg(Color::DarkGray)),
            Span::styled(" 삭제  ", Style::default().fg(Color::DarkGray)),
            Span::styled(" Esc ", Style::default().fg(Color::DarkGray)),
            Span::raw(" 닫기"),
        ]);
        f.render_widget(Paragraph::new(hint), inner[1]);
    }

    /// 도움말 오버레이
    fn render_help_overlay(f: &mut Frame, _app: &App) {
        use ratatui::{
            layout::Alignment,
            style::{Color, Modifier, Style},
            text::Line,
            widgets::{Block, Borders, Clear, Paragraph},
        };

        let area = centered_rect(70, 80, f.area());
        f.render_widget(Clear, area);

        let help_text = vec![
            Line::from(""),
            Line::from(vec![
                ratatui::text::Span::styled(
                    "  VEX 단축키 목록",
                    Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan),
                ),
            ]),
            Line::from(""),
            Line::from("  ── 탐색 ─────────────────────"),
            Line::from("  ↑/k       위로 이동"),
            Line::from("  ↓/j       아래로 이동"),
            Line::from("  ←/h       상위 폴더"),
            Line::from("  →/l/Enter 진입 또는 열기"),
            Line::from(""),
            Line::from("  ── 보기 ─────────────────────"),
            Line::from("  Space     전체화면 뷰어"),
            Line::from("  PageUp    미리보기 위로"),
            Line::from("  PageDown  미리보기 아래로"),
            Line::from(""),
            Line::from("  ── 경로 클립보드 ────────────────"),
            Line::from("  y         현재 항목 경로 저장"),
            Line::from("  (파일관리 복사/이동) Tab  경로목록 열기"),
            Line::from(""),
            Line::from("  ── 즐겨찾기 ──────────────────"),
            Line::from("  b         현재폴더 추가/제거"),
            Line::from("  Tab       즐겨찾기 패널 포커스"),
            Line::from("  (패널) ↑↓  항목 이동"),
            Line::from("  (패널) Enter  해당 폴더로 이동"),
            Line::from("  (패널) d   항목 삭제"),
            Line::from(""),
            Line::from("  ── 기타 ─────────────────────"),
            Line::from("  Ctrl+P    명령어 팔레트"),
            Line::from("  Ctrl+,    설정 화면"),
            Line::from("  ?         이 도움말"),
            Line::from("  q         종료"),
            Line::from(""),
            Line::from("  (Esc 또는 ? 로 닫기)"),
        ];

        let block = Block::default()
            .title(" 도움말 ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let para = Paragraph::new(help_text)
            .block(block)
            .alignment(Alignment::Left);
        f.render_widget(para, area);
    }

    /// 명령어 팔레트 오버레이
    fn render_palette_overlay(f: &mut Frame, _app: &App) {
        use ratatui::{
            style::{Color, Style},
            text::Line,
            widgets::{Block, Borders, Clear, Paragraph},
        };

        let area = centered_rect(60, 40, f.area());
        f.render_widget(Clear, area);

        let block = Block::default()
            .title(" 명령어 팔레트 (Ctrl+P) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from("  > 명령어를 입력하세요..."),
            Line::from(""),
            Line::from("  파일 열기 / 이동 / 복사 / 삭제"),
        ])
        .block(block);
        f.render_widget(para, area);
    }

    /// 열기 프로그램 선택 오버레이
    fn render_open_with_overlay(f: &mut Frame, app: &App) {
        use ratatui::{
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Clear},
        };

        let file_name = app
            .selected_path()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("파일");

        // 항목 수에 맞게 높이 동적 결정 (테두리 2 + 빈줄 2 + 항목 + 힌트 2)
        let num_items = 1 + app.config.openers.len();
        let needed_height = (num_items + 6) as u16;
        let area_height = needed_height.min(f.area().height.saturating_sub(4));
        let area = centered_rect_abs(50, area_height, f.area());
        f.render_widget(Clear, area);

        let title = format!(" 열기: {file_name} ");
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));

        // 항목 목록 구성
        let mut items: Vec<ListItem> = vec![ListItem::new(Line::from(vec![
            Span::styled(" 기본 앱으로 열기", Style::default().fg(Color::White)),
        ]))];
        for opener in &app.config.openers {
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ({})", opener.name, opener.command),
                    Style::default().fg(Color::White),
                ),
            ])));
        }

        let mut state = ListState::default();
        state.select(Some(app.open_with_index));

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Green)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        // 목록 영역 (하단 힌트 1줄 제외)
        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        f.render_stateful_widget(list, inner[0], &mut state);

        // 하단 힌트
        let hint = Line::from(vec![
            Span::styled(" [Enter]", Style::default().fg(Color::Black).bg(Color::Green)),
            Span::styled(" 열기  ", Style::default().fg(Color::DarkGray)),
            Span::styled(" [Esc]", Style::default().fg(Color::Black).bg(Color::DarkGray)),
            Span::styled(" 취소", Style::default().fg(Color::DarkGray)),
        ]);
        use ratatui::widgets::Paragraph;
        f.render_widget(Paragraph::new(hint), inner[1]);
    }
}

/// 절대 크기로 중앙 정렬 사각형 계산
pub fn centered_rect_abs(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}

/// 중앙 정렬 사각형 계산
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
