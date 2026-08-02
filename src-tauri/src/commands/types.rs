//! Request/response DTOs for the Tauri command layer. Kept separate from
//! the domain types (even though several are thin wrappers) so the IPC
//! contract can evolve independently of internal domain modeling.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    ClassificationConfidence, InstallationReason, InstallationScope, InventorySummary,
    PackageManager, ProviderError, ProviderWarning, SoftwareCategory, SoftwareItem,
};

pub const MAX_PAGE_SIZE: u32 = 500;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatusResponse {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub available: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StartScanRequest {
    /// Per-provider timeout in milliseconds. Defaults to 30s if omitted.
    pub per_provider_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum ScanStatusResponse {
    Idle {
        last_summary: Option<InventorySummary>,
    },
    Running {
        started_at: DateTime<Utc>,
        elapsed_ms: i64,
    },
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListSoftwareItemsRequest {
    /// 1-based page number.
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: Option<u32>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub categories: Option<Vec<SoftwareCategory>>,
    #[serde(default)]
    pub package_managers: Option<Vec<PackageManager>>,
    #[serde(default)]
    pub scopes: Option<Vec<InstallationScope>>,
    #[serde(default)]
    pub installation_reasons: Option<Vec<InstallationReason>>,
    #[serde(default)]
    pub update_available_only: Option<bool>,
    #[serde(default)]
    pub min_confidence: Option<ClassificationConfidence>,
    #[serde(default)]
    pub sort_by: Option<SortField>,
    #[serde(default)]
    pub sort_direction: Option<SortDirection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortField {
    DisplayName,
    Category,
    PackageManager,
    Version,
    InstalledSize,
    Confidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSoftwareItemsResponse {
    pub items: Vec<SoftwareItem>,
    pub total_count: usize,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderWarningsResponse {
    pub provider_id: String,
    pub warnings: Vec<ProviderWarning>,
    pub error: Option<ProviderError>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub format: ExportFormat,
    #[serde(default)]
    pub mode: ExportMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    Json,
    Yaml,
    Csv,
}

/// `full` dumps every scanned field for every item, verbatim. `reinstallationManifest`
/// instead answers "what do I run to get this back" -- it groups items whose package
/// manager supports non-interactive reinstall by name (apt/flatpak/snap/pip/pipx/npm/
/// cargo) separately from items Kunger cannot automatically reproduce (manual/AppImage/
/// unknown-manager finds), per product spec FR-11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ExportMode {
    #[default]
    Full,
    ReinstallationManifest,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResponse {
    pub schema_version: u32,
    pub format: ExportFormat,
    pub content: String,
}

// `DuplicateGroup` itself (imported above) is used directly as
// `list_duplicate_groups`'s return type — no separate response wrapper
// needed.
