//! Provider layer: the common interface every inventory source (APT,
//! Flatpak, desktop entries, fonts, AppImage, manual detection, ...)
//! implements. See `docs/ARCHITECTURE.md` §2.2 and §4.

pub mod appimage;
pub mod apt;
pub mod desktop;
pub mod dpkg_ownership;
pub mod flatpak;
pub mod fonts;
pub mod manual;
#[cfg(test)]
pub mod mock;

use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use tokio_util::sync::CancellationToken;

use crate::domain::{ProviderInventory, ProviderStatus};

/// Stable identifier for a provider (e.g. `"apt"`, `"flatpak"`,
/// `"desktop"`). Not tied to [`crate::domain::PackageManager`] because some
/// providers (desktop entries, fonts) aren't themselves package managers —
/// they discover items later associated with an owning package manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProviderId(&'static str);

impl ProviderId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Static descriptive information about a provider, independent of any
/// particular scan run.
#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    pub id: ProviderId,
    pub display_name: &'static str,
    pub description: &'static str,
}

/// Per-scan context passed to [`InventoryProvider::scan`].
///
/// Carries the timeout budget and a cancellation token so a provider can
/// (a) be forcibly timed out by the orchestrator even if it never checks
/// `cancellation`, and (b) cooperatively abort early between expensive
/// internal stages and still return whatever partial data it already has.
/// See `docs/ARCHITECTURE.md` §13 and `docs/DECISIONS.md` ADR-0007.
#[derive(Debug, Clone)]
pub struct ScanContext {
    pub timeout: Duration,
    pub cancellation: CancellationToken,
    /// When false, providers should skip expensive optional data (e.g. full
    /// dependency graphs, file lists) in favor of a fast basic scan. See
    /// `docs/ARCHITECTURE.md` §12 (staged/lazy scanning).
    pub include_expensive_details: bool,
}

impl ScanContext {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            cancellation: CancellationToken::new(),
            include_expensive_details: false,
        }
    }

    pub fn with_expensive_details(mut self, include: bool) -> Self {
        self.include_expensive_details = include;
        self
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }
}

/// Common interface every inventory source implements.
///
/// Implementations must never panic on malformed external input and must
/// never return a bare `Result::Err` that would propagate out of `scan` —
/// all failure information belongs inside the returned [`ProviderInventory`]
/// (as `warnings` for partial issues, `error` for a fatal one), so that one
/// misbehaving provider can never crash the whole application. See
/// `docs/ARCHITECTURE.md` §4 and `docs/SECURITY.md`.
#[async_trait]
pub trait InventoryProvider: Send + Sync {
    fn id(&self) -> ProviderId;

    fn metadata(&self) -> ProviderMetadata;

    /// Cheap check for whether this provider's underlying tool or data
    /// source is present on this system at all (e.g. is `flatpak` on
    /// `PATH`). Must not itself run an expensive scan.
    async fn is_available(&self) -> bool;

    /// Run a full scan. Always returns a [`ProviderInventory`], even on
    /// fatal failure — a failure is data, not a crash.
    async fn scan(&self, ctx: &ScanContext) -> ProviderInventory;
}

/// Runs `provider.scan(ctx)` under an orchestration-level timeout
/// (`ctx.timeout`), in addition to whatever timeout the provider applies
/// internally to its own subprocess calls. If the provider does not
/// complete in time, returns a synthesized [`ProviderInventory`] with
/// [`ProviderStatus::TimedOut`] rather than letting the scan hang
/// indefinitely. See `docs/DECISIONS.md` ADR-0007.
pub async fn run_provider_scan(
    provider: &dyn InventoryProvider,
    ctx: &ScanContext,
) -> ProviderInventory {
    let started_at = Utc::now();

    match tokio::time::timeout(ctx.timeout, provider.scan(ctx)).await {
        Ok(inventory) => inventory,
        Err(_) => ProviderInventory::started(provider.id().as_str(), started_at)
            .finish(Utc::now(), ProviderStatus::TimedOut),
    }
}

#[cfg(test)]
mod tests {
    use super::mock::MockInventoryProvider;
    use super::*;
    use crate::domain::SoftwareItem;

    #[test]
    fn provider_id_displays_as_its_str() {
        let id = ProviderId::new("apt");
        assert_eq!(id.to_string(), "apt");
        assert_eq!(id.as_str(), "apt");
    }

    #[tokio::test]
    async fn run_provider_scan_returns_the_providers_result_when_within_budget() {
        let provider = MockInventoryProvider::new("apt").with_items(vec![SoftwareItem::new(
            "apt:git",
            "git",
            "Git",
            crate::domain::PackageManager::Apt,
        )]);
        let ctx = ScanContext::new(Duration::from_secs(5));

        let result = run_provider_scan(&provider, &ctx).await;

        assert_eq!(result.items.len(), 1);
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn run_provider_scan_times_out_a_slow_provider() {
        let provider = MockInventoryProvider::new("flatpak").with_delay(Duration::from_millis(200));
        let ctx = ScanContext::new(Duration::from_millis(20));

        let result = run_provider_scan(&provider, &ctx).await;

        assert_eq!(result.status, ProviderStatus::TimedOut);
        assert!(result.items.is_empty());
    }

    #[tokio::test]
    async fn cancellation_token_lets_a_provider_abort_early() {
        let provider = MockInventoryProvider::new("appimage").with_delay(Duration::from_secs(5));
        let ctx = ScanContext::new(Duration::from_secs(30));
        ctx.cancellation.cancel();

        let result = run_provider_scan(&provider, &ctx).await;

        assert_eq!(result.status, ProviderStatus::Cancelled);
    }

    #[tokio::test]
    async fn mock_provider_reports_unavailable() {
        let provider = MockInventoryProvider::new("snap").unavailable();
        assert!(!provider.is_available().await);
    }
}
