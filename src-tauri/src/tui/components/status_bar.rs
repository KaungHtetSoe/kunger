use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use crate::tui::app::App;

pub struct StatusBar;

impl StatusBar {
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let page_info = if app.total_pages() > 0 {
            format!(
                "Page {}/{} | {} items | Selection: {}/{}",
                app.current_page + 1,
                app.total_pages(),
                app.item_count(),
                app.selected_index + 1,
                app.item_count()
            )
        } else {
            "No items to display".to_string()
        };

        let help_text = " ↑/↓: Navigate | PgUp/PgDn: Page | q: Quit";

        let status = Paragraph::new(format!("{:<60} {}", page_info, help_text))
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));

        f.render_widget(status, area);
    }
}
