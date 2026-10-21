# Security

Threat model + reporting guidance for **Rusty Requester**. The short
version lives in [`readme.md`](./readme.md#-security) — this file is the
long version.

## Threat model

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
  *Security note* in the [readme Quickstart](./readme.md#-quickstart).
  You can move individual environment-variable values out of this file
  into the OS credential store — see *Keychain-backed secrets* below.

### Keychain-backed secrets

Mark an environment variable secret (the lock toggle in the Environments
editor) and its value moves from `data.json` into the OS credential store
— Keychain on macOS, Secret Service on Linux, Credential Manager on
Windows — the next time the app saves. The file then keeps only the
`secret` flag and an opaque reference (`secret_ref`); the value itself is
never written to disk and is never logged or included in exports or code
snippets.

This is opt-in and reversible: marking a variable is the migration step
for an existing plaintext value, and un-marking it deletes the keychain
entry and returns the value to the file. What this does and does not buy
you:

- **Buys:** a secret at rest sits in the OS store under its own access
  rules, not in a world-readable-if-your-umask-slips JSON file. A copy of
  `data.json` alone no longer carries the value.
- **Does not buy:** protection from code running as you. While the app is
  running it reads the value into memory to send requests, and anything
  with your keychain access can read it too. This is confidentiality at
  rest, not a sandbox.
- **Degrades, never loses.** If no credential store is available (headless
  Linux with no Secret Service, a locked keychain), the value stays in
  `data.json` exactly as before — portable, offline usage still works.
  If a referenced entry is later missing, the field is left empty for you
  to re-enter rather than the app crashing.
- **Supply chain on our deps.** Rust isn't magic. A compromised
  upstream (`reqwest`, `tokio`, `serde`, `egui`) would ship in our
  binary. We mitigate with pinned `Cargo.lock`, widely-used crates
  only, and `cargo audit` before each release — but we can't
  eliminate the risk.

## Reporting a vulnerability

Found a security issue? Open a **private** security advisory via GitHub:
**Security → Report a vulnerability** on the repo. Please don't file a
public issue for exploitable bugs.
