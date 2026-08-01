//! Pure `.desktop` file parsing. Independent of filesystem access so it's
//! directly unit-testable against fixtures — see
//! `src-tauri/tests/fixtures/desktop/`.
//!
//! `Exec` is stored verbatim, never unescaped, tokenized, or executed:
//! Kunger only ever displays it as metadata. See `docs/SECURITY.md` — "must
//! not trust Exec fields."

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DesktopEntryRecord {
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub no_display: bool,
    pub hidden: bool,
    pub terminal: bool,
    pub startup_wm_class: Option<String>,
}

/// Parses the `[Desktop Entry]` section of a `.desktop` file's contents.
/// `source` is used only to make warning messages actionable (typically the
/// file path). Returns `None` for files with no `[Desktop Entry]` section,
/// no (or an empty) `Name`, or a `Type` other than `Application` (e.g.
/// `Link`/`Directory` entries, which aren't software launchers) — every
/// such case is recorded as a warning except the `Type` mismatch, which is
/// an expected, valid condition, not malformed input.
pub fn parse_desktop_entry(
    content: &str,
    source: &str,
    warnings: &mut Vec<String>,
) -> Option<DesktopEntryRecord> {
    let mut in_target_section = false;
    let mut found_section = false;
    let mut fields: HashMap<&str, String> = HashMap::new();
    let mut categories: Vec<String> = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_target_section = line == "[Desktop Entry]";
            if in_target_section {
                found_section = true;
            }
            continue;
        }

        if !in_target_section {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            warnings.push(format!(
                "{source}: ignored malformed line (no \"=\"): \"{line}\""
            ));
            continue;
        };

        let key = key.trim();
        let value = value.trim();

        match key {
            "Name" | "GenericName" | "Comment" | "Exec" | "Icon" | "StartupWMClass" | "Type"
            | "NoDisplay" | "Hidden" | "Terminal" => {
                fields.insert(key, value.to_string());
            }
            "Categories" => {
                categories = value
                    .split(';')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            _ => {
                // Includes locale-suffixed keys like `Name[fr]` and any
                // other key Kunger doesn't use -- valid per the Desktop
                // Entry spec, silently ignored rather than warned about.
            }
        }
    }

    if !found_section {
        warnings.push(format!(
            "{source}: no [Desktop Entry] section found; skipped"
        ));
        return None;
    }

    if let Some(type_value) = fields.get("Type") {
        if type_value != "Application" {
            return None;
        }
    }

    let name = match fields.get("Name") {
        Some(name) if !name.is_empty() => name.clone(),
        _ => {
            warnings.push(format!("{source}: missing or empty Name key; skipped"));
            return None;
        }
    };

    Some(DesktopEntryRecord {
        name,
        generic_name: fields.get("GenericName").cloned(),
        comment: fields.get("Comment").cloned(),
        exec: fields.get("Exec").cloned(),
        icon: fields.get("Icon").cloned(),
        categories,
        no_display: is_true(fields.get("NoDisplay")),
        hidden: is_true(fields.get("Hidden")),
        terminal: is_true(fields.get("Terminal")),
        startup_wm_class: fields.get("StartupWMClass").cloned(),
    })
}

fn is_true(value: Option<&String>) -> bool {
    value.map(|v| v == "true").unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIREFOX: &str = include_str!("../../../tests/fixtures/desktop/firefox.desktop");
    const SETTINGS: &str = include_str!("../../../tests/fixtures/desktop/settings.desktop");
    const HIDDEN_APP: &str = include_str!("../../../tests/fixtures/desktop/hidden-app.desktop");
    const LINK_ENTRY: &str = include_str!("../../../tests/fixtures/desktop/link-entry.desktop");
    const MALFORMED: &str = include_str!("../../../tests/fixtures/desktop/malformed.desktop");
    const NO_NAME: &str = include_str!("../../../tests/fixtures/desktop/no-name.desktop");
    const ESCAPED_EXEC: &str = include_str!("../../../tests/fixtures/desktop/escaped-exec.desktop");
    const LOCALIZED: &str = include_str!("../../../tests/fixtures/desktop/localized.desktop");

    #[test]
    fn parses_a_well_formed_application_entry() {
        let mut warnings = Vec::new();
        let entry =
            parse_desktop_entry(FIREFOX, "firefox.desktop", &mut warnings).expect("should parse");

        assert!(warnings.is_empty());
        assert_eq!(entry.name, "Firefox");
        assert_eq!(entry.generic_name.as_deref(), Some("Web Browser"));
        assert_eq!(entry.comment.as_deref(), Some("Browse the World Wide Web"));
        assert_eq!(entry.exec.as_deref(), Some("/usr/bin/firefox %u"));
        assert_eq!(entry.icon.as_deref(), Some("firefox"));
        assert_eq!(entry.categories, vec!["Network", "WebBrowser"]);
        assert_eq!(entry.startup_wm_class.as_deref(), Some("firefox"));
        assert!(!entry.terminal);
        assert!(!entry.hidden);
        assert!(!entry.no_display);
    }

    #[test]
    fn parses_settings_categories() {
        let mut warnings = Vec::new();
        let entry =
            parse_desktop_entry(SETTINGS, "settings.desktop", &mut warnings).expect("should parse");

        assert_eq!(
            entry.categories,
            vec!["Settings", "GNOME", "X-GNOME-Settings-Panel"]
        );
    }

    #[test]
    fn hidden_and_no_display_flags_are_captured_not_used_to_drop_the_entry() {
        let mut warnings = Vec::new();
        let entry = parse_desktop_entry(HIDDEN_APP, "hidden-app.desktop", &mut warnings)
            .expect("should still parse");

        assert!(entry.hidden);
        assert!(entry.no_display);
    }

    #[test]
    fn link_type_entries_are_skipped_without_a_warning() {
        let mut warnings = Vec::new();
        let entry = parse_desktop_entry(LINK_ENTRY, "link-entry.desktop", &mut warnings);

        assert_eq!(entry, None);
        assert!(
            warnings.is_empty(),
            "Type=Link is valid, not malformed -- no warning expected"
        );
    }

    #[test]
    fn file_with_no_desktop_entry_section_is_skipped_with_a_warning() {
        let mut warnings = Vec::new();
        let entry = parse_desktop_entry(MALFORMED, "malformed.desktop", &mut warnings);

        assert_eq!(entry, None);
        assert!(warnings
            .iter()
            .any(|w| w.contains("no [Desktop Entry] section")));
    }

    #[test]
    fn missing_name_is_skipped_with_a_warning() {
        let mut warnings = Vec::new();
        let entry = parse_desktop_entry(NO_NAME, "no-name.desktop", &mut warnings);

        assert_eq!(entry, None);
        assert!(warnings.iter().any(|w| w.contains("missing or empty Name")));
    }

    #[test]
    fn exec_field_is_stored_verbatim_including_quotes_and_backslash_escapes() {
        let mut warnings = Vec::new();
        let entry = parse_desktop_entry(ESCAPED_EXEC, "escaped-exec.desktop", &mut warnings)
            .expect("should parse");

        assert_eq!(
            entry.exec.as_deref(),
            Some(r#"/usr/bin/app --flag="value with spaces" --path=/tmp/some\ dir %f"#)
        );
    }

    #[test]
    fn localized_name_keys_are_ignored_in_favor_of_the_base_name() {
        let mut warnings = Vec::new();
        let entry = parse_desktop_entry(LOCALIZED, "localized.desktop", &mut warnings)
            .expect("should parse");

        assert_eq!(entry.name, "Firewall");
        assert_eq!(entry.comment.as_deref(), Some("Configure the firewall"));
    }

    #[test]
    fn empty_content_is_skipped_with_a_warning_not_a_panic() {
        let mut warnings = Vec::new();
        let entry = parse_desktop_entry("", "empty.desktop", &mut warnings);

        assert_eq!(entry, None);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn malformed_lines_inside_the_section_are_warned_about_but_do_not_abort_parsing() {
        let content = "[Desktop Entry]\nType=Application\nName=Recovers\nthis line has no equals sign\nExec=/usr/bin/recovers\n";
        let mut warnings = Vec::new();
        let entry = parse_desktop_entry(content, "recovers.desktop", &mut warnings)
            .expect("should still parse");

        assert_eq!(entry.name, "Recovers");
        assert_eq!(entry.exec.as_deref(), Some("/usr/bin/recovers"));
        assert!(warnings.iter().any(|w| w.contains("malformed line")));
    }
}
