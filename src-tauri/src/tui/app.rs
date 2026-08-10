use crate::domain::{
    ClassificationConfidence, InstallationReason, InstallationScope, PackageManager,
    SoftwareCategory, SoftwareItem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Name,
    Category,
    Manager,
    Version,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterDimension {
    Category,
    Manager,
    Scope,
    Reason,
    Confidence,
    UpdateAvailable,
}

impl FilterDimension {
    pub fn next(self) -> Self {
        match self {
            Self::Category => Self::Manager,
            Self::Manager => Self::Scope,
            Self::Scope => Self::Reason,
            Self::Reason => Self::Confidence,
            Self::Confidence => Self::UpdateAvailable,
            Self::UpdateAvailable => Self::Category,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Category => Self::UpdateAvailable,
            Self::Manager => Self::Category,
            Self::Scope => Self::Manager,
            Self::Reason => Self::Scope,
            Self::Confidence => Self::Reason,
            Self::UpdateAvailable => Self::Confidence,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Category => "Category",
            Self::Manager => "Manager",
            Self::Scope => "Scope",
            Self::Reason => "Reason",
            Self::Confidence => "Confidence",
            Self::UpdateAvailable => "Updates",
        }
    }

    pub fn value_count(self) -> usize {
        match self {
            Self::Category => SoftwareCategory::ALL.len(),
            Self::Manager => PackageManager::ALL.len(),
            Self::Scope => InstallationScope::ALL.len(),
            Self::Reason => InstallationReason::ALL.len(),
            Self::Confidence => ClassificationConfidence::ALL.len(),
            Self::UpdateAvailable => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFilter {
    Any,
    Available,
    NotAvailable,
}

impl UpdateFilter {
    pub fn cycle(self) -> Self {
        match self {
            Self::Any => Self::Available,
            Self::Available => Self::NotAvailable,
            Self::NotAvailable => Self::Any,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Any => "Any",
            Self::Available => "Available",
            Self::NotAvailable => "Not Available",
        }
    }
}

#[derive(Debug, Clone)]
pub struct App {
    pub all_items: Vec<SoftwareItem>,
    pub filtered_items: Vec<SoftwareItem>,
    pub selected_index: usize,
    pub search_query: String,
    pub page_size: usize,
    pub current_page: usize,
    pub scroll_offset: usize,

    // Filters
    pub category_filters: Vec<SoftwareCategory>,
    pub manager_filters: Vec<PackageManager>,
    pub scope_filters: Vec<InstallationScope>,
    pub reason_filters: Vec<InstallationReason>,
    pub confidence_filters: Vec<ClassificationConfidence>,
    pub update_filter: UpdateFilter,

    // Sorting
    pub sort_field: SortField,
    pub sort_order: SortOrder,

    // UI state
    pub search_focused: bool,
    pub cursor_position: usize,
    pub detail_view_visible: bool,
    pub filter_panel_visible: bool,
    pub filter_dimension: FilterDimension,
    pub filter_cursor: usize,
    pub is_scanning: bool,
    pub scan_progress: u16,
    pub scan_message: Option<String>,
}

impl App {
    pub fn new(items: Vec<SoftwareItem>) -> Self {
        Self {
            all_items: items.clone(),
            filtered_items: items,
            selected_index: 0,
            search_query: String::new(),
            page_size: 20,
            current_page: 0,
            scroll_offset: 0,
            category_filters: vec![],
            manager_filters: vec![],
            scope_filters: vec![],
            reason_filters: vec![],
            confidence_filters: vec![],
            update_filter: UpdateFilter::Any,
            sort_field: SortField::Name,
            sort_order: SortOrder::Ascending,
            search_focused: false,
            cursor_position: 0,
            detail_view_visible: false,
            filter_panel_visible: false,
            filter_dimension: FilterDimension::Category,
            filter_cursor: 0,
            is_scanning: false,
            scan_progress: 0,
            scan_message: None,
        }
    }

    pub fn select_next(&mut self) {
        if !self.filtered_items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_items.len();
            self.update_page_for_selection();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.filtered_items.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.filtered_items.len() - 1;
            } else {
                self.selected_index -= 1;
            }
            self.update_page_for_selection();
        }
    }

    pub fn page_down(&mut self) {
        let total_pages = self.total_pages();
        if self.current_page < total_pages.saturating_sub(1) {
            self.current_page += 1;
            self.selected_index = self.current_page * self.page_size;
            if self.selected_index >= self.filtered_items.len() {
                self.selected_index = self.filtered_items.len().saturating_sub(1);
            }
        }
    }

    pub fn page_up(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.selected_index = self.current_page * self.page_size;
        }
    }

    fn update_page_for_selection(&mut self) {
        self.current_page = self.selected_index / self.page_size;
        self.scroll_offset = self.selected_index % self.page_size;
    }

    pub fn visible_items(&self) -> Vec<&SoftwareItem> {
        let start = self.current_page * self.page_size;
        let end = (start + self.page_size).min(self.filtered_items.len());
        self.filtered_items[start..end].iter().collect()
    }

    pub fn selected_item(&self) -> Option<&SoftwareItem> {
        self.filtered_items.get(self.selected_index)
    }

    pub fn total_pages(&self) -> usize {
        (self.filtered_items.len() + self.page_size - 1) / self.page_size
    }

    pub fn item_count(&self) -> usize {
        self.filtered_items.len()
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.selected_index = 0;
        self.current_page = 0;
        self.apply_filters();
    }

    pub fn apply_filters(&mut self) {
        self.filtered_items = self
            .all_items
            .iter()
            .filter(|item| {
                // Search filter
                if !self.search_query.is_empty() {
                    let query_lower = self.search_query.to_lowercase();
                    let matches_search = item.display_name.to_lowercase().contains(&query_lower)
                        || item.package_name.to_lowercase().contains(&query_lower)
                        || item
                            .description
                            .as_ref()
                            .map_or(false, |d| d.to_lowercase().contains(&query_lower));
                    if !matches_search {
                        return false;
                    }
                }

                // Category filter
                if !self.category_filters.is_empty()
                    && !self.category_filters.contains(&item.category)
                {
                    return false;
                }

                // Manager filter
                if !self.manager_filters.is_empty()
                    && !self.manager_filters.contains(&item.package_manager)
                {
                    return false;
                }

                // Scope filter
                if !self.scope_filters.is_empty() && !self.scope_filters.contains(&item.scope) {
                    return false;
                }

                // Reason filter
                if !self.reason_filters.is_empty()
                    && !self.reason_filters.contains(&item.installation_reason)
                {
                    return false;
                }

                // Confidence filter
                if !self.confidence_filters.is_empty()
                    && !self
                        .confidence_filters
                        .contains(&item.classification_confidence)
                {
                    return false;
                }

                // Update availability filter
                match self.update_filter {
                    UpdateFilter::Available if !item.update_available => return false,
                    UpdateFilter::NotAvailable if item.update_available => return false,
                    _ => {}
                }

                true
            })
            .cloned()
            .collect();

        // Apply sorting
        match self.sort_field {
            SortField::Name => {
                self.filtered_items
                    .sort_by(|a, b| a.display_name.cmp(&b.display_name));
            }
            SortField::Category => {
                self.filtered_items
                    .sort_by(|a, b| format!("{:?}", a.category).cmp(&format!("{:?}", b.category)));
            }
            SortField::Manager => {
                self.filtered_items.sort_by(|a, b| {
                    format!("{:?}", a.package_manager).cmp(&format!("{:?}", b.package_manager))
                });
            }
            SortField::Version => {
                self.filtered_items.sort_by(|a, b| {
                    let av = a.version.as_deref().unwrap_or("");
                    let bv = b.version.as_deref().unwrap_or("");
                    av.cmp(bv)
                });
            }
        }

        if self.sort_order == SortOrder::Descending {
            self.filtered_items.reverse();
        }
    }

    pub fn toggle_category_filter(&mut self, category: SoftwareCategory) {
        if self.category_filters.contains(&category) {
            self.category_filters.retain(|c| c != &category);
        } else {
            self.category_filters.push(category);
        }
        self.selected_index = 0;
        self.current_page = 0;
        self.apply_filters();
    }

    pub fn toggle_manager_filter(&mut self, manager: PackageManager) {
        if self.manager_filters.contains(&manager) {
            self.manager_filters.retain(|m| m != &manager);
        } else {
            self.manager_filters.push(manager);
        }
        self.selected_index = 0;
        self.current_page = 0;
        self.apply_filters();
    }

    pub fn toggle_scope_filter(&mut self, scope: InstallationScope) {
        if self.scope_filters.contains(&scope) {
            self.scope_filters.retain(|s| s != &scope);
        } else {
            self.scope_filters.push(scope);
        }
        self.selected_index = 0;
        self.current_page = 0;
        self.apply_filters();
    }

    pub fn toggle_reason_filter(&mut self, reason: InstallationReason) {
        if self.reason_filters.contains(&reason) {
            self.reason_filters.retain(|r| r != &reason);
        } else {
            self.reason_filters.push(reason);
        }
        self.selected_index = 0;
        self.current_page = 0;
        self.apply_filters();
    }

    pub fn toggle_confidence_filter(&mut self, confidence: ClassificationConfidence) {
        if self.confidence_filters.contains(&confidence) {
            self.confidence_filters.retain(|c| c != &confidence);
        } else {
            self.confidence_filters.push(confidence);
        }
        self.selected_index = 0;
        self.current_page = 0;
        self.apply_filters();
    }

    pub fn cycle_update_filter(&mut self) {
        self.update_filter = self.update_filter.cycle();
        self.selected_index = 0;
        self.current_page = 0;
        self.apply_filters();
    }

    pub fn clear_all_filters(&mut self) {
        self.category_filters.clear();
        self.manager_filters.clear();
        self.scope_filters.clear();
        self.reason_filters.clear();
        self.confidence_filters.clear();
        self.update_filter = UpdateFilter::Any;
        self.search_query.clear();
        self.cursor_position = 0;
        self.selected_index = 0;
        self.current_page = 0;
        self.apply_filters();
    }

    pub fn add_search_char(&mut self, ch: char) {
        self.search_query.insert(self.cursor_position, ch);
        self.cursor_position += 1;
        self.selected_index = 0;
        self.current_page = 0;
        self.apply_filters();
    }

    pub fn backspace_search(&mut self) {
        if self.cursor_position > 0 {
            self.search_query.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
            self.selected_index = 0;
            self.current_page = 0;
            self.apply_filters();
        }
    }

    pub fn cycle_sort_field(&mut self) {
        self.sort_field = match self.sort_field {
            SortField::Name => SortField::Category,
            SortField::Category => SortField::Manager,
            SortField::Manager => SortField::Version,
            SortField::Version => SortField::Name,
        };
        self.apply_filters();
    }

    pub fn toggle_sort_order(&mut self) {
        self.sort_order = match self.sort_order {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        };
        self.apply_filters();
    }

    pub fn toggle_detail_view(&mut self) {
        self.detail_view_visible = !self.detail_view_visible;
    }

    pub fn close_detail_view(&mut self) {
        self.detail_view_visible = false;
    }

    pub fn open_filter_panel(&mut self) {
        self.filter_panel_visible = true;
    }

    pub fn close_filter_panel(&mut self) {
        self.filter_panel_visible = false;
    }

    pub fn filter_dimension_next(&mut self) {
        self.filter_dimension = self.filter_dimension.next();
        self.filter_cursor = 0;
    }

    pub fn filter_dimension_prev(&mut self) {
        self.filter_dimension = self.filter_dimension.prev();
        self.filter_cursor = 0;
    }

    pub fn filter_cursor_next(&mut self) {
        let count = self.filter_dimension.value_count();
        if count > 0 {
            self.filter_cursor = (self.filter_cursor + 1) % count;
        }
    }

    pub fn filter_cursor_prev(&mut self) {
        let count = self.filter_dimension.value_count();
        if count > 0 {
            self.filter_cursor = if self.filter_cursor == 0 {
                count - 1
            } else {
                self.filter_cursor - 1
            };
        }
    }

    pub fn toggle_current_filter_value(&mut self) {
        match self.filter_dimension {
            FilterDimension::Category => {
                if self.filter_cursor < SoftwareCategory::ALL.len() {
                    self.toggle_category_filter(SoftwareCategory::ALL[self.filter_cursor]);
                }
            }
            FilterDimension::Manager => {
                if self.filter_cursor < PackageManager::ALL.len() {
                    self.toggle_manager_filter(PackageManager::ALL[self.filter_cursor]);
                }
            }
            FilterDimension::Scope => {
                if self.filter_cursor < InstallationScope::ALL.len() {
                    self.toggle_scope_filter(InstallationScope::ALL[self.filter_cursor]);
                }
            }
            FilterDimension::Reason => {
                if self.filter_cursor < InstallationReason::ALL.len() {
                    self.toggle_reason_filter(InstallationReason::ALL[self.filter_cursor]);
                }
            }
            FilterDimension::Confidence => {
                if self.filter_cursor < ClassificationConfidence::ALL.len() {
                    self.toggle_confidence_filter(
                        ClassificationConfidence::ALL[self.filter_cursor],
                    );
                }
            }
            FilterDimension::UpdateAvailable => {
                self.cycle_update_filter();
            }
        }
    }

    pub fn start_scan(&mut self) {
        self.is_scanning = true;
        self.scan_progress = 0;
        self.scan_message = Some("Scanning inventory...".to_string());
    }

    pub fn update_scan_progress(&mut self, progress: u16) {
        self.scan_progress = progress.min(100);
    }

    pub fn finish_scan(&mut self) {
        self.is_scanning = false;
        self.scan_progress = 100;
    }

    pub fn complete_scan(&mut self, message: impl Into<String>) {
        self.finish_scan();
        self.scan_message = Some(message.into());
    }

    pub fn fail_scan(&mut self, message: impl Into<String>) {
        self.is_scanning = false;
        self.scan_progress = 0;
        self.scan_message = Some(message.into());
    }

    pub fn set_scan_message(&mut self, message: impl Into<String>) {
        self.scan_message = Some(message.into());
    }

    pub fn replace_items(&mut self, items: Vec<SoftwareItem>) {
        self.all_items = items;
        self.selected_index = 0;
        self.current_page = 0;
        self.scroll_offset = 0;
        self.apply_filters();
    }

    pub fn reset(&mut self) {
        self.search_query.clear();
        self.cursor_position = 0;
        self.category_filters.clear();
        self.manager_filters.clear();
        self.scope_filters.clear();
        self.reason_filters.clear();
        self.confidence_filters.clear();
        self.update_filter = UpdateFilter::Any;
        self.selected_index = 0;
        self.current_page = 0;
        self.sort_field = SortField::Name;
        self.sort_order = SortOrder::Ascending;
        self.detail_view_visible = false;
        self.filter_panel_visible = false;
        self.filter_dimension = FilterDimension::Category;
        self.filter_cursor = 0;
        self.is_scanning = false;
        self.scan_progress = 0;
        self.scan_message = None;
        self.filtered_items = self.all_items.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ClassificationConfidence, PackageManager, SoftwareCategory};

    fn create_test_item(name: &str, package_name: &str, description: &str) -> SoftwareItem {
        let mut item = SoftwareItem::new(name, package_name, name, PackageManager::Apt);
        item.description = Some(description.to_string());
        item.version = Some("1.0".to_string());
        item.category = SoftwareCategory::Application;
        item.classification_confidence = ClassificationConfidence::High;
        item
    }

    #[test]
    fn test_app_initialization() {
        let items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Vim", "vim", "Text editor"),
        ];
        let app = App::new(items.clone());

        assert_eq!(app.item_count(), 2);
        assert_eq!(app.total_pages(), 1);
        assert_eq!(app.selected_index, 0);
        assert_eq!(app.all_items.len(), 2);
        assert_eq!(app.filtered_items.len(), 2);
    }

    #[test]
    fn test_pagination_multiple_pages() {
        let mut items = vec![];
        for i in 0..50 {
            items.push(create_test_item(
                &format!("App{}", i),
                &format!("app{}", i),
                "Test application",
            ));
        }

        let app = App::new(items);
        assert_eq!(app.total_pages(), 3); // 50 items / 20 per page = 3 pages
        assert_eq!(app.visible_items().len(), 20);
    }

    #[test]
    fn test_navigation_next() {
        let items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Vim", "vim", "Text editor"),
            create_test_item("Git", "git", "Version control"),
        ];
        let mut app = App::new(items);

        assert_eq!(app.selected_index, 0);
        app.select_next();
        assert_eq!(app.selected_index, 1);
        app.select_next();
        assert_eq!(app.selected_index, 2);
        app.select_next(); // Wraps around
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_navigation_previous() {
        let items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Vim", "vim", "Text editor"),
            create_test_item("Git", "git", "Version control"),
        ];
        let mut app = App::new(items);

        app.selected_index = 1;
        app.select_previous();
        assert_eq!(app.selected_index, 0);
        app.select_previous(); // Wraps around
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn test_page_navigation() {
        let mut items = vec![];
        for i in 0..50 {
            items.push(create_test_item(
                &format!("App{}", i),
                &format!("app{}", i),
                "Test application",
            ));
        }

        let mut app = App::new(items);
        assert_eq!(app.current_page, 0);

        app.page_down();
        assert_eq!(app.current_page, 1);

        app.page_down();
        assert_eq!(app.current_page, 2);

        app.page_down(); // Should not go beyond last page
        assert_eq!(app.current_page, 2);

        app.page_up();
        assert_eq!(app.current_page, 1);
    }

    #[test]
    fn test_search_filtering() {
        let items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Thunderbird", "thunderbird", "Email client"),
            create_test_item("Vim", "vim", "Text editor"),
        ];
        let mut app = App::new(items);

        app.set_search_query("firefox".to_string());
        assert_eq!(app.item_count(), 1);
        assert_eq!(app.selected_item().unwrap().display_name, "Firefox");
    }

    #[test]
    fn test_search_case_insensitive() {
        let items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Vim", "vim", "Text editor"),
        ];
        let mut app = App::new(items);

        app.set_search_query("FIREFOX".to_string());
        assert_eq!(app.item_count(), 1);
    }

    #[test]
    fn test_search_package_name() {
        let items = vec![
            create_test_item("Firefox", "firefox-esr", "Web browser"),
            create_test_item("Vim", "vim", "Text editor"),
        ];
        let mut app = App::new(items);

        app.set_search_query("firefox".to_string());
        assert_eq!(app.item_count(), 1);
    }

    #[test]
    fn test_search_description() {
        let items = vec![
            create_test_item("Firefox", "firefox", "Web browser and network tools"),
            create_test_item("Vim", "vim", "Text editor"),
        ];
        let mut app = App::new(items);

        app.set_search_query("browser".to_string());
        assert_eq!(app.item_count(), 1);
    }

    #[test]
    fn test_reset_filters() {
        let items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Vim", "vim", "Text editor"),
        ];
        let mut app = App::new(items);

        app.set_search_query("firefox".to_string());
        assert_eq!(app.item_count(), 1);

        app.reset();
        assert_eq!(app.item_count(), 2);
        assert_eq!(app.search_query, "");
    }

    #[test]
    fn test_selected_item_on_page() {
        let mut items = vec![];
        for i in 0..50 {
            items.push(create_test_item(
                &format!("App{}", i),
                &format!("app{}", i),
                "Test application",
            ));
        }

        let mut app = App::new(items);
        app.selected_index = 0;
        let selected = app.selected_item();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().display_name, "App0");

        app.selected_index = 25;
        app.page_down(); // Move to page 1
        let selected = app.selected_item();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().display_name, "App20");
    }

    #[test]
    fn test_empty_items() {
        let app = App::new(vec![]);
        assert_eq!(app.item_count(), 0);
        assert_eq!(app.total_pages(), 0);
        assert!(app.selected_item().is_none());
    }

    // Phase 2 tests: Filtering & Search

    #[test]
    fn test_category_filter() {
        let mut items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Git", "git", "Version control"),
        ];
        items[0].category = SoftwareCategory::Application;
        items[1].category = SoftwareCategory::CommandLineTool;

        let mut app = App::new(items);
        app.toggle_category_filter(SoftwareCategory::Application);
        assert_eq!(app.item_count(), 1);
        assert_eq!(app.selected_item().unwrap().display_name, "Firefox");
    }

    #[test]
    fn test_manager_filter() {
        let mut items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Slack", "slack", "Chat app"),
        ];
        items[0].package_manager = PackageManager::Apt;
        items[1].package_manager = PackageManager::Flatpak;

        let mut app = App::new(items);
        app.toggle_manager_filter(PackageManager::Apt);
        assert_eq!(app.item_count(), 1);
        assert_eq!(app.selected_item().unwrap().display_name, "Firefox");
    }

    #[test]
    fn test_scope_filter() {
        let mut items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Git", "git", "Version control"),
        ];
        items[0].scope = InstallationScope::System;
        items[1].scope = InstallationScope::User;

        let mut app = App::new(items);
        app.toggle_scope_filter(InstallationScope::System);
        assert_eq!(app.item_count(), 1);
        assert_eq!(app.selected_item().unwrap().display_name, "Firefox");
    }

    #[test]
    fn test_reason_filter() {
        let mut items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("LibX11", "libx11", "X11 library"),
        ];
        items[0].installation_reason = InstallationReason::Manual;
        items[1].installation_reason = InstallationReason::Automatic;

        let mut app = App::new(items);
        app.toggle_reason_filter(InstallationReason::Manual);
        assert_eq!(app.item_count(), 1);
        assert_eq!(app.selected_item().unwrap().display_name, "Firefox");
    }

    #[test]
    fn test_multiple_filters_combined() {
        let mut items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Git", "git", "Version control"),
            create_test_item("Vim", "vim", "Text editor"),
        ];
        items[0].category = SoftwareCategory::Application;
        items[0].package_manager = PackageManager::Apt;
        items[1].category = SoftwareCategory::CommandLineTool;
        items[1].package_manager = PackageManager::Apt;
        items[2].category = SoftwareCategory::CommandLineTool;
        items[2].package_manager = PackageManager::Cargo;

        let mut app = App::new(items);
        app.toggle_category_filter(SoftwareCategory::CommandLineTool);
        app.toggle_manager_filter(PackageManager::Apt);

        assert_eq!(app.item_count(), 1);
        assert_eq!(app.selected_item().unwrap().display_name, "Git");
    }

    #[test]
    fn test_filter_toggle() {
        let items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Vim", "vim", "Text editor"),
        ];
        let mut app = App::new(items);

        app.toggle_category_filter(SoftwareCategory::Application);
        assert_eq!(app.category_filters.len(), 1);

        app.toggle_category_filter(SoftwareCategory::Application);
        assert_eq!(app.category_filters.len(), 0);
    }

    #[test]
    fn test_clear_all_filters() {
        let mut items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Vim", "vim", "Text editor"),
        ];
        items[0].category = SoftwareCategory::Application;

        let mut app = App::new(items);
        app.set_search_query("firefox".to_string());
        app.toggle_category_filter(SoftwareCategory::Application);

        assert_eq!(app.item_count(), 1);
        app.clear_all_filters();
        assert_eq!(app.item_count(), 2);
        assert_eq!(app.search_query, "");
        assert!(app.category_filters.is_empty());
    }

    #[test]
    fn test_sorting_by_name() {
        let items = vec![
            create_test_item("Zsh", "zsh", "Shell"),
            create_test_item("Bash", "bash", "Shell"),
            create_test_item("Fish", "fish", "Shell"),
        ];
        let mut app = App::new(items);
        app.apply_filters();

        let names: Vec<_> = app
            .filtered_items
            .iter()
            .map(|i| i.display_name.as_str())
            .collect();
        assert_eq!(names, vec!["Bash", "Fish", "Zsh"]);
    }

    #[test]
    fn test_sorting_descending() {
        let items = vec![
            create_test_item("Bash", "bash", "Shell"),
            create_test_item("Fish", "fish", "Shell"),
            create_test_item("Zsh", "zsh", "Shell"),
        ];
        let mut app = App::new(items);
        app.sort_order = SortOrder::Descending;
        app.apply_filters();

        let names: Vec<_> = app
            .filtered_items
            .iter()
            .map(|i| i.display_name.as_str())
            .collect();
        assert_eq!(names, vec!["Zsh", "Fish", "Bash"]);
    }

    #[test]
    fn test_sorting_by_category() {
        let mut items = vec![
            create_test_item("Vim", "vim", "Text editor"),
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Git", "git", "Tool"),
        ];
        items[0].category = SoftwareCategory::CommandLineTool;
        items[1].category = SoftwareCategory::Application;
        items[2].category = SoftwareCategory::CommandLineTool;

        let mut app = App::new(items);
        app.sort_field = SortField::Category;
        app.apply_filters();

        let categories: Vec<_> = app.filtered_items.iter().map(|i| i.category).collect();
        assert_eq!(categories[0], SoftwareCategory::Application);
    }

    #[test]
    fn test_search_and_filter_combined() {
        let mut items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Firefox-ESR", "firefox-esr", "Web browser ESR"),
            create_test_item("Chrome", "chrome", "Web browser"),
        ];
        items[0].package_manager = PackageManager::Apt;
        items[1].package_manager = PackageManager::Apt;
        items[2].package_manager = PackageManager::Flatpak;

        let mut app = App::new(items);
        app.set_search_query("firefox".to_string());
        app.toggle_manager_filter(PackageManager::Apt);

        assert_eq!(app.item_count(), 2);
        let names: Vec<_> = app
            .filtered_items
            .iter()
            .map(|i| i.display_name.as_str())
            .collect();
        assert!(names.contains(&"Firefox"));
        assert!(names.contains(&"Firefox-ESR"));
    }

    #[test]
    fn test_add_search_char() {
        let items = vec![create_test_item("Firefox", "firefox", "Web browser")];
        let mut app = App::new(items);

        app.search_focused = true;
        app.add_search_char('f');
        assert_eq!(app.search_query, "f");
        assert_eq!(app.cursor_position, 1);

        app.add_search_char('i');
        assert_eq!(app.search_query, "fi");
        assert_eq!(app.cursor_position, 2);
    }

    #[test]
    fn test_backspace_search() {
        let items = vec![create_test_item("Firefox", "firefox", "Web browser")];
        let mut app = App::new(items);

        app.search_query = "test".to_string();
        app.cursor_position = 4;
        app.backspace_search();

        assert_eq!(app.search_query, "tes");
        assert_eq!(app.cursor_position, 3);
    }

    #[test]
    fn test_cycle_sort_field() {
        let items = vec![create_test_item("Firefox", "firefox", "Web browser")];
        let mut app = App::new(items);

        assert_eq!(app.sort_field, SortField::Name);
        app.cycle_sort_field();
        assert_eq!(app.sort_field, SortField::Category);
        app.cycle_sort_field();
        assert_eq!(app.sort_field, SortField::Manager);
        app.cycle_sort_field();
        assert_eq!(app.sort_field, SortField::Version);
        app.cycle_sort_field();
        assert_eq!(app.sort_field, SortField::Name);
    }

    #[test]
    fn test_toggle_sort_order() {
        let items = vec![create_test_item("Firefox", "firefox", "Web browser")];
        let mut app = App::new(items);

        assert_eq!(app.sort_order, SortOrder::Ascending);
        app.toggle_sort_order();
        assert_eq!(app.sort_order, SortOrder::Descending);
        app.toggle_sort_order();
        assert_eq!(app.sort_order, SortOrder::Ascending);
    }

    // Phase 3 tests: Detail View & Scan

    #[test]
    fn test_detail_view_visibility() {
        let items = vec![create_test_item("Firefox", "firefox", "Web browser")];
        let mut app = App::new(items);

        assert!(!app.detail_view_visible);
        app.toggle_detail_view();
        assert!(app.detail_view_visible);
        app.toggle_detail_view();
        assert!(!app.detail_view_visible);
    }

    #[test]
    fn test_close_detail_view() {
        let items = vec![create_test_item("Firefox", "firefox", "Web browser")];
        let mut app = App::new(items);

        app.detail_view_visible = true;
        app.close_detail_view();
        assert!(!app.detail_view_visible);
    }

    #[test]
    fn test_selected_item_in_detail_view() {
        let items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Git", "git", "Version control"),
        ];
        let mut app = App::new(items);

        app.select_next(); // Select Git
        app.toggle_detail_view();

        assert!(app.detail_view_visible);
        assert_eq!(app.selected_item().unwrap().display_name, "Git");
    }

    #[test]
    fn test_scan_state_transitions() {
        let items = vec![create_test_item("Firefox", "firefox", "Web browser")];
        let mut app = App::new(items);

        // Initial state
        assert!(!app.is_scanning);
        assert_eq!(app.scan_progress, 0);

        // Start scan
        app.start_scan();
        assert!(app.is_scanning);
        assert_eq!(app.scan_progress, 0);

        // Update progress
        app.update_scan_progress(50);
        assert!(app.is_scanning);
        assert_eq!(app.scan_progress, 50);

        // Finish scan
        app.finish_scan();
        assert!(!app.is_scanning);
        assert_eq!(app.scan_progress, 100);
    }

    #[test]
    fn test_scan_progress_clamping() {
        let items = vec![create_test_item("Firefox", "firefox", "Web browser")];
        let mut app = App::new(items);

        // Progress over 100 should be clamped
        app.update_scan_progress(150);
        assert_eq!(app.scan_progress, 100);

        app.update_scan_progress(50);
        assert_eq!(app.scan_progress, 50);
    }

    #[test]
    fn test_reset_clears_detail_and_scan_state() {
        let items = vec![create_test_item("Firefox", "firefox", "Web browser")];
        let mut app = App::new(items);

        app.toggle_detail_view();
        app.start_scan();
        app.update_scan_progress(75);

        assert!(app.detail_view_visible);
        assert!(app.is_scanning);
        assert_eq!(app.scan_progress, 75);

        app.reset();

        assert!(!app.detail_view_visible);
        assert!(!app.is_scanning);
        assert_eq!(app.scan_progress, 0);
    }

    #[test]
    fn test_detail_view_with_filtered_items() {
        let mut items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Vim", "vim", "Text editor"),
        ];
        items[0].category = SoftwareCategory::Application;
        items[1].category = SoftwareCategory::CommandLineTool;

        let mut app = App::new(items);

        // Filter to only Applications
        app.toggle_category_filter(SoftwareCategory::Application);
        assert_eq!(app.item_count(), 1);

        // Show detail view
        app.toggle_detail_view();
        assert!(app.detail_view_visible);
        assert_eq!(app.selected_item().unwrap().display_name, "Firefox");
    }

    #[test]
    fn test_detail_view_preserves_navigation() {
        let items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Git", "git", "Version control"),
            create_test_item("Vim", "vim", "Text editor"),
        ];
        let mut app = App::new(items);

        app.toggle_detail_view();
        assert_eq!(app.selected_item().unwrap().display_name, "Firefox");

        // Navigate while detail view is open
        app.select_next();
        assert_eq!(app.selected_item().unwrap().display_name, "Git");

        app.select_next();
        assert_eq!(app.selected_item().unwrap().display_name, "Vim");

        // Detail view should still be visible
        assert!(app.detail_view_visible);
    }

    #[test]
    fn test_multiple_detail_view_toggles() {
        let items = vec![
            create_test_item("Firefox", "firefox", "Web browser"),
            create_test_item("Git", "git", "Version control"),
        ];
        let mut app = App::new(items);

        // Toggle multiple times and verify alternation
        assert!(!app.detail_view_visible);

        app.toggle_detail_view();
        assert!(app.detail_view_visible);

        app.toggle_detail_view();
        assert!(!app.detail_view_visible);

        app.toggle_detail_view();
        assert!(app.detail_view_visible);

        app.toggle_detail_view();
        assert!(!app.detail_view_visible);

        app.toggle_detail_view();
        assert!(app.detail_view_visible);

        // Should end up in open state (odd number of toggles from closed start)
        assert!(app.detail_view_visible);
    }

    #[test]
    fn test_scan_progress_boundaries() {
        let items = vec![create_test_item("Firefox", "firefox", "Web browser")];
        let mut app = App::new(items);

        app.start_scan();

        // Test boundary values
        app.update_scan_progress(0);
        assert_eq!(app.scan_progress, 0);

        app.update_scan_progress(50);
        assert_eq!(app.scan_progress, 50);

        app.update_scan_progress(100);
        assert_eq!(app.scan_progress, 100);

        // Over 100 should clamp
        app.update_scan_progress(200);
        assert_eq!(app.scan_progress, 100);
    }

    #[test]
    fn replace_items_preserves_active_filters_and_resets_selection() {
        let mut app = App::new(vec![create_test_item("Firefox", "firefox", "Web browser")]);
        app.toggle_category_filter(SoftwareCategory::Application);
        app.selected_index = 1;
        app.current_page = 1;

        let mut matching = create_test_item("Vim", "vim", "Text editor");
        matching.category = SoftwareCategory::Application;
        let mut filtered_out = create_test_item("Git", "git", "Version control");
        filtered_out.category = SoftwareCategory::CommandLineTool;
        app.replace_items(vec![matching, filtered_out]);

        assert_eq!(app.all_items.len(), 2);
        assert_eq!(app.item_count(), 1);
        assert_eq!(app.selected_index, 0);
        assert_eq!(app.current_page, 0);
        assert_eq!(app.selected_item().unwrap().display_name, "Vim");
    }

    #[test]
    fn scan_messages_cover_completion_and_failure() {
        let mut app = App::new(vec![]);
        app.start_scan();
        assert_eq!(app.scan_message.as_deref(), Some("Scanning inventory..."));

        app.complete_scan("Scan complete: 0 items found.");
        assert!(!app.is_scanning);
        assert_eq!(
            app.scan_message.as_deref(),
            Some("Scan complete: 0 items found.")
        );

        app.fail_scan("Scan cancelled.");
        assert!(!app.is_scanning);
        assert_eq!(app.scan_message.as_deref(), Some("Scan cancelled."));
    }

    #[test]
    fn test_confidence_filter() {
        let mut item1 = create_test_item("Firefox", "firefox", "Web browser");
        item1.classification_confidence = ClassificationConfidence::High;
        let mut item2 = create_test_item("Vim", "vim", "Text editor");
        item2.classification_confidence = ClassificationConfidence::Low;

        let mut app = App::new(vec![item1, item2]);
        assert_eq!(app.item_count(), 2);

        app.toggle_confidence_filter(ClassificationConfidence::High);
        assert_eq!(app.item_count(), 1);
        assert_eq!(
            app.filtered_items[0].classification_confidence,
            ClassificationConfidence::High
        );

        app.toggle_confidence_filter(ClassificationConfidence::Low);
        assert_eq!(app.item_count(), 2);

        app.toggle_confidence_filter(ClassificationConfidence::High);
        assert_eq!(app.item_count(), 1);
        assert_eq!(
            app.filtered_items[0].classification_confidence,
            ClassificationConfidence::Low
        );
    }

    #[test]
    fn test_update_filter_cycle() {
        let mut item1 = create_test_item("Firefox", "firefox", "Web browser");
        item1.update_available = true;
        let mut item2 = create_test_item("Vim", "vim", "Text editor");
        item2.update_available = false;

        let mut app = App::new(vec![item1, item2]);
        assert_eq!(app.item_count(), 2);
        assert_eq!(app.update_filter, UpdateFilter::Any);

        app.cycle_update_filter();
        assert_eq!(app.update_filter, UpdateFilter::Available);
        assert_eq!(app.item_count(), 1);
        assert!(app.filtered_items[0].update_available);

        app.cycle_update_filter();
        assert_eq!(app.update_filter, UpdateFilter::NotAvailable);
        assert_eq!(app.item_count(), 1);
        assert!(!app.filtered_items[0].update_available);

        app.cycle_update_filter();
        assert_eq!(app.update_filter, UpdateFilter::Any);
        assert_eq!(app.item_count(), 2);
    }

    #[test]
    fn test_filter_panel_navigation() {
        let _app = App::new(vec![]);

        // Test dimension navigation
        let mut dimension = FilterDimension::Category;
        for _ in 0..6 {
            dimension = dimension.next();
        }
        assert_eq!(dimension, FilterDimension::Category);

        dimension = FilterDimension::Category;
        for _ in 0..6 {
            dimension = dimension.prev();
        }
        assert_eq!(dimension, FilterDimension::Category);

        // Test cursor wrapping for different dimensions
        let mut app = App::new(vec![create_test_item("Test", "test", "Test")]);
        app.filter_dimension = FilterDimension::UpdateAvailable;
        app.filter_cursor = 2;
        app.filter_cursor_next();
        assert_eq!(app.filter_cursor, 0);

        app.filter_cursor = 0;
        app.filter_cursor_prev();
        assert_eq!(app.filter_cursor, 2);
    }

    #[test]
    fn test_toggle_current_filter_value_dispatches_by_dimension() {
        let mut item1 = create_test_item("Firefox", "firefox", "Web browser");
        item1.category = SoftwareCategory::Application;
        let mut item2 = create_test_item("Vim", "vim", "Text editor");
        item2.category = SoftwareCategory::CommandLineTool;

        let mut app = App::new(vec![item1, item2]);
        app.filter_dimension = FilterDimension::Category;
        app.filter_cursor = 0;

        app.toggle_current_filter_value();
        assert_eq!(app.category_filters.len(), 1);
        assert!(app
            .category_filters
            .contains(&SoftwareCategory::Application));
    }

    #[test]
    fn test_filter_panel_visibility() {
        let mut app = App::new(vec![]);
        assert!(!app.filter_panel_visible);

        app.open_filter_panel();
        assert!(app.filter_panel_visible);

        app.close_filter_panel();
        assert!(!app.filter_panel_visible);
    }

    #[test]
    fn test_clear_all_filters_with_confidence_and_update() {
        let mut item = create_test_item("Firefox", "firefox", "Web browser");
        item.classification_confidence = ClassificationConfidence::High;
        item.update_available = true;

        let mut app = App::new(vec![item]);
        app.toggle_confidence_filter(ClassificationConfidence::High);
        app.cycle_update_filter();

        assert_eq!(app.confidence_filters.len(), 1);
        assert_ne!(app.update_filter, UpdateFilter::Any);

        app.clear_all_filters();
        assert_eq!(app.confidence_filters.len(), 0);
        assert_eq!(app.update_filter, UpdateFilter::Any);
        assert_eq!(app.item_count(), 1);
    }
}
