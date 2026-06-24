pub mod curl;
pub mod git_workspace;

use crate::model::{
    Auth, Folder, HttpMethod, KvRow, OpenApiSource, Request, RequestSource, SyncConfig,
};
use crate::privacy::is_sensitive_key;
use crate::secret_scanner;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use uuid::Uuid;

const MAX_IMPORT_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
pub struct Export {
    pub version: String,
    pub folders: Vec<Folder>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Json,
    Yaml,
}

pub fn export_string(folders: &[Folder], format: Format) -> Result<String, String> {
    export_folders(folders, format)
}

pub fn export_string_redacted(folders: &[Folder], format: Format) -> Result<String, String> {
    let redacted = secret_scanner::redact_folders(folders);
    export_folders(&redacted, format)
}

fn export_folders(folders: &[Folder], format: Format) -> Result<String, String> {
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
    let size = std::fs::metadata(path)
        .map_err(|e| format!("Read error: {}", e))?
        .len();
    if size > MAX_IMPORT_BYTES {
        return Err(format!(
            "Import file is too large ({} MB max)",
            MAX_IMPORT_BYTES / 1024 / 1024
        ));
    }
    let content = std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
    import_from_str(
        &content,
        path.extension().and_then(|s| s.to_str()).unwrap_or(""),
    )
}

pub fn import_from_str(content: &str, ext_hint: &str) -> Result<Vec<Folder>, String> {
    let is_yaml = ext_hint.eq_ignore_ascii_case("yaml") || ext_hint.eq_ignore_ascii_case("yml");

    if is_yaml {
        let folders = try_parse_yaml(content)?;
        return Ok(regen_ids_all(folders));
    }

    // JSON: try Postman first (by shape), then our native, then YAML fallback
    if let Ok(value) = serde_json::from_str::<Value>(content) {
        if looks_like_openapi(&value) {
            return Ok(regen_ids_all(openapi_to_folders(&value)?));
        }

        if looks_like_postman(&value) {
            let pc: PostmanCollection =
                serde_json::from_value(value).map_err(|e| format!("Postman parse error: {}", e))?;
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
    if let Ok(value) = serde_yaml::from_str::<Value>(content) {
        if looks_like_openapi(&value) {
            return openapi_to_folders(&value);
        }
    }

    if let Ok(export) = serde_yaml::from_str::<Export>(content) {
        return Ok(export.folders);
    }
    if let Ok(folders) = serde_yaml::from_str::<Vec<Folder>>(content) {
        return Ok(folders);
    }
    if let Ok(folder) = serde_yaml::from_str::<Folder>(content) {
        return Ok(vec![folder]);
    }
    Err("Could not parse as JSON, YAML, OpenAPI, or Postman collection".to_string())
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

fn looks_like_openapi(value: &Value) -> bool {
    value
        .get("openapi")
        .and_then(|v| v.as_str())
        .map(|v| v.starts_with("3."))
        .unwrap_or(false)
        && value.get("paths").and_then(|v| v.as_object()).is_some()
}

fn openapi_to_folders(root: &Value) -> Result<Vec<Folder>, String> {
    let paths = root
        .get("paths")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "OpenAPI parse error: missing paths object".to_string())?;

    let mut folders = Vec::new();
    for (path, path_item) in paths {
        let Some(path_item_obj) = path_item.as_object() else {
            continue;
        };

        for method_name in [
            "get", "post", "put", "delete", "patch", "query", "head", "options",
        ] {
            let Some(operation) = path_item_obj.get(method_name) else {
                continue;
            };
            let Some(method) = openapi_method(method_name) else {
                continue;
            };
            if !operation.is_object() {
                continue;
            }

            let folder_name =
                openapi_operation_group(operation).unwrap_or_else(|| openapi_path_root(path));
            let request =
                openapi_operation_to_request(root, path, method, method_name, path_item, operation);
            folder_by_name_mut(&mut folders, &folder_name)
                .requests
                .push(request);
        }
    }

    if folders.is_empty() {
        return Err("OpenAPI parse error: no supported operations found".to_string());
    }

    Ok(folders)
}

#[allow(dead_code)]
pub fn refresh_openapi_folders(folders: &mut [Folder], content: &str) -> Result<usize, String> {
    let value = serde_json::from_str::<Value>(content)
        .or_else(|_| serde_yaml::from_str::<Value>(content))
        .map_err(|e| format!("OpenAPI parse error: {}", e))?;
    if !looks_like_openapi(&value) {
        return Err("OpenAPI parse error: expected OpenAPI 3.x document".to_string());
    }

    let generated = openapi_to_folders(&value)?;
    let generated_requests = generated
        .iter()
        .flat_map(|folder| folder.requests.iter())
        .collect::<Vec<_>>();

    Ok(refresh_openapi_folder_slice(folders, &generated_requests))
}

fn refresh_openapi_folder_slice(folders: &mut [Folder], generated_requests: &[&Request]) -> usize {
    let mut updated = 0;
    for folder in folders {
        for request in &mut folder.requests {
            let Some(existing_key) = openapi_request_key(request) else {
                continue;
            };
            let Some(generated) = generated_requests.iter().find(|candidate| {
                openapi_request_key(candidate)
                    .as_ref()
                    .map(|candidate_key| openapi_keys_match(&existing_key, candidate_key))
                    .unwrap_or(false)
            }) else {
                continue;
            };
            merge_openapi_refresh(request, generated);
            updated += 1;
        }
        updated += refresh_openapi_folder_slice(&mut folder.subfolders, generated_requests);
    }
    updated
}

fn folder_by_name_mut<'a>(folders: &'a mut Vec<Folder>, name: &str) -> &'a mut Folder {
    if let Some(idx) = folders.iter().position(|f| f.name == name) {
        return &mut folders[idx];
    }
    folders.push(Folder {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        requests: Vec::new(),
        subfolders: Vec::new(),
        description: String::new(),
        sync: SyncConfig::default(),
    });
    folders.last_mut().expect("folder was just pushed")
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OpenApiRequestKey {
    operation_id: Option<String>,
    method: String,
    path: String,
}

fn openapi_request_key(request: &Request) -> Option<OpenApiRequestKey> {
    let RequestSource::OpenApi(source) = request.source.as_ref()?;
    Some(OpenApiRequestKey {
        operation_id: (!source.operation_id.is_empty()).then(|| source.operation_id.clone()),
        method: source.method.to_ascii_uppercase(),
        path: source.path.clone(),
    })
}

fn openapi_keys_match(left: &OpenApiRequestKey, right: &OpenApiRequestKey) -> bool {
    match (&left.operation_id, &right.operation_id) {
        (Some(left), Some(right)) if left == right => true,
        _ => left.method == right.method && left.path == right.path,
    }
}

fn merge_openapi_refresh(existing: &mut Request, generated: &Request) {
    let old_source = match existing.source.as_ref() {
        Some(RequestSource::OpenApi(source)) => source.clone(),
        _ => OpenApiSource {
            operation_id: String::new(),
            method: String::new(),
            path: String::new(),
            generated_query_keys: Vec::new(),
            generated_path_keys: Vec::new(),
            generated_header_keys: Vec::new(),
        },
    };

    existing.method = generated.method.clone();
    existing.url = generated.url.clone();
    existing.description = generated.description.clone();
    existing.query_params = merge_openapi_rows(
        &existing.query_params,
        &generated.query_params,
        &old_source.generated_query_keys,
    );
    existing.path_params = merge_openapi_rows(
        &existing.path_params,
        &generated.path_params,
        &old_source.generated_path_keys,
    );
    existing.headers = merge_openapi_rows(
        &existing.headers,
        &generated.headers,
        &old_source.generated_header_keys,
    );
    existing.body = generated.body.clone();
    existing.body_ext = generated.body_ext.clone();
    if matches!(existing.auth, Auth::None) {
        existing.auth = generated.auth.clone();
    }
    existing.source = generated.source.clone();
}

fn merge_openapi_rows(
    existing: &[KvRow],
    generated: &[KvRow],
    old_generated_keys: &[String],
) -> Vec<KvRow> {
    let mut rows = generated.to_vec();
    for row in existing {
        if old_generated_keys.iter().any(|key| key == &row.key) {
            continue;
        }
        if rows.iter().any(|generated| generated.key == row.key) {
            continue;
        }
        rows.push(row.clone());
    }
    rows
}

fn openapi_operation_to_request(
    root: &Value,
    path: &str,
    method: HttpMethod,
    method_name: &str,
    path_item: &Value,
    operation: &Value,
) -> Request {
    let mut query_params = Vec::new();
    let mut path_params = Vec::new();
    let mut headers = Vec::new();
    collect_openapi_parameters(
        path_item.get("parameters"),
        root,
        &mut query_params,
        &mut path_params,
        &mut headers,
    );
    collect_openapi_parameters(
        operation.get("parameters"),
        root,
        &mut query_params,
        &mut path_params,
        &mut headers,
    );

    let generated_query_keys = kv_keys(&query_params);
    let generated_path_keys = kv_keys(&path_params);
    let generated_header_keys = kv_keys(&headers);

    Request {
        id: Uuid::new_v4().to_string(),
        name: openapi_operation_name(operation, method_name, path),
        description: openapi_operation_description(operation),
        method,
        url: path.to_string(),
        query_params,
        path_params,
        headers,
        cookies: Vec::new(),
        body: openapi_request_body(root, operation),
        body_ext: None,
        auth: openapi_auth_hint(root, operation),
        extractors: Vec::new(),
        assertions: Vec::new(),
        source: Some(RequestSource::OpenApi(OpenApiSource {
            operation_id: operation
                .get("operationId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            method: method_name.to_ascii_uppercase(),
            path: path.to_string(),
            generated_query_keys,
            generated_path_keys,
            generated_header_keys,
        })),
    }
}

fn openapi_method(method_name: &str) -> Option<HttpMethod> {
    match method_name {
        "get" => Some(HttpMethod::GET),
        "post" => Some(HttpMethod::POST),
        "put" => Some(HttpMethod::PUT),
        "delete" => Some(HttpMethod::DELETE),
        "patch" => Some(HttpMethod::PATCH),
        "query" => Some(HttpMethod::QUERY),
        "head" => Some(HttpMethod::HEAD),
        "options" => Some(HttpMethod::OPTIONS),
        _ => None,
    }
}

fn openapi_operation_group(operation: &Value) -> Option<String> {
    operation
        .get("tags")
        .and_then(|v| v.as_array())
        .and_then(|tags| tags.iter().find_map(|tag| tag.as_str()))
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToString::to_string)
}

fn openapi_path_root(path: &str) -> String {
    path.trim_start_matches('/')
        .split('/')
        .find(|part| !part.is_empty())
        .map(|part| part.trim_matches('{').trim_matches('}'))
        .filter(|part| !part.is_empty())
        .unwrap_or("Root")
        .to_string()
}

fn openapi_operation_name(operation: &Value, method_name: &str, path: &str) -> String {
    for field in ["summary", "operationId"] {
        if let Some(name) = operation.get(field).and_then(|v| v.as_str()) {
            let name = name.trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }
    format!("{} {}", method_name.to_ascii_uppercase(), path)
}

fn openapi_operation_description(operation: &Value) -> String {
    operation
        .get("description")
        .or_else(|| operation.get("summary"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn collect_openapi_parameters(
    parameters: Option<&Value>,
    root: &Value,
    query_params: &mut Vec<KvRow>,
    path_params: &mut Vec<KvRow>,
    headers: &mut Vec<KvRow>,
) {
    let Some(parameters) = parameters.and_then(|v| v.as_array()) else {
        return;
    };

    for parameter in parameters {
        let parameter = resolve_local_ref(parameter, root);
        let Some(name) = parameter.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(location) = parameter.get("in").and_then(|v| v.as_str()) else {
            continue;
        };
        let row = KvRow {
            enabled: true,
            key: name.to_string(),
            value: openapi_parameter_value(name, parameter, root),
            description: parameter
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        };

        match location {
            "query" => upsert_kv_row(query_params, row),
            "path" => upsert_kv_row(path_params, row),
            "header" => upsert_kv_row(headers, row),
            _ => {}
        }
    }
}

fn kv_keys(rows: &[KvRow]) -> Vec<String> {
    rows.iter()
        .map(|row| row.key.clone())
        .filter(|key| !key.is_empty())
        .collect()
}

fn upsert_kv_row(rows: &mut Vec<KvRow>, row: KvRow) {
    if let Some(existing) = rows.iter_mut().find(|existing| existing.key == row.key) {
        *existing = row;
    } else {
        rows.push(row);
    }
}

fn openapi_parameter_value(name: &str, parameter: &Value, root: &Value) -> String {
    if is_sensitive_key(name) {
        return String::new();
    }

    [
        parameter.get("example"),
        first_openapi_example(parameter.get("examples"), root),
        parameter.get("default"),
        parameter.get("schema").and_then(|schema| {
            let schema = resolve_local_ref(schema, root);
            schema.get("example").or_else(|| schema.get("default"))
        }),
    ]
    .into_iter()
    .flatten()
    .next()
    .map(json_value_to_field)
    .unwrap_or_default()
}

fn first_openapi_example<'a>(examples: Option<&'a Value>, root: &'a Value) -> Option<&'a Value> {
    let examples = examples?.as_object()?;
    examples.values().find_map(|example| {
        let example = resolve_local_ref(example, root);
        example.get("value").or_else(|| {
            if example.get("externalValue").is_none() {
                Some(example)
            } else {
                None
            }
        })
    })
}

fn json_value_to_field(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn openapi_request_body(root: &Value, operation: &Value) -> String {
    let Some(request_body) = operation.get("requestBody") else {
        return String::new();
    };
    let request_body = resolve_local_ref(request_body, root);
    let Some(content) = request_body.get("content").and_then(|v| v.as_object()) else {
        return String::new();
    };
    let Some(media_type) = select_json_media_type(content) else {
        return String::new();
    };

    let body = media_type
        .get("example")
        .cloned()
        .or_else(|| first_openapi_example(media_type.get("examples"), root).cloned())
        .or_else(|| {
            media_type
                .get("schema")
                .and_then(|schema| json_example_for_schema(schema, root, 0))
        });

    body.map(|value| serde_json::to_string_pretty(&value).unwrap_or_default())
        .unwrap_or_default()
}

fn openapi_auth_hint(root: &Value, operation: &Value) -> Auth {
    let security = operation.get("security").or_else(|| root.get("security"));
    let Some(requirements) = security.and_then(|v| v.as_array()) else {
        return Auth::None;
    };

    for requirement in requirements {
        let Some(requirement) = requirement.as_object() else {
            continue;
        };
        for scheme_name in requirement.keys() {
            let Some(scheme) = root
                .pointer(&format!(
                    "/components/securitySchemes/{}",
                    escape_json_pointer_segment(scheme_name)
                ))
                .and_then(|v| v.as_object())
            else {
                continue;
            };

            match (
                scheme.get("type").and_then(|v| v.as_str()),
                scheme.get("scheme").and_then(|v| v.as_str()),
            ) {
                (Some("http"), Some(scheme)) if scheme.eq_ignore_ascii_case("bearer") => {
                    return Auth::Bearer {
                        token: String::new(),
                    };
                }
                (Some("http"), Some(scheme)) if scheme.eq_ignore_ascii_case("basic") => {
                    return Auth::Basic {
                        username: String::new(),
                        password: String::new(),
                    };
                }
                _ => {}
            }
        }
    }

    Auth::None
}

fn escape_json_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

fn select_json_media_type(content: &serde_json::Map<String, Value>) -> Option<&Value> {
    content.get("application/json").or_else(|| {
        content.iter().find_map(|(content_type, media_type)| {
            let content_type = content_type.to_ascii_lowercase();
            if content_type.ends_with("+json") || content_type.contains("/json") {
                Some(media_type)
            } else {
                None
            }
        })
    })
}

fn json_example_for_schema(schema: &Value, root: &Value, depth: usize) -> Option<Value> {
    if depth > 12 {
        return None;
    }
    let schema = resolve_local_ref(schema, root);

    for field in ["example", "default", "const"] {
        if let Some(value) = schema.get(field) {
            return Some(value.clone());
        }
    }
    if let Some(value) = schema
        .get("enum")
        .and_then(|v| v.as_array())
        .and_then(|values| values.first())
    {
        return Some(value.clone());
    }
    if let Some(composed) = json_example_from_composed_schema(schema, root, depth) {
        return Some(composed);
    }

    let schema_type = schema.get("type").and_then(|v| v.as_str());
    if schema_type == Some("object") || schema.get("properties").is_some() {
        let mut out = serde_json::Map::new();
        if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
            for (name, property_schema) in properties {
                if let Some(value) = json_example_for_schema(property_schema, root, depth + 1) {
                    out.insert(name.clone(), value);
                }
            }
        }
        return Some(Value::Object(out));
    }

    match schema_type {
        Some("array") => Some(Value::Array(
            schema
                .get("items")
                .and_then(|items| json_example_for_schema(items, root, depth + 1))
                .map(|value| vec![value])
                .unwrap_or_default(),
        )),
        Some("integer") => Some(Value::Number(0.into())),
        Some("number") => serde_json::Number::from_f64(0.0).map(Value::Number),
        Some("boolean") => Some(Value::Bool(false)),
        Some("string") => Some(Value::String(String::new())),
        _ => None,
    }
}

fn json_example_from_composed_schema(schema: &Value, root: &Value, depth: usize) -> Option<Value> {
    for key in ["oneOf", "anyOf"] {
        if let Some(value) = schema
            .get(key)
            .and_then(|v| v.as_array())
            .and_then(|schemas| schemas.first())
            .and_then(|schema| json_example_for_schema(schema, root, depth + 1))
        {
            return Some(value);
        }
    }

    let all_of = schema.get("allOf").and_then(|v| v.as_array())?;
    let mut merged = serde_json::Map::new();
    let mut fallback = None;
    for item in all_of {
        if let Some(value) = json_example_for_schema(item, root, depth + 1) {
            match value {
                Value::Object(obj) => merged.extend(obj),
                other => {
                    fallback.get_or_insert(other);
                }
            };
        }
    }

    if merged.is_empty() {
        fallback
    } else {
        Some(Value::Object(merged))
    }
}

fn resolve_local_ref<'a>(value: &'a Value, root: &'a Value) -> &'a Value {
    let Some(reference) = value.get("$ref").and_then(|v| v.as_str()) else {
        return value;
    };
    let Some(pointer) = reference.strip_prefix('#') else {
        return value;
    };
    root.pointer(pointer).unwrap_or(value)
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
                            let disabled =
                                q.get("disabled").and_then(|d| d.as_bool()).unwrap_or(false);
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
        description: String::new(),
        sync: SyncConfig::default(),
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
            description: String::new(),
            sync: SyncConfig::default(),
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
        description: String::new(),
        method,
        url,
        query_params,
        path_params: Vec::new(),
        headers: filtered_headers,
        cookies: Vec::new(),
        body,
        body_ext: None,
        auth: final_auth,
        extractors: Vec::new(),
        assertions: Vec::new(),
        source: None,
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
        "QUERY" => HttpMethod::QUERY,
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

    fn fixture_folders() -> Vec<Folder> {
        vec![Folder {
            id: "collection-1".into(),
            name: "Fixture API".into(),
            requests: vec![Request {
                id: "request-1".into(),
                name: "Create widget".into(),
                description: String::new(),
                method: HttpMethod::POST,
                url: "https://api.example.com/widgets".into(),
                query_params: vec![
                    KvRow::new("include", "details"),
                    KvRow {
                        enabled: false,
                        key: "debug".into(),
                        value: "true".into(),
                        description: "disabled query is preserved".into(),
                    },
                ],
                path_params: vec![],
                headers: vec![
                    KvRow::new("Content-Type", "application/json"),
                    KvRow {
                        enabled: false,
                        key: "X-Skip".into(),
                        value: "1".into(),
                        description: "disabled header is preserved".into(),
                    },
                ],
                cookies: vec![KvRow::new("session", "abc123")],
                body: "{\"name\":\"demo\"}".into(),
                body_ext: None,
                auth: Auth::Bearer {
                    token: "fixture-token".into(),
                },
                extractors: vec![],
                assertions: vec![],
                source: None,
            }],
            subfolders: vec![Folder {
                id: "collection-1-sub".into(),
                name: "Nested".into(),
                requests: vec![Request {
                    id: "request-2".into(),
                    name: "Status".into(),
                    description: String::new(),
                    method: HttpMethod::GET,
                    url: "https://api.example.com/status".into(),
                    query_params: vec![],
                    path_params: vec![],
                    headers: vec![],
                    cookies: vec![],
                    body: String::new(),
                    body_ext: None,
                    auth: Auth::None,
                    extractors: vec![],
                    assertions: vec![],
                    source: None,
                }],
                subfolders: vec![],
                description: "nested folder fixture".into(),
                sync: SyncConfig::default(),
            }],
            description: "top-level collection fixture".into(),
            sync: SyncConfig::default(),
        }]
    }

    fn assert_fixture_shape(folders: &[Folder]) {
        assert_eq!(folders.len(), 1);
        let root = &folders[0];
        assert_eq!(root.name, "Fixture API");
        assert_eq!(root.description, "top-level collection fixture");
        assert_eq!(root.requests.len(), 1);
        assert_eq!(root.subfolders.len(), 1);

        let create = &root.requests[0];
        assert_eq!(create.name, "Create widget");
        assert_eq!(create.method, HttpMethod::POST);
        assert_eq!(create.url, "https://api.example.com/widgets");
        assert_eq!(create.query_params.len(), 2);
        assert_eq!(create.query_params[0].key, "include");
        assert_eq!(create.query_params[0].value, "details");
        assert!(!create.query_params[1].enabled);
        assert_eq!(
            create.query_params[1].description,
            "disabled query is preserved"
        );
        assert_eq!(create.headers.len(), 2);
        assert_eq!(create.headers[0].key, "Content-Type");
        assert!(!create.headers[1].enabled);
        assert_eq!(create.cookies.len(), 1);
        assert_eq!(create.body, "{\"name\":\"demo\"}");
        assert!(matches!(&create.auth, Auth::Bearer { token } if token == "fixture-token"));

        let nested = &root.subfolders[0];
        assert_eq!(nested.name, "Nested");
        assert_eq!(nested.description, "nested folder fixture");
        assert_eq!(nested.requests.len(), 1);
        assert_eq!(nested.requests[0].name, "Status");
        assert_eq!(nested.requests[0].method, HttpMethod::GET);
    }

    #[test]
    fn round_trip_json() {
        let folders = vec![Folder {
            id: "x".into(),
            name: "Test".into(),
            requests: vec![Request {
                id: "r".into(),
                name: "GET root".into(),
                description: String::new(),
                method: HttpMethod::GET,
                url: "https://example.com".into(),
                query_params: vec![KvRow::new("q", "1")],
                path_params: vec![],
                headers: vec![KvRow::new("X-Foo", "bar")],
                cookies: vec![],
                body: String::new(),
                body_ext: None,
                auth: Auth::None,
                extractors: vec![],
                assertions: vec![],
                source: None,
            }],
            subfolders: vec![],
            description: String::new(),
            sync: SyncConfig::default(),
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
            description: String::new(),
            sync: SyncConfig::default(),
        }];
        let s = export_string(&folders, Format::Yaml).unwrap();
        let back = import_from_str(&s, "yaml").unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].name, "Test");
    }

    #[test]
    fn fixture_round_trip_json_preserves_collection_shape() {
        let folders = fixture_folders();
        let s = export_string(&folders, Format::Json).unwrap();
        assert!(s.contains("\"version\": \"1\""));

        let back = import_from_str(&s, "json").unwrap();
        assert_fixture_shape(&back);
        assert_ne!(back[0].id, "collection-1");
        assert_ne!(back[0].requests[0].id, "request-1");
    }

    #[test]
    fn fixture_round_trip_yaml_preserves_collection_shape() {
        let folders = fixture_folders();
        let s = export_string(&folders, Format::Yaml).unwrap();
        assert!(s.contains("version: '1'"));

        let back = import_from_str(&s, "yaml").unwrap();
        assert_fixture_shape(&back);
        assert_ne!(back[0].id, "collection-1");
        assert_ne!(back[0].subfolders[0].id, "collection-1-sub");
    }

    #[test]
    fn redacted_export_masks_secrets_and_stays_importable() {
        let folders = fixture_folders();
        let s = export_string_redacted(&folders, Format::Json).unwrap();

        assert!(s.contains(crate::secret_scanner::REDACTED));
        assert!(!s.contains("fixture-token"));
        assert!(!s.contains("abc123"));

        let back = import_from_str(&s, "json").unwrap();
        assert_eq!(
            back[0].requests[0].cookies[0].value,
            crate::secret_scanner::REDACTED
        );
        assert!(
            matches!(&back[0].requests[0].auth, Auth::Bearer { token } if token == crate::secret_scanner::REDACTED)
        );
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
                fn walk(f: &Folder, _depth: usize) -> (usize, usize) {
                    let mut reqs = f.requests.len();
                    let mut subs = f.subfolders.len();
                    for s in &f.subfolders {
                        let (r, sub) = walk(s, _depth + 1);
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
        assert!(
            matches!(&req.auth, Auth::Basic { username, password } if username == "u" && password == "p")
        );
    }

    #[test]
    fn import_postman_edge_cases_preserve_disabled_rows_and_promote_bearer_header() {
        let postman = r#"{
            "info": {"name": "Edge Cases", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
            "item": [
                {
                    "name": "",
                    "item": [{
                        "name": "",
                        "request": {
                            "method": "PATCH",
                            "header": [
                                {"key": "Authorization", "value": "Bearer from-header"},
                                {"key": "X-Disabled", "value": "nope", "disabled": true},
                                {"key": "", "value": "ignored"}
                            ],
                            "body": {"mode": "raw", "raw": "{\"patched\":true}"},
                            "url": {
                                "raw": "https://api.example.com/items/7?active=true&draft=false",
                                "query": [
                                    {"key": "active", "value": "true"},
                                    {"key": "draft", "value": "false", "disabled": true}
                                ]
                            }
                        }
                    }]
                },
                {
                    "name": "String URL",
                    "request": {
                        "method": "NOPE",
                        "url": "https://api.example.com/search?q=rust&empty"
                    }
                }
            ]
        }"#;

        let folders = import_from_str(postman, "json").unwrap();
        let root = &folders[0];
        assert_eq!(root.name, "Edge Cases");
        assert_eq!(root.requests.len(), 1);
        assert_eq!(root.requests[0].name, "String URL");
        assert_eq!(root.requests[0].method, HttpMethod::GET);
        assert_eq!(root.requests[0].url, "https://api.example.com/search");
        assert_eq!(root.requests[0].query_params[0].key, "q");
        assert_eq!(root.requests[0].query_params[1].key, "empty");
        assert_eq!(root.requests[0].query_params[1].value, "");

        let sub = &root.subfolders[0];
        assert_eq!(sub.name, "Folder");
        let req = &sub.requests[0];
        assert_eq!(req.name, "Request");
        assert_eq!(req.method, HttpMethod::PATCH);
        assert_eq!(req.url, "https://api.example.com/items/7");
        assert_eq!(req.body, "{\"patched\":true}");
        assert_eq!(req.query_params.len(), 2);
        assert_eq!(req.query_params[0].key, "active");
        assert!(req.query_params[0].enabled);
        assert_eq!(req.query_params[1].key, "draft");
        assert!(!req.query_params[1].enabled);
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers[0].key, "X-Disabled");
        assert!(!req.headers[0].enabled);
        assert!(matches!(&req.auth, Auth::Bearer { token } if token == "from-header"));
    }

    #[test]
    fn import_openapi_json_groups_by_first_tag() {
        let openapi = r##"{
            "openapi": "3.0.3",
            "info": {"title": "Example API", "version": "1.0.0"},
            "security": [{"bearerAuth": []}],
            "paths": {
                "/users": {
                    "parameters": [
                        {
                            "name": "X-Tenant",
                            "in": "header",
                            "schema": {"type": "string", "default": "acme"}
                        }
                    ],
                    "get": {
                        "tags": ["Users"],
                        "summary": "List users",
                        "parameters": [
                        {
                            "name": "page",
                            "in": "query",
                            "schema": {"type": "integer", "default": 2}
                        },
                        {
                            "name": "Authorization",
                            "in": "header",
                            "example": "Bearer should-not-persist"
                        }
                    ]
                },
                    "post": {
                        "tags": ["Users", "Admin"],
                        "operationId": "createUser",
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/NewUser"}
                                }
                            }
                        }
                    }
                }
            },
            "components": {
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer"
                    }
                },
                "schemas": {
                    "NewUser": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string", "example": "Ada"},
                            "active": {"type": "boolean", "default": true}
                        }
                    }
                }
            }
        }"##;

        let folders = import_from_str(openapi, "json").unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].name, "Users");
        assert_eq!(folders[0].requests.len(), 2);

        let list = &folders[0].requests[0];
        assert_eq!(list.name, "List users");
        assert_eq!(list.description, "List users");
        assert_eq!(list.method, HttpMethod::GET);
        assert_eq!(list.url, "/users");
        assert!(matches!(list.auth, Auth::Bearer { ref token } if token.is_empty()));
        assert!(matches!(list.source, Some(RequestSource::OpenApi(_))));
        assert_eq!(list.query_params.len(), 1);
        assert_eq!(list.query_params[0].key, "page");
        assert_eq!(list.query_params[0].value, "2");
        assert_eq!(list.headers.len(), 2);
        assert_eq!(list.headers[0].key, "X-Tenant");
        assert_eq!(list.headers[0].value, "acme");
        assert_eq!(list.headers[1].key, "Authorization");
        assert_eq!(list.headers[1].value, "");

        let create = &folders[0].requests[1];
        assert_eq!(create.name, "createUser");
        assert_eq!(create.method, HttpMethod::POST);
        assert_eq!(
            serde_json::from_str::<Value>(&create.body).unwrap(),
            serde_json::json!({"active": true, "name": "Ada"})
        );
    }

    #[test]
    fn import_openapi_yaml_groups_by_path_root_without_tag() {
        let openapi = r#"
openapi: 3.1.0
info:
  title: Store API
  version: 1.0.0
paths:
  /orders/{orderId}:
    patch:
      summary: Update order
      parameters:
        - name: orderId
          in: path
          example: ORD-1
        - name: trace
          in: header
          example: abc-123
      requestBody:
        content:
          application/json:
            example:
              status: shipped
"#;

        let folders = import_from_str(openapi, "yaml").unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].name, "orders");
        assert_eq!(folders[0].requests.len(), 1);

        let req = &folders[0].requests[0];
        assert_eq!(req.name, "Update order");
        assert_eq!(req.method, HttpMethod::PATCH);
        assert_eq!(req.url, "/orders/{orderId}");
        assert_eq!(req.path_params.len(), 1);
        assert_eq!(req.path_params[0].key, "orderId");
        assert_eq!(req.path_params[0].value, "ORD-1");
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers[0].key, "trace");
        assert_eq!(req.headers[0].value, "abc-123");
        assert_eq!(
            serde_json::from_str::<Value>(&req.body).unwrap(),
            serde_json::json!({"status": "shipped"})
        );
    }

    #[test]
    fn import_openapi_query_operation() {
        let openapi = r#"
openapi: 3.1.0
info:
  title: Search API
  version: 1.0.0
paths:
  /search:
    query:
      summary: Complex search
      requestBody:
        content:
          application/json:
            example:
              filter: active
"#;

        let folders = import_from_str(openapi, "yaml").unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].requests.len(), 1);

        let req = &folders[0].requests[0];
        assert_eq!(req.name, "Complex search");
        assert_eq!(req.method, HttpMethod::QUERY);
        assert_eq!(req.url, "/search");
        assert_eq!(
            serde_json::from_str::<Value>(&req.body).unwrap(),
            serde_json::json!({"filter": "active"})
        );
    }

    #[test]
    fn refresh_openapi_updates_generated_fields_and_preserves_user_edits() {
        let v1 = r##"{
            "openapi": "3.0.3",
            "info": {"title": "Example API", "version": "1.0.0"},
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "listUsers",
                        "summary": "List users",
                        "parameters": [
                            {"name": "page", "in": "query", "schema": {"default": 1}},
                            {"name": "X-Trace", "in": "header", "example": "old"}
                        ]
                    }
                }
            }
        }"##;
        let v2 = r##"{
            "openapi": "3.0.3",
            "info": {"title": "Example API", "version": "1.1.0"},
            "security": [{"basicAuth": []}],
            "paths": {
                "/members": {
                    "get": {
                        "operationId": "listUsers",
                        "summary": "List members",
                        "parameters": [
                            {"name": "page", "in": "query", "schema": {"default": 3}},
                            {"name": "sort", "in": "query", "schema": {"default": "name"}},
                            {"name": "X-Trace", "in": "header", "example": "new"}
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "example": {"filter": "active"}
                                }
                            }
                        }
                    }
                }
            },
            "components": {
                "securitySchemes": {
                    "basicAuth": {"type": "http", "scheme": "basic"}
                }
            }
        }"##;

        let mut folders = import_from_str(v1, "json").unwrap();
        let req = &mut folders[0].requests[0];
        req.name = "My saved list".to_string();
        req.headers.push(KvRow::new("X-Custom", "keep"));
        req.auth = Auth::Bearer {
            token: "saved-token".to_string(),
        };

        let updated = refresh_openapi_folders(&mut folders, v2).unwrap();

        assert_eq!(updated, 1);
        let req = &folders[0].requests[0];
        assert_eq!(req.name, "My saved list");
        assert_eq!(req.description, "List members");
        assert_eq!(req.url, "/members");
        assert_eq!(req.query_params.len(), 2);
        assert_eq!(req.query_params[0].key, "page");
        assert_eq!(req.query_params[0].value, "3");
        assert_eq!(req.query_params[1].key, "sort");
        assert_eq!(req.query_params[1].value, "name");
        assert_eq!(req.headers.len(), 2);
        assert_eq!(req.headers[0].key, "X-Trace");
        assert_eq!(req.headers[0].value, "new");
        assert_eq!(req.headers[1].key, "X-Custom");
        assert_eq!(req.headers[1].value, "keep");
        assert!(matches!(&req.auth, Auth::Bearer { token } if token == "saved-token"));
        assert_eq!(
            serde_json::from_str::<Value>(&req.body).unwrap(),
            serde_json::json!({"filter": "active"})
        );
    }
}
