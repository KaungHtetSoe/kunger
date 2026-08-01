//! The Flatpak inventory provider. See `docs/ARCHITECTURE.md` §2.2 and
//! `docs/SECURITY.md`.

mod parser;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::{
    ClassificationConfidence, InstallationScope, PackageManager, ProviderError, ProviderInventory,
    ProviderStatus, ProviderWarning, SoftwareCategory, SoftwareItem,
};
use crate::process::{CommandSpec, ProcessRunner, RunError};
use crate::providers::{InventoryProvider, ProviderId, ProviderMetadata, ScanContext};

const FLATPAK_PROVIDER_ID: ProviderId = ProviderId::new("flatpak");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefKind {
    App,
    Runtime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    User,
    System,
}

impl Scope {
    fn flag(self) -> &'static str {
        match self {
            Scope::User => "--user",
            Scope::System => "--system",
        }
    }

    fn as_installation_scope(self) -> InstallationScope {
        match self {
            Scope::User => InstallationScope::User,
            Scope::System => InstallationScope::System,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Scope::User => "user",
            Scope::System => "system",
        }
    }
}

/// Inventories Flatpak applications and runtimes (extensions surface as
/// runtimes with a `flatpak_kind = "extension"` metadata tag — Kunger's
/// category taxonomy has no separate Extension bucket; see
/// `docs/PRODUCT_SPEC.md` §6).
///
/// Runs four independent, batched `flatpak list` calls (user/system ×
/// app/runtime) rather than one call per installed ref. Each call is
/// handled independently: if Flatpak itself isn't installed, the whole
/// provider reports `ProviderStatus::Unavailable` (not `Failed` — this is
/// an expected, common condition, not an error). If Flatpak is installed
/// but one particular call fails (e.g. a permission error scanning the
/// system installation), that failure becomes a warning and the other
/// three calls still contribute their results.
pub struct FlatpakProvider {
    runner: ProcessRunner,
    flatpak_bin: String,
}

impl FlatpakProvider {
    pub fn new() -> Self {
        Self {
            runner: ProcessRunner::default(),
            flatpak_bin: "flatpak".to_string(),
        }
    }

    /// Test-only constructor for overriding the `flatpak` binary name, so
    /// scan orchestration can be tested deterministically without
    /// depending on whether Flatpak is actually installed on the host.
    #[cfg(test)]
    fn with_binary(runner: ProcessRunner, flatpak_bin: impl Into<String>) -> Self {
        Self {
            runner,
            flatpak_bin: flatpak_bin.into(),
        }
    }

    async fn list(&self, scope: Scope, kind: RefKind) -> Result<String, RunError> {
        let kind_flag = match kind {
            RefKind::App => "--app",
            RefKind::Runtime => "--runtime",
        };

        let spec = CommandSpec::new(&self.flatpak_bin)
            .arg("list")
            .arg(scope.flag())
            .arg(kind_flag)
            .arg(format!("--columns={}", parser::FLATPAK_COLUMNS));

        self.runner.run(&spec).await.map(|output| output.stdout)
    }
}

impl Default for FlatpakProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InventoryProvider for FlatpakProvider {
    fn id(&self) -> ProviderId {
        FLATPAK_PROVIDER_ID
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: FLATPAK_PROVIDER_ID,
            display_name: "Flatpak",
            description: "Applications and runtimes installed via Flatpak.",
        }
    }

    async fn is_available(&self) -> bool {
        let spec = CommandSpec::new(&self.flatpak_bin).arg("--version");
        self.runner.run(&spec).await.is_ok()
    }

    async fn scan(&self, ctx: &ScanContext) -> ProviderInventory {
        let started_at = Utc::now();
        let base = ProviderInventory::started(FLATPAK_PROVIDER_ID.as_str(), started_at);

        if ctx.is_cancelled() {
            return base.finish(Utc::now(), ProviderStatus::Cancelled);
        }

        let mut warnings: Vec<ProviderWarning> = Vec::new();
        let mut items: Vec<SoftwareItem> = Vec::new();
        let mut calls_attempted = 0_u32;
        let mut calls_succeeded = 0_u32;

        for (scope, kind) in [
            (Scope::User, RefKind::App),
            (Scope::User, RefKind::Runtime),
            (Scope::System, RefKind::App),
            (Scope::System, RefKind::Runtime),
        ] {
            if ctx.is_cancelled() {
                return base.finish(Utc::now(), ProviderStatus::Cancelled);
            }

            calls_attempted += 1;

            match self.list(scope, kind).await {
                Ok(stdout) => {
                    calls_succeeded += 1;
                    let mut parse_warnings = Vec::new();
                    let records = parser::parse_flatpak_list(&stdout, &mut parse_warnings);
                    warnings.extend(parse_warnings.into_iter().map(ProviderWarning::new));
                    items.extend(
                        records
                            .into_iter()
                            .map(|record| build_item(&record, scope, kind)),
                    );
                }
                Err(RunError::NotFound(_)) => {
                    // Flatpak isn't installed at all -- this is expected on
                    // many systems, not a failure. No point trying the
                    // remaining three calls; they would fail identically.
                    return base.finish(Utc::now(), ProviderStatus::Unavailable);
                }
                Err(error) => {
                    warnings.push(ProviderWarning::new(format!(
                        "could not list {} {} Flatpak refs: {error}",
                        scope.as_str(),
                        match kind {
                            RefKind::App => "application",
                            RefKind::Runtime => "runtime",
                        }
                    )));
                }
            }
        }

        let status = if calls_succeeded == 0 && calls_attempted > 0 {
            let mut failed = base.finish(Utc::now(), ProviderStatus::Failed);
            failed.error = Some(ProviderError::Other(
                "all flatpak list calls failed; see warnings for details".to_string(),
            ));
            failed.warnings = warnings;
            return failed;
        } else if warnings.is_empty() {
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

fn looks_like_extension(application_id: &str) -> bool {
    [
        ".Locale",
        ".Debug",
        ".GL.",
        ".Extension",
        ".Codecs",
        ".Sources",
    ]
    .iter()
    .any(|marker| application_id.contains(marker))
}

fn build_item(record: &parser::FlatpakRecord, scope: Scope, kind: RefKind) -> SoftwareItem {
    let display_name = record
        .name
        .clone()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| record.application.clone());

    let mut item = SoftwareItem::new(
        format!("flatpak:{}:{}", scope.as_str(), record.application),
        record.application.clone(),
        display_name,
        PackageManager::Flatpak,
    );

    item.version = record.version.clone();
    item.architecture = record.arch.clone();
    item.scope = scope.as_installation_scope();
    item.repository = record.origin.clone();
    item.installed_size_bytes = record.installed_size_bytes;
    item.classification_confidence = ClassificationConfidence::Certain;

    if let Some(branch) = &record.branch {
        item.metadata.insert("branch".to_string(), branch.clone());
    }

    match kind {
        RefKind::App => {
            item.category = SoftwareCategory::Application;
            item.classification_reasons = vec!["installed as a Flatpak application".to_string()];
        }
        RefKind::Runtime => {
            item.category = SoftwareCategory::Runtime;
            if looks_like_extension(&record.application) {
                item.metadata
                    .insert("flatpak_kind".to_string(), "extension".to_string());
                item.classification_reasons =
                    vec!["installed as a Flatpak extension (modeled as a Runtime)".to_string()];
            } else {
                item.metadata
                    .insert("flatpak_kind".to_string(), "runtime".to_string());
                item.classification_reasons = vec!["installed as a Flatpak runtime".to_string()];
            }
        }
    }

    item
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn record(application: &str) -> parser::FlatpakRecord {
        parser::FlatpakRecord {
            application: application.to_string(),
            name: None,
            version: None,
            branch: None,
            arch: None,
            origin: None,
            installed_size_bytes: None,
        }
    }

    #[test]
    fn build_item_for_an_app_uses_application_category_with_certain_confidence() {
        let mut r = record("org.mozilla.firefox");
        r.name = Some("Firefox".to_string());

        let item = build_item(&r, Scope::User, RefKind::App);

        assert_eq!(item.id, "flatpak:user:org.mozilla.firefox");
        assert_eq!(item.display_name, "Firefox");
        assert_eq!(item.category, SoftwareCategory::Application);
        assert_eq!(
            item.classification_confidence,
            ClassificationConfidence::Certain
        );
        assert_eq!(item.scope, InstallationScope::User);
        assert_eq!(item.package_manager, PackageManager::Flatpak);
    }

    #[test]
    fn build_item_falls_back_to_application_id_when_name_is_missing() {
        let item = build_item(&record("org.example.NoName"), Scope::System, RefKind::App);

        assert_eq!(item.display_name, "org.example.NoName");
        assert_eq!(item.scope, InstallationScope::System);
    }

    #[test]
    fn build_item_for_a_plain_runtime_tags_metadata_as_runtime() {
        let item = build_item(&record("org.gnome.Platform"), Scope::User, RefKind::Runtime);

        assert_eq!(item.category, SoftwareCategory::Runtime);
        assert_eq!(
            item.metadata.get("flatpak_kind").map(String::as_str),
            Some("runtime")
        );
    }

    #[test]
    fn build_item_for_an_extension_like_ref_tags_metadata_as_extension() {
        let item = build_item(
            &record("org.freedesktop.Platform.GL.default"),
            Scope::User,
            RefKind::Runtime,
        );

        assert_eq!(item.category, SoftwareCategory::Runtime);
        assert_eq!(
            item.metadata.get("flatpak_kind").map(String::as_str),
            Some("extension")
        );
    }

    #[test]
    fn build_item_ids_differ_by_scope_so_user_and_system_installs_do_not_collide() {
        let user_item = build_item(&record("org.example.App"), Scope::User, RefKind::App);
        let system_item = build_item(&record("org.example.App"), Scope::System, RefKind::App);

        assert_ne!(user_item.id, system_item.id);
    }

    #[tokio::test]
    async fn is_available_is_false_when_flatpak_binary_does_not_exist() {
        let provider = FlatpakProvider::with_binary(
            ProcessRunner::default(),
            "kunger-nonexistent-flatpak-xyz",
        );

        assert!(!provider.is_available().await);
    }

    #[tokio::test]
    async fn is_available_is_true_for_a_real_binary_regardless_of_its_purpose() {
        let provider = FlatpakProvider::with_binary(ProcessRunner::default(), "true");

        assert!(provider.is_available().await);
    }

    #[tokio::test]
    async fn scan_reports_cancelled_immediately_when_the_context_is_already_cancelled() {
        let provider = FlatpakProvider::new();
        let ctx = ScanContext::new(Duration::from_secs(5));
        ctx.cancellation.cancel();

        let result = provider.scan(&ctx).await;

        assert_eq!(result.status, ProviderStatus::Cancelled);
    }

    #[tokio::test]
    async fn scan_reports_unavailable_not_failed_when_flatpak_is_not_installed() {
        let provider = FlatpakProvider::with_binary(
            ProcessRunner::default(),
            "kunger-nonexistent-flatpak-xyz",
        );
        let ctx = ScanContext::new(Duration::from_secs(5));

        let result = provider.scan(&ctx).await;

        assert_eq!(result.status, ProviderStatus::Unavailable);
        assert!(result.error.is_none());
        assert!(result.items.is_empty());
    }
}
