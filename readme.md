# 🦀 Rusty Requester

A **native, offline, lightweight** API client built with Rust and `egui` — a Postman alternative that doesn't chew through hundreds of MB of RAM just to make HTTP requests.

Vibe-coded because I got tired of Postman's bloat and cloud sync I never wanted, and tired of managing a wall of raw `curl` commands in my terminal.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![macOS](https://img.shields.io/badge/mac%20os-000000?style=for-the-badge&logo=macos&logoColor=F0F0F0)

---

## ✨ Features

### Core
- 🚀 **Truly native** — Rust + `egui`, no Electron, no Chromium
- 💾 **Fully offline** — all data lives in one local JSON file, no cloud sync, no telemetry
- 🎨 **Rust-forge dark UI** — warm copper / rust-orange / amber palette with colored HTTP-method pills and underlined tabs
- 🍎 Builds for Apple Silicon, Intel Mac, Linux, and Windows

### Request building
- 🔧 Full HTTP methods: `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`
- 📝 Tabbed editor: **Params · Headers · Cookies · Body · Auth**
- 🔗 Query-param builder with live "final URL" preview
- 🍪 Cookies list (merged into a `Cookie` header on send)
- 🔐 Auth presets: **No Auth · Bearer Token · Basic Auth**
- 📦 **Body modes**: Raw / `x-www-form-urlencoded` / `multipart/form-data` / **GraphQL** (query + variables, sent as `application/json`)
- ✨ **Prettify / Minify** JSON body (one-click)
- 🌱 **Environment variables** — define key/value vars per environment, reference them anywhere with `{{varname}}`. Switch active env from the sidebar.
- 📜 **Request history** — every send is logged (method, URL, status, time, response preview); browse the last 200 from the sidebar's History tab.

### Responses
- 📊 Status code, response time, and **all response headers** displayed
- 🎨 Auto-pretty-printed JSON responses
- 📋 One-click copy of the full response
- Body / Headers tabs

### cURL interop
- 📋 **Copy as cURL** — current request → clipboard as a `curl` command
- 📥 **Paste from cURL** — paste any `curl` command and it becomes a request (method, URL, headers, body, auth, cookies, params — all parsed)

### Collections
- 📚 **Collections & subfolders** — organize requests in nested folders
- 🔎 **Search** across request names, URLs, methods, and folder names (⌘K to focus)
- 📤 **Export** a single collection or all collections as **JSON** or **YAML**
- 📥 **Import** JSON, YAML, or **Postman Collection v2.1** files
- 📋 **Duplicate** a request via right-click
- ✏️ Rename via right-click

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

## 📥 Install (macOS)

### One-line install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | bash
```

The script pulls the latest `RustyRequester.dmg` from GitHub Releases,
copies the app into `/Applications` (falls back to `~/Applications` if
the system folder isn't writable), and strips the Gatekeeper quarantine
attribute so the first launch "just works."

Install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | VERSION=v0.2.0 bash
```

Leave the quarantine attribute intact (you'll do the "right-click → Open"
dance yourself on first launch):

```bash
curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | SKIP_QUARANTINE_STRIP=1 bash
```

After it finishes, launch from Spotlight, Launchpad, or:

```bash
open /Applications/RustyRequester.app
```

### Manual install (the old way)

1. Grab `RustyRequester.dmg` from the
   [Releases page](https://github.com/chud-lori/rusty-requester/releases/latest)
2. Open the `.dmg`, drag **`RustyRequester.app`** onto the
   **`Applications`** shortcut, eject the disk image
3. Launch from Spotlight / Launchpad / `/Applications`

### First launch — Gatekeeper

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

### HTTP method pills

Each request row / tab shows a compact colored pill with the method,
matching the rust-forge palette:

- 🟢 **GET** — patina green (`#86AC71`)
- 🟠 **POST** — amber gold (`#F59E0B`)
- 🟧 **PUT** — deep rust (`#B7410E`)
- 🔴 **DELETE** — crimson (`#DC2626`)
- 🟤 **PATCH** — burnt sienna (`#BA7850`)
- ⚪ **HEAD / OPTIONS** — warm muted (`#8A735F`)

The primary UI accent (selected tab, Send button, focus ring) is the brand
**rust orange** `#CE422B`.

### Status code colors

- 🟢 `2xx` — green
- 🟠 `3xx` — orange
- 🔴 `4xx` / `5xx` — red

### Icons

- 📚 Collection (top-level)
- 📁 Folder (nested)

---

## 🏗️ Architecture

- **`eframe` / `egui`** — immediate-mode native GUI (the whole UI)
- **`reqwest`** — HTTP client
- **`tokio`** — async runtime (used for the single send; runs on a background thread via `poll-promise`)
- **`serde` / `serde_json` / `serde_yaml`** — persistence + import / export
- **`base64`** — Basic auth encoding
- **`rfd`** — native open / save file dialogs
- **`uuid`** — stable IDs for folders / requests

Source layout:

```
src/
  main.rs       # ApiClient state + UI rendering + main()
  model.rs      # Data types (Request, KvRow, Auth, HttpMethod, Folder, ...)
  theme.rs      # Rust-forge color constants + global egui style
  widgets.rs    # Reusable widgets (kv_table, tab pill, paint_x, ...)
  snippet.rs    # Code-snippet generators (cURL, Python, JS, HTTPie)
  icon.rs      # App icon loading (texture + window icon)
  curl.rs       # cURL tokenizer / parser / builder       (unit-tested)
  io.rs         # JSON / YAML export + Postman v2.1 import (unit-tested)

assets/
  icon.png      # 512×512 generated by scripts/generate_icon.py

scripts/
  generate_icon.py
```

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

```bash
# 1. Bump the version in Cargo.toml + Makefile (VERSION := ...)
$EDITOR Cargo.toml Makefile

# 2. Commit + tag
git commit -am "Release v0.2.0"
git tag v0.2.0
git push origin main --tags

# 3. Watch GitHub Actions build + publish the release
#    (or run the workflow manually from the Actions tab)
```

### Building a DMG locally (no GitHub needed)

```bash
make dmg              # → target/bundle/RustyRequester.dmg
open target/bundle/RustyRequester.dmg
```

The resulting `.dmg` is exactly what gets shipped — drag the app to
Applications and you're done.

---

## 🗺️ Roadmap

**Shipped:**

- [x] cURL import (paste in URL bar) / export (right-side code panel: cURL, Python, JS, HTTPie)
- [x] Postman Collection v2.1 import
- [x] JSON / YAML export & import (per-collection or all)
- [x] Query parameter builder (table layout with enable/disable + description)
- [x] Cookies tab
- [x] Bearer / Basic auth presets
- [x] Response headers viewer (separate Body / Headers tabs)
- [x] Search across requests, URLs, methods, folder names
- [x] Subfolders
- [x] **Form-data / urlencoded body modes**
- [x] **GraphQL body mode** — query + variables, sent as `application/json`
- [x] **Environment variables** — `{{var}}` substitution with active env selector + manage modal
- [x] **Request history** — last 200 sends with preview, accessible from sidebar
- [x] Postman-style request tabs (open multiple requests, switch with one click)
- [x] Inline rename via double-click

**In progress / planned:**

- [ ] **WebSocket testing** — needs a separate connection lifecycle and message-log UI;
      the plan is to add a `RequestKind::WebSocket` mode using `tokio-tungstenite` with a
      send box + scrolling message log per request.
- [ ] **Request chaining / pre-request scripts** — needs an embedded scripting engine.
      The lightweight first step is *post-response extractors*: simple JSONPath / regex
      rules that capture a value from a response into an environment variable, so the
      next request can `{{token}}` it. Full JS-style scripts (with Rhai or Boa) are
      a stretch goal.

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
