use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

/// 검색 매칭 줄에 배경 하이라이트 적용
pub fn apply_search_highlights(
    lines: Vec<Line<'static>>,
    match_lines: &[usize],
    current_match_line: Option<usize>,
) -> Vec<Line<'static>> {
    if match_lines.is_empty() {
        return lines;
    }
    let match_set: std::collections::HashSet<usize> = match_lines.iter().copied().collect();
    lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            if !match_set.contains(&i) {
                return line;
            }
            let is_current = current_match_line == Some(i);
            let spans: Vec<Span<'static>> = if is_current {
                line.spans
                    .into_iter()
                    .map(|s| Span::styled(s.content, s.style.bg(Color::Yellow).fg(Color::Black)))
                    .collect()
            } else {
                line.spans
                    .into_iter()
                    .map(|s| Span::styled(s.content, s.style.bg(Color::Rgb(80, 60, 0))))
                    .collect()
            };
            Line::from(spans)
        })
        .collect()
}
