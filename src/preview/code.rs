use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};
use std::path::PathBuf;
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

/// 코드 파일을 신택스 하이라이팅과 함께 렌더링
pub fn render(
    f: &mut Frame,
    area: Rect,
    path: &PathBuf,
    _lang: &str,
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

    let lines = highlight_code(&content, path);
    let lines = crate::preview::highlight::apply_search_highlights(lines, search_matches, search_current_line);
    let h = if wrap { 0 } else { h_scroll };
    let mut para = Paragraph::new(lines).block(block).scroll((scroll, h));
    if wrap {
        para = para.wrap(Wrap { trim: false });
    }
    f.render_widget(para, area);
}

/// 소스 코드 → 하이라이팅된 ratatui Line 목록
pub fn highlight_code(content: &str, path: &PathBuf) -> Vec<Line<'static>> {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(|ext| ss.find_syntax_by_extension(ext))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let theme = &ts.themes["base16-ocean.dark"];
    let mut h = HighlightLines::new(syntax, theme);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut line_num = 1u32;

    for line in LinesWithEndings::from(content) {
        let ranges = h.highlight_line(line, &ss).unwrap_or_default();
        let mut spans: Vec<Span<'static>> = vec![
            // 줄 번호
            Span::styled(
                format!("{line_num:4} │ ", ),
                Style::default().fg(Color::DarkGray),
            ),
        ];

        for (style, text) in &ranges {
            let fg = syntect_color_to_ratatui(style.foreground);
            let mut rat_style = Style::default().fg(fg);
            if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
                rat_style = rat_style.add_modifier(Modifier::BOLD);
            }
            if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
                rat_style = rat_style.add_modifier(Modifier::ITALIC);
            }
            // 줄 끝 개행 제거
            let clean = text.trim_end_matches(['\n', '\r']);
            if !clean.is_empty() {
                spans.push(Span::styled(clean.to_string(), rat_style));
            }
        }

        lines.push(Line::from(spans));
        line_num += 1;
    }

    lines
}

/// syntect RGBA → ratatui Color 변환
fn syntect_color_to_ratatui(c: syntect::highlighting::Color) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}
