---
name: ship
description: Standard workflow for landing any Kunger change — branch, implement, full check suite, real-app verification, commit, PR. Use whenever making and landing a code or doc change in this repo.
---

# Ship a Kunger change

The standard loop used for every milestone and fix in this repo. `main` is branch-protected (no
direct pushes, even for the owner) and CI-gated, so this is the only path a change can take.

## 1. Branch

Never work directly on `main`. Sync it first, then branch:

```bash
git checkout main && git fetch origin && git merge --ff-only origin/main
git checkout -b <type>/<short-description>
```

Branch prefixes already in use: `feat/`, `fix/`, `docs/`, `test/`, `security/`, `perf/`, `ci/`,
`release/`, `chore/`. Pick the one matching the change.

## 2. Implement

Make the change. If it touches the Rust backend and adds new logic, write tests alongside it —
this codebase has no untested provider, command, or persistence logic (see `docs/TESTING.md`).
If it touches the frontend, cover every loading/error/empty/populated state the change exposes
(see `docs/ARCHITECTURE.md`'s "no UI state may be silently skipped" rule).

## 3. Run the full check suite

Both stacks, every time, even for small changes:

```bash
cd src-tauri
cargo fmt --check
cargo clippy --all-targets
cargo test

cd ..
npm run typecheck
npm run lint
npm run format:check
npm test
npm run build
```

Fix anything that fails. Never suppress a warning/lint/test to make this pass — see
`RELEASE_CHECKLIST.md`'s suppression audit for why (grep for `#[ignore]`, `eslint-disable`,
`@ts-ignore`, `.only`/`.skip`, undocumented `#[allow(clippy::...)]`, and `TODO`/`FIXME` markers
before considering a change done; there should be none).

## 4. Verify in the real app (for anything UI-facing or backend-command-facing)

This sandbox has no screen recording and WebView click automation is unreliable (static text
reads work; button/toggle clicks frequently miss). The established verification approach:

```bash
(npm run tauri dev > /tmp/kunger-dev.log 2>&1 &)
# wait for "Kunger starting up" in the log, then:
osascript -e '
tell application "System Events"
  tell process "kunger"
    set frontmost to true
    return entire contents of window 1
  end tell
end tell' > /tmp/kunger-ax.txt
grep -o 'static text [^,]*' /tmp/kunger-ax.txt | sed 's/ of .*//' | sort -u
```

Confirm the expected UI text/data renders and the dev log has no errors, then kill the dev
process. Treat the Vitest suite (`userEvent` in jsdom) as the actual proof of click/interaction
correctness, not the accessibility-tree read — that's only for confirming real data renders.

## 5. Commit

Detailed message: what changed, why, and how it was verified. Follow the style already in
`git log` — explain design decisions and tradeoffs, not just restate the diff. Never commit with
suppressed checks or an untested change.

## 6. Push and open a PR

```bash
git push -u origin <branch-name>
gh pr create --repo <owner>/kunger --title "..." --body "$(cat <<'EOF'
## Summary
- ...

## Test plan
- [x] full check suite (see above) — clean
- [ ] CI (fill in after checking)
EOF
)"
```

## 7. Check CI, then stop

```bash
gh pr checks <number> --repo <owner>/kunger
```

Report the PR link and CI status to the user. **Do not merge the PR or push to `main`** — the
user merges manually after their own review/testing. This is a standing preference, not a
one-time instruction (see the branch-protection setup and the user's explicit statement that they
want to review and merge PRs themselves).

## 8. After the user merges

Sync local `main`:

```bash
git checkout main && git fetch origin && git merge --ff-only origin/main
```
