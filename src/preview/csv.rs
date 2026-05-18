use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Cell, Row, Table},
    Frame,
};
use std::path::PathBuf;

/// CSV 파일을 테이블 위젯으로 렌더링
pub fn render(f: &mut Frame, area: Rect, path: &PathBuf, block: Block) {
    match parse_csv(path) {
        Ok((headers, rows)) => {
            render_table(f, area, headers, rows, block);
        }
        Err(e) => {
            use ratatui::widgets::Paragraph;
            let para = Paragraph::new(format!("[CSV 파싱 오류: {e}]")).block(block);
            f.render_widget(para, area);
        }
    }
}

/// CSV 파일 파싱 → (헤더, 행 목록)
fn parse_csv(path: &PathBuf) -> Result<(Vec<String>, Vec<Vec<String>>), csv::Error> {
    let mut reader = csv::Reader::from_path(path)?;

    // 헤더 행
    let headers: Vec<String> = reader
        .headers()?
        .iter()
        .map(|s| s.to_string())
        .collect();

    // 데이터 행
    let rows: Vec<Vec<String>> = reader
        .records()
        .filter_map(|r| r.ok())
        .map(|record| record.iter().map(|s| s.to_string()).collect())
        .collect();

    Ok((headers, rows))
}

/// 파싱된 CSV 데이터를 ratatui Table 위젯으로 렌더링
fn render_table(
    f: &mut Frame,
    area: Rect,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    block: Block,
) {
    let header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let row_style = Style::default().fg(Color::White);
    let alt_row_style = Style::default().fg(Color::Gray);

    // 헤더 셀
    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::from(h.as_str()).style(header_style))
        .collect();
    let header_row = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

    // 데이터 행 (짝수/홀수 배경 교대)
    let data_rows: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(i, row_data)| {
            let style = if i % 2 == 0 { row_style } else { alt_row_style };
            let cells: Vec<Cell> = row_data
                .iter()
                .map(|cell| Cell::from(cell.as_str()).style(style))
                .collect();
            Row::new(cells).height(1)
        })
        .collect();

    // 컬럼 너비 자동 계산 (최소 6, 최대 20)
    let col_count = headers.len().max(1);
    let widths: Vec<ratatui::layout::Constraint> = (0..col_count)
        .map(|i| {
            let max_width = rows
                .iter()
                .filter_map(|r| r.get(i))
                .map(|s| s.len())
                .max()
                .unwrap_or(0)
                .max(headers.get(i).map(|h| h.len()).unwrap_or(0))
                .clamp(6, 20);
            ratatui::layout::Constraint::Length(max_width as u16 + 2)
        })
        .collect();

    let table = Table::new(
        std::iter::once(header_row).chain(data_rows),
        widths,
    )
    .block(block)
    .column_spacing(1);

    f.render_widget(table, area);
}

/// TSV 파일 파싱 (탭 구분자)
pub fn parse_tsv(path: &PathBuf) -> Result<(Vec<String>, Vec<Vec<String>>), csv::Error> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_path(path)?;

    let headers: Vec<String> = reader
        .headers()?
        .iter()
        .map(|s| s.to_string())
        .collect();

    let rows: Vec<Vec<String>> = reader
        .records()
        .filter_map(|r| r.ok())
        .map(|record| record.iter().map(|s| s.to_string()).collect())
        .collect();

    Ok((headers, rows))
}
