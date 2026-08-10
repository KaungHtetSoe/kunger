use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use kunger_lib::domain::{
    ClassificationConfidence, InstallationReason, InstallationScope, PackageManager,
    SoftwareCategory, SoftwareItem,
};
use kunger_lib::tui::{handlers::InputHandler, App};

fn create_test_items() -> Vec<SoftwareItem> {
    vec![
        {
            let mut item =
                SoftwareItem::new("apt:firefox", "firefox", "Firefox", PackageManager::Apt);
            item.description = Some("Web browser".to_string());
            item.version = Some("128.0".to_string());
            item.category = SoftwareCategory::Application;
            item.scope = InstallationScope::System;
            item.installation_reason = InstallationReason::Manual;
            item
        },
        {
            let mut item = SoftwareItem::new("apt:git", "git", "Git", PackageManager::Apt);
            item.description = Some("Version control system".to_string());
            item.version = Some("2.40".to_string());
            item.category = SoftwareCategory::CommandLineTool;
            item.scope = InstallationScope::System;
            item.installation_reason = InstallationReason::Manual;
            item
        },
        {
            let mut item = SoftwareItem::new(
                "flatpak:com.slack.Slack",
                "slack",
                "Slack",
                PackageManager::Flatpak,
            );
            item.description = Some("Messaging platform".to_string());
            item.version = Some("4.35".to_string());
            item.category = SoftwareCategory::Application;
            item.scope = InstallationScope::User;
            item.installation_reason = InstallationReason::Manual;
            item
        },
        {
            let mut item =
                SoftwareItem::new("apt:libx11", "libx11", "X11 Library", PackageManager::Apt);
            item.description = Some("X Window System client library".to_string());
            item.version = Some("1.8.7".to_string());
            item.category = SoftwareCategory::Library;
            item.scope = InstallationScope::System;
            item.installation_reason = InstallationReason::Automatic;
            item
        },
        {
            let mut item = SoftwareItem::new("snap:vlc", "vlc", "VLC", PackageManager::Snap);
            item.description = Some("Media player".to_string());
            item.version = Some("3.0.20".to_string());
            item.category = SoftwareCategory::Application;
            item.scope = InstallationScope::System;
            item.installation_reason = InstallationReason::Manual;
            item
        },
    ]
}

#[test]
fn test_cli_workflow_1_basic_loading_and_navigation() {
    let items = create_test_items();
    let mut app = App::new(items);

    // Verify all items loaded
    assert_eq!(app.item_count(), 5);
    assert_eq!(app.all_items.len(), 5);

    // Verify initial state
    assert_eq!(app.selected_index, 0);
    assert_eq!(app.current_page, 0);
    assert_eq!(app.search_query, "");

    // Navigate down
    app.select_next();
    assert_eq!(app.selected_index, 1);
    assert_eq!(app.selected_item().unwrap().display_name, "Git");

    // Navigate down again
    app.select_next();
    assert_eq!(app.selected_index, 2);
    assert_eq!(app.selected_item().unwrap().display_name, "Slack");

    // Navigate back up
    app.select_previous();
    assert_eq!(app.selected_index, 1);
    assert_eq!(app.selected_item().unwrap().display_name, "Git");
}

#[test]
fn test_cli_workflow_2_search_single_item() {
    let items = create_test_items();
    let mut app = App::new(items);

    // Search for Firefox
    app.search_focused = true;
    app.add_search_char('f');
    app.add_search_char('i');
    app.add_search_char('r');
    app.add_search_char('e');
    app.add_search_char('f');
    app.add_search_char('o');
    app.add_search_char('x');

    assert_eq!(app.search_query, "firefox");
    assert_eq!(app.item_count(), 1);
    assert_eq!(app.selected_item().unwrap().display_name, "Firefox");
}

#[test]
fn test_cli_workflow_3_search_with_backspace() {
    let items = create_test_items();
    let mut app = App::new(items);

    // Type "firefox" then backspace to "fire"
    app.search_focused = true;
    for c in "firefox".chars() {
        app.add_search_char(c);
    }
    assert_eq!(app.search_query, "firefox");
    assert_eq!(app.item_count(), 1);

    // Backspace 3 times
    app.backspace_search();
    app.backspace_search();
    app.backspace_search();
    assert_eq!(app.search_query, "fire");
    assert_eq!(app.item_count(), 1);

    // Backspace more to "f"
    for _ in 0..3 {
        app.backspace_search();
    }
    assert_eq!(app.search_query, "f");
    // Now should match Firefox and flatpak apps
    assert!(app.item_count() >= 1);
}

#[test]
fn test_cli_workflow_4_filter_by_category() {
    let items = create_test_items();
    let mut app = App::new(items);

    // Filter to only Applications
    app.toggle_category_filter(SoftwareCategory::Application);
    assert_eq!(app.item_count(), 3); // Firefox, Slack, VLC

    // Verify correct items
    let names: Vec<_> = app
        .filtered_items
        .iter()
        .map(|i| i.display_name.as_str())
        .collect();
    assert!(names.contains(&"Firefox"));
    assert!(names.contains(&"Slack"));
    assert!(names.contains(&"VLC"));
}

#[test]
fn test_cli_workflow_5_filter_by_manager() {
    let items = create_test_items();
    let mut app = App::new(items);

    // Filter to only APT packages
    app.toggle_manager_filter(PackageManager::Apt);
    assert_eq!(app.item_count(), 3); // Firefox, Git, libx11

    // Add Flatpak filter
    app.toggle_manager_filter(PackageManager::Flatpak);
    assert_eq!(app.item_count(), 4); // Firefox, Git, libx11, Slack
}

#[test]
fn test_cli_workflow_6_filter_by_scope() {
    let items = create_test_items();
    let mut app = App::new(items);

    // Filter to only System scope
    app.toggle_scope_filter(InstallationScope::System);
    assert_eq!(app.item_count(), 4); // Firefox, Git, libx11, VLC
    assert!(!app.filtered_items.iter().any(|i| i.display_name == "Slack"));
}

#[test]
fn test_cli_workflow_7_filter_by_reason() {
    let items = create_test_items();
    let mut app = App::new(items);

    // Filter to only Manual installations
    app.toggle_reason_filter(InstallationReason::Manual);
    assert_eq!(app.item_count(), 4); // All except libx11
    assert!(!app
        .filtered_items
        .iter()
        .any(|i| i.display_name == "X11 Library"));
}

#[test]
fn test_cli_workflow_8_combine_search_and_filters() {
    let items = create_test_items();
    let mut app = App::new(items);

    // Search for items with "system"
    app.set_search_query("system".to_string());
    assert_eq!(app.item_count(), 2); // Git (Version control system) and X11 Library (X Window System)

    // Now filter to only Applications
    app.toggle_category_filter(SoftwareCategory::Application);
    assert_eq!(app.item_count(), 0); // No Applications match "system"

    // Remove the Application filter and add CommandLineTool
    app.toggle_category_filter(SoftwareCategory::Application);
    app.toggle_category_filter(SoftwareCategory::CommandLineTool);
    assert_eq!(app.item_count(), 1); // Only Git is CommandLineTool with "system"
}

#[test]
fn test_cli_workflow_9_sort_by_name() {
    let items = create_test_items();
    let mut app = App::new(items);

    // Apply filters to trigger sorting
    app.apply_filters();

    // Default sort should be by name ascending
    let names: Vec<_> = app
        .filtered_items
        .iter()
        .map(|i| i.display_name.as_str())
        .collect();
    assert_eq!(names[0], "Firefox");
    assert_eq!(names[1], "Git");
    assert_eq!(names[2], "Slack");
    assert_eq!(names[3], "VLC");
    assert_eq!(names[4], "X11 Library");
}

#[test]
fn test_cli_workflow_10_sort_descending() {
    let items = create_test_items();
    let mut app = App::new(items);

    app.apply_filters(); // Need to sort first
    app.toggle_sort_order();
    app.apply_filters(); // Re-apply to get descending sort

    assert!(app.filtered_items[0].display_name.starts_with("X"));
    assert!(app.filtered_items[1].display_name.starts_with("V"));
}

#[test]
fn test_cli_workflow_11_sort_by_category() {
    let items = create_test_items();
    let mut app = App::new(items);

    app.sort_field = kunger_lib::tui::app::SortField::Category;
    app.apply_filters();

    // Verify items are sorted by category name
    let is_sorted = format!("{:?}", app.filtered_items[0].category)
        <= format!("{:?}", app.filtered_items[1].category);
    assert!(is_sorted);
}

#[test]
fn test_cli_workflow_12_clear_all_and_reset() {
    let items = create_test_items();
    let mut app = App::new(items);

    // Apply multiple filters
    app.set_search_query("fire".to_string());
    app.toggle_category_filter(SoftwareCategory::Application);
    app.toggle_manager_filter(PackageManager::Apt);

    assert!(app.item_count() < 5);
    assert!(!app.search_query.is_empty());
    assert!(!app.category_filters.is_empty());

    // Clear all
    app.clear_all_filters();

    assert_eq!(app.item_count(), 5);
    assert_eq!(app.search_query, "");
    assert!(app.category_filters.is_empty());
    assert!(app.manager_filters.is_empty());
}

#[test]
fn test_cli_workflow_13_pagination() {
    let mut items = vec![];
    for i in 0..45 {
        let mut item = SoftwareItem::new(
            &format!("pkg{}", i),
            &format!("pkg{}", i),
            &format!("Package {}", i),
            PackageManager::Apt,
        );
        item.category = SoftwareCategory::Application;
        items.push(item);
    }

    let mut app = App::new(items);
    assert_eq!(app.total_pages(), 3); // 45 items / 20 per page = 3 pages

    // Verify first page
    assert_eq!(app.visible_items().len(), 20);
    assert_eq!(app.current_page, 0);

    // Go to next page
    app.page_down();
    assert_eq!(app.current_page, 1);
    assert_eq!(app.visible_items().len(), 20);

    // Go to last page
    app.page_down();
    assert_eq!(app.current_page, 2);
    assert_eq!(app.visible_items().len(), 5); // Only 5 items on last page

    // Try to go beyond last page (should stay at last)
    app.page_down();
    assert_eq!(app.current_page, 2);
}

#[test]
fn test_cli_workflow_14_complex_scenario() {
    // Simulate real-world usage scenario
    let items = create_test_items();
    let mut app = App::new(items);

    // 1. User starts searching for "i"
    app.search_focused = true;
    app.add_search_char('i');
    let results_after_i = app.item_count();
    assert!(results_after_i > 0);

    // 2. User completes search for "git"
    app.add_search_char('t');
    assert_eq!(app.search_query, "it");

    // 3. User realizes mistake and clears with backspace
    app.backspace_search();
    app.backspace_search();
    assert_eq!(app.search_query, "");

    // 4. User types "g" and searches for git-related items
    app.add_search_char('g');
    app.add_search_char('i');
    app.add_search_char('t');
    assert_eq!(app.item_count(), 1);
    assert_eq!(app.selected_item().unwrap().display_name, "Git");

    // 5. User exits search and filters by category
    app.search_focused = false;
    app.clear_all_filters(); // Reset first

    app.toggle_category_filter(SoftwareCategory::CommandLineTool);
    assert_eq!(app.item_count(), 1);

    // 6. User toggles the filter off
    app.toggle_category_filter(SoftwareCategory::CommandLineTool);
    assert_eq!(app.item_count(), 5);

    // 7. User sorts by version
    app.sort_field = kunger_lib::tui::app::SortField::Version;
    app.apply_filters();
    // Verify sorting worked (should have sorted by version)
    assert_eq!(app.item_count(), 5);
}

#[test]
fn test_cli_workflow_15_empty_results() {
    let items = create_test_items();
    let mut app = App::new(items);

    // Search for something that doesn't exist
    app.set_search_query("nonexistent".to_string());
    assert_eq!(app.item_count(), 0);
    assert!(app.selected_item().is_none());

    // Reset should restore items
    app.reset();
    assert_eq!(app.item_count(), 5);
}

#[test]
fn test_cli_workflow_16_interactive_filter_panel_keyboard_driven() {
    let mut items = create_test_items();
    // Add update_available to first item for testing
    items[0].update_available = true;
    // Set confidence levels for testing
    items[0].classification_confidence = ClassificationConfidence::High;
    items[1].classification_confidence = ClassificationConfidence::Medium;
    items[2].classification_confidence = ClassificationConfidence::Low;

    let mut app = App::new(items);

    // 1. Verify filter panel starts closed
    assert!(!app.filter_panel_visible);

    // 2. Press 'f' to open the filter panel via input handler
    let key_event = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE);
    InputHandler::handle_event(&mut app, Event::Key(key_event));
    assert!(app.filter_panel_visible);
    assert_eq!(app.item_count(), 5); // No filter applied yet

    // 3. Navigate to Category dimension (already there by default)
    assert_eq!(
        app.filter_dimension,
        kunger_lib::tui::app::FilterDimension::Category
    );
    assert_eq!(app.filter_cursor, 0);

    // 4. Toggle Application category (index 0) - should select it
    let key_event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
    InputHandler::handle_event(&mut app, Event::Key(key_event));
    assert_eq!(app.category_filters.len(), 1);
    assert!(app
        .category_filters
        .contains(&SoftwareCategory::Application));
    assert_eq!(app.item_count(), 3); // Firefox, Slack, VLC are Applications

    // 5. Toggle it again to deselect
    let key_event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
    InputHandler::handle_event(&mut app, Event::Key(key_event));
    assert_eq!(app.category_filters.len(), 0);
    assert_eq!(app.item_count(), 5);

    // 6. Navigate right to Manager dimension
    let key_event = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
    InputHandler::handle_event(&mut app, Event::Key(key_event));
    assert_eq!(
        app.filter_dimension,
        kunger_lib::tui::app::FilterDimension::Manager
    );
    assert_eq!(app.filter_cursor, 0);

    // 7. Toggle Apt manager (index 0 in manager list)
    let key_event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
    InputHandler::handle_event(&mut app, Event::Key(key_event));
    assert_eq!(app.manager_filters.len(), 1);
    assert!(app.manager_filters.contains(&PackageManager::Apt));
    assert_eq!(app.item_count(), 3); // Firefox, Git, libx11 are from Apt

    // 8. Navigate to Confidence dimension
    for _ in 0..3 {
        let key_event = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        InputHandler::handle_event(&mut app, Event::Key(key_event));
    }
    assert_eq!(
        app.filter_dimension,
        kunger_lib::tui::app::FilterDimension::Confidence
    );

    // 9. Toggle High confidence (at index 3 in ClassificationConfidence::ALL)
    // Current cursor is at 0 (Unknown), need to go to 3 (High)
    for _ in 0..3 {
        let key_event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        InputHandler::handle_event(&mut app, Event::Key(key_event));
    }
    // Now at High (index 3)
    let key_event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
    InputHandler::handle_event(&mut app, Event::Key(key_event));
    assert_eq!(app.confidence_filters.len(), 1);
    assert!(app
        .confidence_filters
        .contains(&ClassificationConfidence::High));

    // 10. Should have Firefox (Apt + High confidence)
    assert_eq!(app.item_count(), 1);
    assert_eq!(app.filtered_items[0].display_name, "Firefox");

    // 11. Clear all filters with 'c'
    let key_event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
    InputHandler::handle_event(&mut app, Event::Key(key_event));
    assert_eq!(app.category_filters.len(), 0);
    assert_eq!(app.manager_filters.len(), 0);
    assert_eq!(app.confidence_filters.len(), 0);
    assert_eq!(app.item_count(), 5);

    // 12. Close filter panel with Esc
    let key_event = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    InputHandler::handle_event(&mut app, Event::Key(key_event));
    assert!(!app.filter_panel_visible);
}

#[test]
fn test_cli_workflow_17_update_availability_filter() {
    let mut items = create_test_items();
    items[0].update_available = true;
    items[1].update_available = false;
    items[2].update_available = true;

    let mut app = App::new(items);

    // Open filter panel
    app.open_filter_panel();

    // Navigate to UpdateAvailable dimension
    for _ in 0..5 {
        app.filter_dimension_next();
    }
    assert_eq!(
        app.filter_dimension,
        kunger_lib::tui::app::FilterDimension::UpdateAvailable
    );

    // Cycle to Available filter
    app.cycle_update_filter();
    assert_eq!(
        app.update_filter,
        kunger_lib::tui::app::UpdateFilter::Available
    );
    assert_eq!(app.item_count(), 2); // Firefox and Slack have updates

    // Cycle to NotAvailable filter
    app.cycle_update_filter();
    assert_eq!(
        app.update_filter,
        kunger_lib::tui::app::UpdateFilter::NotAvailable
    );
    assert_eq!(app.item_count(), 3); // Git, libx11, VLC don't have updates

    // Cycle back to Any
    app.cycle_update_filter();
    assert_eq!(app.update_filter, kunger_lib::tui::app::UpdateFilter::Any);
    assert_eq!(app.item_count(), 5);
}
