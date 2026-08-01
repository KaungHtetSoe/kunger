//! Merges [`SoftwareItem`]s sharing the same `id` across providers into
//! one record.
//!
//! Some providers deliberately emit items with the *same* id another
//! provider uses for the underlying package (e.g. the desktop and font
//! providers reuse `apt:{package}` when they resolve dpkg ownership —
//! ADR-0012) so that ownership-known enrichment data converges onto the
//! authoritative record instead of appearing as a fabricated duplicate.
//! This module is where that convergence actually happens.

use std::collections::HashMap;

use crate::domain::{InstallationReason, SoftwareItem};

/// Merges same-id items, preserving the order the first occurrence of
/// each id appeared in `items`.
///
/// Merge is order-sensitive: for each id, the *first* item encountered is
/// the base record, and every subsequent item with the same id enriches
/// it (fills empty fields, unions list fields, upgrades classification
/// only on higher confidence). Callers should register/list providers so
/// the most authoritative source for a given id namespace (e.g. the APT
/// provider for `apt:*` ids) is processed first — see
/// `docs/ARCHITECTURE.md` §7 and `docs/DECISIONS.md` ADR-0012.
pub fn merge_by_id(items: Vec<SoftwareItem>) -> Vec<SoftwareItem> {
    let mut grouped: HashMap<String, Vec<SoftwareItem>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for item in items {
        if !grouped.contains_key(&item.id) {
            order.push(item.id.clone());
        }
        grouped.entry(item.id.clone()).or_default().push(item);
    }

    order
        .into_iter()
        .filter_map(|id| grouped.remove(&id).and_then(merge_group))
        .collect()
}

fn merge_group(mut group: Vec<SoftwareItem>) -> Option<SoftwareItem> {
    if group.is_empty() {
        return None;
    }

    let mut iter = group.drain(..);
    let mut base = iter.next()?;
    for other in iter {
        merge_into(&mut base, &other);
    }
    Some(base)
}

fn merge_into(base: &mut SoftwareItem, other: &SoftwareItem) {
    fill(&mut base.description, &other.description);
    fill(&mut base.version, &other.version);
    fill(&mut base.architecture, &other.architecture);
    fill(&mut base.package_source, &other.package_source);
    fill(&mut base.icon_path, &other.icon_path);
    fill(&mut base.package_section, &other.package_section);
    fill(&mut base.installed_size_bytes, &other.installed_size_bytes);
    fill(&mut base.installed_at, &other.installed_at);
    fill(&mut base.available_version, &other.available_version);
    fill(&mut base.repository, &other.repository);
    fill(&mut base.homepage, &other.homepage);
    fill(&mut base.license, &other.license);

    // A display name that's just the package name repeated verbatim (as
    // providers that don't have a nicer name default it, e.g. AptProvider)
    // isn't preferable to a real human-readable one another provider found
    // (e.g. a .desktop file's Name= key) -- exact comparison, not
    // case-insensitive, since "Firefox" vs. "firefox" is itself a real
    // improvement worth keeping.
    let base_is_generic =
        base.display_name.trim().is_empty() || base.display_name == base.package_name;
    let other_is_specific =
        !other.display_name.trim().is_empty() && other.display_name != other.package_name;
    if base_is_generic && other_is_specific {
        base.display_name = other.display_name.clone();
    }

    union_into(&mut base.install_paths, &other.install_paths);
    union_into(&mut base.executable_paths, &other.executable_paths);
    union_into(&mut base.desktop_file_paths, &other.desktop_file_paths);
    union_into(&mut base.dependencies, &other.dependencies);
    union_into(&mut base.reverse_dependencies, &other.reverse_dependencies);
    union_into(&mut base.warnings, &other.warnings);

    if other.classification_confidence > base.classification_confidence {
        base.category = other.category;
        base.classification_confidence = other.classification_confidence;
        base.classification_reasons = other.classification_reasons.clone();
    } else if other.classification_confidence == base.classification_confidence
        && other.category == base.category
    {
        union_into(
            &mut base.classification_reasons,
            &other.classification_reasons,
        );
    }

    union_into(&mut base.secondary_categories, &other.secondary_categories);
    let primary = base.category;
    base.secondary_categories
        .retain(|category| *category != primary);

    for (key, value) in &other.metadata {
        base.metadata
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }

    base.update_available = base.update_available || other.update_available;

    if base.installation_reason == InstallationReason::Unknown {
        base.installation_reason = other.installation_reason;
    }
}

fn fill<T: Clone>(base: &mut Option<T>, other: &Option<T>) {
    if base.is_none() {
        *base = other.clone();
    }
}

fn union_into<T: Clone + PartialEq>(base: &mut Vec<T>, other: &[T]) {
    for item in other {
        if !base.contains(item) {
            base.push(item.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ClassificationConfidence, PackageManager, SoftwareCategory};

    fn apt_base() -> SoftwareItem {
        let mut item = SoftwareItem::new("apt:firefox", "firefox", "firefox", PackageManager::Apt);
        item.version = Some("128.0".to_string());
        item.package_section = Some("web".to_string());
        item.category = SoftwareCategory::Application;
        item.classification_confidence = ClassificationConfidence::High;
        item.classification_reasons = vec!["package provides a desktop launcher".to_string()];
        item
    }

    fn desktop_enrichment() -> SoftwareItem {
        let mut item = SoftwareItem::new("apt:firefox", "firefox", "Firefox", PackageManager::Apt);
        item.desktop_file_paths = vec!["/usr/share/applications/firefox.desktop".to_string()];
        item.icon_path = Some("firefox".to_string());
        item
    }

    #[test]
    fn items_with_different_ids_are_not_merged() {
        let a = SoftwareItem::new("apt:git", "git", "git", PackageManager::Apt);
        let b = SoftwareItem::new("apt:curl", "curl", "curl", PackageManager::Apt);

        let merged = merge_by_id(vec![a, b]);

        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn merge_preserves_order_of_first_occurrence() {
        let a = SoftwareItem::new("apt:aaa", "aaa", "aaa", PackageManager::Apt);
        let b = SoftwareItem::new("apt:bbb", "bbb", "bbb", PackageManager::Apt);

        let merged = merge_by_id(vec![b.clone(), a.clone()]);

        assert_eq!(merged[0].id, "apt:bbb");
        assert_eq!(merged[1].id, "apt:aaa");
    }

    #[test]
    fn enrichment_fills_empty_fields_on_the_base_item() {
        let merged = merge_by_id(vec![apt_base(), desktop_enrichment()]);

        assert_eq!(merged.len(), 1);
        let item = &merged[0];
        assert_eq!(
            item.desktop_file_paths,
            vec!["/usr/share/applications/firefox.desktop".to_string()]
        );
        assert_eq!(item.icon_path.as_deref(), Some("firefox"));
        // Fields the base already had are untouched.
        assert_eq!(item.version.as_deref(), Some("128.0"));
    }

    #[test]
    fn a_more_specific_display_name_overrides_a_generic_one() {
        let merged = merge_by_id(vec![apt_base(), desktop_enrichment()]);

        assert_eq!(merged[0].display_name, "Firefox");
    }

    #[test]
    fn does_not_overwrite_an_already_specific_display_name() {
        let mut base = apt_base();
        base.display_name = "Mozilla Firefox Web Browser".to_string();

        let merged = merge_by_id(vec![base, desktop_enrichment()]);

        assert_eq!(merged[0].display_name, "Mozilla Firefox Web Browser");
    }

    #[test]
    fn higher_confidence_classification_wins() {
        let mut low_confidence_first =
            SoftwareItem::new("apt:libssl3", "libssl3", "libssl3", PackageManager::Apt);
        low_confidence_first.category = SoftwareCategory::Unclassified;
        low_confidence_first.classification_confidence = ClassificationConfidence::Unknown;

        let mut high_confidence_second =
            SoftwareItem::new("apt:libssl3", "libssl3", "libssl3", PackageManager::Apt);
        high_confidence_second.category = SoftwareCategory::Library;
        high_confidence_second.classification_confidence = ClassificationConfidence::High;
        high_confidence_second.classification_reasons =
            vec!["Debian section is \"libs\"".to_string()];

        let merged = merge_by_id(vec![low_confidence_first, high_confidence_second]);

        assert_eq!(merged[0].category, SoftwareCategory::Library);
        assert_eq!(
            merged[0].classification_confidence,
            ClassificationConfidence::High
        );
    }

    #[test]
    fn equal_confidence_and_category_unions_reasons_instead_of_discarding() {
        let mut first = apt_base();
        first.classification_reasons = vec!["reason A".to_string()];

        let mut second = apt_base();
        second.classification_reasons = vec!["reason B".to_string()];

        let merged = merge_by_id(vec![first, second]);

        assert_eq!(
            merged[0].classification_reasons,
            vec!["reason A".to_string(), "reason B".to_string()]
        );
    }

    #[test]
    fn secondary_categories_never_duplicate_the_final_primary_category() {
        let mut first = apt_base();
        first.category = SoftwareCategory::Library;
        first.classification_confidence = ClassificationConfidence::High;

        let mut second = apt_base();
        second.category = SoftwareCategory::Library;
        second.classification_confidence = ClassificationConfidence::High;
        second.secondary_categories = vec![
            SoftwareCategory::Library,
            SoftwareCategory::DevelopmentPackage,
        ];

        let merged = merge_by_id(vec![first, second]);

        assert!(!merged[0]
            .secondary_categories
            .contains(&SoftwareCategory::Library));
        assert!(merged[0]
            .secondary_categories
            .contains(&SoftwareCategory::DevelopmentPackage));
    }

    #[test]
    fn metadata_maps_merge_without_overwriting_existing_keys() {
        let mut first = apt_base();
        first
            .metadata
            .insert("maintainer".to_string(), "Real Maintainer".to_string());

        let mut second = apt_base();
        second
            .metadata
            .insert("maintainer".to_string(), "Should Not Win".to_string());
        second
            .metadata
            .insert("owning_package".to_string(), "firefox".to_string());

        let merged = merge_by_id(vec![first, second]);

        assert_eq!(
            merged[0].metadata.get("maintainer").map(String::as_str),
            Some("Real Maintainer")
        );
        assert_eq!(
            merged[0].metadata.get("owning_package").map(String::as_str),
            Some("firefox")
        );
    }

    #[test]
    fn update_available_is_true_if_any_source_says_so() {
        let mut first = apt_base();
        first.update_available = false;

        let mut second = apt_base();
        second.update_available = true;
        second.available_version = Some("129.0".to_string());

        let merged = merge_by_id(vec![first, second]);

        assert!(merged[0].update_available);
        assert_eq!(merged[0].available_version.as_deref(), Some("129.0"));
    }

    #[test]
    fn installation_reason_unknown_is_filled_from_a_later_source() {
        let mut first = apt_base();
        first.installation_reason = InstallationReason::Unknown;

        let mut second = apt_base();
        second.installation_reason = InstallationReason::Manual;

        let merged = merge_by_id(vec![first, second]);

        assert_eq!(merged[0].installation_reason, InstallationReason::Manual);
    }

    #[test]
    fn three_way_merge_combines_paths_from_every_source() {
        let base = apt_base();
        let mut desktop_item = desktop_enrichment();
        desktop_item.desktop_file_paths =
            vec!["/usr/share/applications/firefox.desktop".to_string()];

        let mut font_like_third =
            SoftwareItem::new("apt:firefox", "firefox", "firefox", PackageManager::Apt);
        font_like_third.install_paths = vec!["/usr/lib/firefox/firefox".to_string()];

        let merged = merge_by_id(vec![base, desktop_item, font_like_third]);

        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].install_paths,
            vec!["/usr/lib/firefox/firefox".to_string()]
        );
        assert_eq!(
            merged[0].desktop_file_paths,
            vec!["/usr/share/applications/firefox.desktop".to_string()]
        );
    }
}
