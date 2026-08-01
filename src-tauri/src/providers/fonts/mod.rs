//! The font inventory provider. See `docs/ARCHITECTURE.md` §2.2 and
//! `docs/SECURITY.md`.
//!
//! Uses `fc-list` only for reading — never `fc-cache`, which would rebuild
//! the font cache. Kunger never modifies fontconfig's cache or config.

mod parser;

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::{
    ClassificationConfidence, InstallationScope, PackageManager, ProviderError, ProviderInventory,
    ProviderStatus, ProviderWarning, SoftwareCategory, SoftwareItem,
};
use crate::process::{CommandSpec, ProcessRunner, RunError};
use crate::providers::dpkg_ownership;
use crate::providers::{InventoryProvider, ProviderId, ProviderMetadata, ScanContext};

const FONT_PROVIDER_ID: ProviderId = ProviderId::new("fonts");

const SYSTEM_FONT_DIRS: &[&str] = &["/usr/share/fonts", "/usr/local/share/fonts"];

/// Inventories fonts registered with Fontconfig, restricted to the
/// documented set of known font locations (`/usr/share/fonts`,
/// `/usr/local/share/fonts`, `~/.local/share/fonts`, `~/.fonts`) — fonts
/// `fc-list` reports from elsewhere (e.g. a Flatpak app's bundled fonts)
/// are out of this provider's scope and are silently excluded, per
/// `docs/SECURITY.md`'s bounded-scanning requirement.
///
/// Multiple font *files* belonging to the same family (e.g. Regular,
/// Bold, Italic, Bold Italic) are grouped into one logical
/// [`SoftwareItem`] per `(scope, family)` pair, preserving each file's
/// path in `install_paths`. A family installed in more than one scope
/// (e.g. both system-wide and a user override) intentionally produces two
/// separate items rather than one merged item, so the duplicate
/// installation is visible rather than silently collapsed — see
/// `docs/PRODUCT_SPEC.md`'s "is it duplicated" requirement.
///
/// Ownership-known families (every file in the group owned by the same
/// dpkg package) reuse that package's `apt:{name}` id, following the same
/// merge-by-id convention as the desktop provider (ADR-0012).
pub struct FontProvider {
    runner: ProcessRunner,
    fc_list_bin: String,
    dpkg_bin: String,
}

impl FontProvider {
    pub fn new() -> Self {
        Self {
            runner: ProcessRunner::default(),
            fc_list_bin: "fc-list".to_string(),
            dpkg_bin: "dpkg".to_string(),
        }
    }

    #[cfg(test)]
    fn with_binaries(
        runner: ProcessRunner,
        fc_list_bin: impl Into<String>,
        dpkg_bin: impl Into<String>,
    ) -> Self {
        Self {
            runner,
            fc_list_bin: fc_list_bin.into(),
            dpkg_bin: dpkg_bin.into(),
        }
    }
}

impl Default for FontProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InventoryProvider for FontProvider {
    fn id(&self) -> ProviderId {
        FONT_PROVIDER_ID
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: FONT_PROVIDER_ID,
            display_name: "Fonts",
            description: "Fonts registered with Fontconfig in known font directories.",
        }
    }

    async fn is_available(&self) -> bool {
        let spec = CommandSpec::new(&self.fc_list_bin).arg("--version");
        self.runner.run(&spec).await.is_ok()
    }

    async fn scan(&self, ctx: &ScanContext) -> ProviderInventory {
        let started_at = Utc::now();
        let base = ProviderInventory::started(FONT_PROVIDER_ID.as_str(), started_at);

        if ctx.is_cancelled() {
            return base.finish(Utc::now(), ProviderStatus::Cancelled);
        }

        let spec =
            CommandSpec::new(&self.fc_list_bin).arg(format!("--format={}", parser::FC_LIST_FORMAT));

        let output = match self.runner.run(&spec).await {
            Ok(output) => output,
            Err(RunError::NotFound(_)) => {
                // Fontconfig not being installed is an expected condition
                // on some minimal systems, not a failure.
                return base.finish(Utc::now(), ProviderStatus::Unavailable);
            }
            Err(error) => {
                let mut failed = base.finish(Utc::now(), ProviderStatus::Failed);
                failed.error = Some(map_run_error(&error));
                return failed;
            }
        };

        let mut warnings: Vec<ProviderWarning> = Vec::new();
        let mut parse_warnings = Vec::new();
        let records = parser::parse_fc_list(&output.stdout, &mut parse_warnings);
        warnings.extend(parse_warnings.into_iter().map(ProviderWarning::new));

        let user_dirs = user_font_dirs();
        let in_scope: Vec<(parser::FontFileRecord, InstallationScope)> = records
            .into_iter()
            .filter_map(|record| {
                classify_scope(&record.file, &user_dirs).map(|scope| (record, scope))
            })
            .collect();

        if ctx.is_cancelled() {
            return base.finish(Utc::now(), ProviderStatus::Cancelled);
        }

        let paths: Vec<std::path::PathBuf> = in_scope
            .iter()
            .map(|(record, _)| std::path::PathBuf::from(&record.file))
            .collect();
        let owners =
            match dpkg_ownership::resolve_owners(&self.runner, &self.dpkg_bin, &paths).await {
                Ok(owners) => owners,
                Err(error) => {
                    warnings.push(ProviderWarning::new(format!(
                        "could not resolve font package ownership via dpkg -S ({error}); \
                     all fonts will be treated as unowned"
                    )));
                    HashMap::new()
                }
            };

        let items = build_items(&in_scope, &owners);

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

fn user_font_dirs() -> Vec<String> {
    let mut dirs_list = Vec::new();
    if let Some(data_local) = dirs::data_local_dir() {
        dirs_list.push(data_local.join("fonts").to_string_lossy().into_owned());
    }
    if let Some(home) = dirs::home_dir() {
        dirs_list.push(home.join(".fonts").to_string_lossy().into_owned());
    }
    dirs_list
}

fn classify_scope(path: &str, user_dirs: &[String]) -> Option<InstallationScope> {
    if SYSTEM_FONT_DIRS.iter().any(|dir| path.starts_with(dir)) {
        return Some(InstallationScope::System);
    }
    if user_dirs.iter().any(|dir| path.starts_with(dir.as_str())) {
        return Some(InstallationScope::User);
    }
    None
}

fn slugify(family: &str) -> String {
    family
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
}

fn build_items(
    in_scope: &[(parser::FontFileRecord, InstallationScope)],
    owners: &HashMap<String, String>,
) -> Vec<SoftwareItem> {
    let mut groups: HashMap<(InstallationScope, String), Vec<&parser::FontFileRecord>> =
        HashMap::new();

    for (record, scope) in in_scope {
        groups
            .entry((*scope, record.family.clone()))
            .or_default()
            .push(record);
    }

    let mut items: Vec<SoftwareItem> = groups
        .into_iter()
        .map(|((scope, family), records)| build_item(&family, scope, &records, owners))
        .collect();

    items.sort_by(|a, b| a.id.cmp(&b.id));
    items
}

fn build_item(
    family: &str,
    scope: InstallationScope,
    records: &[&parser::FontFileRecord],
    owners: &HashMap<String, String>,
) -> SoftwareItem {
    let file_owners: HashSet<Option<&String>> =
        records.iter().map(|r| owners.get(&r.file)).collect();

    let single_owner = if file_owners.len() == 1 {
        file_owners.into_iter().next().flatten()
    } else {
        None
    };

    let (id, package_manager, package_name) = match single_owner {
        Some(pkg) => (format!("apt:{pkg}"), PackageManager::Apt, pkg.clone()),
        None => {
            let slug = slugify(family);
            (
                format!("font:{}:{slug}", scope_str(scope)),
                PackageManager::Manual,
                slug,
            )
        }
    };

    let mut item = SoftwareItem::new(id, package_name, family, package_manager);
    item.scope = scope;
    item.install_paths = records.iter().map(|r| r.file.clone()).collect();

    match single_owner {
        Some(pkg) => {
            item.metadata
                .insert("owning_package".to_string(), pkg.clone());
        }
        None => {
            item.category = SoftwareCategory::Font;
            item.classification_confidence = ClassificationConfidence::Certain;
            item.classification_reasons = vec![
                "package owns TrueType/OpenType font files in a known font directory".to_string(),
            ];
        }
    }

    let styles: Vec<String> = {
        let mut list: Vec<String> = records.iter().filter_map(|r| r.style.clone()).collect();
        list.sort();
        list.dedup();
        list
    };
    if !styles.is_empty() {
        item.metadata.insert("styles".to_string(), styles.join(";"));
    }

    let formats: Vec<String> = {
        let mut list: Vec<String> = records
            .iter()
            .filter_map(|r| r.font_format.clone())
            .collect();
        list.sort();
        list.dedup();
        list
    };
    if !formats.is_empty() {
        item.metadata
            .insert("font_formats".to_string(), formats.join(";"));
    }

    item.metadata
        .insert("file_count".to_string(), records.len().to_string());

    if let Some(language_coverage) = records.iter().find_map(|r| r.language_coverage.clone()) {
        item.metadata
            .insert("language_coverage".to_string(), language_coverage);
    }

    item
}

fn scope_str(scope: InstallationScope) -> &'static str {
    match scope {
        InstallationScope::System => "system",
        InstallationScope::User => "user",
        InstallationScope::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn record(file: &str, family: &str, style: &str) -> parser::FontFileRecord {
        parser::FontFileRecord {
            file: file.to_string(),
            family: family.to_string(),
            style: Some(style.to_string()),
            fullname: None,
            font_format: Some("TrueType".to_string()),
            language_coverage: Some("en".to_string()),
        }
    }

    #[test]
    fn classify_scope_recognizes_system_directories() {
        let user_dirs = vec!["/home/user/.local/share/fonts".to_string()];
        assert_eq!(
            classify_scope("/usr/share/fonts/truetype/foo/Foo.ttf", &user_dirs),
            Some(InstallationScope::System)
        );
        assert_eq!(
            classify_scope("/usr/local/share/fonts/Foo.ttf", &user_dirs),
            Some(InstallationScope::System)
        );
    }

    #[test]
    fn classify_scope_recognizes_user_directories() {
        let user_dirs = vec![
            "/home/user/.local/share/fonts".to_string(),
            "/home/user/.fonts".to_string(),
        ];
        assert_eq!(
            classify_scope("/home/user/.local/share/fonts/Foo.ttf", &user_dirs),
            Some(InstallationScope::User)
        );
        assert_eq!(
            classify_scope("/home/user/.fonts/Foo.ttf", &user_dirs),
            Some(InstallationScope::User)
        );
    }

    #[test]
    fn classify_scope_excludes_paths_outside_known_directories() {
        let user_dirs = vec!["/home/user/.local/share/fonts".to_string()];
        assert_eq!(
            classify_scope(
                "/var/lib/flatpak/app/org.example.App/files/share/fonts/App.ttf",
                &user_dirs
            ),
            None
        );
    }

    #[test]
    fn build_items_groups_multiple_files_into_one_family_item() {
        let records = vec![
            (
                record(
                    "/usr/share/fonts/NotoSans-Regular.ttf",
                    "Noto Sans",
                    "Regular",
                ),
                InstallationScope::System,
            ),
            (
                record("/usr/share/fonts/NotoSans-Bold.ttf", "Noto Sans", "Bold"),
                InstallationScope::System,
            ),
        ];

        let items = build_items(&records, &HashMap::new());

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].display_name, "Noto Sans");
        assert_eq!(items[0].install_paths.len(), 2);
        assert_eq!(
            items[0].metadata.get("file_count").map(String::as_str),
            Some("2")
        );
        assert_eq!(
            items[0].metadata.get("styles").map(String::as_str),
            Some("Bold;Regular")
        );
    }

    #[test]
    fn build_items_keeps_same_family_in_different_scopes_as_separate_items() {
        let records = vec![
            (
                record(
                    "/usr/share/fonts/NotoSans-Regular.ttf",
                    "Noto Sans",
                    "Regular",
                ),
                InstallationScope::System,
            ),
            (
                record(
                    "/home/user/.local/share/fonts/NotoSans-Regular.ttf",
                    "Noto Sans",
                    "Regular",
                ),
                InstallationScope::User,
            ),
        ];

        let items = build_items(&records, &HashMap::new());

        assert_eq!(items.len(), 2);
        assert_ne!(items[0].id, items[1].id);
        assert!(items.iter().any(|i| i.scope == InstallationScope::System));
        assert!(items.iter().any(|i| i.scope == InstallationScope::User));
    }

    #[test]
    fn build_items_uses_owning_package_id_when_every_file_shares_one_owner() {
        let records = vec![
            (
                record(
                    "/usr/share/fonts/NotoSans-Regular.ttf",
                    "Noto Sans",
                    "Regular",
                ),
                InstallationScope::System,
            ),
            (
                record("/usr/share/fonts/NotoSans-Bold.ttf", "Noto Sans", "Bold"),
                InstallationScope::System,
            ),
        ];
        let owners: HashMap<String, String> = [
            (
                "/usr/share/fonts/NotoSans-Regular.ttf".to_string(),
                "fonts-noto-core".to_string(),
            ),
            (
                "/usr/share/fonts/NotoSans-Bold.ttf".to_string(),
                "fonts-noto-core".to_string(),
            ),
        ]
        .into_iter()
        .collect();

        let items = build_items(&records, &owners);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "apt:fonts-noto-core");
        assert_eq!(items[0].package_manager, PackageManager::Apt);
        assert_eq!(items[0].category, SoftwareCategory::Unclassified);
    }

    #[test]
    fn build_items_falls_back_to_manual_when_files_have_different_owners() {
        let records = vec![
            (
                record(
                    "/usr/share/fonts/NotoSans-Regular.ttf",
                    "Noto Sans",
                    "Regular",
                ),
                InstallationScope::System,
            ),
            (
                record("/usr/share/fonts/NotoSans-Bold.ttf", "Noto Sans", "Bold"),
                InstallationScope::System,
            ),
        ];
        let owners: HashMap<String, String> = [
            (
                "/usr/share/fonts/NotoSans-Regular.ttf".to_string(),
                "fonts-noto-core".to_string(),
            ),
            (
                "/usr/share/fonts/NotoSans-Bold.ttf".to_string(),
                "fonts-noto-extra".to_string(),
            ),
        ]
        .into_iter()
        .collect();

        let items = build_items(&records, &owners);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].package_manager, PackageManager::Manual);
        assert_eq!(items[0].category, SoftwareCategory::Font);
    }

    #[test]
    fn build_items_unowned_font_is_classified_as_font_with_certain_confidence() {
        let records = vec![(
            record("/home/user/.fonts/MyFont.ttf", "My Font", "Regular"),
            InstallationScope::User,
        )];

        let items = build_items(&records, &HashMap::new());

        assert_eq!(items[0].category, SoftwareCategory::Font);
        assert_eq!(
            items[0].classification_confidence,
            ClassificationConfidence::Certain
        );
        assert_eq!(items[0].id, "font:user:my-font");
    }

    #[tokio::test]
    async fn is_available_is_false_when_fc_list_binary_does_not_exist() {
        let provider = FontProvider::with_binaries(
            ProcessRunner::default(),
            "kunger-nonexistent-fc-list-xyz",
            "dpkg",
        );

        assert!(!provider.is_available().await);
    }

    #[tokio::test]
    async fn is_available_is_true_for_a_real_binary_regardless_of_its_purpose() {
        let provider = FontProvider::with_binaries(ProcessRunner::default(), "true", "dpkg");

        assert!(provider.is_available().await);
    }

    #[tokio::test]
    async fn scan_reports_cancelled_immediately_when_the_context_is_already_cancelled() {
        let provider = FontProvider::new();
        let ctx = ScanContext::new(Duration::from_secs(5));
        ctx.cancellation.cancel();

        let result = provider.scan(&ctx).await;

        assert_eq!(result.status, ProviderStatus::Cancelled);
    }

    #[tokio::test]
    async fn scan_reports_unavailable_not_failed_when_fc_list_is_missing() {
        let provider = FontProvider::with_binaries(
            ProcessRunner::default(),
            "kunger-nonexistent-fc-list-xyz",
            "dpkg",
        );
        let ctx = ScanContext::new(Duration::from_secs(5));

        let result = provider.scan(&ctx).await;

        assert_eq!(result.status, ProviderStatus::Unavailable);
        assert!(result.error.is_none());
        assert!(result.items.is_empty());
    }
}
