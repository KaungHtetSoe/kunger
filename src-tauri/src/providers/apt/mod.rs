//! The APT/dpkg inventory provider for Debian and Ubuntu. See
//! `docs/ARCHITECTURE.md` §2.2 and `docs/SECURITY.md`.

mod parser;

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use chrono::Utc;

use crate::classification::{classify, Evidence};
use crate::domain::{
    InstallationReason, InstallationScope, PackageManager, ProviderError, ProviderInventory,
    ProviderStatus, ProviderWarning, SoftwareItem,
};
use crate::process::{CommandSpec, ProcessRunner, RunError};
use crate::providers::{InventoryProvider, ProviderId, ProviderMetadata, ScanContext};

const APT_PROVIDER_ID: ProviderId = ProviderId::new("apt");

/// Inventories packages known to dpkg/APT.
///
/// Scanning proceeds in stages, per `docs/ARCHITECTURE.md` §12:
/// 1. Fast basic inventory via a single batched `dpkg-query` call (fatal if
///    this fails — without it there is no inventory at all).
/// 2. Manual vs. automatic installation state via `apt-mark showmanual`
///    (best-effort: failure here degrades `installation_reason` to
///    `Unknown` for every item rather than failing the whole scan).
/// 3. Update availability via `apt list --upgradable` (best-effort, same
///    degrade-not-fail treatment).
///
/// Per-item expensive detail (full dependency graphs, owned file lists) is
/// intentionally deferred to a future lazy-loading stage (see
/// `docs/ARCHITECTURE.md` §10 `get_software_item`) rather than fetched
/// eagerly here, since that would mean one subprocess per package.
pub struct AptProvider {
    runner: ProcessRunner,
    dpkg_query_bin: String,
    apt_mark_bin: String,
    apt_bin: String,
}

impl AptProvider {
    pub fn new() -> Self {
        Self {
            runner: ProcessRunner::default(),
            dpkg_query_bin: "dpkg-query".to_string(),
            apt_mark_bin: "apt-mark".to_string(),
            apt_bin: "apt".to_string(),
        }
    }

    /// Test-only constructor letting the underlying binary names be
    /// overridden, so scan orchestration can be exercised deterministically
    /// without depending on whether the host machine actually has APT
    /// installed, or on what packages happen to be present. See
    /// `docs/PRODUCT_SPEC.md`'s requirement that provider tests never
    /// depend on the host's real package state.
    #[cfg(test)]
    fn with_binaries(
        runner: ProcessRunner,
        dpkg_query_bin: impl Into<String>,
        apt_mark_bin: impl Into<String>,
        apt_bin: impl Into<String>,
    ) -> Self {
        Self {
            runner,
            dpkg_query_bin: dpkg_query_bin.into(),
            apt_mark_bin: apt_mark_bin.into(),
            apt_bin: apt_bin.into(),
        }
    }

    async fn fetch_manual_packages(
        &self,
        warnings: &mut Vec<ProviderWarning>,
    ) -> Option<HashSet<String>> {
        let spec = CommandSpec::new(&self.apt_mark_bin).arg("showmanual");
        match self.runner.run(&spec).await {
            Ok(output) => Some(parser::parse_manual_packages(&output.stdout)),
            Err(error) => {
                warnings.push(ProviderWarning::new(format!(
                    "could not determine manual vs. automatic installation state via apt-mark \
                     ({error}); installation reason will be left unknown for all items"
                )));
                None
            }
        }
    }

    async fn fetch_upgradable(
        &self,
        warnings: &mut Vec<ProviderWarning>,
    ) -> HashMap<String, String> {
        let spec = CommandSpec::new(&self.apt_bin).args(["list", "--upgradable"]);
        match self.runner.run(&spec).await {
            Ok(output) => {
                let mut parse_warnings = Vec::new();
                let upgradable = parser::parse_upgradable(&output.stdout, &mut parse_warnings);
                warnings.extend(parse_warnings.into_iter().map(ProviderWarning::new));
                upgradable
            }
            Err(error) => {
                warnings.push(ProviderWarning::new(format!(
                    "could not determine available updates via \"apt list --upgradable\" ({error})"
                )));
                HashMap::new()
            }
        }
    }
}

impl Default for AptProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InventoryProvider for AptProvider {
    fn id(&self) -> ProviderId {
        APT_PROVIDER_ID
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: APT_PROVIDER_ID,
            display_name: "APT / dpkg",
            description: "Packages installed via APT and tracked by dpkg.",
        }
    }

    async fn is_available(&self) -> bool {
        let spec = CommandSpec::new(&self.dpkg_query_bin).arg("--version");
        self.runner.run(&spec).await.is_ok()
    }

    async fn scan(&self, ctx: &ScanContext) -> ProviderInventory {
        let started_at = Utc::now();
        let base = ProviderInventory::started(APT_PROVIDER_ID.as_str(), started_at);

        if ctx.is_cancelled() {
            return base.finish(Utc::now(), ProviderStatus::Cancelled);
        }

        let dpkg_spec = CommandSpec::new(&self.dpkg_query_bin)
            .arg("-W")
            .arg("--showformat")
            .arg(parser::DPKG_QUERY_FORMAT);

        let dpkg_output = match self.runner.run(&dpkg_spec).await {
            Ok(output) => output,
            Err(error) => {
                let mut failed = base.finish(Utc::now(), ProviderStatus::Failed);
                failed.error = Some(map_run_error(&error));
                return failed;
            }
        };

        let mut warnings: Vec<ProviderWarning> = Vec::new();
        let mut parse_warnings = Vec::new();
        let records = parser::parse_dpkg_query(&dpkg_output.stdout, &mut parse_warnings);
        warnings.extend(parse_warnings.into_iter().map(ProviderWarning::new));

        if ctx.is_cancelled() {
            return base.finish(Utc::now(), ProviderStatus::Cancelled);
        }

        let manual_packages = self.fetch_manual_packages(&mut warnings).await;

        if ctx.is_cancelled() {
            return base.finish(Utc::now(), ProviderStatus::Cancelled);
        }

        let upgradable = self.fetch_upgradable(&mut warnings).await;

        let items = build_items(&records, manual_packages.as_ref(), &upgradable);

        let status = if warnings.is_empty() {
            ProviderStatus::Success
        } else {
            ProviderStatus::PartialSuccess
        };

        let mut inventory = base.finish(Utc::now(), status);
        inventory.items = items;
        inventory.warnings = warnings;
        inventory
    }
}

fn map_run_error(error: &RunError) -> ProviderError {
    match error {
        RunError::NotFound(cmd) => ProviderError::CommandNotFound(cmd.clone()),
        RunError::TimedOut(cmd, duration) => {
            ProviderError::Timeout(format!("{cmd} timed out after {duration:?}"))
        }
        RunError::NonZeroExit(cmd, code, stderr) => {
            ProviderError::Other(format!("{cmd} exited with status {code}: {stderr}"))
        }
        RunError::OutputTooLarge(cmd, limit) => {
            ProviderError::Other(format!("{cmd} output exceeded the {limit} byte limit"))
        }
        RunError::SpawnFailed(cmd, message) => {
            ProviderError::Other(format!("failed to run {cmd}: {message}"))
        }
    }
}

fn build_items(
    records: &[parser::DpkgRecord],
    manual_packages: Option<&HashSet<String>>,
    upgradable: &HashMap<String, String>,
) -> Vec<SoftwareItem> {
    records
        .iter()
        .map(|record| build_item(record, manual_packages, upgradable))
        .collect()
}

fn build_item(
    record: &parser::DpkgRecord,
    manual_packages: Option<&HashSet<String>>,
    upgradable: &HashMap<String, String>,
) -> SoftwareItem {
    let evidence = Evidence {
        package_name: record.package.clone(),
        debian_section: record.section.clone(),
        ..Default::default()
    };
    let classification = classify(&evidence);

    let mut item = SoftwareItem::new(
        format!("apt:{}", record.package),
        record.package.clone(),
        // dpkg has no separate "display name" field; the desktop-entry
        // provider (M3.1) enriches this with a proper display name where
        // an owning .desktop file is found during inventory merging.
        record.package.clone(),
        PackageManager::Apt,
    );

    item.description = record.summary.clone();
    item.version = Some(record.version.clone());
    item.architecture = Some(record.architecture.clone());
    item.category = classification.category;
    item.secondary_categories = classification.secondary_categories;
    item.classification_confidence = classification.confidence;
    item.classification_reasons = classification.reasons;
    item.package_section = record.section.clone();
    item.installed_size_bytes = record.installed_size_kb.map(|kb| kb * 1024);
    item.dependencies = record.dependencies.clone();
    item.homepage = record.homepage.clone();
    item.scope = InstallationScope::System;

    item.installation_reason = match manual_packages {
        Some(manual) if manual.contains(&record.package) => InstallationReason::Manual,
        Some(_) => InstallationReason::Automatic,
        None => InstallationReason::Unknown,
    };

    if let Some(available_version) = upgradable.get(&record.package) {
        item.update_available = true;
        item.available_version = Some(available_version.clone());
    }

    if let Some(maintainer) = &record.maintainer {
        item.metadata
            .insert("maintainer".to_string(), maintainer.clone());
    }

    item
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ClassificationConfidence, SoftwareCategory};
    use std::time::Duration;

    fn record(package: &str) -> parser::DpkgRecord {
        parser::DpkgRecord {
            package: package.to_string(),
            version: "1.0".to_string(),
            architecture: "amd64".to_string(),
            section: None,
            priority: None,
            installed_size_kb: None,
            maintainer: None,
            homepage: None,
            summary: None,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn build_item_sets_a_stable_provider_prefixed_id() {
        let item = build_item(&record("git"), None, &HashMap::new());
        assert_eq!(item.id, "apt:git");
        assert_eq!(item.package_manager, PackageManager::Apt);
        assert_eq!(item.scope, InstallationScope::System);
    }

    #[test]
    fn build_item_converts_installed_size_from_kib_to_bytes() {
        let mut r = record("git");
        r.installed_size_kb = Some(2048);

        let item = build_item(&r, None, &HashMap::new());

        assert_eq!(item.installed_size_bytes, Some(2048 * 1024));
    }

    #[test]
    fn build_item_marks_manual_when_present_in_the_manual_set() {
        let manual: HashSet<String> = ["git".to_string()].into_iter().collect();
        let item = build_item(&record("git"), Some(&manual), &HashMap::new());

        assert_eq!(item.installation_reason, InstallationReason::Manual);
    }

    #[test]
    fn build_item_marks_automatic_when_absent_from_a_known_manual_set() {
        let manual: HashSet<String> = ["git".to_string()].into_iter().collect();
        let item = build_item(&record("libssl3"), Some(&manual), &HashMap::new());

        assert_eq!(item.installation_reason, InstallationReason::Automatic);
    }

    #[test]
    fn build_item_leaves_installation_reason_unknown_when_apt_mark_was_unavailable() {
        // None (not an empty set) signals "we couldn't ask apt-mark at
        // all" -- must not be conflated with "we asked, and it said
        // automatic," which would silently mislabel every item.
        let item = build_item(&record("git"), None, &HashMap::new());

        assert_eq!(item.installation_reason, InstallationReason::Unknown);
    }

    #[test]
    fn build_item_wires_up_update_availability() {
        let upgradable: HashMap<String, String> = [("git".to_string(), "2.0".to_string())]
            .into_iter()
            .collect();
        let item = build_item(&record("git"), None, &upgradable);

        assert!(item.update_available);
        assert_eq!(item.available_version.as_deref(), Some("2.0"));
    }

    #[test]
    fn build_item_leaves_update_unavailable_when_not_listed() {
        let item = build_item(&record("git"), None, &HashMap::new());

        assert!(!item.update_available);
        assert_eq!(item.available_version, None);
    }

    #[test]
    fn build_item_classifies_using_the_debian_section() {
        let mut r = record("libfoo1");
        r.section = Some("libs".to_string());

        let item = build_item(&r, None, &HashMap::new());

        assert_eq!(item.category, SoftwareCategory::Library);
        assert_eq!(
            item.classification_confidence,
            ClassificationConfidence::High
        );
    }

    #[test]
    fn build_item_stores_maintainer_in_metadata() {
        let mut r = record("git");
        r.maintainer = Some("Someone <someone@example.com>".to_string());

        let item = build_item(&r, None, &HashMap::new());

        assert_eq!(
            item.metadata.get("maintainer").map(String::as_str),
            Some("Someone <someone@example.com>")
        );
    }

    #[tokio::test]
    async fn is_available_is_false_when_dpkg_query_binary_does_not_exist() {
        let provider = AptProvider::with_binaries(
            ProcessRunner::default(),
            "kunger-nonexistent-dpkg-query-xyz",
            "apt-mark",
            "apt",
        );

        assert!(!provider.is_available().await);
    }

    #[tokio::test]
    async fn is_available_is_true_for_a_real_binary_regardless_of_its_purpose() {
        // `true` exists on every Unix system Kunger targets or develops on
        // and always exits 0, so this deterministically exercises the
        // "found and ran successfully" path without depending on dpkg.
        let provider =
            AptProvider::with_binaries(ProcessRunner::default(), "true", "apt-mark", "apt");

        assert!(provider.is_available().await);
    }

    #[tokio::test]
    async fn scan_reports_cancelled_immediately_when_the_context_is_already_cancelled() {
        let provider = AptProvider::new();
        let ctx = ScanContext::new(Duration::from_secs(5));
        ctx.cancellation.cancel();

        let result = provider.scan(&ctx).await;

        assert_eq!(result.status, ProviderStatus::Cancelled);
        assert!(result.items.is_empty());
    }

    #[tokio::test]
    async fn scan_fails_gracefully_when_dpkg_query_is_missing() {
        let provider = AptProvider::with_binaries(
            ProcessRunner::default(),
            "kunger-nonexistent-dpkg-query-xyz",
            "apt-mark",
            "apt",
        );
        let ctx = ScanContext::new(Duration::from_secs(5));

        let result = provider.scan(&ctx).await;

        assert_eq!(result.status, ProviderStatus::Failed);
        assert!(result.items.is_empty());
        assert!(matches!(
            result.error,
            Some(ProviderError::CommandNotFound(_))
        ));
    }
}
