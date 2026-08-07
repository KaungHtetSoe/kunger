use crossterm::event::{Event, KeyCode, KeyEvent};
use crate::tui::app::App;

pub struct InputHandler;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SelectNext,
    SelectPrevious,
    PageDown,
    PageUp,
    Quit,
    None,
}

impl InputHandler {
    pub fn handle_event(app: &mut App, event: Event) -> Action {
        match event {
            Event::Key(key) => Self::handle_key(app, key),
            _ => Action::None,
        }
    }

    fn handle_key(app: &mut App, key: KeyEvent) -> Action {
        // Handle search input when focused
        if app.search_focused {
            return Self::handle_search_input(app, key);
        }

        // Handle detail view when visible
        if app.detail_view_visible {
            match key.code {
                KeyCode::Esc => {
                    app.close_detail_view();
                    return Action::None;
                }
                _ => return Action::None,
            }
        }

        // Navigation and global keybindings
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Up => {
                app.select_previous();
                Action::None
            }
            KeyCode::Down => {
                app.select_next();
                Action::None
            }
            KeyCode::PageUp => {
                app.page_up();
                Action::None
            }
            KeyCode::PageDown => {
                app.page_down();
                Action::None
            }
            KeyCode::Tab => {
                app.search_focused = true;
                Action::None
            }
            KeyCode::Enter => {
                // Open detail view for selected item
                app.toggle_detail_view();
                Action::None
            }
            KeyCode::Char('c') => {
                // Clear all filters
                app.clear_all_filters();
                Action::None
            }
            KeyCode::Char('s') => {
                // Toggle sort order
                app.toggle_sort_order();
                Action::None
            }
            KeyCode::F(5) => {
                // F5 to trigger scan
                app.start_scan();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_search_input(app: &mut App, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                app.search_focused = false;
                Action::None
            }
            KeyCode::Enter => {
                app.search_focused = false;
                Action::None
            }
            KeyCode::Backspace => {
                app.backspace_search();
                Action::None
            }
            KeyCode::Left => {
                if app.cursor_position > 0 {
                    app.cursor_position -= 1;
                }
                Action::None
            }
            KeyCode::Right => {
                if app.cursor_position < app.search_query.len() {
                    app.cursor_position += 1;
                }
                Action::None
            }
            KeyCode::Home => {
                app.cursor_position = 0;
                Action::None
            }
            KeyCode::End => {
                app.cursor_position = app.search_query.len();
                Action::None
            }
            KeyCode::Char(c) => {
                app.add_search_char(c);
                Action::None
            }
            _ => Action::None,
        }
    }
}
