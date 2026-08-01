# Kunger — Classification

Status: v0.1 — matches the rule table implemented in `src-tauri/src/classification/`
(`rules.rs` for the priority-ordered table, `mod.rs` for the evaluation pipeline). If this
document and the code ever disagree, the code is authoritative; update this file in the same
change that changes `rules.rs`.

See `docs/ARCHITECTURE.md` §5–6 for how classification fits into the overall provider →
classification → inventory-service pipeline.

## How classification works

Providers observe raw, source-specific signals about a package (a Debian section, whether it
owns `.desktop` launcher, header files, shared libraries, etc.) and populate an `Evidence`
struct — a provider-agnostic snapshot of what was directly observed. The classification engine
never runs commands or reads files itself; it only reasons over `Evidence` that's already been
collected.

`classify(&Evidence) -> ClassificationResult` evaluates a fixed, priority-ordered list of rules
top to bottom:

- The **first rule that matches** sets the primary category and its confidence level.
- A **later rule matching the same category** is treated as corroborating evidence: its reason
  is appended, and confidence is raised one level (capped at `Certain`).
- A **later rule matching a different category** is recorded as a secondary category (each
  category appears at most once in `secondary_categories`), but never overrides the primary
  category chosen by a higher-priority rule.
- If **no rule matches at all**, the result is `SoftwareCategory::Unclassified` with
  `ClassificationConfidence::Unknown` and no reasons — Kunger never forces a low-evidence item
  into a guessed category (see "Ambiguity handling" below).

This is a deterministic, pure function: same `Evidence` in, same `ClassificationResult` out,
with no I/O and no hidden state — which is what makes it exhaustively table-driven-testable.

## Rule priority

Rules are ordered strongest/most-authoritative signal first. The rationale for this ordering:

1. **Declared package metadata** (Debian section) comes first — it's a direct assertion by the
   package itself/its distribution, not an inference from side effects.
2. **Direct filesystem/ownership evidence** (owns firmware files, owns a systemd unit, owns
   `.so` files, provides a desktop launcher, owns headers) comes next — these are things the
   provider directly observed the package doing, not guesses.
3. **Package-name heuristics** (`-dev`, `-doc` suffixes, `driver`/`dkms` substrings) come last —
   naming conventions are common but not guaranteed, and are explicitly the weakest signal per
   the project's own rule: _"Do not classify solely from package-name prefixes when stronger
   metadata is available."_ Every name-based rule sits below every metadata- and
   ownership-based rule for exactly this reason.

The full table, in evaluation order (see `src-tauri/src/classification/rules.rs` for the
literal source of truth):

| #   | Category           | Confidence | Trigger                                                     |
| --- | ------------------ | ---------- | ----------------------------------------------------------- |
| 1   | Font               | High       | Debian section is `fonts`                                   |
| 2   | Library            | High       | Debian section is `libs`                                    |
| 3   | DevelopmentPackage | High       | Debian section is `libdevel`                                |
| 4   | DevelopmentPackage | High       | Debian section is `devel`                                   |
| 5   | Documentation      | High       | Debian section is `doc`                                     |
| 6   | KernelComponent    | High       | Debian section is `kernel`                                  |
| 7   | LanguagePack       | Medium     | Debian section is `localization`                            |
| 8   | Runtime            | Medium     | Debian section is `interpreters`                            |
| 9   | Miscellaneous      | Low        | Debian section is `misc` or `metapackages`                  |
| 10  | Firmware           | High       | package owns files under `/lib/firmware`                    |
| 11  | KernelComponent    | High       | package owns files under `/lib/modules`                     |
| 12  | SystemService      | High       | package installs a systemd unit file                        |
| 13  | Theme              | High       | package owns files under `/usr/share/themes`                |
| 14  | IconPack           | Medium     | package owns an icon theme under `/usr/share/icons`         |
| 15  | DesktopComponent   | Medium     | has a desktop launcher categorized `Settings`/`System`      |
| 16  | Application        | High       | package provides a desktop launcher                         |
| 17  | DevelopmentPackage | High       | package owns header files under `/usr/include`              |
| 18  | DevelopmentPackage | Medium     | package owns pkg-config (`.pc`) files                       |
| 19  | Library            | High       | package owns shared library (`.so`) files                   |
| 20  | DevelopmentPackage | Medium     | package name ends with `-dev`                               |
| 21  | Documentation      | Medium     | package name ends with `-doc`                               |
| 22  | LanguagePack       | Medium     | package name contains `language-pack`                       |
| 23  | Documentation      | Medium     | package appears to contain only documentation files         |
| 24  | CommandLineTool    | Medium     | package installs executables without a desktop launcher     |
| 25  | Driver             | Low        | package name suggests a hardware driver (`-dkms`, `driver`) |

If nothing above matches: **Unclassified**, confidence **Unknown**.

Note rule 16 (Application) sits below rule 15 (DesktopComponent): both require a desktop
launcher, but 15 additionally requires the launcher's `Categories=` to include `Settings` or
`System` — the more specific rule is checked first so a settings applet doesn't get
mis-classified as a generic Application before the more precise rule gets a chance. Rule 24
(CommandLineTool) requires _no_ desktop launcher, so a package with both executables and a
launcher classifies as Application (rule 16), not CommandLineTool — a GUI app that happens to
also drop a CLI binary is still primarily an application.

## Confidence scoring

Confidence is one of five ordered levels: `Unknown` < `Low` < `Medium` < `High` < `Certain`.

- **Unknown** — no rule matched (`Unclassified`).
- **Low** — a single weak, indirect, or purely heuristic signal (e.g. a name substring, or a
  vague Debian section like `misc`).
- **Medium** — a single moderately reliable signal (e.g. a `.desktop` `Categories=` hint, a
  `-dev`/`-doc` name suffix, owning `.pc` files).
- **High** — a single strong, direct signal (an authoritative Debian section, direct ownership
  of category-defining files like `.so`, headers, firmware, or a desktop launcher).
- **Certain** — reached only through **corroboration**: two or more independent rules agreeing
  on the same category. A single rule alone never produces `Certain`, no matter how strong —
  confidence starts at whatever level the first matching rule specifies, and each _additional_
  same-category match raises it one level, capped at `Certain`.

This means confidence answers two different questions at once: how strong was the _best_
individual signal, and did multiple _independent_ signals agree. A package with Debian section
`devel` and its own header files under `/usr/include` gets `Certain` — either signal alone would
be `High`, but together they corroborate each other.

## Secondary categories and ambiguity handling

Real packages sometimes legitimately span categories — a font package might ship a small font
manager GUI (so it's both `Font` and, weakly, `Application`); a library package might also match
a driver-sounding name heuristic. Kunger records this as `secondary_categories` rather than
picking one and discarding the rest, or refusing to classify at all:

- The **primary category** is always the one from the _highest-priority_ matching rule — never
  overridden by weaker later signals, even if several weaker rules agree with each other and
  disagree with the primary. Priority order, not signal count, decides the primary.
- Every other matched category becomes a **secondary category**, deduplicated, in the order
  first encountered.
- If literally nothing matches, the item is `Unclassified` — this is treated as a legitimate,
  first-class outcome (not an error), and is exactly what the UI should show rather than a
  fabricated best guess. See `docs/PRODUCT_SPEC.md` §6.

## Known limitations

- The rule table currently reasons only over the `Evidence` shape defined in
  `src-tauri/src/classification/mod.rs`. As real providers are implemented (M2.1+), evidence
  collection for signals like `owns_pkgconfig_files`, `owns_gtk_theme_files`, or
  `documentation_only` needs to be wired up per provider — until a provider actually populates a
  given `Evidence` field, that signal is simply never true for its items, which is a safe
  default (fewer classifications, not wrong ones) but means classification quality depends on
  provider completeness, not just rule quality.
- `Runtime` and `LanguagePack` currently only trigger from Debian section values
  (`interpreters`, `localization`) and one name substring (`language-pack`) — this is
  intentionally conservative given the project has not yet validated these categories against
  real package data; expect this table to grow once M2.1 (APT provider) ships against a real
  system and known gaps surface.
- `Driver` is the weakest-evidenced category in the table (name-heuristic only, `Low`
  confidence) because there is no reliable structural signal available purely from Debian
  package metadata to distinguish "hardware driver" from "any other kernel-adjacent package" —
  this is expected to remain a low-confidence, low-priority category rather than be
  strengthened artificially.
- Rules are evaluated independently per item; cross-item reasoning (e.g. "this package is
  probably a duplicate of that Flatpak app, so trust its category less") is out of scope for the
  classification layer and belongs to duplicate detection in the inventory service (M4.1) — see
  `docs/ARCHITECTURE.md` §7.
