# Kunger

**Kunger** is a Linux desktop application that inventories installed software on Debian and
Ubuntu systems and organizes it by category (applications, libraries, fonts, runtimes,
development packages, and more) and by installation source (APT/dpkg, Flatpak, AppImage, manual
installs, and others).

Kunger is **not** a package manager or app store. It is a **read-only** inventory, ownership,
dependency, and system-inspection tool. It never installs, updates, or removes software, and it
never requires root privileges.

See [`docs/PRODUCT_SPEC.md`](docs/PRODUCT_SPEC.md) for the full product specification and
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the technical design.

## Screenshots

### CLI TUI (Headless/VPS-Friendly)

The Kunger CLI provides a **text-based user interface** for inventorying software without a display server. Perfect for remote machines, VPS, or container environments.

#### Overview

<p align="center">
  <img src="screenshots/kunger-cli-overall.png" width="800" alt="Kunger CLI overview showing paginated table with software items, status bar, and keybinding hints">
</p>

#### Interactive Search

<p align="center">
  <img src="screenshots/kunger-cli-search.png" width="800" alt="Kunger CLI search mode with real-time filtering as you type">
</p>

#### Advanced Filtering

<p align="center">
  <img src="screenshots/kunger-cli-filter.png" width="800" alt="Kunger CLI interactive filter panel showing category, manager, scope, reason, confidence, and update availability dimensions">
</p>

#### Detail View

<p align="center">
  <img src="screenshots/kunger-cli-detail.png" width="800" alt="Kunger CLI detail view showing dependencies, version, classification confidence, and other metadata for a selected item">
</p>

**CLI Features:**

- Paginated table browsing (arrow keys)
- Real-time search filtering (Tab to focus)
- **6-dimensional filtering** with interactive modal panel (press `f`)
- Sort by name, category, manager, version
- 50/50 split detail view (Enter to open)
- Scan management with progress tracking (F5)
- Fully keyboard-operable — no mouse required
- Works over SSH without X11 forwarding

### Desktop GUI

All screenshots below are from a real scan on Debian (v0.1.0) — 2,094 items across APT and
manually-installed software, not sample data.

<p align="center">
  <img src="screenshots/kunger-dashboard.png" width="800" alt="Kunger dashboard with a real scan summary: 2,094 total items broken down by category, package manager distribution, and duplicate-group count">
</p>

<p align="center">
  <img src="screenshots/kunger-allsoftware.png" width="800" alt="All Software table view with manager/scope/install-reason filters and sortable columns">
</p>

<p align="center">
  <img src="screenshots/kunger-details.png" width="800" alt="Software details view for a driver package, showing classification confidence, the specific reasons behind it, and its dependency graph">
</p>

<p align="center">
  <img src="screenshots/kunger-duplicates.png" width="800" alt="Duplicates view flagging apps installed both natively (APT) and via Waydroid's Android environment, without merging or removing either">
</p>

<details>
<summary>More screenshots — category views (Applications, Libraries, Runtimes, System)</summary>

<p align="center">
  <img src="screenshots/kunger-applications.png" width="800" alt="Applications category view">
</p>
<p align="center">
  <img src="screenshots/kunger-libraries.png" width="800" alt="Libraries category view">
</p>
<p align="center">
  <img src="screenshots/kunger-runtimes.png" width="800" alt="Runtimes category view">
</p>
<p align="center">
  <img src="screenshots/kunger-system.png" width="800" alt="System category view, showing drivers and kernel components">
</p>

</details>

## Usage

### CLI (Headless)

```bash
./kunger-cli
```

The CLI starts with an empty inventory. Press `F5` to scan for installed software. Use arrow keys to navigate and explore.

**Main Keybindings:**

| Key                   | Action               |
| --------------------- | -------------------- |
| `Tab`                 | Focus search box     |
| `↑`/`↓`               | Navigate items       |
| `Page Up`/`Page Down` | Jump pages           |
| `Enter`               | Open detail view     |
| `f`                   | Open filter panel    |
| `s`                   | Toggle sort order    |
| `c`                   | Clear all filters    |
| `F5`                  | Start inventory scan |
| `q` / `Esc`           | Quit                 |

**Filter Panel** (press `f`):

| Key            | Action                                                                        |
| -------------- | ----------------------------------------------------------------------------- |
| `Left`/`Right` | Switch dimension (Category / Manager / Scope / Reason / Confidence / Updates) |
| `↑`/`↓`        | Navigate filter values                                                        |
| `Space`        | Toggle/cycle value                                                            |
| `c`            | Clear all filters                                                             |
| `Esc`          | Close panel                                                                   |

The status bar at the bottom shows all available keybindings in your current context.

### Desktop GUI

```bash
npm run tauri dev    # Development
npm run tauri build  # Production build
```

## Status

v0.1.0, feature-complete for V1 — see [`RELEASE_NOTES.md`](RELEASE_NOTES.md) for what's included,
[`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md) for the acceptance-criteria verification behind
that claim, and [`KNOWN_ISSUES.md`](KNOWN_ISSUES.md) for what to know before relying on it.
Milestone history in [`TASKS.md`](TASKS.md).

## Technology

- **Desktop**: [Tauri 2](https://tauri.app/) + Rust backend, React + TypeScript + Tailwind CSS
- **CLI/TUI**: [Ratatui](https://ratatui.rs/) + [Crossterm](https://docs.rs/crossterm/) (pure Rust, no display server)
- **Data**: SQLite for local caching, Rusqlite for querying
- **Testing**: Vitest (frontend), Rust unit/integration tests (backend + CLI)
- **Architecture**: Shared `kunger_lib` domain model used by both desktop and CLI

## Development

Prerequisites: Node.js 20+, Rust (stable, via `rustup`), and the Tauri system dependencies for
your platform (see [Tauri's prerequisites page](https://v2.tauri.app/start/prerequisites/) for
the authoritative, up-to-date list). On Debian/Ubuntu:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libdbus-1-dev \
  libgtk-3-dev \
  pkg-config
```

`libdbus-1-dev` and `libgtk-3-dev` are both easy to miss — Tauri's documented Debian/Ubuntu
prerequisites list doesn't call them out explicitly, expecting `apt` to pull them in as
dependencies of `libwebkit2gtk-4.1-dev` automatically. That dependency resolution doesn't always
happen cleanly on every Debian variant/mirror state, so they're listed explicitly here.
`tao` (Tauri's window-management crate) depends on `dbus` and `gtk` unconditionally on Linux, and
the build fails with `libdbus-sys`/`dbus-1` or `gdk-sys`/`gdk-3.0` `pkg-config` errors without
them. If you still hit a `pkg-config`/`*-sys` build error after installing everything above, the
error message names the exact missing package — install that one too and retry; there may be
further transitive GTK/WebKit dependencies (e.g. `libjavascriptcoregtk-4.1-dev`,
`libsoup-3.0-dev`) that a particular Debian release's package split doesn't pull in automatically.

```bash
npm install       # install frontend dependencies
npm run tauri dev # run the app in development mode
```

### Checks

```bash
npm run lint         # ESLint
npm run format:check # Prettier
npm run typecheck    # TypeScript
npm test              # Vitest
npm run build         # production frontend build

cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features
cargo test
```

CI (`.github/workflows/ci.yml`) runs all of the above on every push to `main` and every pull
request.

### Building packages

```bash
npm run tauri build -- --bundles appimage,deb
```

Produces an AppImage and a `.deb` under `src-tauri/target/release/bundle/`. Requires the Tauri
Linux build dependencies (see [prerequisites](https://v2.tauri.app/start/prerequisites/)) — this
only works on Linux, not macOS or Windows, since it links against `webkit2gtk`. Pushing a `v*` tag
(e.g. `v0.1.0`) runs `.github/workflows/release.yml`, which builds both bundles on Ubuntu and
attaches them to a draft GitHub Release.

## Documentation

- [`docs/PRODUCT_SPEC.md`](docs/PRODUCT_SPEC.md) — what Kunger does and does not do
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — system design
- [`docs/DECISIONS.md`](docs/DECISIONS.md) — architecture decision log
- [`docs/CLASSIFICATION.md`](docs/CLASSIFICATION.md) — how software is categorized
- [`docs/SECURITY.md`](docs/SECURITY.md) — security model and constraints
- [`docs/SECURITY_REVIEW.md`](docs/SECURITY_REVIEW.md) — pre-release security review findings
- [`docs/TESTING.md`](docs/TESTING.md) — test suite coverage and quality gate
- [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) — performance review and measurements
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — development workflow
- [`SECURITY.md`](SECURITY.md) — how to report a vulnerability

## License

Not yet decided.
