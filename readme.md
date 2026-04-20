<h1 align="center">
  <img src="assets/icon.png" width="96" alt="Rusty Requester" /><br/>
  Rusty Requester
</h1>

<p align="center">
A <b>native, offline, lightweight</b> API client built with Rust and <code>egui</code> —
a Postman alternative that doesn't chew through hundreds of MB of RAM just to make HTTP requests.
</p>

<p align="center">
<i>Why "Rusty"?</i> It's a double pun on <b>Rust</b> (the language) and
<b>rust-as-in-old-stuff-that-still-works</b>. Plenty of developers
are on older / low-spec machines that can't stomach a 500 MB Electron
app with half a gig of idle RAM — so this is built for them first.
~15 MB binary, ~30 MB idle RAM, &lt;100 ms cold start.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/mac%20os-000000?style=for-the-badge&logo=macos&logoColor=F0F0F0" alt="macOS" />
  <img src="https://img.shields.io/badge/linux-000000?style=for-the-badge&logo=linux&logoColor=F0F0F0" alt="Linux" />
</p>

---

## 🎯 Why Rusty Requester?

Most API clients today are Electron apps — Chromium + Node wrapped
around a form builder. That buys you cross-platform consistency at the
cost of hundreds of MB of RAM and a supply chain with thousands of npm
packages. Rusty Requester is a single ~15 MB native binary: Rust +
`egui`, no webview, no Node, no account.

### vs Postman / Insomnia / Bruno

|                   | Postman | Insomnia | Bruno | **Rusty Requester** |
|-------------------|---------|----------|-------|---------------------|
| Runtime           | Electron | Electron | Electron | **Rust + egui (native)** |
| Download size     | ~500 MB | ~200 MB | ~200 MB | **~15 MB** |
| Idle RAM          | 400–800 MB | 300–500 MB | 200–400 MB | **~30 MB** |
| Cold start        | 2–5 s | 1–3 s | 1–2 s | **<100 ms** |
| Account required  | yes (for sync) | yes (gated 2023) | no | **no** |
| Telemetry         | yes | yes | off by default | **none** |
| Storage           | cloud-first | cloud or local | git-native files | **one local JSON file** |
| Supply chain      | ~1000+ npm deps | ~1000+ npm deps | ~800 npm deps | **~150 Rust crates** |
| Response HTML     | Chromium webview | Chromium webview | Chromium webview | **egui text/markup — no JS engine** |

Bruno is the closest match in spirit (offline, file-based, OSS) — it's
a good product. The differentiator is runtime: it still ships Chromium.

### Why Rust for an API client?

- **Memory safety.** A malformed response can't buffer-overflow the
  parser the way a C client could. Rust's bounds checks and
  borrow-checker eliminate a whole class of CVE.
- **No JS runtime means no JS CVEs.** Response HTML renders as markup
  in `egui`, not in a Chromium webview. A hostile server can't hit
  you with a V8 exploit because there's no V8.
- **Smaller attack surface.** ~150 Rust crates vs ~1000+ npm packages
  per Electron competitor. Fewer transitive deps = fewer places for a
  supply-chain compromise to land.
- **Well-audited networking.** `reqwest` + `rustls` (or `native-tls`)
  handle TLS and redirects — both heavily used across the Rust
  ecosystem.
- **Honest caveat.** Rust isn't magically safe from supply-chain
  attacks. We mitigate with `Cargo.lock` pinning, sticking to
  widely-used crates (`reqwest`, `tokio`, `serde`, `egui`), and
  running `cargo audit` before every release — but a compromised
  upstream would still bite us.

### Highlights

Tabbed request editor, per-environment variables + cookie jar,
Postman Collection v2.1 import, syntax-highlighted JSON / Tree / HTML /
SSE views, **Server-Sent Events streaming** for LLM APIs, **Cancel**
mid-flight, **Response diff** across sends, **⌘P request finder** +
**⇧⌘P actions palette**, and a native macOS menu bar. Full catalog in
[`docs/FEATURES.md`](docs/FEATURES.md).

---

## 🔐 Security

An API client lives on a trust boundary — you type a URL, a stranger's
server sends bytes back. We treat that boundary seriously. This is the
threat model, stated plainly.

### What a hostile server CAN'T do to you

- **No auto-download.** Response bytes never touch disk on their own.
  Every "save response" goes through the OS save dialog (`rfd`) where
  *you* pick the path and filename. There is no `Content-Disposition`
  auto-save path — a server cannot write `~/.ssh/authorized_keys` or
  drop a binary into your Startup folder.
- **No code execution on response content.** JSON / HTML / XML / SSE
  are parsed as data. The HTML preview renders as markup in `egui` —
  no DOM, no JavaScript engine, no MIME sniffing. A response
  containing `<script>` just shows the tag as text.
- **No memory-corruption path.** HTTP and TLS are handled by
  `reqwest` → `hyper` → `rustls` / `native-tls`. All safe Rust (or
  OS-audited for native-tls on macOS). A malformed chunked-transfer
  body, oversized header, or junk TLS record can't buffer-overflow
  the client the way a C-based user agent could.
- **No shell execution on curl import.** Pasting a `curl` command
  parses its flags as data — we never `exec` the command. Worst case
  is the same as typing the URL yourself.

### What you ARE still responsible for

- **SSRF from your own machine.** If you send a request to
  `http://localhost:6379`, an internal IP, or `file://` (where
  supported), we'll do it — same as `curl`. The app is a hardened
  boundary on *inbound* bytes, not a policy engine on *outbound*
  destinations. Don't blindly send requests from URLs you haven't
  read.
- **Saved-then-opened files.** If you explicitly save a response and
  then open that file in a vulnerable downstream app (Preview, an
  editor plugin, a media player), that's on the downstream app. We
  don't auto-open, don't `chmod +x`, and don't set the
  `com.apple.quarantine` bypass on anything we write.
- **Local `data.json`.** Your tokens, env vars, and cookies live in a
  plaintext JSON file under your home directory at `0600` perms. If
  someone has shell access as your user, they can read it. See the
  *Security note* under [Quickstart](#-quickstart).
- **Supply chain on our deps.** Rust isn't magic. A compromised
  upstream (`reqwest`, `tokio`, `serde`, `egui`) would ship in our
  binary. We mitigate with pinned `Cargo.lock`, widely-used crates
  only, and `cargo audit` before each release — but we can't
  eliminate the risk.

### Reporting vulnerabilities

Found a security issue? Open a **private** security advisory via
GitHub: **Security → Report a vulnerability** on the repo. Please
don't file a public issue for exploitable bugs.

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
- **Linux** (x86_64 glibc 2.35+, so Ubuntu 22.04 / Debian 12 /
  Fedora 36+ / RHEL 9 and newer) —
  `RustyRequester-vX.Y.Z-linux-x86_64.tar.gz`. Installs the binary
  directly at `~/.local/bin/rusty-requester`, drops a `.desktop`
  entry into `~/.local/share/applications`, and puts icons in both
  `hicolor/512x512/apps/` and `pixmaps/`. User data lives
  separately at `~/.local/share/rusty-requester/data.json` so the
  two never get tangled. No `sudo`. If `~/.local/bin` isn't on your
  `PATH`, the script tells you how to add it.

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

### Uninstall

The same one-liner in `UNINSTALL=1` mode removes the app and
preserves your `data.json` (collections, history, OAuth tokens):

```bash
curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | UNINSTALL=1 bash
```

Add `PURGE=1` to wipe user data too:

```bash
curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | UNINSTALL=1 PURGE=1 bash
```

Or, if you still have the extracted Linux tarball around, run
`./uninstall-local.sh` (pass `--purge` to also delete data). macOS:
the one-liner removes `RustyRequester.app` from `/Applications`
(or `~/Applications`), quits any running instance, and
— with `PURGE=1` — clears
`~/Library/Application Support/rusty-requester`.

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
Requires **Rust 1.73+** — install via [rustup.rs](https://rustup.rs).

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

**⌘⏎** Send · **⌘N** New request · **⌘W** Close tab · **⌘D** Duplicate tab · **⌘F** Find in response · **⌘K** Focus search · **⌘P** Command palette · **⇧⌘P** Actions palette · **F2** Rename · **Esc** Dismiss modals

(Use **Ctrl** instead of **⌘** on Linux / Windows — the app binds both.)

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
  Linux binary at `~/.local/bin/rusty-requester` (distinct from
  the user-data dir `~/.local/share/rusty-requester/` which holds
  `data.json`).
- **CLI surface** — `--version` / `-V` flag, environment variables read by
  `install.sh` (`VERSION`, `SKIP_QUARANTINE_STRIP`, `RUSTY_REPO`,
  `UNINSTALL`, `PURGE`).
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
