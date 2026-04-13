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

#[derive(Clone, Debug)]
pub struct OpenTab {
    pub folder_path: Vec<String>,
    pub request_id: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RequestTab {
    Params,
    Headers,
    Cookies,
    Body,
    Auth,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    Body,
    Headers,
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
}
