# Features

Full feature catalog for Rusty Requester. For a quick pitch see the
[main README](../readme.md); for internal design see
[`ARCHITECTURE.md`](./ARCHITECTURE.md).

## Core

- 🚀 **Truly native** — Rust + `egui`, no Electron, no Chromium
- 💾 **Fully offline** — all data lives in one local JSON file, no cloud sync, no telemetry
- 🎨 **Dark / Light / Postman themes** — rust-orange / amber accents on either a deep Monokai-style dark canvas *(v0.1)* or a GitHub-ish near-white canvas *(v0.14)*; plus a **Postman** theme (pure-white canvas, warm-gray sidebar chrome, Postman-blue accent, Inter-Light UI font) *(v0.18)*. Toggle in Settings. Syntax-highlighted response body adapts: Monokai on dark, dark-on-paper on the paper themes.
- 🍎 Builds for Apple Silicon, Intel Mac, Linux, and Windows
- 🪟 **Native macOS title-bar integration** — traffic-light buttons float over the app content (no dark stub strip above the workspace).

## Request building

- 🔧 Full HTTP methods: `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`
- 📝 Tabbed editor: **Params · Headers · Cookies · Body · Auth · Tests**
- 🔗 **Bidirectional URL ↔ Params sync** (Postman-style): type `?foo=bar` in the URL bar → params populate the table; edit the table → URL bar rebuilds. Auto-growing "ghost" row.
- 🍪 Cookies list (merged into a `Cookie` header on send)
- 🔐 Auth presets: **No Auth · Bearer Token · Basic Auth · OAuth 2.0** + **JWT decoder** (Bearer tokens auto-decode below the input — header & payload pretty JSON, scope/exp at a glance)
- 🪪 **OAuth 2.0 Authorization Code + PKCE** *(v0.15)* — click **Get New Token** in the Auth tab; browser opens the provider's authorize URL, loopback listener catches the redirect, code is exchanged for a token via the provider's `token_url`. Access token auto-injected as `Authorization: Bearer <token>` on every send. Live status badge (valid / expired / refreshing soon). No client secret required for PKCE clients.
- 📦 **Body modes**: Raw / `x-www-form-urlencoded` / `multipart/form-data` / **GraphQL** (query + variables, sent as `application/json`)
- 📐 **Line-numbered request body** with subtle `Beautify` / `Minify` action links
- 🌱 **Environment variables** — define key/value vars per environment, reference them anywhere with `{{varname}}`. Switch active env from the sidebar.
- 📜 **Request history** — every send is logged (method, URL, status, time, response preview); browse the last 200 from the sidebar's History tab.
- 🔗 **Post-response extractors (Tests tab)** *(v0.4)* — dot/bracket paths (`data.token`, `items[0].id`, `$.x`) or header names that pull a value from the response and write it into the active environment, so the next request can `{{token}}` it.
- ✅ **Assertions (Tests tab)** *(v0.4)* — pass/fail rules against the response: status equals/`>`/`<`, header exists, body-path equals / contains / matches `^2..$`. Result-dot per row (green / red / amber); toast summarizes after each send.

## Responses

- 📊 Status pill + response time + size rendered inline with the Body/Headers tab row, on the right (Postman-style: `Body  Headers  JSON Tree Raw  [🔍][📋][💾]    200 OK · 54 ms · 434 B`)
- 🛈 **Size hover tooltip** — breakdown of response headers/body and request headers/body bytes
- 🛈 **Time hover tooltip** — gantt-style phase breakdown: Prepare · Waiting (TTFB) · Download
- 🧩 **Body view modes**: **JSON** (syntax-highlighted code editor with line numbers + Postman-style fold chevrons next to every multi-line `{` / `[` — *(v0.18.4)*), **Tree** (collapsible JSON tree with filter + right-click "Copy path"), **Preview** (HTML rendered as readable text for error pages / login challenges), **Events** (structured log for `text/event-stream` / SSE responses), **Diff** (unified +/− against the previous response), **Raw** (verbatim) — pills are inline with the section tabs and don't scroll away
- 🪗 **JSON folding** *(v0.18.4)* — click the chevron next to any `{` or `[` to collapse it into a `{ …}` / `[ …]` placeholder; click again to expand. Selection / copy / ⌘F search keep working over the visible (un-folded) text. Fold state is per-response and resets on every Send.
- 🧠 **Per-request response cache** *(v0.15.6)* — switching tabs preserves each request's last response (body, status, timings, headers, SSE events, assertion results). Session-only; closed tabs drop their cache.
- 📡 **Server-Sent Events (SSE)** *(v0.10)* — native streaming support for LLM / event-stream APIs. Auto-detected by `Content-Type: text/event-stream`; events flow into a collapsible-per-row Events view with auto-scroll, per-event timestamps, and JSON-pretty-printed data. Cancel aborts the stream instantly.
- 🔀 **Response diff** *(v0.11)* — send a request twice to compare. The **Diff** pill shows a unified `+/-` line-diff of the current response against the previous one, with `+A −B` summary.
- 🛑 **Cancel button** *(v0.7)* — Send flips to Cancel while a request is in flight. Instantly aborts the tokio task + underlying hyper connection (no per-chunk polling).
- 🖼 **Failed/cancelled state** — dedicated illustrated screen with status headline + error-detail pill instead of opaque text, for network failures, TLS issues, or user cancels.
- 🔍 **Find in body** — toolbar search icon highlights all matches inline
- 📋 **Copy response body** + 💾 **Save response to file** (Content-Type → file extension auto-suggested)
- 📑 Separate **Body / Headers** tabs; rust-orange accent on header keys
- ⏳ Rust-orange **loading spinner** while the request is in flight
- 🎨 Auto-pretty-printed JSON responses; click into the view to position caret, select, ⌘A / ⌘C

## Cookie jar (per-environment)

*(v0.5)*

- 🪪 **`Set-Cookie` auto-persisted** into the active environment with name/domain/path/expiry tracked
- 🔄 **Auto-replayed** on the next request that matches host (suffix-match) + path (prefix-match) — same model browsers use
- ⏰ Expired cookies (Max-Age / Expires past) pruned automatically; session cookies kept until app quit
- 🔁 Switching active environment swaps the cookie set (Staging cookies don't leak into Prod)

## cURL interop

- 📋 **Copy as cURL** — current request → clipboard as a `curl` command
- 📥 **Paste from cURL** — paste any `curl` command and it becomes a request (method, URL, headers, body, auth, cookies, params — all parsed)
- 💻 **Code snippet panel** — side panel generating `cURL` / Python `requests` / JavaScript `fetch` / HTTPie, with syntax-highlighted code + line numbers and a copy icon

## Collections

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

## Tabs

- 📑 **Multi-tab workspace** — open many requests in parallel; tabs persist across quit/relaunch
- ⚓ **Pinned tabs** *(v0.9.1)* — right-click → **Pin tab** keeps a tab around; ⌘W and "Close all" skip pinned tabs (accent-colored pin glyph in the tab strip)
- 🧬 **Duplicate tab** *(v0.9.1)* — **⌘D** (or right-click → Duplicate tab) clones the current request as a new draft with `(copy)` appended to the name
- 💾 **"Save changes?" confirmation** *(v0.8)* — closing a draft tab with unsaved content opens a modal (Don't save · Cancel · Save changes)

## Workflow

- 🎛 **Settings modal** *(v0.3)* — request timeout, max body size cap (50 MB default; truncates with banner), proxy URL, TLS verification toggle. All persisted to disk.
- 🔌 **Reused HTTP client + tokio runtime** *(v0.3 / v0.5)* — no per-request connection-pool / runtime spinup; faster repeated sends.
- ⌨️ **⌘P command palette** *(v0.5)* — fuzzy-find any request across every collection, ↑↓ navigate, Enter to open
- ⌨️ **⇧⌘P actions palette** *(v0.11)* — fuzzy-find an app **action** (New request, Duplicate tab, Toggle snippet panel, Copy as cURL, Open environments, Clear history, …). Fully discoverable — open the palette and start typing.
- ⌨️ **↑ / ↓ arrow nav** — step through every request across every collection when nothing's focused; wraps at both ends
- ⌨️ Standard shortcuts: **⌘⏎** Send · **⌘N** New request · **⌘W** Close tab · **⌘D** Duplicate tab · **⌘K** Focus search · **⌘S** Save draft · **⌘P** Command palette · **⇧⌘P** Actions palette · **F2** Rename · **Esc** Dismiss modals
- 🍎 **Native macOS NSMenu bar** *(v0.3)* — (Rusty Requester · File · View · Request · Help) via `muda`; in-window menu on Linux
- 🎨 **Phosphor icon font** *(v0.9)* — 1,200+ tintable icons rendered as font glyphs; crisp at every DPI, zero image assets to ship
- ℹ **Help → About** opens a custom modal with creator credit + Contribute / Report-issue links
- 🔔 **Update check + sidebar pill** *(v0.13 / v0.16.1)* — on-launch GitHub-latest-release check (one silent GET, off-switch in Settings); sidebar pill when a newer tag lands, with a copyable install-one-liner modal. Dismissible per-version so deferring doesn't re-pester.

---

## Usage guide

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
- **⌘/Ctrl + N** → New request tab
- **⌘/Ctrl + W** → Close active tab (skips pinned; prompts to save on unsaved drafts)
- **⌘/Ctrl + D** → Duplicate active tab
- **⌘/Ctrl + K** → Focus the sidebar search
- **⌘/Ctrl + P** → Command palette (fuzzy request finder)
- **⇧⌘/Ctrl⇧ + P** → Actions palette (fuzzy app-action finder — type to see every available action)
- **⌘/Ctrl + S** → Save current draft to a folder
- **↑ / ↓** → Arrow-nav through every request (when no modal or text field is focused)
- **F2** → Rename the active request (VS Code / Finder convention)
- **Double-click a request** in the sidebar → Inline rename
- **Esc** during rename → Cancel; **Enter** → Save
- **Right-click a tab** → Save to folder / Duplicate / Pin / Close / Close others / Close all
- **Right-click a request** → Rename / Duplicate / Delete
- **Right-click a collection / folder** → Rename / Add subfolder / Export / Delete

---

## UI conventions

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

Nested folders render with a small folder glyph; top-level collections
intentionally omit the glyph so users can tell the two apart at a
glance. Icons throughout the UI come from the embedded **Phosphor** icon
font (via `egui-phosphor`) — tintable by color, crisp at every DPI, no
PNG/SVG assets. The collapse-header chevron is a hand-painted triangle
because it animates its rotation openness as the folder expands.

---

## Roadmap

**Shipped** (by category — see [`CHANGELOG.md`](../CHANGELOG.md) for the per-version log):

Foundations
- [x] Full HTTP methods, tabbed editor (Params / Headers / Cookies / Body / Auth / Tests)
- [x] cURL import (paste in URL bar) / export (right-side code panel: cURL, Python, JS, HTTPie) with syntax-highlighted line-numbered code view
- [x] Postman Collection v2.1 import
- [x] JSON / YAML export & import
- [x] Bidirectional **URL ↔ Params sync** — type `?k=v` in URL, table populates; edit table, URL rebuilds
- [x] Bearer / Basic auth presets + **JWT decoder** for Bearer tokens
- [x] **OAuth 2.0 Authorization Code + PKCE** — in-app browser flow with loopback listener, auto-injects Bearer token on send
- [x] Body modes: Raw / form-urlencoded / multipart / GraphQL
- [x] Environment variables — `{{var}}` substitution with active env selector + manage modal
- [x] Request history — last 200 sends with preview

Workflow
- [x] Postman-style request tabs (open multiple requests, switch with one click) with hover preview
- [x] **Duplicate tab (⌘D) and pinned tabs** — pinned tabs skip ⌘W / "Close all"
- [x] **"Save changes?" confirmation** — closing a draft with unsaved content prompts (Don't save / Cancel / Save)
- [x] Inline rename via double-click; F2 shortcut
- [x] Subfolders; collection overview page; inline folder `+`/`⋯` toolbar; duplicate folder recursively; save-draft folder picker with inline "New folder"
- [x] Drag-to-reorder requests within a folder
- [x] Search across requests, URLs, methods, folder names (⌘K)
- [x] **⌘P command palette** — fuzzy-find any request and jump
- [x] **↑/↓ arrow navigation** through every request in the sidebar

Response viewing
- [x] **Body view modes**: JSON (syntax-highlighted code, line numbers), Tree (collapsible JSON tree with **right-click "Copy path"**), **Preview** (HTML rendered as readable text), **Events** (structured SSE log), Raw
- [x] **Server-Sent Events (SSE)** — streaming support for LLM / event-stream APIs with live Events view (auto-scroll, per-event timestamps, JSON pretty-print)
- [x] **Response diff** — send twice to the same request → Diff pill shows unified `+/-` line-diff with `+A −B` summary
- [x] **⇧⌘P actions palette** — fuzzy-find app actions (16 built-in: New / Duplicate / Close tab, Save draft, Copy as cURL, Toggle snippet panel, Open environments, Open settings, Paste cURL, Import/Export, Clear history, etc.)
- [x] **Cancel button** — Send flips to Cancel while in flight; instantly aborts the tokio task
- [x] **Illustrated failed/cancelled state** — network failure / TLS / cancel screens with detail pill instead of raw text
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

UI
- [x] **Phosphor icon font** — 1,200+ tintable icons rendered as font glyphs (replaces every hand-drawn painter icon)
- [x] **Styled hint text** — dim, italic-free placeholders in all TextEdits so `Key`/`Value`/`Description` no longer look like real data
- [x] **Light theme** (Settings → Theme) — flips egui's chrome; saturated accents stay constant across themes

Platform
- [x] macOS + Linux one-line installer (`install.sh`); auto-detects platform *(v0.2.x)*
- [x] Universal macOS DMG (Apple Silicon + Intel) and Linux x86_64 tarball built by GitHub Actions *(v0.2.x)*
- [x] **Native macOS NSMenu bar** (App / File / View / Request / Help) with ⌘N, ⌘W, ⌘P, ⌘⏎, ⇧⌘C accelerators *(v0.3)*
- [x] Custom **About modal** with creator credit + Contribute / Report-issue links *(v0.3)*
- [x] **Native macOS title-bar integration** — traffic-light buttons float over app content *(v0.15.1)*
- [x] **New app icon** — Phosphor hammer on a steel plate with rust-orange plinth *(v0.16)*
- [x] **Launch update check + sidebar pill + dismissible per version** *(v0.13 / v0.16.1)*
- [x] **Clean Linux uninstall** — `UNINSTALL=1` / `UNINSTALL=1 PURGE=1` modes *(v0.17)*

**On the v1.0 pathway:**

_Nothing open — OAuth 2.0 Auth Code + PKCE *(v0.15)* was the last v1.0 item._

**Post-1.0:**

- [ ] **OAuth 2.0 Client Credentials + Refresh flows** — the first release ships Auth Code + PKCE only.
- [ ] **Native keychain integration** — back OAuth + Bearer tokens with the OS keychain via the `keyring` crate.
- [ ] **WebSocket testing** — separate connection lifecycle + per-request message log.
- [ ] **Pre-request scripts** — Rhai/Boa scripting engine for transformations before send.
- [ ] **Windows builds** — CI + installer parity with macOS/Linux.
- [ ] **Collection runner** — run a folder as a sequence, pass extracted vars forward.
