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

## 🎯 Why Rusty Requester?

Postman is a ~500 MB Electron app that phones home and wants you to log in. Rusty Requester:

| | Postman | Rusty Requester |
|---|---|---|
| RAM | ~500 MB+ | ~10–30 MB |
| Startup | seconds | instant |
| Distribution | Electron bundle | single native binary |
| Storage | cloud-dependent | one local JSON file |
| Tracking | analytics + telemetry | none |

Highlights: tabbed request editor, per-environment variables + cookie jar,
Postman Collection v2.1 import, syntax-highlighted JSON / Tree / HTML /
SSE views, **Server-Sent Events streaming** for LLM APIs, **Cancel** mid-
flight, **Response diff** across sends, **⌘P request finder** + **⇧⌘P
actions palette**, and a native macOS menu bar. Full catalog in
[`docs/FEATURES.md`](docs/FEATURES.md).

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

**macOS** — grab `RustyRequester-vX.Y.Z-macos-universal.dmg` from the
[Releases page](https://github.com/chud-lori/rusty-requester/releases/latest),
open the `.dmg`, drag **`RustyRequester.app`** onto the **`Applications`**
shortcut, eject the disk image.

**Linux** (x86_64 glibc) — grab `RustyRequester-vX.Y.Z-linux-x86_64.tar.gz`,
extract it, run `./install-local.sh` inside, then `rusty-requester`
(ensure `~/.local/bin` is on your `PATH`).

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

### Build from source

```bash
git clone https://github.com/chud-lori/rusty-requester
cd rusty-requester

make run        # debug build + run
make release    # optimized binary at target/release/rusty-requester
make app        # build a macOS .app bundle (in target/bundle/)
make app-install  # build the bundle and copy it to /Applications
make help       # list all targets
```

Or use Cargo directly: `cargo run`, `cargo build --release`, `cargo test`.
Requires **Rust 1.70+** — install via [rustup.rs](https://rustup.rs).

---

## 🚀 Quickstart

1. Click **➕ New Collection** in the sidebar.
2. Inside the collection, click **➕ Request**.
3. Pick a method, enter a URL (paste a `curl` command instead — it'll auto-fill method, headers, body, auth, etc.).
4. Click **Send** (or press **⌘/Ctrl + Enter**).

All edits auto-save to a single local JSON file — nothing leaves your machine:

- **macOS:** `~/Library/Application Support/rusty-requester/data.json`
- **Linux:** `~/.local/share/rusty-requester/data.json`
- **Windows:** `%LOCALAPPDATA%\rusty-requester\data.json`

> **Security note:** `data.json` is a plaintext file holding your
> requests **and any tokens / passwords you put into Auth or
> Environment variables**. Rusty Requester trusts local disk
> permissions to protect it — `0600` on Unix by default because it
> lives under your home directory. Don't commit `data.json` to a
> repo, don't share it with anyone you wouldn't share your tokens
> with, and consider symlinking it onto an encrypted volume if your
> setup warrants it. Native-keychain integration is on the post-1.0
> roadmap.

### Useful shortcuts

**⌘⏎** Send · **⌘N** New request · **⌘W** Close tab · **⌘D** Duplicate tab · **⌘K** Focus search · **⌘P** Command palette · **⇧⌘P** Actions palette · **F2** Rename · **Esc** Dismiss modals

The ⇧⌘P actions palette is self-discoverable — open it and start
typing to see every available action.

Full usage guide, body modes, environment-variable examples, import /
export, and UI conventions in [`docs/FEATURES.md`](docs/FEATURES.md).

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

- **`data.json` schema** — the on-disk format. New fields can be added;
  existing fields keep their names, types, and serde tags.
- **Install paths and bundle identity** — `/Applications/RustyRequester.app`,
  bundle ID `com.rustyrequester.app`, fallback `~/Applications/`,
  Linux `~/.local/share/rusty-requester/` and `~/.local/bin/rusty-requester`.
- **CLI surface** — `--version` / `-V` flag, environment variables read by
  `install.sh` (`VERSION`, `SKIP_QUARANTINE_STRIP`, `RUSTY_REPO`).
- **Import / export formats** — JSON / YAML round-trip, Postman
  Collection v2.1 import.
- **Public macOS menu shortcuts** — `⌘⏎` Send, `⌘N` New request,
  `⌘W` Close tab, `⌘D` Duplicate tab, `⌘P` Command palette,
  `⌘K` Sidebar search, `⌘S` Save draft, `⇧⌘C` toggle snippet panel.

Anything *not* in this list (UI layout, internal module structure, exact
binary size, theme colors, syntax-highlight palette, behind-the-scenes
HTTP timing measurement, etc.) is implementation detail and can change
in any release.

See [`CHANGELOG.md`](./CHANGELOG.md) for what's shipped.

---

## 📚 Docs

- [`docs/FEATURES.md`](docs/FEATURES.md) — full feature list, usage walkthroughs, UI conventions, roadmap
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — dependencies, source layout, internal design, release flow
- [`CHANGELOG.md`](CHANGELOG.md) — version history

---

## 🤝 Contributing

1. Fork the repo
2. `git checkout -b feature/my-thing`
3. `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --all -- --check`
4. Commit, push, open a PR

---

## 📝 License

MIT — see [`LICENSE`](./LICENSE).

## 📬 Contact

Created by [@chud-lori](https://github.com/chud-lori).
