# Kunger — Testing and Quality Gate

Status as of M5.1. This is a snapshot, not a living contract — re-run the commands below rather
than trusting these numbers as time passes.

## Current state

| Suite                  | Count                  | Command                                      |
| ---------------------- | ---------------------- | -------------------------------------------- |
| Rust (`src-tauri`)     | 269 tests, 0 failed    | `cd src-tauri && cargo test`                 |
| Frontend (Vitest)      | 82 tests / 26 files    | `npm test`                                   |
| Frontend coverage (v8) | ~88% stmts / 90% lines | `npm run test:coverage`                      |
| Rust lint              | clean                  | `cd src-tauri && cargo clippy --all-targets` |
| Rust format            | clean                  | `cd src-tauri && cargo fmt --check`          |
| Frontend lint          | clean, 0 warnings      | `npm run lint`                               |
| Frontend typecheck     | clean                  | `npm run typecheck`                          |
| Production build       | clean                  | `npm run build`                              |

Rust coverage tooling (`cargo-llvm-cov` / `cargo-tarpaulin`) is not wired up — neither is
available in this sandbox, and a numeric line-coverage % would be less informative than what's
below anyway, since the Rust side leans on fixture-driven parser tests rather than incidental
line hits. If CI (M5.4) later runs on a machine with network access, adding `cargo-llvm-cov` there
is a reasonable follow-up.

## What's covered, by layer

- **Domain** (`domain/`): construction, validation, and serialization round-trips for every type,
  including the externally-tagged `ProviderError` and every enum's `camelCase` wire format
  (ADR-0009).
- **Classification** (`classification/`): 34 dedicated tests, one (or more) per rule in
  `docs/CLASSIFICATION.md`'s table, plus confidence-corroboration and conflicting-signal cases.
  This is the highest test-to-code ratio in the backend on purpose — misclassification is a
  product-trust problem, not just a bug.
- **Providers** (`providers/*`): every provider (apt, flatpak, desktop, fonts, appimage, manual)
  has parser tests against real captured command-output fixtures (`tests/fixtures/`), plus
  `is_available`/unavailable-vs-failed distinction tests and cancellation tests. Real system
  calls are never exercised in tests — everything runs against fixture text or a fake `dpkg -S`
  process, since this dev sandbox is macOS and can't run the real Debian tooling anyway (see
  `docs/DECISIONS.md` for how each provider's tests are structured around that constraint).
- **Inventory** (`inventory/`): merge-by-id (including the display-name-generic-vs-specific
  heuristic) and cross-manager duplicate detection, 20 tests between the two modules.
- **Persistence** (`persistence/`): repository round-trips, corruption recovery (rename-aside +
  fresh DB), schema tests against a real temp SQLite file (never mocked — ADR-0014).
- **Commands** (`commands/`): every `_impl` function tested directly (no Tauri runtime needed —
  ADR-0015), including the M4.6 reinstallation-manifest export (JSON/YAML/CSV × full vs.
  manifest, plus the CSV formula-injection guard added in M5.2 -- 12 cases in `export.rs`).
- **Frontend**: every page now has at least a loading/error/empty/populated-state test where that
  state exists (Dashboard, Duplicates, Providers, Export, Software details, Inventory browser),
  per the "no UI state silently skipped" rule in `docs/ARCHITECTURE.md`. Interaction-heavy pieces
  (search debounce, filter toggles, pagination, scan start/cancel, cache rebuild confirm/cancel)
  are covered with `@testing-library/user-event`, not just render snapshots.

## Gap found and fixed during this milestone

`DuplicatesPage` was still the placeholder built in M4.5a — it always rendered a static "No scan
has been run yet" `EmptyState` regardless of scan state, never calling `list_duplicate_groups`.
Writing the loading/error/populated-state tests this milestone requires surfaced that the page
had no data-fetching at all. Fixed by wiring it to `useDuplicateGroups` (already built for the
software-details duplicate banner in M4.5d) with real loading/error/empty/populated states,
covered by four new tests in `src/pages/DuplicatesPage.test.tsx`.

## Known, accepted gaps

- `InventoryBrowser`'s grouped-view rendering and pagination-change interaction paths are covered
  indirectly through `GroupedList`'s and `PaginationControls`' own unit tests rather than
  end-to-end through `InventoryBrowser` itself — acceptable since those child components own the
  actual rendering logic and are tested in isolation with the exact prop shapes
  `InventoryBrowser` passes them.
- `commands::events::TauriScanEventEmitter` (the real `AppHandle::emit` wrapper) has no direct
  unit test — it's a two-line pass-through with nothing to assert beyond "it calls emit", and
  every command test already exercises the `ScanEventEmitter` trait via `NoopScanEventEmitter`.
- No end-to-end test drives the real Tauri app. This dev sandbox is macOS, so the target Linux
  package managers aren't present to scan against, and scripting clicks into the WebView's
  interactive elements via the OS accessibility API is unreliable (static text reads work
  consistently; button/toggle clicks frequently miss or land on the wrong element). Each
  milestone's real-app verification instead combines a clean `cargo`/`npm` build, clean
  `tauri dev` logs, a macOS accessibility-tree text read to confirm real data renders correctly,
  and this test suite as the proof that interactions behave correctly.
