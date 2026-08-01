//! Parses `dpkg -S <path>...` output to resolve which package (if any)
//! owns a given file path.

use std::collections::HashMap;

/// Parses batched `dpkg -S` output into a map of file path to owning
/// package name.
///
/// Each owned path produces a line like `package: /path/to/file`, or
/// `package1, package2: /path/to/file` when multiple packages claim the
/// same path (e.g. via diversions or alternatives) — the first-listed
/// package is used for attribution in that case. Unowned paths produce no
/// stdout line at all; their failure is reported on stderr with a non-zero
/// overall exit code even when other paths in the same batch succeeded, so
/// callers must invoke `dpkg -S` via
/// [`crate::process::ProcessRunner::run_allow_any_exit`] rather than
/// `run`, and only this function's stdout-only parsing is needed to know
/// which paths resolved.
pub fn parse_dpkg_search(output: &str) -> HashMap<String, String> {
    let mut owners = HashMap::new();

    for line in output.lines() {
        // Package names never contain ':', so the first ": " is always the
        // real delimiter even if a (highly unusual) path itself contained
        // that substring.
        let Some((packages, path)) = line.split_once(": ") else {
            continue;
        };

        let path = path.trim();
        // Defensive: a genuine dpkg -S match line always has an absolute
        // path after the colon. This also guards against stray
        // dpkg/dpkg-query diagnostic lines (normally sent to stderr, never
        // mixed into the stdout this function parses) being misread as a
        // package/path pair if that separation is ever imperfect.
        if !path.starts_with('/') {
            continue;
        }

        let Some(first_package) = packages.split(',').next() else {
            continue;
        };
        let first_package = first_package.trim();
        if first_package.is_empty() {
            continue;
        }

        owners.insert(path.to_string(), first_package.to_string());
    }

    owners
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_owner_lines() {
        let output = "firefox: /usr/share/applications/firefox.desktop\n\
                       git: /usr/share/applications/git-gui.desktop\n";

        let owners = parse_dpkg_search(output);

        assert_eq!(
            owners
                .get("/usr/share/applications/firefox.desktop")
                .map(String::as_str),
            Some("firefox")
        );
        assert_eq!(
            owners
                .get("/usr/share/applications/git-gui.desktop")
                .map(String::as_str),
            Some("git")
        );
    }

    #[test]
    fn multiple_owning_packages_use_the_first_listed_one() {
        let output = "vim, vim-runtime: /usr/share/applications/vim.desktop\n";

        let owners = parse_dpkg_search(output);

        assert_eq!(
            owners
                .get("/usr/share/applications/vim.desktop")
                .map(String::as_str),
            Some("vim")
        );
    }

    #[test]
    fn unrecognized_or_stderr_style_lines_are_ignored_not_panicked_on() {
        let output = "dpkg-query: no path found matching pattern /opt/custom/app.desktop\n\
                       firefox: /usr/share/applications/firefox.desktop\n";

        let owners = parse_dpkg_search(output);

        assert_eq!(owners.len(), 1);
        assert!(owners.contains_key("/usr/share/applications/firefox.desktop"));
    }

    #[test]
    fn empty_output_yields_an_empty_map() {
        assert!(parse_dpkg_search("").is_empty());
    }
}
