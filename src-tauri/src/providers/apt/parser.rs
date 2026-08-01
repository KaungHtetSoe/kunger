//! Pure parsing functions for APT/dpkg command output. Deliberately
//! independent of process execution so these are directly unit-testable
//! against fixtures without needing a real Debian/Ubuntu system — see
//! `src-tauri/tests/fixtures/apt/`.

use std::collections::{HashMap, HashSet};

pub const RECORD_SEPARATOR: char = '\u{1e}';
pub const FIELD_SEPARATOR: char = '\u{1f}';

/// The `dpkg-query --showformat` format string Kunger uses for the fast,
/// batch inventory stage (one process call for every installed package,
/// not one call per package). Uses the ASCII unit/record separator
/// characters — rather than tabs, commas, or pipes — as delimiters, since
/// those cannot appear in any well-formed dpkg field, unlike ordinary
/// punctuation a package description might legitimately contain.
pub const DPKG_QUERY_FORMAT: &str = "${Package}\u{1f}${Status}\u{1f}${Version}\u{1f}${Architecture}\u{1f}${Section}\u{1f}${Priority}\u{1f}${Installed-Size}\u{1f}${Maintainer}\u{1f}${Homepage}\u{1f}${binary:Summary}\u{1f}${Depends}\u{1e}";

const DPKG_FIELD_COUNT: usize = 11;

#[derive(Debug, Clone, PartialEq)]
pub struct DpkgRecord {
    pub package: String,
    pub version: String,
    pub architecture: String,
    pub section: Option<String>,
    pub priority: Option<String>,
    pub installed_size_kb: Option<u64>,
    pub maintainer: Option<String>,
    pub homepage: Option<String>,
    pub summary: Option<String>,
    pub dependencies: Vec<String>,
}

/// Parses `dpkg-query` output produced with [`DPKG_QUERY_FORMAT`]. Skips
/// (with a warning, never a hard failure) any record that isn't in the
/// "installed" state or that doesn't have the expected field count, since
/// `dpkg-query -W` lists every package it has ever known about — including
/// purged/removed ones — and a single malformed record must never take
/// down the whole scan (see `docs/ARCHITECTURE.md` §4).
pub fn parse_dpkg_query(output: &str, warnings: &mut Vec<String>) -> Vec<DpkgRecord> {
    let mut records = Vec::new();

    for (index, raw_record) in output.split(RECORD_SEPARATOR).enumerate() {
        let record = raw_record.trim_matches('\n');
        if record.is_empty() {
            continue;
        }

        let fields: Vec<&str> = record.split(FIELD_SEPARATOR).collect();
        if fields.len() != DPKG_FIELD_COUNT {
            warnings.push(format!(
                "dpkg-query record {index} had {} fields, expected {DPKG_FIELD_COUNT} fields; skipped",
                fields.len()
            ));
            continue;
        }

        let package = fields[0].trim();
        if package.is_empty() {
            warnings.push(format!(
                "dpkg-query record {index} had an empty package name; skipped"
            ));
            continue;
        }

        if !is_installed_status(fields[1].trim()) {
            continue;
        }

        let installed_size_kb = non_empty(fields[6]).and_then(|s| s.parse::<u64>().ok());
        if non_empty(fields[6]).is_some() && installed_size_kb.is_none() {
            warnings.push(format!(
                "package \"{package}\" had a non-numeric Installed-Size (\"{}\"); left blank",
                fields[6].trim()
            ));
        }

        records.push(DpkgRecord {
            package: package.to_string(),
            version: fields[2].trim().to_string(),
            architecture: fields[3].trim().to_string(),
            section: non_empty(fields[4]).map(str::to_string),
            priority: non_empty(fields[5]).map(str::to_string),
            installed_size_kb,
            maintainer: non_empty(fields[7]).map(str::to_string),
            homepage: non_empty(fields[8]).map(str::to_string),
            summary: non_empty(fields[9]).map(str::to_string),
            dependencies: parse_depends(fields[10]),
        });
    }

    records
}

fn non_empty(field: &str) -> Option<&str> {
    let trimmed = field.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn is_installed_status(status: &str) -> bool {
    // dpkg's Status field is "<want> <flag> <status>", e.g.
    // "install ok installed" vs. "deinstall ok config-files".
    status.split_whitespace().next_back() == Some("installed")
}

/// Parses a raw `Depends` field into plain package names: strips version
/// constraints (e.g. `(>= 1.2)`) and alternative-dependency groups
/// (`a | b`), keeping only the first (preferred) alternative from each
/// comma-separated group.
fn parse_depends(field: &str) -> Vec<String> {
    let trimmed = field.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    trimmed
        .split(',')
        .filter_map(|group| {
            let first_alternative = group.split('|').next()?;
            let name = first_alternative.split_whitespace().next()?.trim();
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

/// Parses `apt-mark showmanual` output: one package name per line.
pub fn parse_manual_packages(output: &str) -> HashSet<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Parses `apt list --upgradable` output into a map of package name to
/// available version.
///
/// This format is not guaranteed stable by the `apt` project (upstream
/// documents `apt` as a friendlier CLI not intended for scripting, unlike
/// `apt-get`/`apt-cache`/`dpkg-query`), so this parser is deliberately
/// lenient: unrecognized lines are skipped with a warning rather than
/// failing the whole update-check stage.
pub fn parse_upgradable(output: &str, warnings: &mut Vec<String>) -> HashMap<String, String> {
    let mut upgradable = HashMap::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Listing...") {
            continue;
        }

        match parse_upgradable_line(line) {
            Some((package, version)) => {
                upgradable.insert(package, version);
            }
            None => {
                warnings.push(format!(
                    "could not parse \"apt list --upgradable\" line: \"{line}\""
                ));
            }
        }
    }

    upgradable
}

fn parse_upgradable_line(line: &str) -> Option<(String, String)> {
    // e.g. "firefox/jammy-updates 129.0+build2-0ubuntu0.22.04.1 amd64 [upgradable from: 128.0]"
    let mut parts = line.split_whitespace();
    let name_and_repo = parts.next()?;
    let available_version = parts.next()?;
    let package = name_and_repo.split('/').next()?;

    if package.is_empty() || available_version.is_empty() {
        return None;
    }

    Some((package.to_string(), available_version.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASIC_FIXTURE: &str = include_str!("../../../tests/fixtures/apt/dpkg_query_basic.txt");
    const MALFORMED_FIXTURE: &str =
        include_str!("../../../tests/fixtures/apt/dpkg_query_malformed.txt");
    const MANUAL_FIXTURE: &str =
        include_str!("../../../tests/fixtures/apt/apt_mark_showmanual.txt");
    const UPGRADABLE_FIXTURE: &str =
        include_str!("../../../tests/fixtures/apt/apt_list_upgradable.txt");
    const UPGRADABLE_MALFORMED_FIXTURE: &str =
        include_str!("../../../tests/fixtures/apt/apt_list_upgradable_malformed.txt");

    #[test]
    fn parses_well_formed_records_and_filters_non_installed_status() {
        let mut warnings = Vec::new();
        let records = parse_dpkg_query(BASIC_FIXTURE, &mut warnings);

        assert!(warnings.is_empty());
        // 4 records in the fixture, but "old-removed-pkg" is
        // deinstall/config-files, not installed, so only 3 should surface.
        assert_eq!(records.len(), 3);
        assert!(records.iter().all(|r| r.package != "old-removed-pkg"));
    }

    #[test]
    fn parses_full_field_set_for_a_record() {
        let mut warnings = Vec::new();
        let records = parse_dpkg_query(BASIC_FIXTURE, &mut warnings);

        let git = records
            .iter()
            .find(|r| r.package == "git")
            .expect("git record present");

        assert_eq!(git.version, "1:2.34.1-1ubuntu1.10");
        assert_eq!(git.architecture, "amd64");
        assert_eq!(git.section.as_deref(), Some("vcs"));
        assert_eq!(git.priority.as_deref(), Some("optional"));
        assert_eq!(git.installed_size_kb, Some(27648));
        assert_eq!(
            git.maintainer.as_deref(),
            Some("Alex Someone <alex@example.com>")
        );
        assert_eq!(git.homepage.as_deref(), Some("https://git-scm.com/"));
        assert_eq!(
            git.summary.as_deref(),
            Some("fast, scalable, distributed revision control system")
        );
    }

    #[test]
    fn parses_alternative_dependencies_by_taking_the_first_alternative() {
        let mut warnings = Vec::new();
        let records = parse_dpkg_query(BASIC_FIXTURE, &mut warnings);
        let git = records
            .iter()
            .find(|r| r.package == "git")
            .expect("git record present");

        assert_eq!(
            git.dependencies,
            vec!["libc6", "libcurl3-gnutls", "libpcre2-8-0"]
        );
    }

    #[test]
    fn empty_optional_fields_become_none() {
        let mut warnings = Vec::new();
        let records = parse_dpkg_query(BASIC_FIXTURE, &mut warnings);
        let libssl3 = records
            .iter()
            .find(|r| r.package == "libssl3")
            .expect("libssl3 record present");

        assert_eq!(libssl3.homepage, None);
        assert!(libssl3.dependencies.is_empty());
    }

    #[test]
    fn a_record_with_the_wrong_field_count_is_skipped_with_a_warning() {
        let mut warnings = Vec::new();
        let records = parse_dpkg_query(MALFORMED_FIXTURE, &mut warnings);

        assert!(!records.iter().any(|r| r.package == "brokenpkg"));
        assert!(warnings.iter().any(|w| w.contains("expected 11 fields")));
    }

    #[test]
    fn a_record_with_an_empty_package_name_is_skipped_with_a_warning() {
        let mut warnings = Vec::new();
        let _records = parse_dpkg_query(MALFORMED_FIXTURE, &mut warnings);

        assert!(warnings.iter().any(|w| w.contains("empty package name")));
    }

    #[test]
    fn a_non_numeric_installed_size_is_left_blank_with_a_warning_but_the_record_still_parses() {
        let mut warnings = Vec::new();
        let records = parse_dpkg_query(MALFORMED_FIXTURE, &mut warnings);

        let weird = records
            .iter()
            .find(|r| r.package == "weird-size-pkg")
            .expect("weird-size-pkg should still be parsed despite the bad size field");

        assert_eq!(weird.installed_size_kb, None);
        assert!(warnings
            .iter()
            .any(|w| w.contains("non-numeric Installed-Size")));
    }

    #[test]
    fn parsing_continues_after_malformed_records_and_recovers_valid_ones() {
        let mut warnings = Vec::new();
        let records = parse_dpkg_query(MALFORMED_FIXTURE, &mut warnings);

        assert!(records.iter().any(|r| r.package == "curl"));
    }

    #[test]
    fn empty_output_yields_no_records_and_no_warnings() {
        let mut warnings = Vec::new();
        let records = parse_dpkg_query("", &mut warnings);

        assert!(records.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn parses_manual_package_list() {
        let manual = parse_manual_packages(MANUAL_FIXTURE);

        assert_eq!(manual.len(), 3);
        assert!(manual.contains("git"));
        assert!(manual.contains("firefox"));
        assert!(manual.contains("curl"));
        assert!(!manual.contains("libssl3"));
    }

    #[test]
    fn empty_manual_output_yields_an_empty_set() {
        assert!(parse_manual_packages("").is_empty());
        assert!(parse_manual_packages("\n\n  \n").is_empty());
    }

    #[test]
    fn parses_upgradable_packages() {
        let mut warnings = Vec::new();
        let upgradable = parse_upgradable(UPGRADABLE_FIXTURE, &mut warnings);

        assert!(warnings.is_empty());
        assert_eq!(
            upgradable.get("firefox").map(String::as_str),
            Some("129.0+build2-0ubuntu0.22.04.1")
        );
        assert_eq!(
            upgradable.get("git").map(String::as_str),
            Some("1:2.34.1-1ubuntu1.11")
        );
    }

    #[test]
    fn unparseable_upgradable_lines_are_skipped_with_a_warning_not_a_failure() {
        let mut warnings = Vec::new();
        let upgradable = parse_upgradable(UPGRADABLE_MALFORMED_FIXTURE, &mut warnings);

        // The one well-formed line should still come through...
        assert_eq!(
            upgradable.get("firefox").map(String::as_str),
            Some("129.0+build2-0ubuntu0.22.04.1")
        );
        // ...while the garbage line produced a warning instead of a panic
        // or a dropped whole-batch failure.
        assert!(warnings
            .iter()
            .any(|w| w.contains("this-is-not-a-valid-line")));
    }

    #[test]
    fn upgradable_output_with_only_the_listing_header_is_empty_and_has_no_warnings() {
        let mut warnings = Vec::new();
        let upgradable = parse_upgradable("Listing... Done\n", &mut warnings);

        assert!(upgradable.is_empty());
        assert!(warnings.is_empty());
    }
}
