# Contributing to Kunger

## Development rules

These rules are binding for all contributions, human or AI-assisted:

1. Never push directly to `main`.
2. Create a feature branch per milestone (`feat/...`, `fix/...`, `chore/...`, `test/...`,
   `ci/...`).
3. Make small, reviewable commits.
4. Do not perform destructive package operations. Kunger is read-only in v1.
5. Do not install, update, or remove system packages from within Kunger unless a future version
   explicitly and separately decides to add that capability.
6. Never invoke `sudo` or any privilege-escalation mechanism.
7. Never execute discovered software (binaries, AppImages, or `.desktop` `Exec` fields).
8. Never parse human-formatted command output when a stable machine-readable format exists.
9. Treat all external command output as untrusted input.
10. Add timeouts to all external command execution.
11. Avoid shell interpolation — pass command arguments separately (argument vectors, not shell
    strings).
12. Gracefully handle missing package managers and tools.
13. Every provider must return partial results without crashing the whole inventory scan.

## Workflow

For each task:

1. Inspect relevant existing files before writing code.
2. Explain the proposed implementation.
3. Identify affected files.
4. Implement the smallest complete change.
5. Add or update tests.
6. Run formatting, linting, type checking, and tests.
7. Review the diff.
8. Update documentation (`README.md`, `docs/ARCHITECTURE.md`, `docs/DECISIONS.md`,
   `docs/SECURITY.md`, `docs/CLASSIFICATION.md`, `TASKS.md` as relevant).

## Required checks before opening a PR

```bash
npm run lint
npm run format:check
npm run typecheck
npm test
npm run build

cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features
cargo test
```

Do not hide warnings, incomplete features, or test failures.

## Architecture changes

Before changing architecture, explain the change and record it as a new entry in
`docs/DECISIONS.md`.
