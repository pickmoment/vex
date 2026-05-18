use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Paragraph},
    Frame,
};
use std::path::PathBuf;

/// 이미지 렌더링 프로토콜
#[derive(Debug, Clone, PartialEq)]
pub enum ImageProtocol {
    Kitty,
    ITerm2,
    Sixel,
    Braille,
    Unsupported,
}

/// 터미널 이미지 렌더링 프로토콜 자동 감지
pub fn detect_protocol() -> ImageProtocol {
    // TERM_PROGRAM 환경변수로 감지
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    let term = std::env::var("TERM").unwrap_or_default();

    if term_program.contains("iTerm") {
        return ImageProtocol::ITerm2;
    }
    if term == "xterm-kitty" || std::env::var("KITTY_WINDOW_ID").is_ok() {
        return ImageProtocol::Kitty;
    }
    // Sixel: WezTerm, xterm with sixel
    if term_program.contains("WezTerm") || term.contains("sixel") {
        return ImageProtocol::Sixel;
    }
    // 폴백: Braille 픽셀
    ImageProtocol::Braille
}

/// 이미지 파일 렌더링 (프로토콜 자동 선택)
pub fn render(f: &mut Frame, area: Rect, path: &PathBuf, block: Block) {
    let protocol = detect_protocol();
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

    match protocol {
        ImageProtocol::Braille => {
            render_braille_preview(f, area, path, block);
        }
        ImageProtocol::Kitty => {
            // Kitty 프로토콜: ratatui-image 크레이트 활용 (향후 통합)
            render_protocol_placeholder(f, area, file_name, "Kitty", block);
        }
        ImageProtocol::ITerm2 => {
            render_protocol_placeholder(f, area, file_name, "iTerm2", block);
        }
        ImageProtocol::Sixel => {
            render_protocol_placeholder(f, area, file_name, "Sixel", block);
        }
        ImageProtocol::Unsupported => {
            render_unsupported(f, area, file_name, block);
        }
    }
}

/// Braille 패턴을 이용한 픽셀 아트 렌더링
pub fn render_braille_preview(f: &mut Frame, area: Rect, path: &PathBuf, block: Block) {
    use image::imageops::FilterType;
    use image::GenericImageView;

    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

    let img = match image::open(path) {
        Ok(i) => i,
        Err(e) => {
            let para = Paragraph::new(vec![
                Line::from(""),
                Line::from(format!("  이미지 로드 실패: {e}")),
                Line::from(format!("  파일: {file_name}")),
            ])
            .block(block);
            f.render_widget(para, area);
            return;
        }
    };

    // 터미널 크기에 맞게 리사이즈 (Braille: 2x4 픽셀 = 1 문자)
    let inner = area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });
    let char_w = inner.width as u32;
    let char_h = inner.height as u32;
    let px_w = char_w * 2;
    let px_h = char_h * 4;

    let resized = img.resize(px_w, px_h, FilterType::Lanczos3);
    let (rw, rh) = resized.dimensions();
    let gray = resized.to_luma8();

    let mut lines: Vec<Line> = Vec::new();

    // 4행 2열 픽셀 블록을 하나의 Braille 문자로 변환
    let mut y = 0u32;
    while y + 4 <= rh {
        let mut spans_on_line = vec![];
        let mut x = 0u32;
        while x + 2 <= rw {
            let braille = pixels_to_braille(&gray, x, y);
            spans_on_line.push(ratatui::text::Span::raw(braille.to_string()));
            x += 2;
        }
        lines.push(Line::from(spans_on_line));
        y += 4;
    }

    // 메타 정보
    let (orig_w, orig_h) = img.dimensions();
    lines.push(Line::from(""));
    lines.push(Line::from(ratatui::text::Span::styled(
        format!("  {file_name}  ({orig_w}×{orig_h})  [Braille 렌더링]"),
        Style::default().fg(Color::DarkGray),
    )));

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, area);
}

/// 2×4 픽셀 블록을 Braille 유니코드 문자로 변환
fn pixels_to_braille(gray: &image::GrayImage, x: u32, y: u32) -> char {
    // Braille 패턴: ⠀ (U+2800) 기준
    // 비트 레이아웃:
    //  픽셀 위치  비트
    //  (0,0)→1   (1,0)→8
    //  (0,1)→2   (1,1)→16
    //  (0,2)→4   (1,2)→32
    //  (0,3)→64  (1,3)→128
    let threshold = 128u8;
    let mut bits = 0u32;

    let bit_map = [
        (0, 0, 0x01), (0, 1, 0x02), (0, 2, 0x04), (0, 3, 0x40),
        (1, 0, 0x08), (1, 1, 0x10), (1, 2, 0x20), (1, 3, 0x80),
    ];

    for (dx, dy, bit) in &bit_map {
        let px = x + dx;
        let py = y + dy;
        if px < gray.width() && py < gray.height() {
            if gray.get_pixel(px, py).0[0] < threshold {
                bits |= bit;
            }
        }
    }

    char::from_u32(0x2800 + bits).unwrap_or('?')
}

/// 프로토콜 지원 플레이스홀더 (실제 렌더링 전 상태)
fn render_protocol_placeholder(
    f: &mut Frame,
    area: Rect,
    file_name: &str,
    protocol: &str,
    block: Block,
) {
    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(format!("  이미지: {file_name}")),
        Line::from(""),
        Line::from(format!("  렌더링 프로토콜: {protocol}")),
        Line::from("  (ratatui-image 통합으로 렌더링 예정)"),
    ])
    .block(block);
    f.render_widget(para, area);
}

/// 미지원 환경 안내
fn render_unsupported(f: &mut Frame, area: Rect, file_name: &str, block: Block) {
    let para = Paragraph::new(vec![
        Line::from(""),
        Line::from(format!("  이미지: {file_name}")),
        Line::from(""),
        Line::from("  이미지 렌더링이 지원되지 않는 터미널입니다."),
        Line::from("  Kitty / WezTerm / iTerm2 권장"),
    ])
    .block(block);
    f.render_widget(para, area);
}
