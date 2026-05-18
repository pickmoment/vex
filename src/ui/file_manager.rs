use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, FmOp};

const MENU_ITEMS: &[(&str, &str, &str)] = &[
    ("복사", "대상 경로로 복사", "c"),
    ("이동", "대상 경로로 이동", "v"),
    ("이름 변경", "현재 디렉토리에서 이름 변경", "r"),
    ("삭제", "파일/디렉토리 삭제", "d"),
];

pub fn render(f: &mut Frame, app: &App) {
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
    let area = centered_rect_abs(54, 10, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" 삭제 확인 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

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
        Line::from(""),
    ];
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
        _ => ("", ""),
    };

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
        Line::from(vec![
            Span::styled(
                format!("  {input_label}: "),
                Style::default().fg(Color::White),
            ),
            Span::styled(app.fm_input.clone(), Style::default().fg(Color::Yellow)),
            Span::styled("█", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
    ];
    if let Some(ref err) = app.fm_error {
        lines.push(Line::from(Span::styled(
            format!("  오류: {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    f.render_widget(Paragraph::new(lines).block(block), inner[0]);

    let hint = Line::from(vec![
        Span::styled(" Enter ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" 확인  ", Style::default().fg(Color::DarkGray)),
        Span::styled(" Esc ", Style::default().fg(Color::DarkGray)),
        Span::raw("메뉴로"),
    ]);
    f.render_widget(Paragraph::new(hint), inner[1]);
}

fn centered_rect_abs(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}
