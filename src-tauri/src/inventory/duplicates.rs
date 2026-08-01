//! Cross-manager duplicate detection: the same underlying software
//! installed through more than one package manager (e.g. Firefox via APT
//! and Flatpak).
//!
//! This runs *after* [`super::merge::merge_by_id`] — same-id records are
//! already combined by then, so everything reaching this module has a
//! genuinely distinct id and, if flagged, represents a real duplicate
//! *installation*, not just a duplicate *record*. Duplicate desktop
//! entries and manually-detected-but-dpkg-owned binaries are already
//! resolved upstream (by the desktop and manual providers respectively)
//! and never reach this stage at all.

use std::collections::{HashMap, HashSet};

use crate::domain::{ClassificationConfidence, DuplicateGroup, PackageManager, SoftwareItem};

/// Groups items by normalized display name; any group spanning more than
/// one [`PackageManager`] is reported as a likely duplicate installation.
/// Name matching is deliberately conservative (exact match after
/// lowercasing and stripping non-alphanumeric characters, not fuzzy
/// matching) — see `docs/ARCHITECTURE.md` §7 for why Kunger never
/// auto-resolves these (ADR-0005).
pub fn detect_duplicates(items: &[SoftwareItem]) -> Vec<DuplicateGroup> {
    let mut by_name: HashMap<String, Vec<&SoftwareItem>> = HashMap::new();

    for item in items {
        let key = normalize_name(&item.display_name);
        if key.is_empty() {
            continue;
        }
        by_name.entry(key).or_default().push(item);
    }

    let mut groups: Vec<DuplicateGroup> = by_name
        .into_iter()
        .filter_map(|(name, group_items)| {
            let distinct_managers: HashSet<PackageManager> =
                group_items.iter().map(|item| item.package_manager).collect();

            if group_items.len() < 2 || distinct_managers.len() < 2 {
                return None;
            }

            let mut item_ids: Vec<String> = group_items.iter().map(|item| item.id.clone()).collect();
            item_ids.sort();

            Some(DuplicateGroup {
                id: format!("dup:{name}"),
                item_ids,
                reason: format!(
                    "{} items share the normalized display name \"{name}\" across {} different package managers",
                    group_items.len(),
                    distinct_managers.len()
                ),
                confidence: ClassificationConfidence::Medium,
            })
        })
        .collect();

    groups.sort_by(|a, b| a.id.cmp(&b.id));
    groups
}

fn normalize_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, display_name: &str, manager: PackageManager) -> SoftwareItem {
        SoftwareItem::new(id, id, display_name, manager)
    }

    #[test]
    fn same_name_different_managers_is_flagged() {
        let items = vec![
            item("apt:firefox", "Firefox", PackageManager::Apt),
            item(
                "flatpak:user:org.mozilla.firefox",
                "Firefox",
                PackageManager::Flatpak,
            ),
        ];

        let groups = detect_duplicates(&items);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].item_ids.len(), 2);
        assert_eq!(groups[0].confidence, ClassificationConfidence::Medium);
    }

    #[test]
    fn name_match_is_case_and_punctuation_insensitive() {
        let items = vec![
            item("apt:firefox", "Fire-Fox", PackageManager::Apt),
            item(
                "flatpak:user:org.mozilla.firefox",
                "FIREFOX",
                PackageManager::Flatpak,
            ),
        ];

        let groups = detect_duplicates(&items);

        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn same_name_same_manager_is_not_flagged() {
        let items = vec![
            item("apt:firefox", "Firefox", PackageManager::Apt),
            item("apt:firefox-esr", "Firefox", PackageManager::Apt),
        ];

        let groups = detect_duplicates(&items);

        assert!(groups.is_empty());
    }

    #[test]
    fn unique_names_are_not_flagged() {
        let items = vec![
            item("apt:git", "Git", PackageManager::Apt),
            item(
                "flatpak:user:org.mozilla.firefox",
                "Firefox",
                PackageManager::Flatpak,
            ),
        ];

        let groups = detect_duplicates(&items);

        assert!(groups.is_empty());
    }

    #[test]
    fn manual_binary_already_excluded_upstream_never_reaches_this_stage() {
        // ManualSoftwareProvider excludes dpkg-owned paths entirely before
        // they're ever returned (see providers/manual), so there is no
        // "manual + apt duplicate" scenario for this function to catch --
        // this test documents that expectation rather than exercising new
        // behavior in this module.
        let items = vec![item("apt:git", "Git", PackageManager::Apt)];

        assert!(detect_duplicates(&items).is_empty());
    }

    #[test]
    fn three_way_duplicate_across_apt_flatpak_and_appimage() {
        let items = vec![
            item("apt:obsidian", "Obsidian", PackageManager::Apt),
            item(
                "flatpak:user:md.obsidian.Obsidian",
                "Obsidian",
                PackageManager::Flatpak,
            ),
            item(
                "appimage:/home/user/Applications/Obsidian.AppImage",
                "Obsidian",
                PackageManager::AppImage,
            ),
        ];

        let groups = detect_duplicates(&items);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].item_ids.len(), 3);
    }

    #[test]
    fn empty_display_name_is_never_grouped() {
        let items = vec![
            item("apt:a", "", PackageManager::Apt),
            item("flatpak:user:b", "", PackageManager::Flatpak),
        ];

        assert!(detect_duplicates(&items).is_empty());
    }

    #[test]
    fn group_ids_and_item_ids_are_sorted_for_deterministic_output() {
        let items = vec![
            item(
                "flatpak:user:org.mozilla.firefox",
                "Firefox",
                PackageManager::Flatpak,
            ),
            item("apt:firefox", "Firefox", PackageManager::Apt),
        ];

        let groups = detect_duplicates(&items);

        assert_eq!(
            groups[0].item_ids,
            vec![
                "apt:firefox".to_string(),
                "flatpak:user:org.mozilla.firefox".to_string()
            ]
        );
    }
}
