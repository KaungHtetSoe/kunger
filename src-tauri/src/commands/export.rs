//! `export_inventory`.
//!
//! Implements both export modes (JSON/YAML/CSV in each): the full technical
//! inventory dump, and the reinstallation manifest, which separates items
//! Kunger can point a package manager at by name from items it can only
//! flag for manual review (product spec FR-11).

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;

use crate::domain::{InstallationReason, PackageManager, SoftwareItem};

use super::{
    run_blocking, AppState, CommandError, ExportFormat, ExportMode, ExportRequest, ExportResponse,
};

const EXPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportedInventory {
    schema_version: u32,
    exported_at: chrono::DateTime<Utc>,
    item_count: usize,
    items: Vec<SoftwareItem>,
}

pub async fn export_inventory_impl(
    state: &AppState,
    request: ExportRequest,
) -> Result<ExportResponse, CommandError> {
    let repository = Arc::clone(&state.repository);
    let items = run_blocking(move || repository.latest_items()).await?;

    let content = match (request.mode, request.format) {
        (ExportMode::Full, ExportFormat::Json) => export_json(&items)?,
        (ExportMode::Full, ExportFormat::Yaml) => export_yaml(&items)?,
        (ExportMode::Full, ExportFormat::Csv) => export_csv(&items)?,
        (ExportMode::ReinstallationManifest, format) => export_manifest(&items, format)?,
    };

    Ok(ExportResponse {
        schema_version: EXPORT_SCHEMA_VERSION,
        format: request.format,
        content,
    })
}

fn export_json(items: &[SoftwareItem]) -> Result<String, CommandError> {
    let payload = ExportedInventory {
        schema_version: EXPORT_SCHEMA_VERSION,
        exported_at: Utc::now(),
        item_count: items.len(),
        items: items.to_vec(),
    };
    serde_json::to_string_pretty(&payload)
        .map_err(|e| CommandError::internal(format!("failed to serialize JSON export: {e}")))
}

fn export_yaml(items: &[SoftwareItem]) -> Result<String, CommandError> {
    let payload = ExportedInventory {
        schema_version: EXPORT_SCHEMA_VERSION,
        exported_at: Utc::now(),
        item_count: items.len(),
        items: items.to_vec(),
    };
    serde_yaml::to_string(&payload)
        .map_err(|e| CommandError::internal(format!("failed to serialize YAML export: {e}")))
}

/// Neutralizes CSV/spreadsheet formula injection (CWE-1236). A scanned
/// package id, name, or version is normally safe, but it ultimately comes
/// from package metadata Kunger doesn't control -- a malicious or corrupted
/// package could in principle set a name like `=cmd|'/c calc'!A1`, which
/// Excel/LibreOffice Calc/Google Sheets treat as a formula when the
/// exported CSV is opened, not literal text. Prefixing such values with a
/// single quote forces spreadsheet apps to treat the cell as text; it does
/// not affect CSV-syntax escaping (commas/quotes), which the `csv` crate
/// already handles separately.
fn csv_safe(value: &str) -> String {
    match value.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{value}"),
        _ => value.to_string(),
    }
}

fn export_csv(items: &[SoftwareItem]) -> Result<String, CommandError> {
    let mut writer = csv::Writer::from_writer(Vec::new());

    writer
        .write_record([
            "id",
            "packageName",
            "displayName",
            "category",
            "packageManager",
            "scope",
            "version",
            "installedSizeBytes",
            "updateAvailable",
            "classificationConfidence",
        ])
        .map_err(|e| CommandError::internal(format!("failed to write CSV header: {e}")))?;

    for item in items {
        writer
            .write_record([
                csv_safe(&item.id),
                csv_safe(&item.package_name),
                csv_safe(&item.display_name),
                format!("{:?}", item.category),
                format!("{:?}", item.package_manager),
                format!("{:?}", item.scope),
                csv_safe(item.version.as_deref().unwrap_or("")),
                item.installed_size_bytes
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                item.update_available.to_string(),
                format!("{:?}", item.classification_confidence),
            ])
            .map_err(|e| {
                CommandError::internal(format!("failed to write CSV row for {}: {e}", item.id))
            })?;
    }

    let bytes = writer
        .into_inner()
        .map_err(|e| CommandError::internal(format!("failed to finalize CSV export: {e}")))?;
    String::from_utf8(bytes)
        .map_err(|e| CommandError::internal(format!("CSV export was not valid UTF-8: {e}")))
}

/// Package managers Kunger can point at a package name to reinstall
/// non-interactively. Order here is the order groups appear in the manifest.
const REPRODUCIBLE_MANAGERS: &[PackageManager] = &[
    PackageManager::Apt,
    PackageManager::Flatpak,
    PackageManager::Snap,
    PackageManager::Pip,
    PackageManager::Pipx,
    PackageManager::Npm,
    PackageManager::Cargo,
];

fn install_hint(manager: PackageManager) -> &'static str {
    match manager {
        PackageManager::Apt => "sudo apt install <package names>",
        PackageManager::Flatpak => "flatpak install <package names>",
        PackageManager::Snap => "sudo snap install <package names>",
        PackageManager::Pip => "pip install <package names>",
        PackageManager::Pipx => "pipx install <package names>",
        PackageManager::Npm => "npm install -g <package names>",
        PackageManager::Cargo => "cargo install <package names>",
        PackageManager::AppImage | PackageManager::Manual | PackageManager::Unknown => {
            "no automatic install command"
        }
    }
}

fn manual_review_reason(item: &SoftwareItem) -> String {
    match item.package_manager {
        PackageManager::AppImage => {
            "AppImage bundle -- no package registry entry; keep or re-download the file manually."
                .to_string()
        }
        PackageManager::Manual => {
            "Found in a local bin/lib/opt directory with no owning package manager; review manually."
                .to_string()
        }
        _ => "Kunger could not determine a package manager for this item; review manually."
            .to_string(),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReinstallManifest {
    schema_version: u32,
    exported_at: chrono::DateTime<Utc>,
    /// Items whose package manager can reinstall them by name -- run each
    /// group's `installHint` with its `packages` substituted in.
    reproducible: Vec<ReproducibleGroup>,
    /// Items Kunger cannot automatically reproduce (see `docs/PRODUCT_SPEC.md`
    /// FR-11). Installation paths are included here, and may contain the
    /// user's home directory / username -- the export UI discloses this.
    manual_review: Vec<ManualReviewItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReproducibleGroup {
    package_manager: PackageManager,
    install_hint: String,
    packages: Vec<ReproduciblePackage>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReproduciblePackage {
    package_name: String,
    display_name: String,
    version: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManualReviewItem {
    id: String,
    display_name: String,
    package_manager: PackageManager,
    reason: String,
    paths: Vec<String>,
}

/// Items installed automatically as a dependency are left out of the
/// manifest entirely: reinstalling the manually-chosen packages in
/// `reproducible` pulls them back in via normal dependency resolution, so
/// listing them separately would just be noise.
fn build_manifest(items: &[SoftwareItem]) -> ReinstallManifest {
    let mut grouped: HashMap<PackageManager, Vec<ReproduciblePackage>> = HashMap::new();
    let mut manual_review = Vec::new();

    for item in items {
        if item.installation_reason == InstallationReason::Automatic {
            continue;
        }

        if REPRODUCIBLE_MANAGERS.contains(&item.package_manager) {
            grouped
                .entry(item.package_manager)
                .or_default()
                .push(ReproduciblePackage {
                    package_name: item.package_name.clone(),
                    display_name: item.display_name.clone(),
                    version: item.version.clone(),
                });
        } else {
            manual_review.push(ManualReviewItem {
                id: item.id.clone(),
                display_name: item.display_name.clone(),
                package_manager: item.package_manager,
                reason: manual_review_reason(item),
                paths: item.install_paths.clone(),
            });
        }
    }

    let reproducible = REPRODUCIBLE_MANAGERS
        .iter()
        .filter_map(|manager| {
            grouped.remove(manager).map(|mut packages| {
                packages.sort_by(|a, b| a.package_name.cmp(&b.package_name));
                ReproducibleGroup {
                    package_manager: *manager,
                    install_hint: install_hint(*manager).to_string(),
                    packages,
                }
            })
        })
        .collect();

    manual_review.sort_by(|a, b| a.display_name.cmp(&b.display_name));

    ReinstallManifest {
        schema_version: EXPORT_SCHEMA_VERSION,
        exported_at: Utc::now(),
        reproducible,
        manual_review,
    }
}

fn export_manifest(items: &[SoftwareItem], format: ExportFormat) -> Result<String, CommandError> {
    let manifest = build_manifest(items);
    match format {
        ExportFormat::Json => serde_json::to_string_pretty(&manifest)
            .map_err(|e| CommandError::internal(format!("failed to serialize manifest: {e}"))),
        ExportFormat::Yaml => serde_yaml::to_string(&manifest)
            .map_err(|e| CommandError::internal(format!("failed to serialize manifest: {e}"))),
        ExportFormat::Csv => export_manifest_csv(&manifest),
    }
}

fn export_manifest_csv(manifest: &ReinstallManifest) -> Result<String, CommandError> {
    let mut writer = csv::Writer::from_writer(Vec::new());

    writer
        .write_record([
            "reproducible",
            "packageManager",
            "packageName",
            "displayName",
            "version",
            "paths",
            "reason",
        ])
        .map_err(|e| CommandError::internal(format!("failed to write CSV header: {e}")))?;

    for group in &manifest.reproducible {
        for package in &group.packages {
            writer
                .write_record([
                    "yes".to_string(),
                    format!("{:?}", group.package_manager),
                    csv_safe(&package.package_name),
                    csv_safe(&package.display_name),
                    csv_safe(package.version.as_deref().unwrap_or("")),
                    String::new(),
                    group.install_hint.clone(),
                ])
                .map_err(|e| CommandError::internal(format!("failed to write CSV row: {e}")))?;
        }
    }

    for item in &manifest.manual_review {
        writer
            .write_record([
                "no".to_string(),
                format!("{:?}", item.package_manager),
                String::new(),
                csv_safe(&item.display_name),
                String::new(),
                csv_safe(&item.paths.join("; ")),
                item.reason.clone(),
            ])
            .map_err(|e| {
                CommandError::internal(format!("failed to write CSV row for {}: {e}", item.id))
            })?;
    }

    let bytes = writer
        .into_inner()
        .map_err(|e| CommandError::internal(format!("failed to finalize CSV export: {e}")))?;
    String::from_utf8(bytes)
        .map_err(|e| CommandError::internal(format!("manifest CSV was not valid UTF-8: {e}")))
}

#[tauri::command]
pub async fn export_inventory(
    state: tauri::State<'_, Arc<AppState>>,
    request: ExportRequest,
) -> Result<ExportResponse, CommandError> {
    export_inventory_impl(state.inner(), request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::{state_after_scan, test_state};
    use crate::domain::PackageManager;
    use crate::providers::mock::MockInventoryProvider;

    async fn state_with_items(items: Vec<SoftwareItem>) -> AppState {
        state_after_scan(vec![Box::new(
            MockInventoryProvider::new("apt").with_items(items),
        )])
        .await
    }

    async fn state_with_one_item() -> AppState {
        state_with_items(vec![SoftwareItem::new(
            "apt:git",
            "git",
            "Git",
            PackageManager::Apt,
        )])
        .await
    }

    fn full_request(format: ExportFormat) -> ExportRequest {
        ExportRequest {
            format,
            mode: ExportMode::Full,
        }
    }

    fn manifest_request(format: ExportFormat) -> ExportRequest {
        ExportRequest {
            format,
            mode: ExportMode::ReinstallationManifest,
        }
    }

    #[test]
    fn csv_safe_prefixes_formula_leading_characters() {
        for dangerous in ["=SUM(A1)", "+1", "-1", "@cmd", "\ttab", "\rcr"] {
            let escaped = csv_safe(dangerous);
            assert!(
                escaped.starts_with('\''),
                "expected {dangerous:?} to be escaped"
            );
            assert_eq!(&escaped[1..], dangerous);
        }
    }

    #[test]
    fn csv_safe_leaves_ordinary_values_untouched() {
        for ordinary in ["firefox", "1.2.3", "", "a=b", "GNU/Linux"] {
            assert_eq!(csv_safe(ordinary), ordinary);
        }
    }

    #[tokio::test]
    async fn json_export_round_trips_item_data() {
        let state = state_with_one_item().await;

        let response = export_inventory_impl(&state, full_request(ExportFormat::Json))
            .await
            .expect("export");

        assert_eq!(response.schema_version, EXPORT_SCHEMA_VERSION);
        assert!(response.content.contains("\"id\": \"apt:git\""));
    }

    #[tokio::test]
    async fn yaml_export_contains_the_item() {
        let state = state_with_one_item().await;

        let response = export_inventory_impl(&state, full_request(ExportFormat::Yaml))
            .await
            .expect("export");

        assert!(response.content.contains("apt:git"));
    }

    #[tokio::test]
    async fn csv_export_has_a_header_and_one_data_row() {
        let state = state_with_one_item().await;

        let response = export_inventory_impl(&state, full_request(ExportFormat::Csv))
            .await
            .expect("export");

        let lines: Vec<&str> = response.content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("id,packageName"));
        assert!(lines[1].starts_with("apt:git,git,Git"));
    }

    #[tokio::test]
    async fn export_with_no_scanned_items_still_produces_valid_output() {
        let state = test_state(vec![]);

        let response = export_inventory_impl(&state, full_request(ExportFormat::Json))
            .await
            .expect("export");

        assert!(response.content.contains("\"itemCount\": 0"));
    }

    fn manifest_fixture_items() -> Vec<SoftwareItem> {
        let mut manual_apt =
            SoftwareItem::new("apt:ripgrep", "ripgrep", "ripgrep", PackageManager::Apt);
        manual_apt.installation_reason = InstallationReason::Manual;
        manual_apt.version = Some("14.1.0".to_string());

        let mut auto_apt = SoftwareItem::new("apt:libc6", "libc6", "libc6", PackageManager::Apt);
        auto_apt.installation_reason = InstallationReason::Automatic;

        let mut manual_local = SoftwareItem::new(
            "manual:/usr/local/bin/mytool",
            "mytool",
            "mytool",
            PackageManager::Manual,
        );
        manual_local.install_paths = vec!["/home/alice/.local/bin/mytool".to_string()];

        let mut appimage = SoftwareItem::new(
            "appimage:/opt/App.AppImage",
            "App",
            "App",
            PackageManager::AppImage,
        );
        appimage.install_paths = vec!["/opt/App.AppImage".to_string()];

        vec![manual_apt, auto_apt, manual_local, appimage]
    }

    #[tokio::test]
    async fn manifest_json_separates_reproducible_from_manual_review_and_drops_automatic_deps() {
        let state = state_with_items(manifest_fixture_items()).await;

        let response = export_inventory_impl(&state, manifest_request(ExportFormat::Json))
            .await
            .expect("export");

        assert!(response.content.contains("ripgrep"));
        assert!(response.content.contains("sudo apt install"));
        assert!(response.content.contains("mytool"));
        assert!(response.content.contains("/home/alice/.local/bin/mytool"));
        assert!(response.content.contains("App.AppImage") || response.content.contains("\"App\""));
        // libc6 was installed automatically as a dependency -- it must not
        // appear anywhere in the manifest.
        assert!(!response.content.contains("libc6"));
    }

    #[tokio::test]
    async fn manifest_yaml_contains_both_sections() {
        let state = state_with_items(manifest_fixture_items()).await;

        let response = export_inventory_impl(&state, manifest_request(ExportFormat::Yaml))
            .await
            .expect("export");

        assert!(response.content.contains("reproducible:"));
        assert!(response.content.contains("manualReview:"));
    }

    #[tokio::test]
    async fn manifest_csv_marks_each_row_reproducible_or_not() {
        let state = state_with_items(manifest_fixture_items()).await;

        let response = export_inventory_impl(&state, manifest_request(ExportFormat::Csv))
            .await
            .expect("export");

        let lines: Vec<&str> = response.content.lines().collect();
        // header + 1 reproducible (ripgrep) + 2 manual-review (mytool, App) -- libc6 excluded.
        assert_eq!(lines.len(), 4);
        assert!(lines[0].starts_with("reproducible,packageManager"));
        assert!(lines.iter().any(|line| line.starts_with("yes,Apt,ripgrep")));
        assert!(lines.iter().any(|line| line.starts_with("no,Manual,")));
        assert!(!response.content.contains("libc6"));
    }

    #[tokio::test]
    async fn full_csv_export_neutralizes_formula_prefixes_in_scanned_fields() {
        let mut item = SoftwareItem::new(
            "apt:evil-pkg",
            "=SUM(A1:A9)",
            "@evil()",
            PackageManager::Apt,
        );
        item.version = Some("+1.0".to_string());
        let state = state_with_items(vec![item]).await;

        let response = export_inventory_impl(&state, full_request(ExportFormat::Csv))
            .await
            .expect("export");

        let data_row = response.content.lines().nth(1).expect("data row");
        assert!(data_row.contains("'=SUM(A1:A9)"));
        assert!(data_row.contains("'@evil()"));
        assert!(data_row.contains("'+1.0"));
    }

    #[tokio::test]
    async fn manifest_csv_neutralizes_formula_prefixes_in_scanned_fields() {
        let mut manual_apt = SoftwareItem::new("apt:x", "=cmd()", "-2+3", PackageManager::Apt);
        manual_apt.installation_reason = InstallationReason::Manual;

        let mut manual_local =
            SoftwareItem::new("manual:y", "y", "@display", PackageManager::Manual);
        manual_local.install_paths = vec!["=HYPERLINK(\"http://evil\")".to_string()];

        let state = state_with_items(vec![manual_apt, manual_local]).await;

        let response = export_inventory_impl(&state, manifest_request(ExportFormat::Csv))
            .await
            .expect("export");

        assert!(response.content.contains("'=cmd()"));
        assert!(response.content.contains("'-2+3"));
        assert!(response.content.contains("'@display"));
        assert!(response.content.contains("'=HYPERLINK"));
    }

    #[tokio::test]
    async fn manifest_with_no_items_still_produces_valid_output() {
        let state = test_state(vec![]);

        let response = export_inventory_impl(&state, manifest_request(ExportFormat::Json))
            .await
            .expect("export");

        assert!(response.content.contains("\"reproducible\": []"));
        assert!(response.content.contains("\"manualReview\": []"));
    }
}
