use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Request {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub query_params: Vec<KvRow>,
    #[serde(default)]
    pub headers: Vec<KvRow>,
    #[serde(default)]
    pub cookies: Vec<KvRow>,
    pub body: String,
    /// When `body_ext` is present, the body content / encoding is determined
    /// by it. When absent, `body` is used as raw text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_ext: Option<BodyExt>,
    #[serde(default)]
    pub auth: Auth,
    /// Post-response extractors — rules that pull values out of the
    /// response and write them into the active environment, so the next
    /// request can reference them via `{{var}}`.
    #[serde(default)]
    pub extractors: Vec<ResponseExtractor>,
    /// Post-response assertions — rules that check the response
    /// against expected values (status, header presence, JSON path
    /// equality, substring / regex match). Evaluated alongside the
    /// extractors after each send; results shown inline in the
    /// Tests tab.
    #[serde(default)]
    pub assertions: Vec<ResponseAssertion>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ResponseExtractor {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Env var name to write the extracted value into.
    pub variable: String,
    pub source: ExtractorSource,
    /// Expression interpreted per-source: for Body it's a dot-/bracket-
    /// path into the JSON body (e.g. `data.token`, `items[0].id`); for
    /// Header it's the header name; for Status it's ignored.
    pub expression: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ExtractorSource {
    Body,
    Header,
    Status,
}

impl ExtractorSource {
    pub fn label(&self) -> &'static str {
        match self {
            ExtractorSource::Body => "Body",
            ExtractorSource::Header => "Header",
            ExtractorSource::Status => "Status",
        }
    }
}

/// A single pass/fail check run against the response. Evaluated after
/// each send; the outcome is transient (not persisted) and shown as a
/// dot next to the assertion row in the Tests tab.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ResponseAssertion {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub source: AssertionSource,
    /// Per-source:
    ///   - `Body` — dot/bracket JSON path (same syntax as extractors)
    ///   - `Header` — header name
    ///   - `Status` — ignored
    #[serde(default)]
    pub expression: String,
    pub op: AssertionOp,
    #[serde(default)]
    pub expected: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssertionSource {
    Status,
    Header,
    Body,
}

impl AssertionSource {
    pub fn label(&self) -> &'static str {
        match self {
            AssertionSource::Status => "Status",
            AssertionSource::Header => "Header",
            AssertionSource::Body => "Body",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssertionOp {
    Equals,
    NotEquals,
    Contains,
    Matches,
    Exists,
    GreaterThan,
    LessThan,
}

impl AssertionOp {
    pub fn label(&self) -> &'static str {
        match self {
            AssertionOp::Equals => "equals",
            AssertionOp::NotEquals => "≠",
            AssertionOp::Contains => "contains",
            AssertionOp::Matches => "matches /re/",
            AssertionOp::Exists => "exists",
            AssertionOp::GreaterThan => ">",
            AssertionOp::LessThan => "<",
        }
    }
    /// Whether the `expected` value is meaningful for this operator.
    /// `Exists` is a presence check with no right-hand side.
    pub fn takes_expected(&self) -> bool {
        !matches!(self, AssertionOp::Exists)
    }
}

/// Transient outcome of running one assertion. Not persisted — the
/// Tests tab recomputes it after each response.
#[derive(Clone, Debug, PartialEq)]
pub enum AssertionResult {
    Pass,
    Fail(String),  // human-readable reason, e.g. "got 500, expected 200"
    Error(String), // regex parse failure / path not found / etc.
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum BodyExt {
    /// Sent as `application/x-www-form-urlencoded` with keys/values from `fields`.
    FormUrlEncoded { fields: Vec<KvRow> },
    /// Sent as `multipart/form-data` with text-only parts from `fields`.
    MultipartForm { fields: Vec<KvRow> },
    /// Sent as JSON `{"query": <body>, "variables": <parsed variables>}`
    /// with `application/json` content type.
    GraphQL { variables: String },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BodyMode {
    Raw,
    FormUrlEncoded,
    MultipartForm,
    GraphQL,
}

impl BodyMode {
    pub fn label(&self) -> &'static str {
        match self {
            BodyMode::Raw => "Raw",
            BodyMode::FormUrlEncoded => "x-www-form-urlencoded",
            BodyMode::MultipartForm => "form-data",
            BodyMode::GraphQL => "GraphQL",
        }
    }
}

// Variant names mirror the HTTP-protocol token verbatim (capitalized,
// per RFC 9110). Clippy's `upper_case_acronyms` lint wants e.g. `Get`,
// but we serialize/display these directly to JSON and to the URL bar
// where users expect to see "GET", not "Get".
#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum Auth {
    #[default]
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    /// OAuth 2.0 Authorization Code + PKCE flow. Holds the user's
    /// provider config plus the cached access / refresh tokens from
    /// the last successful flow. Boxed to keep the `Auth` enum small
    /// — OAuth config adds ~10 strings worth of payload compared to
    /// Bearer's single token.
    #[serde(rename = "OAuth2")]
    OAuth2(Box<OAuth2State>),
}

/// Cached OAuth2 state — config (stable) + tokens (refreshed per
/// flow). Persisted in `data.json` the same way other `Auth` data is.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct OAuth2State {
    pub config: OAuth2Config,
    /// Access token from the last successful flow. Empty until the
    /// user clicks "Get New Token" and completes the browser dance.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub access_token: String,
    /// Refresh token, if the provider returned one. Not used yet by
    /// the request path (scope: v0.15 ships manual re-auth only);
    /// persisted so a future auto-refresh can pick it up without a
    /// schema change.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub refresh_token: String,
    /// Unix epoch seconds when the access token expires. `None` =
    /// no expiry info returned (long-lived token, or provider
    /// omitted `expires_in`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}

/// User-editable OAuth2 provider config. All fields are free-form
/// strings so users can point at any RFC 6749-compliant provider.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct OAuth2Config {
    pub auth_url: String,
    pub token_url: String,
    pub client_id: String,
    /// Empty for public clients (the common case for a native app —
    /// PKCE replaces the secret). Some providers require it anyway
    /// for "confidential" apps; we send it if non-empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub client_secret: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scope: String,
    /// Registered redirect URI. Must match what's configured on the
    /// provider side. We always listen on a random 127.0.0.1 port
    /// unless the user overrides — the usual setup is to register
    /// something like `http://127.0.0.1/callback` (many providers
    /// allow any 127.0.0.1 port for PKCE clients).
    #[serde(default = "default_redirect_uri")]
    pub redirect_uri: String,
}

fn default_redirect_uri() -> String {
    "http://127.0.0.1/callback".to_string()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    None,
    Bearer,
    Basic,
    OAuth2,
}

impl From<&Auth> for AuthKind {
    fn from(a: &Auth) -> Self {
        match a {
            Auth::None => AuthKind::None,
            Auth::Bearer { .. } => AuthKind::Bearer,
            Auth::Basic { .. } => AuthKind::Basic,
            Auth::OAuth2(_) => AuthKind::OAuth2,
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct KvRow {
    pub enabled: bool,
    pub key: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

fn default_true() -> bool {
    true
}

impl KvRow {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            enabled: true,
            key: key.into(),
            value: value.into(),
            description: String::new(),
        }
    }

    pub fn empty() -> Self {
        Self::new("", "")
    }

    pub fn is_blank(&self) -> bool {
        self.key.is_empty() && self.value.is_empty()
    }
}

impl<'de> Deserialize<'de> for KvRow {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Either {
            Tuple(String, String),
            Struct {
                #[serde(default = "default_true")]
                enabled: bool,
                #[serde(default)]
                key: String,
                #[serde(default)]
                value: String,
                #[serde(default)]
                description: String,
            },
        }
        match Either::deserialize(d)? {
            Either::Tuple(k, v) => Ok(KvRow::new(k, v)),
            Either::Struct {
                enabled,
                key,
                value,
                description,
            } => Ok(KvRow {
                enabled,
                key,
                value,
                description,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub requests: Vec<Request>,
    #[serde(default)]
    pub subfolders: Vec<Folder>,
    /// Free-text description shown on the collection/folder overview
    /// page. Multiline; can be empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct AppState {
    pub folders: Vec<Folder>,
    #[serde(default)]
    pub environments: Vec<Environment>,
    #[serde(default)]
    pub active_env_id: Option<String>,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
    /// Unsaved requests — created via the "+" button, live here until the
    /// user explicitly saves them to a folder or closes the tab.
    #[serde(default)]
    pub drafts: Vec<Request>,
    /// Persisted open tabs — restored on next launch so the workspace
    /// survives quit/relaunch like Postman.
    #[serde(default)]
    pub open_tabs: Vec<OpenTab>,
    /// `request_id` of the tab that was active at save time.
    #[serde(default)]
    pub active_tab_id: Option<String>,
    /// App-wide settings: timeout, body cap, proxy, TLS verification.
    /// Exposed via the Settings modal in the sidebar.
    #[serde(default)]
    pub settings: AppSettings,
}

/// User-configurable networking / safety knobs. Defaults are tuned so
/// first-time users never need to open the Settings modal.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppSettings {
    /// Per-request timeout in seconds. `0` disables the timeout.
    #[serde(default = "default_timeout_sec")]
    pub timeout_sec: u64,
    /// Max response body size in megabytes. Bytes past this are
    /// discarded and a banner is shown. `0` disables the cap.
    #[serde(default = "default_max_body_mb")]
    pub max_body_mb: u64,
    /// When false, the HTTPS client accepts self-signed / expired
    /// certificates. Useful for internal dev APIs; dangerous on the
    /// public internet.
    #[serde(default = "default_verify_tls")]
    pub verify_tls: bool,
    /// HTTP/HTTPS/SOCKS5 proxy URL (e.g. `http://proxy:8080`). Empty
    /// = direct.
    #[serde(default)]
    pub proxy_url: String,
    /// UI theme. `Dark` is the default, opinionated palette; `Light`
    /// flips egui's chrome (panels, text, borders, widget backgrounds)
    /// for bright environments. Saturated accents (method colors,
    /// status pills, rust-orange accent) stay the same across themes
    /// — they're tuned to read on both backgrounds.
    #[serde(default)]
    pub theme: Theme,
    /// When true (default), the app makes one silent GET to
    /// `api.github.com/.../releases/latest` on launch and shows a
    /// toast if a newer version exists. Disable for strict offline
    /// operation — no outbound traffic from this app on startup.
    #[serde(default = "default_check_updates")]
    pub check_updates_on_launch: bool,
    /// Version tag the user last dismissed from the sidebar pill
    /// (e.g. "v0.16.3"). Suppresses the pill for that exact version —
    /// so users who deferred updating don't see the same pill every
    /// launch. Reappears automatically when a newer version drops.
    #[serde(default)]
    pub dismissed_update_version: Option<String>,
}

/// UI theme — drives `apply_style`'s choice of `Visuals`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

fn default_timeout_sec() -> u64 {
    60
}
fn default_max_body_mb() -> u64 {
    50
}
fn default_verify_tls() -> bool {
    true
}
fn default_check_updates() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            timeout_sec: default_timeout_sec(),
            max_body_mb: default_max_body_mb(),
            verify_tls: default_verify_tls(),
            proxy_url: String::new(),
            theme: Theme::Dark,
            check_updates_on_launch: default_check_updates(),
            dismissed_update_version: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Environment {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub variables: Vec<KvRow>,
    /// Persistent cookie jar — populated by `Set-Cookie` response
    /// headers and sent back on subsequent requests whose host +
    /// path match. Scoped to the active environment so switching
    /// envs (Staging → Prod) swaps cookie sets too.
    #[serde(default)]
    pub cookies: Vec<StoredCookie>,
}

/// Minimal RFC 6265-ish cookie record. Only tracks the fields we
/// actually need for replay (domain, path, expiry). No SameSite /
/// Priority / partitioning — not needed for an API client.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoredCookie {
    pub name: String,
    pub value: String,
    /// Lowercase host match — "example.com" matches "api.example.com"
    /// and "example.com" itself. Empty = match the request's exact host.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub domain: String,
    /// URL-prefix match, default "/".
    #[serde(default = "default_cookie_path")]
    pub path: String,
    /// Unix epoch seconds. `None` = session cookie (kept until the
    /// app quits; treated as valid indefinitely for our purposes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<i64>,
    /// Whether the cookie was marked `Secure` — we still send it on
    /// plain http (dev APIs), but we track the flag.
    #[serde(default, skip_serializing_if = "is_false")]
    pub secure: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub http_only: bool,
}

#[inline]
fn is_false(b: &bool) -> bool {
    !*b
}

fn default_cookie_path() -> String {
    "/".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryEntry {
    pub id: String,
    /// Unix epoch seconds.
    pub timestamp: i64,
    pub method: HttpMethod,
    pub url: String,
    pub status: String,
    pub time_ms: u64,
    /// Up to ~256 chars of the response body, for preview.
    pub response_preview: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OpenTab {
    /// Empty vec = the tab points at an entry in `AppState.drafts` (a
    /// not-yet-saved "Untitled" request). Non-empty = path into
    /// `AppState.folders` to locate the request.
    #[serde(default)]
    pub folder_path: Vec<String>,
    pub request_id: String,
    /// Pinned tabs are skipped by ⌘W and "Close others" / "Close all"
    /// so they can be kept around as persistent references.
    #[serde(default, skip_serializing_if = "is_false")]
    pub pinned: bool,
}

impl OpenTab {
    pub fn is_draft(&self) -> bool {
        self.folder_path.is_empty()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RequestTab {
    Params,
    Headers,
    Cookies,
    Body,
    Auth,
    Tests,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    Body,
    Headers,
}

/// Display mode for the Response → Body tab.
///   • `Json` — pretty-printed with syntax highlighting + line numbers
///     (default, matches Postman's JSON viewer).
///   • `Tree` — collapsible tree with keys/leaves; useful for big/deep
///     payloads.
///   • `Preview` — HTML rendered as readable text (script/style
///     stripped, entities decoded). Only offered when the response
///     Content-Type is `text/html`.
///   • `Events` — structured SSE event log with per-event rows.
///     Only offered when the response Content-Type is
///     `text/event-stream`.
///   • `Diff` — line-diff against the previous response body. Only
///     offered when a prior response exists for the same request.
///   • `Raw` — verbatim text, no formatting.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BodyView {
    Json,
    Tree,
    Preview,
    Events,
    Diff,
    Raw,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SidebarView {
    Collections,
    History,
}

pub struct ResponseData {
    pub body: String,
    pub status: String,
    pub time: String,
    pub headers: Vec<(String, String)>,
    /// Parsed `Set-Cookie` headers, ready to merge into the active
    /// environment's jar. Empty for cross-thread responses that had
    /// no `Set-Cookie` at all.
    pub set_cookies: Vec<StoredCookie>,
    /// Serialized byte size of the response headers (key: value CRLFs).
    pub response_headers_bytes: usize,
    /// Byte size of the *raw* response body (pre-pretty-print).
    pub response_body_bytes: usize,
    /// Serialized byte size of the outgoing request line + headers.
    pub request_headers_bytes: usize,
    /// Byte size of the outgoing request body.
    pub request_body_bytes: usize,
    /// Phase timings in milliseconds. We only measure what reqwest's
    /// high-level API exposes; finer phases (DNS/TCP/TLS) are rolled
    /// into TTFB since the underlying connector hides them.
    pub prepare_ms: u64,
    pub waiting_ms: u64,
    pub download_ms: u64,
    pub total_ms: u64,
}
