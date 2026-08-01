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

## Status

Early development (pre-0.1). Follow progress in [`TASKS.md`](TASKS.md).

## Technology

- [Tauri 2](https://tauri.app/) + Rust backend
- React + TypeScript + Tailwind CSS frontend
- SQLite for local caching
- Vitest (frontend) and Rust unit/integration tests (backend)

## Development

Prerequisites: Node.js 20+, Rust (stable, via `rustup`), and the
[Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your platform.

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

## Documentation

- [`docs/PRODUCT_SPEC.md`](docs/PRODUCT_SPEC.md) — what Kunger does and does not do
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — system design
- [`docs/DECISIONS.md`](docs/DECISIONS.md) — architecture decision log
- [`docs/CLASSIFICATION.md`](docs/CLASSIFICATION.md) — how software is categorized
- [`docs/SECURITY.md`](docs/SECURITY.md) — security model and constraints
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — development workflow
- [`SECURITY.md`](SECURITY.md) — how to report a vulnerability

## License

Not yet decided.
