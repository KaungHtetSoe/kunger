# Kunger — Tasks

This file tracks milestone-level progress. Update it after every completed milestone (see operating rules).

Legend: `[ ]` not started · `[~]` in progress · `[x]` done

---

## Phase 0 — Requirements

- [x] **M0.1** — Product specification written (`docs/PRODUCT_SPEC.md`)

## Phase 1 — Foundation

- [x] **M1.1** — Architecture design (`docs/ARCHITECTURE.md`, `docs/DECISIONS.md`)
- [x] **M1.2** — Repository setup (Tauri 2 + React/TS/Tailwind + Rust toolchain, lint/format/test scaffolding, minimal app shell)
- [x] **M1.3** — Core domain models (`SoftwareItem` and related enums, serialization tests)
- [x] **M1.4** — Provider interface (`InventoryProvider` trait, mock provider)
- [x] **M1.5** — Classification engine skeleton + `docs/CLASSIFICATION.md`

## Phase 2 — Debian/Ubuntu MVP

- [x] **M2.1** — APT/dpkg provider (staged scan, fixtures, no per-package subprocess spam)
- [x] **M2.2** — Classification rules for APT-sourced items (wired into the APT provider directly via `Evidence`/`classify()`)
- [x] **M2.3** — Inventory browser (table, filters, sort, search) — basic version — superseded by the fuller **M4.5c**/**M4.5e** once the frontend shell existed; never built separately
- [x] **M2.4** — Software details view — superseded by the fuller **M4.5d**; never built separately

## Phase 3 — Desktop Inventory

- [x] **M3.1** — Desktop application provider (`.desktop` parsing, ownership association)
- [x] **M3.2** — Font provider (Fontconfig, family grouping, ownership association)
- [x] **M3.3** — Flatpak provider (apps/runtimes/extensions, user vs. system scope)
- [x] **M3.4** — AppImage provider (bounded directories, no execution)
- [x] **M3.5** — Manual software provider (bounded directories, ownership-check-first)

## Phase 4 — Intelligence

- [x] **M4.1** — Unified inventory service (combine providers, timeouts, partial-success reporting)
- [x] **M4.2** — Duplicate detection across managers (completed alongside M4.1 — `inventory::duplicates`)
- [x] **M4.3** — SQLite persistence + scan history / diffing
- [x] **M4.4** — Tauri IPC command layer + typed frontend service
- [x] **M4.5a** — Frontend application shell (sidebar, top bar, routing, notifications, scan/provider status indicators)
- [x] **M4.5b** — Dashboard (scan summary stats, Scan System button, live progress)
- [x] **M4.5c** — Inventory browser (table, category/manager/scope filters, sort, pagination)
- [x] **M4.5d** — Software details view
- [x] **M4.5e** — Search and filters (full wiring of the global search box)
- [x] **M4.6** — Export (JSON/YAML/CSV, full inventory + reinstallation manifest)

## Phase 5 — Release

- [x] **M5.1** — Testing and quality gate (`test/inventory-quality-suite`)
- [x] **M5.2** — Security review (`docs/SECURITY_REVIEW.md`)
- [x] **M5.3** — Performance review (`docs/PERFORMANCE.md`)
- [x] **M5.4** — CI/CD and packaging (GitHub Actions, AppImage + .deb)
- [x] **M5.5** — Final release review (`RELEASE_CHECKLIST.md`, `KNOWN_ISSUES.md`, v0.1.0 release notes)

---

## Notes

- Project name: **Kunger**. (Earlier planning material referred to this product as "PackageLens" — that name is retired; all docs/code should use "Kunger.")
- First release targets Debian and Ubuntu only, read-only inventory, no privileged operations.
- v0.1.0 is feature-complete for V1 as of M5.5 — see `RELEASE_CHECKLIST.md` for the
  acceptance-criteria-by-acceptance-criteria verification and `KNOWN_ISSUES.md` for what to know
  before relying on it.
