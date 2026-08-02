# Kunger v0.1.0 — Release Checklist

Checked 2026-08-02 as the final step of M5.5. Each item below is checked against the actual
current state of the repository (code read, tests run), not against what a milestone was
originally scoped to do — see the linked evidence for each.

## V1 Acceptance Criteria (`docs/PRODUCT_SPEC.md` §14)

| #   | Criterion                                            | Met? | Evidence                                                                                                                                                         |
| --- | ---------------------------------------------------- | ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Runs on Debian/Ubuntu without root                   | Yes  | No `sudo`/privilege-escalation call anywhere (`docs/SECURITY_REVIEW.md` risk area 1). Not run on real Debian/Ubuntu hardware in this session — see Known Issues. |
| 2   | APT/dpkg inventory incl. manual vs. automatic        | Yes  | `providers/apt/` (M2.1), `apt-mark showmanual` drives `InstallationReason`.                                                                                      |
| 3   | Flatpak inventory, graceful degradation              | Yes  | `providers/flatpak/` (M3.3); `is_available()` distinguishes "not installed" from "failed" throughout.                                                            |
| 4   | Desktop application parsing                          | Yes  | `providers/desktop/` (M3.1), bounded recursion, non-symlink-following.                                                                                           |
| 5   | Font inventory via Fontconfig                        | Yes  | `providers/fonts/` (M3.2), `fc-list` machine-readable output.                                                                                                    |
| 6   | AppImage detection, never executed                   | Yes  | `providers/appimage/` (M3.4); confirmed no `Command::new` on discovered paths anywhere (`docs/SECURITY_REVIEW.md` risk area 5).                                  |
| 7   | Manual software detection, bounded                   | Yes  | `providers/manual/` (M3.5); ownership-check-first against dpkg, non-recursive (`docs/SECURITY_REVIEW.md` risk area 2).                                           |
| 8   | Full category set with confidence + reasons          | Yes  | `classification/` (M1.5), 34 dedicated rule-table tests (`docs/CLASSIFICATION.md`).                                                                              |
| 9   | Package-manager ownership shown where determinable   | Yes  | `providers/dpkg_ownership.rs`, shared across desktop/fonts/manual providers.                                                                                     |
| 10  | Search + multi-dimension filtering                   | Yes  | `InventoryBrowser`/`FilterBar` (M4.5c/e), debounced, category/manager/scope/reason/confidence/update filters.                                                    |
| 11  | Cross-manager duplicate detection, no auto-resolve   | Yes  | `inventory::duplicates` (M4.1/M4.2); `DuplicatesPage` only flags, never merges or removes (fixed from a dead placeholder in M5.1).                               |
| 12  | Export JSON/YAML/CSV incl. reinstallation manifest   | Yes  | `commands::export` (M4.4/M4.6), 12 dedicated tests incl. the M5.2 CSV-injection fix.                                                                             |
| 13  | Missing providers don't fail the scan                | Yes  | Every provider's `is_available()` short-circuits to `Unavailable`, not `Failed`; `InventoryService` continues regardless.                                        |
| 14  | Partial provider failures don't fail the whole scan  | Yes  | `a_slow_provider_times_out_without_blocking_the_others` and related tests (`docs/PERFORMANCE.md`).                                                               |
| 15  | Never installs/updates/removes, never `sudo`         | Yes  | `docs/SECURITY_REVIEW.md` risk area 1; `NFR-6` verified by code review, not just by omission.                                                                    |
| 16  | All automated checks pass, nothing hidden/suppressed | Yes  | See "Suppression audit" below.                                                                                                                                   |

## Suppression audit (criterion 16, verified directly)

Grepped the whole tree rather than trusting that nothing was suppressed:

- `#[ignore]` on any Rust test: **none**.
- `.only`/`.skip`/`it.todo`/`test.todo` in any Vitest test: **none**.
- `eslint-disable` anywhere in `src/`: **none**.
- `@ts-ignore`/`@ts-expect-error` anywhere in `src/`: **none**.
- `#[allow(clippy::...)]` outside the documented `cfg(test)` relaxation in `lib.rs`: **one** —
  `#[allow(clippy::too_many_arguments)]` on a manual-provider helper (a stylistic lint about
  parameter count, not a correctness suppression).
- `TODO`/`FIXME`/`XXX:` markers in any source file: **none**.

## Automated checks (run immediately before this checklist)

| Check                          | Result                                                                                                                                      |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo fmt --check`            | Clean                                                                                                                                       |
| `cargo clippy --all-targets`   | Clean (1 pre-existing, accepted warning on `lib.rs`'s top-level `.expect()` — see `docs/SECURITY_REVIEW.md`)                                |
| `cargo test`                   | 269/269 passing                                                                                                                             |
| `npm run typecheck`            | Clean                                                                                                                                       |
| `npm run lint`                 | Clean, 0 warnings                                                                                                                           |
| `npm run format:check`         | Clean                                                                                                                                       |
| `npm test`                     | 82/82 passing (26 files)                                                                                                                    |
| `npm run build`                | Clean production build                                                                                                                      |
| `npm run tauri dev` (real app) | Starts cleanly, Dashboard/Inventory/Export/Duplicates all render correctly against a live scan cache, confirmed via accessibility-tree read |

## Documentation completeness

- [x] `docs/PRODUCT_SPEC.md`, `docs/ARCHITECTURE.md`, `docs/DECISIONS.md` (18 ADRs), `docs/CLASSIFICATION.md`, `docs/SECURITY.md` — all present and current.
- [x] `docs/TESTING.md` — test suite state (M5.1).
- [x] `docs/SECURITY_REVIEW.md` — security review findings (M5.2).
- [x] `docs/PERFORMANCE.md` — performance review findings (M5.3).
- [x] `TASKS.md` — every milestone checked off, including M2.3/M2.4 reconciled as superseded by their fuller M4.5c/d/e equivalents rather than left ambiguously unchecked.
- [x] `README.md` — build, check, and packaging instructions current.
- [x] `KNOWN_ISSUES.md` — this milestone's other deliverable (see below).

## Release readiness: **READY**

All 16 V1 acceptance criteria are met with direct evidence, the full automated suite passes with
nothing suppressed, and every planned milestone through M5.5 is complete. Kunger v0.1.0 is ready
to tag, pending the caveats in `KNOWN_ISSUES.md` — none of which block a v0.1.0 release, but all
of which a user or maintainer should know about going in.
