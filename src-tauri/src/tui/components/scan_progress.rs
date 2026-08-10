use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};

pub struct ScanProgress;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Scanning,
    Complete,
    Failed,
}

impl ScanProgress {
    pub fn render(f: &mut Frame, is_scanning: bool, progress: u16, area: Rect) {
        if !is_scanning {
            return;
        }

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(" Scanning... ")
                    .borders(Borders::ALL),
            )
            .gauge_style(Style::default().fg(Color::Cyan))
            .ratio(progress as f64 / 100.0)
            .label("Scanning inventory... Esc to cancel");

        f.render_widget(gauge, area);
    }

    pub fn render_centered_message(f: &mut Frame, message: &str, area: Rect) {
        let paragraph = Paragraph::new(message)
            .block(Block::default().title(" Scanner ").borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }
}
