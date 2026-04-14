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

- _placeholder for the next release_

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
