use crate::domain::{
    ClassificationConfidence, InstallationReason, InstallationScope, PackageManager,
    SoftwareCategory,
};
use crate::tui::app::{App, FilterDimension, UpdateFilter};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs};

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
            let cats = categories
                .iter()
                .map(|c| format!("{:?}", c))
                .collect::<Vec<_>>()
                .join(", ");
            filters.push(format!("Categories: {}", cats));
        }

        if !managers.is_empty() {
            let mgrs = managers
                .iter()
                .map(|m| format!("{:?}", m))
                .collect::<Vec<_>>()
                .join(", ");
            filters.push(format!("Managers: {}", mgrs));
        }

        if !scopes.is_empty() {
            let scs = scopes
                .iter()
                .map(|s| format!("{:?}", s))
                .collect::<Vec<_>>()
                .join(", ");
            filters.push(format!("Scopes: {}", scs));
        }

        if !reasons.is_empty() {
            let rsns = reasons
                .iter()
                .map(|r| format!("{:?}", r))
                .collect::<Vec<_>>()
                .join(", ");
            filters.push(format!("Reasons: {}", rsns));
        }

        let filter_text = if filters.is_empty() {
            "No filters active".to_string()
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
            .block(Block::default().title(" Filters ").borders(Borders::ALL));

        f.render_widget(paragraph, area);
    }

    pub fn render_open(f: &mut Frame, app: &App, area: Rect) {
        let dimension_labels = [
            FilterDimension::Category.label(),
            FilterDimension::Manager.label(),
            FilterDimension::Scope.label(),
            FilterDimension::Reason.label(),
            FilterDimension::Confidence.label(),
            FilterDimension::UpdateAvailable.label(),
        ];

        let selected_tab = match app.filter_dimension {
            FilterDimension::Category => 0,
            FilterDimension::Manager => 1,
            FilterDimension::Scope => 2,
            FilterDimension::Reason => 3,
            FilterDimension::Confidence => 4,
            FilterDimension::UpdateAvailable => 5,
        };

        if area.height < 5 {
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(3)])
            .split(area);

        let tabs = Tabs::new(dimension_labels)
            .select(selected_tab)
            .block(Block::default().borders(Borders::BOTTOM))
            .divider("");

        f.render_widget(tabs, chunks[0]);

        let items = Self::get_dimension_items(app);
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(app.filter_cursor));

        f.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    fn get_dimension_items(app: &App) -> Vec<ListItem<'_>> {
        match app.filter_dimension {
            FilterDimension::Category => SoftwareCategory::ALL
                .iter()
                .map(|c| {
                    let is_checked = app.category_filters.contains(c);
                    let checkbox = if is_checked { "[x]" } else { "[ ]" };
                    ListItem::new(format!("{} {:?}", checkbox, c))
                })
                .collect(),
            FilterDimension::Manager => PackageManager::ALL
                .iter()
                .map(|m| {
                    let is_checked = app.manager_filters.contains(m);
                    let checkbox = if is_checked { "[x]" } else { "[ ]" };
                    ListItem::new(format!("{} {:?}", checkbox, m))
                })
                .collect(),
            FilterDimension::Scope => InstallationScope::ALL
                .iter()
                .map(|s| {
                    let is_checked = app.scope_filters.contains(s);
                    let checkbox = if is_checked { "[x]" } else { "[ ]" };
                    ListItem::new(format!("{} {:?}", checkbox, s))
                })
                .collect(),
            FilterDimension::Reason => InstallationReason::ALL
                .iter()
                .map(|r| {
                    let is_checked = app.reason_filters.contains(r);
                    let checkbox = if is_checked { "[x]" } else { "[ ]" };
                    ListItem::new(format!("{} {:?}", checkbox, r))
                })
                .collect(),
            FilterDimension::Confidence => ClassificationConfidence::ALL
                .iter()
                .map(|c| {
                    let is_checked = app.confidence_filters.contains(c);
                    let checkbox = if is_checked { "[x]" } else { "[ ]" };
                    ListItem::new(format!("{} {:?}", checkbox, c))
                })
                .collect(),
            FilterDimension::UpdateAvailable => {
                let any_checked = app.update_filter == UpdateFilter::Any;
                let available_checked = app.update_filter == UpdateFilter::Available;
                let not_available_checked = app.update_filter == UpdateFilter::NotAvailable;

                vec![
                    ListItem::new(format!("{} Any", if any_checked { "(•)" } else { "( )" })),
                    ListItem::new(format!(
                        "{} Available",
                        if available_checked { "(•)" } else { "( )" }
                    )),
                    ListItem::new(format!(
                        "{} Not Available",
                        if not_available_checked {
                            "(•)"
                        } else {
                            "( )"
                        }
                    )),
                ]
            }
        }
    }
}
