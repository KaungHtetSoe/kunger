use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use crate::domain::{SoftwareCategory, PackageManager, InstallationScope, InstallationReason};

pub struct FilterPanel;

impl FilterPanel {
    pub fn render(
        f: &mut Frame,
        categories: &[SoftwareCategory],
        managers: &[PackageManager],
        scopes: &[InstallationScope],
        reasons: &[InstallationReason],
        area: Rect,
    ) {
        let mut filters = vec![];

        if !categories.is_empty() {
            let cats = categories.iter()
                .map(|c| format!("{:?}", c))
                .collect::<Vec<_>>()
                .join(", ");
            filters.push(format!("Categories: {}", cats));
        }

        if !managers.is_empty() {
            let mgrs = managers.iter()
                .map(|m| format!("{:?}", m))
                .collect::<Vec<_>>()
                .join(", ");
            filters.push(format!("Managers: {}", mgrs));
        }

        if !scopes.is_empty() {
            let scs = scopes.iter()
                .map(|s| format!("{:?}", s))
                .collect::<Vec<_>>()
                .join(", ");
            filters.push(format!("Scopes: {}", scs));
        }

        if !reasons.is_empty() {
            let rsns = reasons.iter()
                .map(|r| format!("{:?}", r))
                .collect::<Vec<_>>()
                .join(", ");
            filters.push(format!("Reasons: {}", rsns));
        }

        let filter_text = if filters.is_empty() {
            "No filters active (press F to open filter menu)".to_string()
        } else {
            filters.join(" | ")
        };

        let style = if filters.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let paragraph = Paragraph::new(filter_text)
            .style(style)
            .block(Block::default()
                .title(" Filters ")
                .borders(Borders::ALL));

        f.render_widget(paragraph, area);
    }
}
