//! Pure parsing functions for `flatpak list --columns=...` output.
//! Independent of process execution — see `src-tauri/tests/fixtures/flatpak/`.

/// Column order requested from `flatpak list --columns=...`. Flatpak emits
/// one tab-separated row per line in exactly this order; unlike `apt list`,
/// this is a documented, stable machine-readable mode of the `flatpak` CLI.
pub const FLATPAK_COLUMNS: &str = "application,name,version,branch,arch,origin,size";
const FLATPAK_FIELD_COUNT: usize = 7;

#[derive(Debug, Clone, PartialEq)]
pub struct FlatpakRecord {
    pub application: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub branch: Option<String>,
    pub arch: Option<String>,
    pub origin: Option<String>,
    pub installed_size_bytes: Option<u64>,
}

/// Parses `flatpak list --columns=application,name,version,branch,arch,origin,size`
/// output. Skips (with a warning, never a hard failure) any line with the
/// wrong field count or an empty application id — see
/// `docs/ARCHITECTURE.md` §4.
pub fn parse_flatpak_list(output: &str, warnings: &mut Vec<String>) -> Vec<FlatpakRecord> {
    let mut records = Vec::new();

    for (index, raw_line) in output.lines().enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != FLATPAK_FIELD_COUNT {
            warnings.push(format!(
                "flatpak list line {index} had {} fields, expected {FLATPAK_FIELD_COUNT} fields; skipped",
                fields.len()
            ));
            continue;
        }

        let application = fields[0].trim();
        if application.is_empty() {
            warnings.push(format!(
                "flatpak list line {index} had an empty application id; skipped"
            ));
            continue;
        }

        let installed_size_bytes = non_empty(fields[6]).and_then(parse_human_size);
        if non_empty(fields[6]).is_some() && installed_size_bytes.is_none() {
            warnings.push(format!(
                "could not parse installed size \"{}\" for \"{application}\"; left blank",
                fields[6].trim()
            ));
        }

        records.push(FlatpakRecord {
            application: application.to_string(),
            name: non_empty(fields[1]).map(str::to_string),
            version: non_empty(fields[2]).map(str::to_string),
            branch: non_empty(fields[3]).map(str::to_string),
            arch: non_empty(fields[4]).map(str::to_string),
            origin: non_empty(fields[5]).map(str::to_string),
            installed_size_bytes,
        });
    }

    records
}

fn non_empty(field: &str) -> Option<&str> {
    let trimmed = field.trim();
    if trimmed.is_empty() || trimmed == "-" {
        None
    } else {
        Some(trimmed)
    }
}

/// Parses Flatpak's human-readable size column (e.g. `"312.5 MB"`,
/// `"780 MB"`) into a byte count. Flatpak's `list --columns` mode does not
/// expose a raw byte count, only this formatted string (via GLib's
/// `g_format_size`, which uses decimal SI units — 1 MB = 1,000,000 bytes,
/// not 1,048,576), so this is a best-effort, documented approximation
/// rather than an exact figure. Returns `None` on anything unrecognized
/// rather than guessing.
pub fn parse_human_size(input: &str) -> Option<u64> {
    let trimmed = input.trim();
    let split_at = trimmed.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
    let (number, unit) = trimmed.split_at(split_at);

    let value: f64 = number.trim().parse().ok()?;
    if value.is_sign_negative() {
        return None;
    }

    let multiplier = match unit.trim().to_ascii_lowercase().as_str() {
        "b" | "byte" | "bytes" => 1.0,
        "kb" => 1_000.0,
        "mb" => 1_000_000.0,
        "gb" => 1_000_000_000.0,
        "tb" => 1_000_000_000_000.0,
        "kib" => 1024.0,
        "mib" => 1024.0 * 1024.0,
        "gib" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };

    Some((value * multiplier).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const APPS_USER_FIXTURE: &str =
        include_str!("../../../tests/fixtures/flatpak/flatpak_list_apps_user.txt");
    const RUNTIMES_USER_FIXTURE: &str =
        include_str!("../../../tests/fixtures/flatpak/flatpak_list_runtimes_user.txt");
    const MALFORMED_FIXTURE: &str =
        include_str!("../../../tests/fixtures/flatpak/flatpak_list_malformed.txt");
    const EMPTY_FIXTURE: &str =
        include_str!("../../../tests/fixtures/flatpak/flatpak_list_empty.txt");

    #[test]
    fn parses_well_formed_app_records() {
        let mut warnings = Vec::new();
        let records = parse_flatpak_list(APPS_USER_FIXTURE, &mut warnings);

        assert!(warnings.is_empty());
        assert_eq!(records.len(), 2);

        let firefox = records
            .iter()
            .find(|r| r.application == "org.mozilla.firefox")
            .expect("firefox present");
        assert_eq!(firefox.name.as_deref(), Some("Firefox"));
        assert_eq!(firefox.version.as_deref(), Some("129.0"));
        assert_eq!(firefox.branch.as_deref(), Some("stable"));
        assert_eq!(firefox.arch.as_deref(), Some("x86_64"));
        assert_eq!(firefox.origin.as_deref(), Some("flathub"));
        assert_eq!(firefox.installed_size_bytes, Some(312_500_000));
    }

    #[test]
    fn dash_placeholder_version_becomes_none() {
        let mut warnings = Vec::new();
        let records = parse_flatpak_list(RUNTIMES_USER_FIXTURE, &mut warnings);

        let mesa = records
            .iter()
            .find(|r| r.application == "org.freedesktop.Platform.GL.default")
            .expect("mesa extension-like runtime present");

        assert_eq!(mesa.version, None);
    }

    #[test]
    fn line_with_wrong_field_count_is_skipped_with_a_warning() {
        let mut warnings = Vec::new();
        let records = parse_flatpak_list(MALFORMED_FIXTURE, &mut warnings);

        assert!(!records
            .iter()
            .any(|r| r.application == "org.example.Broken"));
        assert!(warnings.iter().any(|w| w.contains("expected 7 fields")));
    }

    #[test]
    fn line_with_empty_application_id_is_skipped_with_a_warning() {
        let mut warnings = Vec::new();
        let _records = parse_flatpak_list(MALFORMED_FIXTURE, &mut warnings);

        assert!(warnings.iter().any(|w| w.contains("empty application id")));
    }

    #[test]
    fn unparseable_size_is_left_blank_with_a_warning_but_record_still_parses() {
        let mut warnings = Vec::new();
        let records = parse_flatpak_list(MALFORMED_FIXTURE, &mut warnings);

        let bad_size = records
            .iter()
            .find(|r| r.application == "org.example.BadSize")
            .expect("record should still be parsed despite the bad size field");

        assert_eq!(bad_size.installed_size_bytes, None);
        assert!(warnings
            .iter()
            .any(|w| w.contains("could not parse installed size")));
    }

    #[test]
    fn parsing_continues_after_malformed_lines_and_recovers_valid_ones() {
        let mut warnings = Vec::new();
        let records = parse_flatpak_list(MALFORMED_FIXTURE, &mut warnings);

        assert!(records.iter().any(|r| r.application == "org.example.Good"));
    }

    #[test]
    fn empty_output_yields_no_records_and_no_warnings() {
        let mut warnings = Vec::new();
        let records = parse_flatpak_list(EMPTY_FIXTURE, &mut warnings);

        assert!(records.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn parse_human_size_handles_decimal_si_units() {
        assert_eq!(parse_human_size("312.5 MB"), Some(312_500_000));
        assert_eq!(parse_human_size("1 GB"), Some(1_000_000_000));
        assert_eq!(parse_human_size("500 kB"), Some(500_000));
        assert_eq!(parse_human_size("128 bytes"), Some(128));
    }

    #[test]
    fn parse_human_size_handles_binary_units_defensively() {
        assert_eq!(parse_human_size("1 KiB"), Some(1024));
        assert_eq!(parse_human_size("1 MiB"), Some(1024 * 1024));
    }

    #[test]
    fn parse_human_size_rejects_unrecognized_units_and_negative_numbers() {
        assert_eq!(parse_human_size("notasize"), None);
        assert_eq!(parse_human_size("12 furlongs"), None);
        assert_eq!(parse_human_size("-5 MB"), None);
    }
}
