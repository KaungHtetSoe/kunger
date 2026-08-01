# Kunger — Security Model

Status: Draft v0.1 (living document — updated as providers are implemented and as
`docs/SECURITY_REVIEW.md` findings are addressed)

## 1. Core guarantees

Kunger makes the following guarantees for v1 and treats them as binding constraints on every
future change, not just the initial release:

- **No privilege escalation.** Kunger never requires root and never invokes `sudo` or any other
  privilege-escalation mechanism, automatically or otherwise.
- **No package operations.** Kunger never installs, updates, removes, or otherwise mutates
  installed software.
- **No execution of discovered software.** Kunger never executes a discovered binary, script, or
  AppImage, and never trusts or executes a `.desktop` file's `Exec` field — these are read only
  as metadata.
- **No shell interpolation.** All external process execution passes arguments as discrete
  argument vectors. Kunger never builds a command by string-concatenating untrusted values into
  a shell command line.
- **Untrusted output.** All external command output (package manager output, file contents,
  filenames, `.desktop`/font/AppImage metadata) is treated as untrusted input: size-bounded,
  validated before parsing, and never assumed well-formed.
- **Bounded scanning.** Filesystem scanning is restricted to a documented, bounded set of
  directories at bounded depth. Kunger never performs an unrestricted walk of the filesystem or
  of arbitrary user-supplied paths.
- **Timeouts everywhere.** All external command execution is time-bounded, at both the
  per-command and per-provider level (see `docs/ARCHITECTURE.md` §13).

## 2. Scanned locations (bounded scope)

Kunger only reads from the following locations. This list is the authoritative scope boundary —
any new location a future provider needs must be added here explicitly, with justification.

| Source          | Locations                                                                                                                |
| --------------- | ------------------------------------------------------------------------------------------------------------------------ |
| APT/dpkg        | dpkg/apt databases via `dpkg-query`, `apt-cache`, `apt-mark` (no direct filesystem parsing of `/var/lib/dpkg` internals) |
| Desktop entries | `/usr/share/applications`, `/usr/local/share/applications`, `~/.local/share/applications`                                |
| Fonts           | `/usr/share/fonts`, `/usr/local/share/fonts`, `~/.local/share/fonts`, `~/.fonts`                                         |
| Flatpak         | via `flatpak` CLI machine-readable output only                                                                           |
| AppImage        | `~/Applications`, `~/.local/bin`, `~/Downloads`, `/opt`, `/usr/local/bin`                                                |
| Manual software | `/opt`, `/usr/local/bin`, `/usr/local/lib`, `~/.local/bin`, `~/.local/share/applications`                                |

Kunger does **not** scan user documents, media, browser data, or any directory outside this list.
Directory scans are non-recursive or depth-limited as specified per provider — no provider walks
an unbounded directory tree.

## 3. Privacy considerations

- Installation paths frequently contain the user's home directory and therefore their username.
  Exports must make this clear to the user rather than silently including it; see
  `docs/PRODUCT_SPEC.md` FR-11 and the export security review item in
  `docs/SECURITY_REVIEW.md`.
- Kunger does not transmit inventory data anywhere by default — it is a local, offline tool.
  Any future network features (e.g., checking for updates against a remote index) must be
  explicitly opt-in and documented before being added.
- Logs must not contain full command output or file contents by default at anything above debug
  level, to avoid incidentally capturing sensitive path or filename data in persisted logs.

## 4. Manual software provider — specific scope limitation

The manual-software provider (`providers/manual/`) only inspects `/opt`, `/usr/local/bin`,
`/usr/local/lib`, and `~/.local/bin`, checks whether a known provider (dpkg) already owns a
given path before classifying it as manual, and only reports metadata about
executables/libraries/directories — never file contents. `/opt` entries are captured as whole
top-level directories and never descended into. It does not inspect arbitrary user directories,
and it is not a general-purpose file scanner.

It deliberately does **not** scan `~/.local/share/applications`, even though earlier planning
material listed it in this provider's scope: the desktop-entry provider already fully owns that
directory (parsing, ownership resolution, classification), so a second, cruder pass over the
same `.desktop` files here would only produce confusing duplicate records for the same file, not
new information. It also skips `.AppImage`-extension files in the bin directories it scans,
leaving those to the AppImage provider.

## 5. Known risk areas to review before release

Tracked in full in `docs/SECURITY_REVIEW.md` (Prompt 11):

- Command injection via provider subprocess invocation
- Path traversal / symlink attacks during bounded directory scans
- Malicious or malformed `.desktop` files
- Malformed font metadata
- Malicious AppImage filenames
- SQLite injection in persistence-layer queries
- Tauri IPC input validation
- Unbounded output from external commands (denial-of-service via memory exhaustion)
- Privacy leakage in exports (usernames/home paths)
