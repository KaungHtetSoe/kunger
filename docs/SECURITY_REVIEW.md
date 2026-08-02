# Kunger — Security Review (M5.2)

Status: Completed 2026-08-02, pre-v0.1.0 release. This reviews the nine risk areas
`docs/SECURITY.md` §5 flagged as "review before release," plus anything else surfaced along the
way. Re-run the checks below (they're mostly greps and a manual read) if this file goes stale —
it is a snapshot of one pass, not a standing guarantee.

## Method

Static review of the Rust backend and TypeScript frontend: targeted `grep` sweeps for each risk
class, followed by reading the actual code paths those sweeps turned up (not just the comments
describing them). No dynamic scanning tools were available in this sandbox (see "Tooling gaps"
below) — this is a manual code review, not a scanner report.

## The nine tracked risk areas

| #   | Risk area                                            | Verdict  | Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| --- | ---------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | Command injection via provider subprocesses          | **Pass** | Every external command goes through `process::ProcessRunner`/`CommandSpec`, which takes an argument vector and never builds a shell string. `grep`'d the whole tree for `Command::new`/`"-c"`/`"sh"`/`"bash"` outside `process/mod.rs` — only hits are in that module's own tests, using `sh -c` deliberately to exercise timeout/truncation behavior, not to run untrusted input.                                                                                                                                                                                                                                                                                                                                                   |
| 2   | Path traversal / symlink escape during bounded scans | **Pass** | `desktop/mod.rs` recurses up to `MAX_RECURSION_DEPTH = 2` (XDG subdirectory support) but explicitly uses `DirEntry::file_type()`, which does not follow symlinks, so a symlinked directory is never recursed into — a scan root can't be escaped via a symlink pointing outside it (see the comment at `desktop/mod.rs` around the recursion helper). `manual/mod.rs` and `appimage/mod.rs` don't recurse at all — `collect_opt_entries` and `collect_flat_files` are single-level `read_dir` calls with no self-recursion, confirmed by reading the functions, not just their doc comments. `fs::metadata` (which does follow symlinks) is only used to stat a single already-discovered entry, never to walk into a new directory. |
| 3   | Malicious/malformed `.desktop` files                 | **Pass** | Reads are capped at `MAX_DESKTOP_FILE_BYTES` (1 MiB) via `Read::take`, output is parsed as untrusted text, and the `Exec=` field is read as metadata only — never executed (verified no `Command::new` anywhere near desktop-entry or AppImage-integration parsing). Parser has dedicated fixture tests.                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 4   | Malformed font metadata                              | **Pass** | `fc-list` output goes through `ProcessRunner` (timeout + size cap) and a fixture-tested parser that treats every field as untrusted text.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 5   | Malicious AppImage filenames                         | **Pass** | AppImage files are never executed or opened — only stat'd (size/mtime) and string-matched against `.desktop` file content for integration metadata. Filenames are treated as opaque display strings, not interpreted.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 6   | SQLite injection in persistence-layer queries        | **Pass** | Every `execute`/`query_row`/`prepare` call in `persistence/` uses `?N` placeholders with `params![...]` — none string-format user-controlled values into SQL. `list_software_items`'s `search` filter never touches SQL at all: it's an in-memory `to_lowercase().contains()` over `latest_items()` (ADR-0014), so there's no SQL surface for it to inject into in the first place.                                                                                                                                                                                                                                                                                                                                                  |
| 7   | Tauri IPC input validation                           | **Pass** | `list_software_items_impl` rejects `page == 0` and `pageSize` outside `1..=MAX_PAGE_SIZE` with `CommandError::invalid_request` before touching the repository. `get_software_item_impl` rejects an empty/whitespace-only `id`. Enum-typed fields (category, package manager, scope, etc.) are rejected at the serde-deserialization boundary before the command body ever runs, by construction — an invalid variant string can't reach command logic. `id` itself is only ever used as an in-memory equality key against `SoftwareItem::id`, never as a filesystem path or SQL fragment.                                                                                                                                            |
| 8   | Unbounded output / memory-exhaustion DoS             | **Pass** | `ProcessRunner` caps subprocess stdout/stderr and marks output as truncated rather than growing unbounded (`process/mod.rs`). File reads for `.desktop`/AppImage-integration content are similarly capped at 1 MiB via `Read::take`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 9   | Privacy leakage in exports (usernames/home paths)    | **Pass** | Addressed in M4.6: the Export page shows a persistent, unavoidable notice that exported content can include installation paths containing the user's home directory/username, before any download happens — not just a line in documentation the user may never read.                                                                                                                                                                                                                                                                                                                                                                                                                                                                |

## Findings from this pass (fixed, with regression tests)

### CSV/spreadsheet formula injection (CWE-1236) — fixed

Both CSV export paths (`export_csv` and `export_manifest_csv` in `commands/export.rs`) wrote
scanned package IDs, names, display names, versions, and paths into cells without checking for a
leading `=`, `+`, `-`, `@`, tab, or carriage return. Excel, LibreOffice Calc, and Google Sheets
all treat a cell starting with one of those characters as a formula when the CSV is opened —
not literal text. Since package metadata comes from data Kunger doesn't control (a malicious or
corrupted package could set a name like `=cmd|'/c calc'!A1`), this was a real if narrow injection
surface into whatever spreadsheet app the user opens the export in.

**Fix:** added `csv_safe()`, which prefixes any value starting with one of those characters with
a single quote — the standard mitigation, which forces spreadsheet apps to render the cell as
text without changing the underlying data or breaking the `csv` crate's own comma/quote escaping.
Applied to every scanned-data field written into either CSV export mode (id, package name,
display name, version, paths) — not to Kunger-authored literals like `"yes"`/`"no"` or the
install-hint strings, which are never attacker-influenced.

**Regression tests:** `csv_safe_prefixes_formula_leading_characters`,
`csv_safe_leaves_ordinary_values_untouched`,
`full_csv_export_neutralizes_formula_prefixes_in_scanned_fields`,
`manifest_csv_neutralizes_formula_prefixes_in_scanned_fields` (`commands/export.rs`).

### No Content-Security-Policy set — fixed

`tauri.conf.json` had `"csp": null`, the scaffolding default, meaning the WebView had no CSP at
all. Tauri's own security guidance is explicit that CSP "should be as restricted as possible."
Kunger's blast radius from this was already low (React auto-escapes, no
`dangerouslySetInnerHTML` anywhere, no `fetch`/`XMLHttpRequest` usage, no remote content ever
loaded), but shipping with no CSP is a standard, easily-avoidable hardening gap.

**Fix:** set
`default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; form-action 'none'`.
`connect-src` needs the `ipc:`/`http://ipc.localhost` allowance for Tauri's IPC bridge itself;
`style-src` needs `'unsafe-inline'` for the one dynamic inline `style` in
`PackageManagerBreakdown` (a computed bar-width percentage from Kunger's own data, not
attacker-controlled). Everything else is same-origin only. `object-src 'none'`, `base-uri 'none'`,
and `form-action 'none'` close off plugin embeds, `<base>`-tag hijacking, and form submission —
none of which the app uses, so there's no functional cost.

**Verification:** production build (`npm run build`) succeeds; `npm run tauri dev` starts clean
and the Dashboard renders with real cached data (confirmed via accessibility-tree read) under the
new CSP, so it isn't blocking the IPC calls or the app's own same-origin script/styles.

### Unused `tauri-plugin-opener` — removed

The default Tauri scaffolding registers `tauri-plugin-opener` (opens URLs/files via the OS
default handler) and grants it `opener:default` in `capabilities/default.json`. Kunger never
calls it from the frontend (`grep`'d for `@tauri-apps/plugin-opener` imports and any
`opener`-related `invoke` calls — none) and has no feature that needs it (no "open containing
folder," no external link handling). An enabled-but-unused plugin with a granted permission is
attack surface with no offsetting functionality — removed the Rust dependency, the
`.plugin(tauri_plugin_opener::init())` call, the `opener:default` capability grant, and the
`@tauri-apps/plugin-opener` JS package.

**Verification:** `cargo build`, full Rust suite (265/265), `npm run build`, full frontend suite
(82/82) all pass with the plugin removed; `npm run tauri dev` starts clean and renders correctly.

## Tooling gaps (accepted)

- **`cargo audit` and `npm audit` now run in CI** (`.github/workflows/ci.yml`, added in M5.4):
  `cargo audit` gates the backend job (fails the build on a new advisory), `npm audit
--audit-level=high` runs informationally (`continue-on-error: true`) since it currently reports
  the one pre-existing, already-reviewed advisory below and a hard gate would permanently red the
  build over a risk that's been explicitly accepted. Neither could be run in this sandbox at
  review time (no network access to install `cargo-audit`, and `npm audit`'s output was already
  captured directly via `npm audit` — see below) — GitHub's runners have full network access, so
  this is real coverage going forward, not aspirational.
- **`npm audit` shows one pre-existing high-severity advisory**: `react-router-dom` /
  `react-router` (GHSA-qwww-vcr4-c8h2, an RSC-mode CSRF bypass). Already reviewed and accepted in
  ADR-0008: Kunger has no server and never uses React Router's RSC/data mode, so the vulnerable
  code path is unreachable. No fix is available above the vulnerable range without downgrading to
  an older release; kept the current version.
- **No dynamic/fuzz testing** of the parsers (`.desktop`, font, APT field-splitting). They're
  fixture-tested against real captured output and bounded-read, but a malformed-input fuzzer
  wasn't run. Reasonable to defer given the bounded-read/never-panics-on-bad-input properties are
  already covered by unit tests per parser; flagging here rather than silently skipping it.

## Overall verdict

No critical or high-severity issue found that blocks release. The CSV formula-injection fix is
the most significant finding (real, if narrow, injection surface with a standard well-tested
fix); the CSP and unused-plugin items are hardening improvements rather than exploitable bugs
given Kunger's current feature set. All three are fixed and covered by regression tests or
build/log/accessibility-tree verification. Proceeding to M5.3 (performance review).
