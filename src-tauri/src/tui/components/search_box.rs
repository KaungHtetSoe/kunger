use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub struct SearchBox;

impl SearchBox {
    pub fn render(f: &mut Frame, search_query: &str, cursor_position: usize, area: Rect) {
        let display_query = if search_query.is_empty() {
            "Type to search...".to_string()
        } else {
            search_query.to_string()
        };

        let style = if search_query.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        let paragraph = Paragraph::new(display_query)
            .style(style)
            .block(Block::default().title(" Search ").borders(Borders::ALL));

        f.render_widget(paragraph, area);

        if !search_query.is_empty() && cursor_position <= search_query.len() {
            f.set_cursor_position((area.x + cursor_position as u16 + 1, area.y + 1));
        }
    }
}
