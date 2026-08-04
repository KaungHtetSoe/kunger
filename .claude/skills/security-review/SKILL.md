---
name: security-review
description: Methodical security audit of Kunger's Rust backend and TypeScript frontend, following the M5.2 methodology in docs/SECURITY_REVIEW.md. Use before a release, after adding a new provider or export format, or whenever asked to review security.
---

# Kunger security review

Static code review, not a scanner report — read the actual code paths each check turns up, not
just the comments describing them. This is the exact methodology behind `docs/SECURITY_REVIEW.md`
(M5.2), which caught three real issues (CSV formula injection, missing CSP, an unused plugin with
a granted permission) that a surface-level read would have missed.

## The tracked risk areas (docs/SECURITY.md §5)

For each, grep first, then read the code the grep turns up:

1. **Command injection.** `grep -rn "Command::new\|\"-c\"\|\"sh\"\|\"bash\"" src-tauri/src/` —
   every hit outside `process/mod.rs` (and that file's own tests) is a finding. All external
   commands must go through `ProcessRunner`/`CommandSpec` as argument vectors.
2. **Path traversal / symlink escape.** For every provider that walks a directory: does it
   recurse? If yes, does it use `DirEntry::file_type()` (doesn't follow symlinks) rather than
   `fs::metadata()` (does) to decide whether to recurse into a subdirectory? If it doesn't
   recurse at all, confirm that in the code, not just a comment.
3. **Malformed/malicious input from external commands or files.** Confirm every read is bounded
   (`Read::take`, a `MAX_*_BYTES` constant) and every field is parsed as untrusted text (no
   assumption of well-formedness).
4. **Execution of discovered software.** `grep -rn "Command::new" src-tauri/src/providers/` —
   confirm no discovered path/binary is ever executed, and `.desktop` `Exec=` fields are read as
   metadata only.
5. **SQL injection.** `grep -rn "\.execute(\|\.query_row(\|\.prepare(" src-tauri/src/persistence/`
   — confirm every one uses `?N` placeholders with `params![...]`, never string-formats a
   user-controlled value into a query.
6. **CSV/spreadsheet formula injection (CWE-1236).** Any new CSV export field that comes from
   scanned package metadata (not a Kunger-authored literal) must go through `csv_safe()`
   (`commands/export.rs`) or an equivalent guard against leading `=`/`+`/`-`/`@`/tab/CR.
7. **Tauri IPC input validation.** Every command that takes frontend-supplied input should
   validate it (bounds, non-empty, etc.) before touching business logic — check
   `commands/inventory_commands.rs`'s `list_software_items_impl` for the pattern
   (`CommandError::invalid_request` before any repository call).
8. **Unbounded output / memory-exhaustion DoS.** Confirm `ProcessRunner` output caps and any new
   file-read path is similarly bounded.
9. **Privacy leakage in exports.** Any new export field containing filesystem paths needs the
   same disclosure the Export page already gives (a persistent, unavoidable notice, not just a
   line in documentation) — see `docs/SECURITY.md` §3.

## Also check, beyond the tracked list

- **CSP** (`tauri.conf.json`'s `app.security.csp`) still matches actual resource usage — no new
  external resource, inline script, or relaxed directive without updating it and justifying why
  in an ADR.
- **Tauri capabilities** (`src-tauri/capabilities/default.json`) — any newly-added plugin or
  permission actually used from the frontend? An enabled-but-unused plugin with a granted
  permission is attack surface with no offsetting functionality (this is exactly what the
  `tauri-plugin-opener` removal in ADR-0017 fixed).
- **Dependency advisories** — `cargo audit` (gates CI) and `npm audit --audit-level=high`
  (informational in CI). Cross-reference any new advisory against already-accepted ones
  (`ADR-0008` for `react-router-dom`) before treating it as a new finding — don't re-litigate an
  accepted risk, but do verify the accepted risk's reasoning still holds if the dependency tree
  changed.

## Output

Write findings to `docs/SECURITY_REVIEW.md` (update in place, dated) in the same format as the
existing table: risk area, verdict, evidence. For anything actually broken (not just
theoretical), fix it with a regression test in the same change — see the `csv_safe()` tests in
`commands/export.rs` for the bar (a dedicated unit test for the guard function itself, plus an
end-to-end test proving a realistic malicious value gets neutralized in real output). Then follow
the `ship` skill to land it.
