# Kunger — Decisions Log

Architecture Decision Records (ADRs), lightweight format. Append new entries; do not rewrite history — if a decision is reversed, add a new entry that supersedes the old one and mark the old one as superseded.

---

## ADR-0001 — Product name is "Kunger", not "PackageLens"

**Date:** 2026-08-01
**Status:** Accepted

**Context:** Initial planning material (the original prompt tree) used the working name "PackageLens" throughout. The actual project/repository is named Kunger.

**Decision:** All docs, code, UI strings, and package metadata use "Kunger" as the product name. "PackageLens" is retired and should not appear in new material.

**Consequences:** None functionally — this is a naming-only change. Historical references to "PackageLens" in planning notes describe the same product under its old working name.

---

## ADR-0002 — Layered Rust architecture with domain layer isolated from I/O

**Date:** 2026-08-01
**Status:** Accepted

**Context:** The backend must support many independent, unreliable inventory sources (APT, Flatpak, desktop files, fonts, AppImage, manual detection) while keeping the domain model, classification logic, and UI free of package-manager-specific knowledge.

**Decision:** Six-layer backend (domain → providers → classification → inventory service → persistence, with commands on top), each layer depending only downward. Domain models have zero dependency on process execution, SQLite, or Tauri.

**Consequences:** Slightly more boilerplate (explicit conversions at layer boundaries) in exchange for independently testable layers and the ability to add/remove providers without touching classification, persistence, or the frontend. This is the basis the Architect agent will use to evaluate future proposals for "does this leak provider-specific logic upward."

---

## ADR-0003 — Enums over independent optional booleans in domain models

**Date:** 2026-08-01
**Status:** Accepted

**Context:** Fields like "manually installed" vs. "automatically installed" could be modeled as two separate `Option<bool>` fields, but that allows representing contradictory or doubly-unknown states.

**Decision:** Prefer a single meaningful enum (e.g., an installation-reason enum) over multiple booleans whenever the underlying states are mutually exclusive. This was already stated as a requirement in the original prompt tree (Prompt 04A) and is affirmed here as binding architecture, not just a style preference.

**Consequences:** Slightly more enum types up front; in exchange, illegal states (e.g., "both manual and automatic" or silently defaulting a missing bool to `false`) become unrepresentable.

---

## ADR-0004 — Classification confidence is a leveled enum + explicit reasons list, not a bare numeric score

**Date:** 2026-08-01
**Status:** Accepted

**Context:** The product spec requires classification to be transparent and explainable (NFR-4), not just a probability number a user has to trust blindly.

**Decision:** `ClassificationConfidence` is an ordered enum (exact levels to be finalized in `docs/CLASSIFICATION.md`), always paired with a `classification_reasons: Vec<String>` populated with the specific evidence used (e.g., "Debian section is 'fonts'", "package owns TrueType font files").

**Consequences:** UI and export always have something human-readable to show, not just a number with no justification. Slightly more data to carry per item, judged worth it for the product's core "explainability" promise.

---

## ADR-0005 — Duplicate detection never auto-merges or auto-removes

**Date:** 2026-08-01
**Status:** Accepted

**Context:** Kunger is explicitly read-only and non-destructive (product spec Section 5, Non-Goals). Cross-manager duplicate detection (e.g., Firefox via APT and Flatpak) could tempt an "auto-resolve" feature.

**Decision:** Duplicate detection only ever produces `DuplicateGroup` records for the user to review. No code path may delete, hide, merge, or otherwise mutate either side of a detected duplicate automatically, in this or any future version, without a separate explicit product decision revisiting this ADR.

**Consequences:** Users must manually act on duplicate information outside of Kunger (Kunger doesn't perform package operations at all, per non-goals). This keeps the read-only guarantee airtight and avoids a whole class of "Kunger deleted something I needed" risk.

---

## ADR-0006 — SQLite is a rebuildable cache, not the source of truth

**Date:** 2026-08-01
**Status:** Accepted

**Context:** The actual source of truth for "what's installed" is always the live system (APT database, filesystem, Flatpak installations, etc.), not Kunger's own storage.

**Decision:** The local SQLite database only ever stores derived/cached scan results and history. `rebuild_cache` must always be able to fully reconstruct valid state from a fresh scan. No feature may treat the database as authoritative in a way that would produce incorrect behavior if the database were deleted and rebuilt.

**Consequences:** Simplifies corruption/recovery handling (delete and rescan is always a safe fallback) at the cost of scan time being the only way to get fresh data (acceptable given V1 is manual/on-demand scanning only).

---

## ADR-0007 — Provider timeouts enforced at two layers (orchestration + process execution)

**Date:** 2026-08-01
**Status:** Accepted

**Context:** A single hung external command (e.g., a stalled `flatpak` call against an unreachable remote) must never hang the whole scan or the whole application.

**Decision:** Timeouts are enforced both by the inventory service around each provider's whole `scan()` call, and independently inside `process/` around each individual subprocess invocation. Providers are also expected to honor a cancellation token between internal stages.

**Consequences:** Defense in depth against hangs, at the cost of two places that need timeout configuration to stay sane relative to each other (documented together so they don't drift, e.g., process-level timeout must always be shorter than the provider-level budget it runs inside).

---

## ADR-0008 — Frontend server-state library: TanStack Query

**Date:** 2026-08-02
**Status:** Accepted (supersedes the "deferred" placeholder recorded on 2026-08-01)

**Context:** Needed a strategy for caching/refetching IPC-derived data (scan status, provider
status, inventory summary/items) in the React frontend without ad hoc `useEffect` fetching per
feature, once the real IPC command shapes existed (M4.4).

**Decision:** `@tanstack/react-query` (`src/app/QueryProvider.tsx`), with query hooks colocated
under `src/hooks/` (`useScanStatus`, `useProviderStatus`) rather than fetching inline in every
component. `useScanStatus` polls `get_scan_status` once per second only while a scan is actually
running (`refetchInterval` keyed off the current query data), and otherwise relies on the
`scan-started`/`scan-completed`/`scan-failed`/`scan-cancelled` Tauri events
(`src/services/kungerApi.ts`'s `scanEvents`) to invalidate the relevant query keys immediately —
no continuous polling while idle.

**Consequences:** Server-state caching, loading/error states, and refetch-on-focus behavior come
for free per query hook rather than being hand-rolled per feature. Also chose
`react-router-dom` (hash-based routing — `createHashRouter`, since Tauri serves static assets
with no server available to rewrite deep-linked paths) for the shell's navigation, and
`lucide-react` for icons. Note: `react-router-dom` currently has an open high-severity advisory
(GHSA-qwww-vcr4-c8h2, RSC-mode CSRF bypass); the only available fix is a downgrade to 7.11.0, and
the vulnerability class doesn't apply to Kunger (no server, no React Router RSC mode in use) —
kept at the latest 7.18.2 as an accepted, inapplicable risk rather than downgrading. Revisit if a
version above the vulnerable range (>8.2.0) is released.

---

## ADR-0009 — Domain types serialize as camelCase JSON; `id` is a provider-defined string

**Date:** 2026-08-01
**Status:** Accepted

**Context:** Domain types (`SoftwareItem`, `ProviderInventory`, `InventorySummary`, etc.) cross
the Tauri IPC boundary into TypeScript. Rust's idiomatic `snake_case` field names don't match
idiomatic TypeScript/JSON `camelCase`. Separately, `SoftwareItem::id` needs a uniqueness strategy.

**Decision:**

- All domain structs and enums use `#[serde(rename_all = "camelCase")]`, so JSON/TypeScript sees
  `packageName`, `classificationConfidence`, etc., while Rust source stays idiomatic `snake_case`.
- `ProviderError` serializes as an externally-tagged `{ "kind": "...", "message": "..." }` shape
  rather than a plain string, so the frontend can branch on error kind without string matching.
- `SoftwareItem::id` is a provider-defined `String` (e.g. `apt:firefox`,
  `flatpak:org.mozilla.firefox`), not a random UUID — ids are meant to be human-legible in logs
  and stable across scans of the same system, not globally unique across systems.

**Consequences:** No serde attribute needs repeating per-field; new fields automatically get
correct casing. The frontend's generated/hand-written TypeScript types (Prompt 08) should mirror
this camelCase shape directly rather than re-casing at the IPC boundary.

---

## ADR-0010 — `dpkg-query` output uses ASCII unit/record separators, not delimited text

**Date:** 2026-08-01
**Status:** Accepted

**Context:** The APT provider's fast inventory stage needs one batched `dpkg-query` call
returning every installed package's metadata (name, version, section, description, etc.) in a
single parseable blob, per `docs/SECURITY.md`'s "never parse human-formatted output when a
stable machine-readable format exists" and "avoid one command per package" requirements. A
package's description or maintainer field can contain almost any printable character, including
common delimiter choices like commas, pipes, or tabs.

**Decision:** The `--showformat` string passed to `dpkg-query` separates fields with the ASCII
Unit Separator (`\u{1f}`) and records with the ASCII Record Separator (`\u{1e}`) — control
characters that cannot appear in any well-formed dpkg field, rather than punctuation a
description might legitimately contain. `src-tauri/src/providers/apt/parser.rs` parses this
format; fixtures under `src-tauri/tests/fixtures/apt/` were generated with `printf` (not the
`Write` tool) to guarantee byte-exact control characters.

**Consequences:** Parsing is unambiguous regardless of description content, at the cost of
fixture files being illegible in a plain text editor (`od -c` or equivalent is needed to inspect
them). A record with an unexpected field count is treated as a parse warning and skipped, never
a fatal error — see `docs/ARCHITECTURE.md` §4.

---

## ADR-0011 — `ProcessRunner`: one timeout wraps output-reading and exit-waiting together

**Date:** 2026-08-01
**Status:** Accepted

**Context:** `src-tauri/src/process/mod.rs` is the single safe process-execution abstraction
every provider uses (`docs/ARCHITECTURE.md` §2.7). It needs both a timeout and an output-size
cap, and needs to decide how those two protections interact when a child process misbehaves in
both ways at once (e.g. hangs _and_ produces runaway output).

**Decision:** `ProcessRunner::run` wraps stdout/stderr reading and `child.wait()` in a _single_
`tokio::time::timeout(self.timeout, ...)`, rather than separate timeouts per phase. If the
output cap is hit, the reader stops early and returns `OutputTooLarge` once both streams finish
being read — but if the child is still writing past that cap and blocks on a full pipe buffer,
the process is only guaranteed to be killed when the single overall timeout elapses, not
immediately upon exceeding the byte cap.

**Consequences:** Simpler implementation and a single timeout value to reason about per call
site, at the cost of slightly delayed cleanup in the rare "hung and oversized output" case —
still bounded (never hangs forever), just not maximally responsive. Callers needing tighter
responsiveness should construct a `ProcessRunner` with a shorter `timeout`. Every
`ProcessRunner` timeout used by a provider must stay shorter than that provider's
`ScanContext::timeout` budget, per ADR-0007.

---

## ADR-0012 — Ownership-known desktop entries reuse their owning package's item id

**Date:** 2026-08-01
**Status:** Accepted

**Context:** Providers run independently and don't share state during their own `scan()` call
(ADR-0002) — but the original prompt tree requires the desktop-entry provider to "not duplicate
an APT package as a separate manual application when ownership is known." Full cross-provider
association is explicitly an Inventory Service (M4.1) responsibility
(`docs/ARCHITECTURE.md` §2.4), which doesn't exist yet.

**Decision:** When `DesktopProvider` resolves a `.desktop` file's owning package via `dpkg -S`,
it emits that item with the _same_ id the APT provider uses for the same package
(`apt:{package}`), `package_manager = PackageManager::Apt`, and leaves `category` as
`Unclassified` (full classification is deferred until the item is merged with the owning
package's richer evidence). When no owner is found, it emits a standalone item instead, with a
`desktop:{filename}` id, `package_manager = PackageManager::Manual`, classified immediately via
the classification engine using `has_desktop_launcher` + `Categories=` evidence.

**Consequences:** The "don't duplicate" requirement is satisfied structurally (via a shared id
that a future id-keyed merge in the inventory service will naturally converge) without the
desktop provider needing to know anything about APT's internal state. This makes the inventory
service's merge contract partly load-bearing already: M4.1 must merge same-id records from
different providers (not just deduplicate identical ids), with a defined field-level merge
strategy — this ADR is the reason that requirement exists, and M4.1's design should reference it
rather than rediscover the need.

---

## ADR-0013 — Inventory service: order-sensitive id merge + conservative name-based duplicate detection

**Date:** 2026-08-01
**Status:** Accepted

**Context:** M4.1 needed to fulfill the merge contract ADR-0012 anticipated (same-id records from
different providers must combine, not just deduplicate) and separately implement cross-manager
duplicate detection (different ids, same underlying software — e.g. Firefox via APT and
Flatpak) without ever auto-resolving duplicates (ADR-0005).

**Decision:**

- `inventory::merge::merge_by_id` treats the _first_ item seen for a given id as the
  authoritative base and every later same-id item as enrichment (fills empty `Option` fields,
  unions list fields de-duplicated, upgrades classification only on strictly higher confidence,
  unions reasons when confidence and category tie). This makes provider _registration order_
  load-bearing: `InventoryService::with_default_providers` registers APT before the desktop and
  font providers specifically so `apt:*` ids get their authoritative record first.
- Display-name preference during merge compares `display_name` to `package_name` with exact
  (not case-insensitive) equality to detect "was this just defaulted to the raw package name" —
  case-only improvements like "firefox" → "Firefox" are real improvements, not noise.
- `inventory::duplicates::detect_duplicates` runs _after_ merging and only groups items by
  normalized display name (lowercased, non-alphanumeric characters stripped) that also differ in
  `package_manager`. No fuzzy/similarity matching — exact normalized match only, confidence fixed
  at `Medium` (a name match alone is never `Certain`). Duplicate desktop entries and
  dpkg-owned-but-manually-detected binaries are already resolved further upstream (by the desktop
  and manual providers respectively — see their own module docs) and never reach this stage.

**Consequences:** Merge correctness depends on registration order being right, which is
documented but not compiler-enforced — a future provider that emits `apt:*`-style ids must be
registered after APT, or its enrichment would incorrectly become the base record instead.
Name-based duplicate detection will miss real duplicates with divergent naming (e.g. "GIMP" vs.
"GNU Image Manipulation Program") and is a known, documented limitation rather than a defect —
stronger matching (app-id heuristics, fuzzy string matching) is future work, not required for
v0.1's read-only inventory promise.

---

## ADR-0014 — SQLite persistence: indexed columns + full JSON blob per row; sync `rusqlite` behind a `Mutex`

**Date:** 2026-08-02
**Status:** Accepted

**Context:** M4.3 needed a schema for `scan_sessions`, `software_items`, `duplicate_groups`, and
`provider_results` that supports the filtering/sorting `list_software_items` (Prompt 08) will
need, without requiring a schema migration every time a domain field changes, and needed a
SQLite binding choice. The domain types already fully round-trip through `serde_json`
(ADR-0009).

**Decision:**

- Each table carries a handful of real, indexed SQL columns for the fields actually
  filtered/sorted on (category, package_manager, scope, installation_reason, confidence,
  version, display_name) _plus_ a `data_json` column holding the complete serialized domain
  type. Reads reconstruct the full type from `data_json`; only the indexed columns are used in
  `WHERE`/`ORDER BY` clauses. Enum values are stored as their plain serde camelCase string (e.g.
  `"commandLineTool"`) via `serde_json::to_value(..).as_str()`, reusing serde's naming instead of
  hand-written match arms per enum.
- Uses `rusqlite` (bundled SQLite, no system dependency) rather than an async driver like
  `sqlx`. `rusqlite::Connection` is synchronous and not `Sync`, so `SqliteScanRepository` wraps
  it in a `std::sync::Mutex`. Repository methods are themselves synchronous; the Tauri command
  layer (Prompt 08) is responsible for calling them via `tokio::task::spawn_blocking` so they
  never block the async runtime.
- A corrupted or unreadable database file is never a fatal error: `persistence::db::open`
  attempts to open + migrate, and on any failure renames the file aside
  (`kunger.db.corrupt-<timestamp>`) and creates a fresh one in its place, per ADR-0006 (the
  database is a rebuildable cache, never the source of truth).

**Consequences:** Adding a new field to `SoftwareItem` never requires a migration unless that
field also needs to be filterable/sortable at the SQL level — cheap for the common case, at the
cost of the schema not being a fully normalized relational model (some data is only accessible
by deserializing `data_json`, not via SQL). The `Mutex<Connection>` serializes all database
access; acceptable for a single local desktop app with modest (thousands, not millions of rows)
data volume, revisit if profiling ever shows contention.

---

## ADR-0015 — Command layer: thin `#[tauri::command]` wrappers over plain testable `_impl` functions; `Arc<AppState>` managed state

**Date:** 2026-08-02
**Status:** Accepted

**Context:** M4.4 needed to implement all eleven required IPC commands with real unit test
coverage, but `tauri::State`/`tauri::AppHandle` can't be constructed outside a running Tauri
app, and `start_inventory_scan` needs to run the actual scan in a background `tokio::spawn` task
outlived the command call itself.

**Decision:**

- Every command is a thin `#[tauri::command]` wrapper that extracts Tauri-specific parameters
  and immediately delegates to a plain `..._impl` function taking `&AppState` (or `Arc<AppState>`
  for the one command that spawns a background task). All 25 command-layer tests call the
  `_impl` functions directly, never through Tauri's invoke machinery.
- Scan lifecycle events go through a `ScanEventEmitter` trait (`TauriScanEventEmitter` for real
  use, `NoopScanEventEmitter` for tests) rather than calling `AppHandle::emit` directly from
  command logic, for the same testability reason.
- Tauri-managed state is `Arc<AppState>` (not `AppState` directly), so `start_inventory_scan` can
  clone the `Arc` into its spawned task. Every command uniformly takes
  `tauri::State<'_, Arc<AppState>>` for consistency, even ones that don't spawn anything.
- A `run_blocking` helper wraps every `ScanRepository` call in `tokio::task::spawn_blocking`
  (rusqlite is synchronous — ADR-0014) and converts the result to `CommandError` in one place.
- `list_software_items` filters/sorts/paginates in memory over `latest_items()` rather than at
  the SQL layer (see that command's module doc for the volume-based rationale).
- `export_inventory` implements the full technical inventory export (JSON/YAML/CSV) now; the
  reinstallation-manifest export mode is deliberately left for the dedicated M4.6 export
  milestone rather than bolted on here.

**Consequences:** Command logic is fully unit-testable without a running Tauri app or real
filesystem paths beyond throwaway temp SQLite files. The `#[tauri::command]` macro generates
hidden sibling items (`__cmd__<name>` etc.) in the function's _original_ module, which are not
reachable through a flat `pub use` re-export — `tauri::generate_handler!` in `lib.rs` must
reference each command by its full submodule path (e.g. `commands::scan::start_inventory_scan`),
not a re-exported flat path; this tripped up the first implementation attempt and is called out
here so it isn't rediscovered. Capability/ACL entries for these custom commands
(`capabilities/default.json`) are deferred to M4.5+, once the frontend actually calls them and
any permission errors can be diagnosed against a real `invoke()`.

## ADR-0016 — Reinstallation manifest structure; downloads via a Blob/anchor pattern, not a Tauri plugin

**Date:** 2026-08-02
**Status:** Accepted

**Context:** M4.6 needed to implement the reinstallation-manifest export mode ADR-0015 deferred,
plus a way for the frontend to actually get exported content onto disk. Product spec FR-11
requires the manifest to clearly separate what Kunger can vs. cannot automatically reproduce.

**Decision:**

- `ExportRequest` gained a `mode: full | reinstallationManifest` field (`#[serde(default)]` to
  `full`, so every pre-M4.6 caller keeps working unchanged). `export_inventory` branches on
  `(mode, format)`.
- The manifest mode groups items by whether their package manager supports non-interactive
  reinstall by name (apt/flatpak/snap/pip/pipx/npm/cargo) into a `reproducible` section — each
  group carries a plain-English `installHint` — versus everything else (manual finds, AppImages,
  unknown-manager items) into a `manualReview` section with the reason and install paths. Items
  with `installationReason == Automatic` are dropped entirely rather than placed in either
  section: reinstalling the manually-chosen packages pulls dependencies back in on its own, so
  listing them would just be noise the user has to read past.
- CSV manifest output is a single flat table (`reproducible` yes/no column) rather than two
  separate CSVs, so the format stays consistent with the full-export CSV (one file per export)
  and still opens cleanly in a spreadsheet.
- The frontend downloads exported content via `Blob` + a temporary `<a download>` element
  (`src/utils/download.ts`), not a `@tauri-apps/plugin-dialog`/`plugin-fs` save dialog. This
  avoids adding a new Tauri capability/permission surface and native dependency for what the
  WebView already does natively; the tradeoff is the user gets their WebView's default download
  behavior (typically straight to `~/Downloads`) instead of a native "Save As" location picker.
  Revisit if user feedback wants the picker.
- The export UI shows a persistent privacy notice that installation paths may contain the user's
  home directory/username (`docs/SECURITY.md` §3, product spec FR-11) rather than only mentioning
  it in documentation the user may never open.

**Consequences:** `export_inventory`'s test suite grew from 4 to 8 cases (JSON/YAML/CSV × full
vs. manifest, plus an empty-scan case for each mode). The manifest's on-page preview
(`ManifestPreview`) always fetches in JSON regardless of the format the user will eventually
download, so switching formats doesn't require a second round-trip just to re-render the preview.

## ADR-0017 — Security review fixes: CSV formula-injection guard, restrictive CSP, dropped unused opener plugin

**Date:** 2026-08-02
**Status:** Accepted

**Context:** M5.2's security review (`docs/SECURITY_REVIEW.md`) found three concrete issues: CSV
exports were vulnerable to spreadsheet formula injection (CWE-1236), `tauri.conf.json` shipped
with no Content-Security-Policy at all (scaffolding default), and the default-scaffolded
`tauri-plugin-opener` was enabled with a granted permission despite being completely unused.

**Decision:**

- Added `csv_safe()` in `commands/export.rs`: prefixes any scanned-data CSV field beginning with
  `=`, `+`, `-`, `@`, tab, or CR with a single quote, applied to every field whose value
  ultimately comes from package metadata (id, package name, display name, version, paths) in both
  CSV export modes. Kunger-authored literals (`"yes"`/`"no"`, install hints, manual-review
  reasons) are left alone since they can't be attacker-influenced.
- Set a real CSP in `tauri.conf.json`:
  `default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; form-action 'none'`.
  `connect-src` needs the `ipc:`/`http://ipc.localhost` allowance for Tauri's own IPC bridge;
  `style-src` needs `'unsafe-inline'` for `PackageManagerBreakdown`'s one dynamic inline
  bar-width style (computed from Kunger's own data, not user input). Everything else defaults to
  same-origin-only, with `object-src`/`base-uri`/`form-action` explicitly closed off since the
  app uses none of them.
- Removed `tauri-plugin-opener` entirely (Cargo dependency, `.plugin(...)` registration,
  `opener:default` capability grant, `@tauri-apps/plugin-opener` npm package) — it was scaffolded
  by default and never called from the frontend. An enabled plugin with a granted permission and
  zero call sites is attack surface with no offsetting functionality.

**Consequences:** `export.rs` gained 4 regression tests for the formula-injection guard (261 ->
265 Rust tests). The CSP and plugin removal have no unit-testable surface of their own; verified
by a clean production build, a clean `cargo build`, and a real `npm run tauri dev` run whose
Dashboard rendered correctly against the live scan cache under both changes (confirmed via
accessibility-tree read, per the verification approach documented in `docs/TESTING.md`). If a
future milestone needs the opener plugin (e.g. an "open containing folder" feature), re-add it
deliberately with a scoped permission rather than restoring the default-scaffolded one.

## ADR-0018 — GitHub Actions CI/CD; bundle targets restricted to AppImage + .deb

**Date:** 2026-08-02
**Status:** Accepted

**Context:** M5.4 needed a CI pipeline enforcing the same checks `CONTRIBUTING.md`'s PR template
already lists (fmt, clippy, test, lint, typecheck, test, build) and a release pipeline producing
installable Linux packages, without a live GitHub remote in this environment to test against.

**Decision:**

- `.github/workflows/ci.yml` runs on every push to `main` and every PR, as two independent jobs
  (`backend`, `frontend`) so a failure in one doesn't block visibility into the other. The backend
  job runs on `ubuntu-22.04` (not `ubuntu-latest`) specifically because Tauri 2's WebView
  dependency is `libwebkit2gtk-4.1-dev`, and pinning the version keeps the required system
  package list from silently drifting if GitHub changes what `ubuntu-latest` points to.
- `tauri.conf.json`'s `bundle.targets` changed from the scaffolding default `"all"` to
  `["appimage", "deb"]`. Kunger only runs on Debian/Ubuntu by design (it scans `dpkg`/`apt`
  directly) — building `.msi`/`.dmg`/other-OS bundle targets would produce artifacts for a
  platform the app doesn't actually support, so restricting the target list matches the product's
  actual scope rather than scaffolding boilerplate. Also added `category`, `shortDescription`,
  and `longDescription` to the bundle config, which feed the generated `.desktop` entry and `.deb`
  control file — previously empty scaffolding defaults.
- `.github/workflows/release.yml` uses the official `tauri-apps/tauri-action`, triggered only by
  pushing a `v*` tag, rather than hand-rolling `cargo tauri build` + artifact-upload steps. It
  creates a **draft** GitHub Release (`releaseDraft: true`) rather than publishing immediately —
  a human should review the generated bundles before making a release public, matching M5.5's
  release-checklist gate rather than auto-publishing on every tag push.

**Consequences:** Neither workflow could be executed end-to-end in this sandbox (no GitHub remote
configured, and AppImage/.deb bundling itself requires Linux tooling — `appimagetool`,
`dpkg-deb` — not present on macOS). Verified what's checkable locally: both YAML files parse as
valid YAML (`npx js-yaml`), the changed `tauri.conf.json` still passes `tauri-build`'s own
schema validation (a plain `cargo build` fails loudly on a malformed config, and didn't), and a
`npm run tauri dev` run after the config change still renders correctly. The actual Linux build
and release flow needs verification the first time this repository is pushed to GitHub with a
real remote — flagged here rather than silently assumed correct.
