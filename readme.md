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
- 🎨 **Tokyo-Night-inspired dark UI** with colored HTTP-method pills and underlined tabs
- 🍎 Builds for Apple Silicon, Intel Mac, Linux, and Windows

### Request building
- 🔧 Full HTTP methods: `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`
- 📝 Tabbed editor: **Params · Headers · Cookies · Body · Auth**
- 🔗 Query-param builder with live "final URL" preview
- 🍪 Cookies list (merged into a `Cookie` header on send)
- 🔐 Auth presets: **No Auth · Bearer Token · Basic Auth**
- ✨ **Prettify / Minify** JSON body (one-click)

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

## 📦 Installation

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

### macOS app bundle (so the dock icon works)

When you run via `cargo run` from a terminal, macOS shows the parent
**Terminal** icon in the Dock and Cmd+Tab — this is a macOS limitation, not
something we can change at runtime. To get the proper icon, build a `.app`
bundle:

```bash
make app                  # creates target/bundle/RustyRequester.app
open target/bundle/RustyRequester.app

# or install once and launch from Spotlight / Launchpad:
make app-install          # copies to /Applications/RustyRequester.app
```

The bundle script uses macOS's built-in `sips` and `iconutil` to convert
`assets/icon.png` into a multi-resolution `AppIcon.icns`.

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

### Searching

Type in the **🔎 Search** box in the sidebar. It filters by request name, URL, HTTP method, and folder name in real time. Collections auto-expand while searching. Click **✕** to clear.

### Keyboard shortcuts & gestures

- **⌘/Ctrl + Enter** → Send the current request (from anywhere)
- **Enter** (in URL field) → Send request
- **⌘/Ctrl + K** → Focus the sidebar search
- **Double-click a request** in the sidebar → Inline rename
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

Each request row shows a compact colored pill with the method:

- 🟢 **GET** — green
- 🟠 **POST** — orange
- 🔵 **PUT** — blue (accent)
- 🩷 **DELETE** — pink
- 🟣 **PATCH** — purple
- ⚪ **HEAD / OPTIONS** — muted

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
  theme.rs      # Tokyo-Night color constants + global egui style
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

## 🗺️ Roadmap

- [x] cURL import / export
- [x] Postman Collection v2.1 import
- [x] JSON / YAML export & import
- [x] Query parameter builder
- [x] Cookies tab
- [x] Bearer / Basic auth presets
- [x] Response headers viewer
- [x] Search
- [x] Subfolders
- [ ] Request history
- [ ] Environment variables
- [ ] Request chaining / pre-request scripts
- [ ] GraphQL support
- [ ] WebSocket testing
- [ ] Form-data / urlencoded body modes

---

## 🤝 Contributing

1. Fork the repo
2. `git checkout -b feature/my-thing`
3. `cargo test` to make sure nothing broke
4. Commit, push, open a PR

---

## 📝 License

MIT — see [`LICENSE`](./LICENSE).

## 🙏 Acknowledgments

- [egui](https://github.com/emilk/egui) by Emil Ernerfeldt — the reason this is so fast
- [Tokyo Night](https://github.com/enkia/tokyo-night-vscode-theme) — color palette inspiration

## 📬 Contact

Created by [@chud-lori](https://github.com/chud-lori).
