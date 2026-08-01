use serde::{Deserialize, Serialize};

use super::enums::ClassificationConfidence;

/// A set of [`super::SoftwareItem`]s the inventory service believes
/// represent the same underlying software installed through more than one
/// mechanism (e.g. Firefox via APT and Flatpak). Never acted on
/// automatically — see `docs/DECISIONS.md` ADR-0005: Kunger only ever
/// surfaces duplicate groups for the user to review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub id: String,
    /// The [`super::SoftwareItem::id`]s belonging to this group, sorted for
    /// deterministic output.
    pub item_ids: Vec<String>,
    pub reason: String,
    pub confidence: ClassificationConfidence,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_to_camel_case_json() {
        let group = DuplicateGroup {
            id: "dup:firefox".to_string(),
            item_ids: vec![
                "apt:firefox".to_string(),
                "flatpak:user:org.mozilla.firefox".to_string(),
            ],
            reason: "shared normalized name across package managers".to_string(),
            confidence: ClassificationConfidence::Medium,
        };

        let json = serde_json::to_value(&group).expect("serialization should succeed");
        assert_eq!(json["itemIds"][0], "apt:firefox");
        assert_eq!(json["confidence"], "medium");
    }
}
