//! Pure parsing functions for `fc-list --format=...` output. Independent
//! of process execution — see `src-tauri/tests/fixtures/fonts/`.

pub const RECORD_SEPARATOR: char = '\u{1e}';
pub const FIELD_SEPARATOR: char = '\u{1f}';

/// The `fc-list --format` string Kunger uses. Uses ASCII unit/record
/// separators as delimiters for the same reason as the APT provider's
/// `dpkg-query` format (ADR-0010): family/style names can contain almost
/// any printable character, so a control character no font field could
/// ever legitimately contain is the only fully unambiguous choice.
pub const FC_LIST_FORMAT: &str =
    "%{file}\u{1f}%{family}\u{1f}%{style}\u{1f}%{fullname}\u{1f}%{fontformat}\u{1f}%{lang}\u{1e}";

const FC_LIST_FIELD_COUNT: usize = 6;

#[derive(Debug, Clone, PartialEq)]
pub struct FontFileRecord {
    pub file: String,
    pub family: String,
    pub style: Option<String>,
    pub fullname: Option<String>,
    pub font_format: Option<String>,
    /// Raw fontconfig language-coverage list (e.g. `"en|fr|de"`), stored
    /// as-is rather than parsed into a structured set.
    pub language_coverage: Option<String>,
}

/// Parses `fc-list` output produced with [`FC_LIST_FORMAT`]. Skips (with a
/// warning, never a hard failure) any record with the wrong field count,
/// an empty file path, or an empty family — see `docs/ARCHITECTURE.md` §4.
pub fn parse_fc_list(output: &str, warnings: &mut Vec<String>) -> Vec<FontFileRecord> {
    let mut records = Vec::new();

    for (index, raw_record) in output.split(RECORD_SEPARATOR).enumerate() {
        let record = raw_record.trim_matches('\n');
        if record.is_empty() {
            continue;
        }

        let fields: Vec<&str> = record.split(FIELD_SEPARATOR).collect();
        if fields.len() != FC_LIST_FIELD_COUNT {
            warnings.push(format!(
                "fc-list record {index} had {} fields, expected {FC_LIST_FIELD_COUNT} fields; skipped",
                fields.len()
            ));
            continue;
        }

        let file = fields[0].trim();
        if file.is_empty() {
            warnings.push(format!(
                "fc-list record {index} had an empty file path; skipped"
            ));
            continue;
        }

        let family = fields[1].trim();
        if family.is_empty() {
            warnings.push(format!(
                "fc-list record {index} (\"{file}\") had an empty family; skipped"
            ));
            continue;
        }

        records.push(FontFileRecord {
            file: file.to_string(),
            family: family.to_string(),
            style: non_empty(fields[2]).map(str::to_string),
            fullname: non_empty(fields[3]).map(str::to_string),
            font_format: non_empty(fields[4]).map(str::to_string),
            language_coverage: non_empty(fields[5]).map(str::to_string),
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

#[cfg(test)]
mod tests {
    use super::*;

    const BASIC_FIXTURE: &str = include_str!("../../../tests/fixtures/fonts/fc_list_basic.txt");
    const MALFORMED_FIXTURE: &str =
        include_str!("../../../tests/fixtures/fonts/fc_list_malformed.txt");
    const EMPTY_FIXTURE: &str = include_str!("../../../tests/fixtures/fonts/fc_list_empty.txt");

    #[test]
    fn parses_all_well_formed_records() {
        let mut warnings = Vec::new();
        let records = parse_fc_list(BASIC_FIXTURE, &mut warnings);

        assert!(warnings.is_empty());
        // 4 Noto Sans styles + 1 DejaVu Sans + 1 user-scope custom font +
        // 1 font outside all known directories (which this pure parser
        // does not filter -- that's classify_scope's job in mod.rs).
        assert_eq!(records.len(), 7);
    }

    #[test]
    fn parses_full_field_set_for_a_record() {
        let mut warnings = Vec::new();
        let records = parse_fc_list(BASIC_FIXTURE, &mut warnings);

        let bold = records
            .iter()
            .find(|r| r.file == "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf")
            .expect("bold record present");

        assert_eq!(bold.family, "Noto Sans");
        assert_eq!(bold.style.as_deref(), Some("Bold"));
        assert_eq!(bold.fullname.as_deref(), Some("Noto Sans Bold"));
        assert_eq!(bold.font_format.as_deref(), Some("TrueType"));
        assert_eq!(bold.language_coverage.as_deref(), Some("en|fr|de"));
    }

    #[test]
    fn multiple_files_share_the_same_family() {
        let mut warnings = Vec::new();
        let records = parse_fc_list(BASIC_FIXTURE, &mut warnings);

        let noto_sans_count = records.iter().filter(|r| r.family == "Noto Sans").count();
        assert_eq!(noto_sans_count, 4);
    }

    #[test]
    fn record_with_wrong_field_count_is_skipped_with_a_warning() {
        let mut warnings = Vec::new();
        let records = parse_fc_list(MALFORMED_FIXTURE, &mut warnings);

        assert!(!records.iter().any(|r| r.family == "Broken Font"));
        assert!(warnings.iter().any(|w| w.contains("expected 6 fields")));
    }

    #[test]
    fn record_with_empty_file_path_is_skipped_with_a_warning() {
        let mut warnings = Vec::new();
        let _records = parse_fc_list(MALFORMED_FIXTURE, &mut warnings);

        assert!(warnings.iter().any(|w| w.contains("empty file path")));
    }

    #[test]
    fn record_with_empty_family_is_skipped_with_a_warning() {
        let mut warnings = Vec::new();
        let _records = parse_fc_list(MALFORMED_FIXTURE, &mut warnings);

        assert!(warnings.iter().any(|w| w.contains("empty family")));
    }

    #[test]
    fn parsing_continues_after_malformed_records_and_recovers_valid_ones() {
        let mut warnings = Vec::new();
        let records = parse_fc_list(MALFORMED_FIXTURE, &mut warnings);

        assert!(records.iter().any(|r| r.family == "Good Font"));
    }

    #[test]
    fn empty_output_yields_no_records_and_no_warnings() {
        let mut warnings = Vec::new();
        let records = parse_fc_list(EMPTY_FIXTURE, &mut warnings);

        assert!(records.is_empty());
        assert!(warnings.is_empty());
    }
}
