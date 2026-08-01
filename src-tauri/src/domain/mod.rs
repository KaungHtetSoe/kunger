//! Domain layer: pure data types with no dependency on process execution,
//! SQLite, or Tauri. See `docs/ARCHITECTURE.md` §2.1.

mod duplicate_group;
mod enums;
mod provider_result;
mod software_item;
mod summary;

pub use duplicate_group::DuplicateGroup;
pub use enums::{
    ClassificationConfidence, InstallationReason, InstallationScope, InventoryStatus,
    PackageManager, ProviderStatus, SoftwareCategory, SoftwareRiskLevel,
};
pub use provider_result::{ProviderError, ProviderInventory, ProviderWarning};
pub use software_item::{SoftwareItem, ValidationError};
pub use summary::InventorySummary;
