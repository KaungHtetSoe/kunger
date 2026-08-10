use crate::tui::app::App;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

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

        let status_text = if app.is_scanning {
            "Scanning inventory... Esc: Cancel | q: Quit".to_string()
        } else if app.filter_panel_visible {
            "Filters: ◄► Dim | ▲▼ Val | Space: Toggle | c: Clear | Esc: Close".to_string()
        } else {
            format!(
                "{} | Tab: Search | ↑↓: Nav | Enter: Detail | f: Filters | F5: Scan | q: Quit",
                app.scan_message.clone().unwrap_or(page_info)
            )
        };

        let status = Paragraph::new(status_text)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));

        f.render_widget(status, area);
    }
}
