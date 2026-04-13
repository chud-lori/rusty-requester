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

```bash
# Clone
git clone https://github.com/chud-lori/rusty-requester
cd rusty-requester

# Run in debug mode (fast to compile, great for iteration)
cargo run

# Or build an optimized release binary
cargo build --release
./target/release/rusty-requester
```

### Build for Apple Silicon (M1/M2/M3)

```bash
cargo build --release --target aarch64-apple-darwin
# Binary at: target/aarch64-apple-darwin/release/rusty-requester
```

### Build for Intel Mac

```bash
cargo build --release --target x86_64-apple-darwin
```

### Run the tests

```bash
cargo test
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

Click **📥 Paste** next to Send (or **📥 Import → Paste cURL command…** in the sidebar), paste a `curl` command, and Import. Supports:

- `-X / --request` — method
- `-H / --header` — headers
- `-d / --data / --data-raw / --data-binary` — body (and implies `POST` if method not given)
- `-u / --user` — Basic auth
- `-b / --cookie` — cookies (split into the Cookies list)
- `-A`, `-e`, `--url`, `-G`, `-I`, and line-continuations (`\`)

### Exporting as cURL

Click **📋 cURL** next to Send. The full request is copied to your clipboard, ready to paste into a terminal or a docs snippet.

### Importing / exporting collections

- **Sidebar → 📥 Import → Import collection file…** — pick a `.json`, `.yaml`, or `.yml` file. Postman Collection v2.1 files are auto-detected (schema sniffed) and land as one new collection. IDs are regenerated on import so nothing collides with your existing data.
- **Sidebar → 📤 Export → Export all as JSON / YAML…** — dumps every collection into a single file (good for backups).
- **Right-click any collection / folder → Export as JSON / YAML…** — exports just that subtree (this is the "collection-level" export for sharing).

### Searching

Type in the **🔎 Search** box in the sidebar. It filters by request name, URL, HTTP method, and folder name in real time. Collections auto-expand while searching. Click **✕** to clear.

### Keyboard shortcuts

- **⌘/Ctrl + Enter** → Send the current request (from anywhere)
- **Enter** (while focused in the URL field) → Send request
- **⌘/Ctrl + K** → Focus the sidebar search
- **Right-click collection / folder** → Rename / Add subfolder / Export / Delete
- **Right-click request** → Duplicate / Delete

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
  main.rs   # app state, UI, send pipeline
  curl.rs   # cURL tokenizer, parser, builder  (unit-tested)
  io.rs     # JSON/YAML export, JSON/YAML/Postman import  (unit-tested)
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

MIT — see `LICENSE`.

## 🙏 Acknowledgments

- [egui](https://github.com/emilk/egui) by Emil Ernerfeldt — the reason this is so fast
- [Tokyo Night](https://github.com/enkia/tokyo-night-vscode-theme) — color palette inspiration

## 📬 Contact

Created by [@chud-lori](https://github.com/chud-lori).

---

**Made with 🦀 and ❤️ in Rust**
