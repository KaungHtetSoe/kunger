//! The unified inventory service: runs every registered provider
//! independently, merges their results, detects cross-manager duplicates,
//! and produces an [`InventorySummary`]. See `docs/ARCHITECTURE.md` §2.4.

mod duplicates;
mod merge;

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;

use crate::domain::{
    DuplicateGroup, InventoryStatus, InventorySummary, PackageManager, ProviderInventory,
    ProviderStatus, SoftwareCategory, SoftwareItem,
};
use crate::providers::{
    run_provider_scan, InventoryProvider, ProviderId, ProviderMetadata, ScanContext,
};

/// The full result of one unified inventory scan.
pub struct ScanResult {
    /// Final, merged, classified item list — safe to hand directly to the
    /// UI or persistence layer.
    pub items: Vec<SoftwareItem>,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub summary: InventorySummary,
    /// Raw, unmerged per-provider results, preserved for provider-level
    /// warning/status display (`docs/ARCHITECTURE.md` §10
    /// `list_provider_warnings`).
    pub provider_results: Vec<ProviderInventory>,
}

/// Orchestrates the registered [`InventoryProvider`]s.
///
/// Provider **registration order matters**: [`merge::merge_by_id`] treats
/// the first provider to report a given item id as the authoritative base
/// record and every later same-id report as enrichment. Register the APT
/// provider before the desktop and font providers, which deliberately
/// reuse `apt:{package}` ids for ownership-known entries (ADR-0012).
pub struct InventoryService {
    providers: Vec<Box<dyn InventoryProvider>>,
}

impl InventoryService {
    pub fn new(providers: Vec<Box<dyn InventoryProvider>>) -> Self {
        Self { providers }
    }

    /// Builds a service with every provider Kunger ships, in the
    /// registration order required by the merge contract above.
    pub fn with_default_providers() -> Self {
        Self::new(vec![
            Box::new(crate::providers::apt::AptProvider::new()),
            Box::new(crate::providers::desktop::DesktopProvider::new()),
            Box::new(crate::providers::fonts::FontProvider::new()),
            Box::new(crate::providers::flatpak::FlatpakProvider::new()),
            Box::new(crate::providers::appimage::AppImageProvider::new()),
            Box::new(crate::providers::manual::ManualSoftwareProvider::new()),
        ])
    }

    /// Cheap availability check per provider, independent of running a
    /// full scan — e.g. for a `get_provider_status` IPC command
    /// (`docs/ARCHITECTURE.md` §10) shown before the user triggers a scan.
    pub async fn provider_statuses(&self) -> Vec<(ProviderId, bool)> {
        let mut statuses = Vec::with_capacity(self.providers.len());
        for provider in &self.providers {
            statuses.push((provider.id(), provider.is_available().await));
        }
        statuses
    }

    /// Like [`Self::provider_statuses`], but includes each provider's
    /// static [`ProviderMetadata`] (display name, description) — what the
    /// `get_provider_status` IPC command actually needs to render a
    /// provider list to the user.
    pub async fn provider_status_details(&self) -> Vec<(ProviderMetadata, bool)> {
        let mut details = Vec::with_capacity(self.providers.len());
        for provider in &self.providers {
            details.push((provider.metadata(), provider.is_available().await));
        }
        details
    }

    /// Runs every registered provider concurrently, each under
    /// `per_provider_timeout` (enforced independently per provider, on
    /// top of whatever internal timeout the provider applies to its own
    /// subprocess calls — ADR-0007), sharing one `cancellation` token so
    /// cancelling the scan stops every provider at once. One provider
    /// failing, timing out, or being unavailable never prevents the
    /// others from contributing their results.
    pub async fn scan(
        &self,
        per_provider_timeout: Duration,
        cancellation: CancellationToken,
    ) -> ScanResult {
        let started_at = Utc::now();

        let ctx = ScanContext {
            timeout: per_provider_timeout,
            cancellation,
            include_expensive_details: false,
        };

        let provider_results: Vec<ProviderInventory> = futures::future::join_all(
            self.providers
                .iter()
                .map(|provider| run_provider_scan(provider.as_ref(), &ctx)),
        )
        .await;

        let all_items: Vec<SoftwareItem> = provider_results
            .iter()
            .flat_map(|result| result.items.clone())
            .collect();

        let merged_items = merge::merge_by_id(all_items);
        let duplicate_groups = duplicates::detect_duplicates(&merged_items);

        let completed_at = Utc::now();
        let summary = build_summary(
            &provider_results,
            &merged_items,
            &duplicate_groups,
            started_at,
            completed_at,
        );

        ScanResult {
            items: merged_items,
            duplicate_groups,
            summary,
            provider_results,
        }
    }
}

fn build_summary(
    provider_results: &[ProviderInventory],
    items: &[SoftwareItem],
    duplicate_groups: &[DuplicateGroup],
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
) -> InventorySummary {
    let mut items_by_category: HashMap<SoftwareCategory, usize> = HashMap::new();
    let mut items_by_package_manager: HashMap<PackageManager, usize> = HashMap::new();

    for item in items {
        *items_by_category.entry(item.category).or_insert(0) += 1;
        *items_by_package_manager
            .entry(item.package_manager)
            .or_insert(0) += 1;
    }

    let providers_with_warnings: Vec<String> = provider_results
        .iter()
        .filter(|result| !result.warnings.is_empty())
        .map(|result| result.provider_id.clone())
        .collect();

    let providers_with_errors: Vec<String> = provider_results
        .iter()
        .filter(|result| result.error.is_some())
        .map(|result| result.provider_id.clone())
        .collect();

    let any_succeeded = provider_results.iter().any(ProviderInventory::is_success);
    let any_failed = provider_results
        .iter()
        .any(|result| result.status == ProviderStatus::Failed);
    let any_degraded = provider_results.iter().any(|result| {
        matches!(
            result.status,
            ProviderStatus::PartialSuccess | ProviderStatus::TimedOut
        )
    });

    let status = if any_failed && !any_succeeded {
        InventoryStatus::Failed
    } else if any_failed {
        InventoryStatus::PartiallyFailed
    } else if any_degraded || !providers_with_warnings.is_empty() {
        InventoryStatus::CompletedWithWarnings
    } else {
        InventoryStatus::Completed
    };

    let scan_duration_ms = (completed_at - started_at).num_milliseconds().max(0) as u64;

    InventorySummary {
        status,
        total_items: items.len(),
        items_by_category,
        items_by_package_manager,
        providers_with_warnings,
        providers_with_errors,
        duplicate_group_count: duplicate_groups.len(),
        last_scan_started_at: Some(started_at),
        last_scan_completed_at: Some(completed_at),
        scan_duration_ms: Some(scan_duration_ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProviderError, ProviderWarning};
    use crate::providers::mock::MockInventoryProvider;

    fn item(id: &str, display_name: &str, manager: PackageManager) -> SoftwareItem {
        // Mirrors real provider id conventions ("apt:firefox" -> package
        // name "firefox") so display-name-vs-package-name merge heuristics
        // behave the same way they would against real provider output.
        let package_name = id.rsplit(':').next().unwrap_or(id);
        SoftwareItem::new(id, package_name, display_name, manager)
    }

    #[tokio::test]
    async fn scan_combines_items_from_every_provider() {
        let service = InventoryService::new(vec![
            Box::new(MockInventoryProvider::new("apt").with_items(vec![item(
                "apt:git",
                "git",
                PackageManager::Apt,
            )])),
            Box::new(MockInventoryProvider::new("flatpak").with_items(vec![item(
                "flatpak:user:org.example.App",
                "Example App",
                PackageManager::Flatpak,
            )])),
        ]);

        let result = service
            .scan(Duration::from_secs(5), CancellationToken::new())
            .await;

        assert_eq!(result.items.len(), 2);
        assert_eq!(result.summary.total_items, 2);
        assert_eq!(result.summary.status, InventoryStatus::Completed);
    }

    #[tokio::test]
    async fn one_provider_failing_does_not_prevent_others_from_contributing() {
        let service = InventoryService::new(vec![
            Box::new(MockInventoryProvider::new("apt").with_items(vec![item(
                "apt:git",
                "git",
                PackageManager::Apt,
            )])),
            Box::new(
                MockInventoryProvider::new("broken")
                    .with_error(ProviderError::CommandNotFound("broken-tool".to_string())),
            ),
        ]);

        let result = service
            .scan(Duration::from_secs(5), CancellationToken::new())
            .await;

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.summary.status, InventoryStatus::PartiallyFailed);
        assert!(result
            .summary
            .providers_with_errors
            .contains(&"broken".to_string()));
    }

    #[tokio::test]
    async fn every_provider_failing_reports_failed_status_with_zero_items() {
        let service = InventoryService::new(vec![Box::new(
            MockInventoryProvider::new("broken")
                .with_error(ProviderError::CommandNotFound("x".to_string())),
        )]);

        let result = service
            .scan(Duration::from_secs(5), CancellationToken::new())
            .await;

        assert!(result.items.is_empty());
        assert_eq!(result.summary.status, InventoryStatus::Failed);
    }

    #[tokio::test]
    async fn provider_warnings_produce_completed_with_warnings_status() {
        let service = InventoryService::new(vec![Box::new(
            MockInventoryProvider::new("desktop")
                .with_items(vec![item("desktop:app", "App", PackageManager::Manual)])
                .with_warning(ProviderWarning::new("one malformed .desktop file")),
        )]);

        let result = service
            .scan(Duration::from_secs(5), CancellationToken::new())
            .await;

        assert_eq!(
            result.summary.status,
            InventoryStatus::CompletedWithWarnings
        );
        assert!(result
            .summary
            .providers_with_warnings
            .contains(&"desktop".to_string()));
        // Raw per-provider warnings are preserved for later display.
        assert_eq!(result.provider_results[0].warnings.len(), 1);
    }

    #[tokio::test]
    async fn same_id_items_from_different_providers_are_merged_not_duplicated() {
        let mut apt_item = item("apt:firefox", "firefox", PackageManager::Apt);
        apt_item.version = Some("128.0".to_string());

        let mut desktop_item = item("apt:firefox", "Firefox", PackageManager::Apt);
        desktop_item.desktop_file_paths =
            vec!["/usr/share/applications/firefox.desktop".to_string()];

        let service = InventoryService::new(vec![
            Box::new(MockInventoryProvider::new("apt").with_items(vec![apt_item])),
            Box::new(MockInventoryProvider::new("desktop").with_items(vec![desktop_item])),
        ]);

        let result = service
            .scan(Duration::from_secs(5), CancellationToken::new())
            .await;

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].display_name, "Firefox");
        assert_eq!(result.items[0].version.as_deref(), Some("128.0"));
        assert_eq!(
            result.items[0].desktop_file_paths,
            vec!["/usr/share/applications/firefox.desktop".to_string()]
        );
    }

    #[tokio::test]
    async fn cross_manager_duplicates_are_detected_after_merging() {
        let service = InventoryService::new(vec![
            Box::new(MockInventoryProvider::new("apt").with_items(vec![item(
                "apt:firefox",
                "Firefox",
                PackageManager::Apt,
            )])),
            Box::new(MockInventoryProvider::new("flatpak").with_items(vec![item(
                "flatpak:user:org.mozilla.firefox",
                "Firefox",
                PackageManager::Flatpak,
            )])),
        ]);

        let result = service
            .scan(Duration::from_secs(5), CancellationToken::new())
            .await;

        assert_eq!(result.items.len(), 2);
        assert_eq!(result.duplicate_groups.len(), 1);
        assert_eq!(result.summary.duplicate_group_count, 1);
    }

    #[tokio::test]
    async fn cancellation_token_is_shared_across_every_provider() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        let service = InventoryService::new(vec![
            Box::new(MockInventoryProvider::new("apt")),
            Box::new(MockInventoryProvider::new("flatpak")),
        ]);

        let result = service.scan(Duration::from_secs(30), cancellation).await;

        assert!(result
            .provider_results
            .iter()
            .all(|r| r.status == ProviderStatus::Cancelled));
    }

    #[tokio::test]
    async fn a_slow_provider_times_out_without_blocking_the_others() {
        let service = InventoryService::new(vec![
            Box::new(MockInventoryProvider::new("slow").with_delay(Duration::from_millis(500))),
            Box::new(MockInventoryProvider::new("fast").with_items(vec![item(
                "apt:git",
                "git",
                PackageManager::Apt,
            )])),
        ]);

        let start = std::time::Instant::now();
        let result = service
            .scan(Duration::from_millis(50), CancellationToken::new())
            .await;
        let elapsed = start.elapsed();

        // Both ran concurrently under a 50ms per-provider timeout, so this
        // should complete quickly even though "slow" was configured for
        // 500ms -- proof the fast provider wasn't blocked waiting on it.
        assert!(
            elapsed < Duration::from_millis(400),
            "providers did not run concurrently: {elapsed:?}"
        );
        assert_eq!(result.items.len(), 1);
        let slow_result = result
            .provider_results
            .iter()
            .find(|r| r.provider_id == "slow")
            .expect("slow result present");
        assert_eq!(slow_result.status, ProviderStatus::TimedOut);
    }

    #[tokio::test]
    async fn provider_statuses_reports_availability_without_running_a_scan() {
        let service = InventoryService::new(vec![
            Box::new(MockInventoryProvider::new("available")),
            Box::new(MockInventoryProvider::new("unavailable").unavailable()),
        ]);

        let statuses = service.provider_statuses().await;

        assert_eq!(statuses.len(), 2);
        assert!(statuses
            .iter()
            .any(|(id, available)| id.as_str() == "available" && *available));
        assert!(statuses
            .iter()
            .any(|(id, available)| id.as_str() == "unavailable" && !*available));
    }

    #[tokio::test]
    async fn empty_scan_produces_a_completed_summary_with_zero_items() {
        let service = InventoryService::new(Vec::new());

        let result = service
            .scan(Duration::from_secs(5), CancellationToken::new())
            .await;

        assert!(result.items.is_empty());
        assert_eq!(result.summary.status, InventoryStatus::Completed);
        assert_eq!(result.summary.total_items, 0);
    }
}
