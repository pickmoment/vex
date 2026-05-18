use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;
use crate::fs::ops::FileEntry;

/// 파일 목록 위젯 렌더링
pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    // 검색 중이면 하단 1줄을 검색 바로 분리
    let (list_area, search_area) = if app.is_searching {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let border_color = if app.is_searching { Color::Yellow } else { Color::Blue };
    let title = if app.is_searching {
        format!(" 파일 목록  {}건 ", app.filtered_indices.len())
    } else {
        " 파일 목록 ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let git_root = app.git_status.as_ref().map(|s| &s.root);
    let git_map = app.git_status.as_ref().map(|s| &s.file_map);

    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .map(|&i| {
            let entry = &app.file_entries[i];
            let git_marker = if !entry.is_dir {
                git_root.zip(git_map).and_then(|(root, map)| {
                    entry.path.strip_prefix(root).ok()
                        .and_then(|p| p.to_str())
                        .and_then(|rel| map.get(rel))
                        .copied()
                })
            } else {
                None
            };
            make_list_item(entry, &app.config, &app.search_query, git_marker)
        })
        .collect();

    let mut state = ListState::default();
    state.select(if app.filtered_indices.is_empty() { None } else { Some(app.selected_index) });

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, list_area, &mut state);

    // 검색 바 렌더링
    if let Some(sa) = search_area {
        let query_display = app.search_query.clone();
        let line = Line::from(vec![
            Span::styled(" / ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(query_display, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Yellow)),
        ]);
        f.render_widget(Paragraph::new(line), sa);
    }
}

/// 파일 항목 → ListItem 변환 (검색어 매칭 부분 강조)
fn make_list_item<'a>(
    entry: &'a FileEntry,
    config: &crate::config::Config,
    query: &str,
    git_marker: Option<(char, char)>,
) -> ListItem<'a> {
    let icon = if config.ui.show_icons {
        get_icon(entry)
    } else {
        if entry.is_dir { "D " } else { "F " }
    };

    let size_str = if entry.is_dir {
        "      ".to_string()
    } else {
        format_size(entry.size)
    };

    let name_spans = if !query.is_empty() {
        highlight_match(&entry.name, query, entry.is_dir)
    } else {
        let style = if entry.is_dir {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        vec![Span::styled(entry.name.clone(), style)]
    };

    let mut spans = vec![Span::raw(icon)];
    spans.extend(name_spans);
    if let Some((x, y)) = git_marker {
        let (marker_char, color) = if x != ' ' && x != '?' {
            (x, Color::Green)
        } else if y == 'M' {
            (y, Color::Yellow)
        } else if y == 'D' {
            (y, Color::Red)
        } else if x == '?' {
            ('?', Color::DarkGray)
        } else {
            (' ', Color::DarkGray)
        };
        spans.push(Span::styled(
            format!(" {marker_char}"),
            Style::default().fg(color),
        ));
    }
    spans.push(Span::styled(
        format!("  {size_str}"),
        Style::default().fg(Color::DarkGray),
    ));

    ListItem::new(Line::from(spans))
}

/// 파일명에서 검색어 매칭 부분을 강조 표시
fn highlight_match(name: &str, query: &str, is_dir: bool) -> Vec<Span<'static>> {
    let name_lower = name.to_lowercase();
    let query_lower = query.to_lowercase();

    let base_style = if is_dir {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let match_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

    match name_lower.find(&query_lower) {
        None => vec![Span::styled(name.to_string(), base_style)],
        Some(start) => {
            let end = start + query_lower.len();
            let mut spans = Vec::new();
            if start > 0 {
                spans.push(Span::styled(name[..start].to_string(), base_style));
            }
            spans.push(Span::styled(name[start..end].to_string(), match_style));
            if end < name.len() {
                spans.push(Span::styled(name[end..].to_string(), base_style));
            }
            spans
        }
    }
}

/// 파일 타입별 아이콘 반환
fn get_icon(entry: &FileEntry) -> &'static str {
    if entry.is_dir {
        return " ";
    }
    let ext = entry
        .path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "md" | "markdown" => " ",
        "rs" => " ",
        "py" => " ",
        "js" | "mjs" | "ts" | "tsx" => " ",
        "go" => " ",
        "c" | "cpp" | "h" => " ",
        "java" => " ",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" => " ",
        "pdf" => " ",
        "csv" | "tsv" => " ",
        "zip" | "tar" | "gz" | "bz2" | "7z" | "rar" => " ",
        "toml" | "yaml" | "yml" | "json" => " ",
        "sh" | "bash" | "zsh" => " ",
        "txt" | "log" => " ",
        _ => " ",
    }
}

/// 파일 크기 포맷팅
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.0}K", size as f64 / KB as f64)
    } else {
        format!("{size}B")
    }
}
