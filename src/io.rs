use crate::{Auth, Folder, HttpMethod, KvRow, Request};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Export {
    pub version: String,
    pub folders: Vec<Folder>,
}

#[derive(Clone, Copy)]
pub enum Format {
    Json,
    Yaml,
}

pub fn export_string(folders: &[Folder], format: Format) -> Result<String, String> {
    let export = Export {
        version: "1".to_string(),
        folders: folders.to_vec(),
    };
    match format {
        Format::Json => serde_json::to_string_pretty(&export).map_err(|e| e.to_string()),
        Format::Yaml => serde_yaml::to_string(&export).map_err(|e| e.to_string()),
    }
}

pub fn import_from_file(path: &Path) -> Result<Vec<Folder>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
    import_from_str(&content, path.extension().and_then(|s| s.to_str()).unwrap_or(""))
}

pub fn import_from_str(content: &str, ext_hint: &str) -> Result<Vec<Folder>, String> {
    let is_yaml = ext_hint.eq_ignore_ascii_case("yaml") || ext_hint.eq_ignore_ascii_case("yml");

    if is_yaml {
        let folders = try_parse_yaml(content)?;
        return Ok(regen_ids_all(folders));
    }

    // JSON: try Postman first (by shape), then our native, then YAML fallback
    if let Ok(value) = serde_json::from_str::<Value>(content) {
        if looks_like_postman(&value) {
            let pc: PostmanCollection = serde_json::from_value(value)
                .map_err(|e| format!("Postman parse error: {}", e))?;
            return Ok(regen_ids_all(vec![postman_to_folder(&pc)]));
        }

        if let Ok(export) = serde_json::from_value::<Export>(value.clone()) {
            return Ok(regen_ids_all(export.folders));
        }
        if let Ok(folders) = serde_json::from_value::<Vec<Folder>>(value.clone()) {
            return Ok(regen_ids_all(folders));
        }
        if let Ok(folder) = serde_json::from_value::<Folder>(value) {
            return Ok(regen_ids_all(vec![folder]));
        }
    }

    // Last resort: try YAML
    let folders = try_parse_yaml(content)?;
    Ok(regen_ids_all(folders))
}

fn try_parse_yaml(content: &str) -> Result<Vec<Folder>, String> {
    if let Ok(export) = serde_yaml::from_str::<Export>(content) {
        return Ok(export.folders);
    }
    if let Ok(folders) = serde_yaml::from_str::<Vec<Folder>>(content) {
        return Ok(folders);
    }
    if let Ok(folder) = serde_yaml::from_str::<Folder>(content) {
        return Ok(vec![folder]);
    }
    Err("Could not parse as JSON, YAML, or Postman collection".to_string())
}

fn looks_like_postman(value: &Value) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };
    if !obj.contains_key("item") {
        return false;
    }
    if let Some(info) = obj.get("info") {
        if let Some(schema) = info.get("schema").and_then(|v| v.as_str()) {
            if schema.contains("getpostman") || schema.contains("postman") {
                return true;
            }
        }
        if info.get("name").is_some() {
            return true;
        }
    }
    false
}

fn regen_ids_all(mut folders: Vec<Folder>) -> Vec<Folder> {
    for f in folders.iter_mut() {
        regen_ids(f);
    }
    folders
}

fn regen_ids(folder: &mut Folder) {
    folder.id = Uuid::new_v4().to_string();
    for r in folder.requests.iter_mut() {
        r.id = Uuid::new_v4().to_string();
    }
    for f in folder.subfolders.iter_mut() {
        regen_ids(f);
    }
}

// ============== Postman v2.1 types ==============

#[derive(Deserialize)]
struct PostmanCollection {
    info: PostmanInfo,
    #[serde(default)]
    item: Vec<PostmanItem>,
}

#[derive(Deserialize)]
struct PostmanInfo {
    name: String,
    #[serde(default)]
    #[allow(dead_code)]
    schema: String,
}

#[derive(Deserialize)]
struct PostmanItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    item: Option<Vec<PostmanItem>>,
    #[serde(default)]
    request: Option<PostmanRequest>,
}

#[derive(Deserialize)]
struct PostmanRequest {
    #[serde(default)]
    method: String,
    #[serde(default)]
    header: Vec<PostmanHeader>,
    #[serde(default)]
    body: Option<PostmanBody>,
    #[serde(default)]
    url: serde_json::Value,
    #[serde(default)]
    auth: Option<PostmanAuth>,
}

#[derive(Deserialize)]
struct PostmanHeader {
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Deserialize)]
struct PostmanBody {
    #[serde(default)]
    mode: String,
    #[serde(default)]
    raw: String,
}

fn parse_postman_url(v: &serde_json::Value) -> (String, Vec<KvRow>) {
    match v {
        serde_json::Value::String(s) => split_url_basic(s),
        serde_json::Value::Object(obj) => {
            let raw = obj
                .get("raw")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let query: Vec<KvRow> = obj
                .get("query")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|q| {
                            let key = q.get("key").and_then(|k| k.as_str())?.to_string();
                            let value = q
                                .get("value")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let disabled = q
                                .get("disabled")
                                .and_then(|d| d.as_bool())
                                .unwrap_or(false);
                            Some(KvRow {
                                enabled: !disabled,
                                key,
                                value,
                                description: String::new(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            if query.is_empty() {
                split_url_basic(&raw)
            } else {
                (strip_query(&raw), query)
            }
        }
        _ => (String::new(), Vec::new()),
    }
}

#[derive(Deserialize)]
struct PostmanAuth {
    #[serde(rename = "type", default)]
    auth_type: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, Value>,
}

fn postman_to_folder(pc: &PostmanCollection) -> Folder {
    let mut root = Folder {
        id: Uuid::new_v4().to_string(),
        name: if pc.info.name.is_empty() {
            "Imported".to_string()
        } else {
            pc.info.name.clone()
        },
        requests: Vec::new(),
        subfolders: Vec::new(),
    };
    for item in &pc.item {
        process_item(item, &mut root);
    }
    root
}

fn process_item(item: &PostmanItem, parent: &mut Folder) {
    if let Some(subitems) = &item.item {
        let mut sub = Folder {
            id: Uuid::new_v4().to_string(),
            name: if item.name.is_empty() {
                "Folder".to_string()
            } else {
                item.name.clone()
            },
            requests: Vec::new(),
            subfolders: Vec::new(),
        };
        for s in subitems {
            process_item(s, &mut sub);
        }
        parent.subfolders.push(sub);
    } else if let Some(req) = &item.request {
        parent.requests.push(postman_to_request(&item.name, req));
    }
}

fn postman_to_request(name: &str, r: &PostmanRequest) -> Request {
    let method = parse_method(&r.method);

    let (url, query_params) = parse_postman_url(&r.url);

    let headers: Vec<KvRow> = r
        .header
        .iter()
        .filter(|h| !h.key.is_empty())
        .map(|h| KvRow {
            enabled: !h.disabled,
            key: h.key.clone(),
            value: h.value.clone(),
            description: String::new(),
        })
        .collect();

    let body = match &r.body {
        Some(b) if b.mode == "raw" => b.raw.clone(),
        _ => String::new(),
    };

    let auth = r.auth.as_ref().map(postman_auth).unwrap_or(Auth::None);

    let (filtered_headers, final_auth) = promote_auth_header(headers, auth);

    Request {
        id: Uuid::new_v4().to_string(),
        name: if name.is_empty() {
            "Request".to_string()
        } else {
            name.to_string()
        },
        method,
        url,
        query_params,
        headers: filtered_headers,
        cookies: Vec::new(),
        body,
        auth: final_auth,
    }
}

fn postman_auth(a: &PostmanAuth) -> Auth {
    match a.auth_type.as_str() {
        "bearer" => Auth::Bearer {
            token: extract_auth(&a.extra, "bearer", "token"),
        },
        "basic" => Auth::Basic {
            username: extract_auth(&a.extra, "basic", "username"),
            password: extract_auth(&a.extra, "basic", "password"),
        },
        _ => Auth::None,
    }
}

fn extract_auth(extra: &serde_json::Map<String, Value>, section: &str, key: &str) -> String {
    match extra.get(section) {
        Some(Value::Array(arr)) => {
            for item in arr {
                if item.get("key").and_then(|v| v.as_str()) == Some(key) {
                    return item
                        .get("value")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }
            }
            String::new()
        }
        Some(Value::Object(obj)) => obj
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

fn promote_auth_header(headers: Vec<KvRow>, auth: Auth) -> (Vec<KvRow>, Auth) {
    let mut out = Vec::with_capacity(headers.len());
    let mut final_auth = auth;
    for h in headers {
        if h.key.eq_ignore_ascii_case("Authorization") && matches!(final_auth, Auth::None) {
            let trimmed = h.value.trim();
            if let Some(rest) = trimmed
                .strip_prefix("Bearer ")
                .or_else(|| trimmed.strip_prefix("bearer "))
            {
                final_auth = Auth::Bearer {
                    token: rest.to_string(),
                };
                continue;
            }
        }
        out.push(h);
    }
    (out, final_auth)
}

fn parse_method(s: &str) -> HttpMethod {
    match s.to_ascii_uppercase().as_str() {
        "GET" => HttpMethod::GET,
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "DELETE" => HttpMethod::DELETE,
        "PATCH" => HttpMethod::PATCH,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET,
    }
}

fn strip_query(url: &str) -> String {
    match url.split_once('?') {
        Some((base, _)) => base.to_string(),
        None => url.to_string(),
    }
}

fn split_url_basic(full: &str) -> (String, Vec<KvRow>) {
    match full.split_once('?') {
        None => (full.to_string(), Vec::new()),
        Some((base, query)) => {
            let params = query
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|p| match p.split_once('=') {
                    Some((k, v)) => KvRow::new(k, v),
                    None => KvRow::new(p, ""),
                })
                .collect();
            (base.to_string(), params)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_json() {
        let folders = vec![Folder {
            id: "x".into(),
            name: "Test".into(),
            requests: vec![Request {
                id: "r".into(),
                name: "GET root".into(),
                method: HttpMethod::GET,
                url: "https://example.com".into(),
                query_params: vec![KvRow::new("q", "1")],
                headers: vec![KvRow::new("X-Foo", "bar")],
                cookies: vec![],
                body: String::new(),
                auth: Auth::None,
            }],
            subfolders: vec![],
        }];
        let s = export_string(&folders, Format::Json).unwrap();
        let back = import_from_str(&s, "json").unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].name, "Test");
        assert_eq!(back[0].requests.len(), 1);
        assert_eq!(back[0].requests[0].url, "https://example.com");
    }

    #[test]
    fn round_trip_yaml() {
        let folders = vec![Folder {
            id: "x".into(),
            name: "Test".into(),
            requests: vec![],
            subfolders: vec![],
        }];
        let s = export_string(&folders, Format::Yaml).unwrap();
        let back = import_from_str(&s, "yaml").unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].name, "Test");
    }

    #[test]
    fn import_postman_v21() {
        let postman = r#"{
            "info": {
                "name": "My API",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Users",
                    "item": [
                        {
                            "name": "List users",
                            "request": {
                                "method": "GET",
                                "header": [
                                    {"key": "X-Req-Id", "value": "abc"}
                                ],
                                "url": {
                                    "raw": "https://api.example.com/users?page=1",
                                    "query": [
                                        {"key": "page", "value": "1"}
                                    ]
                                },
                                "auth": {
                                    "type": "bearer",
                                    "bearer": [
                                        {"key": "token", "value": "TOK123"}
                                    ]
                                }
                            }
                        }
                    ]
                },
                {
                    "name": "Ping",
                    "request": {
                        "method": "POST",
                        "header": [],
                        "body": {"mode": "raw", "raw": "{\"hi\":true}"},
                        "url": "https://api.example.com/ping"
                    }
                }
            ]
        }"#;
        let folders = import_from_str(postman, "json").unwrap();
        assert_eq!(folders.len(), 1);
        let root = &folders[0];
        assert_eq!(root.name, "My API");
        assert_eq!(root.requests.len(), 1);
        assert_eq!(root.requests[0].method, HttpMethod::POST);
        assert_eq!(root.requests[0].body, "{\"hi\":true}");
        assert_eq!(root.subfolders.len(), 1);
        assert_eq!(root.subfolders[0].name, "Users");
        let req = &root.subfolders[0].requests[0];
        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.url, "https://api.example.com/users");
        assert_eq!(req.query_params.len(), 1);
        assert_eq!(req.query_params[0].key, "page");
        assert_eq!(req.query_params[0].value, "1");
        assert!(matches!(&req.auth, Auth::Bearer { token } if token == "TOK123"));
    }

    #[test]
    #[ignore]
    fn debug_user_postman_file() {
        let path = "/Users/nurchudlori/Documents/Personal.postman_collection.json";
        let content = std::fs::read_to_string(path).expect("read user file");
        let result = import_from_str(&content, "json");
        match result {
            Ok(folders) => {
                fn walk(f: &Folder, depth: usize) -> (usize, usize) {
                    let mut reqs = f.requests.len();
                    let mut subs = f.subfolders.len();
                    for s in &f.subfolders {
                        let (r, sub) = walk(s, depth + 1);
                        reqs += r;
                        subs += sub;
                    }
                    (reqs, subs)
                }
                println!("Imported {} top-level folder(s)", folders.len());
                for f in &folders {
                    let (r, s) = walk(f, 0);
                    println!("  {} → {} requests, {} subfolders total", f.name, r, s);
                }
            }
            Err(e) => panic!("import failed: {}", e),
        }
    }

    #[test]
    fn import_postman_basic_auth_object_form() {
        let postman = r#"{
            "info": {"name": "X", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
            "item": [{
                "name": "One",
                "request": {
                    "method": "GET",
                    "url": "https://x.com",
                    "auth": {
                        "type": "basic",
                        "basic": {"username": "u", "password": "p"}
                    }
                }
            }]
        }"#;
        let folders = import_from_str(postman, "json").unwrap();
        let req = &folders[0].requests[0];
        assert!(matches!(&req.auth, Auth::Basic { username, password } if username == "u" && password == "p"));
    }
}
