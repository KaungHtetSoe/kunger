---
name: add-provider
description: Scaffold a new Kunger inventory provider (e.g. Snap, pip, pipx, npm, Cargo — see docs/PRODUCT_SPEC.md's post-V1 roadmap). Use when asked to add support for scanning a new package manager or software source.
---

# Add a new Kunger inventory provider

Every existing provider (`apt`, `flatpak`, `desktop`, `fonts`, `appimage`, `manual` — all under
`src-tauri/src/providers/`) follows the same shape. Follow it exactly; deviating from an
established pattern here is a red flag, not a judgment call, unless there's a documented reason
(check `docs/DECISIONS.md` for precedent first).

## Non-negotiable constraints (docs/SECURITY.md §1)

- No privilege escalation — never `sudo` or any privileged mechanism.
- No package operations — read-only, always.
- Never execute a discovered binary, script, or AppImage, and never trust/execute a `.desktop`
  file's `Exec=` field — metadata only.
- No shell interpolation — every external command goes through `process::ProcessRunner`/
  `CommandSpec` as an argument vector, never a shell string.
- Treat all external output as untrusted: size-bounded, validated before parsing.
- Bounded scanning only — a documented, fixed set of directories/commands, never an unrestricted
  walk. Non-recursive unless there's a specific need (see `desktop/mod.rs`'s
  `MAX_RECURSION_DEPTH` for the one exception, and note its symlink-safety comment).
- Never one-subprocess-per-package — batch queries the way `apt`'s `dpkg-query` does.
- Distinguish "not installed" (`is_available() == false`) from "failed" (`ProviderStatus::Failed`)
  — a missing package manager must never fail the whole scan.

## Steps

1. **Scope the provider first.** What exact commands/directories will it read? Add them to
   `docs/SECURITY.md` §2's scanned-locations table _before_ writing code — that table is the
   authoritative scope boundary per its own header ("any new location a future provider needs
   must be added here explicitly, with justification").

2. **Add the domain enum variant** if this is a new package manager, in
   `src-tauri/src/domain/enums.rs`'s `PackageManager` enum (mirror the `camelCase` serde rename
   already on every other variant — see ADR-0009).

3. **Implement the provider** at `src-tauri/src/providers/<name>/mod.rs`, implementing
   `InventoryProvider` (`providers/mod.rs`): `id()`, `metadata()`, `is_available()`, `scan()`.
   Use `ProcessRunner::run()`/`run_allow_any_exit()` for any subprocess calls. Prefer a
   machine-readable output format over parsing human text wherever the tool supports it
   (NFR-2) — check for a `--format=json` or similar flag before falling back to text parsing.

4. **Parse defensively.** Bound file/output reads (`Read::take`, matching `MAX_DESKTOP_FILE_BYTES`
   patterns elsewhere). If output could contain ambiguous punctuation, consider ASCII Unit/Record
   Separator delimiters the way the `apt` and `fonts` providers do for `dpkg-query`/`fc-list`.

5. **Wire classification signals** if the new provider surfaces evidence the classification engine
   should use — add to `classification/rules.rs` and document in `docs/CLASSIFICATION.md`,
   following its existing rule-table format (priority-ordered, confidence corroboration).

6. **Add fixtures and tests** under `src-tauri/tests/fixtures/<name>/`, captured from real command
   output where possible (not hand-invented). Write: a parser test per fixture, an
   `is_available()` true/false test, a cancellation test (`ScanContext::is_cancelled()` checked
   before any real work), and an unavailable-vs-failed distinction test. Match the density of the
   existing providers — `docs/TESTING.md` records roughly a dozen tests per provider as the bar.

7. **Register the provider** in `InventoryService::with_default_providers()`
   (`src-tauri/src/inventory/mod.rs`) — note the registration-order comment above that function:
   providers that reuse another provider's `id` scheme for ownership-known entries (like desktop/
   fonts reusing `apt:{package}`) must come after the provider they depend on.

8. **Update the frontend** if the new package manager should be user-filterable: the
   `PACKAGE_MANAGERS` list in `src/features/inventory/FilterBar.tsx`, and
   `PACKAGE_MANAGER_LABELS` in `src/utils/labels.ts`. If it's reproducible by name (like apt/
   flatpak), also add it to `REPRODUCIBLE_MANAGERS` and `install_hint()` in
   `src-tauri/src/commands/export.rs` so the reinstallation manifest picks it up.

9. **Update docs**: `TASKS.md` (new milestone entry), `docs/ARCHITECTURE.md` if the provider list
   is enumerated there, and an ADR in `docs/DECISIONS.md` if any non-obvious design choice was
   made (e.g. why a particular directory is/isn't scanned — see ADR pattern in the manual
   provider's scope-limitation section of `docs/SECURITY.md` §4 for the style).

10. **Ship it** — follow the `ship` skill for branch/check-suite/verification/PR.
