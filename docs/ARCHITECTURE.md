# Architecture

Internal design notes for contributors. For end-user features see
[`FEATURES.md`](./FEATURES.md).

## Dependencies

- **`eframe` / `egui`** — immediate-mode native GUI (the whole UI)
- **`egui-phosphor`** — embedded Phosphor icon font (1,200+ glyphs as typed constants)
- **`reqwest`** — HTTP client
- **`tokio`** — async runtime. Sends spawn onto a long-lived multi-thread runtime as `JoinHandle`s so **Cancel** can `.abort()` mid-flight; the result flows back through an `mpsc::channel` the UI polls.
- **`serde` / `serde_json` / `serde_yaml`** — persistence + import / export
- **`muda`** — native macOS NSMenu bar
- **`base64`** — Basic auth encoding
- **`sha2`** — S256 PKCE challenge for OAuth 2.0 (pure-Rust, ~40 KB, added v0.15)
- **`rfd`** — native open / save file dialogs
- **`uuid`** — stable IDs for folders / requests

## Source layout

Version tags below mark when each module was *introduced*. Files without
a tag were part of the initial cut (v0.1).

```
src/
  main.rs              # ApiClient struct + core state + fn main + macOS menu dispatch
  model.rs             # Data types — Request, Folder, Auth, HttpMethod, KvRow,
                       #   ResponseExtractor, ResponseAssertion, AppSettings,
                       #   Environment (with cookie jar), StoredCookie, OpenTab, ...
  theme.rs             # Color constants + global egui style + Phosphor font
                       #   registration + theme-aware palette (Dark / Light /
                       #   Postman) + `hint()` styled-placeholder helper
                       #   (Light theme v0.14, Postman theme v0.18)
  widgets.rs           # Reusable widgets — kv_table, body view pills, icon button,
                       #   JSON tree (with right-click "Copy path"), tooltip panels,
                       #   tab strip (with pin/duplicate/close actions), animated
                       #   folder chevron
  net.rs               # execute_request + reqwest::Client / tokio::Runtime builders +
                       #   URL scheme, error formatting, RequestUpdate enum for
                       #   streaming responses (Progress/Final), SSE streaming path
  sse.rs               # SSE wire-format parser + event formatter (8 tests, zero deps)
                       #   (v0.10)
  diff.rs              # LCS-based line diff for the response Diff view (5 tests)
                       #   (v0.11)
  snippet.rs           # Code-snippet generators + syntax highlighters (cURL / JSON /
                       #   Python / JS / HTTPie) with line-number gutter
  html_preview.rs      # Minimal HTML → readable-text renderer for the Preview body
                       #   view (strips script/style, decodes entities; 5 tests)
                       #   (v0.7)
  actions.rs           # ⇧⌘P actions palette enum + labels/keywords/shortcut metadata
                       #   (v0.11)
  extract.rs           # Dot/bracket JSON-path evaluator for extractors (5 tests)
                       #   (v0.4)
  assertion.rs         # Assertion evaluator (status/header/body × 7 ops) +
                       #   tiny regex engine (no `regex` crate dep) (7 tests)
                       #   (v0.4)
  cookies.rs           # RFC 6265-ish cookie jar — parse Set-Cookie, host/path
                       #   matching, expiry pruning (8 tests)
                       #   (v0.5)
  oauth.rs             # OAuth 2.0 Authorization Code + PKCE — loopback listener,
                       #   S256 challenge (RFC 7636), token exchange,
                       #   token caching + masked preview (6 tests; `sha2` dep)
                       #   (v0.15)
  macos_menu.rs        # Native NSMenu via muda — App / File / View / Request /
                       #   Help submenus + ⌘N / ⌘W / ⌘D / ⌘P / ⇧⌘P / ⌘⏎ / ⇧⌘C
                       #   accelerators + native title-bar integration
                       #   (v0.3; title-bar integration v0.15.1)
  icon.rs              # App icon loading (texture + window icon + macOS dock icon)
                       #   (hammer-on-plate icon: v0.16)
  io/
    mod.rs             # JSON / YAML export + Postman v2.1 import       (unit-tested)
    curl.rs            # cURL tokenizer / parser / builder              (unit-tested)
  ui/
    mod.rs             # submodule wiring
    sidebar.rs         # left panel — collections tree, env picker, history, folder
                       #   row drag-to-reorder, search box, update-pill
                       #   (drag-reorder v0.5, update-pill v0.16.1)
    editor.rs          # central panel — tab strip, URL bar, request-editor tabs
                       #   (Params/Headers/Cookies/Body/Auth/Tests), collection
                       #   overview page, JWT decoder (JWT + overview v0.3/v0.4)
    response.rs        # response panel — inline status chips, body toolbar
                       #   (JSON/Tree/Preview/Events/Diff/Raw), search, copy, save,
                       #   headers grid, loading spinner, illustrated failed state,
                       #   structured SSE Events log, line diff renderer
    modals.rs          # env mgr, save-draft folder picker, "Save changes?" confirm,
                       #   cURL paste, snippet panel, settings, About modal,
                       #   command palette (⌘P), actions palette (⇧⌘P),
                       #   update-instructions modal, toast

assets/
  icon.svg             # canonical vector source for the app icon
                       #   (rust-orange plinth + hammer striking plate; v0.16)
  icon.png             # 1024×1024 PNG rendered from icon.svg via
                       #   scripts/generate_icon.py (resvg-py)

scripts/
  deploy.sh            # one-arg release script — version bump + clippy + tag +
                       #   push; refuses to release without a `## Unreleased`
                       #   section in CHANGELOG.md (CHANGELOG gate v0.16.5)
  make_dmg.sh          # DMG packager invoked by `make dmg`
  generate_icon.py     # SVG → 1024×1024 PNG via resvg-py (v0.16)
  generate_dmg_bg.py   # DMG-volume background image generator
install.sh             # macOS + Linux one-line installer (UNINSTALL / PURGE modes v0.17)
```

55 unit tests across `extract`, `assertion`, `cookies`, `oauth`, `io`,
`io::curl`, `html_preview`, `sse`, and `diff` (plus 1 ignored integration
scratch test). Run `cargo test` to verify everything builds + passes.

## Design notes

### Request execution & Cancel

Sends happen on a single long-lived `tokio::Runtime` (built once at app
startup via `net::build_runtime`, stored on `ApiClient`). Each send is
a `runtime.spawn(...)` returning a `JoinHandle`; the handle lives on
`InFlightRequest` alongside a `std::sync::mpsc::Receiver<RequestUpdate>`
that the UI polls every frame.

The `RequestUpdate` enum carries either a `Progress` snapshot (used by
SSE streaming — each event emits one) or the terminal `Final(ResponseData)`.
Cancel is immediate: `handle.abort()` drops the future, and dropping
the future drops the hyper connection, freeing the TCP/TLS resources.
No per-chunk polling, no timeout trick.

### HTTP client reuse

`reqwest::Client` is built once from `AppSettings` (timeout / proxy /
TLS verification) and reused across every send. Rebuilt on Settings
modal save. This preserves the connection pool and TLS session cache
between clicks of Send — ~1ms saved per request.

### Cookie jar

Scoped to the active `Environment`, not global. `Set-Cookie` headers
come back into `ResponseData.set_cookies`; the main loop merges them
into `state.environments[active].cookies` after a successful Final
update. On the outbound side, `net.rs` walks the jar matching the
request host (suffix match) + path (prefix match), expiry-checked,
and merges the hits into the `Cookie` header — deduped with any
explicit headers from the Cookies tab (last wins).

### SSE streaming

`net::stream_sse_response` forks off `execute_request_async` when
the response `Content-Type` is `text/event-stream`. The task reads
chunks via `response.chunk().await`, feeds each into an `SseParser`,
and emits a `RequestUpdate::Progress { snapshot, new_events }` per
batch. The UI accumulates `new_events` into `ApiClient.streaming_events`
for the structured Events view, while `snapshot.body` holds the
formatted text log for the Raw view.

### State persistence

`AppState` (defined in `model.rs`) is the entire persistent model:
folders, environments, active_env_id, history, drafts, open_tabs,
active_tab_id, settings. Serialized as pretty-printed JSON to
`data.json` on every mutating edit. Every optional field uses
`#[serde(default)]` so old files load cleanly into new builds;
empty defaults use `skip_serializing_if` so the file stays readable.

## Releasing

A push of a `v*` git tag triggers
[`.github/workflows/release.yml`](../.github/workflows/release.yml), which
runs two parallel build jobs:

**macOS universal DMG** (`macos-latest`):

1. Builds the release binary (`cargo build --release`, universal)
2. Wraps it as a `.app` bundle (`make app`)
3. Packages a drag-to-Applications **`.dmg`** (`make dmg` →
   `scripts/make_dmg.sh`)
4. Smoke-tests the universal binary, uploads
   `RustyRequester-vX.Y.Z-macos-universal.dmg` to the GitHub Release

**Linux x86_64 tarball** (`ubuntu-22.04`, for glibc 2.35+ compatibility —
see v0.16.7):

1. Builds the release binary
2. Bundles it with `install-local.sh` / `uninstall-local.sh` +
   `.desktop` file + PNG icon into
   `RustyRequester-vX.Y.Z-linux-x86_64.tar.gz`
3. Smoke-tests the binary, uploads the tarball to the same Release

Both jobs extract their release notes from the matching `## [X.Y.Z]` block
in `CHANGELOG.md` and pass them to `action-gh-release` via `body_path`, so
the Release page shows hand-written notes above GitHub's auto-generated
commit list. (CHANGELOG-driven release notes landed in v0.16.5.)

### Cutting a release

There's a one-arg deploy script that handles the bump + tag + push flow
with preflight checks (clean tree, on `main`, tag doesn't already exist):

```bash
./scripts/deploy.sh v0.2.0
```

What it does:

1. Validates the tag format (`vX.Y.Z`).
2. Bumps `[package] version` in `Cargo.toml` (the `Makefile` derives
   `VERSION` from `Cargo.toml` via `awk`, so the single source of truth
   is `Cargo.toml`; `deploy.sh` still rewrites a legacy hardcoded
   `VERSION := X.Y.Z` line in older `Makefile`s if present).
3. Promotes the `## Unreleased` section in `CHANGELOG.md` to
   `## [X.Y.Z] — YYYY-MM-DD`. **Refuses to release** if the section
   doesn't exist or is empty.
4. Runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo build --release` (to refresh `Cargo.lock`), and `cargo test`.
5. Shows the diff and asks for confirmation.
6. Commits `"Release vX.Y.Z"`, creates an annotated tag, pushes `main`
   and the tag.

Pushing the tag triggers the release workflow above.

<details>
<summary>Manual flow (if you don't use the script)</summary>

```bash
$EDITOR Cargo.toml                 # bump [package].version
cargo build --release              # refresh Cargo.lock
cargo test
# promote ## Unreleased → ## [X.Y.Z] — YYYY-MM-DD in CHANGELOG.md
git commit -am "Release vX.Y.Z"
git tag vX.Y.Z
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

## Compatibility & stability

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
