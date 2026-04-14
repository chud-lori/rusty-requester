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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Auth {
    None,
    Bearer { token: String },
    Basic { username: String, password: String },
}

impl Default for Auth {
    fn default() -> Self {
        Auth::None
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    None,
    Bearer,
    Basic,
}

impl From<&Auth> for AuthKind {
    fn from(a: &Auth) -> Self {
        match a {
            Auth::None => AuthKind::None,
            Auth::Bearer { .. } => AuthKind::Bearer,
            Auth::Basic { .. } => AuthKind::Basic,
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

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            timeout_sec: default_timeout_sec(),
            max_body_mb: default_max_body_mb(),
            verify_tls: default_verify_tls(),
            proxy_url: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Environment {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub variables: Vec<KvRow>,
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
///   • `Raw` — verbatim text, no formatting.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BodyView {
    Json,
    Tree,
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
