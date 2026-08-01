//! Classification layer: turns raw, provider-observed evidence about a
//! software item into a primary category, optional secondary categories, a
//! confidence level, and human-readable reasons. Pure functions, no I/O —
//! see `docs/ARCHITECTURE.md` §2.3 and `docs/CLASSIFICATION.md`.

mod rules;

use crate::domain::{ClassificationConfidence, SoftwareCategory};

/// Raw, structured signals a provider observed about a package, independent
/// of any category decision. Providers populate this from whatever they
/// can cheaply determine (declared metadata, owned file paths, desktop
/// entry contents); the classification engine never inspects file contents
/// or runs commands itself.
#[derive(Debug, Clone, Default)]
pub struct Evidence {
    /// Lowercase-insensitive; used only for the weakest, name-based rules.
    pub package_name: String,
    /// Debian package section (e.g. `"libs"`, `"fonts"`), if declared.
    pub debian_section: Option<String>,
    pub has_desktop_launcher: bool,
    /// The `Categories=` field from a `.desktop` file, if any.
    pub desktop_categories: Vec<String>,
    pub has_executables: bool,
    pub owns_shared_libraries: bool,
    pub owns_header_files: bool,
    pub owns_pkgconfig_files: bool,
    pub owns_font_files: bool,
    pub owns_firmware_files: bool,
    pub owns_kernel_module_files: bool,
    pub owns_systemd_unit_files: bool,
    pub owns_gtk_theme_files: bool,
    pub owns_icon_theme_files: bool,
    /// True when the package appears to own only documentation/man-page
    /// files and nothing executable, headers, or libraries.
    pub documentation_only: bool,
}

/// The outcome of classifying one item's [`Evidence`]. Defaults to
/// `Unclassified` / `Unknown` confidence with no reasons — the same result
/// produced when no rule matches.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassificationResult {
    pub category: SoftwareCategory,
    pub secondary_categories: Vec<SoftwareCategory>,
    pub confidence: ClassificationConfidence,
    pub reasons: Vec<String>,
}

/// Classifies a single item's evidence. Never fails and never panics on any
/// input — worst case, evidence with no matching rule yields
/// `SoftwareCategory::Unclassified` with `ClassificationConfidence::Unknown`
/// and no reasons, rather than a forced guess. See
/// `docs/CLASSIFICATION.md` §"Ambiguity handling".
pub fn classify(evidence: &Evidence) -> ClassificationResult {
    let mut primary: Option<(SoftwareCategory, ClassificationConfidence)> = None;
    let mut secondary_categories: Vec<SoftwareCategory> = Vec::new();
    let mut reasons: Vec<String> = Vec::new();

    for rule in rules::RULES {
        if !(rule.matches)(evidence) {
            continue;
        }

        match primary {
            None => {
                primary = Some((rule.category, rule.confidence));
                reasons.push(rule.reason.to_string());
            }
            Some((category, confidence)) if category == rule.category => {
                reasons.push(rule.reason.to_string());
                primary = Some((category, bump_confidence(confidence)));
            }
            Some(_) => {
                if !secondary_categories.contains(&rule.category) {
                    secondary_categories.push(rule.category);
                }
            }
        }
    }

    match primary {
        Some((category, confidence)) => ClassificationResult {
            category,
            secondary_categories,
            confidence,
            reasons,
        },
        None => ClassificationResult::default(),
    }
}

/// Corroborating evidence for the same category raises confidence one
/// level, capped at `Certain`. See `docs/CLASSIFICATION.md` §"Confidence
/// scoring".
fn bump_confidence(confidence: ClassificationConfidence) -> ClassificationConfidence {
    match confidence {
        ClassificationConfidence::Unknown => ClassificationConfidence::Low,
        ClassificationConfidence::Low => ClassificationConfidence::Medium,
        ClassificationConfidence::Medium => ClassificationConfidence::High,
        ClassificationConfidence::High | ClassificationConfidence::Certain => {
            ClassificationConfidence::Certain
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> Evidence {
        Evidence {
            package_name: "example".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn no_evidence_is_unclassified_with_unknown_confidence_and_no_reasons() {
        let result = classify(&evidence());

        assert_eq!(result.category, SoftwareCategory::Unclassified);
        assert_eq!(result.confidence, ClassificationConfidence::Unknown);
        assert!(result.reasons.is_empty());
        assert!(result.secondary_categories.is_empty());
    }

    #[test]
    fn debian_section_fonts_classifies_as_font() {
        let mut e = evidence();
        e.debian_section = Some("fonts".to_string());

        let result = classify(&e);

        assert_eq!(result.category, SoftwareCategory::Font);
        assert_eq!(result.confidence, ClassificationConfidence::High);
        assert_eq!(result.reasons, vec!["Debian section is \"fonts\""]);
    }

    #[test]
    fn debian_section_comparison_is_case_insensitive() {
        let mut e = evidence();
        e.debian_section = Some("FONTS".to_string());

        assert_eq!(classify(&e).category, SoftwareCategory::Font);
    }

    #[test]
    fn debian_section_libs_classifies_as_library() {
        let mut e = evidence();
        e.debian_section = Some("libs".to_string());

        assert_eq!(classify(&e).category, SoftwareCategory::Library);
    }

    #[test]
    fn debian_section_libdevel_classifies_as_development_package() {
        let mut e = evidence();
        e.debian_section = Some("libdevel".to_string());

        assert_eq!(classify(&e).category, SoftwareCategory::DevelopmentPackage);
    }

    #[test]
    fn debian_section_devel_classifies_as_development_package() {
        let mut e = evidence();
        e.debian_section = Some("devel".to_string());

        assert_eq!(classify(&e).category, SoftwareCategory::DevelopmentPackage);
    }

    #[test]
    fn debian_section_doc_classifies_as_documentation() {
        let mut e = evidence();
        e.debian_section = Some("doc".to_string());

        assert_eq!(classify(&e).category, SoftwareCategory::Documentation);
    }

    #[test]
    fn debian_section_kernel_classifies_as_kernel_component() {
        let mut e = evidence();
        e.debian_section = Some("kernel".to_string());

        assert_eq!(classify(&e).category, SoftwareCategory::KernelComponent);
    }

    #[test]
    fn debian_section_localization_classifies_as_language_pack() {
        let mut e = evidence();
        e.debian_section = Some("localization".to_string());

        let result = classify(&e);
        assert_eq!(result.category, SoftwareCategory::LanguagePack);
        assert_eq!(result.confidence, ClassificationConfidence::Medium);
    }

    #[test]
    fn debian_section_interpreters_classifies_as_runtime() {
        let mut e = evidence();
        e.debian_section = Some("interpreters".to_string());

        assert_eq!(classify(&e).category, SoftwareCategory::Runtime);
    }

    #[test]
    fn debian_section_misc_classifies_as_miscellaneous_with_low_confidence() {
        let mut e = evidence();
        e.debian_section = Some("misc".to_string());

        let result = classify(&e);
        assert_eq!(result.category, SoftwareCategory::Miscellaneous);
        assert_eq!(result.confidence, ClassificationConfidence::Low);
    }

    #[test]
    fn debian_section_metapackages_classifies_as_miscellaneous() {
        let mut e = evidence();
        e.debian_section = Some("metapackages".to_string());

        assert_eq!(classify(&e).category, SoftwareCategory::Miscellaneous);
    }

    #[test]
    fn firmware_files_classify_as_firmware() {
        let mut e = evidence();
        e.owns_firmware_files = true;

        assert_eq!(classify(&e).category, SoftwareCategory::Firmware);
    }

    #[test]
    fn kernel_module_files_classify_as_kernel_component() {
        let mut e = evidence();
        e.owns_kernel_module_files = true;

        assert_eq!(classify(&e).category, SoftwareCategory::KernelComponent);
    }

    #[test]
    fn systemd_unit_files_classify_as_system_service() {
        let mut e = evidence();
        e.owns_systemd_unit_files = true;

        assert_eq!(classify(&e).category, SoftwareCategory::SystemService);
    }

    #[test]
    fn gtk_theme_files_classify_as_theme() {
        let mut e = evidence();
        e.owns_gtk_theme_files = true;

        assert_eq!(classify(&e).category, SoftwareCategory::Theme);
    }

    #[test]
    fn icon_theme_files_classify_as_icon_pack() {
        let mut e = evidence();
        e.owns_icon_theme_files = true;

        assert_eq!(classify(&e).category, SoftwareCategory::IconPack);
    }

    #[test]
    fn desktop_launcher_with_settings_category_classifies_as_desktop_component() {
        let mut e = evidence();
        e.has_desktop_launcher = true;
        e.desktop_categories = vec!["Settings".to_string()];

        assert_eq!(classify(&e).category, SoftwareCategory::DesktopComponent);
    }

    #[test]
    fn desktop_launcher_without_system_categories_classifies_as_application() {
        let mut e = evidence();
        e.has_desktop_launcher = true;
        e.desktop_categories = vec!["Utility".to_string()];

        let result = classify(&e);
        assert_eq!(result.category, SoftwareCategory::Application);
        assert_eq!(result.confidence, ClassificationConfidence::High);
    }

    #[test]
    fn header_files_classify_as_development_package() {
        let mut e = evidence();
        e.owns_header_files = true;

        assert_eq!(classify(&e).category, SoftwareCategory::DevelopmentPackage);
    }

    #[test]
    fn pkgconfig_files_classify_as_development_package_with_medium_confidence() {
        let mut e = evidence();
        e.owns_pkgconfig_files = true;

        let result = classify(&e);
        assert_eq!(result.category, SoftwareCategory::DevelopmentPackage);
        assert_eq!(result.confidence, ClassificationConfidence::Medium);
    }

    #[test]
    fn shared_library_files_classify_as_library() {
        let mut e = evidence();
        e.owns_shared_libraries = true;

        assert_eq!(classify(&e).category, SoftwareCategory::Library);
    }

    #[test]
    fn name_suffix_dev_classifies_as_development_package() {
        let mut e = evidence();
        e.package_name = "libfoo-dev".to_string();

        let result = classify(&e);
        assert_eq!(result.category, SoftwareCategory::DevelopmentPackage);
        assert_eq!(result.confidence, ClassificationConfidence::Medium);
    }

    #[test]
    fn name_suffix_doc_classifies_as_documentation() {
        let mut e = evidence();
        e.package_name = "libfoo-doc".to_string();

        assert_eq!(classify(&e).category, SoftwareCategory::Documentation);
    }

    #[test]
    fn name_containing_language_pack_classifies_as_language_pack() {
        let mut e = evidence();
        e.package_name = "language-pack-en".to_string();

        assert_eq!(classify(&e).category, SoftwareCategory::LanguagePack);
    }

    #[test]
    fn documentation_only_flag_classifies_as_documentation() {
        let mut e = evidence();
        e.documentation_only = true;

        assert_eq!(classify(&e).category, SoftwareCategory::Documentation);
    }

    #[test]
    fn executables_without_desktop_launcher_classify_as_command_line_tool() {
        let mut e = evidence();
        e.has_executables = true;

        let result = classify(&e);
        assert_eq!(result.category, SoftwareCategory::CommandLineTool);
        assert_eq!(result.confidence, ClassificationConfidence::Medium);
    }

    #[test]
    fn executables_with_desktop_launcher_prefer_application_over_command_line_tool() {
        let mut e = evidence();
        e.has_executables = true;
        e.has_desktop_launcher = true;

        assert_eq!(classify(&e).category, SoftwareCategory::Application);
    }

    #[test]
    fn dkms_suffix_classifies_as_driver_with_low_confidence() {
        let mut e = evidence();
        e.package_name = "nvidia-dkms".to_string();

        let result = classify(&e);
        assert_eq!(result.category, SoftwareCategory::Driver);
        assert_eq!(result.confidence, ClassificationConfidence::Low);
    }

    #[test]
    fn name_containing_driver_classifies_as_driver() {
        let mut e = evidence();
        e.package_name = "example-graphics-driver".to_string();

        assert_eq!(classify(&e).category, SoftwareCategory::Driver);
    }

    #[test]
    fn section_takes_priority_over_desktop_launcher() {
        // A package with an authoritative "fonts" section but that also
        // happens to ship a desktop launcher (e.g. a font manager UI
        // bundled with the font itself) should still classify as Font: the
        // section is a stronger signal than a desktop launcher.
        let mut e = evidence();
        e.debian_section = Some("fonts".to_string());
        e.has_desktop_launcher = true;

        let result = classify(&e);
        assert_eq!(result.category, SoftwareCategory::Font);
        assert!(result
            .secondary_categories
            .contains(&SoftwareCategory::Application));
    }

    #[test]
    fn corroborating_signals_for_the_same_category_raise_confidence() {
        // Both the section and an explicit header-file signal point at
        // DevelopmentPackage; confidence should end up higher than either
        // rule alone would produce.
        let mut e = evidence();
        e.debian_section = Some("devel".to_string());
        e.owns_header_files = true;

        let result = classify(&e);

        assert_eq!(result.category, SoftwareCategory::DevelopmentPackage);
        assert_eq!(result.confidence, ClassificationConfidence::Certain);
        assert_eq!(result.reasons.len(), 2);
    }

    #[test]
    fn conflicting_weaker_signals_become_secondary_categories_not_overrides() {
        let mut e = evidence();
        e.debian_section = Some("libs".to_string());
        e.package_name = "libfoo-driver-shim".to_string();

        let result = classify(&e);

        assert_eq!(result.category, SoftwareCategory::Library);
        assert_eq!(result.secondary_categories, vec![SoftwareCategory::Driver]);
    }

    #[test]
    fn secondary_categories_do_not_contain_duplicates() {
        let mut e = evidence();
        e.debian_section = Some("libs".to_string());
        e.has_desktop_launcher = true;
        e.owns_header_files = true;

        let result = classify(&e);

        // Application (from desktop launcher) and DevelopmentPackage (from
        // header files) are both distinct secondary categories, each once.
        assert_eq!(result.category, SoftwareCategory::Library);
        assert_eq!(result.secondary_categories.len(), 2);
        assert!(result
            .secondary_categories
            .contains(&SoftwareCategory::Application));
        assert!(result
            .secondary_categories
            .contains(&SoftwareCategory::DevelopmentPackage));
    }
}
