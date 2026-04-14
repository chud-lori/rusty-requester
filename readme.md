<h1 align="center">
  <img src="assets/icon.png" width="96" alt="Rusty Requester" /><br/>
  Rusty Requester
</h1>

<p align="center">
A <b>native, offline, lightweight</b> API client built with Rust and <code>egui</code> —
a Postman alternative that doesn't chew through hundreds of MB of RAM just to make HTTP requests.
</p>

<p align="center">
Vibe-coded because I got tired of Postman's bloat and cloud sync I never wanted,
and of managing a wall of raw <code>curl</code> commands in my terminal.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/mac%20os-000000?style=for-the-badge&logo=macos&logoColor=F0F0F0" alt="macOS" />
  <img src="https://img.shields.io/badge/linux-000000?style=for-the-badge&logo=linux&logoColor=F0F0F0" alt="Linux" />
</p>

---

## ✨ Features

### Core
- 🚀 **Truly native** — Rust + `egui`, no Electron, no Chromium
- 💾 **Fully offline** — all data lives in one local JSON file, no cloud sync, no telemetry
- 🎨 **Rust-forge dark UI** — warm copper / rust-orange / amber palette with colored HTTP-method pills and underlined tabs
- 🍎 Builds for Apple Silicon, Intel Mac, Linux, and Windows

### Request building
- 🔧 Full HTTP methods: `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`
- 📝 Tabbed editor: **Params · Headers · Cookies · Body · Auth · Tests**
- 🔗 Query-param builder with live "final URL" preview and auto-growing "ghost" row (type and a new empty row appears below)
- 🍪 Cookies list (merged into a `Cookie` header on send)
- 🔐 Auth presets: **No Auth · Bearer Token · Basic Auth** + **JWT decoder** (Bearer tokens auto-decode below the input — header & payload pretty JSON, scope/exp at a glance)
- 📦 **Body modes**: Raw / `x-www-form-urlencoded` / `multipart/form-data` / **GraphQL** (query + variables, sent as `application/json`)
- 📐 **Line-numbered request body** with subtle `Beautify` / `Minify` action links
- 🌱 **Environment variables** — define key/value vars per environment, reference them anywhere with `{{varname}}`. Switch active env from the sidebar.
- 📜 **Request history** — every send is logged (method, URL, status, time, response preview); browse the last 200 from the sidebar's History tab.
- 🔗 **Post-response extractors (Tests tab)** — dot/bracket paths (`data.token`, `items[0].id`, `$.x`) or header names that pull a value from the response and write it into the active environment, so the next request can `{{token}}` it.
- ✅ **Assertions (Tests tab)** — pass/fail rules against the response: status equals/`>`/`<`, header exists, body-path equals / contains / matches `^2..$`. Result-dot per row (green / red / amber); toast summarizes after each send.

### Responses
- 📊 Status pill + response time + size rendered inline with the Body/Headers tab row, on the right (Postman-style: `Body  Headers  JSON Tree Raw  [🔍][📋][💾]    200 OK · 54 ms · 434 B`)
- 🛈 **Size hover tooltip** — breakdown of response headers/body and request headers/body bytes
- 🛈 **Time hover tooltip** — gantt-style phase breakdown: Prepare · Waiting (TTFB) · Download
- 🧩 **Body view modes**: **JSON** (syntax-highlighted code editor with line numbers), **Tree** (collapsible JSON tree with filter + right-click "Copy path"), **Raw** (verbatim) — pills are inline with the section tabs and don't scroll away
- 🔍 **Find in body** — toolbar search icon highlights all matches inline
- 📋 **Copy response body** + 💾 **Save response to file** (Content-Type → file extension auto-suggested)
- 📑 Separate **Body / Headers** tabs; rust-orange accent on header keys
- ⏳ Rust-orange **loading spinner** while the request is in flight
- 🎨 Auto-pretty-printed JSON responses; click into the view to position caret, select, ⌘A / ⌘C

### Cookie jar (per-environment)
- 🪪 **`Set-Cookie` auto-persisted** into the active environment with name/domain/path/expiry tracked
- 🔄 **Auto-replayed** on the next request that matches host (suffix-match) + path (prefix-match) — same model browsers use
- ⏰ Expired cookies (Max-Age / Expires past) pruned automatically; session cookies kept until app quit
- 🔁 Switching active environment swaps the cookie set (Staging cookies don't leak into Prod)

### cURL interop
- 📋 **Copy as cURL** — current request → clipboard as a `curl` command
- 📥 **Paste from cURL** — paste any `curl` command and it becomes a request (method, URL, headers, body, auth, cookies, params — all parsed)
- 💻 **Code snippet panel** — side panel generating `cURL` / Python `requests` / JavaScript `fetch` / HTTPie, with syntax-highlighted code + line numbers and a copy icon

### Collections
- 📚 **Collections & subfolders** — organize requests in nested folders
- ➕ **Inline `+` button** on every folder header — adds a request in one click
- ⋯ **Overflow menu** on every folder header — Open overview · Add request · Add folder · Rename · Duplicate · Delete
- 📖 **Collection overview page** — click "Open overview" to see a dedicated homepage with title, recursive request/folder counts, an editable description, and a clickable request list
- 🪆 **Duplicate** folders recursively (keeps structure, fresh UUIDs) or individual requests
- 💾 **Save draft to any folder** — the save-draft modal shows a full folder tree with search + "New folder"
- 🔄 **Drag to reorder** requests within a folder (drag the row, drop on a new position)
- 🔎 **Search** across request names, URLs, methods, and folder names (⌘K to focus)
- 📤 **Export** all collections as **JSON** or **YAML**
- 📥 **Import** JSON, YAML, or **Postman Collection v2.1** files
- ✏️ Rename via double-click or right-click

### Workflow
- 🎛 **Settings modal** — request timeout, max body size cap (50 MB default; truncates with banner), proxy URL, TLS verification toggle. All persisted to disk.
- 🔌 **Reused HTTP client + tokio runtime** — no per-request connection-pool / runtime spinup; faster repeated sends.
- ⌨️ **⌘P command palette** — fuzzy-find any request across every collection, ↑↓ navigate, Enter to open
- ⌨️ Standard shortcuts: ⌘⏎ Send · ⌘K focus search · ⌘S save draft · F2 rename · Esc dismiss modals
- 🍎 **Native macOS NSMenu bar** (Rusty Requester · File · View · Request · Help) via `muda`; in-window menu on Linux
- ℹ **Help → About** opens a custom modal with creator credit + Contribute / Report-issue links

---

## 🎯 Why Rusty Requester?

Postman is a ~500 MB Electron app that phones home and wants you to log in. Rusty Requester:

| | Postman | Rusty Requester |
|---|---|---|
| RAM | ~500 MB+ | ~10–30 MB |
| Startup | seconds | instant |
| Distribution | Electron bundle | single native binary |
| Storage | cloud-dependent | one local JSON file |
| Tracking | analytics + telemetry | none |

---

## 📥 Install

### One-line install (macOS + Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | bash
```

The installer auto-detects your platform and pulls the matching release
asset:

- **macOS** (universal, Apple Silicon + Intel) — `RustyRequester-vX.Y.Z-macos-universal.dmg`.
  Copies `RustyRequester.app` into `/Applications` (falls back to
  `~/Applications` if the system folder isn't writable), quits any
  running instance, strips the Gatekeeper quarantine attribute, and
  re-registers with Launch Services so Dock / Spotlight pick up the
  new bundle.
- **Linux** (x86_64 glibc) — `RustyRequester-vX.Y.Z-linux-x86_64.tar.gz`.
  Extracts to `~/.local/share/rusty-requester`, symlinks the binary
  into `~/.local/bin`, installs a `.desktop` entry into
  `~/.local/share/applications`. No `sudo`. If `~/.local/bin` isn't on
  your `PATH`, the script tells you how to add it.

Install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | VERSION=v0.3.0 bash
```

macOS: keep the Gatekeeper quarantine attribute (you'll do the
"right-click → Open" dance yourself on first launch):

```bash
curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | SKIP_QUARANTINE_STRIP=1 bash
```

After it finishes, launch:

```bash
open /Applications/RustyRequester.app     # macOS
rusty-requester                           # Linux (if ~/.local/bin on PATH)
```

### Manual install

#### macOS

1. Grab `RustyRequester-vX.Y.Z-macos-universal.dmg` from the
   [Releases page](https://github.com/chud-lori/rusty-requester/releases/latest)
2. Open the `.dmg`, drag **`RustyRequester.app`** onto the
   **`Applications`** shortcut, eject the disk image
3. Launch from Spotlight / Launchpad / `/Applications`

#### Linux (x86_64 glibc)

1. Grab `RustyRequester-vX.Y.Z-linux-x86_64.tar.gz` from the
   [Releases page](https://github.com/chud-lori/rusty-requester/releases/latest)
2. `tar -xzf RustyRequester-*-linux-x86_64.tar.gz`
3. `cd RustyRequester && ./install-local.sh`
4. Run `rusty-requester` (make sure `~/.local/bin` is on your `PATH`)

### First launch — Gatekeeper (macOS)

The app isn't notarised by Apple (no paid developer account), so macOS
will refuse to open it on the first launch with *"can't be opened
because Apple cannot check it for malicious software"* — **unless you
installed with the one-liner above**, which auto-strips the quarantine
flag.

If you used the manual install, work around it once:

- **Right-click** the app → **Open** → confirm in the dialog, **OR**
- **System Settings → Privacy & Security**, scroll down, and click
  **"Open Anyway"** next to the Rusty Requester entry

You only need to do this once.

---

## 📦 Build from source

### Prerequisites
- **Rust 1.70+** — install via [rustup.rs](https://rustup.rs)
- macOS, Linux, or Windows

### Build & run from source

The repo ships with a `Makefile` for the common tasks:

```bash
git clone https://github.com/chud-lori/rusty-requester
cd rusty-requester

make run        # debug build + run
make release    # optimized binary at target/release/rusty-requester
make test       # unit tests
make icon       # regenerate assets/icon.png from scripts/generate_icon.py
make app        # build a macOS .app bundle (in target/bundle/)
make app-install  # build the bundle and copy it to /Applications
make help       # list all targets
```

Or use Cargo directly: `cargo run`, `cargo build --release`, `cargo test`.

### macOS dock / Cmd+Tab icon

The app calls `NSApplication.setApplicationIconImage` and forces
`NSApplicationActivationPolicyRegular` at startup, so even when launched via
`cargo run` from a terminal it gets its own entry in **Cmd+Tab** and the Dock
with the Rusty Requester icon — not the parent terminal's icon. (See
`src/icon.rs::set_macos_dock_icon`.)

For distribution, build a proper `.app` bundle:

```bash
make app                  # creates target/bundle/RustyRequester.app
open target/bundle/RustyRequester.app

# or install once and launch from Spotlight / Launchpad:
make app-install          # copies to /Applications/RustyRequester.app
```

The bundle script uses macOS's built-in `sips` and `iconutil` to convert
`assets/icon.png` into a multi-resolution `AppIcon.icns`, plus writes an
`Info.plist`. The bundle is what you'd ship to other users.

### Build for Apple Silicon / Intel explicitly

```bash
cargo build --release --target aarch64-apple-darwin   # M1/M2/M3
cargo build --release --target x86_64-apple-darwin    # Intel Mac
```

---

## 📖 Usage

### Your first request

1. Click **➕ New Collection** in the sidebar
2. Inside the collection, click **➕ Request** (or **➕ Folder** to nest further)
3. Pick a method, enter a URL, add headers / params / cookies / body / auth as needed
4. Click **Send** (or press **Enter** while in the URL field)

All edits auto-save to local disk as you type.

### Importing from cURL

**Just paste it into the URL bar.** When the field's text starts with `curl `,
the app parses the command and fills in method, URL, headers, params, cookies,
body, and auth automatically. A toast confirms the import.

Supported `curl` flags:

- `-X / --request` — method
- `-H / --header` — headers
- `-d / --data / --data-raw / --data-binary` — body (also implies `POST` if no `-X`)
- `-u / --user` — Basic auth
- `-b / --cookie` — cookies (split into the Cookies list)
- `-A`, `-e`, `--url`, `-G`, `-I`, and line-continuations (`\`)

There's also a **Sidebar → 📥 Import → Paste cURL command…** option for users
who'd rather use a dedicated dialog.

### Exporting as cURL / Python / JavaScript / HTTPie

Click **`</> Code`** next to Send. A **right-side code panel** opens with a
language picker (cURL · Python `requests` · JavaScript `fetch` · HTTPie). The
code respects which headers / params / cookies you've enabled. Click **Copy**
to grab it.

### Importing / exporting collections

- **Sidebar → 📥 Import → Import collection file…** — pick a `.json`, `.yaml`, or `.yml` file. Postman Collection v2.1 files are auto-detected (schema sniffed) and land as one new collection. IDs are regenerated on import so nothing collides with your existing data.
- **Sidebar → 📤 Export → Export all as JSON / YAML…** — dumps every collection into a single file (good for backups).
- **Right-click any collection / folder → Export as JSON / YAML…** — exports just that subtree (this is the "collection-level" export for sharing).

### Environment variables

Use the **Env** dropdown at the top of the sidebar to switch between
environments (e.g. *Local*, *Staging*, *Prod*). Click the **⚙** button next to
it to open the **Environments** manager: add an environment, rename it, and
fill in key/value pairs. Then reference any variable as `{{name}}` anywhere in
your URL, query params, headers, cookies, body, or auth — substitution happens
at send time.

Example: set `host = api.staging.com` and `token = abc123`, then a request URL
of `https://{{host}}/v1/users` with header `Authorization: Bearer {{token}}`
will send to the right host with the right token, and you can switch the whole
thing by changing the active environment.

### Request history

Every send is logged in the **History** tab in the sidebar — method, URL,
status code, response time, and a 256-character preview of the body. The last
200 entries are kept and persisted to disk. Hit **Clear** to wipe the log.

### Body modes

In the request's **Body** tab pick a mode:

- **Raw** — what you'd expect; pair with a `Content-Type` header for JSON / XML / etc.
- **x-www-form-urlencoded** — table of key/value fields, encoded as `key=val&...`.
- **form-data** — same shape but sent as `multipart/form-data` (text fields only for now).
- **GraphQL** — Query + Variables (JSON) editors. Sent as `{ "query": ..., "variables": ... }` with `application/json`.

### Searching

Type in the **🔎 Search** box in the sidebar. It filters by request name, URL, HTTP method, and folder name in real time. Collections auto-expand while searching. Click **✕** to clear.

### Keyboard shortcuts & gestures

- **⌘/Ctrl + Enter** → Send the current request (from anywhere)
- **Enter** (in URL field) → Send request
- **⌘/Ctrl + K** → Focus the sidebar search
- **F2** → Rename the active request (VS Code / Finder convention)
- **Double-click a request** in the sidebar → Inline rename
- **Esc** during rename → Cancel; **Enter** → Save
- **Right-click a request** → Rename / Duplicate / Delete
- **Right-click a collection / folder** → Rename / Add subfolder / Export / Delete

### Where is my data stored?

One JSON file on your machine — nothing leaves it:

- **macOS:** `~/Library/Application Support/rusty-requester/data.json`
- **Linux:** `~/.local/share/rusty-requester/data.json`
- **Windows:** `%LOCALAPPDATA%\rusty-requester\data.json`

Back it up, version-control it, or hand it to a teammate. It's just JSON.

---

## 🎨 UI conventions

### HTTP method colors

The method name next to each request is rendered as colored text (no
filled pill background — matches Postman's current look):

- 🟢 **GET** — patina green (`#86AC71`)
- 🟠 **POST** — amber gold (`#F59E0B`)
- 🟧 **PUT** — deep rust (`#B7410E`)
- 🔴 **DELETE** — crimson (`#DC2626`)
- 🟤 **PATCH** — burnt sienna (`#BA7850`)
- ⚪ **HEAD / OPTIONS** — warm muted (`#7E8391`)

The primary UI accent (selected tab, Send button, focus ring) is the brand
**rust orange** `#CE422B`.

### Status code colors

- 🟢 `2xx` — green
- 🟠 `3xx` — orange
- 🔴 `4xx` / `5xx` — red

### Collection vs folder

Nested folders render with a small painter-drawn folder glyph (outlined
silhouette); top-level collections intentionally omit the glyph so users
can tell the two apart at a glance. The same font-safe approach is used
for the collapse chevron (painter triangle), search / copy / plus /
overflow-menu icons, and all UI tooling — no unicode glyphs, so nothing
renders as a "tofu" square on systems where egui's bundled font lacks
the character.

---

## 🏗️ Architecture

- **`eframe` / `egui`** — immediate-mode native GUI (the whole UI)
- **`reqwest`** — HTTP client
- **`tokio`** — async runtime (used for the single send; runs on a background thread via `poll-promise`)
- **`serde` / `serde_json` / `serde_yaml`** — persistence + import / export
- **`base64`** — Basic auth encoding
- **`rfd`** — native open / save file dialogs
- **`uuid`** — stable IDs for folders / requests

Source layout (~10 KLOC across 18 files):

```
src/
  main.rs              # ApiClient struct + core state + fn main + macOS menu dispatch
  model.rs             # Data types — Request, Folder, Auth, HttpMethod, KvRow,
                       #   ResponseExtractor, ResponseAssertion, AppSettings,
                       #   Environment (with cookie jar), StoredCookie, ...
  theme.rs             # Rust-forge color constants + global egui style
  widgets.rs           # Reusable widgets — kv_table, body view pills, icon painters,
                       #   JSON tree (with right-click "Copy path"), tooltip panels,
                       #   tab pills, drag/drop helpers
  net.rs               # execute_request + reqwest::Client / tokio::Runtime builders +
                       #   URL scheme, error formatting (no UI deps; unit-testable)
  snippet.rs           # Code-snippet generators + syntax highlighters (cURL / JSON /
                       #   Python / JS) with line-number gutter
  extract.rs           # Dot/bracket JSON-path evaluator for extractors (5 tests)
  assertion.rs         # Assertion evaluator (status/header/body × 7 ops) +
                       #   tiny regex engine (no `regex` crate dep) (7 tests)
  cookies.rs           # RFC 6265-ish cookie jar — parse Set-Cookie, host/path
                       #   matching, expiry pruning (8 tests)
  macos_menu.rs        # Native NSMenu via muda — App / File / View / Request /
                       #   Help submenus + ⌘P / ⌘⏎ / ⇧⌘C accelerators
  icon.rs              # App icon loading (texture + window icon + macOS dock icon)
  io/
    mod.rs             # JSON / YAML export + Postman v2.1 import       (unit-tested)
    curl.rs            # cURL tokenizer / parser / builder              (unit-tested)
  ui/
    mod.rs             # submodule wiring
    sidebar.rs         # left panel — collections tree, env picker, history, folder
                       #   row drag-to-reorder, search box
    editor.rs          # central panel — tabs bar, URL bar, request-editor tabs
                       #   (Params/Headers/Cookies/Body/Auth/Tests), collection
                       #   overview page, JWT decoder
    response.rs        # response panel — inline status chips, body toolbar
                       #   (JSON/Tree/Raw), search, copy, save, headers grid,
                       #   loading spinner, empty state
    modals.rs          # env mgr, save-draft folder picker, cURL paste, snippet
                       #   panel, settings, About modal, command palette (⌘P), toast

assets/
  icon.png             # 512×512 generated by scripts/generate_icon.py

scripts/
  deploy.sh            # one-arg release script (version bump + tag + push)
  make_dmg.sh          # DMG packager invoked by `make dmg`
  generate_icon.py
install.sh             # macOS + Linux one-line installer
```

29 unit tests across `extract`, `assertion`, `cookies`, `io`, `io::curl`. Run
`cargo test` to verify everything builds + passes.

---

## 🚀 Releasing (maintainers)

A push of a `v*` git tag triggers
[`.github/workflows/release.yml`](.github/workflows/release.yml), which on
`macos-latest`:

1. Builds the release binary (`cargo build --release`)
2. Wraps it as a `.app` bundle (`make app`)
3. Packages a drag-to-Applications **`.dmg`** (`make dmg` →
   `scripts/make_dmg.sh`)
4. Creates a GitHub Release for the tag and uploads
   `RustyRequester.dmg` as an asset, with auto-generated release notes

### Cutting a release

There's a one-arg deploy script that handles the bump + tag + push flow
with preflight checks (clean tree, on `main`, tag doesn't already exist):

```bash
./scripts/deploy.sh v0.2.0
```

What it does:

1. Validates the tag format (`vX.Y.Z`).
2. Bumps `[package] version` in `Cargo.toml` and `VERSION :=` in
   `Makefile`.
3. Runs `cargo build --release` (to refresh `Cargo.lock`) and
   `cargo test`.
4. Shows the diff and asks for confirmation.
5. Commits `"Release vX.Y.Z"`, creates an annotated tag, pushes `main`
   and the tag.

Pushing the tag triggers the release workflow above.

<details>
<summary>Manual flow (if you don't use the script)</summary>

```bash
$EDITOR Cargo.toml Makefile        # bump [package].version and VERSION
cargo build --release              # refresh Cargo.lock
cargo test
git commit -am "Release v0.2.0"
git tag v0.2.0
git push origin main --tags
```

</details>

### Building a DMG locally (no GitHub needed)

```bash
make dmg              # → target/bundle/RustyRequester.dmg
open target/bundle/RustyRequester.dmg
```

The resulting `.dmg` is exactly what gets shipped — drag the app to
Applications and you're done.

---

## 🗺️ Roadmap

**Shipped through v0.5:**

Foundations
- [x] Full HTTP methods, tabbed editor (Params / Headers / Cookies / Body / Auth / Tests)
- [x] cURL import (paste in URL bar) / export (right-side code panel: cURL, Python, JS, HTTPie) with syntax-highlighted line-numbered code view
- [x] Postman Collection v2.1 import
- [x] JSON / YAML export & import
- [x] Query parameter builder + auto-growing ghost row pattern for all KV tables
- [x] Bearer / Basic auth presets + **JWT decoder** for Bearer tokens
- [x] Body modes: Raw / form-urlencoded / multipart / GraphQL
- [x] Environment variables — `{{var}}` substitution with active env selector + manage modal
- [x] Request history — last 200 sends with preview

Workflow
- [x] Postman-style request tabs (open multiple requests, switch with one click) with hover preview
- [x] Inline rename via double-click; F2 shortcut
- [x] Subfolders; collection overview page; inline folder `+`/`⋯` toolbar; duplicate folder recursively; save-draft folder picker with inline "New folder"
- [x] Drag-to-reorder requests within a folder
- [x] Search across requests, URLs, methods, folder names (⌘K)
- [x] **⌘P command palette** — fuzzy-find any request and jump

Response viewing
- [x] **Body view modes**: JSON (syntax-highlighted code, line numbers), Tree (collapsible JSON tree with **right-click "Copy path"**), Raw
- [x] Find-in-body, copy-body, **save-response-to-file**
- [x] Response info chips (status / time / size) **inline** with the Body/Headers tab row; tooltips for size & time breakdowns
- [x] Loading spinner; readable error chains (no more opaque "builder error")
- [x] Auto-pretty-printed JSON; click-and-drag selection works in read-only views

Testing
- [x] **Post-response extractors** — dot/bracket JSON path → env variables for chaining
- [x] **Assertions** — status / header / body × equals · contains · matches `^2..$` · exists · `>` · `<`. Per-row pass/fail dot.

Networking & safety
- [x] **Settings modal** — request timeout, max body cap (streaming + truncation banner), proxy URL, TLS verification toggle
- [x] **Reused `reqwest::Client` + `tokio::Runtime`** across sends — no per-request spinup cost
- [x] **Cookie jar (per-environment)** — `Set-Cookie` parsed, host/path-matched on next send, expiry-aware

Platform
- [x] macOS + Linux one-line installer (`install.sh`); auto-detects platform
- [x] Universal macOS DMG (Apple Silicon + Intel) and Linux x86_64 tarball built by GitHub Actions
- [x] **Native macOS NSMenu bar** (App / File / View / Request / Help) with ⌘P, ⌘⏎, ⇧⌘C accelerators
- [x] Custom **About modal** with creator credit + Contribute / Report-issue links

**In progress / planned:**

- [ ] **WebSocket testing** — needs a separate connection lifecycle and message-log UI;
      plan is to add a `RequestKind::WebSocket` mode using `tokio-tungstenite` with a
      send box + scrolling message log per request.
- [ ] **OAuth 2.0** flows (Authorization Code + PKCE, Client Credentials, refresh
      token). Probably the highest-impact next addition for users hitting auth-gated
      APIs; tokens would slot directly into the existing env / extractor system.
- [ ] **Pre-request scripts** — full JS-style scripts via Rhai or Boa; the lightweight
      post-response extractors + assertions cover ~80% of what users reach for, but
      pre-request transformations need an embedded scripting engine.
- [ ] **Server-Sent Events (SSE)** support — handy for the LLM-streaming endpoints
      that have become common.
- [ ] **Response diff** — "send twice, diff me" between two recent responses; useful
      regression-test feature unique to this app.
- [ ] **Light theme** — the dark theme is opinionated; a parallel light palette would
      be straightforward but require touching every color constant.

---

## 🛡 Compatibility & stability

Rusty Requester follows [Semantic Versioning](https://semver.org/).

**Pre-1.0 (current).** Format additions are guarded with `#[serde(default)]`
so old `data.json` files load cleanly into newer builds, but breaking
changes (renames, type widening, removed fields) can still happen at any
minor bump if necessary. Don't pin a specific feature shape across 0.x
releases.

**From 1.0 onward.** The following are stable across the entire `1.x`
series — anything breaking them requires a major-version bump (`2.0`)
with a documented migration path:

- **`data.json` schema** — the on-disk format at
  `~/Library/Application Support/rusty-requester/data.json` (macOS) /
  `~/.local/share/rusty-requester/data.json` (Linux). New fields can be
  added; existing fields keep their names, types, and serde tags.
- **Install paths and bundle identity** — `/Applications/RustyRequester.app`,
  bundle ID `com.rustyrequester.app`, fallback `~/Applications/`,
  Linux `~/.local/share/rusty-requester/` and `~/.local/bin/rusty-requester`.
- **CLI surface** — `--version` / `-V` flag, environment variables read by
  `install.sh` (`VERSION`, `SKIP_QUARANTINE_STRIP`, `RUSTY_REPO`).
- **Import / export formats** — JSON / YAML round-trip, Postman
  Collection v2.1 import.
- **Public macOS menu shortcuts** — `⌘⏎` Send, `⌘P` Command palette,
  `⌘K` Sidebar search, `⇧⌘C` toggle snippet panel.

Anything *not* in this list (UI layout, internal module structure, exact
binary size, theme colors, syntax-highlight palette, behind-the-scenes
HTTP timing measurement, etc.) is implementation detail and can change
in any release.

See [`CHANGELOG.md`](./CHANGELOG.md) for what's shipped.

---

## 🤝 Contributing

1. Fork the repo
2. `git checkout -b feature/my-thing`
3. `cargo test` to make sure nothing broke
4. Commit, push, open a PR

---

## 📝 License

MIT — see [`LICENSE`](./LICENSE).

## 📬 Contact

Created by [@chud-lori](https://github.com/chud-lori).
