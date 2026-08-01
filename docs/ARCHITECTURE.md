# Kunger — Architecture

Status: Draft v0.1
Based on: `docs/PRODUCT_SPEC.md`
Scope: Design only — no implementation in this document

---

## 1. Technology Stack

- **Shell / runtime:** Tauri 2
- **Backend:** Rust
- **Frontend:** React + TypeScript + Tailwind CSS
- **Local storage:** SQLite (cache/history, not source of truth)
- **Backend tests:** Rust unit + integration tests
- **Frontend tests:** Vitest
- **CI:** GitHub Actions

---

## 2. Layered Rust Architecture

The backend is organized into six layers with a strict one-directional dependency rule: each layer may depend only on layers below it. No layer reaches "up."

```
Tauri command layer        (commands/)
        │
Inventory service layer    (inventory/)
        │
Classification layer       (classification/)
        │
Provider layer              (providers/)
        │
Persistence layer          (persistence/)   ← used by inventory + commands, not by providers/classification
        │
Domain layer                (domain/)
```

Domain models have **no** dependency on process execution, SQLite, or Tauri. This is the architectural boundary the Architect agent is responsible for protecting: package-manager-specific logic must never leak into the domain layer or into the frontend — it lives only inside individual providers.

### 2.1 Domain layer (`domain/`)

Pure data types: `SoftwareItem`, category/enum types, provider result types (`ProviderInventory`, `ProviderError`, `ProviderWarning`), `InventorySummary`, duplicate-group types. Serde-serializable, no I/O, no async, fully unit-testable in isolation. See Prompt 04 series for the exact field list already specified in the product spec's data questions (Section 2).

### 2.2 Provider layer (`providers/`)

One module per inventory source: `apt/`, `flatpak/`, `desktop/`, `fonts/`, `appimage/`, `manual/`. Every provider implements the shared `InventoryProvider` async trait (see Section 5). Providers own all knowledge of their specific source format; nothing outside a provider module should know, e.g., what `dpkg-query` output looks like.

Providers depend on the `process/` module for safe external command execution and on `domain/` for the types they populate. Providers must never depend on `persistence/` or `commands/`.

### 2.3 Classification layer (`classification/`)

Pure functions (plus a small rule-priority table) that take raw provider items and produce classification results: primary category, secondary categories, confidence, and reasons. No I/O. Fully table-driven and unit-testable — see `docs/CLASSIFICATION.md` (to be written as part of Prompt 04C).

### 2.4 Inventory service layer (`inventory/`)

Orchestrates: runs providers (independently, with timeouts), merges results, deduplicates provider-internal records, invokes classification, associates related records (desktop entries ↔ APT packages, fonts ↔ packages), performs cross-manager duplicate detection, and produces the final `InventorySummary` plus the full item list, tolerating and recording partial failures at every step.

### 2.5 Persistence layer (`persistence/`)

SQLite access: schema/migrations, repository interfaces (`ScanRepository`, `SoftwareItemRepository`, `SettingsRepository`, etc.), transactional writes. Used by the inventory service (to persist a completed scan) and by the command layer (to serve cached data without rerunning a scan). Not used by providers or classification directly.

### 2.6 Tauri command layer (`commands/`)

Thin translation layer between the frontend and the inventory/persistence layers. Validates all input from the frontend, maps internal errors to typed IPC error responses, emits scan-progress events. Contains no business logic itself.

### 2.7 Supporting modules

- `process/` — the single safe abstraction for running external commands (used only by providers). Enforces: no shell interpolation, argument-vector execution, timeouts, output size limits, structured exit-status/stderr handling.
- `errors/` — shared error types and conversions between layers (`ProviderError` → domain-level error → IPC error), so that error handling stays typed end-to-end instead of collapsing to strings at the boundary.

### 2.8 Suggested directory structure

```
src-tauri/src/
├── domain/
│   ├── software_item.rs
│   ├── enums.rs            # SoftwareCategory, PackageManager, InstallationScope, ...
│   ├── provider_result.rs  # ProviderInventory, ProviderError, ProviderWarning
│   └── summary.rs          # InventorySummary
├── providers/
│   ├── mod.rs               # InventoryProvider trait, MockInventoryProvider
│   ├── apt/
│   ├── flatpak/
│   ├── desktop/
│   ├── fonts/
│   ├── appimage/
│   └── manual/
├── classification/
│   ├── mod.rs
│   └── rules.rs
├── inventory/
│   ├── service.rs            # orchestration
│   ├── merge.rs               # cross-provider association
│   └── duplicates.rs
├── persistence/
│   ├── schema.rs / migrations/
│   ├── repositories/
│   └── db.rs
├── commands/
│   └── *.rs                   # one module per IPC command group
├── process/
│   └── runner.rs               # safe process execution abstraction
└── errors/
    └── mod.rs
```

### 2.9 Frontend structure

```
src/
├── app/                # app shell, routing, providers (React context)
├── components/         # shared/presentational components
├── features/
│   ├── dashboard/
│   ├── inventory/
│   ├── software-details/
│   ├── filters/
│   └── export/
├── hooks/
├── services/            # typed Tauri IPC client wrappers
├── types/               # TypeScript types mirroring Rust domain/IPC types
└── utils/
```

The frontend never encodes package-manager-specific logic (e.g., "if apt then..."); it renders whatever the domain model + classification already resolved. Package-manager-aware logic belongs entirely in the Rust provider/classification layers.

---

## 3. Domain Models (design summary)

`SoftwareItem` is the central model. Full field list is defined in the product spec's Section 2 questions and enumerated in Prompt 04A; at the architecture level, the important design decisions are:

- **Enums over optional booleans.** E.g., `manually_installed` / `automatically_installed` collapse into a single `InstallationScope`-adjacent enum rather than two independently-settable booleans that could contradict each other (see `docs/DECISIONS.md`).
- **Confidence is structured, not a float alone.** `ClassificationConfidence` is an ordered enum (e.g., `Certain`, `High`, `Medium`, `Low`, `Unknown`) paired with a `Vec<String>` of human-readable `classification_reasons`, so the UI can always explain a classification rather than showing a bare number.
- **Warnings travel with the item, not just the provider.** Item-level `warnings: Vec<String>` allow surfacing "this .desktop file had a malformed Exec field" without failing the whole item.
- **Metadata is an escape hatch, not a dumping ground.** A typed `metadata: HashMap<String, String>` (or similarly bounded structure) exists for provider-specific extra fields that don't warrant a first-class field yet, to avoid constant schema churn — but structured fields are preferred whenever a piece of data is used by classification, duplicate detection, or the UI directly.

---

## 4. Provider Interface

```rust
#[async_trait::async_trait]
pub trait InventoryProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    async fn is_available(&self) -> bool;
    async fn scan(&self, ctx: &ScanContext) -> ProviderInventory;
    fn metadata(&self) -> ProviderMetadata; // version info, capabilities
}
```

Design points:

- `scan` always returns a `ProviderInventory` value (never a bare `Result` that can propagate a panic-worthy error out of the trait) — it embeds its own errors/warnings so a provider failure is data, not a crash.
- `ScanContext` carries the timeout budget, cancellation token, and any scan-level options (e.g., "skip expensive dependency resolution").
- `ProviderInventory` includes: items found so far, `Vec<ProviderWarning>`, optional fatal `ProviderError`, scan start/end timestamps, and duration — so even a fatally-failed provider still reports whatever partial state it has and clear timing data.
- The inventory service enforces the timeout at the orchestration level (via `tokio::time::timeout` or equivalent) in addition to whatever internal timeout the provider applies to its own subprocess calls — belt and suspenders, since a hung subprocess must never hang the whole scan.
- `MockInventoryProvider` (test-only) implements the trait with configurable canned results/delays/failures, enabling inventory-service-level tests without any real provider.

---

## 5. Classification Pipeline

1. Each provider tags items with whatever source-specific signal it directly observed (Debian section, presence of `.desktop` launcher, `.so` files, header files, font files, etc.) as raw, structured evidence — not yet a category.
2. The classification layer runs a **priority-ordered rule set** over that evidence (strongest, most direct signals first — e.g., Debian section beats package-name suffix heuristics) and accumulates: chosen primary category, any secondary categories, a confidence level, and a reasons list.
3. Items with no rule match at all are `Unclassified` with `Unknown` confidence, never force-fit into a guessed category.
4. The full rule table, priority order, and confidence rubric are documented in `docs/CLASSIFICATION.md`, produced under Prompt 04C — this document only fixes the pipeline shape, not the rule contents.

---

## 6. Confidence Scoring (design shape)

Confidence is derived from **how many independent, non-contradictory signals** support a category, and how direct those signals are (e.g., "Debian section is exactly `fonts`" is stronger than "package name ends in `-doc`"). The classification layer produces both the level and the specific reasons list so the UI/export can always show _why_, never just a bare score. Exact weighting is defined in `docs/CLASSIFICATION.md`, not here.

---

## 7. Duplicate Detection (design shape)

Runs after per-item classification, at the inventory-service layer, comparing items **across** providers (never within a single provider's own list, which providers are expected to already dedupe internally). Candidate signals: normalized display name similarity, `.desktop` `StartupWMClass`/app-id matching to a Flatpak application ID, a manually-detected binary path matching a known dpkg-owned path. Produces `DuplicateGroup` records (member item IDs + reason + confidence) that are surfaced to the user — **Kunger never merges or removes either side automatically**, consistent with the read-only, non-destructive product requirement.

---

## 8. Caching Strategy

- SQLite holds the **most recent completed scan** plus a bounded history of prior scan summaries, sufficient to compute new/removed/version-changed diffs (per product spec FR-13).
- The cache is explicitly a derived artifact: it can be safely dropped and rebuilt via a fresh scan (`rebuild_cache` command) with no loss of authority, because the filesystem/package managers are always the source of truth.
- Reads that only need "last known state" (e.g., opening the app before triggering a new scan) are served from SQLite without re-running providers.
- Writes happen once per completed (or partially-completed) scan, inside a transaction, so a crash mid-write cannot leave a half-written scan looking valid.

---

## 9. SQLite Schema (design-level, not final DDL)

Core tables:

- `scan_sessions` (id, started_at, completed_at, status, summary JSON/columns)
- `provider_results` (scan_id FK, provider_id, status, warnings JSON, error JSON, duration_ms)
- `software_items` (scan_id FK, plus the full `SoftwareItem` field set, indexed on name/category/package_manager for search+filter)
- `classification_results` — either folded into `software_items` or kept separate if we want to preserve historical reclassification without touching the raw item (decision deferred to implementation time)
- `duplicate_groups` (scan_id FK, group id, member item ids, reason, confidence)
- `settings` (key/value, app-level preferences e.g. last view mode)

Indexing: at minimum, indexes on `(scan_id, category)`, `(scan_id, package_manager)`, and a text index (or FTS5 virtual table) covering name/description/paths to back the search requirement (FR-10) without full scans of a several-thousand-row table on every keystroke.

Migrations are forward-only, versioned, and applied at startup; a corrupted or unreadable database is treated as "no cache" (triggering a rebuild path), never a fatal startup error — consistent with NFR-5 in the product spec.

---

## 10. Tauri IPC Commands (design-level list)

Matches product spec FR-9 through FR-13:

- `get_provider_status` — availability + last-known health per provider
- `start_inventory_scan` — begins an async scan, returns a scan id, emits progress events
- `get_scan_status` — poll-based fallback / initial state alongside events
- `cancel_inventory_scan`
- `get_inventory_summary`
- `list_software_items` — paginated, filterable, sortable, searchable
- `get_software_item` — full detail by id, including lazy-loaded expensive fields
- `list_duplicate_groups`
- `list_provider_warnings`
- `export_inventory` — format + mode (full vs. reinstallation manifest) as typed request
- `rebuild_cache`

All commands take/return strongly typed structs mirrored into TypeScript; no arbitrary string-keyed payloads. Commands validate all frontend-supplied input (e.g., pagination bounds, filter enum values) before touching the inventory/persistence layers — the command layer is a trust boundary, not a pass-through.

Scan progress is pushed via Tauri events (not polling) so the frontend can show live per-provider progress without hammering `get_scan_status`.

---

## 11. Frontend State Strategy

- Server-ish state (inventory data, scan status, provider status) is owned by a thin service layer (`services/`) wrapping typed `invoke` calls, with React Query–style caching/refetch semantics (exact library choice deferred to `docs/DECISIONS.md` at implementation time) — not raw component-local `useEffect` fetching scattered across features.
- UI-only state (selected filters, view mode, selected item for detail panel) lives in local component state or a lightweight store, kept separate from server state so a scan refresh doesn't clobber user-chosen filters.
- Scan progress events update a small global "scan state" store that the dashboard and any in-progress indicators subscribe to.

---

## 12. Background Inventory Refresh

V1 scans are **user-triggered only** (per product spec non-goals — no scheduled/background scanning in V1). The architecture still supports a scan running in the background of the app (i.e., the user can keep browsing cached data while a new scan runs), because `start_inventory_scan` is async and progress is event-driven rather than blocking the UI thread. True OS-level scheduled/background scanning is explicitly future roadmap.

---

## 13. Cancellation and Command Timeouts

- Every provider `scan` call is wrapped in an orchestration-level timeout by the inventory service.
- `ScanContext` carries a cancellation token; providers are expected to check it between expensive steps (e.g., between per-stage APT queries) and abort early, returning whatever partial `ProviderInventory` they have rather than leaving no result.
- `cancel_inventory_scan` triggers the same cancellation token from the frontend.
- All individual external command executions inside `process/` carry their own timeout as a second layer of defense, so a single hung subprocess cannot outlive the provider-level or scan-level timeout even if a provider's own cancellation-checking logic has a gap.

---

## 14. Test Strategy (design-level)

- **Domain layer:** serialization round-trip tests, validation helper tests — no mocking needed, pure data.
- **Providers:** fixture-based tests only (captured real-world command output saved under `src-tauri/tests/fixtures/<provider>/`), never dependent on the host machine's actual installed packages; include malformed/truncated/empty fixture cases per provider.
- **Classification:** table-driven tests mapping (synthetic evidence) → (expected category, confidence, reasons).
- **Inventory service:** integration tests using `MockInventoryProvider` configured for success/failure/timeout/slow-partial scenarios, verifying partial-success behavior end-to-end.
- **Persistence:** repository tests against a real (temp-file or in-memory) SQLite instance, including migration tests and corrupted-database recovery behavior.
- **Commands:** tests validating input-rejection paths (bad pagination, invalid filter values) in addition to happy-path passthrough.
- **Frontend:** Vitest component tests for filter/search/export behavior and for each defined loading/empty/error/partial-result UI state (per product spec — no state may be silently skipped in tests).

---

## 15. Open Design Decisions

Deferred to `docs/DECISIONS.md`, to be recorded as they are actually decided (not pre-answered here):

- Exact frontend server-state library choice (React Query vs. a lighter custom hook layer).
- Whether `classification_results` is a separate table or columns embedded in `software_items`.
- FTS5 vs. plain indexed `LIKE` search for V1 (given expected item counts are in the low thousands, this may not need FTS5 yet).
- Exact confidence-level enum values and scoring rubric (owned by `docs/CLASSIFICATION.md`, Prompt 04C).
