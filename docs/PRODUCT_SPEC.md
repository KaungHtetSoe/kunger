# Kunger — Product Specification

Status: Draft v0.1
Scope: Version 1 (V1) — Debian and Ubuntu, read-only inventory
Non-scope for this document: implementation, code, architecture diagrams (see `docs/ARCHITECTURE.md`, to be written separately)

---

## 1. Product Summary

Kunger is a Linux desktop application that inventories all installed software on a Debian or Ubuntu system and organizes it by category and by installation source (package manager / ownership). Kunger is **read-only**: it never installs, updates, or removes software, and never requires elevated privileges to produce its inventory.

Kunger exists to answer questions that no single package manager can answer on its own, because modern Linux systems mix APT, Flatpak, Snap, AppImage, language-level package managers, and manually placed binaries — and no built-in tool reconciles them into one picture.

---

## 2. Questions Kunger Must Answer

For every discovered software item, Kunger must be able to answer:

1. What software is installed?
2. What category does it belong to?
3. Which package manager (or lack thereof) owns it?
4. What version is installed?
5. Where is it installed (paths)?
6. Was it installed manually or pulled in automatically (as a dependency)?
7. Which other packages depend on it (reverse dependencies)?
8. What does it depend on?
9. Is an update available?
10. Is it duplicated through another package manager (e.g., Firefox via APT and Flatpak)?
11. How confident is Kunger in its classification, and why?

---

## 3. User Personas

### 3.1 The System Auditor

A power user, sysadmin, or security-conscious individual who wants a full, honest picture of what's on a machine before making trust or cleanup decisions. Cares most about completeness, ownership clarity, and not missing anything installed outside APT.

### 3.2 The Developer Workstation Owner

A developer with a machine accumulated over years — APT packages, Flatpak apps, pipx tools, cargo-installed binaries, AppImages dropped in `~/Applications`, and forgotten `/opt` installs. Wants to understand what's actually there and what's safe to ignore, without Kunger touching anything.

### 3.3 The Migrator / Rebuilder

Someone preparing to reinstall the OS or move to a new machine. Wants an exportable manifest of "what I have" so they can reconstruct their environment, understanding that some categories (manual installs, AppImages without metadata) can only be flagged for manual review, not automatically reproduced.

### 3.4 The Curious Learner

A user newer to Linux who wants to understand what's installed on their system and how Linux package management actually works, using categorized, explained output as a learning aid.

---

## 4. Primary Use Cases

- View a categorized, filterable inventory of everything installed on the system.
- Determine which package manager owns a given piece of software.
- Identify software installed manually (by the user) vs. automatically (as a dependency).
- Identify duplicate installations of the same logical software across managers.
- Inspect dependency and reverse-dependency relationships for a package.
- Check whether updates are available for installed packages (where the underlying manager exposes this safely).
- Export the inventory for backup, documentation, audit, or migration-planning purposes.
- Understand _why_ Kunger classified something a certain way (transparent classification reasons).
- Continue to get a useful, non-crashing result even when some inventory sources (e.g., Flatpak) are unavailable.

---

## 5. Non-Goals (V1 and beyond, unless explicitly revisited)

- Kunger is **not** a package manager, app store, or software center. It does not install, upgrade, remove, or repair software.
- Kunger does not execute discovered binaries or AppImages during inventory or for any reason.
- Kunger does not require or invoke `sudo` or any privilege escalation.
- Kunger does not automatically resolve or "fix" duplicate installations.
- Kunger does not scan the entire filesystem; it scans a bounded, documented set of known locations.
- Kunger does not inspect user documents, media, or unrelated personal directories.
- Kunger does not guarantee a fully automated reinstall of manually installed or AppImage software — it flags these for manual review rather than fabricating instructions it cannot verify.
- Kunger does not support distributions other than Debian and Ubuntu in V1 (RPM-based, Arch, etc. are future roadmap).
- Kunger does not implement Snap as a fully supported provider in V1 (detection only, best-effort).

---

## 6. Required Categories

- Application
- Command-line tool
- Library
- Font
- Runtime
- Development package
- Theme
- Icon pack
- Firmware
- Driver
- Kernel component
- System service
- Desktop component
- Documentation
- Language pack
- Miscellaneous
- Unclassified

Every item must be assigned exactly one primary category, may have zero or more secondary categories, and must carry a classification confidence level plus human-readable classification reasons. Items that cannot be confidently classified must be labeled **Unclassified** rather than forced into a best-guess bucket — see `docs/CLASSIFICATION.md` (future document) for the full rule set.

---

## 7. Required Package / Installation Sources

- APT/dpkg
- Flatpak
- Snap — detected only; not a fully implemented provider in V1
- AppImage
- pip
- pipx
- npm
- Cargo
- Manual (unowned files in known locations)

### 7.1 Fully implemented in V1

- APT/dpkg
- Flatpak
- Desktop applications (`.desktop` files)
- Fontconfig-managed fonts
- AppImage detection
- Manual software detection in known, bounded directories

### 7.2 Detected but not fully implemented in V1

- Snap (presence/version detection only, best-effort, optional)
- pip / pipx / npm / Cargo (explicitly out of scope for full inventory in V1; see roadmap)

---

## 8. Functional Requirements

**FR-1.** Kunger must enumerate all APT/dpkg-installed packages with name, version, architecture, section, description, installed size, and manual/automatic installation state.

**FR-2.** Kunger must enumerate Flatpak applications, runtimes, and extensions (when Flatpak is present), distinguishing user-scope vs. system-scope installs.

**FR-3.** Kunger must enumerate desktop applications from standard `.desktop` file locations and associate each with an owning package where ownership can be determined.

**FR-4.** Kunger must enumerate fonts registered via Fontconfig and known font directories, grouping files into logical font families while retaining per-file detail.

**FR-5.** Kunger must detect AppImage files in a bounded set of known directories without executing them.

**FR-6.** Kunger must detect manually installed software (executables, libraries, desktop entries) in a bounded set of known directories, and must first check known-provider ownership before labeling something "manual."

**FR-7.** Kunger must classify every discovered item into exactly one primary category (plus optional secondary categories), with a confidence level and human-readable reasons.

**FR-8.** Kunger must detect likely duplicate installations of the same logical software across different package managers or install methods, and present these as duplicate groups with reasons — never auto-resolving them.

**FR-9.** Kunger must display, for each item, the information described in Section 2 to the extent the underlying source makes it available; unavailable data must be clearly marked as unavailable, not fabricated or silently omitted.

**FR-10.** Kunger must let the user search and filter the inventory by name, category, package manager, scope, manual/automatic status, update availability, and classification confidence.

**FR-11.** Kunger must let the user export the inventory in JSON, YAML, and CSV, in both a full technical inventory mode and a reinstallation-manifest mode (the latter clearly separating what can vs. cannot be automatically reproduced).

**FR-12.** Kunger must run a scan on demand (triggered by the user), show live progress per provider, and allow cancellation.

**FR-13.** Kunger must cache the most recent scan and be able to show a summary of changes (new / removed / version-changed items) relative to the previous scan.

---

## 9. Non-Functional Requirements

**NFR-1 (Resilience).** Failure or unavailability of any single provider (e.g., Flatpak not installed) must not prevent results from other providers from being displayed. Every scan must be able to complete in a "partial success" state.

**NFR-2 (Determinism of parsing).** Wherever a stable, machine-readable output format exists for an external tool, Kunger must use it in preference to parsing human-formatted text.

**NFR-3 (Bounded execution).** All external command execution must be time-bounded, and all filesystem scanning must be bounded to documented directories and depths — no unbounded recursive scans of the filesystem or home directory.

**NFR-4 (Transparency).** Every classification decision and every "unavailable" data field must be explainable to the user; nothing should appear falsely authoritative.

**NFR-5 (Portability of data).** The local cache/database is a convenience layer, not the source of truth — it must be safely rebuildable from a fresh scan at any time.

**NFR-6 (No side effects).** Running Kunger, including a full scan and export, must never modify system state (no package operations, no font cache rebuilds, no execution of discovered software).

---

## 10. Security Requirements

- Kunger must never require root and must never invoke `sudo` or any other privilege-escalation mechanism, automatically or otherwise.
- Kunger must never execute discovered binaries, scripts, or AppImages.
- Kunger must never trust or execute `Exec` fields from `.desktop` files — these are read as metadata only.
- All external process execution must avoid shell interpolation; arguments must be passed as discrete argument vectors, never built via string concatenation into a shell command.
- All external command output must be treated as untrusted input: bounded in size, validated before parsing, and safe against malformed or adversarially crafted content (e.g., a hostile `.desktop` file or font metadata field).
- Filesystem scanning must be restricted to a documented, bounded set of directories at bounded depth — never an unrestricted walk of the filesystem or arbitrary user-specified paths.
- Symlink handling during filesystem scanning must avoid following links in a way that could escape the intended scan roots or cause unbounded traversal.
- Exports must not unintentionally leak sensitive data (e.g., full home-directory paths containing the username) without the user understanding that the export contains this information.
- The local database must be stored under the user's own application data directory with no elevated or shared-location storage.
- Full security requirements and threat modeling are the subject of a dedicated `docs/SECURITY.md` and a later `docs/SECURITY_REVIEW.md` — this spec establishes the non-negotiable constraints above as binding for every subsequent milestone.

---

## 11. Error-Handling Requirements

- Every provider must catch and structure its own errors; a provider failure must produce a `ProviderError` result, not a process crash or an unhandled exception surfaced to the UI as a blank screen.
- Warnings (partial, non-fatal issues — e.g., one malformed `.desktop` file among thousands) must be tracked separately from fatal provider errors and must not block the rest of that provider's results.
- Missing external tools (e.g., `flatpak` not installed) must be detected proactively and reported as "provider unavailable," not surfaced as a generic failure.
- Timeouts on external command execution must produce a clear, typed error distinguishable from "command not found" or "command failed."
- The UI must never silently hide a failed or partial provider — failures and warnings must be visible to the user, with detail available on demand.
- The application must never crash the whole scan due to a single malformed item; malformed items must be skipped with a recorded warning.

---

## 12. Accessibility Requirements

- All interactive UI must be operable via keyboard alone (navigation, filtering, search, detail views, export).
- Color must never be the sole means of conveying status (e.g., provider warnings, update availability, confidence level) — pair color with text/iconography.
- Text and interactive elements must meet WCAG 2.1 AA contrast minimums in both light and dark presentation, if both are offered.
- Tables and lists must be screen-reader navigable with meaningful labels (not just visual grouping).
- Loading, empty, and error states must be announced in a way assistive technology can pick up (not purely visual spinners with no accessible text).

---

## 13. Performance Requirements

- A full scan on a typical desktop system (a few thousand APT packages, tens of Flatpak apps, hundreds of fonts) should complete within a target of well under one minute for the fast/basic inventory stage, with detailed/expensive data (e.g., full dependency graphs, file lists) loaded lazily rather than blocking the initial result.
- The UI must remain responsive during a scan; scanning must not block the main interface.
- Kunger must avoid one-subprocess-per-package patterns in favor of batched queries wherever the underlying tool supports it (this is a hard requirement, not just an optimization — see `docs/DECISIONS.md` once architecture work begins).
- Rendering the inventory table/browser must remain smooth for systems with several thousand items (pagination and/or virtualization required, not optional).
- Exact benchmark targets and measured results are deferred to a future `docs/PERFORMANCE.md`; this section establishes the performance _behaviors_ required, not final numbers.

---

## 14. Version 1 (V1) Acceptance Criteria

Kunger v0.1.0 is considered feature-complete for V1 when it can:

1. Run on Debian or Ubuntu without requiring root.
2. Inventory all APT/dpkg packages, including manual vs. automatic installation state.
3. Inventory Flatpak applications and runtimes when Flatpak is present, and degrade gracefully when it is not.
4. Parse and display desktop applications from standard `.desktop` file locations.
5. Inventory fonts registered via Fontconfig / known font directories.
6. Detect AppImages in known, bounded directories without executing them.
7. Detect a defined set of manually installed software in known, bounded directories.
8. Classify all discovered items into the required category set, with confidence and reasons.
9. Show package-manager ownership for every item where determinable.
10. Support search and multi-dimension filtering across the inventory.
11. Detect and present likely cross-manager duplicates, without auto-resolving them.
12. Export the inventory as JSON, YAML, and CSV, including a reinstallation-manifest mode.
13. Handle missing providers (e.g., no Flatpak installed) without failing the scan.
14. Handle partial provider failures without failing the entire scan.
15. Never install, update, or remove software, and never invoke `sudo`.
16. Pass all automated checks (formatting, linting, type checking, and the full test suite) with no hidden or suppressed failures.

---

## 15. Future Roadmap (post-V1, not committed)

- Full Snap provider (beyond detection).
- Language-ecosystem inventories: pip, pipx, npm (global), Cargo (installed binaries via `cargo install`).
- Additional distributions: Fedora/RPM family, Arch family.
- Scheduled/background scans (V1 is manual/on-demand only).
- Historical trend views across many scans (beyond the single previous-scan diff in V1).
- Optional integration with vulnerability/CVE data sources for installed package versions (read-only, informational only — still no remediation actions).
- Richer AppImage metadata extraction (e.g., safely reading embedded desktop/icon metadata without execution) if a safe method is validated.
- Plugin-style extensibility for additional inventory providers.

---

## 16. Open Questions / Risks (to revisit during architecture)

- Exact strategy for safely reading AppImage-embedded metadata (e.g., via SquashFS offset parsing) without ever executing the file — needs a safe-parsing spike before commitment.
- Precise duplicate-detection heuristics (name similarity, `.desktop` StartupWMClass matching, Flatpak app-id vs. binary name mapping) need concrete rules, to be defined during the classification/duplicate-detection design work.
- Confidence-scoring model (how "confidence" is computed and displayed) needs a concrete rubric — deferred to `docs/CLASSIFICATION.md`.
