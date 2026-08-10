use crate::tui::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table};

pub struct TableWidget;

impl TableWidget {
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let items = app.visible_items();

        if items.is_empty() {
            let message = if app.all_items.is_empty() {
                "No inventory available yet. Press F5 to scan."
            } else {
                "No items match the current search or filters."
            };
            let empty_state = Paragraph::new(message)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .title(" Software Items ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                );
            f.render_widget(empty_state, area);
            return;
        }

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

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(40),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(" Software Items ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );

        f.render_widget(table, area);
    }
}
