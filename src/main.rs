use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

mod app;
mod config;
mod fs;
mod git;
mod preview;
mod state;
mod ui;

use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // 로거 초기화
    env_logger::init();

    // 터미널 초기화
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 앱 생성 및 실행
    let mut app = App::new()?;
    let result = app.run(&mut terminal).await;

    // 터미널 복원
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // 에러 출력
    if let Err(err) = result {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}
