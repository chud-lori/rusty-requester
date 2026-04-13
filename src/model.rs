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
    #[serde(default)]
    pub auth: Auth,
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

#[derive(Clone, Debug, Serialize)]
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

pub struct ResponseData {
    pub body: String,
    pub status: String,
    pub time: String,
    pub headers: Vec<(String, String)>,
}
