use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::enums::ProviderStatus;
use super::software_item::SoftwareItem;

/// A non-fatal issue encountered while scanning (e.g. one malformed
/// `.desktop` file among thousands). Warnings never cause a provider's scan
/// to fail outright — see `docs/ARCHITECTURE.md` §4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderWarning {
    pub message: String,
    /// The `SoftwareItem::id` this warning relates to, if any.
    pub item_id: Option<String>,
    /// Free-form extra detail (e.g. the offending file path).
    pub context: Option<String>,
}

impl ProviderWarning {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            item_id: None,
            context: None,
        }
    }

    pub fn for_item(mut self, item_id: impl Into<String>) -> Self {
        self.item_id = Some(item_id.into());
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

/// A fatal error that stopped a provider from completing its scan. The
/// provider may still have partial `items`/`warnings` alongside this in its
/// [`ProviderInventory`] — a fatal error does not discard prior progress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "message")]
pub enum ProviderError {
    CommandNotFound(String),
    Timeout(String),
    PermissionDenied(String),
    MalformedOutput(String),
    Cancelled(String),
    Other(String),
}

impl ProviderError {
    pub fn message(&self) -> &str {
        match self {
            ProviderError::CommandNotFound(m)
            | ProviderError::Timeout(m)
            | ProviderError::PermissionDenied(m)
            | ProviderError::MalformedOutput(m)
            | ProviderError::Cancelled(m)
            | ProviderError::Other(m) => m,
        }
    }
}

/// The full result of a single provider's inventory scan: whatever items it
/// found, plus warnings, an optional fatal error, and timing information.
///
/// A provider that fails part-way through still returns a `ProviderInventory`
/// carrying its partial `items`/`warnings` rather than propagating a bare
/// error — see `docs/ARCHITECTURE.md` §4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInventory {
    pub provider_id: String,
    pub status: ProviderStatus,
    pub items: Vec<SoftwareItem>,
    pub warnings: Vec<ProviderWarning>,
    pub error: Option<ProviderError>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    /// Version of the underlying tool (e.g. `dpkg --version`), when cheaply
    /// available.
    pub provider_version: Option<String>,
}

impl ProviderInventory {
    /// Begin recording a scan for `provider_id`, started at `started_at`.
    pub fn started(provider_id: impl Into<String>, started_at: DateTime<Utc>) -> Self {
        Self {
            provider_id: provider_id.into(),
            status: ProviderStatus::NotRun,
            items: Vec::new(),
            warnings: Vec::new(),
            error: None,
            started_at,
            completed_at: None,
            duration_ms: None,
            provider_version: None,
        }
    }

    /// Marks the scan complete, computing `duration_ms` from `started_at`.
    pub fn finish(mut self, completed_at: DateTime<Utc>, status: ProviderStatus) -> Self {
        let duration = completed_at - self.started_at;
        self.duration_ms = Some(duration.num_milliseconds().max(0) as u64);
        self.completed_at = Some(completed_at);
        self.status = status;
        self
    }

    pub fn is_success(&self) -> bool {
        matches!(
            self.status,
            ProviderStatus::Success | ProviderStatus::PartialSuccess
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn provider_warning_builder_sets_optional_fields() {
        let warning = ProviderWarning::new("bad file")
            .for_item("apt:weird-pkg")
            .with_context("/usr/share/applications/weird.desktop");

        assert_eq!(warning.message, "bad file");
        assert_eq!(warning.item_id.as_deref(), Some("apt:weird-pkg"));
        assert_eq!(
            warning.context.as_deref(),
            Some("/usr/share/applications/weird.desktop")
        );
    }

    #[test]
    fn provider_error_message_extracts_inner_string_for_every_variant() {
        let errors = vec![
            ProviderError::CommandNotFound("flatpak".into()),
            ProviderError::Timeout("dpkg-query timed out".into()),
            ProviderError::PermissionDenied("denied".into()),
            ProviderError::MalformedOutput("bad json".into()),
            ProviderError::Cancelled("cancelled".into()),
            ProviderError::Other("other".into()),
        ];

        for error in errors {
            assert!(!error.message().is_empty());
        }
    }

    #[test]
    fn finish_computes_duration_and_sets_status() {
        let started_at = Utc::now();
        let inventory = ProviderInventory::started("apt", started_at);
        let completed_at = started_at + Duration::milliseconds(250);

        let finished = inventory.finish(completed_at, ProviderStatus::Success);

        assert_eq!(finished.duration_ms, Some(250));
        assert_eq!(finished.completed_at, Some(completed_at));
        assert!(finished.is_success());
    }

    #[test]
    fn partial_success_counts_as_success_for_is_success() {
        let started_at = Utc::now();
        let inventory = ProviderInventory::started("flatpak", started_at)
            .finish(started_at, ProviderStatus::PartialSuccess);

        assert!(inventory.is_success());
    }

    #[test]
    fn failed_status_is_not_success() {
        let started_at = Utc::now();
        let inventory = ProviderInventory::started("flatpak", started_at)
            .finish(started_at, ProviderStatus::Failed);

        assert!(!inventory.is_success());
    }

    #[test]
    fn provider_inventory_serializes_with_tagged_error() {
        let started_at = Utc::now();
        let mut inventory = ProviderInventory::started("flatpak", started_at);
        inventory.error = Some(ProviderError::CommandNotFound("flatpak".into()));

        let json = serde_json::to_value(&inventory).expect("serialization should succeed");
        assert_eq!(json["error"]["kind"], "commandNotFound");
        assert_eq!(json["error"]["message"], "flatpak");
    }
}
