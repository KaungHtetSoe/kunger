---
name: release
description: Cut a new Kunger release — version bump, tag, push, watch the release build, verify the AppImage/.deb artifacts, and publish. Use when the user asks to release, tag, or ship a new version.
---

# Cut a Kunger release

Pushing a `v*` tag triggers `.github/workflows/release.yml`, which builds the AppImage and `.deb`
on Ubuntu via `tauri-apps/tauri-action` and attaches them to a **draft** GitHub Release. Tags
aren't branches, so `main`'s branch protection doesn't block this step — but publishing the draft
is a real public action and needs explicit user confirmation, every time.

## 1. Decide the version

Ask the user which version, unless it's obvious from context (e.g. they already bumped it, or
this is clearly the next patch/minor per semver). Do not guess a version number silently.

## 2. Bump and sync the version (if it changed)

Two files must always match:

```bash
grep -n '"version"' package.json src-tauri/tauri.conf.json
```

If bumping, update both in the same commit, then run this through the `ship` skill (branch, full
check suite, PR, wait for merge) like any other change — **do not tag an unmerged or unreviewed
version bump.**

## 3. Sync local `main`

```bash
git checkout main && git fetch origin && git merge --ff-only origin/main
```

Confirm the tip is the commit you actually want to release (check `git log --oneline -5` against
what the user expects — e.g. did the version-bump PR from step 2 get merged first?).

## 4. Tag and push

```bash
git tag -a vX.Y.Z -m "Kunger vX.Y.Z

<one paragraph: what's in this release, pointing at RELEASE_NOTES.md>"
git push origin vX.Y.Z
```

## 5. Watch the release build

```bash
gh run list --repo <owner>/kunger --limit 3   # find the Release run
gh run watch --repo <owner>/kunger <run-id> --interval 20
```

This takes longer than CI (full Tauri bundling, ~5 minutes). If it's still running after the
watch command backgrounds itself, schedule a wakeup rather than polling tightly — see this
project's established pattern of a long fallback (240-300s) rather than short repeated checks.

If it fails, the failure is almost always a missing Linux system dependency in the workflow's
"Install Tauri Linux build dependencies" step (this has happened twice already — `libdbus-1-dev`
and `libgtk-3-dev` were both missing at different points). Read the actual `pkg-config`/`*-sys`
error, add the named package to **both** `.github/workflows/ci.yml` and `release.yml` plus
`README.md`'s prerequisites list, and ship that fix (via the `ship` skill) before retrying.

## 6. Verify the draft release

```bash
gh release view vX.Y.Z --repo <owner>/kunger --json isDraft,url,assets
```

Confirm both `Kunger_X.Y.Z_amd64.AppImage` and `Kunger_X.Y.Z_amd64.deb` are present and
`isDraft: true`. The `html_url` pointing at an `untagged-...` slug while still a draft is normal
GitHub behavior, not a bug — it resolves to the real `vX.Y.Z` URL once published.

## 7. Ask before publishing

Publishing makes the release publicly visible and notifies watchers. Always confirm with the user
first (report the draft URL and asset list, ask "ready to publish?") rather than publishing
automatically as part of the release flow — this has been a distinct, separately-confirmed step
every time so far.

## 8. Publish

Only after explicit confirmation:

```bash
gh release edit vX.Y.Z --repo <owner>/kunger --draft=false
```

Report the final release URL and the install commands (`wget` + `dpkg -i` for the `.deb`,
`wget` + `chmod +x` for the AppImage — see `RELEASE_NOTES.md` for the exact form).
