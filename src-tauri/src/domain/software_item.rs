use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::enums::{
    ClassificationConfidence, InstallationReason, InstallationScope, PackageManager,
    SoftwareCategory, SoftwareRiskLevel,
};

/// A single piece of software discovered on the system, normalized across
/// whichever provider(s) found it.
///
/// Field selection follows the questions Kunger must be able to answer for
/// every item (see `docs/PRODUCT_SPEC.md` §2). This type has no dependency
/// on process execution, SQLite, or Tauri — see `docs/ARCHITECTURE.md` §2.1.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareItem {
    /// Stable identifier, unique within a single scan. Format is
    /// provider-defined (e.g. `apt:firefox`, `flatpak:org.mozilla.firefox`).
    pub id: String,
    pub package_name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub architecture: Option<String>,

    pub category: SoftwareCategory,
    pub secondary_categories: Vec<SoftwareCategory>,

    pub package_manager: PackageManager,
    /// Human-readable origin within the package manager (e.g. an APT
    /// repository name or a Flatpak remote), when known.
    pub package_source: Option<String>,
    pub scope: InstallationScope,

    pub install_paths: Vec<String>,
    pub executable_paths: Vec<String>,
    pub desktop_file_paths: Vec<String>,
    pub icon_path: Option<String>,

    /// Debian package section (e.g. `libs`, `fonts`), when applicable.
    pub package_section: Option<String>,
    pub installed_size_bytes: Option<u64>,
    pub installed_at: Option<DateTime<Utc>>,

    pub installation_reason: InstallationReason,

    pub dependencies: Vec<String>,
    pub reverse_dependencies: Vec<String>,

    pub update_available: bool,
    pub available_version: Option<String>,

    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,

    pub classification_confidence: ClassificationConfidence,
    pub classification_reasons: Vec<String>,
    pub risk_level: SoftwareRiskLevel,

    /// Provider-specific extra fields that don't yet warrant a first-class
    /// field. Prefer adding a typed field over growing this map once a
    /// piece of data is used by classification, duplicate detection, or the
    /// UI directly — see `docs/ARCHITECTURE.md` §3.
    pub metadata: HashMap<String, String>,

    /// Item-level warnings (e.g. "desktop file had a malformed Exec field")
    /// that should not fail the whole item.
    pub warnings: Vec<String>,
}

/// Domain-level validation failure for a [`SoftwareItem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("field `{0}` must not be empty")]
    EmptyField(&'static str),
}

impl SoftwareItem {
    /// Construct a minimal, valid item with the required identifying
    /// fields set and everything else defaulted.
    pub fn new(
        id: impl Into<String>,
        package_name: impl Into<String>,
        display_name: impl Into<String>,
        package_manager: PackageManager,
    ) -> Self {
        Self {
            id: id.into(),
            package_name: package_name.into(),
            display_name: display_name.into(),
            package_manager,
            ..Default::default()
        }
    }

    /// Checks the minimal invariants every item must satisfy regardless of
    /// which provider produced it. This is not a substitute for
    /// provider-specific validation, just a shared floor.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.id.trim().is_empty() {
            return Err(ValidationError::EmptyField("id"));
        }
        if self.package_name.trim().is_empty() {
            return Err(ValidationError::EmptyField("package_name"));
        }
        if self.display_name.trim().is_empty() {
            return Err(ValidationError::EmptyField("display_name"));
        }
        Ok(())
    }

    /// True when at least one secondary category is present in addition to
    /// the primary `category`.
    pub fn has_secondary_categories(&self) -> bool {
        !self.secondary_categories.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item() -> SoftwareItem {
        let mut item = SoftwareItem::new("apt:firefox", "firefox", "Firefox", PackageManager::Apt);
        item.version = Some("128.0".to_string());
        item.category = SoftwareCategory::Application;
        item.classification_confidence = ClassificationConfidence::High;
        item.classification_reasons = vec!["package owns a desktop launcher".to_string()];
        item
    }

    #[test]
    fn new_sets_required_fields_and_defaults_the_rest() {
        let item = SoftwareItem::new("apt:git", "git", "Git", PackageManager::Apt);

        assert_eq!(item.id, "apt:git");
        assert_eq!(item.package_name, "git");
        assert_eq!(item.display_name, "Git");
        assert_eq!(item.package_manager, PackageManager::Apt);
        assert_eq!(item.category, SoftwareCategory::Unclassified);
        assert_eq!(item.installation_reason, InstallationReason::Unknown);
        assert!(item.dependencies.is_empty());
        assert!(item.metadata.is_empty());
    }

    #[test]
    fn validate_rejects_empty_required_fields() {
        let mut item = sample_item();
        item.id = "  ".to_string();

        assert_eq!(item.validate(), Err(ValidationError::EmptyField("id")));
    }

    #[test]
    fn validate_accepts_a_well_formed_item() {
        assert_eq!(sample_item().validate(), Ok(()));
    }

    #[test]
    fn serializes_to_camel_case_json() {
        let item = sample_item();
        let json = serde_json::to_value(&item).expect("serialization should succeed");

        assert_eq!(json["packageName"], "firefox");
        assert_eq!(json["displayName"], "Firefox");
        assert_eq!(json["classificationConfidence"], "high");
    }

    #[test]
    fn round_trips_through_json() {
        let item = sample_item();
        let json = serde_json::to_string(&item).expect("serialization should succeed");
        let restored: SoftwareItem =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(item, restored);
    }

    #[test]
    fn has_secondary_categories_reflects_the_vec() {
        let mut item = sample_item();
        assert!(!item.has_secondary_categories());

        item.secondary_categories
            .push(SoftwareCategory::CommandLineTool);
        assert!(item.has_secondary_categories());
    }
}
