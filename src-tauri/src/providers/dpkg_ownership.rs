//! Shared `dpkg -S` file-ownership resolution, used by any provider that
//! needs to associate filesystem paths with an owning APT package (desktop
//! entries, fonts, and future providers like AppImage/manual detection).

use std::collections::HashMap;
use std::path::PathBuf;

use crate::process::{CommandSpec, ProcessRunner, RunError};

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

/// Batched `dpkg -S` ownership lookup for a set of paths: one process call
/// regardless of how many paths are given, never one call per path.
/// Returns an empty map (not an error) for an empty `paths` slice, without
/// running any command.
pub async fn resolve_owners(
    runner: &ProcessRunner,
    dpkg_bin: &str,
    paths: &[PathBuf],
) -> Result<HashMap<String, String>, RunError> {
    if paths.is_empty() {
        return Ok(HashMap::new());
    }

    let spec = CommandSpec::new(dpkg_bin)
        .arg("-S")
        .args(paths.iter().map(|p| p.to_string_lossy().into_owned()));

    runner
        .run_allow_any_exit(&spec)
        .await
        .map(|output| parse_dpkg_search(&output.stdout))
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

    #[tokio::test]
    async fn resolve_owners_with_no_paths_short_circuits_without_running_a_command() {
        let runner = ProcessRunner::default();
        let owners = resolve_owners(&runner, "kunger-nonexistent-dpkg-xyz", &[])
            .await
            .expect("should not error");

        assert!(owners.is_empty());
    }

    #[tokio::test]
    async fn resolve_owners_surfaces_a_run_error_when_dpkg_is_missing() {
        let runner = ProcessRunner::default();
        let paths = vec![PathBuf::from("/usr/share/applications/firefox.desktop")];

        let result = resolve_owners(&runner, "kunger-nonexistent-dpkg-xyz", &paths).await;

        assert!(matches!(result, Err(RunError::NotFound(_))));
    }
}
