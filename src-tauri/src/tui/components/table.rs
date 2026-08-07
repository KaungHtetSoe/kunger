use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Table, Row, Cell, BorderType};
use crate::tui::app::App;

pub struct TableWidget;

impl TableWidget {
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let items = app.visible_items();
        let header = Row::new(vec!["Name", "Category", "Manager", "Version"])
            .style(Style::default().bold().fg(Color::Cyan))
            .bottom_margin(1);

        let rows = items.iter().enumerate().map(|(idx, item)| {
            let category = format!("{:?}", item.category);
            let manager = format!("{:?}", item.package_manager);
            let version = item.version.as_ref().map(|v| v.as_str()).unwrap_or("—");

            let cells = vec![
                Cell::from(item.display_name.as_str()),
                Cell::from(category),
                Cell::from(manager),
                Cell::from(version),
            ];

            let is_selected = app.current_page * app.page_size + idx == app.selected_index;
            if is_selected {
                Row::new(cells).style(Style::default().bg(Color::DarkGray).fg(Color::White))
            } else {
                Row::new(cells)
            }
        });

        let table = Table::new(rows, [
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .header(header)
        .block(Block::default()
            .title(" Software Items ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded));

        f.render_widget(table, area);
    }
}
