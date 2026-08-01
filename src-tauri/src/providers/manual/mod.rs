//! The manual software provider: catches executables, libraries, and
//! `/opt` install directories not owned by any known package manager. See
//! `docs/ARCHITECTURE.md` §2.2 and `docs/SECURITY.md`.
//!
//! Deliberately does **not** scan `~/.local/share/applications`, even
//! though it appears in the original scope list: the desktop-entry
//! provider already fully owns that directory (parsing, ownership
//! resolution, classification) — a second, cruder pass over the same
//! `.desktop` files here would only produce confusing duplicate records
//! for the same file, not new information. Likewise, files with the
//! `.AppImage` extension in the scanned bin directories are left to the
//! AppImage provider.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{
    ClassificationConfidence, InstallationScope, PackageManager, ProviderInventory, ProviderStatus,
    ProviderWarning, SoftwareCategory, SoftwareItem,
};
use crate::process::ProcessRunner;
use crate::providers::dpkg_ownership;
use crate::providers::{InventoryProvider, ProviderId, ProviderMetadata, ScanContext};

const MANUAL_PROVIDER_ID: ProviderId = ProviderId::new("manual");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirKind {
    Opt,
    LocalBin,
    LocalLib,
}

/// Discovers software in the bounded, documented set of locations where
/// manually installed (unmanaged) software conventionally lives: `/opt`,
/// `/usr/local/bin`, `/usr/local/lib`, and `~/.local/bin`. Every directory
/// is scanned non-recursively (`/opt` entries are captured as whole
/// top-level directories, never descended into) — see
/// `docs/SECURITY.md`.
///
/// Every candidate path is checked against dpkg ownership (batched, one
/// `dpkg -S` call for every candidate) before being reported: a path dpkg
/// already owns is not "manual" by definition and is excluded entirely
/// rather than emitted as a redundant stub record, since the APT provider
/// already fully represents that package.
pub struct ManualSoftwareProvider {
    runner: ProcessRunner,
    dpkg_bin: String,
    opt_dir: PathBuf,
    usr_local_bin_dir: PathBuf,
    usr_local_lib_dir: PathBuf,
    home_override: Option<PathBuf>,
}

impl ManualSoftwareProvider {
    pub fn new() -> Self {
        Self {
            runner: ProcessRunner::default(),
            dpkg_bin: "dpkg".to_string(),
            opt_dir: PathBuf::from("/opt"),
            usr_local_bin_dir: PathBuf::from("/usr/local/bin"),
            usr_local_lib_dir: PathBuf::from("/usr/local/lib"),
            home_override: None,
        }
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn with_test_roots(
        opt_dir: PathBuf,
        usr_local_bin_dir: PathBuf,
        usr_local_lib_dir: PathBuf,
        home: PathBuf,
        dpkg_bin: impl Into<String>,
    ) -> Self {
        Self {
            runner: ProcessRunner::default(),
            dpkg_bin: dpkg_bin.into(),
            opt_dir,
            usr_local_bin_dir,
            usr_local_lib_dir,
            home_override: Some(home),
        }
    }

    fn local_bin_dir(&self) -> Option<PathBuf> {
        self.home_override
            .clone()
            .or_else(dirs::home_dir)
            .map(|home| home.join(".local/bin"))
    }
}

impl Default for ManualSoftwareProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InventoryProvider for ManualSoftwareProvider {
    fn id(&self) -> ProviderId {
        MANUAL_PROVIDER_ID
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: MANUAL_PROVIDER_ID,
            display_name: "Manual Software",
            description:
                "Executables, libraries, and /opt installs not owned by any known package manager.",
        }
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn scan(&self, ctx: &ScanContext) -> ProviderInventory {
        let started_at = Utc::now();
        let base = ProviderInventory::started(MANUAL_PROVIDER_ID.as_str(), started_at);

        if ctx.is_cancelled() {
            return base.finish(Utc::now(), ProviderStatus::Cancelled);
        }

        let mut warnings: Vec<ProviderWarning> = Vec::new();
        let mut candidates: Vec<Candidate> = Vec::new();

        collect_opt_entries(&self.opt_dir, &mut warnings, &mut candidates);
        collect_flat_files(
            &self.usr_local_bin_dir,
            InstallationScope::System,
            DirKind::LocalBin,
            &mut warnings,
            &mut candidates,
        );
        collect_flat_files(
            &self.usr_local_lib_dir,
            InstallationScope::System,
            DirKind::LocalLib,
            &mut warnings,
            &mut candidates,
        );

        if let Some(local_bin) = self.local_bin_dir() {
            collect_flat_files(
                &local_bin,
                InstallationScope::User,
                DirKind::LocalBin,
                &mut warnings,
                &mut candidates,
            );
        }

        if ctx.is_cancelled() {
            return base.finish(Utc::now(), ProviderStatus::Cancelled);
        }

        let paths: Vec<PathBuf> = candidates.iter().map(|c| c.path.clone()).collect();
        let owners =
            match dpkg_ownership::resolve_owners(&self.runner, &self.dpkg_bin, &paths).await {
                Ok(owners) => owners,
                Err(error) => {
                    warnings.push(ProviderWarning::new(format!(
                        "could not check dpkg ownership for candidate manual paths ({error}); \
                     proceeding as if all candidates are unowned"
                    )));
                    HashMap::new()
                }
            };

        let items: Vec<SoftwareItem> = candidates
            .into_iter()
            .filter(|candidate| !is_owned(candidate, &owners))
            .map(|candidate| build_item(&candidate))
            .collect();

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

/// A candidate whose exact path dpkg reports as owned is not "manual" by
/// definition -- the APT provider already fully represents that package,
/// so it's excluded here rather than emitted as a redundant stub.
fn is_owned(candidate: &Candidate, owners: &HashMap<String, String>) -> bool {
    owners.contains_key(&candidate.path.to_string_lossy().into_owned())
}

struct Candidate {
    path: PathBuf,
    kind: DirKind,
    is_dir: bool,
    scope: InstallationScope,
    size_bytes: Option<u64>,
    modified_at: Option<DateTime<Utc>>,
    executable: bool,
}

/// Captures each top-level entry under `/opt` as one candidate,
/// representing the whole install bundle -- never descends into it.
fn collect_opt_entries(dir: &Path, warnings: &mut Vec<ProviderWarning>, out: &mut Vec<Candidate>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                warnings.push(ProviderWarning::new(format!(
                    "could not read {}: {error}",
                    dir.display()
                )));
            }
            return;
        }
    };

    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let (size_bytes, modified_at, executable) = inspect_metadata(&path);

        out.push(Candidate {
            path,
            kind: DirKind::Opt,
            is_dir,
            scope: InstallationScope::System,
            size_bytes,
            modified_at,
            executable,
        });
    }
}

/// Lists only regular files directly inside `dir` (never subdirectories,
/// never recursed into). For `DirKind::LocalBin` directories, files with
/// the `.AppImage` extension are skipped -- that's the AppImage
/// provider's territory.
fn collect_flat_files(
    dir: &Path,
    scope: InstallationScope,
    kind: DirKind,
    warnings: &mut Vec<ProviderWarning>,
    out: &mut Vec<Candidate>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                warnings.push(ProviderWarning::new(format!(
                    "could not read {}: {error}",
                    dir.display()
                )));
            }
            return;
        }
    };

    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();

        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(true) {
            continue;
        }

        if kind == DirKind::LocalBin
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("AppImage"))
                .unwrap_or(false)
        {
            continue;
        }

        let (size_bytes, modified_at, executable) = inspect_metadata(&path);

        out.push(Candidate {
            path,
            kind,
            is_dir: false,
            scope,
            size_bytes,
            modified_at,
            executable,
        });
    }
}

fn inspect_metadata(path: &Path) -> (Option<u64>, Option<DateTime<Utc>>, bool) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return (None, None, false);
    };

    let size_bytes = if metadata.is_dir() {
        None
    } else {
        Some(metadata.len())
    };
    let modified_at = metadata.modified().ok().and_then(system_time_to_utc);
    let executable = is_executable(&metadata);

    (size_bytes, modified_at, executable)
}

#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &std::fs::Metadata) -> bool {
    false
}

fn system_time_to_utc(time: SystemTime) -> Option<DateTime<Utc>> {
    let duration = time.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
}

fn build_item(candidate: &Candidate) -> SoftwareItem {
    let filename = candidate
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let (category, confidence, reason) = classify_candidate(candidate, &filename);

    let mut item = SoftwareItem::new(
        format!("manual:{}", candidate.path.display()),
        filename,
        candidate
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("(unknown)")
            .to_string(),
        PackageManager::Manual,
    );

    item.scope = candidate.scope;
    item.category = category;
    item.classification_confidence = confidence;
    item.classification_reasons = vec![reason];
    item.installed_size_bytes = candidate.size_bytes;
    item.installed_at = candidate.modified_at;

    let path_str = candidate.path.to_string_lossy().into_owned();
    if candidate.is_dir {
        item.install_paths = vec![path_str];
    } else {
        item.install_paths = vec![path_str.clone()];
        if candidate.executable {
            item.executable_paths = vec![path_str];
        }
    }

    item.metadata
        .insert("executable".to_string(), candidate.executable.to_string());
    item.metadata
        .insert("is_directory".to_string(), candidate.is_dir.to_string());

    item
}

fn classify_candidate(
    candidate: &Candidate,
    filename: &str,
) -> (SoftwareCategory, ClassificationConfidence, String) {
    match (candidate.kind, candidate.is_dir) {
        (DirKind::Opt, true) => (
            SoftwareCategory::Application,
            ClassificationConfidence::Low,
            "found as a top-level directory under /opt (commonly used for self-contained application installs), not owned by any known package manager".to_string(),
        ),
        (DirKind::Opt, false) => {
            if candidate.executable {
                (
                    SoftwareCategory::CommandLineTool,
                    ClassificationConfidence::Low,
                    "found as an executable file directly under /opt, not owned by any known package manager".to_string(),
                )
            } else {
                (
                    SoftwareCategory::Miscellaneous,
                    ClassificationConfidence::Low,
                    "found as a non-executable file directly under /opt, not owned by any known package manager".to_string(),
                )
            }
        }
        (DirKind::LocalLib, false) => {
            let looks_like_shared_object = filename.contains(".so");
            let confidence =
                if looks_like_shared_object { ClassificationConfidence::High } else { ClassificationConfidence::Medium };
            (
                SoftwareCategory::Library,
                confidence,
                "found in /usr/local/lib, not owned by any known package manager".to_string(),
            )
        }
        (DirKind::LocalBin, false) => {
            let confidence = if candidate.executable { ClassificationConfidence::Medium } else { ClassificationConfidence::Low };
            (
                SoftwareCategory::CommandLineTool,
                confidence,
                "found in a local bin directory, not owned by any known package manager".to_string(),
            )
        }
        // collect_flat_files() never sets is_dir for LocalBin/LocalLib (it
        // skips directories outright), but the match must still be
        // exhaustive over the type.
        (DirKind::LocalLib | DirKind::LocalBin, true) => (
            SoftwareCategory::Unclassified,
            ClassificationConfidence::Unknown,
            "unexpected directory entry outside /opt -- not descended into".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn make_temp_dir(name: &str) -> TempDir {
        let path =
            std::env::temp_dir().join(format!("kunger-manual-test-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir { path }
    }

    fn write_executable(dir: &Path, name: &str) {
        let path = dir.join(name);
        std::fs::write(&path, b"#!/bin/sh\necho hi\n").expect("write file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }
    }

    fn write_plain_file(dir: &Path, name: &str) {
        std::fs::write(dir.join(name), b"data").expect("write file");
    }

    #[test]
    fn collect_opt_entries_captures_directories_without_descending_into_them() {
        let opt = make_temp_dir("opt");
        std::fs::create_dir_all(opt.path().join("google-chrome/resources")).expect("nested dir");
        write_plain_file(&opt.path().join("google-chrome"), "chrome-binary");

        let mut warnings = Vec::new();
        let mut out = Vec::new();
        collect_opt_entries(opt.path(), &mut warnings, &mut out);

        assert_eq!(out.len(), 1);
        assert!(out[0].is_dir);
        assert_eq!(out[0].path, opt.path().join("google-chrome"));
    }

    #[test]
    fn collect_flat_files_skips_subdirectories_and_appimage_files_in_bin_dirs() {
        let bin = make_temp_dir("bin");
        write_executable(bin.path(), "my-tool");
        write_executable(bin.path(), "SomeApp.AppImage");
        std::fs::create_dir_all(bin.path().join("nested")).expect("nested dir");

        let mut warnings = Vec::new();
        let mut out = Vec::new();
        collect_flat_files(
            bin.path(),
            InstallationScope::User,
            DirKind::LocalBin,
            &mut warnings,
            &mut out,
        );

        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].path.file_name().and_then(|n| n.to_str()),
            Some("my-tool")
        );
    }

    #[test]
    fn collect_flat_files_does_not_skip_appimage_extension_for_lib_dirs() {
        let lib = make_temp_dir("lib-appimage-name");
        write_plain_file(lib.path(), "weird.AppImage");

        let mut warnings = Vec::new();
        let mut out = Vec::new();
        collect_flat_files(
            lib.path(),
            InstallationScope::System,
            DirKind::LocalLib,
            &mut warnings,
            &mut out,
        );

        assert_eq!(out.len(), 1);
    }

    #[test]
    fn missing_directory_produces_no_warning() {
        let mut warnings = Vec::new();
        let mut out = Vec::new();
        collect_opt_entries(
            Path::new("/kunger/definitely/does/not/exist"),
            &mut warnings,
            &mut out,
        );

        assert!(warnings.is_empty());
        assert!(out.is_empty());
    }

    #[test]
    fn build_item_for_opt_directory_is_application_with_low_confidence() {
        let candidate = Candidate {
            path: PathBuf::from("/opt/my-app"),
            kind: DirKind::Opt,
            is_dir: true,
            scope: InstallationScope::System,
            size_bytes: None,
            modified_at: None,
            executable: false,
        };

        let item = build_item(&candidate);

        assert_eq!(item.category, SoftwareCategory::Application);
        assert_eq!(
            item.classification_confidence,
            ClassificationConfidence::Low
        );
        assert_eq!(item.package_manager, PackageManager::Manual);
        assert_eq!(item.id, "manual:/opt/my-app");
    }

    #[test]
    fn build_item_for_shared_object_in_local_lib_has_high_confidence() {
        let candidate = Candidate {
            path: PathBuf::from("/usr/local/lib/libcustom.so.1"),
            kind: DirKind::LocalLib,
            is_dir: false,
            scope: InstallationScope::System,
            size_bytes: Some(2048),
            modified_at: None,
            executable: false,
        };

        let item = build_item(&candidate);

        assert_eq!(item.category, SoftwareCategory::Library);
        assert_eq!(
            item.classification_confidence,
            ClassificationConfidence::High
        );
    }

    #[test]
    fn build_item_for_local_bin_executable_is_command_line_tool() {
        let candidate = Candidate {
            path: PathBuf::from("/home/user/.local/bin/my-tool"),
            kind: DirKind::LocalBin,
            is_dir: false,
            scope: InstallationScope::User,
            size_bytes: Some(512),
            modified_at: None,
            executable: true,
        };

        let item = build_item(&candidate);

        assert_eq!(item.category, SoftwareCategory::CommandLineTool);
        assert_eq!(
            item.classification_confidence,
            ClassificationConfidence::Medium
        );
        assert_eq!(
            item.executable_paths,
            vec!["/home/user/.local/bin/my-tool".to_string()]
        );
    }

    #[tokio::test]
    async fn is_available_is_always_true() {
        assert!(ManualSoftwareProvider::new().is_available().await);
    }

    #[tokio::test]
    async fn scan_reports_cancelled_immediately_when_the_context_is_already_cancelled() {
        let provider = ManualSoftwareProvider::new();
        let ctx = ScanContext::new(Duration::from_secs(5));
        ctx.cancellation.cancel();

        let result = provider.scan(&ctx).await;

        assert_eq!(result.status, ProviderStatus::Cancelled);
    }

    #[test]
    fn is_owned_excludes_exactly_the_paths_dpkg_reports() {
        let owned = Candidate {
            path: PathBuf::from("/usr/local/bin/owned-tool"),
            kind: DirKind::LocalBin,
            is_dir: false,
            scope: InstallationScope::System,
            size_bytes: Some(1),
            modified_at: None,
            executable: true,
        };
        let unowned = Candidate {
            path: PathBuf::from("/usr/local/bin/unowned-tool"),
            kind: DirKind::LocalBin,
            is_dir: false,
            scope: InstallationScope::System,
            size_bytes: Some(1),
            modified_at: None,
            executable: true,
        };
        let owners: HashMap<String, String> = [(
            "/usr/local/bin/owned-tool".to_string(),
            "some-package".to_string(),
        )]
        .into_iter()
        .collect();

        assert!(is_owned(&owned, &owners));
        assert!(!is_owned(&unowned, &owners));
    }

    #[tokio::test]
    async fn scan_degrades_gracefully_when_dpkg_is_unavailable_and_still_reports_candidates() {
        let opt = make_temp_dir("scan-opt");
        let bin = make_temp_dir("scan-bin");
        let lib = make_temp_dir("scan-lib");
        let home = make_temp_dir("scan-home");
        std::fs::create_dir_all(home.path().join(".local/bin")).expect("local bin");

        write_executable(bin.path(), "owned-tool");
        write_executable(bin.path(), "unowned-tool");

        let provider = ManualSoftwareProvider::with_test_roots(
            opt.path().to_path_buf(),
            bin.path().to_path_buf(),
            lib.path().to_path_buf(),
            home.path().to_path_buf(),
            "kunger-nonexistent-dpkg-xyz",
        );

        let ctx = ScanContext::new(Duration::from_secs(5));
        let result = provider.scan(&ctx).await;

        // dpkg is unavailable in this test (nonexistent binary), so
        // ownership resolution degrades gracefully: both files still
        // surface as manual (ownership can't be checked, not assumed
        // negative), with a warning recorded rather than a failed scan.
        assert_eq!(result.items.len(), 2);
        assert!(!result.warnings.is_empty());
        assert_eq!(result.status, ProviderStatus::PartialSuccess);
    }

    #[tokio::test]
    async fn scan_finds_candidates_across_all_scan_roots() {
        let opt = make_temp_dir("full-opt");
        let bin = make_temp_dir("full-bin");
        let lib = make_temp_dir("full-lib");
        let home = make_temp_dir("full-home");
        std::fs::create_dir_all(home.path().join(".local/bin")).expect("local bin");

        std::fs::create_dir_all(opt.path().join("my-app")).expect("opt app dir");
        write_executable(bin.path(), "sys-tool");
        write_plain_file(lib.path(), "libfoo.so.1");
        write_executable(&home.path().join(".local/bin"), "user-tool");

        let provider = ManualSoftwareProvider::with_test_roots(
            opt.path().to_path_buf(),
            bin.path().to_path_buf(),
            lib.path().to_path_buf(),
            home.path().to_path_buf(),
            "kunger-nonexistent-dpkg-xyz",
        );

        let ctx = ScanContext::new(Duration::from_secs(5));
        let result = provider.scan(&ctx).await;

        assert_eq!(result.items.len(), 4);
        assert!(result
            .items
            .iter()
            .any(|i| i.category == SoftwareCategory::Application));
        assert!(result
            .items
            .iter()
            .any(|i| i.category == SoftwareCategory::Library));
        assert!(result
            .items
            .iter()
            .any(|i| i.scope == InstallationScope::User));
        assert!(result
            .items
            .iter()
            .any(|i| i.scope == InstallationScope::System));
    }
}
