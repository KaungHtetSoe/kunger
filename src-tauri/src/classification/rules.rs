//! The classification rule table: priority-ordered, strongest/most direct
//! evidence first. See `docs/CLASSIFICATION.md` for the human-readable
//! version of this table and the rationale behind the ordering.

use super::Evidence;
use crate::domain::{ClassificationConfidence, SoftwareCategory};

pub(super) struct Rule {
    pub(super) category: SoftwareCategory,
    pub(super) confidence: ClassificationConfidence,
    pub(super) reason: &'static str,
    pub(super) matches: fn(&Evidence) -> bool,
}

fn section_eq(evidence: &Evidence, expected: &str) -> bool {
    evidence
        .debian_section
        .as_deref()
        .is_some_and(|s| s.eq_ignore_ascii_case(expected))
}

fn section_in(evidence: &Evidence, options: &[&str]) -> bool {
    evidence
        .debian_section
        .as_deref()
        .is_some_and(|s| options.iter().any(|o| s.eq_ignore_ascii_case(o)))
}

fn name_ends_with(evidence: &Evidence, suffix: &str) -> bool {
    evidence.package_name.to_ascii_lowercase().ends_with(suffix)
}

fn name_contains(evidence: &Evidence, needle: &str) -> bool {
    evidence.package_name.to_ascii_lowercase().contains(needle)
}

fn desktop_categories_contains_any(evidence: &Evidence, options: &[&str]) -> bool {
    evidence
        .desktop_categories
        .iter()
        .any(|c| options.iter().any(|o| c.eq_ignore_ascii_case(o)))
}

/// Priority-ordered classification rules. Evaluated top to bottom by
/// [`super::classify`] — the first matching rule sets the primary category;
/// later rules matching the *same* category corroborate it (raising
/// confidence); later rules matching a *different* category become
/// secondary categories. See `docs/CLASSIFICATION.md` §"Rule priority".
pub(super) static RULES: &[Rule] = &[
    Rule {
        category: SoftwareCategory::Font,
        confidence: ClassificationConfidence::High,
        reason: "Debian section is \"fonts\"",
        matches: |e| section_eq(e, "fonts"),
    },
    Rule {
        category: SoftwareCategory::Library,
        confidence: ClassificationConfidence::High,
        reason: "Debian section is \"libs\"",
        matches: |e| section_eq(e, "libs"),
    },
    Rule {
        category: SoftwareCategory::DevelopmentPackage,
        confidence: ClassificationConfidence::High,
        reason: "Debian section is \"libdevel\" (development files for a library)",
        matches: |e| section_eq(e, "libdevel"),
    },
    Rule {
        category: SoftwareCategory::DevelopmentPackage,
        confidence: ClassificationConfidence::High,
        reason: "Debian section is \"devel\"",
        matches: |e| section_eq(e, "devel"),
    },
    Rule {
        category: SoftwareCategory::Documentation,
        confidence: ClassificationConfidence::High,
        reason: "Debian section is \"doc\"",
        matches: |e| section_eq(e, "doc"),
    },
    Rule {
        category: SoftwareCategory::KernelComponent,
        confidence: ClassificationConfidence::High,
        reason: "Debian section is \"kernel\"",
        matches: |e| section_eq(e, "kernel"),
    },
    Rule {
        category: SoftwareCategory::LanguagePack,
        confidence: ClassificationConfidence::Medium,
        reason: "Debian section is \"localization\"",
        matches: |e| section_eq(e, "localization"),
    },
    Rule {
        category: SoftwareCategory::Runtime,
        confidence: ClassificationConfidence::Medium,
        reason: "Debian section is \"interpreters\"",
        matches: |e| section_eq(e, "interpreters"),
    },
    Rule {
        category: SoftwareCategory::Miscellaneous,
        confidence: ClassificationConfidence::Low,
        reason: "Debian section is \"misc\" or \"metapackages\"",
        matches: |e| section_in(e, &["misc", "metapackages"]),
    },
    Rule {
        category: SoftwareCategory::Firmware,
        confidence: ClassificationConfidence::High,
        reason: "package owns files under /lib/firmware",
        matches: |e| e.owns_firmware_files,
    },
    Rule {
        category: SoftwareCategory::KernelComponent,
        confidence: ClassificationConfidence::High,
        reason: "package owns files under /lib/modules",
        matches: |e| e.owns_kernel_module_files,
    },
    Rule {
        category: SoftwareCategory::SystemService,
        confidence: ClassificationConfidence::High,
        reason: "package installs a systemd unit file",
        matches: |e| e.owns_systemd_unit_files,
    },
    Rule {
        category: SoftwareCategory::Theme,
        confidence: ClassificationConfidence::High,
        reason: "package owns files under /usr/share/themes",
        matches: |e| e.owns_gtk_theme_files,
    },
    Rule {
        category: SoftwareCategory::IconPack,
        confidence: ClassificationConfidence::Medium,
        reason: "package owns an icon theme under /usr/share/icons",
        matches: |e| e.owns_icon_theme_files,
    },
    Rule {
        category: SoftwareCategory::DesktopComponent,
        confidence: ClassificationConfidence::Medium,
        reason: "package provides a desktop launcher categorized as a system/settings component",
        matches: |e| {
            e.has_desktop_launcher && desktop_categories_contains_any(e, &["settings", "system"])
        },
    },
    Rule {
        category: SoftwareCategory::Application,
        confidence: ClassificationConfidence::High,
        reason: "package provides a desktop launcher",
        matches: |e| e.has_desktop_launcher,
    },
    Rule {
        category: SoftwareCategory::DevelopmentPackage,
        confidence: ClassificationConfidence::High,
        reason: "package owns header files under /usr/include",
        matches: |e| e.owns_header_files,
    },
    Rule {
        category: SoftwareCategory::DevelopmentPackage,
        confidence: ClassificationConfidence::Medium,
        reason: "package owns pkg-config (.pc) files",
        matches: |e| e.owns_pkgconfig_files,
    },
    Rule {
        category: SoftwareCategory::Library,
        confidence: ClassificationConfidence::High,
        reason: "package owns shared library (.so) files",
        matches: |e| e.owns_shared_libraries,
    },
    Rule {
        category: SoftwareCategory::DevelopmentPackage,
        confidence: ClassificationConfidence::Medium,
        reason: "package name ends with \"-dev\"",
        matches: |e| name_ends_with(e, "-dev"),
    },
    Rule {
        category: SoftwareCategory::Documentation,
        confidence: ClassificationConfidence::Medium,
        reason: "package name ends with \"-doc\"",
        matches: |e| name_ends_with(e, "-doc"),
    },
    Rule {
        category: SoftwareCategory::LanguagePack,
        confidence: ClassificationConfidence::Medium,
        reason: "package name contains \"language-pack\"",
        matches: |e| name_contains(e, "language-pack"),
    },
    Rule {
        category: SoftwareCategory::Documentation,
        confidence: ClassificationConfidence::Medium,
        reason: "package appears to contain only documentation files",
        matches: |e| e.documentation_only,
    },
    Rule {
        category: SoftwareCategory::CommandLineTool,
        confidence: ClassificationConfidence::Medium,
        reason: "package installs executables without a desktop launcher",
        matches: |e| e.has_executables && !e.has_desktop_launcher,
    },
    Rule {
        category: SoftwareCategory::Driver,
        confidence: ClassificationConfidence::Low,
        reason: "package name suggests a hardware driver",
        matches: |e| name_ends_with(e, "-dkms") || name_contains(e, "driver"),
    },
];
