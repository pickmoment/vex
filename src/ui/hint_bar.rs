use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, AppMode, FocusedPanel};

/// 하단 힌트 바 렌더링 (컨텍스트별 단축키 표시)
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let hints = get_hints(app);
    let spans: Vec<Span> = hints
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(
                    format!(" {key}"),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {desc} "),
                    Style::default().fg(Color::DarkGray),
                ),
            ]
        })
        .collect();

    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), area);
}

/// 모드별 힌트 목록 반환
fn get_hints(app: &App) -> Vec<(&'static str, &'static str)> {
    if app.mode == AppMode::FileList && app.focused_panel == FocusedPanel::Bookmarks {
        return vec![
            ("↑↓", "이동"),
            ("Enter/→", "폴더이동"),
            ("d", "삭제"),
            ("Tab/Esc", "나가기"),
        ];
    }
    match app.mode {
        AppMode::FileList => vec![
            ("↑↓", "이동"),
            ("Enter", "열기메뉴"),
            ("→/l", "뷰어/진입"),
            ("←", "상위"),
            ("Space", "뷰어"),
            ("b", "즐겨찾기"),
            ("/", "검색"),
            ("q", "종료"),
        ],
        AppMode::OpenWith => vec![
            ("↑↓", "선택"),
            ("Enter", "열기"),
            ("Esc", "취소"),
        ],
        AppMode::Viewer => vec![
            ("q/Esc", "돌아가기"),
            ("↑↓/←→", "스크롤"),
            ("/", "검색"),
            ("n/N", "매칭이동"),
            (":", "줄이동"),
            ("gg/G", "처음/끝"),
            ("w", "줄바꿈"),
        ],
        AppMode::Settings => vec![
            ("Esc", "닫기"),
            ("↑↓", "이동"),
            ("Enter", "변경"),
        ],
        AppMode::CommandPalette => vec![
            ("Esc", "닫기"),
            ("Enter", "실행"),
            ("↑↓", "선택"),
        ],
        AppMode::Help => vec![
            ("Esc/?", "닫기"),
        ],
    }
}

