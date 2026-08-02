//! `export_inventory`.
//!
//! Implements the full technical inventory export (JSON/YAML/CSV) for the
//! latest scan. The reinstallation-manifest export mode is a separate,
//! larger feature (Prompt 09F / M4.6) with its own frontend workflow and
//! is deliberately not implemented here — this command's job is just to
//! give the IPC surface a real, working export capability now.

use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;

use crate::domain::SoftwareItem;

use super::{run_blocking, AppState, CommandError, ExportFormat, ExportRequest, ExportResponse};

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

    let content = match request.format {
        ExportFormat::Json => export_json(&items)?,
        ExportFormat::Yaml => export_yaml(&items)?,
        ExportFormat::Csv => export_csv(&items)?,
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
                item.id.as_str(),
                item.package_name.as_str(),
                item.display_name.as_str(),
                &format!("{:?}", item.category),
                &format!("{:?}", item.package_manager),
                &format!("{:?}", item.scope),
                item.version.as_deref().unwrap_or(""),
                &item
                    .installed_size_bytes
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                &item.update_available.to_string(),
                &format!("{:?}", item.classification_confidence),
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
    use crate::commands::events::NoopScanEventEmitter;
    use crate::commands::scan::start_inventory_scan_impl;
    use crate::commands::test_support::test_state;
    use crate::commands::StartScanRequest;
    use crate::domain::PackageManager;
    use crate::providers::mock::MockInventoryProvider;
    use std::time::Duration;

    async fn state_with_one_item() -> AppState {
        let item = SoftwareItem::new("apt:git", "git", "Git", PackageManager::Apt);
        let state = Arc::new(test_state(vec![Box::new(
            MockInventoryProvider::new("apt").with_items(vec![item]),
        )]));
        start_inventory_scan_impl(
            Arc::clone(&state),
            Arc::new(NoopScanEventEmitter),
            StartScanRequest::default(),
        )
        .await
        .expect("scan starts");
        tokio::time::sleep(Duration::from_millis(100)).await;
        Arc::try_unwrap(state)
            .unwrap_or_else(|arc| panic!("state still has {} refs", Arc::strong_count(&arc)))
    }

    #[tokio::test]
    async fn json_export_round_trips_item_data() {
        let state = state_with_one_item().await;

        let response = export_inventory_impl(
            &state,
            ExportRequest {
                format: ExportFormat::Json,
            },
        )
        .await
        .expect("export");

        assert_eq!(response.schema_version, EXPORT_SCHEMA_VERSION);
        assert!(response.content.contains("\"id\": \"apt:git\""));
    }

    #[tokio::test]
    async fn yaml_export_contains_the_item() {
        let state = state_with_one_item().await;

        let response = export_inventory_impl(
            &state,
            ExportRequest {
                format: ExportFormat::Yaml,
            },
        )
        .await
        .expect("export");

        assert!(response.content.contains("apt:git"));
    }

    #[tokio::test]
    async fn csv_export_has_a_header_and_one_data_row() {
        let state = state_with_one_item().await;

        let response = export_inventory_impl(
            &state,
            ExportRequest {
                format: ExportFormat::Csv,
            },
        )
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

        let response = export_inventory_impl(
            &state,
            ExportRequest {
                format: ExportFormat::Json,
            },
        )
        .await
        .expect("export");

        assert!(response.content.contains("\"itemCount\": 0"));
    }
}
