use ratatui::prelude::*;
use crate::tui::app::App;
use crate::tui::components::{TableWidget, StatusBar, SearchBox, FilterPanel, DetailView, ScanProgress};

pub struct UI;

impl UI {
    pub fn render(f: &mut Frame, app: &App) {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Search box
                Constraint::Length(3),  // Filter panel
                Constraint::Min(5),     // Table/Detail pane
                Constraint::Length(1),  // Status bar
            ])
            .split(f.area());

        SearchBox::render(f, &app.search_query, app.cursor_position, vertical_chunks[0]);
        FilterPanel::render(
            f,
            &app.category_filters,
            &app.manager_filters,
            &app.scope_filters,
            &app.reason_filters,
            vertical_chunks[1],
        );

        // Show split layout if detail view is visible
        if app.detail_view_visible {
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(vertical_chunks[2]);

            TableWidget::render(f, app, horizontal_chunks[0]);
            let selected = app.selected_item();
            DetailView::render(f, selected, horizontal_chunks[1]);
        } else {
            TableWidget::render(f, app, vertical_chunks[2]);
        }

        // Show scan progress if scanning
        if app.is_scanning {
            let scan_area = Rect {
                x: vertical_chunks[2].x + 2,
                y: vertical_chunks[2].y + vertical_chunks[2].height / 2,
                width: vertical_chunks[2].width.saturating_sub(4),
                height: 3,
            };
            ScanProgress::render(f, true, app.scan_progress, scan_area);
        }

        StatusBar::render(f, app, vertical_chunks[3]);
    }
}
