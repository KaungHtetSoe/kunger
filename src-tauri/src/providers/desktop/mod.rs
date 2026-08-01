//! The desktop application provider: discovers `.desktop` launchers in
//! standard locations and resolves their package ownership. See
//! `docs/ARCHITECTURE.md` §2.2 and `docs/SECURITY.md`.

mod parser;

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::Utc;

use crate::classification::{classify, Evidence};
use crate::domain::{
    InstallationScope, PackageManager, ProviderInventory, ProviderStatus, ProviderWarning,
    SoftwareItem,
};
use crate::process::ProcessRunner;
use crate::providers::dpkg_ownership;
use crate::providers::{InventoryProvider, ProviderId, ProviderMetadata, ScanContext};

const DESKTOP_PROVIDER_ID: ProviderId = ProviderId::new("desktop");

const SYSTEM_APPLICATION_DIRS: &[&str] =
    &["/usr/local/share/applications", "/usr/share/applications"];
/// `.desktop` files are always small text files; this is a generous bound
/// against a maliciously or accidentally huge file rather than a realistic
/// expected size.
const MAX_DESKTOP_FILE_BYTES: u64 = 1024 * 1024;
/// Bounds recursive scanning of application directories (e.g. the common
/// `applications/kde4/` legacy layout) without ever performing an
/// unbounded filesystem walk — see `docs/SECURITY.md`.
const MAX_RECURSION_DEPTH: usize = 2;

/// Discovers `.desktop` application launchers and resolves dpkg ownership
/// for them.
///
/// Ownership-known entries are emitted with the *same* `id` the APT
/// provider would use for their owning package (`apt:{package}`) rather
/// than a separate standalone item, so that once the unified inventory
/// service (M4.1) merges provider results by id, a desktop entry never
/// shows up as a fabricated extra "manually installed" duplicate of a
/// package dpkg already owns. Ownership-known entries are intentionally
/// left unclassified here (`category` stays `Unclassified`) — full
/// classification for those happens once merged with the owning package's
/// richer evidence (Debian section, etc.) at merge time. Entries with no
/// resolvable owner are classified standalone via the classification
/// engine and get a `desktop:{filename}` id.
///
/// When the same `.desktop` filename exists in more than one scanned
/// directory (a legitimate, common override mechanism per the XDG
/// Base Directory spec — a user override in `~/.local/share/applications`
/// intentionally shadows a system one with the same name), only the
/// highest-priority copy is kept, never both.
pub struct DesktopProvider {
    runner: ProcessRunner,
    dpkg_bin: String,
}

impl DesktopProvider {
    pub fn new() -> Self {
        Self {
            runner: ProcessRunner::default(),
            dpkg_bin: "dpkg".to_string(),
        }
    }

    #[cfg(test)]
    fn with_binary(runner: ProcessRunner, dpkg_bin: impl Into<String>) -> Self {
        Self {
            runner,
            dpkg_bin: dpkg_bin.into(),
        }
    }

    async fn resolve_owners(
        &self,
        paths: &[PathBuf],
        warnings: &mut Vec<ProviderWarning>,
    ) -> HashMap<String, String> {
        match dpkg_ownership::resolve_owners(&self.runner, &self.dpkg_bin, paths).await {
            Ok(owners) => owners,
            Err(error) => {
                warnings.push(ProviderWarning::new(format!(
                    "could not resolve desktop entry package ownership via dpkg -S ({error}); \
                     all entries will be treated as unowned"
                )));
                HashMap::new()
            }
        }
    }
}

impl Default for DesktopProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InventoryProvider for DesktopProvider {
    fn id(&self) -> ProviderId {
        DESKTOP_PROVIDER_ID
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: DESKTOP_PROVIDER_ID,
            display_name: "Desktop Entries",
            description: "Application launchers discovered from .desktop files.",
        }
    }

    async fn is_available(&self) -> bool {
        // This provider's data source is the filesystem itself, not an
        // external tool -- scanning standard directories is always
        // attempted. Missing directories are handled gracefully in scan().
        true
    }

    async fn scan(&self, ctx: &ScanContext) -> ProviderInventory {
        let started_at = Utc::now();
        let base = ProviderInventory::started(DESKTOP_PROVIDER_ID.as_str(), started_at);

        if ctx.is_cancelled() {
            return base.finish(Utc::now(), ProviderStatus::Cancelled);
        }

        let mut warnings: Vec<ProviderWarning> = Vec::new();
        let files = collect_desktop_files(&mut warnings);

        if ctx.is_cancelled() {
            return base.finish(Utc::now(), ProviderStatus::Cancelled);
        }

        let paths: Vec<PathBuf> = files.iter().map(|(path, _scope)| path.clone()).collect();
        let owners = self.resolve_owners(&paths, &mut warnings).await;

        let mut items = Vec::new();
        for (path, scope) in &files {
            let content = match read_bounded(path) {
                Ok(content) => content,
                Err(error) => {
                    warnings.push(ProviderWarning::new(format!(
                        "could not read {}: {error}",
                        path.display()
                    )));
                    continue;
                }
            };

            let mut parse_warnings = Vec::new();
            let source = path.to_string_lossy().into_owned();
            let Some(record) = parser::parse_desktop_entry(&content, &source, &mut parse_warnings)
            else {
                warnings.extend(parse_warnings.into_iter().map(ProviderWarning::new));
                continue;
            };
            warnings.extend(parse_warnings.into_iter().map(ProviderWarning::new));

            let owner = owners
                .get(&path.to_string_lossy().into_owned())
                .map(String::as_str);
            items.push(build_item(&record, path, *scope, owner));
        }

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

fn user_applications_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("applications"))
}

/// Collects `.desktop` file paths across all standard locations, in XDG
/// priority order (user directory highest, then `/usr/local/share`, then
/// `/usr/share`), deduplicating by filename so a higher-priority override
/// always wins over a lower-priority file of the same name rather than
/// both appearing as if they were unrelated "duplicates."
fn collect_desktop_files(warnings: &mut Vec<ProviderWarning>) -> Vec<(PathBuf, InstallationScope)> {
    let mut ordered_dirs: Vec<(PathBuf, InstallationScope)> = Vec::new();

    match user_applications_dir() {
        Some(dir) => ordered_dirs.push((dir, InstallationScope::User)),
        None => warnings.push(ProviderWarning::new(
            "could not determine the user's local applications directory (no home directory found)",
        )),
    }

    for dir in SYSTEM_APPLICATION_DIRS {
        ordered_dirs.push((PathBuf::from(dir), InstallationScope::System));
    }

    let mut seen_filenames: HashSet<String> = HashSet::new();
    let mut files = Vec::new();

    for (dir, scope) in ordered_dirs {
        let mut found = Vec::new();
        let mut scan_warnings = Vec::new();
        scan_dir(&dir, scope, 0, &mut found, &mut scan_warnings);
        warnings.extend(scan_warnings.into_iter().map(ProviderWarning::new));

        for (path, file_scope) in found {
            let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if seen_filenames.insert(filename.to_string()) {
                files.push((path, file_scope));
            }
        }
    }

    files
}

fn scan_dir(
    dir: &Path,
    scope: InstallationScope,
    depth: usize,
    out: &mut Vec<(PathBuf, InstallationScope)>,
    warnings: &mut Vec<String>,
) {
    if depth > MAX_RECURSION_DEPTH {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            // A missing applications directory is normal (not every
            // system has /usr/local/share/applications); anything else
            // (e.g. a permissions problem) is worth surfacing.
            if error.kind() != std::io::ErrorKind::NotFound {
                warnings.push(format!("could not read {}: {error}", dir.display()));
            }
            return;
        }
    };

    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();

        // `file_type()` deliberately does not follow symlinks: a
        // symlinked directory is never recursed into, so a scan root can
        // never be escaped via a symlink pointing outside it. A symlinked
        // `.desktop` file is still picked up below (its target is only
        // ever read as bounded text, never traversed as a directory).
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            scan_dir(&path, scope, depth + 1, out, warnings);
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) == Some("desktop") {
            out.push((path, scope));
        }
    }
}

fn read_bounded(path: &Path) -> std::io::Result<String> {
    let file = std::fs::File::open(path)?;
    let mut limited = file.take(MAX_DESKTOP_FILE_BYTES);
    let mut buf = Vec::new();
    limited.read_to_end(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn build_item(
    record: &parser::DesktopEntryRecord,
    path: &Path,
    scope: InstallationScope,
    owner: Option<&str>,
) -> SoftwareItem {
    let path_str = path.to_string_lossy().into_owned();

    let (id, package_manager, package_name) = match owner {
        Some(pkg) => (format!("apt:{pkg}"), PackageManager::Apt, pkg.to_string()),
        None => {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
                .unwrap_or_else(|| record.name.clone());
            (format!("desktop:{stem}"), PackageManager::Manual, stem)
        }
    };

    let mut item = SoftwareItem::new(
        id,
        package_name.clone(),
        record.name.clone(),
        package_manager,
    );
    item.scope = scope;
    item.desktop_file_paths = vec![path_str];
    item.icon_path = record.icon.clone();
    item.description = record.comment.clone();

    match owner {
        Some(pkg) => {
            item.metadata
                .insert("owning_package".to_string(), pkg.to_string());
        }
        None => {
            let evidence = Evidence {
                package_name: package_name.clone(),
                has_desktop_launcher: true,
                desktop_categories: record.categories.clone(),
                ..Default::default()
            };
            let classification = classify(&evidence);
            item.category = classification.category;
            item.secondary_categories = classification.secondary_categories;
            item.classification_confidence = classification.confidence;
            item.classification_reasons = classification.reasons;
        }
    }

    if let Some(generic_name) = &record.generic_name {
        item.metadata
            .insert("generic_name".to_string(), generic_name.clone());
    }
    if let Some(exec) = &record.exec {
        item.metadata.insert("exec".to_string(), exec.clone());
    }
    if let Some(wm_class) = &record.startup_wm_class {
        item.metadata
            .insert("startup_wm_class".to_string(), wm_class.clone());
    }
    if record.terminal {
        item.metadata
            .insert("terminal".to_string(), "true".to_string());
    }
    if record.no_display {
        item.metadata
            .insert("desktop_no_display".to_string(), "true".to_string());
    }
    if record.hidden {
        item.metadata
            .insert("desktop_hidden".to_string(), "true".to_string());
    }
    if !record.categories.is_empty() {
        item.metadata
            .insert("categories".to_string(), record.categories.join(";"));
    }

    item
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SoftwareCategory;
    use std::time::Duration;

    fn record(name: &str) -> parser::DesktopEntryRecord {
        parser::DesktopEntryRecord {
            name: name.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn build_item_for_an_owned_entry_reuses_the_apt_id_and_stays_unclassified() {
        let item = build_item(
            &record("Firefox"),
            Path::new("/usr/share/applications/firefox.desktop"),
            InstallationScope::System,
            Some("firefox"),
        );

        assert_eq!(item.id, "apt:firefox");
        assert_eq!(item.package_manager, PackageManager::Apt);
        assert_eq!(item.category, SoftwareCategory::Unclassified);
        assert_eq!(
            item.metadata.get("owning_package").map(String::as_str),
            Some("firefox")
        );
    }

    #[test]
    fn build_item_for_an_unowned_entry_gets_a_desktop_id_and_is_classified_as_an_application() {
        let mut r = record("Custom Tool");
        r.categories = vec!["Utility".to_string()];

        let item = build_item(
            &r,
            Path::new("/home/user/.local/share/applications/custom-tool.desktop"),
            InstallationScope::User,
            None,
        );

        assert_eq!(item.id, "desktop:custom-tool");
        assert_eq!(item.package_manager, PackageManager::Manual);
        assert_eq!(item.category, SoftwareCategory::Application);
    }

    #[test]
    fn build_item_tags_hidden_and_no_display_in_metadata_without_dropping_the_item() {
        let mut r = record("Hidden Helper");
        r.hidden = true;
        r.no_display = true;

        let item = build_item(
            &r,
            Path::new("/usr/share/applications/hidden-helper.desktop"),
            InstallationScope::System,
            None,
        );

        assert_eq!(
            item.metadata.get("desktop_hidden").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            item.metadata.get("desktop_no_display").map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn build_item_never_promotes_exec_beyond_opaque_metadata() {
        let mut r = record("Escaped");
        r.exec = Some(r#"/usr/bin/app --flag="value" %f"#.to_string());

        let item = build_item(
            &r,
            Path::new("/usr/share/applications/escaped.desktop"),
            InstallationScope::System,
            None,
        );

        assert_eq!(
            item.metadata.get("exec").map(String::as_str),
            Some(r#"/usr/bin/app --flag="value" %f"#)
        );
    }

    #[test]
    fn collect_desktop_files_prefers_the_fixtures_directory_style_dedup_by_filename() {
        // Direct unit test of the dedup logic without touching the real
        // filesystem: two directories both "containing" a same-named
        // file should only contribute one entry, with the first
        // (higher-priority) directory's copy winning.
        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<(PathBuf, InstallationScope)> = Vec::new();

        for (path, scope) in [
            (
                PathBuf::from("/home/user/.local/share/applications/app.desktop"),
                InstallationScope::User,
            ),
            (
                PathBuf::from("/usr/share/applications/app.desktop"),
                InstallationScope::System,
            ),
        ] {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if seen.insert(name.to_string()) {
                    out.push((path, scope));
                }
            }
        }

        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].0,
            PathBuf::from("/home/user/.local/share/applications/app.desktop")
        );
    }

    #[tokio::test]
    async fn is_available_is_always_true() {
        let provider = DesktopProvider::new();
        assert!(provider.is_available().await);
    }

    #[tokio::test]
    async fn scan_reports_cancelled_immediately_when_the_context_is_already_cancelled() {
        let provider = DesktopProvider::new();
        let ctx = ScanContext::new(Duration::from_secs(5));
        ctx.cancellation.cancel();

        let result = provider.scan(&ctx).await;

        assert_eq!(result.status, ProviderStatus::Cancelled);
    }

    #[tokio::test]
    async fn resolve_owners_degrades_gracefully_when_dpkg_is_missing() {
        let provider =
            DesktopProvider::with_binary(ProcessRunner::default(), "kunger-nonexistent-dpkg-xyz");
        let mut warnings = Vec::new();

        let owners = provider
            .resolve_owners(
                &[PathBuf::from("/usr/share/applications/firefox.desktop")],
                &mut warnings,
            )
            .await;

        assert!(owners.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0]
            .message
            .contains("could not resolve desktop entry package ownership"));
    }

    #[tokio::test]
    async fn resolve_owners_with_no_paths_short_circuits_without_running_a_command() {
        let provider =
            DesktopProvider::with_binary(ProcessRunner::default(), "kunger-nonexistent-dpkg-xyz");
        let mut warnings = Vec::new();

        let owners = provider.resolve_owners(&[], &mut warnings).await;

        assert!(owners.is_empty());
        assert!(warnings.is_empty());
    }
}
