use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, FmOp};
use crate::ui::layout::centered_rect_abs;

const MENU_ITEMS: &[(&str, &str, &str)] = &[
    ("복사", "대상 경로로 복사", "c"),
    ("이동", "대상 경로로 이동", "v"),
    ("이름 변경", "현재 디렉토리에서 이름 변경", "r"),
    ("삭제", "파일/디렉토리 삭제", "d"),
    ("새 폴더", "현재 디렉토리에 새 폴더 생성", "n"),
];

pub fn render(f: &mut Frame, app: &App) {
    if app.fm_overwrite_target.is_some() {
        render_overwrite_confirm(f, app);
        return;
    }

    let file_name = app
        .selected_path()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("(없음)")
        .to_string();

    match &app.fm_operation {
        None => render_menu(f, app, &file_name),
        Some(FmOp::Delete) => render_delete_confirm(f, app, &file_name),
        Some(_) => render_input(f, app, &file_name),
    }
}

fn render_menu(f: &mut Frame, app: &App, file_name: &str) {
    let area = centered_rect_abs(52, 12, f.area());
    f.render_widget(Clear, area);

    let title = format!(" 파일 관리: {file_name} ");
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .map(|(label, desc, key)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  {label:<8}", label = label),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {desc}"),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("  [{key}]"),
                    Style::default().fg(Color::Cyan),
                ),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.fm_menu_idx.min(MENU_ITEMS.len() - 1)));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, inner[0], &mut state);

    let hint = Line::from(vec![
        Span::styled(" ↑↓ ", Style::default().fg(Color::DarkGray)),
        Span::raw("이동  "),
        Span::styled(" Enter ", Style::default().fg(Color::DarkGray)),
        Span::raw("선택  "),
        Span::styled(" Esc/q ", Style::default().fg(Color::DarkGray)),
        Span::raw("취소"),
    ]);
    f.render_widget(Paragraph::new(hint), inner[1]);
}

fn render_delete_confirm(f: &mut Frame, app: &App, file_name: &str) {
    let area = centered_rect_abs(58, 11, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" 삭제 확인 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    // 대상이 디렉토리면 항목 수 계산
    let dir_info = app.selected_path().and_then(|p| {
        if p.is_dir() {
            let count = std::fs::read_dir(p).map(|r| r.count()).unwrap_or(0);
            Some(count)
        } else {
            None
        }
    });

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ⚠  정말 삭제하시겠습니까?",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {file_name}"),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
    ];
    if let Some(count) = dir_info {
        lines.push(Line::from(Span::styled(
            format!("  (디렉토리 — {count}개 항목과 모든 하위 파일 삭제)"),
            Style::default().fg(Color::Yellow),
        )));
    }
    lines.push(Line::from(""));
    if let Some(ref err) = app.fm_error {
        lines.push(Line::from(Span::styled(
            format!("  오류: {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    f.render_widget(Paragraph::new(lines).block(block), inner[0]);

    let hint = Line::from(vec![
        Span::styled(" y ", Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled(" 삭제  ", Style::default().fg(Color::DarkGray)),
        Span::styled(" n/Esc ", Style::default().fg(Color::DarkGray)),
        Span::raw("취소"),
    ]);
    f.render_widget(Paragraph::new(hint), inner[1]);
}

fn render_input(f: &mut Frame, app: &App, file_name: &str) {
    let (op_label, input_label) = match app.fm_operation {
        Some(FmOp::Rename) => ("이름 변경", "새 이름"),
        Some(FmOp::Copy)   => ("복사", "대상 경로"),
        Some(FmOp::Move)   => ("이동", "대상 경로"),
        Some(FmOp::NewDir) => ("새 폴더", "폴더 이름"),
        _ => ("", ""),
    };
    let show_tab_hint = matches!(app.fm_operation, Some(FmOp::Copy) | Some(FmOp::Move))
        && !app.path_clipboard.is_empty();

    let area = centered_rect_abs(64, 11, f.area());
    f.render_widget(Clear, area);

    let title = format!(" {op_label}: {file_name} ");
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  원본: {file_name}"),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        {
            let cursor = app.fm_cursor.min(app.fm_input.len());
            let before = &app.fm_input[..cursor];
            let after_str = &app.fm_input[cursor..];
            let (cursor_ch, after) = if let Some(c) = after_str.chars().next() {
                (&after_str[..c.len_utf8()], &after_str[c.len_utf8()..])
            } else {
                (" ", "")
            };
            Line::from(vec![
                Span::styled(format!("  {input_label}: "), Style::default().fg(Color::White)),
                Span::styled(before.to_string(), Style::default().fg(Color::Yellow)),
                Span::styled(cursor_ch.to_string(), Style::default().fg(Color::Black).bg(Color::Yellow)),
                Span::styled(after.to_string(), Style::default().fg(Color::Yellow)),
            ])
        },
        Line::from(""),
    ];
    if let Some(ref err) = app.fm_error {
        lines.push(Line::from(Span::styled(
            format!("  오류: {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    f.render_widget(Paragraph::new(lines).block(block), inner[0]);

    let mut hint_spans = vec![
        Span::styled(" Enter ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" 확인  ", Style::default().fg(Color::DarkGray)),
    ];
    if show_tab_hint {
        hint_spans.push(Span::styled(" Tab ", Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)));
        hint_spans.push(Span::styled(
            format!(" 경로목록({})  ", app.path_clipboard.len()),
            Style::default().fg(Color::DarkGray),
        ));
    }
    hint_spans.push(Span::styled(" Esc ", Style::default().fg(Color::DarkGray)));
    hint_spans.push(Span::raw("메뉴로"));
    let hint = Line::from(hint_spans);
    f.render_widget(Paragraph::new(hint), inner[1]);
}

fn render_overwrite_confirm(f: &mut Frame, app: &App) {
    let dst_str = app
        .fm_overwrite_target
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let area = centered_rect_abs(64, 10, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" 덮어쓰기 확인 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = ratatui::layout::Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ⚠  대상이 이미 존재합니다. 덮어쓰시겠습니까?",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {dst_str}"),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    f.render_widget(Paragraph::new(lines).block(block), inner[0]);

    let hint = Line::from(vec![
        Span::styled(" y ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" 덮어쓰기  ", Style::default().fg(Color::DarkGray)),
        Span::styled(" n/Esc ", Style::default().fg(Color::DarkGray)),
        Span::raw("취소"),
    ]);
    f.render_widget(Paragraph::new(hint), inner[1]);
}
