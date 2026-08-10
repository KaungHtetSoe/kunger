use serde::{Deserialize, Serialize};

/// Primary classification bucket for a discovered piece of software.
///
/// `Unclassified` is the default and is deliberately never treated as an
/// error state: an item with no matching classification rule stays
/// `Unclassified` rather than being force-fit into a guess. See
/// `docs/CLASSIFICATION.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SoftwareCategory {
    Application,
    CommandLineTool,
    Library,
    Font,
    Runtime,
    DevelopmentPackage,
    Theme,
    IconPack,
    Firmware,
    Driver,
    KernelComponent,
    SystemService,
    DesktopComponent,
    Documentation,
    LanguagePack,
    Miscellaneous,
    #[default]
    Unclassified,
}

impl SoftwareCategory {
    pub const ALL: &'static [Self] = &[
        Self::Application,
        Self::CommandLineTool,
        Self::Library,
        Self::Font,
        Self::Runtime,
        Self::DevelopmentPackage,
        Self::Theme,
        Self::IconPack,
        Self::Firmware,
        Self::Driver,
        Self::KernelComponent,
        Self::SystemService,
        Self::DesktopComponent,
        Self::Documentation,
        Self::LanguagePack,
        Self::Miscellaneous,
        Self::Unclassified,
    ];
}

/// The installation source / ownership mechanism for a software item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PackageManager {
    Apt,
    Flatpak,
    Snap,
    AppImage,
    Pip,
    Pipx,
    Npm,
    Cargo,
    /// Detected in a known location but not owned by any recognized package manager.
    Manual,
    #[default]
    Unknown,
}

impl PackageManager {
    pub const ALL: &'static [Self] = &[
        Self::Apt,
        Self::Flatpak,
        Self::Snap,
        Self::AppImage,
        Self::Pip,
        Self::Pipx,
        Self::Npm,
        Self::Cargo,
        Self::Manual,
        Self::Unknown,
    ];
}

/// Whether a software item is installed for the whole system or a single user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum InstallationScope {
    System,
    User,
    #[default]
    Unknown,
}

impl InstallationScope {
    pub const ALL: &'static [Self] = &[Self::System, Self::User, Self::Unknown];
}

/// Why a package ended up installed: explicitly requested by the user, or
/// pulled in automatically as a dependency of something else.
///
/// Modeled as a single enum rather than two independent booleans
/// (`manually_installed` / `automatically_installed`) so that
/// "both" and "neither" cannot be represented — see `docs/DECISIONS.md`
/// ADR-0003.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum InstallationReason {
    Manual,
    Automatic,
    #[default]
    Unknown,
}

impl InstallationReason {
    pub const ALL: &'static [Self] = &[Self::Manual, Self::Automatic, Self::Unknown];
}

/// How confident the classification engine is in an item's assigned category.
///
/// Declaration order is significant: variants are ordered from least to
/// most confident so `ClassificationConfidence` values can be compared
/// directly (`derive(PartialOrd, Ord)`). See `docs/DECISIONS.md` ADR-0004.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "camelCase")]
pub enum ClassificationConfidence {
    #[default]
    Unknown,
    Low,
    Medium,
    High,
    Certain,
}

impl ClassificationConfidence {
    pub const ALL: &'static [Self] = &[
        Self::Unknown,
        Self::Low,
        Self::Medium,
        Self::High,
        Self::Certain,
    ];
}

/// Overall status of a full inventory scan (across all providers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum InventoryStatus {
    #[default]
    NotStarted,
    Scanning,
    Completed,
    CompletedWithWarnings,
    PartiallyFailed,
    Failed,
    Cancelled,
}

/// Outcome of a single provider's scan attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ProviderStatus {
    #[default]
    NotRun,
    Success,
    PartialSuccess,
    Unavailable,
    Failed,
    TimedOut,
    Cancelled,
}

/// Reserved general-purpose trust/risk signal for a software item (e.g. an
/// unmanaged manual install or an unverifiable AppImage may carry a higher
/// risk level than a package installed and signed through APT). Not yet
/// populated by any classification rule as of M1.3 — the enum exists so the
/// domain model has a stable place for this signal once duplicate detection
/// and classification (M1.5, M4.1) start computing it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "camelCase")]
pub enum SoftwareRiskLevel {
    #[default]
    Unknown,
    Low,
    Medium,
    High,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_ordering_is_increasing() {
        assert!(ClassificationConfidence::Unknown < ClassificationConfidence::Low);
        assert!(ClassificationConfidence::Low < ClassificationConfidence::Medium);
        assert!(ClassificationConfidence::Medium < ClassificationConfidence::High);
        assert!(ClassificationConfidence::High < ClassificationConfidence::Certain);
    }

    #[test]
    fn risk_level_ordering_is_increasing() {
        assert!(SoftwareRiskLevel::Unknown < SoftwareRiskLevel::Low);
        assert!(SoftwareRiskLevel::Low < SoftwareRiskLevel::Medium);
        assert!(SoftwareRiskLevel::Medium < SoftwareRiskLevel::High);
    }

    #[test]
    fn defaults_are_the_unknown_variants() {
        assert_eq!(SoftwareCategory::default(), SoftwareCategory::Unclassified);
        assert_eq!(PackageManager::default(), PackageManager::Unknown);
        assert_eq!(InstallationScope::default(), InstallationScope::Unknown);
        assert_eq!(InstallationReason::default(), InstallationReason::Unknown);
        assert_eq!(
            ClassificationConfidence::default(),
            ClassificationConfidence::Unknown
        );
        assert_eq!(InventoryStatus::default(), InventoryStatus::NotStarted);
        assert_eq!(ProviderStatus::default(), ProviderStatus::NotRun);
        assert_eq!(SoftwareRiskLevel::default(), SoftwareRiskLevel::Unknown);
    }
}
