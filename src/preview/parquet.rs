use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use std::path::PathBuf;

const MAX_ROWS: usize = 200;

pub fn render(f: &mut Frame, area: Rect, path: &PathBuf, block: Block, scroll: u16) {
    match parse_parquet(path) {
        Ok((headers, rows, total_rows)) => {
            render_table(f, area, headers, rows, total_rows, block, scroll);
        }
        Err(e) => {
            let para = Paragraph::new(format!("[Parquet 파싱 오류: {e}]")).block(block);
            f.render_widget(para, area);
        }
    }
}

fn parse_parquet(
    path: &PathBuf,
) -> Result<(Vec<String>, Vec<Vec<String>>, i64), Box<dyn std::error::Error>> {
    use parquet::file::reader::{FileReader, SerializedFileReader};
    use std::fs::File;

    let file = File::open(path)?;
    let reader = SerializedFileReader::new(file)?;

    let file_meta = reader.metadata().file_metadata();
    let total_rows = file_meta.num_rows();

    let headers: Vec<String> = file_meta
        .schema_descr()
        .columns()
        .iter()
        .map(|col| col.name().to_string())
        .collect();

    let rows: Vec<Vec<String>> = reader
        .get_row_iter(None)?
        .take(MAX_ROWS)
        .filter_map(|r| r.ok())
        .map(|row| {
            row.get_column_iter()
                .map(|(_, field)| {
                    let s = format!("{field}");
                    if s.chars().count() > 25 {
                        let truncated: String = s.chars().take(22).collect();
                        format!("{truncated}…")
                    } else {
                        s
                    }
                })
                .collect()
        })
        .collect();

    Ok((headers, rows, total_rows))
}

fn render_table(
    f: &mut Frame,
    area: Rect,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    total_rows: i64,
    block: Block,
    scroll: u16,
) {
    let row_count = rows.len();
    let title = if total_rows as usize > row_count {
        format!(" (처음 {row_count}/{total_rows} 행) ")
    } else {
        format!(" ({total_rows} 행) ")
    };
    let block = block.title(title);

    let header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let row_style = Style::default().fg(Color::White);
    let alt_row_style = Style::default().fg(Color::Gray);

    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::from(h.as_str()).style(header_style))
        .collect();
    let header_row = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

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

    let col_count = headers.len().max(1);
    let widths: Vec<ratatui::layout::Constraint> = (0..col_count)
        .map(|i| {
            let max_width = rows
                .iter()
                .filter_map(|r| r.get(i))
                .map(|s| s.chars().count())
                .max()
                .unwrap_or(0)
                .max(headers.get(i).map(|h| h.chars().count()).unwrap_or(0))
                .clamp(6, 20);
            ratatui::layout::Constraint::Length(max_width as u16 + 2)
        })
        .collect();

    let table = Table::new(data_rows, widths)
        .header(header_row)
        .block(block)
        .column_spacing(1);

    let mut state = TableState::default().with_offset(scroll as usize);
    f.render_stateful_widget(table, area, &mut state);
}
