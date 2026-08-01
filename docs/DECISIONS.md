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

## ADR-0008 (pending) — Frontend server-state library

**Date:** 2026-08-01
**Status:** Deferred

**Context:** Need a strategy for caching/refetching IPC-derived data (inventory, scan status, provider status) in the React frontend without ad hoc `useEffect` fetching per feature.

**Decision:** Not yet made. Candidates: a React Query–style library, or a minimal custom hook layer over `invoke`. To be decided during Prompt 09A (frontend shell) once real IPC shapes exist.

**Consequences:** N/A until decided.

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
