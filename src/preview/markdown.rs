use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};
use std::path::PathBuf;

/// 마크다운 파일을 ratatui 위젯으로 렌더링
pub fn render(
    f: &mut Frame,
    area: Rect,
    path: &PathBuf,
    scroll: u16,
    h_scroll: u16,
    wrap: bool,
    block: Block,
    search_matches: &[usize],
    search_current_line: Option<usize>,
) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            let para = Paragraph::new(format!("[읽기 오류: {e}]")).block(block);
            f.render_widget(para, area);
            return;
        }
    };

    let lines = render_markdown(&content);
    let lines = crate::preview::highlight::apply_search_highlights(lines, search_matches, search_current_line);
    let h = if wrap { 0 } else { h_scroll };
    let mut para = Paragraph::new(lines).block(block).scroll((scroll, h));
    if wrap {
        para = para.wrap(Wrap { trim: false });
    }
    f.render_widget(para, area);
}

/// 마크다운 텍스트 → ratatui Line 목록 변환
pub fn render_markdown(content: &str) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(content, options);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];
    let mut in_code_block = false;
    let mut code_buf = String::new();

    // 표 상태
    let mut in_table = false;
    let mut in_table_head = false;
    let mut in_table_cell = false;
    let mut table_header: Vec<String> = Vec::new();
    let mut table_body: Vec<Vec<String>> = Vec::new();
    let mut table_current_row: Vec<String> = Vec::new();
    let mut table_current_cell = String::new();

    let mut list_depth: u32 = 0;
    let mut ordered_list_counter: u32 = 0;

    macro_rules! current_style {
        () => {
            style_stack.last().copied().unwrap_or_default()
        };
    }

    macro_rules! flush_line {
        () => {{
            let spans = std::mem::take(&mut current_spans);
            lines.push(Line::from(spans));
        }};
    }

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    flush_line!();
                    let (prefix, color, bold) = heading_style(level);
                    let style = Style::default()
                        .fg(color)
                        .add_modifier(if bold { Modifier::BOLD } else { Modifier::empty() });
                    style_stack.push(style);
                    current_spans.push(Span::styled(prefix.to_string(), style));
                }
                Tag::Strong => {
                    let s = current_style!().add_modifier(Modifier::BOLD);
                    style_stack.push(s);
                }
                Tag::Emphasis => {
                    let s = current_style!().add_modifier(Modifier::ITALIC);
                    style_stack.push(s);
                }
                Tag::Strikethrough => {
                    let s = current_style!()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::CROSSED_OUT);
                    style_stack.push(s);
                }
                Tag::CodeBlock(_) => {
                    in_code_block = true;
                    code_buf.clear();
                    flush_line!();
                    lines.push(Line::from(Span::styled(
                        "  ┌─ 코드 ────────────────────────────────────".to_string(),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                Tag::BlockQuote(_) => {
                    let s = Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::ITALIC);
                    style_stack.push(s);
                    current_spans.push(Span::styled("  ▌ ".to_string(), s));
                }
                Tag::List(Some(start)) => {
                    list_depth += 1;
                    ordered_list_counter = start as u32;
                    flush_line!();
                }
                Tag::List(None) => {
                    list_depth += 1;
                    flush_line!();
                }
                Tag::Item => {
                    flush_line!();
                    let indent = "  ".repeat(list_depth as usize);
                    if ordered_list_counter > 0 {
                        let prefix = format!("{indent}{}. ", ordered_list_counter);
                        ordered_list_counter += 1;
                        current_spans.push(Span::raw(prefix));
                    } else {
                        current_spans.push(Span::styled(
                            format!("{indent}• "),
                            Style::default().fg(Color::Cyan),
                        ));
                    }
                }
                Tag::Table(_) => {
                    in_table = true;
                    table_header.clear();
                    table_body.clear();
                    flush_line!();
                }
                Tag::TableHead => {
                    in_table_head = true;
                    table_current_row.clear();
                }
                Tag::TableRow => {
                    table_current_row.clear();
                }
                Tag::TableCell => {
                    in_table_cell = true;
                    table_current_cell.clear();
                }
                Tag::Paragraph => {
                    if !current_spans.is_empty() {
                        flush_line!();
                    }
                }
                Tag::HtmlBlock => {}
                _ => {}
            },

            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    flush_line!();
                    lines.push(Line::from(""));
                    style_stack.pop();
                }
                TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough | TagEnd::BlockQuote => {
                    style_stack.pop();
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    for code_line in code_buf.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  │ {code_line}"),
                            Style::default().fg(Color::Green),
                        )));
                    }
                    lines.push(Line::from(Span::styled(
                        "  └────────────────────────────────────────────".to_string(),
                        Style::default().fg(Color::DarkGray),
                    )));
                    lines.push(Line::from(""));
                    code_buf.clear();
                }
                TagEnd::Paragraph => {
                    flush_line!();
                    lines.push(Line::from(""));
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                    ordered_list_counter = 0;
                    flush_line!();
                    lines.push(Line::from(""));
                }
                TagEnd::Item => {
                    flush_line!();
                }
                TagEnd::TableCell => {
                    table_current_row.push(table_current_cell.clone());
                    table_current_cell.clear();
                    in_table_cell = false;
                }
                TagEnd::TableHead => {
                    table_header = table_current_row.clone();
                    table_current_row.clear();
                    in_table_head = false;
                }
                TagEnd::TableRow => {
                    if !in_table_head {
                        table_body.push(table_current_row.clone());
                        table_current_row.clear();
                    }
                }
                TagEnd::Table => {
                    in_table = false;
                    let table_lines = render_table_lines(&table_header, &table_body);
                    lines.extend(table_lines);
                    lines.push(Line::from(""));
                }
                _ => {}
            },

            Event::Text(text) => {
                if in_code_block {
                    code_buf.push_str(&text);
                } else if in_table_cell {
                    table_current_cell.push_str(&text);
                } else if !in_table {
                    current_spans.push(Span::styled(text.to_string(), current_style!()));
                }
            }

            Event::Code(code) => {
                if in_table_cell {
                    table_current_cell.push('`');
                    table_current_cell.push_str(&code);
                    table_current_cell.push('`');
                } else {
                    current_spans.push(Span::styled(
                        format!("`{code}`"),
                        Style::default()
                            .fg(Color::Green)
                            .bg(Color::DarkGray),
                    ));
                }
            }

            Event::SoftBreak => {
                if !in_table {
                    current_spans.push(Span::raw(" "));
                }
            }
            Event::HardBreak => {
                if !in_table {
                    flush_line!();
                }
            }

            Event::Rule => {
                flush_line!();
                lines.push(Line::from(Span::styled(
                    "  ────────────────────────────────────────────────".to_string(),
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
            }

            Event::TaskListMarker(checked) => {
                let marker = if checked { "☑ " } else { "☐ " };
                let color = if checked { Color::Green } else { Color::Yellow };
                current_spans.push(Span::styled(
                    marker.to_string(),
                    Style::default().fg(color),
                ));
            }

            _ => {}
        }
    }

    if !current_spans.is_empty() {
        flush_line!();
    }

    lines
}

/// 표 데이터를 박스 드로잉 문자로 렌더링
fn render_table_lines(header: &[String], body: &[Vec<String>]) -> Vec<Line<'static>> {
    let num_cols = header.len().max(body.iter().map(|r| r.len()).max().unwrap_or(0));
    if num_cols == 0 {
        return vec![];
    }

    // 각 컬럼의 표시 너비 계산
    let col_widths: Vec<usize> = (0..num_cols)
        .map(|i| {
            let h = header.get(i).map(|s| display_width(s)).unwrap_or(0);
            let b = body.iter()
                .map(|row| row.get(i).map(|s| display_width(s)).unwrap_or(0))
                .max()
                .unwrap_or(0);
            h.max(b).max(1)
        })
        .collect();

    let border_style = Style::default().fg(Color::DarkGray);
    let header_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let cell_style = Style::default().fg(Color::White);

    let mut lines: Vec<Line<'static>> = Vec::new();

    // ┌──────┬──────┐
    lines.push(Line::from(Span::styled(
        table_border_top(&col_widths),
        border_style,
    )));

    // │ 헤더 │ 헤더 │
    lines.push(table_row_line(header, &col_widths, num_cols, header_style, border_style));

    // ├──────┼──────┤
    lines.push(Line::from(Span::styled(
        table_border_mid(&col_widths),
        border_style,
    )));

    // │ 셀   │ 셀   │
    for row in body {
        lines.push(table_row_line(row, &col_widths, num_cols, cell_style, border_style));
    }

    // └──────┴──────┘
    lines.push(Line::from(Span::styled(
        table_border_bot(&col_widths),
        border_style,
    )));

    lines
}

fn table_border_top(col_widths: &[usize]) -> String {
    let inner = col_widths.iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("┬");
    format!("  ┌{inner}┐")
}

fn table_border_mid(col_widths: &[usize]) -> String {
    let inner = col_widths.iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("┼");
    format!("  ├{inner}┤")
}

fn table_border_bot(col_widths: &[usize]) -> String {
    let inner = col_widths.iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("┴");
    format!("  └{inner}┘")
}

fn table_row_line(
    cells: &[String],
    col_widths: &[usize],
    num_cols: usize,
    cell_style: Style,
    border_style: Style,
) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled("  │".to_string(), border_style));
    for i in 0..num_cols {
        let cell = cells.get(i).cloned().unwrap_or_default();
        let padded = format!(" {}{} ", cell, " ".repeat(col_widths[i].saturating_sub(display_width(&cell))));
        spans.push(Span::styled(padded, cell_style));
        spans.push(Span::styled("│".to_string(), border_style));
    }
    Line::from(spans)
}

/// 문자열의 터미널 표시 너비 계산 (CJK 문자는 2칸)
fn display_width(s: &str) -> usize {
    s.chars().map(|c| char_width(c)).sum()
}

fn char_width(c: char) -> usize {
    let cp = c as u32;
    if (0x1100..=0x115F).contains(&cp)   // Hangul Jamo
        || (0x2E80..=0x303E).contains(&cp) // CJK Radicals
        || (0x3041..=0x33FF).contains(&cp) // Japanese
        || (0x3400..=0x4DBF).contains(&cp) // CJK Extension A
        || (0x4E00..=0x9FFF).contains(&cp) // CJK Unified
        || (0xA000..=0xA4FF).contains(&cp) // Yi
        || (0xAC00..=0xD7AF).contains(&cp) // Hangul Syllables
        || (0xF900..=0xFAFF).contains(&cp) // CJK Compatibility
        || (0xFE10..=0xFE6F).contains(&cp) // CJK Compatibility Forms
        || (0xFF01..=0xFF60).contains(&cp) // Fullwidth
        || (0xFFE0..=0xFFE6).contains(&cp) // Fullwidth Signs
    {
        2
    } else {
        1
    }
}

/// 제목 레벨별 스타일 반환 (prefix, color, bold)
fn heading_style(level: HeadingLevel) -> (&'static str, Color, bool) {
    match level {
        HeadingLevel::H1 => ("# ", Color::Cyan, true),
        HeadingLevel::H2 => ("## ", Color::Blue, true),
        HeadingLevel::H3 => ("### ", Color::Green, true),
        HeadingLevel::H4 => ("#### ", Color::Yellow, false),
        HeadingLevel::H5 => ("##### ", Color::Magenta, false),
        HeadingLevel::H6 => ("###### ", Color::DarkGray, false),
    }
}
