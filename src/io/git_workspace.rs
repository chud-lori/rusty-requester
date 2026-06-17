#![allow(dead_code)]

use crate::model::{Auth, BodyExt, Folder, KvRow, OAuth2State, Request};
use crate::privacy::{is_sensitive_key, mask_secret_value, redact_url_query_and_fragment};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

const FORMAT_NAME: &str = "rusty-requester-git-workspace";
const FORMAT_VERSION: u32 = 1;
const MANIFEST_FILE: &str = "workspace.json";
const REQUESTS_DIR: &str = "requests";
const MAX_REQUEST_FILES: usize = 5_000;
const MAX_FOLDER_DEPTH: usize = 32;
const MAX_IMPORT_BYTES: u64 = 50 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretPolicy {
    /// Default for Git-friendly exports: keep request structure but mask
    /// known secret-bearing values so accidental commits do not leak tokens.
    Mask,
    /// Lossless export for private repos or local-only backups.
    Include,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportOptions {
    pub secret_policy: SecretPolicy,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            secret_policy: SecretPolicy::Mask,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportSummary {
    pub manifest_path: PathBuf,
    pub request_files: usize,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    secrets: ManifestSecretPolicy,
    folders: Vec<ManifestFolder>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ManifestSecretPolicy {
    Masked,
    Included,
}

#[derive(Serialize, Deserialize)]
struct ManifestFolder {
    id: String,
    name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    description: String,
    #[serde(default)]
    requests: Vec<ManifestRequest>,
    #[serde(default)]
    subfolders: Vec<ManifestFolder>,
}

#[derive(Serialize, Deserialize)]
struct ManifestRequest {
    id: String,
    path: String,
}

pub fn export_workspace_to_dir(
    folders: &[Folder],
    root: &Path,
    options: ExportOptions,
) -> Result<ExportSummary, String> {
    fs::create_dir_all(root).map_err(|e| format!("Create export directory: {}", e))?;

    let requests_root = root.join(REQUESTS_DIR);
    if requests_root.exists() {
        fs::remove_dir_all(&requests_root).map_err(|e| format!("Clean request files: {}", e))?;
    }
    fs::create_dir_all(&requests_root).map_err(|e| format!("Create request directory: {}", e))?;

    let mut request_files = Vec::new();
    let folders = manifest_folders(folders, options, &mut request_files)?;
    request_files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let mut seen_paths = BTreeSet::new();
    for request_file in &request_files {
        if !seen_paths.insert(request_file.relative_path.clone()) {
            return Err(format!(
                "Duplicate export path generated: {}",
                request_file.relative_path.display()
            ));
        }
        let full_path = root.join(&request_file.relative_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Create request directory: {}", e))?;
        }
        write_json_file(&full_path, &request_file.request)
            .map_err(|e| format!("Write {}: {}", full_path.display(), e))?;
    }

    let manifest = Manifest {
        format: FORMAT_NAME.to_string(),
        version: FORMAT_VERSION,
        secrets: match options.secret_policy {
            SecretPolicy::Mask => ManifestSecretPolicy::Masked,
            SecretPolicy::Include => ManifestSecretPolicy::Included,
        },
        folders,
    };
    let manifest_path = root.join(MANIFEST_FILE);
    write_json_file(&manifest_path, &manifest)
        .map_err(|e| format!("Write {}: {}", manifest_path.display(), e))?;

    Ok(ExportSummary {
        manifest_path,
        request_files: request_files.len(),
    })
}

pub fn import_workspace_from_dir(root: &Path) -> Result<Vec<Folder>, String> {
    let manifest_path = root.join(MANIFEST_FILE);
    let mut budget = ImportBudget::default();
    let manifest_content = read_bounded_file(&manifest_path, &mut budget)?;
    let manifest: Manifest = serde_json::from_str(&manifest_content)
        .map_err(|e| format!("Parse {}: {}", manifest_path.display(), e))?;

    if manifest.format != FORMAT_NAME {
        return Err(format!(
            "Unsupported workspace format: expected {}, got {}",
            FORMAT_NAME, manifest.format
        ));
    }
    if manifest.version != FORMAT_VERSION {
        return Err(format!(
            "Unsupported workspace format version: {}",
            manifest.version
        ));
    }

    import_manifest_folders(root, &manifest.folders, &mut budget, 0)
}

struct RequestFile {
    relative_path: PathBuf,
    request: Request,
}

fn manifest_folders(
    folders: &[Folder],
    options: ExportOptions,
    request_files: &mut Vec<RequestFile>,
) -> Result<Vec<ManifestFolder>, String> {
    let mut out = Vec::with_capacity(folders.len());
    for (index, folder) in folders.iter().enumerate() {
        out.push(manifest_folder(
            folder,
            options,
            request_files,
            Vec::new(),
            index,
        )?);
    }
    Ok(out)
}

fn manifest_folder(
    folder: &Folder,
    options: ExportOptions,
    request_files: &mut Vec<RequestFile>,
    mut parent_components: Vec<String>,
    index: usize,
) -> Result<ManifestFolder, String> {
    let folder_component = path_component(index, &folder.name, &folder.id, "folder");
    parent_components.push(folder_component);

    let mut requests = Vec::with_capacity(folder.requests.len());
    for (request_index, request) in folder.requests.iter().enumerate() {
        let mut request_path = PathBuf::from(REQUESTS_DIR);
        for component in &parent_components {
            request_path.push(component);
        }
        request_path.push(format!(
            "{}.json",
            path_component(request_index, &request.name, &request.id, "request")
        ));

        requests.push(ManifestRequest {
            id: request.id.clone(),
            path: path_to_manifest_string(&request_path)?,
        });
        request_files.push(RequestFile {
            relative_path: request_path,
            request: export_request(request, options.secret_policy),
        });
    }

    let mut subfolders = Vec::with_capacity(folder.subfolders.len());
    for (sub_index, subfolder) in folder.subfolders.iter().enumerate() {
        subfolders.push(manifest_folder(
            subfolder,
            options,
            request_files,
            parent_components.clone(),
            sub_index,
        )?);
    }

    Ok(ManifestFolder {
        id: folder.id.clone(),
        name: folder.name.clone(),
        description: folder.description.clone(),
        requests,
        subfolders,
    })
}

fn import_manifest_folders(
    root: &Path,
    folders: &[ManifestFolder],
    budget: &mut ImportBudget,
    depth: usize,
) -> Result<Vec<Folder>, String> {
    if depth > MAX_FOLDER_DEPTH {
        return Err(format!(
            "Workspace folder depth exceeds {}",
            MAX_FOLDER_DEPTH
        ));
    }

    let mut out = Vec::with_capacity(folders.len());
    for folder in folders {
        let mut requests = Vec::with_capacity(folder.requests.len());
        for request_ref in &folder.requests {
            budget.request_files += 1;
            if budget.request_files > MAX_REQUEST_FILES {
                return Err(format!(
                    "Workspace has too many request files ({} max)",
                    MAX_REQUEST_FILES
                ));
            }

            let relative = validate_manifest_path(&request_ref.path)?;
            let path = root.join(relative);
            let content = read_bounded_file(&path, budget)?;
            let request: Request = serde_json::from_str(&content)
                .map_err(|e| format!("Parse {}: {}", path.display(), e))?;
            if request.id != request_ref.id {
                return Err(format!(
                    "Request ID mismatch for {}: manifest has {}, file has {}",
                    request_ref.path, request_ref.id, request.id
                ));
            }
            requests.push(request);
        }

        out.push(Folder {
            id: folder.id.clone(),
            name: folder.name.clone(),
            requests,
            subfolders: import_manifest_folders(root, &folder.subfolders, budget, depth + 1)?,
            description: folder.description.clone(),
        });
    }
    Ok(out)
}

#[derive(Default)]
struct ImportBudget {
    total_bytes: u64,
    request_files: usize,
}

fn read_bounded_file(path: &Path, budget: &mut ImportBudget) -> Result<String, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|e| format!("Read {}: {}", path.display(), e))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("Workspace file is a symlink: {}", path.display()));
    }
    if !metadata.is_file() {
        return Err(format!("Workspace path is not a file: {}", path.display()));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(format!(
            "Workspace file is too large: {} ({} MB max)",
            path.display(),
            MAX_FILE_BYTES / 1024 / 1024
        ));
    }
    budget.total_bytes = budget
        .total_bytes
        .checked_add(metadata.len())
        .ok_or_else(|| "Workspace import size overflow".to_string())?;
    if budget.total_bytes > MAX_IMPORT_BYTES {
        return Err(format!(
            "Workspace import is too large ({} MB max)",
            MAX_IMPORT_BYTES / 1024 / 1024
        ));
    }
    fs::read_to_string(path).map_err(|e| format!("Read {}: {}", path.display(), e))
}

fn validate_manifest_path(path: &str) -> Result<PathBuf, String> {
    let relative = PathBuf::from(path);
    if relative.is_absolute() {
        return Err(format!("Workspace request path must be relative: {}", path));
    }
    if relative.extension() != Some(OsStr::new("json")) {
        return Err(format!(
            "Workspace request path must be a JSON file: {}",
            path
        ));
    }

    let mut components = relative.components();
    match components.next() {
        Some(Component::Normal(part)) if part == OsStr::new(REQUESTS_DIR) => {}
        _ => {
            return Err(format!(
                "Workspace request path must live under {}: {}",
                REQUESTS_DIR, path
            ));
        }
    }

    for component in components {
        match component {
            Component::Normal(_) => {}
            _ => return Err(format!("Workspace request path escapes root: {}", path)),
        }
    }

    Ok(relative)
}

fn export_request(request: &Request, secret_policy: SecretPolicy) -> Request {
    match secret_policy {
        SecretPolicy::Include => request.clone(),
        SecretPolicy::Mask => mask_request(request),
    }
}

fn mask_request(request: &Request) -> Request {
    let mut request = request.clone();
    request.url = redact_url_query_and_fragment(&request.url);
    request.query_params = mask_rows(&request.query_params);
    request.path_params = mask_rows(&request.path_params);
    request.headers = mask_rows(&request.headers);
    request.cookies = mask_rows(&request.cookies);
    request.body_ext = request.body_ext.map(mask_body_ext);
    request.auth = mask_auth(&request.auth);
    request
}

fn mask_body_ext(body_ext: BodyExt) -> BodyExt {
    match body_ext {
        BodyExt::FormUrlEncoded { fields } => BodyExt::FormUrlEncoded {
            fields: mask_rows(&fields),
        },
        BodyExt::MultipartForm { fields } => BodyExt::MultipartForm {
            fields: mask_rows(&fields),
        },
        BodyExt::GraphQL { variables } => BodyExt::GraphQL { variables },
    }
}

fn mask_rows(rows: &[KvRow]) -> Vec<KvRow> {
    rows.iter()
        .map(|row| {
            let mut row = row.clone();
            if is_sensitive_key(&row.key) {
                row.value = mask_secret_value(&row.value);
            }
            row
        })
        .collect()
}

fn mask_auth(auth: &Auth) -> Auth {
    match auth {
        Auth::None => Auth::None,
        Auth::Bearer { token } => Auth::Bearer {
            token: mask_secret_value(token),
        },
        Auth::Basic { username, password } => Auth::Basic {
            username: username.clone(),
            password: mask_secret_value(password),
        },
        Auth::OAuth2(state) => Auth::OAuth2(Box::new(mask_oauth2_state(state))),
    }
}

fn mask_oauth2_state(state: &OAuth2State) -> OAuth2State {
    let mut state = state.clone();
    state.config.client_secret = mask_secret_value(&state.config.client_secret);
    state.access_token = mask_secret_value(&state.access_token);
    state.refresh_token = mask_secret_value(&state.refresh_token);
    state
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    serde_json::to_writer_pretty(&mut file, value)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    file.write_all(b"\n")?;
    file.sync_all()
}

fn path_to_manifest_string(path: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            _ => {
                return Err(format!(
                    "Generated workspace path is not relative: {}",
                    path.display()
                ));
            }
        }
    }
    Ok(parts.join("/"))
}

fn path_component(index: usize, name: &str, id: &str, fallback: &str) -> String {
    format!(
        "{:03}-{}-{}",
        index + 1,
        slug(name, fallback),
        slug(id, "id")
    )
}

fn slug(value: &str, fallback: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for c in value.chars().flat_map(|c| c.to_lowercase()) {
        let keep = c.is_ascii_alphanumeric() || c == '_' || c == '-';
        if keep {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
        if out.len() >= 48 {
            break;
        }
    }

    let out = out.trim_matches('-');
    if out.is_empty() {
        fallback.to_string()
    } else {
        out.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HttpMethod, OAuth2Config};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_folders() -> Vec<Folder> {
        vec![Folder {
            id: "collection-1".into(),
            name: "Fixture API".into(),
            description: "top level".into(),
            requests: vec![Request {
                id: "request-1".into(),
                name: "Create widget".into(),
                description: "creates a widget".into(),
                method: HttpMethod::POST,
                url: "https://api.example.com/widgets?debug=true#token=secret".into(),
                query_params: vec![
                    KvRow::new("include", "details"),
                    KvRow::new("api_token", "query-secret"),
                ],
                path_params: vec![KvRow::new("account_id", "acct_123")],
                headers: vec![
                    KvRow::new("Content-Type", "application/json"),
                    KvRow::new("Authorization", "Bearer header-secret"),
                ],
                cookies: vec![KvRow::new("session_id", "cookie-secret")],
                body: "{\"name\":\"demo\"}".into(),
                body_ext: Some(BodyExt::FormUrlEncoded {
                    fields: vec![
                        KvRow::new("client_id", "client-1"),
                        KvRow::new("client_secret", "form-secret"),
                    ],
                }),
                auth: Auth::Bearer {
                    token: "bearer-secret".into(),
                },
                extractors: vec![],
                assertions: vec![],
                source: None,
            }],
            subfolders: vec![Folder {
                id: "collection-1-sub".into(),
                name: "Nested".into(),
                description: String::new(),
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
                    auth: Auth::OAuth2(Box::new(OAuth2State {
                        config: OAuth2Config {
                            auth_url: "https://auth.example.com/authorize".into(),
                            token_url: "https://auth.example.com/token".into(),
                            client_id: "client-1".into(),
                            client_secret: "oauth-client-secret".into(),
                            scope: "read".into(),
                            redirect_uri: "http://127.0.0.1/callback".into(),
                        },
                        access_token: "oauth-access-token".into(),
                        refresh_token: "oauth-refresh-token".into(),
                        expires_at: Some(42),
                    })),
                    extractors: vec![],
                    assertions: vec![],
                    source: None,
                }],
                subfolders: vec![],
            }],
        }]
    }

    fn temp_workspace(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("rusty-requester-git-workspace-{name}-{nonce}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn read(path: impl AsRef<Path>) -> String {
        fs::read_to_string(path).unwrap()
    }

    fn json_value<T: Serialize>(value: &T) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    #[test]
    fn export_layout_is_deterministic_and_manifest_ordered() {
        let root = temp_workspace("layout");
        let folders = fixture_folders();

        let summary = export_workspace_to_dir(&folders, &root, ExportOptions::default()).unwrap();

        assert_eq!(summary.request_files, 2);
        assert_eq!(summary.manifest_path, root.join(MANIFEST_FILE));
        assert!(root.join(MANIFEST_FILE).is_file());
        assert!(root
            .join("requests/001-fixture-api-collection-1/001-create-widget-request-1.json")
            .is_file());
        assert!(root
            .join(
                "requests/001-fixture-api-collection-1/001-nested-collection-1-sub/001-status-request-2.json"
            )
            .is_file());

        let manifest = read(root.join(MANIFEST_FILE));
        assert!(manifest.contains("\"format\": \"rusty-requester-git-workspace\""));
        assert!(manifest.contains(
            "\"path\": \"requests/001-fixture-api-collection-1/001-create-widget-request-1.json\""
        ));

        let first_manifest = manifest.clone();
        export_workspace_to_dir(&folders, &root, ExportOptions::default()).unwrap();
        assert_eq!(read(root.join(MANIFEST_FILE)), first_manifest);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn default_export_masks_sensitive_values() {
        let root = temp_workspace("mask");
        export_workspace_to_dir(&fixture_folders(), &root, ExportOptions::default()).unwrap();

        let first_request = read(
            root.join("requests/001-fixture-api-collection-1/001-create-widget-request-1.json"),
        );
        assert!(!first_request.contains("query-secret"));
        assert!(!first_request.contains("header-secret"));
        assert!(!first_request.contains("cookie-secret"));
        assert!(!first_request.contains("form-secret"));
        assert!(!first_request.contains("bearer-secret"));
        assert!(first_request.contains("query-...cret"));
        assert!(first_request.contains("https://api.example.com/widgets?...#..."));

        let second_request = read(root.join(
            "requests/001-fixture-api-collection-1/001-nested-collection-1-sub/001-status-request-2.json",
        ));
        assert!(!second_request.contains("oauth-client-secret"));
        assert!(!second_request.contains("oauth-access-token"));
        assert!(!second_request.contains("oauth-refresh-token"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn include_secrets_round_trips_without_data_loss_and_preserves_ids() {
        let root = temp_workspace("round-trip");
        let folders = fixture_folders();

        export_workspace_to_dir(
            &folders,
            &root,
            ExportOptions {
                secret_policy: SecretPolicy::Include,
            },
        )
        .unwrap();
        let imported = import_workspace_from_dir(&root).unwrap();

        assert_eq!(json_value(&imported), json_value(&folders));
        assert_eq!(imported[0].id, "collection-1");
        assert_eq!(imported[0].requests[0].id, "request-1");
        assert_eq!(imported[0].subfolders[0].requests[0].id, "request-2");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn import_rejects_request_id_mismatch() {
        let root = temp_workspace("id-mismatch");
        export_workspace_to_dir(
            &fixture_folders(),
            &root,
            ExportOptions {
                secret_policy: SecretPolicy::Include,
            },
        )
        .unwrap();

        let path =
            root.join("requests/001-fixture-api-collection-1/001-create-widget-request-1.json");
        let mut request: Request = serde_json::from_str(&read(&path)).unwrap();
        request.id = "different".into();
        write_json_file(&path, &request).unwrap();

        let err = import_workspace_from_dir(&root).unwrap_err();
        assert!(err.contains("Request ID mismatch"), "{err}");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn import_rejects_paths_outside_requests_directory() {
        let root = temp_workspace("bad-path");
        export_workspace_to_dir(&fixture_folders(), &root, ExportOptions::default()).unwrap();

        let manifest_path = root.join(MANIFEST_FILE);
        let mut manifest: serde_json::Value = serde_json::from_str(&read(&manifest_path)).unwrap();
        manifest["folders"][0]["requests"][0]["path"] = serde_json::json!("../outside.json");
        write_json_file(&manifest_path, &manifest).unwrap();

        let err = import_workspace_from_dir(&root).unwrap_err();
        assert!(err.contains("must live under requests"), "{err}");
        let _ = fs::remove_dir_all(root);
    }
}
