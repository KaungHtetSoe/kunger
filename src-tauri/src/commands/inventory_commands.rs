//! `get_inventory_summary`, `list_software_items`, `get_software_item`,
//! `list_duplicate_groups`, `list_provider_warnings`, `rebuild_cache`.
//!
//! Filtering/sorting/pagination for `list_software_items` happens
//! in-memory over the latest scan's items rather than at the SQL layer:
//! expected volumes (low thousands of items) make this simple and fast
//! enough for v1 — see ADR-0014. Revisit with SQL-level filtering only if
//! profiling ever shows it's needed.

use std::sync::Arc;

use crate::domain::{DuplicateGroup, InventorySummary, SoftwareItem};

use super::{
    run_blocking, AppState, CommandError, ListSoftwareItemsRequest, ListSoftwareItemsResponse,
    ProviderWarningsResponse, SortDirection, SortField, MAX_PAGE_SIZE,
};

pub async fn get_inventory_summary_impl(
    state: &AppState,
) -> Result<Option<InventorySummary>, CommandError> {
    let repository = Arc::clone(&state.repository);
    run_blocking(move || repository.latest_scan_summary()).await
}

pub async fn list_software_items_impl(
    state: &AppState,
    request: ListSoftwareItemsRequest,
) -> Result<ListSoftwareItemsResponse, CommandError> {
    let page = request.page.unwrap_or(1);
    let page_size = request.page_size.unwrap_or(50);

    if page == 0 {
        return Err(CommandError::invalid_request("page must be >= 1"));
    }
    if page_size == 0 || page_size > MAX_PAGE_SIZE {
        return Err(CommandError::invalid_request(format!(
            "pageSize must be between 1 and {MAX_PAGE_SIZE}"
        )));
    }

    let repository = Arc::clone(&state.repository);
    let all_items = run_blocking(move || repository.latest_items()).await?;

    let mut filtered: Vec<SoftwareItem> = all_items
        .into_iter()
        .filter(|item| matches_filters(item, &request))
        .collect();

    sort_items(&mut filtered, request.sort_by, request.sort_direction);

    let total_count = filtered.len();
    let start = ((page - 1) as usize).saturating_mul(page_size as usize);
    let items: Vec<SoftwareItem> = filtered
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .collect();

    Ok(ListSoftwareItemsResponse {
        items,
        total_count,
        page,
        page_size,
    })
}

fn matches_filters(item: &SoftwareItem, request: &ListSoftwareItemsRequest) -> bool {
    if let Some(search) = request
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let needle = search.to_lowercase();
        let haystack = [
            item.display_name.to_lowercase(),
            item.package_name.to_lowercase(),
            item.description.clone().unwrap_or_default().to_lowercase(),
        ]
        .join(" ");
        if !haystack.contains(&needle) {
            return false;
        }
    }

    if let Some(categories) = &request.categories {
        if !categories.is_empty() && !categories.contains(&item.category) {
            return false;
        }
    }

    if let Some(managers) = &request.package_managers {
        if !managers.is_empty() && !managers.contains(&item.package_manager) {
            return false;
        }
    }

    if let Some(scopes) = &request.scopes {
        if !scopes.is_empty() && !scopes.contains(&item.scope) {
            return false;
        }
    }

    if let Some(reasons) = &request.installation_reasons {
        if !reasons.is_empty() && !reasons.contains(&item.installation_reason) {
            return false;
        }
    }

    if request.update_available_only == Some(true) && !item.update_available {
        return false;
    }

    if let Some(min_confidence) = request.min_confidence {
        if item.classification_confidence < min_confidence {
            return false;
        }
    }

    true
}

fn sort_items(
    items: &mut [SoftwareItem],
    sort_by: Option<SortField>,
    direction: Option<SortDirection>,
) {
    let field = sort_by.unwrap_or(SortField::DisplayName);
    let direction = direction.unwrap_or(SortDirection::Ascending);

    items.sort_by(|a, b| {
        let ordering = match field {
            SortField::DisplayName => a
                .display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase()),
            SortField::Category => format!("{:?}", a.category).cmp(&format!("{:?}", b.category)),
            SortField::PackageManager => {
                format!("{:?}", a.package_manager).cmp(&format!("{:?}", b.package_manager))
            }
            SortField::Version => a
                .version
                .clone()
                .unwrap_or_default()
                .cmp(&b.version.clone().unwrap_or_default()),
            SortField::InstalledSize => a
                .installed_size_bytes
                .unwrap_or(0)
                .cmp(&b.installed_size_bytes.unwrap_or(0)),
            SortField::Confidence => a
                .classification_confidence
                .cmp(&b.classification_confidence),
        };

        match direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    });
}

pub async fn get_software_item_impl(
    state: &AppState,
    id: &str,
) -> Result<Option<SoftwareItem>, CommandError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(CommandError::invalid_request("id must not be empty"));
    }

    let repository = Arc::clone(&state.repository);
    let items = run_blocking(move || repository.latest_items()).await?;
    Ok(items.into_iter().find(|item| item.id == id))
}

pub async fn list_duplicate_groups_impl(
    state: &AppState,
) -> Result<Vec<DuplicateGroup>, CommandError> {
    let repository = Arc::clone(&state.repository);
    run_blocking(move || match repository.latest_scan_id()? {
        Some(scan_id) => repository.list_duplicate_groups(scan_id),
        None => Ok(Vec::new()),
    })
    .await
}

pub async fn list_provider_warnings_impl(
    state: &AppState,
) -> Result<Vec<ProviderWarningsResponse>, CommandError> {
    let repository = Arc::clone(&state.repository);
    let results = run_blocking(move || match repository.latest_scan_id()? {
        Some(scan_id) => repository.list_provider_results(scan_id),
        None => Ok(Vec::new()),
    })
    .await?;

    Ok(results
        .into_iter()
        .filter(|result| !result.warnings.is_empty() || result.error.is_some())
        .map(|result| ProviderWarningsResponse {
            provider_id: result.provider_id,
            warnings: result.warnings,
            error: result.error,
        })
        .collect())
}

pub async fn rebuild_cache_impl(state: &AppState) -> Result<(), CommandError> {
    let repository = Arc::clone(&state.repository);
    run_blocking(move || repository.rebuild_cache()).await
}

#[tauri::command]
pub async fn get_inventory_summary(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Option<InventorySummary>, CommandError> {
    get_inventory_summary_impl(state.inner()).await
}

#[tauri::command]
pub async fn list_software_items(
    state: tauri::State<'_, Arc<AppState>>,
    request: ListSoftwareItemsRequest,
) -> Result<ListSoftwareItemsResponse, CommandError> {
    list_software_items_impl(state.inner(), request).await
}

#[tauri::command]
pub async fn get_software_item(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> Result<Option<SoftwareItem>, CommandError> {
    get_software_item_impl(state.inner(), &id).await
}

#[tauri::command]
pub async fn list_duplicate_groups(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<DuplicateGroup>, CommandError> {
    list_duplicate_groups_impl(state.inner()).await
}

#[tauri::command]
pub async fn list_provider_warnings(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<ProviderWarningsResponse>, CommandError> {
    list_provider_warnings_impl(state.inner()).await
}

#[tauri::command]
pub async fn rebuild_cache(state: tauri::State<'_, Arc<AppState>>) -> Result<(), CommandError> {
    rebuild_cache_impl(state.inner()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::{state_after_scan, test_state};
    use crate::domain::{
        ClassificationConfidence, InstallationScope, PackageManager, SoftwareCategory,
    };
    use crate::providers::mock::MockInventoryProvider;

    fn item(
        id: &str,
        display_name: &str,
        category: SoftwareCategory,
        manager: PackageManager,
    ) -> SoftwareItem {
        let mut item = SoftwareItem::new(id, id, display_name, manager);
        item.category = category;
        item
    }

    async fn state_with_scanned_items(items: Vec<SoftwareItem>) -> AppState {
        state_after_scan(vec![Box::new(
            MockInventoryProvider::new("apt").with_items(items),
        )])
        .await
    }

    #[tokio::test]
    async fn list_software_items_rejects_page_zero() {
        let state = test_state(vec![]);
        let result = list_software_items_impl(
            &state,
            ListSoftwareItemsRequest {
                page: Some(0),
                ..Default::default()
            },
        )
        .await;
        assert!(matches!(result, Err(e) if e.kind == "invalidRequest"));
    }

    #[tokio::test]
    async fn list_software_items_rejects_an_oversized_page_size() {
        let state = test_state(vec![]);
        let result = list_software_items_impl(
            &state,
            ListSoftwareItemsRequest {
                page_size: Some(MAX_PAGE_SIZE + 1),
                ..Default::default()
            },
        )
        .await;
        assert!(matches!(result, Err(e) if e.kind == "invalidRequest"));
    }

    #[tokio::test]
    async fn list_software_items_paginates_results() {
        let items = (0..5)
            .map(|i| {
                item(
                    &format!("apt:pkg{i}"),
                    &format!("Package {i}"),
                    SoftwareCategory::Application,
                    PackageManager::Apt,
                )
            })
            .collect();
        let state = state_with_scanned_items(items).await;

        let page1 = list_software_items_impl(
            &state,
            ListSoftwareItemsRequest {
                page_size: Some(2),
                ..Default::default()
            },
        )
        .await
        .expect("page 1");
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.total_count, 5);

        let page3 = list_software_items_impl(
            &state,
            ListSoftwareItemsRequest {
                page: Some(3),
                page_size: Some(2),
                ..Default::default()
            },
        )
        .await
        .expect("page 3");
        assert_eq!(page3.items.len(), 1);
    }

    #[tokio::test]
    async fn list_software_items_filters_by_search_across_multiple_fields() {
        let mut with_description = item(
            "apt:git",
            "Git",
            SoftwareCategory::CommandLineTool,
            PackageManager::Apt,
        );
        with_description.description = Some("distributed version control".to_string());
        let items = vec![
            with_description,
            item(
                "apt:firefox",
                "Firefox",
                SoftwareCategory::Application,
                PackageManager::Apt,
            ),
        ];
        let state = state_with_scanned_items(items).await;

        let result = list_software_items_impl(
            &state,
            ListSoftwareItemsRequest {
                search: Some("version control".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("search");

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].id, "apt:git");
    }

    #[tokio::test]
    async fn list_software_items_filters_by_category_and_manager() {
        let items = vec![
            item(
                "apt:git",
                "Git",
                SoftwareCategory::CommandLineTool,
                PackageManager::Apt,
            ),
            item(
                "flatpak:app",
                "App",
                SoftwareCategory::Application,
                PackageManager::Flatpak,
            ),
        ];
        let state = state_with_scanned_items(items).await;

        let result = list_software_items_impl(
            &state,
            ListSoftwareItemsRequest {
                categories: Some(vec![SoftwareCategory::CommandLineTool]),
                ..Default::default()
            },
        )
        .await
        .expect("filter");

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].id, "apt:git");
    }

    #[tokio::test]
    async fn list_software_items_sorts_descending_by_display_name() {
        let items = vec![
            item(
                "apt:a",
                "Alpha",
                SoftwareCategory::Application,
                PackageManager::Apt,
            ),
            item(
                "apt:z",
                "Zulu",
                SoftwareCategory::Application,
                PackageManager::Apt,
            ),
        ];
        let state = state_with_scanned_items(items).await;

        let result = list_software_items_impl(
            &state,
            ListSoftwareItemsRequest {
                sort_direction: Some(SortDirection::Descending),
                ..Default::default()
            },
        )
        .await
        .expect("sort");

        assert_eq!(result.items[0].display_name, "Zulu");
    }

    #[tokio::test]
    async fn get_software_item_rejects_an_empty_id() {
        let state = test_state(vec![]);
        let result = get_software_item_impl(&state, "  ").await;
        assert!(matches!(result, Err(e) if e.kind == "invalidRequest"));
    }

    #[tokio::test]
    async fn get_software_item_returns_none_when_not_found() {
        let state = state_with_scanned_items(vec![item(
            "apt:git",
            "Git",
            SoftwareCategory::CommandLineTool,
            PackageManager::Apt,
        )])
        .await;

        let result = get_software_item_impl(&state, "apt:does-not-exist")
            .await
            .expect("lookup");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_software_item_finds_an_existing_item() {
        let state = state_with_scanned_items(vec![item(
            "apt:git",
            "Git",
            SoftwareCategory::CommandLineTool,
            PackageManager::Apt,
        )])
        .await;

        let result = get_software_item_impl(&state, "apt:git")
            .await
            .expect("lookup")
            .expect("present");
        assert_eq!(result.display_name, "Git");
    }

    #[tokio::test]
    async fn rebuild_cache_clears_the_summary() {
        let state = state_with_scanned_items(vec![item(
            "apt:git",
            "Git",
            SoftwareCategory::CommandLineTool,
            PackageManager::Apt,
        )])
        .await;

        assert!(get_inventory_summary_impl(&state)
            .await
            .expect("summary")
            .is_some());

        rebuild_cache_impl(&state).await.expect("rebuild");

        assert!(get_inventory_summary_impl(&state)
            .await
            .expect("summary")
            .is_none());
    }

    #[test]
    fn update_available_only_filter_excludes_items_without_updates() {
        let mut without_update = item(
            "apt:a",
            "A",
            SoftwareCategory::Application,
            PackageManager::Apt,
        );
        without_update.update_available = false;
        let mut with_update = item(
            "apt:b",
            "B",
            SoftwareCategory::Application,
            PackageManager::Apt,
        );
        with_update.update_available = true;

        let request = ListSoftwareItemsRequest {
            update_available_only: Some(true),
            ..Default::default()
        };

        assert!(!matches_filters(&without_update, &request));
        assert!(matches_filters(&with_update, &request));
    }

    #[test]
    fn min_confidence_filter_excludes_lower_confidence_items() {
        let mut low = item(
            "apt:a",
            "A",
            SoftwareCategory::Application,
            PackageManager::Apt,
        );
        low.classification_confidence = ClassificationConfidence::Low;
        let mut high = item(
            "apt:b",
            "B",
            SoftwareCategory::Application,
            PackageManager::Apt,
        );
        high.classification_confidence = ClassificationConfidence::High;

        let request = ListSoftwareItemsRequest {
            min_confidence: Some(ClassificationConfidence::High),
            ..Default::default()
        };

        assert!(!matches_filters(&low, &request));
        assert!(matches_filters(&high, &request));
    }

    #[test]
    fn scope_filter_matches_only_listed_scopes() {
        let mut user_item = item(
            "apt:a",
            "A",
            SoftwareCategory::Application,
            PackageManager::Apt,
        );
        user_item.scope = InstallationScope::User;
        let mut system_item = item(
            "apt:b",
            "B",
            SoftwareCategory::Application,
            PackageManager::Apt,
        );
        system_item.scope = InstallationScope::System;

        let request = ListSoftwareItemsRequest {
            scopes: Some(vec![InstallationScope::User]),
            ..Default::default()
        };

        assert!(matches_filters(&user_item, &request));
        assert!(!matches_filters(&system_item, &request));
    }

    /// Not a micro-benchmark harness (no criterion dependency) -- these
    /// generously-bounded timing assertions exist so a future change that
    /// makes the in-memory filter/sort/paginate path accidentally
    /// quadratic (or removes an obvious optimization) fails CI, and so the
    /// printed timings (run with `--nocapture`) are the real numbers behind
    /// `docs/PERFORMANCE.md`'s "list_software_items at N items" claims,
    /// not guesses.
    mod performance {
        use super::*;
        use std::time::Instant;

        const SYNTHETIC_ITEM_COUNT: usize = 5000;

        async fn state_with_large_scan(items: Vec<SoftwareItem>) -> AppState {
            state_after_scan(vec![Box::new(
                MockInventoryProvider::new("apt").with_items(items),
            )])
            .await
        }

        fn synthetic_items(count: usize) -> Vec<SoftwareItem> {
            let categories = [
                SoftwareCategory::Application,
                SoftwareCategory::Library,
                SoftwareCategory::CommandLineTool,
                SoftwareCategory::DevelopmentPackage,
                SoftwareCategory::Font,
            ];
            let managers = [
                PackageManager::Apt,
                PackageManager::Flatpak,
                PackageManager::Snap,
                PackageManager::Manual,
            ];

            (0..count)
                .map(|i| {
                    let mut it = item(
                        &format!("apt:pkg-{i}"),
                        &format!("Package {i}"),
                        categories[i % categories.len()],
                        managers[i % managers.len()],
                    );
                    it.description = Some(format!(
                        "A synthetic test package number {i} used for performance measurement"
                    ));
                    it.version = Some(format!("1.{i}.0"));
                    it.installed_size_bytes = Some((i as u64) * 1024);
                    it
                })
                .collect()
        }

        #[tokio::test]
        async fn list_software_items_stays_fast_at_thousands_of_items() {
            let state = state_with_large_scan(synthetic_items(SYNTHETIC_ITEM_COUNT)).await;

            let started = Instant::now();
            let response = list_software_items_impl(&state, ListSoftwareItemsRequest::default())
                .await
                .expect("list");
            let elapsed = started.elapsed();

            println!(
                "list_software_items_impl (page 1, no filters) over {SYNTHETIC_ITEM_COUNT} items: {elapsed:?}"
            );
            assert_eq!(response.total_count, SYNTHETIC_ITEM_COUNT);
            assert!(
                elapsed.as_millis() < 500,
                "list_software_items_impl took {elapsed:?}, expected well under 500ms for {SYNTHETIC_ITEM_COUNT} items"
            );
        }

        #[tokio::test]
        async fn searching_with_no_matches_stays_fast_at_thousands_of_items() {
            let state = state_with_large_scan(synthetic_items(SYNTHETIC_ITEM_COUNT)).await;

            let started = Instant::now();
            let response = list_software_items_impl(
                &state,
                ListSoftwareItemsRequest {
                    search: Some("this-string-matches-nothing".to_string()),
                    ..Default::default()
                },
            )
            .await
            .expect("list");
            let elapsed = started.elapsed();

            println!(
                "list_software_items_impl (worst-case non-matching search) over {SYNTHETIC_ITEM_COUNT} items: {elapsed:?}"
            );
            assert_eq!(response.total_count, 0);
            assert!(
                elapsed.as_millis() < 500,
                "worst-case search took {elapsed:?}, expected well under 500ms for {SYNTHETIC_ITEM_COUNT} items"
            );
        }

        #[tokio::test]
        async fn get_inventory_summary_stays_fast_at_thousands_of_items() {
            let state = state_with_large_scan(synthetic_items(SYNTHETIC_ITEM_COUNT)).await;

            let started = Instant::now();
            let summary = get_inventory_summary_impl(&state).await.expect("summary");
            let elapsed = started.elapsed();

            println!("get_inventory_summary_impl over {SYNTHETIC_ITEM_COUNT} items: {elapsed:?}");
            assert_eq!(
                summary.expect("summary present").total_items,
                SYNTHETIC_ITEM_COUNT
            );
            assert!(
                elapsed.as_millis() < 200,
                "get_inventory_summary_impl took {elapsed:?}, expected well under 200ms"
            );
        }
    }
}
