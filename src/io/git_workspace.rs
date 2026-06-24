use crate::model::{Auth, BodyExt, Environment, Folder, HttpMethod, KvRow, OAuth2State, Request};
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
const ENVIRONMENTS_DIR: &str = "environments";
const REQUEST_FILE_EXTENSION: &str = "rr";
const ENV_FILE_EXTENSION: &str = "rrenv";
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportOptions {
    pub secret_policy: SecretPolicy,
    pub mask_rules: MaskRules,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            secret_policy: SecretPolicy::Mask,
            mask_rules: MaskRules::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MaskRules {
    pub mask_patterns: Vec<String>,
    pub allow_patterns: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportSummary {
    pub manifest_path: PathBuf,
    pub request_files: usize,
    pub environment_files: usize,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    secrets: ManifestSecretPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    environments: Vec<ManifestEnvironment>,
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

#[derive(Serialize, Deserialize)]
struct ManifestEnvironment {
    id: String,
    path: String,
}

#[allow(dead_code)]
pub fn export_workspace_to_dir(
    folders: &[Folder],
    root: &Path,
    options: ExportOptions,
) -> Result<ExportSummary, String> {
    export_workspace_with_environments_to_dir(folders, &[], root, options)
}

pub fn export_workspace_with_environments_to_dir(
    folders: &[Folder],
    environments: &[Environment],
    root: &Path,
    options: ExportOptions,
) -> Result<ExportSummary, String> {
    fs::create_dir_all(root).map_err(|e| format!("Create export directory: {}", e))?;

    let requests_root = root.join(REQUESTS_DIR);
    replace_managed_dir(&requests_root, "requests")?;
    fs::create_dir_all(&requests_root).map_err(|e| format!("Create request directory: {}", e))?;

    let environments_root = root.join(ENVIRONMENTS_DIR);
    replace_managed_dir(&environments_root, "environments")?;
    fs::create_dir_all(&environments_root)
        .map_err(|e| format!("Create environments directory: {}", e))?;
    write_workspace_gitignore(root)?;

    let mut request_files = Vec::new();
    let folders = manifest_folders(folders, options.clone(), &mut request_files)?;
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
        write_text_file(&full_path, &request_to_rr(&request_file.request)?)
            .map_err(|e| format!("Write {}: {}", full_path.display(), e))?;
    }

    let mut environment_files = manifest_environments(environments, options.clone());
    environment_files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    let mut seen_env_paths = BTreeSet::new();
    for environment_file in &environment_files {
        if !seen_env_paths.insert(environment_file.relative_path.clone()) {
            return Err(format!(
                "Duplicate environment export path generated: {}",
                environment_file.relative_path.display()
            ));
        }
        let full_path = root.join(&environment_file.relative_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Create environment directory: {}", e))?;
        }
        write_text_file(
            &full_path,
            &environment_to_rrenv(&environment_file.environment)?,
        )
        .map_err(|e| format!("Write {}: {}", full_path.display(), e))?;
    }

    let manifest = Manifest {
        format: FORMAT_NAME.to_string(),
        version: FORMAT_VERSION,
        secrets: match options.secret_policy {
            SecretPolicy::Mask => ManifestSecretPolicy::Masked,
            SecretPolicy::Include => ManifestSecretPolicy::Included,
        },
        environments: environment_files
            .iter()
            .map(|env| {
                Ok(ManifestEnvironment {
                    id: env.environment.id.clone(),
                    path: path_to_manifest_string(&env.relative_path)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        folders,
    };
    let manifest_path = root.join(MANIFEST_FILE);
    write_json_file(&manifest_path, &manifest)
        .map_err(|e| format!("Write {}: {}", manifest_path.display(), e))?;

    Ok(ExportSummary {
        manifest_path,
        request_files: request_files.len(),
        environment_files: environment_files.len(),
    })
}

#[allow(dead_code)]
pub fn import_workspace_from_dir(root: &Path) -> Result<Vec<Folder>, String> {
    import_workspace_bundle_from_dir(root).map(|bundle| bundle.folders)
}

#[derive(Clone, Debug)]
pub struct WorkspaceBundle {
    pub folders: Vec<Folder>,
    pub environments: Vec<Environment>,
}

pub fn import_workspace_bundle_from_dir(root: &Path) -> Result<WorkspaceBundle, String> {
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

    Ok(WorkspaceBundle {
        folders: import_manifest_folders(root, &manifest.folders, &mut budget, 0)?,
        environments: import_manifest_environments(root, &manifest.environments, &mut budget)?,
    })
}

struct RequestFile {
    relative_path: PathBuf,
    request: Request,
}

struct EnvironmentFile {
    relative_path: PathBuf,
    environment: Environment,
}

fn replace_managed_dir(path: &Path, label: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let meta =
        fs::symlink_metadata(path).map_err(|e| format!("Inspect {} directory: {}", label, e))?;
    if meta.file_type().is_symlink() {
        return Err(format!("Refusing to replace symlinked {} directory", label));
    }
    if !meta.is_dir() {
        return Err(format!("Refusing to replace non-directory {} path", label));
    }
    fs::remove_dir_all(path).map_err(|e| format!("Clean {} files: {}", label, e))
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
            options.clone(),
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
            "{}.{}",
            path_component(request_index, &request.name, &request.id, "request"),
            REQUEST_FILE_EXTENSION
        ));

        requests.push(ManifestRequest {
            id: request.id.clone(),
            path: path_to_manifest_string(&request_path)?,
        });
        request_files.push(RequestFile {
            relative_path: request_path,
            request: export_request(request, &options),
        });
    }

    let mut subfolders = Vec::with_capacity(folder.subfolders.len());
    for (sub_index, subfolder) in folder.subfolders.iter().enumerate() {
        subfolders.push(manifest_folder(
            subfolder,
            options.clone(),
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
            let path = root.join(&relative);
            let content = read_bounded_file(&path, budget)?;
            let request = if relative.extension() == Some(OsStr::new("json")) {
                serde_json::from_str(&content)
                    .map_err(|e| format!("Parse {}: {}", path.display(), e))?
            } else {
                rr_to_request(&content).map_err(|e| format!("Parse {}: {}", path.display(), e))?
            };
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
            sync: crate::model::SyncConfig::default(),
        });
    }
    Ok(out)
}

fn manifest_environments(
    environments: &[Environment],
    options: ExportOptions,
) -> Vec<EnvironmentFile> {
    environments
        .iter()
        .enumerate()
        .map(|(index, environment)| {
            let mut path = PathBuf::from(ENVIRONMENTS_DIR);
            path.push(format!(
                "{}.{}",
                path_component(index, &environment.name, &environment.id, "environment"),
                ENV_FILE_EXTENSION
            ));
            EnvironmentFile {
                relative_path: path,
                environment: export_environment(environment, &options),
            }
        })
        .collect()
}

fn import_manifest_environments(
    root: &Path,
    environments: &[ManifestEnvironment],
    budget: &mut ImportBudget,
) -> Result<Vec<Environment>, String> {
    let mut out = Vec::with_capacity(environments.len());
    for environment_ref in environments {
        let relative = validate_environment_path(&environment_ref.path)?;
        let path = root.join(relative);
        let content = read_bounded_file(&path, budget)?;
        let environment = rrenv_to_environment(&content)
            .map_err(|e| format!("Parse {}: {}", path.display(), e))?;
        if environment.id != environment_ref.id {
            return Err(format!(
                "Environment ID mismatch for {}: manifest has {}, file has {}",
                environment_ref.path, environment_ref.id, environment.id
            ));
        }
        out.push(environment);
    }
    Ok(out)
}

#[derive(Default)]
struct ImportBudget {
    total_bytes: u64,
    request_files: usize,
}

fn validate_environment_path(path: &str) -> Result<PathBuf, String> {
    let relative = PathBuf::from(path);
    if relative.is_absolute() {
        return Err(format!(
            "Workspace environment path must be relative: {}",
            path
        ));
    }
    if relative.extension() != Some(OsStr::new(ENV_FILE_EXTENSION)) {
        return Err(format!(
            "Workspace environment path must be a .{} file: {}",
            ENV_FILE_EXTENSION, path
        ));
    }

    let mut components = relative.components();
    match components.next() {
        Some(Component::Normal(part)) if part == OsStr::new(ENVIRONMENTS_DIR) => {}
        _ => {
            return Err(format!(
                "Workspace environment path must live under {}: {}",
                ENVIRONMENTS_DIR, path
            ));
        }
    }

    for component in components {
        match component {
            Component::Normal(_) => {}
            _ => return Err(format!("Workspace environment path escapes root: {}", path)),
        }
    }

    Ok(relative)
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
    let ext = relative.extension();
    if ext != Some(OsStr::new(REQUEST_FILE_EXTENSION)) && ext != Some(OsStr::new("json")) {
        return Err(format!(
            "Workspace request path must be a .rr or .json file: {}",
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

fn export_request(request: &Request, options: &ExportOptions) -> Request {
    match options.secret_policy {
        SecretPolicy::Include => request.clone(),
        SecretPolicy::Mask => mask_request(request, &options.mask_rules),
    }
}

fn export_environment(environment: &Environment, options: &ExportOptions) -> Environment {
    match options.secret_policy {
        SecretPolicy::Include => environment.clone(),
        SecretPolicy::Mask => {
            let mut environment = environment.clone();
            environment.variables = mask_rows(&environment.variables, &options.mask_rules);
            environment.cookies = environment
                .cookies
                .into_iter()
                .map(|mut cookie| {
                    cookie.value = mask_secret_value(&cookie.value);
                    cookie
                })
                .collect();
            environment
        }
    }
}

fn request_to_rr(request: &Request) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("rr 1\n");
    push_dict_block(
        &mut out,
        "meta",
        &[
            ("id", request.id.as_str()),
            ("name", request.name.as_str()),
            ("description", request.description.as_str()),
        ],
    );
    push_dict_block(
        &mut out,
        &request.method.to_string().to_ascii_lowercase(),
        &[("url", request.url.as_str())],
    );
    push_kv_block(&mut out, "params:query", &request.query_params);
    push_kv_block(&mut out, "params:path", &request.path_params);
    push_kv_block(&mut out, "headers", &request.headers);
    push_kv_block(&mut out, "cookies", &request.cookies);
    push_body_blocks(&mut out, request)?;
    push_auth_blocks(&mut out, &request.auth)?;
    push_rr_json_block(&mut out, "extractors:json", &request.extractors)?;
    push_rr_json_block(&mut out, "assertions:json", &request.assertions)?;
    push_rr_json_block(&mut out, "source:json", &request.source)?;
    Ok(out)
}

fn rr_to_request(content: &str) -> Result<Request, String> {
    if is_legacy_rr(content) {
        return legacy_rr_to_request(content);
    }
    block_rr_to_request(content)
}

fn is_legacy_rr(content: &str) -> bool {
    content
        .lines()
        .skip(1)
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .is_some_and(|line| line.starts_with('[') || line.starts_with("id:"))
}

fn legacy_rr_to_request(content: &str) -> Result<Request, String> {
    let mut doc = RrDocument::parse(content, "rr 1")?;
    let id = doc.take_required_field("id")?;
    let name = doc.take_required_field("name")?;
    let description = doc.take_field("description")?.unwrap_or_default();
    let method = doc.take_required_field("method")?;
    let method = serde_json::from_value(serde_json::Value::String(method))
        .map_err(|e| format!("Invalid method: {}", e))?;
    let url = doc.take_required_field("url")?;
    let query_params = doc.take_rows("query")?;
    let path_params = doc.take_rows("path")?;
    let headers = doc.take_rows("headers")?;
    let cookies = doc.take_rows("cookies")?;
    let body = doc.take_text_block("body")?.unwrap_or_default();
    let body_ext = doc
        .take_json_block("body_ext")?
        .map(|value| serde_json::from_value(value).map_err(|e| format!("Invalid body_ext: {}", e)))
        .transpose()?
        .unwrap_or(None);
    let auth = doc
        .take_json_block("auth")?
        .map(|value| serde_json::from_value(value).map_err(|e| format!("Invalid auth: {}", e)))
        .transpose()?
        .unwrap_or_default();
    let extractors = doc
        .take_json_block("extractors")?
        .map(|value| {
            serde_json::from_value(value).map_err(|e| format!("Invalid extractors: {}", e))
        })
        .transpose()?
        .unwrap_or_default();
    let assertions = doc
        .take_json_block("assertions")?
        .map(|value| {
            serde_json::from_value(value).map_err(|e| format!("Invalid assertions: {}", e))
        })
        .transpose()?
        .unwrap_or_default();
    let source = doc
        .take_json_block("source")?
        .map(|value| serde_json::from_value(value).map_err(|e| format!("Invalid source: {}", e)))
        .transpose()?
        .unwrap_or(None);
    doc.reject_unknown()?;

    Ok(Request {
        id,
        name,
        description,
        method,
        url,
        query_params,
        path_params,
        headers,
        cookies,
        body,
        body_ext,
        auth,
        extractors,
        assertions,
        source,
    })
}

fn block_rr_to_request(content: &str) -> Result<Request, String> {
    let mut doc = BlockRrDocument::parse(content, "rr 1")?;
    let meta = doc.take_required_dict("meta")?;
    let id = take_required_dict_value(&meta, "id")?;
    let name = take_required_dict_value(&meta, "name")?;
    let description = lookup_dict(&meta, "description").unwrap_or_default();

    let (method, url) = doc.take_method_and_url()?;
    let query_params = doc.take_kv_rows("params:query")?;
    let path_params = doc.take_kv_rows("params:path")?;
    let headers = doc.take_kv_rows("headers")?;
    let cookies = doc.take_kv_rows("cookies")?;
    let (body, body_ext) = doc.take_body()?;
    let auth = doc.take_auth()?;
    let extractors = doc
        .take_json_block("extractors:json")?
        .map(|value| {
            serde_json::from_value(value).map_err(|e| format!("Invalid extractors: {}", e))
        })
        .transpose()?
        .unwrap_or_default();
    let assertions = doc
        .take_json_block("assertions:json")?
        .map(|value| {
            serde_json::from_value(value).map_err(|e| format!("Invalid assertions: {}", e))
        })
        .transpose()?
        .unwrap_or_default();
    let source = doc
        .take_json_block("source:json")?
        .map(|value| serde_json::from_value(value).map_err(|e| format!("Invalid source: {}", e)))
        .transpose()?
        .unwrap_or(None);
    doc.reject_unknown()?;

    Ok(Request {
        id,
        name,
        description,
        method,
        url,
        query_params,
        path_params,
        headers,
        cookies,
        body,
        body_ext,
        auth,
        extractors,
        assertions,
        source,
    })
}

fn push_dict_block(out: &mut String, name: &str, entries: &[(&str, &str)]) {
    let entries: Vec<(&str, &str)> = entries
        .iter()
        .copied()
        .filter(|(_, value)| !value.is_empty())
        .collect();
    if entries.is_empty() {
        return;
    }
    out.push('\n');
    out.push_str(name);
    out.push_str(" {\n");
    for (key, value) in entries {
        out.push_str("  ");
        out.push_str(key);
        out.push_str(": ");
        out.push_str(&plain_value(value));
        out.push('\n');
    }
    out.push_str("}\n");
}

fn push_kv_block(out: &mut String, name: &str, rows: &[KvRow]) {
    if rows.is_empty() {
        return;
    }
    out.push('\n');
    out.push_str(name);
    out.push_str(" {\n");
    for row in rows {
        out.push_str("  ");
        if !row.enabled {
            out.push('~');
        }
        out.push_str(&plain_key(&row.key));
        out.push_str(": ");
        out.push_str(&plain_value(&row.value));
        out.push('\n');
    }
    out.push_str("}\n");

    let descriptions: Vec<(&str, &str)> = rows
        .iter()
        .filter(|row| !row.description.is_empty())
        .map(|row| (row.key.as_str(), row.description.as_str()))
        .collect();
    if !descriptions.is_empty() {
        push_dict_block(out, &format!("{name}:docs"), &descriptions);
    }
}

fn push_body_blocks(out: &mut String, request: &Request) -> Result<(), String> {
    match &request.body_ext {
        Some(BodyExt::FormUrlEncoded { fields }) => {
            if !request.body.is_empty() {
                push_rr_text_block(out, "body:raw", &request.body);
            }
            push_kv_block(out, "body:form", fields);
        }
        Some(BodyExt::MultipartForm { fields }) => {
            if !request.body.is_empty() {
                push_rr_text_block(out, "body:raw", &request.body);
            }
            push_kv_block(out, "body:multipart", fields);
        }
        Some(BodyExt::GraphQL { variables }) => {
            if !request.body.is_empty() {
                push_rr_text_block(out, "body:graphql", &request.body);
            }
            if !variables.is_empty() {
                push_rr_text_block(out, "body:graphql:variables", variables);
            }
        }
        None => {
            if !request.body.is_empty() {
                push_rr_text_block(out, "body:raw", &request.body);
            }
        }
    }
    Ok(())
}

fn push_auth_blocks(out: &mut String, auth: &Auth) -> Result<(), String> {
    match auth {
        Auth::None => {}
        Auth::Bearer { token } => push_dict_block(out, "auth:bearer", &[("token", token)]),
        Auth::Basic { username, password } => push_dict_block(
            out,
            "auth:basic",
            &[("username", username), ("password", password)],
        ),
        Auth::OAuth2(_) => push_rr_json_block(out, "auth:oauth2", auth)?,
    }
    Ok(())
}

fn push_rr_text_block(out: &mut String, name: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    let delimiter = block_delimiter(value);
    out.push('\n');
    out.push_str(name);
    out.push_str(" <<");
    out.push_str(&delimiter);
    out.push('\n');
    out.push_str(value);
    if !value.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&delimiter);
    out.push('\n');
}

fn push_rr_json_block<T: Serialize>(out: &mut String, name: &str, value: &T) -> Result<(), String> {
    let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    if value.is_null()
        || value == serde_json::json!([])
        || value == serde_json::json!({})
        || value == serde_json::json!({"type": "None"})
    {
        return Ok(());
    }
    let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    push_rr_text_block(out, name, &text);
    Ok(())
}

fn plain_key(value: &str) -> String {
    if value.trim() == value
        && !value.is_empty()
        && !value.contains(':')
        && !value.chars().any(|c| c == '\n' || c == '\r')
    {
        value.to_string()
    } else {
        json_string(value).unwrap_or_else(|_| "\"\"".to_string())
    }
}

fn plain_value(value: &str) -> String {
    if value.trim() == value && !value.chars().any(|c| c == '\n' || c == '\r') {
        value.to_string()
    } else {
        json_string(value).unwrap_or_else(|_| "\"\"".to_string())
    }
}

fn parse_plain_value(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.starts_with('"') {
        serde_json::from_str(value).map_err(|e| format!("Invalid quoted value: {}", e))
    } else {
        Ok(value.to_string())
    }
}

fn take_required_dict_value(dict: &[(String, String)], key: &str) -> Result<String, String> {
    dict.iter()
        .find(|(field, _)| field == key)
        .map(|(_, value)| value.clone())
        .ok_or_else(|| format!("Missing required field: {key}"))
}

fn environment_to_rrenv(environment: &Environment) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("rrenv 1\n");
    push_field(&mut out, "id", &environment.id)?;
    push_field(&mut out, "name", &environment.name)?;
    push_rows(&mut out, "variables", &environment.variables)?;
    push_json_block(&mut out, "cookies", &environment.cookies)?;
    Ok(out)
}

fn rrenv_to_environment(content: &str) -> Result<Environment, String> {
    let mut doc = RrDocument::parse(content, "rrenv 1")?;
    let id = doc.take_required_field("id")?;
    let name = doc.take_required_field("name")?;
    let variables = doc.take_rows("variables")?;
    let cookies = doc
        .take_json_block("cookies")?
        .map(|value| serde_json::from_value(value).map_err(|e| format!("Invalid cookies: {}", e)))
        .transpose()?
        .unwrap_or_default();
    doc.reject_unknown()?;

    Ok(Environment {
        id,
        name,
        variables,
        cookies,
    })
}

fn push_field(out: &mut String, key: &str, value: &str) -> Result<(), String> {
    out.push_str(key);
    out.push_str(": ");
    out.push_str(&json_string(value)?);
    out.push('\n');
    Ok(())
}

fn push_rows(out: &mut String, section: &str, rows: &[KvRow]) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    out.push('\n');
    out.push('[');
    out.push_str(section);
    out.push_str("]\n");
    for row in rows {
        out.push(if row.enabled { '+' } else { '~' });
        out.push(' ');
        out.push_str(&json_string(&row.key)?);
        out.push('\t');
        out.push_str(&json_string(&row.value)?);
        out.push('\t');
        out.push_str(&json_string(&row.description)?);
        out.push('\n');
    }
    Ok(())
}

fn push_json_block<T: Serialize>(out: &mut String, section: &str, value: &T) -> Result<(), String> {
    let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    if value.is_null()
        || value == serde_json::json!([])
        || value == serde_json::json!({})
        || value == serde_json::json!({"type": "None"})
    {
        return Ok(());
    }
    let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    push_block(out, section, "json", &text)
}

fn push_block(out: &mut String, section: &str, kind: &str, value: &str) -> Result<(), String> {
    let delimiter = block_delimiter(value);
    out.push('\n');
    out.push('[');
    out.push_str(section);
    out.push_str("]\n");
    out.push_str(kind);
    out.push_str(" <<");
    out.push_str(&delimiter);
    out.push('\n');
    out.push_str(value);
    if !value.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&delimiter);
    out.push('\n');
    Ok(())
}

fn block_delimiter(value: &str) -> String {
    let mut suffix = 0;
    loop {
        let candidate = if suffix == 0 {
            "RR_BLOCK".to_string()
        } else {
            format!("RR_BLOCK_{}", suffix)
        };
        if !value.lines().any(|line| line == candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn json_string(value: &str) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

#[derive(Default)]
struct BlockRrDocument {
    dicts: std::collections::BTreeMap<String, Vec<(String, String)>>,
    blocks: std::collections::BTreeMap<String, String>,
}

impl BlockRrDocument {
    fn parse(content: &str, header: &str) -> Result<Self, String> {
        let lines: Vec<&str> = content.lines().collect();
        if lines.first().map(|line| line.trim()) != Some(header) {
            return Err(format!("Expected {} header", header));
        }

        let mut doc = BlockRrDocument::default();
        let mut index = 1;
        while index < lines.len() {
            let trimmed = lines[index].trim();
            index += 1;
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some((name, marker)) = trimmed.split_once(" <<") {
                let name = name.trim();
                let marker = marker.trim();
                if name.is_empty() || marker.is_empty() {
                    return Err(format!("Invalid text block opener: {trimmed}"));
                }
                let mut value = String::new();
                let mut closed = false;
                while index < lines.len() {
                    let line = lines[index];
                    index += 1;
                    if line == marker {
                        closed = true;
                        break;
                    }
                    value.push_str(line);
                    value.push('\n');
                }
                if !closed {
                    return Err(format!("Unclosed block: {name}"));
                }
                if value.ends_with('\n') {
                    value.pop();
                }
                doc.blocks.insert(name.to_string(), value);
                continue;
            }

            let Some(name) = trimmed.strip_suffix('{').map(str::trim) else {
                return Err(format!("Expected block opener, got: {trimmed}"));
            };
            if name.is_empty() {
                return Err("Missing block name".to_string());
            }
            let mut dict = Vec::new();
            let mut closed = false;
            while index < lines.len() {
                let line = lines[index];
                index += 1;
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if trimmed == "}" {
                    closed = true;
                    break;
                }
                let (key, value) = trimmed
                    .split_once(':')
                    .ok_or_else(|| format!("Expected key/value in {name}, got: {trimmed}"))?;
                let (enabled, key) = match key.trim().strip_prefix('~') {
                    Some(disabled_key) => (false, disabled_key.trim()),
                    None => (true, key.trim()),
                };
                let mut key = parse_plain_value(key)?;
                if !enabled {
                    key.insert(0, '~');
                }
                dict.push((key, parse_plain_value(value)?));
            }
            if !closed {
                return Err(format!("Unclosed dictionary block: {name}"));
            }
            doc.dicts.insert(name.to_string(), dict);
        }

        Ok(doc)
    }

    fn take_required_dict(&mut self, name: &str) -> Result<Vec<(String, String)>, String> {
        self.dicts
            .remove(name)
            .ok_or_else(|| format!("Missing required block: {name}"))
    }

    fn take_dict(&mut self, name: &str) -> Option<Vec<(String, String)>> {
        self.dicts.remove(name)
    }

    fn take_block(&mut self, name: &str) -> Option<String> {
        self.blocks.remove(name)
    }

    fn take_json_block(&mut self, name: &str) -> Result<Option<serde_json::Value>, String> {
        let Some(value) = self.take_block(name) else {
            return Ok(None);
        };
        serde_json::from_str(&value)
            .map(Some)
            .map_err(|e| format!("Invalid JSON block {name}: {e}"))
    }

    fn take_method_and_url(&mut self) -> Result<(HttpMethod, String), String> {
        for method in HttpMethod::ALL {
            let name = method.to_string().to_ascii_lowercase();
            if let Some(dict) = self.take_dict(&name) {
                let url = take_required_dict_value(&dict, "url")?;
                return Ok((method, url));
            }
        }
        Err("Missing request method block".to_string())
    }

    fn take_kv_rows(&mut self, name: &str) -> Result<Vec<KvRow>, String> {
        let Some(dict) = self.take_dict(name) else {
            return Ok(Vec::new());
        };
        let docs = self.take_dict(&format!("{name}:docs")).unwrap_or_default();
        let mut rows = Vec::with_capacity(dict.len());
        for (key, value) in dict {
            let (enabled, key) = match key.strip_prefix('~') {
                Some(key) => (false, key.to_string()),
                None => (true, key),
            };
            rows.push(KvRow {
                description: lookup_dict(&docs, &key).unwrap_or_default(),
                enabled,
                key,
                value,
            });
        }
        Ok(rows)
    }

    fn take_body(&mut self) -> Result<(String, Option<BodyExt>), String> {
        if let Some(fields) = self.take_dict("body:form") {
            return Ok((
                self.take_block("body:raw").unwrap_or_default(),
                Some(BodyExt::FormUrlEncoded {
                    fields: dict_to_rows(fields, self.take_dict("body:form:docs")),
                }),
            ));
        }
        if let Some(fields) = self.take_dict("body:multipart") {
            return Ok((
                self.take_block("body:raw").unwrap_or_default(),
                Some(BodyExt::MultipartForm {
                    fields: dict_to_rows(fields, self.take_dict("body:multipart:docs")),
                }),
            ));
        }
        if let Some(query) = self.take_block("body:graphql") {
            let variables = self
                .take_block("body:graphql:variables")
                .unwrap_or_default();
            return Ok((query, Some(BodyExt::GraphQL { variables })));
        }
        Ok((self.take_block("body:raw").unwrap_or_default(), None))
    }

    fn take_auth(&mut self) -> Result<Auth, String> {
        if let Some(dict) = self.take_dict("auth:bearer") {
            return Ok(Auth::Bearer {
                token: lookup_dict(&dict, "token").unwrap_or_default(),
            });
        }
        if let Some(dict) = self.take_dict("auth:basic") {
            return Ok(Auth::Basic {
                username: lookup_dict(&dict, "username").unwrap_or_default(),
                password: lookup_dict(&dict, "password").unwrap_or_default(),
            });
        }
        if let Some(value) = self.take_json_block("auth:oauth2")? {
            return serde_json::from_value(value).map_err(|e| format!("Invalid OAuth auth: {e}"));
        }
        Ok(Auth::None)
    }

    fn reject_unknown(self) -> Result<(), String> {
        if let Some(key) = self.dicts.keys().next() {
            return Err(format!("Unknown block: {key}"));
        }
        if let Some(key) = self.blocks.keys().next() {
            return Err(format!("Unknown text block: {key}"));
        }
        Ok(())
    }
}

fn dict_to_rows(dict: Vec<(String, String)>, docs: Option<Vec<(String, String)>>) -> Vec<KvRow> {
    let docs = docs.unwrap_or_default();
    dict.into_iter()
        .map(|(key, value)| {
            let (enabled, key) = match key.strip_prefix('~') {
                Some(key) => (false, key.to_string()),
                None => (true, key),
            };
            KvRow {
                description: lookup_dict(&docs, &key).unwrap_or_default(),
                enabled,
                key,
                value,
            }
        })
        .collect()
}

fn lookup_dict(dict: &[(String, String)], key: &str) -> Option<String> {
    dict.iter()
        .find(|(field, _)| field == key)
        .map(|(_, value)| value.clone())
}

#[derive(Default)]
struct RrDocument {
    fields: std::collections::BTreeMap<String, String>,
    sections: std::collections::BTreeMap<String, RrSection>,
}

#[derive(Default)]
struct RrSection {
    rows: Vec<KvRow>,
    block: Option<RrBlock>,
}

struct RrBlock {
    kind: String,
    value: String,
}

impl RrDocument {
    fn parse(content: &str, header: &str) -> Result<Self, String> {
        let lines: Vec<&str> = content.lines().collect();
        if lines.first().map(|line| line.trim()) != Some(header) {
            return Err(format!("Expected {} header", header));
        }

        let mut doc = RrDocument::default();
        let mut section: Option<String> = None;
        let mut index = 1;
        while index < lines.len() {
            let line = lines[index];
            let trimmed = line.trim();
            index += 1;
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(name) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                section = Some(name.to_string());
                doc.sections.entry(name.to_string()).or_default();
                continue;
            }

            if let Some(section_name) = &section {
                if let Some((kind, rest)) = trimmed.split_once(" <<") {
                    let marker = rest.trim();
                    if marker.is_empty() {
                        return Err("Missing block marker".to_string());
                    }
                    let mut value = String::new();
                    let mut closed = false;
                    while index < lines.len() {
                        let block_line = lines[index];
                        index += 1;
                        if block_line == marker {
                            closed = true;
                            break;
                        }
                        value.push_str(block_line);
                        value.push('\n');
                    }
                    if !closed {
                        return Err(format!("Unclosed block in [{}]", section_name));
                    }
                    if value.ends_with('\n') {
                        value.pop();
                    }
                    doc.sections.entry(section_name.clone()).or_default().block = Some(RrBlock {
                        kind: kind.trim().to_string(),
                        value,
                    });
                    continue;
                }
                let row = parse_rr_row(trimmed)?;
                doc.sections
                    .entry(section_name.clone())
                    .or_default()
                    .rows
                    .push(row);
                continue;
            }

            let (key, value) = trimmed
                .split_once(':')
                .ok_or_else(|| format!("Expected field, got: {}", trimmed))?;
            let value: String = serde_json::from_str(value.trim())
                .map_err(|e| format!("Invalid field {}: {}", key.trim(), e))?;
            doc.fields.insert(key.trim().to_string(), value);
        }
        Ok(doc)
    }

    fn take_required_field(&mut self, key: &str) -> Result<String, String> {
        self.take_field(key)?
            .ok_or_else(|| format!("Missing required field: {}", key))
    }

    fn take_field(&mut self, key: &str) -> Result<Option<String>, String> {
        Ok(self.fields.remove(key))
    }

    fn take_rows(&mut self, section: &str) -> Result<Vec<KvRow>, String> {
        Ok(self
            .sections
            .remove(section)
            .map(|section| section.rows)
            .unwrap_or_default())
    }

    fn take_text_block(&mut self, section: &str) -> Result<Option<String>, String> {
        let section_name = section;
        let Some(section) = self.sections.remove(section_name) else {
            return Ok(None);
        };
        let Some(block) = section.block else {
            return Ok(None);
        };
        if block.kind != "text" {
            return Err(format!("Expected text block, got {}", block.kind));
        }
        Ok(Some(block.value))
    }

    fn take_json_block(&mut self, section: &str) -> Result<Option<serde_json::Value>, String> {
        let section_name = section;
        let Some(section) = self.sections.remove(section_name) else {
            return Ok(None);
        };
        let Some(block) = section.block else {
            return Ok(None);
        };
        if block.kind != "json" {
            return Err(format!("Expected json block, got {}", block.kind));
        }
        serde_json::from_str(&block.value)
            .map(Some)
            .map_err(|e| format!("Invalid JSON block [{}]: {}", section_name, e))
    }

    fn reject_unknown(self) -> Result<(), String> {
        if let Some(key) = self.fields.keys().next() {
            return Err(format!("Unknown field: {}", key));
        }
        if let Some(key) = self.sections.keys().next() {
            return Err(format!("Unknown section: {}", key));
        }
        Ok(())
    }
}

fn parse_rr_row(line: &str) -> Result<KvRow, String> {
    let (enabled, rest) = match line.chars().next() {
        Some('+') => (true, &line[1..]),
        Some('~') => (false, &line[1..]),
        _ => return Err(format!("Expected row prefix + or ~, got: {}", line)),
    };
    let parts: Vec<&str> = rest.trim_start().splitn(3, '\t').collect();
    if parts.len() != 3 {
        return Err(format!("Expected tab-separated row: {}", line));
    }
    Ok(KvRow {
        enabled,
        key: serde_json::from_str(parts[0]).map_err(|e| format!("Invalid row key: {}", e))?,
        value: serde_json::from_str(parts[1]).map_err(|e| format!("Invalid row value: {}", e))?,
        description: serde_json::from_str(parts[2])
            .map_err(|e| format!("Invalid row description: {}", e))?,
    })
}

fn mask_request(request: &Request, rules: &MaskRules) -> Request {
    let mut request = request.clone();
    request.url = redact_url_query_and_fragment(&request.url);
    request.query_params = mask_rows(&request.query_params, rules);
    request.path_params = mask_rows(&request.path_params, rules);
    request.headers = mask_rows(&request.headers, rules);
    request.cookies = mask_rows(&request.cookies, rules);
    request.body_ext = request
        .body_ext
        .map(|body_ext| mask_body_ext(body_ext, rules));
    request.auth = mask_auth(&request.auth);
    request
}

fn mask_body_ext(body_ext: BodyExt, rules: &MaskRules) -> BodyExt {
    match body_ext {
        BodyExt::FormUrlEncoded { fields } => BodyExt::FormUrlEncoded {
            fields: mask_rows(&fields, rules),
        },
        BodyExt::MultipartForm { fields } => BodyExt::MultipartForm {
            fields: mask_rows(&fields, rules),
        },
        BodyExt::GraphQL { variables } => BodyExt::GraphQL { variables },
    }
}

fn mask_rows(rows: &[KvRow], rules: &MaskRules) -> Vec<KvRow> {
    rows.iter()
        .map(|row| {
            let mut row = row.clone();
            if should_mask_key(&row.key, rules) {
                row.value = mask_secret_value(&row.value);
            }
            row
        })
        .collect()
}

fn should_mask_key(key: &str, rules: &MaskRules) -> bool {
    let key = key.trim().to_ascii_lowercase();
    if key.is_empty() {
        return false;
    }
    if rules
        .allow_patterns
        .iter()
        .any(|pattern| key_matches_pattern(&key, pattern))
    {
        return false;
    }
    is_sensitive_key(&key)
        || rules
            .mask_patterns
            .iter()
            .any(|pattern| key_matches_pattern(&key, pattern))
}

fn key_matches_pattern(normalized_key: &str, pattern: &str) -> bool {
    let pattern = pattern.trim().to_ascii_lowercase();
    if pattern.is_empty() {
        return false;
    }
    normalized_key == pattern || normalized_key.contains(&pattern)
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

fn write_text_file(path: &Path, value: &str) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(value.as_bytes())?;
    if !value.ends_with('\n') {
        file.write_all(b"\n")?;
    }
    file.sync_all()
}

fn write_workspace_gitignore(root: &Path) -> Result<(), String> {
    let path = root.join(".gitignore");
    if path.exists() {
        let meta = fs::symlink_metadata(&path)
            .map_err(|e| format!("Inspect workspace .gitignore: {}", e))?;
        if meta.file_type().is_symlink() {
            return Err("Refusing to replace symlinked workspace .gitignore".to_string());
        }
        if !meta.is_file() {
            return Err("Refusing to replace non-file workspace .gitignore".to_string());
        }
    }
    write_text_file(
        &path,
        "# Rusty Requester local secret overlays\nsecrets/\n*.rrsecret\n",
    )
    .map_err(|e| format!("Write {}: {}", path.display(), e))
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
                    KvRow::new("platform", "android"),
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
                sync: crate::model::SyncConfig::default(),
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
            sync: crate::model::SyncConfig::default(),
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
            .join("requests/001-fixture-api-collection-1/001-create-widget-request-1.rr")
            .is_file());
        assert!(root
            .join(
                "requests/001-fixture-api-collection-1/001-nested-collection-1-sub/001-status-request-2.rr"
            )
            .is_file());

        let manifest = read(root.join(MANIFEST_FILE));
        assert!(manifest.contains("\"format\": \"rusty-requester-git-workspace\""));
        assert!(manifest.contains(
            "\"path\": \"requests/001-fixture-api-collection-1/001-create-widget-request-1.rr\""
        ));

        let first_manifest = manifest.clone();
        export_workspace_to_dir(&folders, &root, ExportOptions::default()).unwrap();
        assert_eq!(read(root.join(MANIFEST_FILE)), first_manifest);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn native_rr_uses_readable_block_format() {
        let request = Request {
            id: "request-readable".into(),
            name: "Health check".into(),
            description: "Simple smoke test".into(),
            method: HttpMethod::GET,
            url: "https://api.example.com/health".into(),
            query_params: vec![KvRow::new("platform", "android")],
            path_params: vec![],
            headers: vec![KvRow::new("Accept", "application/json")],
            cookies: vec![],
            body: String::new(),
            body_ext: None,
            auth: Auth::None,
            extractors: vec![],
            assertions: vec![],
            source: None,
        };

        let exported = request_to_rr(&request).unwrap();

        assert!(exported.contains("meta {\n"));
        assert!(exported.contains("  name: Health check\n"));
        assert!(exported.contains("get {\n  url: https://api.example.com/health\n}"));
        assert!(exported.contains("params:query {\n  platform: android\n}"));
        assert!(exported.contains("headers {\n  Accept: application/json\n}"));
        assert!(!exported.contains("[query]"));
        assert_eq!(rr_to_request(&exported).unwrap(), request);
    }

    #[test]
    fn native_rr_round_trips_query_method() {
        let request = Request {
            id: "request-query".into(),
            name: "Complex search".into(),
            description: String::new(),
            method: HttpMethod::QUERY,
            url: "https://api.example.com/search".into(),
            query_params: vec![],
            path_params: vec![],
            headers: vec![KvRow::new("Content-Type", "application/json")],
            cookies: vec![],
            body: "{\"filter\":\"active\"}".into(),
            body_ext: None,
            auth: Auth::None,
            extractors: vec![],
            assertions: vec![],
            source: None,
        };

        let exported = request_to_rr(&request).unwrap();

        assert!(exported.contains("query {\n  url: https://api.example.com/search\n}"));
        assert_eq!(rr_to_request(&exported).unwrap(), request);
    }

    #[test]
    fn default_export_masks_sensitive_values() {
        let root = temp_workspace("mask");
        export_workspace_to_dir(&fixture_folders(), &root, ExportOptions::default()).unwrap();

        let first_request =
            read(root.join("requests/001-fixture-api-collection-1/001-create-widget-request-1.rr"));
        assert!(!first_request.contains("query-secret"));
        assert!(!first_request.contains("header-secret"));
        assert!(!first_request.contains("cookie-secret"));
        assert!(!first_request.contains("form-secret"));
        assert!(!first_request.contains("bearer-secret"));
        assert!(first_request.contains("query-...cret"));
        assert!(first_request.contains("https://api.example.com/widgets?...#..."));

        let second_request = read(root.join(
            "requests/001-fixture-api-collection-1/001-nested-collection-1-sub/001-status-request-2.rr",
        ));
        assert!(!second_request.contains("oauth-client-secret"));
        assert!(!second_request.contains("oauth-access-token"));
        assert!(!second_request.contains("oauth-refresh-token"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mask_rules_can_force_or_allow_keys() {
        let root = temp_workspace("mask-rules");
        export_workspace_to_dir(
            &fixture_folders(),
            &root,
            ExportOptions {
                secret_policy: SecretPolicy::Mask,
                mask_rules: MaskRules {
                    mask_patterns: vec!["platform".to_string()],
                    allow_patterns: vec!["authorization".to_string()],
                },
            },
        )
        .unwrap();

        let first_request =
            read(root.join("requests/001-fixture-api-collection-1/001-create-widget-request-1.rr"));

        assert!(first_request.contains("Bearer header-secret"));
        assert!(!first_request.contains("platform: android"));
        assert!(first_request.contains("platform: *******"));
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
                ..ExportOptions::default()
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
    fn import_accepts_legacy_json_request_files() {
        let root = temp_workspace("legacy-json");
        export_workspace_to_dir(
            &fixture_folders(),
            &root,
            ExportOptions {
                secret_policy: SecretPolicy::Include,
                ..ExportOptions::default()
            },
        )
        .unwrap();

        let rr_path =
            root.join("requests/001-fixture-api-collection-1/001-create-widget-request-1.rr");
        let json_path =
            root.join("requests/001-fixture-api-collection-1/001-create-widget-request-1.json");
        let request = rr_to_request(&read(&rr_path)).unwrap();
        fs::rename(&rr_path, &json_path).unwrap();
        write_json_file(&json_path, &request).unwrap();

        let manifest_path = root.join(MANIFEST_FILE);
        let mut manifest: serde_json::Value = serde_json::from_str(&read(&manifest_path)).unwrap();
        manifest["folders"][0]["requests"][0]["path"] = serde_json::json!(
            "requests/001-fixture-api-collection-1/001-create-widget-request-1.json"
        );
        write_json_file(&manifest_path, &manifest).unwrap();

        let imported = import_workspace_from_dir(&root).unwrap();

        assert_eq!(imported[0].requests[0].id, "request-1");
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn export_refuses_symlinked_requests_directory() {
        use std::os::unix::fs::symlink;

        let root = temp_workspace("symlink-requests");
        let outside = temp_workspace("symlink-target");
        symlink(&outside, root.join(REQUESTS_DIR)).unwrap();

        let err = export_workspace_to_dir(&fixture_folders(), &root, ExportOptions::default())
            .unwrap_err();

        assert!(err.contains("symlinked requests directory"), "{err}");
        let _ = fs::remove_file(root.join(REQUESTS_DIR));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn import_rejects_request_id_mismatch() {
        let root = temp_workspace("id-mismatch");
        export_workspace_to_dir(
            &fixture_folders(),
            &root,
            ExportOptions {
                secret_policy: SecretPolicy::Include,
                ..ExportOptions::default()
            },
        )
        .unwrap();

        let path =
            root.join("requests/001-fixture-api-collection-1/001-create-widget-request-1.rr");
        let mut request = rr_to_request(&read(&path)).unwrap();
        request.id = "different".into();
        write_text_file(&path, &request_to_rr(&request).unwrap()).unwrap();

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
        manifest["folders"][0]["requests"][0]["path"] = serde_json::json!("../outside.rr");
        write_json_file(&manifest_path, &manifest).unwrap();

        let err = import_workspace_from_dir(&root).unwrap_err();
        assert!(err.contains("must live under requests"), "{err}");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn native_rr_round_trips_body_with_default_block_delimiter() {
        let mut folders = fixture_folders();
        folders[0].requests[0].body = "line one\nRR_BLOCK\nline two".to_string();
        let root = temp_workspace("delimiter");

        export_workspace_to_dir(
            &folders,
            &root,
            ExportOptions {
                secret_policy: SecretPolicy::Include,
                ..ExportOptions::default()
            },
        )
        .unwrap();
        let request_file =
            root.join("requests/001-fixture-api-collection-1/001-create-widget-request-1.rr");
        let exported = read(&request_file);

        assert!(exported.contains("body:raw <<RR_BLOCK_1"));
        assert_eq!(
            import_workspace_from_dir(&root).unwrap()[0].requests[0].body,
            "line one\nRR_BLOCK\nline two"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn native_rr_raw_json_array_does_not_trigger_legacy_parser() {
        let mut folders = fixture_folders();
        folders[0].requests[0].body_ext = None;
        folders[0].requests[0].body = "[\n  {\"ok\": true}\n]".to_string();
        let root = temp_workspace("raw-array");

        export_workspace_to_dir(
            &folders,
            &root,
            ExportOptions {
                secret_policy: SecretPolicy::Include,
                ..ExportOptions::default()
            },
        )
        .unwrap();

        assert_eq!(
            import_workspace_from_dir(&root).unwrap()[0].requests[0].body,
            "[\n  {\"ok\": true}\n]"
        );
        let _ = fs::remove_dir_all(root);
    }
}
