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
    selection: Option<(usize, usize)>,
) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            let para = Paragraph::new(format!("[읽기 오류: {e}]")).block(block);
            f.render_widget(para, area);
            return;
        }
    };

    let max_width = if wrap {
        Some(area.width.saturating_sub(2) as usize)
    } else {
        None
    };
    let lines = render_markdown(&content, max_width);
    let lines = crate::preview::highlight::apply_search_highlights(lines, search_matches, search_current_line);
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

/// 마크다운 텍스트 → ratatui Line 목록 변환
pub fn render_markdown(content: &str, max_width: Option<usize>) -> Vec<Line<'static>> {
    let (fm_entries, body) = extract_frontmatter(content);

    let mut lines: Vec<Line<'static>> = Vec::new();

    if !fm_entries.is_empty() {
        let header = vec!["속성".to_string(), "값".to_string()];
        let body_rows: Vec<Vec<String>> = fm_entries
            .into_iter()
            .map(|(k, v)| vec![k, v])
            .collect();
        lines.push(Line::from(Span::styled(
            "  ─── Frontmatter ".to_string()
                + &"─".repeat(30),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
        lines.extend(render_table_lines(&header, &body_rows, max_width));
        lines.push(Line::from(""));
    }

    let content = body;

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(content, options);

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
                    let table_lines = render_table_lines(&table_header, &table_body, max_width);
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
fn render_table_lines(header: &[String], body: &[Vec<String>], max_width: Option<usize>) -> Vec<Line<'static>> {
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

    // wrap 모드일 때 화면 너비에 맞게 컬럼 너비 제한
    // 테이블 총 너비 = 3 ("  │") + num_cols * (col_width + 3 (" cell │"))
    let col_widths = if let Some(mw) = max_width {
        constrain_col_widths(&col_widths, mw)
    } else {
        col_widths
    };

    let border_style = Style::default().fg(Color::DarkGray);
    let header_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let cell_style = Style::default().fg(Color::White);

    let mut lines: Vec<Line<'static>> = Vec::new();

    // ┌──────┬──────┐
    lines.push(Line::from(Span::styled(
        table_border_top(&col_widths),
        border_style,
    )));

    // │ 헤더 │ 헤더 │ (셀 내용이 길면 여러 줄로 표시)
    lines.extend(render_multi_line_row(header, &col_widths, num_cols, header_style, border_style));

    // ├──────┼──────┤
    lines.push(Line::from(Span::styled(
        table_border_mid(&col_widths),
        border_style,
    )));

    // │ 셀   │ 셀   │
    for row in body {
        lines.extend(render_multi_line_row(row, &col_widths, num_cols, cell_style, border_style));
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

/// 한 행을 렌더링. 셀 내용이 컬럼 너비를 초과하면 여러 줄로 래핑
fn render_multi_line_row(
    cells: &[String],
    col_widths: &[usize],
    num_cols: usize,
    cell_style: Style,
    border_style: Style,
) -> Vec<Line<'static>> {
    let wrapped: Vec<Vec<String>> = (0..num_cols)
        .map(|i| {
            let cell = cells.get(i).cloned().unwrap_or_default();
            wrap_cell(&cell, col_widths[i])
        })
        .collect();

    let row_height = wrapped.iter().map(|c| c.len()).max().unwrap_or(1);
    let mut result = Vec::new();

    for line_idx in 0..row_height {
        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled("  │".to_string(), border_style));
        for col_idx in 0..num_cols {
            let content = wrapped[col_idx].get(line_idx).cloned().unwrap_or_default();
            let padded = format!(
                " {}{} ",
                content,
                " ".repeat(col_widths[col_idx].saturating_sub(display_width(&content)))
            );
            spans.push(Span::styled(padded, cell_style));
            spans.push(Span::styled("│".to_string(), border_style));
        }
        result.push(Line::from(spans));
    }

    result
}

/// 셀 텍스트를 max_w 너비 내에서 단어 단위로 줄바꿈
fn wrap_cell(s: &str, max_w: usize) -> Vec<String> {
    if max_w == 0 {
        return vec![String::new()];
    }
    if display_width(s) <= max_w {
        return vec![s.to_string()];
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_w = 0usize;

    for word in s.split_whitespace() {
        let word_w = display_width(word);
        if current_w == 0 {
            if word_w <= max_w {
                current.push_str(word);
                current_w = word_w;
            } else {
                // 단어 자체가 컬럼보다 길면 문자 단위로 나눔
                for part in break_word(word, max_w) {
                    if display_width(&part) == max_w {
                        lines.push(part);
                    } else {
                        current = part.clone();
                        current_w = display_width(&part);
                    }
                }
            }
        } else if current_w + 1 + word_w <= max_w {
            current.push(' ');
            current.push_str(word);
            current_w += 1 + word_w;
        } else {
            lines.push(current.clone());
            current.clear();
            current_w = 0;
            if word_w <= max_w {
                current.push_str(word);
                current_w = word_w;
            } else {
                for part in break_word(word, max_w) {
                    if display_width(&part) == max_w {
                        lines.push(part);
                    } else {
                        current = part.clone();
                        current_w = display_width(&part);
                    }
                }
            }
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// 단어가 max_w보다 길 때 문자 단위로 분할
fn break_word(word: &str, max_w: usize) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut w = 0usize;
    for c in word.chars() {
        let cw = char_width(c);
        if w + cw > max_w {
            if !current.is_empty() {
                parts.push(current.clone());
                current.clear();
                w = 0;
            }
        }
        current.push(c);
        w += cw;
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// wrap 모드에서 테이블 총 너비가 max_width를 넘지 않도록 컬럼 너비를 비례 축소
fn constrain_col_widths(col_widths: &[usize], max_width: usize) -> Vec<usize> {
    let num_cols = col_widths.len();
    if num_cols == 0 {
        return vec![];
    }
    // 총 너비 = 3 + num_cols * 3 + sum(col_widths)
    let overhead = 3 + 3 * num_cols;
    let total: usize = col_widths.iter().sum();
    if overhead + total <= max_width {
        return col_widths.to_vec();
    }
    let available = max_width.saturating_sub(overhead);
    if available < num_cols {
        return vec![1; num_cols];
    }
    // 자연 너비 비율로 분배
    let mut result: Vec<usize> = col_widths.iter()
        .map(|&w| ((w * available) / total).max(1))
        .collect();
    // 정수 나눗셈 오차 보정
    let used: usize = result.iter().sum();
    if used < available {
        for i in 0..(available - used).min(num_cols) {
            result[i] += 1;
        }
    } else if used > available {
        let mut excess = used - available;
        for i in (0..num_cols).rev() {
            if excess == 0 { break; }
            let reduce = result[i].saturating_sub(1).min(excess);
            result[i] -= reduce;
            excess -= reduce;
        }
    }
    result
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

/// YAML frontmatter 추출 (문서 앞의 --- 블록)
fn extract_frontmatter(content: &str) -> (Vec<(String, String)>, &str) {
    let rest = if let Some(r) = content.strip_prefix("---\n") {
        r
    } else if let Some(r) = content.strip_prefix("---\r\n") {
        r
    } else {
        return (vec![], content);
    };

    let Some(close_pos) = rest.find("\n---") else {
        return (vec![], content);
    };

    let fm_str = &rest[..close_pos];
    let after_close = &rest[close_pos + 4..]; // skip "\n---"
    let remaining = if after_close.starts_with("\r\n") {
        &after_close[2..]
    } else if after_close.starts_with('\n') {
        &after_close[1..]
    } else {
        after_close
    };

    let entries = parse_yaml_simple(fm_str);
    if entries.is_empty() {
        return (vec![], content);
    }
    (entries, remaining)
}

/// 간단한 YAML key: value / 리스트 파싱
fn parse_yaml_simple(yaml: &str) -> Vec<(String, String)> {
    let mut entries: Vec<(String, String)> = Vec::new();
    let mut current_key: Option<String> = None;
    let mut list_items: Vec<String> = Vec::new();

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with("- ") {
            list_items.push(trimmed[2..].trim().to_string());
            continue;
        }

        // 이전 리스트 항목 플러시
        if !list_items.is_empty() {
            if let Some(k) = current_key.take() {
                entries.push((k, list_items.join(", ")));
            }
            list_items.clear();
        }

        if let Some(colon_pos) = line.find(": ") {
            let key = line[..colon_pos].trim().to_string();
            let raw_val = line[colon_pos + 2..].trim();
            let value = raw_val.trim_matches('"').trim_matches('\'').to_string();
            if !key.is_empty() {
                if value.is_empty() {
                    current_key = Some(key);
                } else {
                    entries.push((key, value));
                    current_key = None;
                }
            }
        } else if let Some(stripped) = trimmed.strip_suffix(':') {
            current_key = Some(stripped.trim().to_string());
        }
    }

    // 마지막 항목 플러시
    if !list_items.is_empty() {
        if let Some(k) = current_key {
            entries.push((k, list_items.join(", ")));
        }
    } else if let Some(k) = current_key {
        entries.push((k, String::new()));
    }

    entries
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
