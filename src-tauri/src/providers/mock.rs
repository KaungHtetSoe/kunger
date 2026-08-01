//! Test-only mock [`InventoryProvider`], configurable for the
//! success/warning/error/timeout/cancellation scenarios the inventory
//! service (M4.1) needs to exercise without depending on any real provider.

use async_trait::async_trait;
use chrono::Utc;
use std::time::Duration;

use super::{InventoryProvider, ProviderId, ProviderMetadata, ScanContext};
use crate::domain::{
    ProviderError, ProviderInventory, ProviderStatus, ProviderWarning, SoftwareItem,
};

pub struct MockInventoryProvider {
    id: ProviderId,
    available: bool,
    delay: Option<Duration>,
    result: ProviderInventory,
}

impl MockInventoryProvider {
    pub fn new(id: &'static str) -> Self {
        let now = Utc::now();
        Self {
            id: ProviderId::new(id),
            available: true,
            delay: None,
            result: ProviderInventory::started(id, now).finish(now, ProviderStatus::Success),
        }
    }

    pub fn unavailable(mut self) -> Self {
        self.available = false;
        self
    }

    pub fn with_items(mut self, items: Vec<SoftwareItem>) -> Self {
        self.result.items = items;
        self
    }

    pub fn with_warning(mut self, warning: ProviderWarning) -> Self {
        self.result.warnings.push(warning);
        self.result.status = ProviderStatus::PartialSuccess;
        self
    }

    pub fn with_error(mut self, error: ProviderError) -> Self {
        self.result.error = Some(error);
        self.result.status = ProviderStatus::Failed;
        self
    }

    pub fn with_status(mut self, status: ProviderStatus) -> Self {
        self.result.status = status;
        self
    }

    /// Makes `scan` wait `delay` before returning, so tests can exercise
    /// timeout and cancellation handling.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
}

#[async_trait]
impl InventoryProvider for MockInventoryProvider {
    fn id(&self) -> ProviderId {
        self.id
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: self.id,
            display_name: "Mock Provider",
            description: "Test-only provider with configurable, canned results.",
        }
    }

    async fn is_available(&self) -> bool {
        self.available
    }

    async fn scan(&self, ctx: &ScanContext) -> ProviderInventory {
        if ctx.is_cancelled() {
            let now = Utc::now();
            return ProviderInventory::started(self.id.as_str(), now)
                .finish(now, ProviderStatus::Cancelled);
        }

        if let Some(delay) = self.delay {
            tokio::select! {
                () = tokio::time::sleep(delay) => {}
                () = ctx.cancellation.cancelled() => {
                    let now = Utc::now();
                    return ProviderInventory::started(self.id.as_str(), now)
                        .finish(now, ProviderStatus::Cancelled);
                }
            }
        }

        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_mock_reports_available_and_succeeds_with_no_items() {
        let provider = MockInventoryProvider::new("apt");
        let ctx = ScanContext::new(Duration::from_secs(1));

        assert!(provider.is_available().await);

        let result = provider.scan(&ctx).await;
        assert!(result.items.is_empty());
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn with_warning_marks_partial_success_but_keeps_items() {
        let provider = MockInventoryProvider::new("desktop")
            .with_items(vec![SoftwareItem::new(
                "desktop:firefox",
                "firefox",
                "Firefox",
                crate::domain::PackageManager::Apt,
            )])
            .with_warning(ProviderWarning::new("one malformed .desktop file"));

        let ctx = ScanContext::new(Duration::from_secs(1));
        let result = provider.scan(&ctx).await;

        assert_eq!(result.status, ProviderStatus::PartialSuccess);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.warnings.len(), 1);
    }

    #[tokio::test]
    async fn with_error_marks_failed_but_preserves_any_items_collected_so_far() {
        let provider = MockInventoryProvider::new("flatpak")
            .with_items(vec![SoftwareItem::new(
                "flatpak:org.example.App",
                "org.example.App",
                "Example App",
                crate::domain::PackageManager::Flatpak,
            )])
            .with_error(ProviderError::CommandNotFound("flatpak".into()));

        let ctx = ScanContext::new(Duration::from_secs(1));
        let result = provider.scan(&ctx).await;

        assert_eq!(result.status, ProviderStatus::Failed);
        assert!(!result.is_success());
        assert_eq!(result.items.len(), 1);
        assert!(result.error.is_some());
    }
}
