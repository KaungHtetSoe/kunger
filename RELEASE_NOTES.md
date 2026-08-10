# Kunger v0.2.0

## What's new in this release

**CLI TUI for headless/VPS environments** — Kunger now works over SSH without X11 forwarding:

- Interactive TUI (Terminal User Interface) using Ratatui + Crossterm
- 6-dimensional filtering with keyboard-driven modal panel:
  - Category, Package Manager, Installation Scope, Installation Reason, Classification Confidence, Update Availability
- Real-time search with character-by-character filtering
- Paginated table browsing with keyboard navigation
- 50/50 split detail view for selected items
- Scan management with progress tracking (F5 to trigger)
- Full keyboard operability — no mouse required
- Complete documentation with 4 CLI screenshots in README
- 338 tests passing (321 lib + 17 integration, including new filter panel workflows)

---

# Kunger v0.1.0

First release. Kunger is a read-only software inventory, ownership, and dependency explorer for
Debian and Ubuntu — it tells you what's installed on your system, where it came from, and how
confident it is about that classification. It never installs, updates, or removes anything, and
it never needs root.

## What's in this release

**Inventory, across every supported source:**

- APT/dpkg packages, including whether each was installed manually or pulled in automatically as
  a dependency
- Flatpak apps, runtimes, and extensions (system and per-user)
- Desktop applications from standard `.desktop` file locations
- Fonts registered via Fontconfig
- AppImages in common locations (`~/Applications`, `~/.local/bin`, `~/Downloads`, `/opt`,
  `/usr/local/bin`) — detected, never executed
- Manually-installed software in `/opt`, `/usr/local/bin`, `/usr/local/lib`, `~/.local/bin`

Every scan degrades gracefully: if Flatpak isn't installed, or any single provider fails or times
out, the rest of the scan still completes and reports what it found.

**Understanding what you have:**

- Every item is classified into a category (application, library, font, runtime, development
  package, and more) with a confidence level and the specific reasons behind it — never a bare,
  unexplained label
- Package-manager ownership shown wherever Kunger can determine it
- Likely duplicate installations flagged across package managers (e.g., an app installed via both
  APT and Flatpak) — flagged only, never merged or resolved automatically

**Finding and exporting:**

- Full-text search and multi-dimension filtering (package manager, scope, install reason, update
  availability, classification confidence) across the whole inventory, with a paginated table and
  a grouped view
- Export the full technical inventory as JSON, YAML, or CSV
- Export a reinstallation manifest instead, which separates software Kunger can point a package
  manager at by name from software you'd need to reinstall by hand — clearly labeled as which is
  which, not left for you to guess

## Installing

Download the `.AppImage` or `.deb` from this release's assets.

```bash
# AppImage
chmod +x Kunger_0.1.0_amd64.AppImage
./Kunger_0.1.0_amd64.AppImage

# .deb
sudo dpkg -i kunger_0.1.0_amd64.deb
```

Requires Debian or Ubuntu (or a close derivative). No other runtime dependencies — WebKitGTK and
the other Tauri runtime libraries are what a modern desktop already has installed, or what the
`.deb`'s dependency list pulls in.

## Before you rely on this

This release was built and tested primarily in a non-Linux development sandbox — see
[`KNOWN_ISSUES.md`](KNOWN_ISSUES.md) for the full list of caveats, most importantly that the
provider parsers, while thoroughly tested against captured real command output, have not yet been
run against a live Debian/Ubuntu system's actual `apt`/`flatpak` state end to end. If something
looks wrong compared to `dpkg -l` or `flatpak list`, please open an issue.

Also see `KNOWN_ISSUES.md` for: the one accepted `npm audit` advisory (unreachable in Kunger's
usage), the lack of a native "Save As" dialog for exports, and why `Snap` isn't inventoried at
all yet.

## License

Not yet decided — see [`README.md`](README.md).

## Full details

- [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md) — the acceptance-criteria-by-acceptance-criteria
  verification behind this release
- [`docs/SECURITY_REVIEW.md`](docs/SECURITY_REVIEW.md), [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md),
  [`docs/TESTING.md`](docs/TESTING.md) — the three pre-release review passes
- [`docs/DECISIONS.md`](docs/DECISIONS.md) — every non-obvious engineering decision, with why
