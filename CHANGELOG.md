# Changelog

All notable changes to **Rusty Requester**. Format roughly follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versions
follow [Semantic Versioning](https://semver.org/).

The compatibility commitment kicks in at **1.0.0**: from there on,
your `data.json`, the install path / bundle ID, the CLI flags, and the
import / export formats are stable across the 1.x series. Pre-1.0
releases (everything below) shipped a lot of stuff fast and made
breaking-format changes only when guarded by `#[serde(default)]`, so
upgrades read old files cleanly.

## Unreleased

### Fixed
- **Response find layout.** The inline response-body search box now uses a
  fixed bounded width so the close button cannot overlap or clip at the toolbar
  edge.

## [0.27.13] — 2026-06-19

### Added
- **Response body search navigation.** Find-in-body now shows the active match
  position and total matches, supports Enter / Shift+Enter navigation, and
  highlights the active match more strongly.

## [0.27.12] — 2026-06-19

### Fixed
- **Window resize and zoom behavior.** The main app window is explicitly
  resizable, and the macOS custom top chrome now supports window dragging and
  double-click maximize/restore.

## [0.27.11] — 2026-06-19

### Added
- **File-backed collection folders.** Linked collection directories can now stay
  in sync with app edits automatically, writing reviewable `workspace.json`,
  `requests/*.rr`, and `environments/*.rrenv` files after normal saves. New
  `.rr` request files use readable dictionary-style blocks for metadata,
  method, params, headers, cookies, and auth, while older `.rr` files remain
  importable.

## [0.27.10] — 2026-06-19

### Fixed
- **Request and collection navigation polish.** Response body search no longer
  overlaps the toolbar edge, newly opened request tabs scroll into view, sidebar
  request rename fields align with the row, Collection Settings can rename the
  collection, and collection overview now has a close button plus a cleaner
  description field.

## [0.27.9] — 2026-06-19

### Fixed
- **Settings and response control alignment.** Settings text fields now use
  consistent framed padding, Response "Sending request" renders as a quiet
  inline progress state, and the Response body find box no longer squeezes the
  input when no match count is shown.

## [0.27.8] — 2026-06-18

### Fixed
- **Collection Settings and updater layout.** Collection Settings no longer
  lets sidebar selections, note cards, fields, or action buttons overlap, and
  the in-app updater now shows a bounded sanitized status instead of stretching
  around raw terminal progress output.

## [0.27.7] — 2026-06-18

### Fixed
- **Collection Settings form layout.** The modal now uses a fixed-size,
  top-aligned settings layout with vertical form groups and stable button widths
  so fields and actions cannot collapse into vertical text.

## [0.27.6] — 2026-06-18

### Fixed
- **Collection Settings layout density.** The modal now uses compact section
  rows and a framed detail pane instead of oversized card-like navigation and
  sparse controls that looked broken on wide windows.

## [0.27.5] — 2026-06-18

### Fixed
- **Installer update resolution.** `install.sh` now reads the public
  `latest.json` metadata before falling back to the GitHub Releases API, so
  in-app updates keep working when the unauthenticated API returns 403.
- **Collection Settings and response view layout.** Collection Settings now uses
  bounded left/right panes so controls cannot collapse into vertical text, and
  the Response body view switch no longer shows a broken standalone "View"
  label.

## [0.27.4] — 2026-06-18

### Fixed
- **Collection Settings redesign.** The modal now uses a compact custom header,
  clickable section picker, and focused detail pane instead of a non-clickable
  status rail plus stacked settings that could still feel broken on wide
  windows.

## [0.27.3] — 2026-06-18

### Fixed
- **Collection Settings modal collapse.** The modal now reserves fixed regions
  for the sidebar, content pane, and footer so controls cannot be squeezed into
  vertical text or disappear when the window is wide but the internal layout
  recalculates badly.

## [0.27.2] — 2026-06-18

### Fixed
- **Update checks avoid GitHub API rate limits.** Update checks now read
  static release metadata from GitHub Pages first, then use GitHub's REST API
  only as a fallback.

## [0.27.1] — 2026-06-18

### Fixed
- **Collection Settings layout.** The collection settings modal now uses a
  wider two-column layout with a left status rail and grouped right-side
  controls, making Git, masking, folder export, and OpenAPI options easier to
  scan.

## [0.27.0] — 2026-06-18

### Added
- **Native reviewable workspace files.** Git workspace exports now write `.rr`
  request files in Rusty Requester's own compact text format instead of plain
  JSON, keep legacy `.json` request imports working, and include
  `environments/*.rrenv` plus a Git-safe `.gitignore` for local secret overlays.
- **Collection export mask rules.** Collection Settings now lets users choose
  comma-separated key/header/env patterns that should always be masked, plus
  patterns that are safe to keep readable in Git exports.
- **Cleaner update progress foundation.** In-app update progress is being
  reshaped around compact status and animation primitives instead of log-first
  modal chrome.

### Fixed
- **Manual update check accuracy.** Settings no longer reports "latest" when
  the GitHub latest-release check failed; failures now show an explicit message.
- **Mobile docs navigation.** The GitHub Pages usage docs now keep navigation
  available on mobile through a native expandable section menu.

## [0.26.0] — 2026-06-17

### Added
- **Collection-level Git sync settings.** Top-level collections now have a
  Collection Settings entry with a local collection directory, Git remote
  pull/push actions, a compact Git status/diff summary, and collection-scoped
  OpenAPI refresh. Private Git remotes use the user's existing local Git
  credentials; Rusty Requester stores no GitHub tokens.
- **Reviewable `.rr` request files.** Git workspace exports now write each
  request as pretty JSON with a Rusty Requester `.rr` extension while still
  importing older `.json` request files.

## [0.25.0] — 2026-06-17

### Added
- **Optional Workspace Sync.** Workspace Sync is now disabled by default in
  Settings and can be enabled when users want local Git workspace pull/push and
  OpenAPI refresh workflows. The sync modal stores user-selected local paths,
  pulls/pushes the deterministic Git workspace directory, can pull/push GitHub
  remotes through the user's existing local Git credentials for public or
  private repos, keeps exports masked by default, and refreshes
  OpenAPI-generated requests from a local JSON/YAML spec.

## [0.24.1] — 2026-06-17

### Changed
- **Cleaner dense UI selection states.** Request tabs and editor tabs now use
  neutral surfaces with orange underline indicators instead of filled orange
  blocks, and selected sidebar requests use a neutral row highlight with an
  accent rail so HTTP methods stay readable.
- **Compact response metadata.** Response status, timing, and size now render
  as one inline metadata row instead of separate chunky chips.
- **Compact response body find.** The response search action now opens a
  right-aligned find widget in the response toolbar, focuses automatically from
  both the toolbar icon and `Cmd/Ctrl+F`, and shows a small match count without
  stealing a full body row.
- **Quieter request table controls.** Query/header/body row toggles are smaller,
  blank key/value rows no longer use orange background blocks, and table rows
  keep the editor canvas visually calmer.
- **Cleaner sidebar tools.** Sidebar search now uses an integrated compact
  search field, and import/export buttons use consistent icon glyphs instead of
  emoji-style icons.
- **Environment modal cleanup.** The environment manager opens at a shorter
  default height and no longer shows a redundant bottom `Done` button because
  edits save immediately and the titlebar close control already dismisses it.

### Fixed
- **Assertion success toast noise.** Requests no longer show assertion summary
  toasts when all assertions pass; failures and errors still surface.

## [0.24.0] — 2026-06-17

### Added
- **Environment compare.** The environment manager can now compare a
  source and target environment, group added/missing/changed/unchanged
  variable keys, mask sensitive-looking values by default, copy a safe
  summary, and add missing source keys to the target.
- **Collection Runner result details.** Runner rows can now be selected
  by mouse or keyboard to inspect method, redacted URL, collection
  path, status, timing phases, assertion outcomes, and extractor
  misses, with copy-safe summaries that keep response bodies, headers,
  cookies, and extracted values out of reports and copied text.
- **Saved Collection Runner presets.** Runner configurations can now be
  saved, loaded, renamed, duplicated, and deleted, including all-collection
  or folder scope, active environment selection, and explicit data-row text.
  Missing folder or environment references fall back gracefully when loaded.
- **Usage guide for GitHub Pages.** Added a docs page covering first
  requests, collections, environments, auth, runner workflows, import/export,
  safe sharing, backups, and shortcuts.

### Security
- **Secret scanner for collection exports.** JSON/YAML exports now run
  a local-only scan for common token/key patterns and sensitive key
  names before opening the save dialog. When likely secrets are found,
  users can cancel, export the original file, or export a redacted copy.
- **Code snippet sharing is redacted by default.** cURL, Python
  `requests`, JavaScript `fetch`, and HTTPie exports now preserve
  request shape while masking authorization, cookie, sensitive query,
  and sensitive body values. The snippet panel keeps an explicit
  **Copy raw** action for intentional unredacted copies.

## [0.23.0] — 2026-06-17

### Added
- **Git workspace export/import core.** Added a deterministic directory format
  with `workspace.json` plus one readable request JSON file per request,
  stable ID preservation, bounded import validation, and docs for merge
  conflict resolution.
- **OpenAPI 3.x import.** JSON and YAML OpenAPI specs can now be
  imported as request collections, grouped by operation tags or path
  roots, with query/path/header parameters, request descriptions, auth
  hints, JSON request body examples, and generator metadata for refresh.
- **OpenAPI refresh foundation.** Imported OpenAPI requests can be
  refreshed from an updated local spec while preserving user-edited
  request names, saved auth values, custom rows, tests, and extractors.
- **Workspace backup manager.** Collection imports now snapshot the
  current workspace before mutating saved state, and users can manually
  create, inspect, and restore workspace backups from the app menu.

### Security
- **Secret-safe Git workspace exports by default.** The Git workspace exporter
  masks auth values, sensitive key/value rows, cookies, form fields, OAuth
  cached tokens, and URL query/fragment data unless the caller explicitly opts
  into including secrets.
- **Safer import and restore paths.** Imports reject files over 10 MB;
  OpenAPI sensitive parameter examples such as tokens and authorization
  headers are not persisted; backup restore validates workspace JSON and
  only accepts files from the app's backup directory.
- **Private backup permissions on Unix.** Backup directories and files
  are tightened to owner-only permissions where supported.

### Tests
- **Git workspace snapshot and round-trip coverage.** Added tests for
  deterministic layout, ID-preserving lossless imports, default secret masking,
  and conflict/escape validation.
- **Import compatibility coverage.** Added JSON/YAML round-trip,
  Postman edge-case, cURL edge-case, OpenAPI import/refresh, and
  backup/restore tests.

## [0.22.1] — 2026-06-17

### Fixed
- **Save Request modal layout polish.** The modal now uses a custom
  header with clearer title/subtitle treatment, a consistent close
  button, larger default sizing, and a folder icon that matches the
  sidebar request tree.
- **Toast messages no longer wrap into narrow vertical columns.**
  Short messages such as "Request saved" now keep a stable minimum
  width and render on one line.

## [0.22.0] — 2026-06-17

### Added
- **Collection Runner with data-driven runs.** Run all collections or a
  selected folder/collection scope as a sequence, optionally repeating
  the run for each CSV or JSON data row. Data rows are overlaid as
  runner-scoped environment variables, so existing `{{var}}`
  substitution works without changing saved environments.
- **Live runner progress.** The Collection Runner results table now
  fills request-by-request while the run is active, with current
  `X / Y` progress, status, timing, assertion counts, extractor counts,
  and miss counts.
- **Runner report export.** Save runner results as CSV or self-contained
  HTML for sharing or archiving.
- **Collection Runner entry points.** Added an Actions Palette item and
  native macOS Request-menu item for opening the runner.

### Changed
- **Theme selection now previews before saving.** Choosing Dark, Light,
  or Postman in Settings immediately previews the app chrome, but the
  preference is only persisted when Save is pressed.
- **Response inspector polish.** The response area received layout and
  readability refinements so status/timing/size controls, body modes,
  and inspector content behave more consistently across narrow and wide
  layouts.
- **UI stability polish.** Tightened several egui layout and interaction
  edges from the UI/UX polish branch, including title/sidebar/theme
  behavior that previously felt jumpy or stale.

### Security
- **Secret-safe runner reporting.** Runner UI rows and exports avoid
  response bodies, response headers, cookies, and extracted values;
  report URLs redact query strings and fragments.
- **Report output hardening.** HTML reports escape all dynamic values,
  and CSV reports neutralize spreadsheet formula-injection prefixes.
- **Privacy masking helpers.** Added shared privacy utilities for
  sensitive-key detection, secret masking, URL query/fragment redaction,
  and HTML escaping.

## [0.21.1] — 2026-06-04

### Fixed
- **Saved-request tabs now show the amber unsaved-change dot.** Draft
  tabs already showed the dot next to the close button; saved request
  tabs now show the same indicator after edits until an explicit save.
- **Tooltip/status separator dots no longer render as tofu squares.**
  Replaced literal bullet glyphs in rendered egui text with painter
  circles, covering the tab tooltip's "Unsaved draft/changes" marker
  and the response status chip separator. The Body tab's content dot
  and update-instructions bullets are painter-drawn too, preserving
  the original UI without relying on missing font glyphs.

## [0.21.0] — 2026-06-03

### Added
- **Inline update status + "Update now" button in Settings.** Clicking
  "Check for updates now" previously surfaced its result *behind* the
  Settings modal — users had to close Settings to find the Update
  button. Settings now mirrors the result inline: a spinner while
  checking, "You're on the latest version" when up to date, or a green
  block with the new version + **Update now** + Release notes when an
  update is available. Picks up updates found by the launch-time
  auto-check too, so re-opening Settings always reflects current
  state.

### Fixed
- **Post-update success banner auto-dismisses after 10s.** The "Updated
  to vX.Y.Z" pill at bottom-right was supposed to self-dismiss (the
  doc comment claimed ~12 s) but had no timer wired up — it stayed
  forever until the user clicked Dismiss. Now success auto-clears
  after 10 seconds; failure banners still stay so users don't miss
  the error or the View-log button.
- **Post-update banner check icon renders correctly.** The success
  banner used a literal `✓` (U+2713) which isn't in egui's bundled
  font and rendered as a tofu square. Swapped to the Phosphor CHECK
  glyph (`sidebar.rs:926` already flagged this issue elsewhere).

## [0.20.1] — 2026-06-02

### Fixed
- **Search field in the Save Request modal is clickable again.** The
  modal auto-focuses the Request Name field on open, but the guard was
  re-asserting focus every frame the search field was empty — so
  clicking into the search input lost focus instantly on the next
  frame and you could never type a query. Now it's a one-shot: focus
  the name field once when the modal opens, never again.

## [0.20.0] — 2026-05-26

### Fixed
- **Empty assertion ghost row no longer counted as a failure.** The
  Tests tab auto-appends a trailing placeholder row so you can type a
  new assertion. Previously that row was treated as a real `Status
  equals ""` check and reported "0 passed, 1 failed" on every response
  for requests where you hadn't configured any assertions. Now rows
  with empty expression AND empty expected are skipped during
  evaluation — same criteria the editor uses to decide a row is
  "ghost".

## [0.19.0] — 2026-05-19

### Added
- **One-click in-app update (macOS & Linux).** The "update available"
  modal now has a single **Update now** button — no more copying a
  command and pasting it into a terminal. It runs `install.sh` in a
  detached wrapper (`nohup` + `disown`) that survives the installer
  killing the running app, shows a live tail of the install log while
  it runs, and relaunches the new binary automatically when done. On
  the next launch a small banner anchored bottom-right reports
  success ("Updated to vX.Y.Z") or failure with a **View log**
  button. Log / status / runner files live under
  `$TMPDIR/rusty-requester/` (auto-cleaned by the OS — no clutter in
  `Application Support`). Windows users still see the copy-command
  fallback because there's no clean detached-spawn + auto-relaunch
  recipe there.

## [0.18.8] — 2026-05-18

### Fixed
- **Paste-as-cURL with bash ANSI-C body (`--data-raw $'…'`).**
  Browsers' "Copy as cURL" emits `$'…'` whenever the body contains
  escapes; the tokenizer used to drop the `$` into the body itself
  (leading `${"…"}`) and leave `\n` / `!` literal. Now recognized
  as ANSI-C quoting: escapes are decoded and the `$` is consumed.

## [0.18.7] — 2026-05-13

### Added
- **Drag-and-drop tab reorder.** Click and drag any request tab to a
  new position in the strip; a vertical accent indicator marks where
  it will land. Mirrors browser / Postman tab-shuffle behavior.
  Pinned state and per-tab response cache survive the reorder.
- **Double-click a tab to rename the request.** Pops an inline TextEdit
  in place of the tab name — same rename flow as F2 and the sidebar's
  row-level rename. Enter commits, Esc cancels. Right-click → Rename
  in the tab context menu does the same thing for trackpad users.
- **Reveal-active in the sidebar tree.** Clicking a tab now expands all
  ancestor folders in the left sidebar and scrolls the matching
  request row into view. Tabs no longer feel disconnected from the
  collection tree when you've been jumping around — you always know
  where the active request lives.

### Fixed
- **Tabs bar now scrolls with the mouse wheel.** egui's horizontal-only
  ScrollArea only reads the x-axis wheel delta, so a regular mouse
  (which produces y-only delta) couldn't scroll the tab strip when
  many tabs were open — only click-and-drag on the bar worked. Now
  hovering the strip remaps vertical wheel input to horizontal scroll
  in place, matching browser tab-bar behavior.
- **`⌘S` on a saved request now gives visible feedback.** Saved
  requests are auto-persisted on every edit, so `⌘S` previously
  triggered no work *and* no toast, reading as a broken shortcut.
  It now flashes a `Saved` toast so the keypress has a clear
  acknowledgement. Drafts still open the Save-to-folder modal.

## [0.18.5] — 2026-04-30

### Security
- **Bumped `rustls-webpki` from 0.103.12 to 0.103.13 for
  [RUSTSEC-2026-0104](https://rustsec.org/advisories/RUSTSEC-2026-0104).**
  Reachable panic in CRL parsing via the transitive dependency chain
  `reqwest → hyper-rustls → tokio-rustls → rustls → rustls-webpki`.
  Triggerable by a crafted server certificate revocation list during
  TLS validation — i.e. any HTTPS request to a hostile or compromised
  server. Caught by `cargo audit` in CI; lockfile-only bump, no API
  surface change.

## [0.18.4] — 2026-04-30

### Added
- **Postman-style JSON folding in the response Body view.** Every
  multi-line `{` or `[` now has a chevron in the gutter — click it to
  collapse the block into a `{ …}` / `[ …]` placeholder on the opener
  line, click again to expand. Makes it easy to skim deeply nested
  GeoJSON / OpenAPI / GraphQL responses without scrolling through
  thousands of `coordinates` rows. Selection, copy, and ⌘F search
  still work over the visible (un-folded) text. Fold state is
  per-response and resets on every Send.

### Fixed
- **Response status pill / time / size no longer overlap the Body /
  Headers / view-mode tabs when the panel is narrow.** Opening the
  code-snippet panel shrinks the response panel; the right-aligned
  chips were drawn on the same row as the tabs and `ui.horizontal`
  doesn't reserve space between left and right children, so they
  collided visually. Toolbar now measures remaining width and bumps
  the chips + action icons to a second row when there isn't room
  inline.

## [0.18.3] — 2026-04-24

### Fixed
- **JSON response gutter no longer renders ghost line numbers below
  the content.** When a server appended trailing newlines / blank
  lines after the object (or returned NDJSON / debug padding that
  failed to parse), `serde_json::from_str` rejected it and we fell
  through to the raw body verbatim. The gutter counted lines via
  `split('\n')`, so those trailing blanks became empty line numbers
  below the closing `}` — a visible strip of "68, 69, 70…" under a
  response that ended at line 67. Now trims trailing whitespace on
  the body once at ingest so the gutter stops exactly on the last
  content line.

## [0.18.2] — 2026-04-21

### Changed
- **Key-value tables (Params / Headers / Cookies / Form-data) now
  show a row separator.** Previously rows floated on the same canvas
  color with no divider — short values like `1` or `true` forced the
  eye to trace 1000+ px of empty space to pair them with their key.
  Added a thin 1 px bottom border per row matching the header
  separator, so pairings read at a glance. Matches the convention
  used by Postman / Insomnia / Bruno / Hoppscotch.

## [0.18.1] — 2026-04-21

### Fixed
- **App no longer crashes when a response body contains an emoji.**
  `push_history_entry` truncated the response preview to 256 *bytes*
  via `String::truncate`, which panics when byte 256 lands mid-UTF-8
  (e.g. a `👋` straddling the cut). Position-dependent: emojis sitting
  fully before or after byte 256 were fine, but any that spanned the
  cut brought the app down immediately after Send. Now walks back to
  the nearest char boundary before truncating — safe for any emoji /
  non-ASCII text at any byte position.

## [0.18.0] — 2026-04-21

### Added
- **Postman theme.** Third theme option alongside Dark and Light,
  mirroring the Postman web UI: pure-white main canvas, warm-gray
  sidebar chrome (`#F9F9F9`), Postman-blue accent (`#3A82E6`
  sampled from the live Send button), and Inter-Light as the UI
  font for matching reading density. Rides the same paper-path
  syntax highlight (GitHub-ish palette) as the Light theme.
  Contributed by @fauzannadhif in #19.

## [0.17.0] — 2026-04-20

### Added
- **Find in response body** (`Cmd/Ctrl+F`). The inline search bar
  was previously click-only via the magnifying-glass button, so
  Linux / Windows users (who never reach for clickable search
  icons first) effectively had no find-in-response. Now bound to
  `Cmd+F` on macOS and `Ctrl+F` on Linux / Windows — one
  `i.modifiers.command` check covers both since egui maps
  "command" to the platform-appropriate meta key. Switches to
  the Body tab if you're on Headers, focuses the search input
  immediately, and Escape closes.
- **Clean uninstall path.** The Linux tarball now ships
  `uninstall-local.sh` alongside `install-local.sh`, and
  `install.sh` gained an `UNINSTALL=1` mode so the one-liner
  works both ways:
  ```
  curl … | UNINSTALL=1 bash           # keep user data
  curl … | UNINSTALL=1 PURGE=1 bash   # also wipe data.json
  ```
  Works on macOS too (removes `RustyRequester.app` from
  `/Applications` or `~/Applications`; optional purge of
  `~/Library/Application Support/rusty-requester`).

### Fixed
- **Ubuntu dock / Activities icon** (issue #18). GNOME under
  Wayland ignores `_NET_WM_ICON` set by the app — it matches
  the running window to a `.desktop` file via `app_id` /
  `StartupWMClass`, then pulls the icon from there. Neither
  side was set, so the dock fell back to a generic cog. Fixed
  by adding `.with_app_id("rusty-requester")` on the
  `ViewportBuilder` AND `StartupWMClass=rusty-requester` in
  the installed `.desktop` file. Both must match — one without
  the other is a no-op.
- **Linux binary no longer sits next to `data.json`.** Before,
  the installer put the binary at
  `~/.local/share/rusty-requester/rusty-requester` and
  symlinked it into `~/.local/bin`. That's the same directory
  the app writes `data.json` to, so a naive
  `rm -rf ~/.local/share/rusty-requester` uninstall also
  nuked user collections / history / OAuth tokens. Binary is
  now installed directly to `~/.local/bin/rusty-requester`
  (no symlink, no shared dir). `install-local.sh` migrates
  the old layout automatically on upgrade — leaves
  `data.json` untouched.

## [0.16.9] — 2026-04-20

### Fixed
- **Request rename + Enter now commits the new name** (issue #16).
  The commit branch gated on `enter && edit_resp.has_focus()` —
  but egui's singleline TextEdit de-focuses in the same frame
  Enter fires, so `has_focus()` was already false and the commit
  silently dropped. Rename appeared to do nothing. Switched to
  the canonical egui `lost_focus() && enter` pattern (same one
  the folder rename uses) so Enter reliably commits.

## [0.16.8] — 2026-04-20

### Fixed
- **Collection / folder rename input is now readable + the confirm
  button renders** (issue #15). The inline rename row was constrained to the
  header's tight text-width rect, so the TextEdit clipped to a
  sliver; and the ✓ / ✖ glyphs (U+2713 / U+2716) aren't in egui's
  bundled font on some systems, so the confirm button showed as a
  blank rectangle (same failure mode as the pre-0.16.5 update-pill
  arrow). Now the rename row spans the full sidebar width and the
  buttons use Phosphor `CHECK` / `X`. Also added Escape-to-cancel
  parity with the request rename UX.
- **App icon now shows in Ubuntu / GNOME launchers.** Previous
  install path relied on freedesktop icon-theme lookup
  (`Icon=rusty-requester` + PNG in `hicolor/512x512/apps/`), which
  silently failed on GNOME until `gtk-update-icon-cache` was run
  manually or the user logged out. `install-local.sh` now: (1)
  rewrites the `.desktop` file's `Icon=` line to an absolute
  `~/.local/share/pixmaps/rusty-requester.png` path (bypasses
  theme lookup + icon cache entirely — always resolves), (2)
  drops the PNG into both `hicolor/512x512/apps/` and
  `pixmaps/` (legacy fallback for DEs that skip single-size
  hicolor themes), (3) runs `gtk-update-icon-cache` if available
  as a belt-and-suspenders refresh.

## [0.16.7] — 2026-04-20

### Fixed
- **Linux install works on older glibc.** Release builds now run on
  `ubuntu-22.04` (glibc 2.35) instead of `ubuntu-latest` (which
  silently upgraded to 24.04 / glibc 2.39). Users on Ubuntu 22.04,
  Debian 12, Fedora 36+, and RHEL 9 no longer hit
  `libc.so.6: version 'GLIBC_2.39' not found` when launching the
  binary installed by `install.sh`.
- **`install.sh` one-liner works on Linux.** Two install-time bugs:
  - `curl … | awk '{… exit}'` triggered `SIGPIPE` (curl exit 23
    "Failed writing body") because awk closed the pipe after the
    first match. Under `set -o pipefail` that killed the script on
    Linux. Now captures curl's body first, then parses.
  - `mktemp -d -t rusty-requester` fails on Linux ("too few X's")
    since GNU mktemp requires the template to contain `XXXXXX`.
    macOS mktemp is lenient. Switched to an explicit
    `${TMPDIR:-/tmp}/rusty-requester.XXXXXX` template that works on
    both.

## [0.16.6] — 2026-04-18

### Changed
- **Refined palette for both themes.**
  - **Dark "Editor Dark"**: canvas `#1A1A1A` (was `#16181D`),
    elevated cards `#252525`, inputs `#333333`, borders `#404040`,
    primary text `#F3F4F6`, secondary `#9CA3AF`. Flipped the layer
    model — panels and cards are now *brighter* than the canvas
    (modern elevation convention: lift = light), not darker.
    Reduces the harsh near-black bg, keeps the sleek terminal feel.
  - **Light "Paper Light"**: canvas `#E9ECEF` (was `#FCFCFD`),
    cards pure white `#FFFFFF` so they pop as defined containers
    against the cool-gray canvas. Inputs use filled `#F3F4F6` on
    the white cards for structure. Borders `#D1D5DB`, primary
    text `#2D3748` dark slate, secondary `#6B7280` medium slate
    (was `#656D76`, now legible without being washed out).
- **Accent color is now theme-aware.**
  - Dark: `#D85539` warm rust (first coral-red swing `#EF5350`
    overshot into pink; this lands between original saturated
    `#CE422B` and the coral-pink, keeping the "rusty" feel).
  - Light: `#C43C28` deep rust (consistent rust family across
    themes, darker for WCAG AA vs white button text).
  - `C_ACCENT` is now a legacy compile-time constant (kept for
    `const` contexts like icon SVGs); all UI call sites use the
    new theme-aware `accent()` function. 47 sites swept.
- **Sidebar is now an elevated card.** Previously the sidebar
  shared the same `bg()` color as the central canvas, producing a
  flat uniform page. Now it uses `panel_dark()` — pure white in
  light mode (clearly lifted against the gray canvas), `#252525`
  elevated in dark mode. Re-enabled the separator line between
  sidebar and central panel since they're now different colors.
- **KV table inputs are frameless** (Params / Headers / Cookies).
  Each cell inherits the canvas color instead of showing as a
  filled pill. Column-header row + separator line above provide
  table structure; hint text shows through empty cells. Matches
  Postman's "ghost cells" look and fixes the "white pills on
  white / gray canvas" noise in the central panel.
- **URL bar input is frameless** — blends into its outer rounded
  container. The outer container's `fill` matches the canvas
  exactly so no dark-on-dark mismatch inside vs outside the
  rectangle. Method combo + Send/Code buttons keep their own
  visible widget frames.

## [0.16.5] — 2026-04-18

### Added
- **"Check for updates now" button in Settings.** Forces an immediate
  GitHub API call without restarting the app. Useful after dismissing
  the pill, or for users who turned off the launch check. Clears any
  stored per-version dismissal so a manual re-check always reveals a
  pending update.
- **CHANGELOG-driven release notes.** `deploy.sh` now refuses to
  release if `CHANGELOG.md` has no `## Unreleased` section with
  content. On deploy, the section is promoted to
  `## [X.Y.Z] — YYYY-MM-DD` and committed alongside the version
  bump. `.github/workflows/release.yml` extracts the matching
  section at release time and passes it to action-gh-release via
  `body_path`, so the GitHub release page shows human-written notes
  above GitHub's auto-generated PR/commit list.

### Changed
- **Update-available pill: no auto-modal.** Earlier attempt to
  auto-open the install-instructions modal on first detection was
  reverted — a modal on launch blocks users from getting to work.
  The persistent sidebar pill alone is now the notification; users
  click when ready.
- **Update pill is dismissible per-version.** New
  `AppSettings.dismissed_update_version` field stores the last
  version the user explicitly ✕'d from the pill. Suppresses the
  pill for that exact version until a newer tag drops, so users
  who defer updates don't see the same pill every launch.
- **Update pill arrow glyph** switched from U+2191 (`↑`) to
  Phosphor `ARROW_UP` — egui's bundled font lacks U+2191 and
  rendered it as a "tofu" square.

## [0.16.3] — Send-button color parity

### Fixed
- **Send button color** now uses `C_ACCENT` (rust orange) instead of
  `C_PURPLE` (burnt sienna — the PATCH method color repurposed).
  Matches "New Collection" + the active-tab underline; the
  primary-CTA family is finally visually consistent.

## [0.16.2] — Update-modal polish

### Changed
- **Accurate update-modal installer description.** Previous copy
  lied — said the installer "relaunches" the app (it doesn't —
  `install.sh` tells the user to launch from Spotlight after
  it finishes). Rewrote the modal help text to honestly describe
  what the one-line installer does: quits running app, downloads
  DMG / tarball, replaces the installed binary, strips Gatekeeper
  quarantine, refreshes Launch Services. Also calls out
  explicitly that `data.json` (collections, history, OAuth tokens,
  env vars) is untouched on upgrade. Cross-platform accurate:
  mentions macOS `/Applications` and Linux `~/.local/bin` paths.

## [0.16.1] — Update-check toggle + sidebar pill

### Added
- **Settings toggle: "Check for updates on launch"** (defaults on).
  Disables the one silent GET to `api.github.com/.../releases/latest`
  on startup for users who want strict offline operation — no
  outbound traffic from the app unless this is enabled.
- **Sidebar update pill.** When the update-check finds a newer tag,
  a rust-orange pill with the version appears next to the running
  version number in the sidebar header. Persistent (not a toast) —
  stays visible for the whole session.
- **Update-instructions modal.** Click the pill → modal shows the
  running vs available version, the official `curl | bash` one-line
  installer as a copyable code block, a "Copy command" button
  (clipboard + toast confirmation), and a "Release notes" button
  that opens the GitHub release page in the user's default browser.

## [0.16.0] — App icon redesign

### Changed
- **New app icon: Phosphor hammer striking a steel plate.** Replaces
  the old lettermark-heavy alphabet `R`. The plinth uses the app's
  `C_ACCENT` rust orange for brand cohesion — glance at the Dock,
  glance at the Send button inside, same color. Same approach
  Postman takes with their purple. The hammer glyph is MIT-licensed
  from Phosphor Icons (already a dep for in-app UI icons); the
  strike plate is a hand-drawn cream slab with top-highlight +
  bottom-shadow for 3-D depth. SVG source is now the canonical
  master at `assets/icon.svg` (alternatives kept as archives at
  `assets/icon-v*.svg` / `.png` for comparison).
- **Icon pipeline.** `scripts/generate_icon.py` rewritten to render
  `icon.svg` → 1024×1024 transparent PNG via `resvg-py` (pure-Rust
  renderer, no libcairo). Makefile's `sips`-based downscaling to
  the 10 iconset sizes is unchanged.

## [0.15.12] — Polish: response, tree, headers, placeholders

### Changed
- **Response-wipe on tab re-click fixed.** Clicking the
  already-active tab was calling `open_request`, which then called
  `restore_response_for` with an empty cache and wiped the live
  response. `open_request` now early-returns when the clicked tab
  matches `selected_request_id`.
- **Tree view spacing tightened.** Indent per level 16 → 10 px,
  single space (not two) between key and summary, vertical
  item spacing 1 px inside tree rows. Nested JSON now stacks
  densely like a real tree.
- **Headers pane softened.** Keys changed from saturated `C_ACCENT`
  to `muted()`, dropped the `.striped(true)` zebra in favor of the
  default grid with 6-px row gap. Much calmer table.
- **16-px right-edge rule** standardized across the response chips
  row and KV-row trailing margin (was 6 / 12 px respectively). No
  more chips or `×` buttons sitting flush against the scrollbar.

### Added
- **Theme-aware placeholder color** (`hint_color()`). Previously
  `C_HINT = #50545F` — too dark on the dark canvas (contrast below
  WCAG AA), and egui's default `weak_text_color` was too pale on
  the near-white light canvas. New values: `#8A8E98` on dark,
  `#8C949E` on light — both hit ~4.5:1 contrast. Every
  `.hint_text("…")` call site in the app was swept to use the
  `hint()` helper, so all 12 placeholder strings benefit.

## [0.15.11] — Body tab: syntax highlight + radio rows

### Added
- **Request body JSON syntax highlighting.** Same layouter the
  response-body view uses — keys / strings / numbers /
  `true / false / null` all colored (Monokai on dark, GitHub-ish
  dark-on-paper on light). Non-JSON raw text still highlights
  quoted strings and numbers as a sensible fallback.
- **Response JSON: two-column gutter/content layout.** Line
  numbers in a separate left column, content TextEdit wraps inside
  its own right column. Same pattern the snippet panel already
  used. Fixes the previous diagonal-drag-scroll glitch caused by
  the nested horizontal ScrollArea.

### Changed
- **Body-type selector → Postman-style radio rows.** Dropped the
  `"Body type"` prefix label + the saturated `selectable_value`
  pill; now a simple radio row with `ui.radio_value`. Less visual
  noise, matches how Postman does it.

## [0.15.10] — Response cache on tab switch

### Added
- **Per-request response cache.** Switching tabs previously wiped
  the response; now a `CachedResponse` snapshot (body, status,
  timings, headers, SSE events, assertion results) is stashed on
  tab-switch and restored on tab re-activation. In-memory only,
  not persisted to `data.json`. Matches Postman's session-scoped
  behavior. Closed tabs drop their cache entry.

### Fixed
- **Tab-strip click ignored the cache.** `open_request` had the
  stash/restore wired, but the tab-strip click path in
  `editor.rs:275` bypassed it with an inline clear. Routed through
  `open_request` so every tab switch uses the cache.

## [0.15.9] — Blank-row exclusion in tab counts

### Fixed
- **Params / Headers / Cookies count** excluded the trailing blank
  "ghost" row that `render_kv_table` always appends. A brand-new
  request was showing "Params (1)" even though no params existed.
  Now filters with `!r.is_blank()` before counting.

## [0.15.8] — Palette-row polish

### Changed
- **Softer palette-row selection.** Selected row now uses a
  translucent accent-tint fill (`Color32::from_rgba_unmultiplied`
  ~14 %) plus a 3 px left accent bar. Previously the saturated
  `C_ACCENT.linear_multiply(0.18)` red block read as "destructive"
  rather than "selected". Row height bumped 34 → 44 px so the
  breadcrumb has padding instead of sitting flush against the
  bottom edge of the fill.
- **Palette footer arrows.** `↑ ↓` (U+2191 / U+2193) were missing
  from the bundled font and rendered as tofu squares. Swapped for
  Phosphor `ARROW_UP` / `ARROW_DOWN`.

## [0.15.7] — Tab-chrome simplification

### Changed
- **Active-tab indicator collapsed to a single bottom accent line.**
  Previous layout stacked a red top bar, a peach fill tint, green
  method text, and an amber draft dot on the same tab — four
  competing accents, read as "broken color shift". Same convention
  as the inline Body / Headers / Tests tabs so selection semantics
  are consistent across the app.

## [0.15.6] — Light-mode retune + paper-cuts round

### Added
- **Per-request response cache.** Switching tabs no longer wipes
  the response. `CachedResponse` snapshot (body, status, timings,
  headers, SSE events, assertion results) is stashed on tab-switch
  and restored on tab re-activation. In-memory only — not
  persisted to `data.json`. Matches Postman's session-scoped
  behaviour. Closed tabs drop their cache entry.
- **Inline "Copied" flash.** Green ✓ Copied label appears next to
  the response-body and snippet-panel copy buttons for ~1.5 s.
  Replaces the bottom-right toast, which was getting hidden behind
  the snippet side panel.
- **KV paste sanitizer.** Pasting values from Chrome DevTools'
  Network tab used to drop invisible Unicode (U+200B zero-width
  spaces, U+FEFF BOM, bidi marks, ASCII controls) into params /
  headers / cookies — rendered as "tofu" rectangles and silently
  broke requests. `sanitize_pasted` strips them on every change.

### Changed
- **JSON response body: horizontal scroll, no wrap.** Long minified
  lines (e.g. encoded polygon coordinates) used to wrap with no
  gutter indent, so continuation rows visually overlapped the
  line-number column. Matches VS Code's default for source files.
- **Light palette retuned, twice.** The `#EBEDF1` canvas washed out
  syntax-highlighted JSON; the `#D9DCE3` correction read as muddy
  grey. Final values: bg `#FCFCFD` canvas, panel `#F3F4F7`,
  elevated `#EDEFF2`, text `#1F2328`. GitHub-ish dark-on-paper
  syntax palette for the response body in light mode (Monokai
  stays on dark).
- **Snippet / response syntax highlight is now theme-aware.**
  `hl_text()` / `hl_json_key()` / `hl_string()` etc. branch on the
  active theme so the response body reads cleanly in either.

### Fixed
- Clippy `collapsible_match` on `src/cookies.rs` path arm.
- Clippy `while_let_loop` on the request-in-flight poll in
  `src/main.rs`.

## [0.15.5] — Drop the modified-dot; formalise the threat model

### Changed
- **Modified-saved-request dot removed.** Only drafts now show the
  amber indicator. Saved requests auto-persist every keystroke, so
  a "modified" dot was stale by design. Removed `pristine_request`
  snapshot field and the `active_request_is_dirty()` check.
- **Cmd+S "Saved" toast on saved requests removed.** It fired every
  time because edits were already persistent — pure noise.

### Added
- **Explicit security section in `readme.md`.** Threat model laid
  out plainly: what a hostile server can't do to you
  (no auto-download, no code execution on response content, no
  memory-corruption path, no shell on curl paste) vs what the user
  still owns (SSRF to localhost, saved-then-opened files, plaintext
  `data.json`, supply chain on deps).
- **Competitor comparison table.** 4-way against Postman /
  Insomnia / Bruno covering runtime, size, RAM, startup, telemetry,
  account-gating, supply-chain surface, response-HTML sandboxing.

## [0.15.4] — Unify tab indicator

### Changed
- **Single amber dot for both drafts and modified saved tabs.** The
  prior split (solid dot for drafts, hollow ring for modified
  saved) confused users more than it clarified — both indicate
  "edits worth noticing", so collapse to one visual and let the
  tooltip disambiguate. (Fully removed for saved in 0.15.5.)

## [0.15.3] — Modified-since-opened indicator (superseded)

### Added
- **Hollow-ring dirty indicator on saved-request tabs.** Compared
  the live `editing_*` fields to a `pristine_request` snapshot
  taken at load time. Superseded by 0.15.4 then fully removed in
  0.15.5.

### Fixed
- **CI clippy on Linux.** Non-macOS title-bar stub was flagged as
  `dead_code`; fixed with an `#[allow]` on the cross-platform stub
  and `cargo clippy --all-targets -- -D warnings` added to
  `scripts/deploy.sh` so the check runs locally before tagging.

## [0.15.2] — Palette redesign + deploy hardening

### Changed
- **Command palette chrome.** Dropped the dim backdrop (which
  competed with the palette itself); floats with a shadow over the
  content, VS Code-style. Lighter, less intrusive.
- **MSRV declared at 1.73** (was implicitly higher in some
  build paths). Matches what the CI and install docs advertised.
- **crates.io metadata.** Cargo.toml gained `license`, `keywords`,
  `repository`, `homepage` so the crate is publishable.

### Fixed
- **OAuth browser-open cross-platform.** Linux/Windows paths weren't
  compiling because the `open` shim was macOS-only. Gated behind
  `#[cfg]` per platform (`xdg-open` on Linux, `cmd /c start` on
  Windows).
- **Misleading "OAuth ready" status** before the access token was
  actually present.

## [0.15.1] — Native macOS title bar

### Changed
- **Title-bar chrome merged into the app surface.** Raw `objc`
  calls on `NSWindow` to set `fullSizeContentView`,
  `titlebarAppearsTransparent`, and `titleVisibility = .hidden`.
  Traffic-light buttons float over the app content (like Ghostty,
  Xcode 15, etc.); no more dark empty strip above the content
  panel. Linux / Windows unchanged; the helper is `#[cfg]`-stubbed
  with `#[allow(dead_code)]` on non-macOS.

## [0.15.0] — OAuth 2.0

### Added
- **OAuth 2.0 Authorization Code + PKCE.** New `Auth::OAuth2` variant
  alongside Bearer / Basic. Full in-app flow — click **Get New
  Token** in the Auth tab, the app opens your provider's authorize
  URL in your default browser, spins up a loopback listener on a
  random `127.0.0.1:<port>`, parses the redirect, exchanges the
  code for a token via the provider's `token_url`. Access token +
  refresh token + expiry get cached on the request; subsequent
  sends inject `Authorization: Bearer <token>` automatically. The
  Auth tab shows a live status badge (valid / refreshing soon /
  expired) and a masked preview of the stored token.
- New `src/oauth.rs` module (6 unit tests, zero runtime deps
  beyond the new lightweight `sha2` for PKCE's S256 challenge).
  Covers the RFC 7636 canonical test vector for the challenge
  derivation.
- New `sha2` dependency — only for PKCE S256. Pure-Rust, ~40 KB,
  no transitive extras.

### Known limitations (follow-ups for 1.x)
- Client Credentials + Refresh Token flows aren't implemented yet —
  the Auth tab only offers Authorization Code + PKCE. When the
  access token expires, click **Get New Token** to re-run the
  flow manually.
- Tokens are persisted in `data.json` alongside other auth data,
  same security model as Bearer. Native-keychain storage remains
  on the post-1.0 roadmap.

## [0.14.0] — Light theme

### Added
- **Light theme** — new `Theme::Light` option in Settings. Flips
  egui's chrome (panels, text, borders, widget backgrounds) via a
  theme-aware `Palette` struct. Saturated accents — HTTP method
  colors, status pills, rust-orange accent, syntax highlighting —
  stay the same across both themes because they're tuned to read
  on either background. `Theme::Dark` remains the default.

## [0.13.0] — Hardening round 2

### Added
- **Panic log.** Any panic writes to `panic.log` next to `data.json`
  with version + location + backtrace. Chains to the default hook
  so the usual stderr + exit behavior still happens. Gives users a
  single file to attach to bug reports.
- **Update notification.** On launch, a 5-second background check
  against GitHub's latest-release API. If a newer version is
  published, a toast points the user at the Releases page. Silent
  on any failure (offline, rate-limited, parse error) — an API
  client whose update check was noisy would be ironic.
- **Security note in README.** Explicit call-out that `data.json`
  holds tokens in plaintext and relies on local-disk permissions.
  Keychain integration deferred to post-1.0.

### Changed
- **Palette contrast.** ⌘P / ⇧⌘P windows now render on a brighter
  `C_ELEVATED` background with an accent-tinted border and a soft
  drop-shadow — stands out from the darkened backdrop instead of
  blending into it.

### Deferred
- **Integration tests for the send path.** Requires restructuring the
  crate as `lib.rs + main.rs` so a separate integration-test binary
  can access internals, plus a mock HTTP server. Separate effort.
- **Keychain / native secret store.** Planned post-1.0; would back
  token fields in env variables with the OS keychain via the
  `keyring` crate. Until then, protect `data.json` via disk perms.

## [0.12.0] — Hardening round 1

### Added
- **Double size cap on SSE streaming.** Beyond the existing raw-network
  cap, the event log gets a `2 × max_body_mb` ceiling — defends against
  malicious servers that stream millions of tiny events (where pretty-
  printed output can be 3–4× the network size).

### Changed
- **Atomic state writes.** `save_state` now writes to `data.json.tmp`,
  `fsync`s, then renames over the real file. Prevents a crash / power
  cut mid-write from leaving a truncated file the next launch can't
  parse.
- **Corruption recovery.** On launch, if `data.json` fails to parse
  it's renamed to `data.json.broken.<unix-ts>` (preserving the user's
  data for forensic recovery) and the app starts fresh, surfacing a
  toast pointing to the backup file. Previously a corrupt file
  silently reset to empty state.
- **CI tightened.** `cargo clippy --all-targets -- -D warnings` is now
  a blocking job (was informational). `cargo-audit` added as a
  non-blocking informational job so dependency advisories surface on
  every PR.
- **Dependency refresh.** `cargo update` resolved 4 vulnerability
  advisories (`bytes` integer overflow + 3 `rustls-webpki` name /
  wildcard / CRL issues) by pulling in newer patch releases via
  transitive updates. Remaining `cargo-audit` warnings are unmaintained-
  crate notes from `rfd`'s GTK3 bindings — not exploitable.

## [0.11.0] — Docs split + quality of life

### Added
- **Actions palette (⇧⌘P)** — fuzzy-find + run any of 16 built-in app
  actions: New request, Duplicate / Close tab, Toggle pin, Save draft,
  Copy as cURL, Toggle snippet panel, Open environments, Open settings,
  Paste cURL, Import collection, Export JSON / YAML, Clear history,
  Toggle sidebar History/Collections, About. Self-discoverable — open
  the palette and type. Shortcut column on the right for actions that
  also have a keybinding.
- **Response diff** — sending a request twice populates a **Diff** pill
  in the body toolbar. Unified `+/-` line-diff against the previous
  response, `+A −B` summary header. Backed by an LCS-based diff
  implementation in `src/diff.rs` (5 unit tests, zero deps).

### Changed
- **`data.json` trims defaults.** Added `skip_serializing_if` on
  `Folder.description` (empty string), `StoredCookie.domain`,
  `StoredCookie.secure` / `http_only`, and `OpenTab.pinned` (false).
  Reads stay forward-compatible (all fields use `serde(default)`).
- **README split.** The 650-line README became a 217-line hero +
  install + quickstart + stability page, with `docs/FEATURES.md` (full
  feature catalog, usage guide, UI conventions, roadmap) and
  `docs/ARCHITECTURE.md` (dependencies, source layout, design notes,
  release flow) holding the detail.
- **CHANGELOG catch-up.** Filled in entries for v0.6 → v0.10
  (previously stuck at v0.5.1).

## [0.10.0] — Server-Sent Events

### Added
- **Server-Sent Events (SSE)** support. Responses with Content-Type
  `text/event-stream` are auto-detected and stream into a structured
  **Events** body-view — one collapsible row per event with
  `#N event-type · HH:MM:SS.mmm` headers, per-event JSON pretty-print,
  id / retry fields, and Copy-data button. Auto-scrolls while live,
  pulsing "Listening…" indicator at the bottom.
- New `src/sse.rs` module (8 unit tests, zero deps) — incremental
  line-oriented parser handling multi-line data, CRLF, comments,
  chunked splits.
- `RequestUpdate::Progress { snapshot, new_events }` / `Final`
  enum so the send task can emit multiple updates through the existing
  `mpsc::channel` without changing cancel semantics. Cancel still
  instantly aborts the stream via `JoinHandle::abort()`.

## [0.9.1] — Quick wins

### Added
- **Duplicate tab (⌘D)** and **Pinned tabs** — right-click menu gains
  "Duplicate tab" and "Pin tab" / "Unpin tab". Pinned tabs skip ⌘W,
  "Close others", and "Close all". Accent-colored pin glyph in the tab
  strip.
- **↑ / ↓ arrow-key navigation** through every request in the sidebar
  (wraps at both ends). Gated on `ctx.wants_keyboard_input()` so
  text-field cursor movement isn't hijacked.

### Removed
- Redundant "→ http" scheme indicator next to the URL bar (now that
  we always prepend http, the hint added no information).

## [0.9.0] — Icon font

### Added
- Replaced every hand-drawn painter icon (search, X, save, plus,
  three-dots, copy, folder, unplugged-plug, warning) with
  **Phosphor icon font glyphs** via the `egui-phosphor` crate.
  Crisp at every DPI, tintable via `RichText::color()`,
  zero image assets.
- `theme::hint()` helper rendering dim-colored placeholder text —
  fixes the "Key / Value / Description" placeholders looking like
  real data in KV tables and the URL hint.

## [0.8.0] — Draft-close confirmation

### Added
- **"Save changes?" modal** when closing a draft tab that has
  unsaved content (URL / body / headers filled in). Options:
  Don't save · Cancel · Save changes. Empty drafts still close
  silently.
- **⌘N** (new request) and **⌘W** (close tab) keyboard shortcuts
  (menu accelerators on macOS, egui input on Linux/Windows).

## [0.7.0–0.7.4] — Streaming, cancel, HTML preview, URL↔Params sync

### Added
- **Cancel button** — Send flips to Cancel while a request is in
  flight. Aborts the tokio task; dropping the future mid-`.await`
  also drops the hyper connection, so cancel is immediate.
- **HTML Preview** body view — strips `<script>`/`<style>`, replaces
  block tags with newlines, decodes entities. Pill only surfaces for
  `text/html` responses.
- **Illustrated failed / cancelled state** replacing the plain error
  text: large Phosphor icon (WIFI_SLASH / PROHIBIT), headline pill,
  error-detail chip, context hint line.
- **Bidirectional URL ↔ Params sync** (Postman-style): typing
  `?foo=bar` in the URL populates the Params tab; editing the table
  rebuilds the URL bar.
- Stable `id_source("url_bar_edit")` on the URL TextEdit so its undo
  buffer survives widget re-creation.
- Code snippets (cURL / Python / JS / HTTPie) now apply the same
  `ensure_url_scheme` as the send path — the displayed command
  matches what's actually sent.

## [0.6.0–0.6.2] — Tier 3 polish

(See individual release tags for per-patch detail.)

## [0.5.1] — Tier 3 wrap + CI fix

### Fixed
- macOS DMG build under GitHub Actions: skip Finder-layout AppleScript on
  headless runners (it leaves the volume busy and breaks `hdiutil detach`),
  retry detach with backoff, fall back to `diskutil unmount force`.

## [0.5.0] — Polish

### Added
- **Cookie jar (per-environment)** — `Set-Cookie` parsed and persisted with
  domain / path / expiry tracking; auto-replayed on the next request matching
  host (suffix) + path (prefix). Switching active environment swaps the
  cookie set so Staging cookies don't leak into Prod. 8 unit tests.
- **⌘P command palette** — fuzzy-find requests across every collection,
  ↑ / ↓ to navigate, Enter to open, Esc to dismiss. Backdrop dim, breadcrumb
  under each match. Wired into View → Command Palette… in the macOS menu.
- **Right-click "Copy path"** in the response Tree view → clipboard gets
  `data.items[0].id`; pastes directly into Extractors / Assertions.
- **Drag-to-reorder requests within a folder** via egui's built-in
  `DragAndDrop::set_payload` API, with accent-colored drop indicator.

### Changed
- Response status pill / time / size moved **inline with the Body / Headers
  tab row, on the right**, plus 6 px right-edge padding (Postman-style).
  Removed the standalone "Response" title row.
- HTTP client now also reuses a single long-lived `tokio::Runtime` (was
  built per-send) — saves ~1 ms + thread-spawn cost per click of Send.
- README rewrite covering all Tier 1 + 2 + 3 features, regrouped Roadmap.

## [0.4.0] — Testing & polish

### Added
- **Response assertions** in the Tests tab: status / header / body × 7 ops
  (equals, ≠, contains, matches `^2..$`, exists, `>`, `<`). Per-row
  pass/fail dot, hover for failure reason, post-send toast summary. 7 unit
  tests including a hand-rolled regex matcher (no `regex` crate dep).
- **Collection overview page** — click any folder's `⋯` → "Open overview"
  to see a homepage with title, recursive request/folder counts, an
  inline-editable description, and a clickable request list. New
  `Folder.description: String` field.

### Changed
- Single unified body toolbar — JSON / Tree / Raw pills, search / copy /
  save icons, status chips all on one row.
- Body view pills restyled as underline-only text tabs (no more chunky
  bordered rectangles).
- Removed macOS Edit menu (Cut / Copy / Paste / Select All) — its
  predefined items installed AppKit shortcuts that intercepted ⌘C/V/A
  before egui's TextEdit could see them. egui handles those shortcuts
  internally now that the menu doesn't steal them.
- Subtle "Beautify" / "Minify" right-aligned links in the request body
  (replaced chunky early-2000s-style buttons).
- Line-number gutter in the request body editor (matches the snippet
  panel pattern).
- Thin floating sidebar scrollbar (visible-when-needed; `floating = true`
  prevents the previous width-jitter on hover).

## [0.3.0] — Foundation: settings, safety, native menu

### Added
- **Settings modal** (sidebar gear) — request timeout, max body size cap,
  proxy URL, TLS verification toggle. Persisted to `data.json`.
- **Reused `reqwest::Client`** built once from `AppSettings`, rebuilt
  only when settings change.
- **Body size cap** via streaming `Response::chunk()` — large responses
  are truncated with a banner instead of blowing up memory.
- **Save response to file** icon in the body toolbar; auto-picks
  extension from Content-Type.
- **JWT decoder** — Bearer-token Auth tab auto-decodes header + payload
  into pretty JSON when the token has the `header.payload.signature`
  shape. Signature is *not* verified.
- **Native macOS menu bar** via `muda` — Rusty Requester · File · View ·
  Request · Help submenus with `⌘⏎` / `⌘P` / `⇧⌘C` accelerators.
  Linux keeps the in-window egui menu bar.
- **Custom About modal** with creator credit + Contribute / Report-issue
  links (sidesteps AppKit's auto-routing of "About" items to the stock
  panel via a zero-width-space prefix trick).
- Pointing-hand cursor on every clickable surface.
- Code-snippet panel uses a two-column gutter so wrapped long lines
  don't collide with the next line's number.

### Changed
- Removed `egui::Frame` border around the response code editor — the
  outer panel's border serves the role; eliminates "small floating
  card" look on short payloads.

### Fixed
- Sidebar width-jitter when typing in the search box (always-reserve
  the close-X slot so the row width is constant).
- Folder icon overlap with folder name (clean outlined silhouette
  drawn via line segments).

## [0.2.x] — Linux support, deployment automation, bug fixes

- Linux x86_64 tarball + universal macOS DMG built by GitHub Actions.
- One-line installer (`install.sh`) auto-detects platform.
- `scripts/deploy.sh vX.Y.Z` for one-arg release: bumps Cargo.toml +
  Makefile, builds + tests, commits + tags + pushes.
- Various UI fixes (folder icon, menu bar, sidebar resize jitter,
  response panel layout).

## [0.1.0] — Initial public release

- Native HTTP client with Params / Headers / Cookies / Body / Auth tabs
- Collections + subfolders, request tabs, environments, history
- cURL import (paste in URL bar) + export (snippet panel: cURL, Python,
  JS, HTTPie)
- Postman v2.1 collection import
- JSON / YAML export
- macOS DMG bundle, Apple Silicon + Intel
