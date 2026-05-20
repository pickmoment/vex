use std::time::{Duration, Instant};

#[derive(Clone)]
pub enum StatusKind {
    Success,
    Error,
    Info,
}

#[derive(Clone)]
pub struct StatusMessage {
    pub text: String,
    pub kind: StatusKind,
    pub expires_at: Instant,
}

impl StatusMessage {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: StatusKind::Success,
            expires_at: Instant::now() + Duration::from_secs(3),
        }
    }
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: StatusKind::Error,
            expires_at: Instant::now() + Duration::from_secs(5),
        }
    }
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

pub fn render(f: &mut ratatui::Frame, area: ratatui::layout::Rect, msg: &StatusMessage) {
    use ratatui::{
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::Paragraph,
    };
    let (icon, color) = match msg.kind {
        StatusKind::Success => ("✓", Color::Green),
        StatusKind::Error => ("✗", Color::Red),
        StatusKind::Info => ("ℹ", Color::Cyan),
    };
    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", icon),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(msg.text.clone(), Style::default().fg(Color::White)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}
