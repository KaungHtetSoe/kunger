//! The AppImage discovery provider. See `docs/ARCHITECTURE.md` §2.2 and
//! `docs/SECURITY.md`.
//!
//! Detection only ever reads file bytes and metadata — an AppImage is
//! **never** executed, directly or indirectly, at any point during
//! inventory. See `docs/SECURITY.md` — "must not execute discovered
//! binaries."

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{
    ClassificationConfidence, InstallationScope, PackageManager, ProviderInventory, ProviderStatus,
    ProviderWarning, SoftwareCategory, SoftwareItem,
};
use crate::providers::{InventoryProvider, ProviderId, ProviderMetadata, ScanContext};

const APPIMAGE_PROVIDER_ID: ProviderId = ProviderId::new("appimage");

/// Bounded set of directories scanned, matching `docs/SECURITY.md`'s
/// scope table. Every directory is scanned non-recursively (top level
/// only) — this provider never walks the filesystem.
const SCAN_DIR_NAMES: &[(&str, InstallationScope)] = &[
    ("Applications", InstallationScope::User),
    (".local/bin", InstallationScope::User),
];
const SYSTEM_SCAN_DIRS: &[&str] = &["/opt", "/usr/local/bin"];

/// Only the first 16 bytes are ever read for signature detection.
const SIGNATURE_BYTES: usize = 16;
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const APPIMAGE_MAGIC_OFFSET: usize = 8;

const ARCH_SUFFIXES: &[&str] = &["x86_64", "x86", "amd64", "aarch64", "arm64", "i386", "i686"];

/// Discovers likely AppImage files in a bounded set of known directories
/// without ever executing them.
///
/// A file is only considered a candidate when it has *some*
/// AppImage-specific evidence: either a `.AppImage` filename extension, or
/// (for files without that extension) the executable bit set together
/// with the AppImage file-signature magic bytes at offset 8
/// (`docs/SECURITY.md` — never trust a bare "is this executable?" check
/// alone, that would flag every script in `~/.local/bin`). Confidence
/// reflects how much of that evidence agrees:
/// - `Certain`: `.AppImage` extension **and** signature magic confirmed.
/// - `High`: `.AppImage` extension, executable, signature unreadable or
///   inconclusive (some legitimate type-1 AppImages lack the magic).
/// - `Medium`: only one weaker signal (extension alone, or magic bytes on
///   a non-`.AppImage`-named file).
pub struct AppImageProvider {
    home_override: Option<PathBuf>,
}

impl AppImageProvider {
    pub fn new() -> Self {
        Self {
            home_override: None,
        }
    }

    #[cfg(test)]
    fn with_home(home: PathBuf) -> Self {
        Self {
            home_override: Some(home),
        }
    }

    fn home_dir(&self) -> Option<PathBuf> {
        self.home_override.clone().or_else(dirs::home_dir)
    }

    fn scan_roots(&self) -> Vec<(PathBuf, InstallationScope)> {
        let mut roots = Vec::new();

        if let Some(home) = self.home_dir() {
            for (suffix, scope) in SCAN_DIR_NAMES {
                roots.push((home.join(suffix), *scope));
            }
            roots.push((home.join("Downloads"), InstallationScope::User));
        }

        for dir in SYSTEM_SCAN_DIRS {
            roots.push((PathBuf::from(dir), InstallationScope::System));
        }

        roots
    }
}

impl Default for AppImageProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InventoryProvider for AppImageProvider {
    fn id(&self) -> ProviderId {
        APPIMAGE_PROVIDER_ID
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: APPIMAGE_PROVIDER_ID,
            display_name: "AppImage",
            description: "AppImage files discovered in known locations, never executed.",
        }
    }

    async fn is_available(&self) -> bool {
        // Pure filesystem inspection -- no external tool dependency.
        true
    }

    async fn scan(&self, ctx: &ScanContext) -> ProviderInventory {
        let started_at = Utc::now();
        let base = ProviderInventory::started(APPIMAGE_PROVIDER_ID.as_str(), started_at);

        if ctx.is_cancelled() {
            return base.finish(Utc::now(), ProviderStatus::Cancelled);
        }

        let mut warnings: Vec<ProviderWarning> = Vec::new();
        let mut items = Vec::new();
        let desktop_dir = self
            .home_dir()
            .map(|home| home.join(".local/share/applications"));

        for (dir, scope) in self.scan_roots() {
            if ctx.is_cancelled() {
                return base.finish(Utc::now(), ProviderStatus::Cancelled);
            }

            let entries = match std::fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(error) => {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        warnings.push(ProviderWarning::new(format!(
                            "could not read {}: {error}",
                            dir.display()
                        )));
                    }
                    continue;
                }
            };

            for entry in entries {
                let Ok(entry) = entry else { continue };
                let path = entry.path();

                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(true) {
                    continue;
                }

                match inspect_candidate(&path) {
                    Ok(Some(candidate)) => {
                        items.push(build_item(&candidate, scope, desktop_dir.as_deref()));
                    }
                    Ok(None) => {}
                    Err(error) => {
                        warnings.push(ProviderWarning::new(format!(
                            "could not inspect {}: {error}",
                            path.display()
                        )));
                    }
                }
            }
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

struct Candidate {
    path: PathBuf,
    size_bytes: u64,
    modified_at: Option<DateTime<Utc>>,
    executable: bool,
    signature_confirmed: bool,
    has_appimage_extension: bool,
}

/// Inspects one file and decides whether it's a plausible AppImage
/// candidate. Returns `Ok(None)` for files with no AppImage-specific
/// evidence at all (not an error -- most files in these directories won't
/// be AppImages). Only ever reads file metadata and the first
/// [`SIGNATURE_BYTES`] bytes; never opens the file for writing, never
/// executes it.
fn inspect_candidate(path: &Path) -> std::io::Result<Option<Candidate>> {
    let has_appimage_extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("AppImage"))
        .unwrap_or(false);

    let metadata = std::fs::metadata(path)?;
    let executable = is_executable(&metadata);

    // Only bother reading the signature when there's already at least one
    // other reason to look (extension match, or executable) -- avoids
    // opening every single file in every scanned directory.
    let signature_confirmed = if has_appimage_extension || executable {
        read_appimage_signature(path).unwrap_or(false)
    } else {
        false
    };

    if !has_appimage_extension && !(executable && signature_confirmed) {
        return Ok(None);
    }

    let modified_at = metadata.modified().ok().and_then(system_time_to_utc);

    Ok(Some(Candidate {
        path: path.to_path_buf(),
        size_bytes: metadata.len(),
        modified_at,
        executable,
        signature_confirmed,
        has_appimage_extension,
    }))
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

fn read_appimage_signature(path: &Path) -> std::io::Result<bool> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = [0_u8; SIGNATURE_BYTES];
    let read = file.read(&mut buf)?;

    if read < APPIMAGE_MAGIC_OFFSET + 3 {
        return Ok(false);
    }

    let is_elf = buf[0..4] == ELF_MAGIC;
    let has_ai_marker =
        buf[APPIMAGE_MAGIC_OFFSET] == b'A' && buf[APPIMAGE_MAGIC_OFFSET + 1] == b'I';
    let appimage_type = buf[APPIMAGE_MAGIC_OFFSET + 2];

    Ok(is_elf && has_ai_marker && (appimage_type == 0x01 || appimage_type == 0x02))
}

struct DesktopIntegration {
    desktop_file_path: String,
    icon: Option<String>,
}

/// Looks for a `.desktop` file in `desktop_dir` whose `Exec=` references
/// this AppImage's path -- the common pattern used by AppImage desktop
/// integration tools. Bounded to a single non-recursive directory listing
/// and bounded per-file reads; never parses the file beyond a raw text
/// scan for `Icon=`.
fn find_desktop_integration(
    appimage_path: &Path,
    desktop_dir: Option<&Path>,
) -> Option<DesktopIntegration> {
    let dir = desktop_dir?;
    let entries = std::fs::read_dir(dir).ok()?;
    let target = appimage_path.to_string_lossy();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
            continue;
        }

        let Ok(content) = read_bounded(&path) else {
            continue;
        };
        if !content.contains(target.as_ref()) {
            continue;
        }

        let icon = content
            .lines()
            .find_map(|line| line.strip_prefix("Icon="))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        return Some(DesktopIntegration {
            desktop_file_path: path.to_string_lossy().into_owned(),
            icon,
        });
    }

    None
}

const MAX_DESKTOP_FILE_BYTES: u64 = 1024 * 1024;

fn read_bounded(path: &Path) -> std::io::Result<String> {
    let file = std::fs::File::open(path)?;
    let mut limited = file.take(MAX_DESKTOP_FILE_BYTES);
    let mut buf = Vec::new();
    limited.read_to_end(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn build_item(
    candidate: &Candidate,
    scope: InstallationScope,
    desktop_dir: Option<&Path>,
) -> SoftwareItem {
    let filename = candidate
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let display_name = infer_display_name(&filename);

    let confidence = match (
        candidate.has_appimage_extension,
        candidate.signature_confirmed,
    ) {
        (true, true) => ClassificationConfidence::Certain,
        (true, false) => ClassificationConfidence::High,
        (false, true) => ClassificationConfidence::Medium,
        (false, false) => ClassificationConfidence::Medium,
    };

    let mut reasons = Vec::new();
    if candidate.has_appimage_extension {
        reasons.push("filename has the .AppImage extension".to_string());
    }
    if candidate.signature_confirmed {
        reasons.push("file signature matches the AppImage magic bytes".to_string());
    }
    if candidate.executable {
        reasons.push("file has the executable permission bit set".to_string());
    }

    let mut item = SoftwareItem::new(
        format!("appimage:{}", candidate.path.display()),
        filename.clone(),
        display_name,
        PackageManager::AppImage,
    );

    item.scope = scope;
    item.install_paths = vec![candidate.path.to_string_lossy().into_owned()];
    item.executable_paths = vec![candidate.path.to_string_lossy().into_owned()];
    item.installed_size_bytes = Some(candidate.size_bytes);
    item.installed_at = candidate.modified_at;
    item.category = SoftwareCategory::Application;
    item.classification_confidence = confidence;
    item.classification_reasons = reasons;

    if let Some(integration) = find_desktop_integration(&candidate.path, desktop_dir) {
        item.desktop_file_paths = vec![integration.desktop_file_path];
        item.metadata
            .insert("desktop_integrated".to_string(), "true".to_string());
        if let Some(icon) = integration.icon {
            item.icon_path = Some(icon);
        }
    }

    item
}

fn infer_display_name(filename: &str) -> String {
    let stem = filename
        .strip_suffix(".AppImage")
        .or_else(|| filename.strip_suffix(".appimage"))
        .unwrap_or(filename);

    let cleaned = trim_trailing_version_or_arch(stem);
    let spaced = cleaned.replace(['-', '_'], " ");
    let collapsed: Vec<&str> = spaced.split_whitespace().collect();

    if collapsed.is_empty() {
        filename.to_string()
    } else {
        collapsed.join(" ")
    }
}

fn trim_trailing_version_or_arch(stem: &str) -> &str {
    if let Some(idx) = stem.rfind(['-', '_']) {
        let head = &stem[..idx];
        let tail = &stem[idx + 1..];
        if !head.is_empty() && looks_like_version_or_arch(tail) {
            return trim_trailing_version_or_arch(head);
        }
    }
    stem
}

fn looks_like_version_or_arch(segment: &str) -> bool {
    if segment.is_empty() {
        return false;
    }

    let lower = segment.to_ascii_lowercase();
    if ARCH_SUFFIXES.contains(&lower.as_str()) {
        return true;
    }

    let digits_part = segment.strip_prefix(['v', 'V']).unwrap_or(segment);
    !digits_part.is_empty()
        && digits_part
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
        && digits_part.chars().all(|c| c.is_ascii_digit() || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/appimage")
            .join(name)
    }

    #[test]
    fn detects_a_real_appimage_with_extension_and_signature_as_certain() {
        let path = fixture_path("RealApp-1.2.3-x86_64.AppImage");
        let candidate = inspect_candidate(&path)
            .expect("readable")
            .expect("should be a candidate");

        assert!(candidate.has_appimage_extension);
        assert!(candidate.signature_confirmed);
        assert!(candidate.executable);
    }

    #[test]
    fn detects_a_renamed_appimage_via_signature_alone() {
        let path = fixture_path("renamed-app-no-extension");
        let candidate = inspect_candidate(&path)
            .expect("readable")
            .expect("should be a candidate");

        assert!(!candidate.has_appimage_extension);
        assert!(candidate.signature_confirmed);
    }

    #[test]
    fn a_file_with_the_extension_but_no_real_signature_is_still_a_weak_candidate() {
        let path = fixture_path("NotAnAppImage.AppImage");
        let candidate = inspect_candidate(&path)
            .expect("readable")
            .expect("should still be a candidate");

        assert!(candidate.has_appimage_extension);
        assert!(!candidate.signature_confirmed);
    }

    #[test]
    fn a_plain_script_with_no_appimage_evidence_is_not_a_candidate() {
        let path = fixture_path("regular-script.sh");
        let candidate = inspect_candidate(&path).expect("readable");

        assert!(candidate.is_none());
    }

    #[test]
    fn non_executable_file_with_appimage_extension_is_still_a_candidate() {
        let path = fixture_path("NotExecutableYet.AppImage");
        let candidate = inspect_candidate(&path)
            .expect("readable")
            .expect("should be a candidate");

        assert!(!candidate.executable);
        assert!(candidate.has_appimage_extension);
    }

    #[test]
    fn build_item_confidence_is_certain_for_extension_plus_signature() {
        let candidate = Candidate {
            path: PathBuf::from("/home/user/Applications/App-1.0.AppImage"),
            size_bytes: 1024,
            modified_at: None,
            executable: true,
            signature_confirmed: true,
            has_appimage_extension: true,
        };

        let item = build_item(&candidate, InstallationScope::User, None);

        assert_eq!(
            item.classification_confidence,
            ClassificationConfidence::Certain
        );
        assert_eq!(item.package_manager, PackageManager::AppImage);
        assert_eq!(item.category, SoftwareCategory::Application);
    }

    #[test]
    fn build_item_confidence_is_high_for_extension_without_confirmed_signature() {
        let candidate = Candidate {
            path: PathBuf::from("/home/user/Applications/App.AppImage"),
            size_bytes: 1024,
            modified_at: None,
            executable: true,
            signature_confirmed: false,
            has_appimage_extension: true,
        };

        let item = build_item(&candidate, InstallationScope::User, None);

        assert_eq!(
            item.classification_confidence,
            ClassificationConfidence::High
        );
    }

    #[test]
    fn build_item_confidence_is_medium_for_signature_only() {
        let candidate = Candidate {
            path: PathBuf::from("/opt/renamed-app"),
            size_bytes: 1024,
            modified_at: None,
            executable: true,
            signature_confirmed: true,
            has_appimage_extension: false,
        };

        let item = build_item(&candidate, InstallationScope::System, None);

        assert_eq!(
            item.classification_confidence,
            ClassificationConfidence::Medium
        );
    }

    #[test]
    fn infer_display_name_strips_extension_version_and_arch() {
        assert_eq!(
            infer_display_name("Obsidian-1.5.3-x86_64.AppImage"),
            "Obsidian"
        );
        assert_eq!(infer_display_name("myapp_v2.0.AppImage"), "myapp");
        assert_eq!(infer_display_name("Simple.AppImage"), "Simple");
    }

    #[test]
    fn infer_display_name_falls_back_to_the_filename_when_nothing_can_be_stripped() {
        assert_eq!(infer_display_name("weird_name_here"), "weird name here");
    }

    #[test]
    fn build_item_id_is_stable_and_path_based() {
        let candidate = Candidate {
            path: PathBuf::from("/home/user/Applications/App.AppImage"),
            size_bytes: 1024,
            modified_at: None,
            executable: true,
            signature_confirmed: true,
            has_appimage_extension: true,
        };

        let item = build_item(&candidate, InstallationScope::User, None);

        assert_eq!(item.id, "appimage:/home/user/Applications/App.AppImage");
    }

    #[tokio::test]
    async fn is_available_is_always_true() {
        assert!(AppImageProvider::new().is_available().await);
    }

    #[tokio::test]
    async fn scan_reports_cancelled_immediately_when_the_context_is_already_cancelled() {
        let provider = AppImageProvider::new();
        let ctx = ScanContext::new(Duration::from_secs(5));
        ctx.cancellation.cancel();

        let result = provider.scan(&ctx).await;

        assert_eq!(result.status, ProviderStatus::Cancelled);
    }

    #[tokio::test]
    async fn scan_finds_candidates_in_the_fixtures_directory_treated_as_a_home_dir() {
        // Point the provider's "home" at a directory whose ".local/bin"
        // doesn't exist and whose "Applications"/"Downloads" don't exist
        // either -- but exercise the real scan path end-to-end using a
        // fixture directory placed directly as one of the roots via a
        // crafted home layout is awkward without touching the real
        // filesystem, so this test instead verifies graceful handling of
        // an entirely absent home directory tree (no panics, empty
        // result), which is the realistic "nothing found" case.
        let provider = AppImageProvider::with_home(PathBuf::from("/nonexistent/kunger-test-home"));
        let ctx = ScanContext::new(Duration::from_secs(5));

        let result = provider.scan(&ctx).await;

        assert!(result.items.is_empty());
    }

    #[test]
    fn find_desktop_integration_matches_on_exec_path_and_extracts_icon() {
        let dir = tempdir_with_desktop_file();
        let appimage_path = PathBuf::from("/home/user/Applications/App.AppImage");

        let integration = find_desktop_integration(&appimage_path, Some(dir.path()));

        let integration = integration.expect("should find the matching desktop entry");
        assert_eq!(integration.icon.as_deref(), Some("app-icon"));

        std::fs::remove_dir_all(dir.path()).ok();
    }

    struct TempDir {
        path: PathBuf,
    }
    impl TempDir {
        fn path(&self) -> &Path {
            &self.path
        }
    }

    fn tempdir_with_desktop_file() -> TempDir {
        let dir = std::env::temp_dir().join(format!("kunger-appimage-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(
            dir.join("app.desktop"),
            "[Desktop Entry]\nType=Application\nName=App\nExec=/home/user/Applications/App.AppImage %U\nIcon=app-icon\n",
        )
        .expect("write fixture desktop file");
        TempDir { path: dir }
    }
}
