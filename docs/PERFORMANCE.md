# Kunger — Performance Review (M5.3)

Status: Completed 2026-08-02, pre-v0.1.0 release. This is the `docs/PERFORMANCE.md` that
`docs/PRODUCT_SPEC.md` §13 deferred exact numbers to. It checks each required _behavior_ from
that section against the current implementation and, where possible, backs the claim with a real
measurement rather than an estimate. All numbers below are from `cargo test -- --nocapture`
against a debug build in this sandbox (a MacBook-class machine, not the target Debian/Ubuntu
hardware) — treat them as evidence of _shape_ (linear vs. quadratic, milliseconds vs. seconds),
not as guaranteed absolute numbers on every user's machine.

## Required behaviors, checked against the current implementation

| Requirement (spec §13)                                       | Status                      | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ------------------------------------------------------------ | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Full scan completes well under a minute for a typical system | **Meets** (by construction) | `InventoryService::scan()` runs every provider concurrently via `futures::future::join_all` (`inventory/mod.rs`), each individually time-bounded (default 30s/provider). Total wall-clock time is bounded by the _slowest_ provider, not the sum of all of them — proven empirically by the existing test `a_slow_provider_times_out_without_blocking_the_others`, which configures one provider with a 500ms delay and a 50ms timeout alongside a fast provider, and asserts the whole scan still completes in well under 400ms (i.e., the fast provider's result isn't held hostage by the slow one). |
| UI remains responsive during a scan                          | **Meets** (by construction) | The scan runs in a `tokio::spawn`'d background task (`commands/scan.rs`); the frontend polls `get_scan_status` (1s interval while running) and listens for `scan-*` events rather than blocking on an in-flight `invoke()`. `ScanControls`/`ScanStatusIndicator` render a live elapsed-time counter and a working Cancel button throughout.                                                                                                                                                                                                                                                             |
| No one-subprocess-per-package pattern                        | **Meets**                   | The APT provider (the one with by far the most packages, "a few thousand" per spec) makes a fixed, small number of subprocess calls per scan — one batched `dpkg-query` covering every installed package, one `apt-mark showmanual`, one ownership-resolution `dpkg -S` call reused by other providers — never one call per package. Confirmed by reading `providers/apt/mod.rs`: exactly 4 call sites to `runner.run`/`run_allow_any_exit`, none inside a per-package loop.                                                                                                                            |
| Inventory browser stays smooth at thousands of items         | **Meets**                   | Table view paginates at 50 items/page; grouped view caps rendering at the first 500 matching items regardless of total count (with a "showing first 500 of N" notice) — both keep the DOM small regardless of inventory size, satisfying the spec's "pagination and/or virtualization" requirement without needing a virtualization library.                                                                                                                                                                                                                                                            |

## Measured: read-path cost at 5,000 synthetic items

"A few thousand APT packages" (spec §13) is a realistic upper bound for a heavily-used desktop
system, so 5,000 synthetic items is the scale these measurements target. All three tests below
are permanent, `--nocapture`-printed regression tests (generously bounded so they only fail on a
real regression, not sandbox noise) — re-run them any time with:

```
cd src-tauri && cargo test performance:: -- --nocapture
cargo test latest_items_read_cost -- --nocapture
```

| Operation                                                        | Measured   | What it covers                                                                                                                                                                                                                               |
| ---------------------------------------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_inventory_summary_impl`                                     | **55µs**   | Reads the pre-computed `InventorySummary` saved at scan time — doesn't touch the items table at all, so this is flat regardless of item count. The Dashboard's stat tiles are effectively free.                                              |
| `SqliteScanRepository::save_scan` (write, file-backed)           | **~273ms** | One-time cost when a scan finishes: one transaction inserting the summary, all items, duplicate groups, and provider results. Paid once per scan, not per UI interaction — not on the interactive-latency budget.                            |
| `SqliteScanRepository::latest_items` (read, file-backed)         | **~67ms**  | SQL `SELECT` + per-row JSON deserialize of every item. This is the actual bottleneck underneath `list_software_items_impl`, `get_software_item_impl`, and `export_inventory_impl` — all three call `latest_items()` and then work in memory. |
| `list_software_items_impl` (page 1, no filters)                  | **~77ms**  | `latest_items()` (~67ms) + in-memory filter/sort/paginate (~10ms). Confirms the in-memory work itself is cheap — the SQLite read dominates.                                                                                                  |
| `list_software_items_impl` (worst-case: search matching nothing) | **~81ms**  | Same as above; a full linear scan with a substring check on every item adds only ~4ms over the no-filter case.                                                                                                                               |

### What this means in practice

Every `list_software_items` call — including one fired by a debounced search keystroke, a
pagination click, or a sort-column click — re-reads and re-deserializes the _entire_ latest scan
from SQLite (`repository.latest_items()`), even though it only needs to display 50 items. At
5,000 items this costs ~70-80ms per call. That's comfortably inside "feels responsive" (most UX
guidance puts the "instant" threshold around 100ms and "responsive" around 300ms), and the
250ms search debounce added in M4.5e already prevents this from firing on every keystroke. It is
not a blocker for v1.

It is, however, the one place in the read path where cost scales with total inventory size
rather than with what's actually rendered — the natural next step if a future scan size (tens of
thousands of items, or slower target hardware) makes 70-80ms actually noticeable would be an
in-memory cache of the latest scan's items in `AppState`, invalidated on `save_scan`/
`rebuild_cache`, so repeated `list_software_items`/`get_software_item` calls against the same
scan don't re-hit SQLite. `commands/inventory_commands.rs`'s own module doc already flags this
exact tradeoff ("expected volumes... make this simple and fast enough for v1 -- revisit... if
profiling ever shows it's needed" — ADR-0014); this review's numbers confirm that judgment was
right for v1 scale and gives a concrete number (not a guess) for the size at which to revisit it.
**Deliberately not implemented now** — the current numbers don't justify the added complexity and
cache-invalidation surface for a marginal gain at expected v1 scale.

## Frontend bundle size

Production build output (`npm run build`):

```
dist/assets/index-*.css   23.69 kB │ gzip:   5.44 kB
dist/assets/index-*.js   379.96 kB │ gzip: 117.85 kB
```

~118 KB gzipped JS is a small, fast-to-parse bundle for a desktop app served from local disk
(Tauri's `frontendDist`, no network fetch involved) — not a load-time concern.

## What wasn't measured (and why)

- **Real provider scan times against actual `apt`/`flatpak`/`fc-list` output** — this dev sandbox
  is macOS, so none of the target Linux package managers are present to scan against. The
  concurrency and no-per-package-subprocess properties above are verified structurally
  (code + the existing timeout/concurrency test) rather than against real multi-thousand-package
  `dpkg-query` output. Revisit with real numbers once this can run on actual Debian/Ubuntu
  hardware (a natural fit for M5.4's CI, if that runs on Linux).
- **Classification-engine throughput** — the classifier is a pure function over one item's
  evidence at a time with a fixed-size rule table (`docs/CLASSIFICATION.md`); it runs inline
  during each provider's scan, not as a separate bulk pass, so there's no separate "classify N
  items" cost to isolate.
- **Frontend render-time profiling (React DevTools Profiler, actual frame timings)** — not
  available in this sandbox (see `docs/TESTING.md`'s note on WebView automation limitations). The
  pagination/grouped-view caps are a structural guarantee against unbounded DOM growth regardless
  of measured frame times, which is what the spec requirement actually asks for.

## Overall verdict

All four required performance _behaviors_ from the product spec are met, three of them by
architectural construction (concurrency, non-blocking UI, batched subprocess calls) and one
(smooth browsing at scale) by pagination/capping. No performance issue found rises to "fix before
release" — the one real inefficiency identified (repeated full-inventory SQLite reads) is
small in absolute terms at expected v1 scale (~70-80ms) and left as a documented, numbers-backed
future optimization rather than implemented preemptively. Proceeding to M5.4 (CI/CD and
packaging).
