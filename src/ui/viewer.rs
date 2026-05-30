use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::path::PathBuf;

use crate::app::{App, FileType};

/// 전체화면 뷰어 모드 렌더링
pub fn render_fullscreen(f: &mut Frame, area: Rect, app: &mut App) {
    let selected_path = match app.selected_path() {
        Some(p) => p.clone(),
        None => {
            render_no_file(f, area);
            return;
        }
    };

    let file_type = App::detect_file_type(&selected_path);

    // 수직 분할: 상단 툴바 + 컨텐츠 + 하단 상태바
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 뷰어 툴바
            Constraint::Min(0),    // 컨텐츠
            Constraint::Length(1), // 상태바 (스크롤 %)
        ])
        .split(area);

    // 블록 상단 테두리 없음(LEFT|RIGHT|BOTTOM)이므로 내부 높이 = height - 1
    app.viewer_height = chunks[1].height.saturating_sub(1);
    render_viewer_toolbar(f, chunks[0], &selected_path, &file_type, app.preview_wrap);
    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    // 드래그 컨텐츠 영역 기록 (LEFT|RIGHT|BOTTOM 테두리: 좌우 -1, 상단 없음, 하단 -1)
    let content_area = chunks[1];
    app.drag_content_area = Rect {
        x: content_area.x + 1,
        y: content_area.y,
        width: content_area.width.saturating_sub(2),
        height: content_area.height.saturating_sub(1),
    };

    let selection = compute_drag_selection(app);
    let current_match_line = app.viewer_search_matches.get(app.viewer_search_idx).copied();
    render_preview_content(
        f, content_area, &selected_path, &file_type,
        app.preview_scroll, app.preview_h_scroll, app.preview_wrap, block,
        &app.viewer_search_query, &app.viewer_search_matches, current_match_line,
        selection,
    );
    render_status_bar(f, chunks[2], app);
}

/// 드래그 선택 범위를 절대 줄 번호 쌍으로 변환
fn compute_drag_selection(app: &App) -> Option<(usize, usize)> {
    let (start, end) = match (app.drag_start, app.drag_end) {
        (Some(s), Some(e)) => (s, e),
        _ => return None,
    };
    let area = app.drag_content_area;
    if area.height == 0 { return None; }
    let s = (start.1.saturating_sub(area.y) as usize) + app.preview_scroll as usize;
    let e = (end.1.saturating_sub(area.y) as usize) + app.preview_scroll as usize;
    Some(if s <= e { (s, e) } else { (e, s) })
}

/// 뷰어 상단 툴바
fn render_viewer_toolbar(f: &mut Frame, area: Rect, path: &PathBuf, file_type: &FileType, wrap: bool) {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let type_label = match file_type {
        FileType::Markdown => "[MD] ",
        FileType::Code(_) => "[Code] ",
        FileType::Image => "[Image] ",
        FileType::Pdf => "[PDF] ",
        FileType::Csv => "[CSV] ",
        FileType::Parquet => "[Parquet] ",
        FileType::Archive => "[Archive] ",
        FileType::Text => "[Text] ",
        FileType::Unknown => "[File] ",
    };

    let wrap_label = if wrap { " [줄바꿈:ON] " } else { " [줄바꿈:OFF]" };
    let line = Line::from(vec![
        Span::styled(" ← 돌아가기(q) ", Style::default().fg(Color::Yellow)),
        Span::raw("│ "),
        Span::styled(type_label, Style::default().fg(Color::Cyan)),
        Span::styled(file_name, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  [e] 편집기  [←→] 좌우스크롤"),
        Span::styled(wrap_label, Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

/// 뷰어 하단 상태바 — 검색/goto 입력 또는 스크롤 정보 표시
fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    // 복사/작업 결과 상태 메시지
    if let Some(ref msg) = app.status {
        crate::ui::status_bar::render(f, area, msg);
        return;
    }
    // 검색 입력 모드
    if app.viewer_is_searching {
        let line = Line::from(vec![
            Span::styled(" / ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(app.viewer_search_query.clone(), Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Yellow)),
            Span::styled("  [Enter]검색  [Esc]취소", Style::default().fg(Color::DarkGray)),
        ]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }
    // 줄이동 입력 모드
    if app.viewer_is_goto {
        let line = Line::from(vec![
            Span::styled(" : ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(app.viewer_goto_input.clone(), Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Cyan)),
            Span::styled("  줄번호 입력  [Enter]이동  [Esc]취소", Style::default().fg(Color::DarkGray)),
        ]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }
    // 일반 상태
    let v = app.preview_scroll;
    let h = app.preview_h_scroll;
    let h_part = if h > 0 { format!("  →{h}열") } else { String::new() };
    let match_part = if !app.viewer_search_matches.is_empty() {
        let cur = app.viewer_search_idx + 1;
        let total = app.viewer_search_matches.len();
        format!("  [{cur}/{total}매칭]")
    } else if !app.viewer_search_query.is_empty() {
        "  [매칭없음]".to_string()
    } else {
        String::new()
    };
    let line = Line::from(vec![
        Span::styled(
            format!(" ↕{v}줄{h_part}{match_part}  /검색  n/N이동  :줄이동  gg/G처음끝  [?]도움말 "),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

/// 파일 없는 경우 안내
fn render_no_file(f: &mut Frame, area: Rect) {
    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from("  선택된 파일이 없습니다."),
        Line::from("  (q 로 돌아가기)"),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(para, area);
}

/// 미리보기 컨텐츠 렌더링 (패널 모드 + 전체화면 공용)
pub fn render_preview_content(
    f: &mut Frame,
    area: Rect,
    path: &PathBuf,
    file_type: &FileType,
    scroll: u16,
    h_scroll: u16,
    wrap: bool,
    block: Block,
    search_query: &str,
    search_matches: &[usize],
    search_current_line: Option<usize>,
    selection: Option<(usize, usize)>,
) {
    match file_type {
        FileType::Markdown => {
            crate::preview::markdown::render(
                f, area, path, scroll, h_scroll, wrap, block,
                search_matches, search_current_line, selection,
            );
        }
        FileType::Code(lang) => {
            crate::preview::code::render(
                f, area, path, lang, scroll, h_scroll, wrap, block,
                search_matches, search_current_line, selection,
            );
        }
        FileType::Csv => {
            crate::preview::csv::render(f, area, path, block);
        }
        FileType::Parquet => {
            crate::preview::parquet::render(f, area, path, block, scroll);
        }
        FileType::Image => {
            crate::preview::image::render(f, area, path, block);
        }
        FileType::Text | FileType::Unknown => {
            render_text_file(
                f, area, path, scroll, h_scroll, wrap, block,
                search_query, search_matches, search_current_line, selection,
            );
        }
        FileType::Pdf => {
            render_pdf_placeholder(f, area, path, block);
        }
        FileType::Archive => {
            render_archive_placeholder(f, area, path, block);
        }
    }
}

/// 일반 텍스트 파일 렌더링
fn render_text_file(
    f: &mut Frame,
    area: Rect,
    path: &PathBuf,
    scroll: u16,
    h_scroll: u16,
    wrap: bool,
    block: Block,
    search_query: &str,
    search_matches: &[usize],
    search_current_line: Option<usize>,
    selection: Option<(usize, usize)>,
) {
    let content = std::fs::read_to_string(path).unwrap_or_else(|e| format!("[읽기 오류: {e}]"));
    let lines: Vec<Line<'static>> = content
        .lines()
        .map(|l| Line::from(l.to_string()))
        .collect();

    let lines = if !search_query.is_empty() {
        highlight_text_matches(lines, search_query, search_matches, search_current_line)
    } else {
        lines
    };
    let lines = if let Some((s, e)) = selection {
        crate::preview::highlight::apply_selection_highlight(lines, s, e)
    } else {
        lines
    };

    let h = if wrap { 0 } else { h_scroll };
    let mut para = Paragraph::new(lines).block(block).scroll((scroll, h));
    if wrap {
        para = para.wrap(Wrap { trim: false });
    }
    f.render_widget(para, area);
}

/// 텍스트 줄에서 매칭 텍스트를 span 단위로 분리해 하이라이트
fn highlight_text_matches(
    lines: Vec<Line<'static>>,
    query: &str,
    match_lines: &[usize],
    current_match_line: Option<usize>,
) -> Vec<Line<'static>> {
    use ratatui::style::Color;
    let match_set: std::collections::HashSet<usize> = match_lines.iter().copied().collect();
    let q_lower = query.to_lowercase();

    lines.into_iter().enumerate().map(|(i, line)| {
        if !match_set.contains(&i) {
            return line;
        }
        let is_current = current_match_line == Some(i);
        // 줄 전체 텍스트 추출
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        let text_lower = text.to_lowercase();

        let match_bg = if is_current { Color::Yellow } else { Color::Rgb(80, 60, 0) };
        let match_fg = if is_current { Color::Black } else { Color::White };
        let base_bg = if is_current { Color::Rgb(60, 50, 0) } else { Color::Rgb(40, 30, 0) };

        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut pos = 0usize;
        while pos < text.len() {
            if let Some(start) = text_lower[pos..].find(&q_lower) {
                let abs_start = pos + start;
                let abs_end = abs_start + q_lower.len();
                if abs_start > pos {
                    spans.push(Span::styled(
                        text[pos..abs_start].to_string(),
                        ratatui::style::Style::default().bg(base_bg),
                    ));
                }
                spans.push(Span::styled(
                    text[abs_start..abs_end].to_string(),
                    ratatui::style::Style::default().bg(match_bg).fg(match_fg)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ));
                pos = abs_end;
            } else {
                spans.push(Span::styled(
                    text[pos..].to_string(),
                    ratatui::style::Style::default().bg(base_bg),
                ));
                break;
            }
        }
        Line::from(spans)
    }).collect()
}

/// PDF 플레이스홀더
fn render_pdf_placeholder(f: &mut Frame, area: Rect, path: &PathBuf, block: Block) {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(format!("  PDF: {name}")),
        Line::from(""),
        Line::from("  PDF 렌더링은 v0.3에서 지원 예정입니다."),
        Line::from("  (pdfium-render 바인딩)"),
    ])
    .block(block);
    f.render_widget(para, area);
}

/// 압축파일 플레이스홀더
fn render_archive_placeholder(f: &mut Frame, area: Rect, path: &PathBuf, block: Block) {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(format!("  압축 파일: {name}")),
        Line::from(""),
        Line::from("  압축 내부 탐색은 v0.2에서 지원 예정입니다."),
    ])
    .block(block);
    f.render_widget(para, area);
}
