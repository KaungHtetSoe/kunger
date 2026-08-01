use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::enums::{InventoryStatus, PackageManager, SoftwareCategory};

/// Aggregate view over a completed (or partially completed) inventory scan,
/// used to drive the dashboard and to detect what changed between scans.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventorySummary {
    pub status: InventoryStatus,
    pub total_items: usize,
    pub items_by_category: HashMap<SoftwareCategory, usize>,
    pub items_by_package_manager: HashMap<PackageManager, usize>,
    pub providers_with_warnings: Vec<String>,
    pub providers_with_errors: Vec<String>,
    pub duplicate_group_count: usize,
    pub last_scan_started_at: Option<DateTime<Utc>>,
    pub last_scan_completed_at: Option<DateTime<Utc>>,
    pub scan_duration_ms: Option<u64>,
}

impl InventorySummary {
    pub fn is_partial(&self) -> bool {
        matches!(
            self.status,
            InventoryStatus::CompletedWithWarnings | InventoryStatus::PartiallyFailed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_summary_has_not_started_status_and_zero_counts() {
        let summary = InventorySummary::default();

        assert_eq!(summary.status, InventoryStatus::NotStarted);
        assert_eq!(summary.total_items, 0);
        assert!(summary.items_by_category.is_empty());
        assert!(!summary.is_partial());
    }

    #[test]
    fn is_partial_reflects_completed_with_warnings_and_partially_failed() {
        let mut summary = InventorySummary {
            status: InventoryStatus::CompletedWithWarnings,
            ..Default::default()
        };
        assert!(summary.is_partial());

        summary.status = InventoryStatus::PartiallyFailed;
        assert!(summary.is_partial());

        summary.status = InventoryStatus::Completed;
        assert!(!summary.is_partial());
    }

    #[test]
    fn serializes_category_counts_with_camel_case_enum_keys() {
        let mut summary = InventorySummary::default();
        summary
            .items_by_category
            .insert(SoftwareCategory::CommandLineTool, 42);

        let json = serde_json::to_value(&summary).expect("serialization should succeed");
        assert_eq!(json["itemsByCategory"]["commandLineTool"], 42);
    }
}
