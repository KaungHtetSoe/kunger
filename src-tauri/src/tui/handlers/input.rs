use crate::tui::app::App;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

pub struct InputHandler;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SelectNext,
    SelectPrevious,
    PageDown,
    PageUp,
    StartScan,
    CancelScan,
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
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Action::Quit;
        }

        if app.is_scanning {
            return match key.code {
                KeyCode::Esc => Action::CancelScan,
                KeyCode::Char('q') => Action::Quit,
                _ => Action::None,
            };
        }

        // Handle filter panel when visible
        if app.filter_panel_visible {
            return match key.code {
                KeyCode::Esc => {
                    app.close_filter_panel();
                    Action::None
                }
                KeyCode::Left => {
                    app.filter_dimension_prev();
                    Action::None
                }
                KeyCode::Right => {
                    app.filter_dimension_next();
                    Action::None
                }
                KeyCode::Up => {
                    app.filter_cursor_prev();
                    Action::None
                }
                KeyCode::Down => {
                    app.filter_cursor_next();
                    Action::None
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    app.toggle_current_filter_value();
                    Action::None
                }
                KeyCode::Char('c') => {
                    app.clear_all_filters();
                    Action::None
                }
                _ => Action::None,
            };
        }

        if key.code == KeyCode::F(5) {
            return Action::StartScan;
        }

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
            KeyCode::Char('f') => {
                // Open filter panel
                app.open_filter_panel();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SoftwareItem;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    #[test]
    fn f5_requests_a_scan_when_idle() {
        let mut app = App::new(vec![]);

        assert_eq!(
            InputHandler::handle_event(&mut app, key(KeyCode::F(5), KeyModifiers::NONE)),
            Action::StartScan
        );
        assert!(
            !app.is_scanning,
            "the CLI starts the worker after handling the action"
        );
    }

    #[test]
    fn escape_cancels_an_active_scan() {
        let mut app = App::new(vec![]);
        app.start_scan();

        assert_eq!(
            InputHandler::handle_event(&mut app, key(KeyCode::Esc, KeyModifiers::NONE)),
            Action::CancelScan
        );
    }

    #[test]
    fn control_c_quits_without_clearing_filters() {
        let mut app = App::new(vec![]);

        assert_eq!(
            InputHandler::handle_event(&mut app, key(KeyCode::Char('c'), KeyModifiers::CONTROL),),
            Action::Quit
        );
    }

    #[test]
    fn f_opens_filter_panel() {
        let mut app = App::new(vec![]);
        assert!(!app.filter_panel_visible);

        InputHandler::handle_event(&mut app, key(KeyCode::Char('f'), KeyModifiers::NONE));
        assert!(app.filter_panel_visible);
    }

    #[test]
    fn escape_closes_filter_panel() {
        let mut app = App::new(vec![]);
        app.open_filter_panel();
        assert!(app.filter_panel_visible);

        InputHandler::handle_event(&mut app, key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.filter_panel_visible);
    }

    #[test]
    fn left_right_switch_filter_dimension() {
        let mut app = App::new(vec![]);
        app.open_filter_panel();

        let initial_dimension = app.filter_dimension;
        InputHandler::handle_event(&mut app, key(KeyCode::Right, KeyModifiers::NONE));
        assert_ne!(app.filter_dimension, initial_dimension);
        assert_eq!(app.filter_cursor, 0);

        InputHandler::handle_event(&mut app, key(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.filter_dimension, initial_dimension);
        assert_eq!(app.filter_cursor, 0);
    }

    #[test]
    fn space_toggles_highlighted_value() {
        use crate::domain::{PackageManager, SoftwareCategory};

        let mut item = SoftwareItem::new("Test", "test", "Test", PackageManager::Unknown);
        item.category = SoftwareCategory::Application;
        let mut app = App::new(vec![item]);
        app.open_filter_panel();

        assert_eq!(app.category_filters.len(), 0);
        InputHandler::handle_event(&mut app, key(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.category_filters.len(), 1);
    }
}
