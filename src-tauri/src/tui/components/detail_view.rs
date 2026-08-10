use crate::domain::SoftwareItem;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub struct DetailView;

impl DetailView {
    pub fn render(f: &mut Frame, item: Option<&SoftwareItem>, area: Rect) {
        let block = Block::default()
            .title(" Item Details ")
            .borders(Borders::ALL);

        match item {
            Some(item) => {
                let details = Self::format_item_details(item);
                let paragraph = Paragraph::new(details)
                    .block(block)
                    .wrap(Wrap { trim: true });
                f.render_widget(paragraph, area);
            }
            None => {
                let text = "No item selected\n\nPress Enter on an item to view details";
                let paragraph = Paragraph::new(text)
                    .block(block)
                    .style(Style::default().fg(Color::DarkGray));
                f.render_widget(paragraph, area);
            }
        }
    }

    fn format_item_details(item: &SoftwareItem) -> String {
        let mut details = String::new();

        // Basic information
        details.push_str(&format!("Display Name: {}\n", item.display_name));
        details.push_str(&format!("Package Name: {}\n", item.package_name));
        details.push_str(&format!("ID: {}\n", item.id));

        // Description and version
        if let Some(desc) = &item.description {
            details.push_str(&format!("Description: {}\n", desc));
        }
        if let Some(ver) = &item.version {
            details.push_str(&format!("Version: {}\n", ver));
        }

        details.push('\n');

        // Classification
        details.push_str(&format!("Category: {:?}\n", item.category));
        if !item.secondary_categories.is_empty() {
            let secondary = item
                .secondary_categories
                .iter()
                .map(|c| format!("{:?}", c))
                .collect::<Vec<_>>()
                .join(", ");
            details.push_str(&format!("Secondary Categories: {}\n", secondary));
        }
        details.push_str(&format!(
            "Classification Confidence: {:?}\n",
            item.classification_confidence
        ));

        details.push('\n');

        // Installation information
        details.push_str(&format!("Package Manager: {:?}\n", item.package_manager));
        details.push_str(&format!("Installation Scope: {:?}\n", item.scope));
        details.push_str(&format!(
            "Installation Reason: {:?}\n",
            item.installation_reason
        ));

        if let Some(source) = &item.package_source {
            details.push_str(&format!("Package Source: {}\n", source));
        }

        details.push('\n');

        // Dependencies
        if !item.dependencies.is_empty() {
            details.push_str("Dependencies:\n");
            for dep in item.dependencies.iter().take(5) {
                details.push_str(&format!("  • {}\n", dep));
            }
            if item.dependencies.len() > 5 {
                details.push_str(&format!("  ... and {} more\n", item.dependencies.len() - 5));
            }
        }

        if !item.reverse_dependencies.is_empty() {
            details.push_str("Required By:\n");
            for dep in item.reverse_dependencies.iter().take(5) {
                details.push_str(&format!("  • {}\n", dep));
            }
            if item.reverse_dependencies.len() > 5 {
                details.push_str(&format!(
                    "  ... and {} more\n",
                    item.reverse_dependencies.len() - 5
                ));
            }
        }

        details.push('\n');

        // Other information
        if let Some(size) = item.installed_size_bytes {
            details.push_str(&format!("Installed Size: {} bytes\n", size));
        }
        if let Some(arch) = &item.architecture {
            details.push_str(&format!("Architecture: {}\n", arch));
        }

        if item.update_available {
            details.push_str("⬆ Update Available");
            if let Some(avail_ver) = &item.available_version {
                details.push_str(&format!(" ({})", avail_ver));
            }
            details.push('\n');
        }

        if let Some(homepage) = &item.homepage {
            details.push_str(&format!("Homepage: {}\n", homepage));
        }
        if let Some(license) = &item.license {
            details.push_str(&format!("License: {}\n", license));
        }

        details
    }
}
