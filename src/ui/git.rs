use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, GitSection};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(f, chunks[0], app);
    render_content(f, chunks[1], app);
    render_footer(f, chunks[2], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let (branch, staged_count, unstaged_count) = if let Some(ref status) = app.git.status {
        (
            status.branch.as_str(),
            status.staged.len(),
            status.unstaged.len(),
        )
    } else {
        ("—", 0, 0)
    };

    let line = Line::from(vec![
        Span::styled(" Git", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(": "),
        Span::styled(branch, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(
            format!("스테이징 {staged_count}"),
            Style::default().fg(Color::Green),
        ),
        Span::raw("  "),
        Span::styled(
            format!("변경 {unstaged_count}"),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  "),
        Span::styled("[g] git 관리", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn render_content(f: &mut Frame, area: Rect, app: &mut App) {
    if app.git.status.is_none() {
        render_not_a_repo(f, area);
        return;
    }

    if app.git.diff_fullscreen {
        render_diff_fullscreen(f, area, app);
        return;
    }

    if app.git.show_log {
        render_log_mode(f, area, app);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
        render_file_panels(f, chunks[0], app);
        render_right_panel(f, chunks[1], app);
    }
}

/// 로그 모드 전용 3패널 레이아웃: 커밋 목록 | 변경 파일 | diff
fn render_log_mode(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(28),
            Constraint::Percentage(44),
        ])
        .split(area);

    render_log_list(f, chunks[0], app);
    render_commit_files_panel(f, chunks[1], app);
    render_commit_file_diff(f, chunks[2], app);
}

fn render_not_a_repo(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Git ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Git 저장소가 아닙니다.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(block);
    f.render_widget(para, area);
}

fn render_file_panels(f: &mut Frame, area: Rect, app: &App) {
    let status = match app.git.status.as_ref() {
        Some(s) => s,
        None => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // 스테이징 영역
    let staged_focused = app.git.section == GitSection::Staged;
    let staged_border = if staged_focused { Color::Green } else { Color::DarkGray };
    let staged_title = format!(" 스테이징 ({}) ", status.staged.len());

    let staged_items: Vec<ListItem> = status
        .staged
        .iter()
        .map(|f| {
            let status_char = f.x;
            let color = staged_status_color(status_char);
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {status_char} "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(f.path.clone(), Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let staged_block = Block::default()
        .title(staged_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(staged_border));

    if staged_items.is_empty() {
        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  (없음)",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(staged_block);
        f.render_widget(para, chunks[0]);
    } else {
        let mut state = ListState::default();
        if staged_focused {
            state.select(Some(
                app.git.staged_idx.min(status.staged.len().saturating_sub(1)),
            ));
        }
        let list = List::new(staged_items)
            .block(staged_block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list, chunks[0], &mut state);
    }

    // 워킹 트리 (unstaged)
    let unstaged_focused = app.git.section == GitSection::Unstaged;
    let unstaged_border = if unstaged_focused { Color::Yellow } else { Color::DarkGray };
    let unstaged_title = format!(" 워킹 트리 ({}) ", status.unstaged.len());

    let unstaged_items: Vec<ListItem> = status
        .unstaged
        .iter()
        .map(|f| {
            let (status_char, color) = if f.x == '?' && f.y == '?' {
                ('?', Color::DarkGray)
            } else {
                (f.y, unstaged_status_color(f.y))
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {status_char} "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(f.path.clone(), Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let unstaged_block = Block::default()
        .title(unstaged_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(unstaged_border));

    if unstaged_items.is_empty() {
        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  (없음)",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(unstaged_block);
        f.render_widget(para, chunks[1]);
    } else {
        let mut state = ListState::default();
        if unstaged_focused {
            state.select(Some(
                app.git.unstaged_idx.min(status.unstaged.len().saturating_sub(1)),
            ));
        }
        let list = List::new(unstaged_items)
            .block(unstaged_block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list, chunks[1], &mut state);
    }
}

fn render_right_panel(f: &mut Frame, area: Rect, app: &mut App) {
    if !app.git.diff.is_empty() {
        render_diff_panel(f, area, app);
    } else {
        render_diff_hint(f, area, app);
    }
}

fn render_diff_hint(f: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .title(" diff ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  (변경 파일 없음)",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(block);
    f.render_widget(para, area);
}

fn render_diff_panel(f: &mut Frame, area: Rect, app: &mut App) {
    app.git.diff_panel_height = area.height.saturating_sub(2);
    let selected_file = get_selected_file_name(app).unwrap_or_default();
    let wrap_indicator = if app.git.diff_wrap { " [줄바꿈ON]" } else { "" };
    let title = format!(" diff: {selected_file}{wrap_indicator} ");
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    render_diff_content(f, area, &app.git.diff, app.git.diff_scroll,
        app.git.diff_h_scroll, app.git.diff_wrap, block);
}

fn render_log_list(f: &mut Frame, area: Rect, app: &App) {
    let (border_color, title) = if app.git.log_focused && !app.git.log_file_focused {
        (Color::Magenta, " 커밋 로그  [↑↓:이동  →:파일목록  ←:닫기] ")
    } else if app.git.log_focused {
        (Color::DarkGray, " 커밋 로그 ")
    } else {
        (Color::DarkGray, " 커밋 로그  [→/l:선택] ")
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.git.log.is_empty() {
        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  커밋이 없습니다.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(block);
        f.render_widget(para, area);
        return;
    }

    let items: Vec<ListItem> = app
        .git.log
        .iter()
        .map(|entry| {
            let parts: Vec<&str> = entry.splitn(2, ' ').collect();
            let (hash, rest) = if parts.len() == 2 {
                (parts[0], parts[1])
            } else {
                (entry.as_str(), "")
            };
            ListItem::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(hash.to_string(), Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::styled(rest.to_string(), Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    if app.git.log_focused {
        state.select(Some(app.git.log_idx.min(app.git.log.len().saturating_sub(1))));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut state);
}

fn render_commit_files_panel(f: &mut Frame, area: Rect, app: &App) {
    let (border_color, title) = if app.git.log_focused && app.git.log_file_focused {
        (Color::Yellow, " 변경 파일  [↑↓:이동  Enter:diff  ←:커밋으로] ")
    } else if app.git.log_focused {
        (Color::DarkGray, " 변경 파일  [→/Enter:선택] ")
    } else {
        (Color::DarkGray, " 변경 파일 ")
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if !app.git.log_focused {
        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  →/l: 커밋 선택",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(block);
        f.render_widget(para, area);
        return;
    }

    if app.git.commit_files.is_empty() {
        let hint = if app.git.log_focused {
            "  (파일 없음)"
        } else {
            "  커밋을 선택하세요"
        };
        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray))),
        ])
        .block(block);
        f.render_widget(para, area);
        return;
    }

    let items: Vec<ListItem> = app
        .git.commit_files
        .iter()
        .map(|(status, path)| {
            let color = match status {
                'A' => Color::Green,
                'M' => Color::Yellow,
                'D' => Color::Red,
                'R' => Color::Cyan,
                _ => Color::White,
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {status} "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(path.clone(), Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    if app.git.log_file_focused {
        state.select(Some(
            app.git.commit_file_idx
                .min(app.git.commit_files.len().saturating_sub(1)),
        ));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut state);
}

fn render_commit_file_diff(f: &mut Frame, area: Rect, app: &mut App) {
    app.git.diff_panel_height = area.height.saturating_sub(2);
    let file_name = if app.git.log_file_focused {
        app.git.commit_files
            .get(app.git.commit_file_idx)
            .map(|(_, p)| p.as_str())
            .unwrap_or("")
    } else {
        ""
    };

    let border_color = if app.git.log_file_focused { Color::Cyan } else { Color::DarkGray };
    let title = if file_name.is_empty() {
        " diff ".to_string()
    } else {
        format!(" diff: {file_name} ")
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.git.commit_show.is_empty() {
        let hint = if app.git.log_file_focused {
            "  Enter 또는 d: diff 보기"
        } else if app.git.log_focused {
            "  →/Enter: 파일 목록으로 이동"
        } else {
            "  →/l: 커밋 선택"
        };
        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray))),
        ])
        .block(block);
        f.render_widget(para, area);
        return;
    }

    render_diff_content(f, area, &app.git.commit_show, app.git.commit_show_scroll,
        app.git.diff_h_scroll, app.git.diff_wrap, block);
}

/// diff 내용 렌더링 공통 함수 (수직/수평 스크롤, 줄바꿈 지원)
fn render_diff_content(
    f: &mut Frame,
    area: Rect,
    lines: &[String],
    v_scroll: u16,
    h_scroll: u16,
    wrap: bool,
    block: Block,
) {
    use ratatui::widgets::Wrap;

    let inner_height = area.height.saturating_sub(2) as usize;
    let h = if wrap { 0 } else { h_scroll as usize };

    let rendered: Vec<Line> = lines
        .iter()
        .skip(v_scroll as usize)
        .take(inner_height)
        .map(|l| diff_line_to_tui_h(l, h))
        .collect();

    let para = if wrap {
        Paragraph::new(rendered)
            .block(block)
            .wrap(Wrap { trim: false })
    } else {
        Paragraph::new(rendered).block(block)
    };

    f.render_widget(para, area);
}

/// diff 전체화면 렌더링
pub fn render_diff_fullscreen(f: &mut Frame, area: Rect, app: &mut App) {
    app.git.diff_panel_height = area.height.saturating_sub(2);
    use ratatui::widgets::Wrap;

    let (content, v_scroll, file_label) = if !app.git.commit_show.is_empty() {
        let name = app
            .git.commit_files
            .get(app.git.commit_file_idx)
            .map(|(_, p)| p.as_str())
            .unwrap_or("커밋 diff");
        (&app.git.commit_show, app.git.commit_show_scroll, name.to_string())
    } else {
        let name = get_selected_file_name(app).unwrap_or_default();
        (&app.git.diff, app.git.diff_scroll, name)
    };

    let wrap_label = if app.git.diff_wrap { "w:줄바꿈OFF" } else { "w:줄바꿈ON" };
    let title = format!(
        " diff: {file_label}  [↑↓:스크롤  {wrap_label}  [/]:좌우  f/Esc:닫기] "
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_height = area.height.saturating_sub(2) as usize;
    let h = if app.git.diff_wrap { 0 } else { app.git.diff_h_scroll as usize };

    let rendered: Vec<Line> = content
        .iter()
        .skip(v_scroll as usize)
        .take(inner_height)
        .map(|l| diff_line_to_tui_h(l, h))
        .collect();

    let para = if app.git.diff_wrap {
        Paragraph::new(rendered)
            .block(block)
            .wrap(Wrap { trim: false })
    } else {
        Paragraph::new(rendered).block(block)
    };

    f.render_widget(para, area);
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    if app.git.is_committing {
        let line = Line::from(vec![
            Span::styled(
                " 커밋 메시지 ",
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::raw(": "),
            Span::styled(app.git.commit_input.clone(), Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Green)),
            Span::raw("  "),
            Span::styled("[Enter]", Style::default().fg(Color::Black).bg(Color::Green)),
            Span::styled(" 확인  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Esc]", Style::default().fg(Color::Black).bg(Color::DarkGray)),
            Span::styled(" 취소", Style::default().fg(Color::DarkGray)),
        ]);
        f.render_widget(Paragraph::new(line), area);
    } else if app.git.diff_fullscreen {
        // 전체화면 힌트는 타이틀에 포함되므로 빈 줄
        f.render_widget(Paragraph::new(""), area);
        return;
    } else if app.git.log_focused && app.git.log_file_focused {
        let wrap_hint = if app.git.diff_wrap { hint_span("w", "줄바꿈OFF") } else { hint_span("w", "줄바꿈ON") };
        let hints = Line::from(vec![
            hint_span("↑↓", "파일이동"),
            hint_span("Enter/d", "diff"),
            hint_span("PageUp/Dn", "↕스크롤"),
            hint_span("[/]", "↔스크롤"),
            wrap_hint,
            hint_span("f", "전체화면"),
            hint_span("←/h/Esc", "커밋으로"),
            hint_span("q", "닫기"),
        ]);
        f.render_widget(Paragraph::new(hints), area);
    } else if app.git.log_focused {
        let hints = Line::from(vec![
            hint_span("↑↓", "커밋이동"),
            hint_span("→/Enter", "파일목록"),
            hint_span("←/Esc", "닫기"),
            hint_span("L", "로그닫기"),
            hint_span("q", "닫기"),
        ]);
        f.render_widget(Paragraph::new(hints), area);
    } else {
        let has_staged = app.git.status.as_ref().map(|s| !s.staged.is_empty()).unwrap_or(false);
        let log_hint = if app.git.show_log {
            hint_span("→/l", "로그이동")
        } else {
            hint_span("L", "로그")
        };
        let diff_available = !app.git.diff.is_empty();
        let wrap_hint = if app.git.diff_wrap { hint_span("w", "줄바꿈OFF") } else { hint_span("w", "줄바꿈ON") };
        let hints: Vec<Span> = vec![
            hint_span("a", "스테이지"),
            hint_span("u", "언스테이지"),
            if has_staged { hint_span("c", "커밋") } else { Span::raw("") },
            hint_span("f", "전체화면"),
            if diff_available { hint_span("[/]", "↔") } else { Span::raw("") },
            if diff_available { wrap_hint } else { Span::raw("") },
            log_hint,
            hint_span("r", "새로고침"),
            hint_span("Tab", "섹션전환"),
            hint_span("q", "닫기"),
        ];
        let line = Line::from(hints);
        f.render_widget(Paragraph::new(line), area);
    }
}

fn hint_span(key: &'static str, desc: &'static str) -> Span<'static> {
    // 복합 span을 하나로 합칠 수 없어서 key+desc를 하나의 Span으로 표현
    let _ = desc;
    Span::styled(
        format!(" {key}:{desc} "),
        Style::default().fg(Color::DarkGray),
    )
}

fn diff_line_color(line: &str) -> Color {
    if line.starts_with('+') && !line.starts_with("+++") {
        Color::Green
    } else if line.starts_with('-') && !line.starts_with("---") {
        Color::Red
    } else if line.starts_with("@@") {
        Color::Cyan
    } else if line.starts_with("diff ") || line.starts_with("index ") || line.starts_with("# ") {
        Color::Yellow
    } else {
        Color::Gray
    }
}

/// 색상은 원본 기준, 표시는 h_scroll 적용
fn diff_line_to_tui_h(line: &str, h_scroll: usize) -> Line<'static> {
    let color = diff_line_color(line);
    let display = if h_scroll > 0 {
        let byte_idx = line
            .char_indices()
            .nth(h_scroll)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        line[byte_idx..].to_string()
    } else {
        line.to_string()
    };
    Line::from(Span::styled(display, Style::default().fg(color)))
}

fn staged_status_color(c: char) -> Color {
    match c {
        'A' => Color::Green,
        'M' => Color::Yellow,
        'D' => Color::Red,
        'R' => Color::Cyan,
        _ => Color::White,
    }
}

fn unstaged_status_color(c: char) -> Color {
    match c {
        'M' => Color::Yellow,
        'D' => Color::Red,
        '?' => Color::DarkGray,
        _ => Color::White,
    }
}

fn get_selected_file_name(app: &App) -> Option<String> {
    let status = app.git.status.as_ref()?;
    match app.git.section {
        GitSection::Staged => status.staged.get(app.git.staged_idx).map(|f| f.path.clone()),
        GitSection::Unstaged => status.unstaged.get(app.git.unstaged_idx).map(|f| f.path.clone()),
    }
}
