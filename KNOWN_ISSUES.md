# Kunger v0.1.0 — Known Issues

Honest limitations as of the v0.1.0 release, compiled during M5.5. None of these block release
(see `RELEASE_CHECKLIST.md`), but a user or future contributor should know about all of them.

## Development environment caveat (the big one)

**Kunger was built and tested entirely on macOS, not Debian/Ubuntu.** It's a Linux-only tool by
design, but the sandbox this project was developed in has no Linux system to run it against. The
practical consequences:

- Every provider's parser is tested against real, captured command output (`dpkg-query`,
  `apt-mark`, `fc-list`, `flatpak list`, real `.desktop` files) held as fixtures under
  `src-tauri/tests/fixtures/`, not against a live system. The parsing logic is real and
  thoroughly tested; it has never been exercised against today's actual `dpkg`/`flatpak` output
  on a running Debian or Ubuntu box.
- The one real end-to-end verification available in this environment came from the macOS
  `manual` provider path (it scans `~/.local/bin`, `/opt`, etc., which exist on any Unix), which
  is how every milestone's "real app" screenshots-that-aren't-screenshots (see below) show actual
  data (38 items from this machine's own local binaries) flowing through the whole stack —
  scan → persistence → IPC → UI. That proves the pipeline works end to end; it does not prove
  the APT/Flatpak/fonts/desktop-entry providers behave correctly against real Linux output beyond
  what their fixture tests cover.
- **Before trusting this in production, run it on a real Debian or Ubuntu system and compare its
  APT/Flatpak counts against `dpkg -l`/`flatpak list` directly.** This is the single most
  important verification step this release has not had.

## UI automation gap

Screen recording/screenshot capture doesn't work in this sandbox, and scripting clicks into the
Tauri WebView via the macOS accessibility API is unreliable (static text reads work consistently;
interactive-element clicks frequently miss). Every milestone's "real app" verification combined a
clean build, clean `tauri dev` logs, an accessibility-tree text read confirming real data
renders, and the Vitest suite as the actual proof of click/interaction correctness (`userEvent`
in jsdom, not a real WebView). This is a reasonable substitute but not the same as manual QA in a
real desktop session — see `docs/TESTING.md` for the full rationale.

## Performance: read path scales with total inventory size

`list_software_items`, `get_software_item`, and `export_inventory` all re-read and
re-JSON-deserialize the _entire_ latest scan from SQLite on every call, then filter/sort/export
in memory. Measured at ~70-80ms for 5,000 synthetic items — comfortably responsive, not a
release blocker, but the cost scales with total inventory size rather than with what's actually
displayed. See `docs/PERFORMANCE.md` for the numbers and the (deliberately not yet built)
in-memory caching approach that would fix it if a future scan size makes it noticeable.

## Accepted dependency advisory

`npm audit` reports one high-severity advisory: `react-router-dom`/`react-router`
(GHSA-qwww-vcr4-c8h2, an RSC-mode CSRF bypass). Reviewed and accepted in ADR-0008 — Kunger has no
server and never uses React Router's RSC/data mode, so the vulnerable code path is unreachable.
No fix is available above the vulnerable range without downgrading. Runs informationally
(non-blocking) in CI rather than gating the build.

## No Snap support at all (not even detection)

`PackageManager::Snap` exists as an enum variant (used by classification, export, and filtering
code for forward compatibility) but there is no Snap provider — Snap packages are invisible to
Kunger entirely. This isn't a regression: Snap was never in the V1 acceptance criteria
(`docs/PRODUCT_SPEC.md` §14), only mentioned in the post-V1 roadmap ("Full Snap provider (beyond
detection)"). Flagged here because the enum variant's presence could otherwise read as "partial
Snap support exists" when it doesn't.

## No native "Save As" dialog for exports

Exported files download via the WebView's default download behavior (a `Blob` + `<a download>`
element — `src/utils/download.ts`), not a native Tauri file-save dialog. This means exports land
wherever the WebView's default download location is (typically `~/Downloads`) rather than letting
the user pick a location up front. Deliberate tradeoff to avoid adding a new Tauri
plugin/capability surface for a single feature — see ADR-0016. Revisit if user feedback wants a
location picker.

## CI/CD pipelines are unverified end-to-end

`.github/workflows/ci.yml` and `release.yml` (M5.4) parse as valid YAML and were reasoned through
against Tauri's documented Ubuntu build dependencies, but this repository has no GitHub remote
configured, so neither workflow has actually run on GitHub's infrastructure. **The first push to
a real GitHub repository should be treated as this pipeline's real first test**, not as a
formality — particularly the AppImage/.deb bundling step in `release.yml`, which needs Linux
packaging tools (`appimagetool`, `dpkg-deb`) that don't exist on macOS and so could not be
exercised locally at all. See ADR-0018.

## No fuzz/dynamic testing of parsers

The `.desktop`, font, and APT-field parsers are fixture-tested against real captured output and
are bounded-read (never trust unbounded input), but no fuzzer has been run against them. Low risk
given the bounded-read/never-panics-on-malformed-input properties are unit-tested per parser, but
not the same guarantee a fuzz corpus would provide.
