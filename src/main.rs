mod extract;
mod icon;
mod io;
mod model;
mod snippet;
mod theme;
mod widgets;

use io::curl;

use base64::Engine;
use eframe::egui;
use icon::{
    load_icon_color_image, load_window_icon, set_macos_activation_policy_regular,
    set_macos_app_icon_image, APP_ICON_BYTES,
};
use model::*;
use poll_promise::Promise;
use snippet::{
    build_json_layout_job_with_search, build_snippet_layout_job, render_snippet, SnippetLang,
};
use std::fs;
use std::path::PathBuf;
use theme::*;
use uuid::Uuid;
use widgets::*;


struct ApiClient {
    state: AppState,
    selected_folder_path: Vec<String>,
    selected_request_id: Option<String>,

    search_query: String,

    response_text: String,
    response_status: String,
    response_time: String,
    response_headers: Vec<(String, String)>,
    response_headers_bytes: usize,
    response_body_bytes: usize,
    request_headers_bytes: usize,
    request_body_bytes: usize,
    response_prepare_ms: u64,
    response_waiting_ms: u64,
    response_download_ms: u64,
    response_total_ms: u64,
    is_loading: bool,

    editing_url: String,
    editing_body: String,
    editing_name: String,
    editing_method: HttpMethod,
    editing_headers: Vec<KvRow>,
    editing_params: Vec<KvRow>,
    editing_cookies: Vec<KvRow>,
    editing_body_ext: Option<BodyExt>,
    editing_auth: Auth,
    editing_extractors: Vec<ResponseExtractor>,
    editing_request_id_for_history: Option<String>,

    storage_path: PathBuf,

    request_promise: Option<Promise<ResponseData>>,

    renaming_folder_id: Option<String>,
    rename_folder_text: String,

    request_tab: RequestTab,
    response_tab: ResponseTab,

    show_paste_modal: bool,
    paste_curl_text: String,
    paste_error: String,

    show_snippet_panel: bool,
    snippet_lang: SnippetLang,

    sidebar_view: SidebarView,
    show_env_modal: bool,
    selected_env_for_edit: Option<String>,

    toast: Option<(String, f32)>,
    focus_search_next_frame: bool,

    app_icon: Option<egui::TextureHandle>,
    macos_icon_set: bool,

    renaming_request_id: Option<String>,
    rename_request_text: String,
    request_rename_focus_pending: bool,
    /// (request_id, timestamp_secs) of the last click on a request row.
    /// Used for hand-rolled double-click detection — egui's `double_clicked()`
    /// doesn't fire reliably in this setup because the first click mutates
    /// state that re-drives the sidebar layout.
    last_request_click: Option<(String, f64)>,

    /// Pending file-dialog actions — executed at the top of the next
    /// `update()` frame rather than immediately inside the menu closure,
    /// so the popup has a chance to close visibly before `rfd` blocks the
    /// main thread on the OS file picker.
    pending_import: bool,
    pending_export_json: bool,
    pending_export_yaml: bool,

    /// Open "save draft" modal state.
    save_draft_open: bool,
    /// Tab index of the draft currently being saved (valid while modal is open).
    save_draft_tab_idx: Option<usize>,
    /// Path of folder IDs from root → destination folder. Empty = nothing
    /// selected yet. Supports nested subfolders, not just top-level collections.
    save_draft_target_path: Vec<String>,
    /// User-editable name for the request being saved.
    save_draft_name: String,
    /// Free-text filter over the folder tree inside the save-draft modal.
    save_draft_search: String,
    /// When Some, the "New folder" inline input is showing; the string holds
    /// the in-progress name. The new folder is created as a child of the
    /// currently-selected folder (or at root if no folder is selected).
    save_draft_new_folder_name: Option<String>,

    /// Drag-resizable vertical split between the request-editor section
    /// (top) and the response section (bottom). Units: logical pixels of
    /// the request-editor section. Clamped at render time.
    request_split_px: f32,

    /// Response Body display mode — Raw (verbatim text) or Pretty
    /// (JSON rendered as a collapsible tree).
    body_view: BodyView,
    /// Substring filter applied to the JSON tree view — matches keys
    /// or leaf values. Empty = show everything.
    body_tree_filter: String,
    /// Search query for the JSON body view — highlights matches inline
    /// rather than filtering out non-matches.
    body_search_query: String,
    /// Whether the inline search input is visible (toggled by the 🔍
    /// icon button in the body toolbar).
    body_search_visible: bool,
}

impl Default for ApiClient {
    fn default() -> Self {
        let storage_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rusty-requester")
            .join("data.json");

        let state = Self::load_state(&storage_path).unwrap_or_else(|| AppState {
            folders: vec![Folder {
                id: Uuid::new_v4().to_string(),
                name: "My Requests".to_string(),
                requests: vec![],
                subfolders: vec![],
            }],
            environments: vec![],
            active_env_id: None,
            history: vec![],
            drafts: vec![],
            open_tabs: vec![],
            active_tab_id: None,
        });

        let mut this = Self {
            state,
            selected_folder_path: vec![],
            selected_request_id: None,
            search_query: String::new(),
            response_text: String::new(),
            response_status: String::new(),
            response_time: String::new(),
            response_headers_bytes: 0,
            response_body_bytes: 0,
            request_headers_bytes: 0,
            request_body_bytes: 0,
            response_prepare_ms: 0,
            response_waiting_ms: 0,
            response_download_ms: 0,
            response_total_ms: 0,
            response_headers: vec![],
            is_loading: false,
            editing_url: String::new(),
            editing_body: String::new(),
            editing_name: String::new(),
            editing_method: HttpMethod::GET,
            editing_headers: vec![],
            editing_params: vec![],
            editing_cookies: vec![],
            editing_body_ext: None,
            editing_auth: Auth::None,
            editing_extractors: vec![],
            editing_request_id_for_history: None,
            storage_path,
            request_promise: None,
            renaming_folder_id: None,
            rename_folder_text: String::new(),
            request_tab: RequestTab::Params,
            response_tab: ResponseTab::Body,
            show_paste_modal: false,
            paste_curl_text: String::new(),
            paste_error: String::new(),
            show_snippet_panel: false,
            snippet_lang: SnippetLang::Curl,
            sidebar_view: SidebarView::Collections,
            show_env_modal: false,
            selected_env_for_edit: None,
            toast: None,
            focus_search_next_frame: false,
            app_icon: None,
            macos_icon_set: false,
            renaming_request_id: None,
            rename_request_text: String::new(),
            request_rename_focus_pending: false,
            last_request_click: None,
            pending_import: false,
            pending_export_json: false,
            pending_export_yaml: false,
            save_draft_open: false,
            save_draft_tab_idx: None,
            save_draft_target_path: Vec::new(),
            save_draft_name: String::new(),
            save_draft_search: String::new(),
            save_draft_new_folder_name: None,
            request_split_px: 320.0,
            body_view: BodyView::Json,
            body_tree_filter: String::new(),
            body_search_query: String::new(),
            body_search_visible: false,
        };
        // Restore active tab — if state has a saved `active_tab_id`,
        // activate that tab now. Otherwise fall back to the first open tab.
        let active_tab: Option<OpenTab> = {
            let id = this.state.active_tab_id.clone();
            let by_id = id.and_then(|id| {
                this.state.open_tabs.iter().find(|t| t.request_id == id).cloned()
            });
            by_id.or_else(|| this.state.open_tabs.first().cloned())
        };
        if let Some(tab) = active_tab {
            this.selected_folder_path = tab.folder_path;
            this.selected_request_id = Some(tab.request_id);
            this.load_request_for_editing();
        }
        this
    }
}


impl ApiClient {
    fn load_state(path: &PathBuf) -> Option<AppState> {
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save_state(&mut self) {
        // Sync the active-tab id into state so the workspace restores to
        // this tab on next launch.
        self.state.active_tab_id = self.selected_request_id.clone();
        if let Some(parent) = self.storage_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.state) {
            let _ = fs::write(&self.storage_path, json);
        }
    }

    fn get_current_folder_mut(&mut self) -> Option<&mut Folder> {
        if self.selected_folder_path.is_empty() {
            return None;
        }
        let path = self.selected_folder_path.clone();
        let mut folder = self.state.folders.iter_mut().find(|f| f.id == path[0])?;
        for id in &path[1..] {
            folder = folder.subfolders.iter_mut().find(|f| &f.id == id)?;
        }
        Some(folder)
    }

    fn get_current_request(&self) -> Option<Request> {
        let req_id = self.selected_request_id.as_ref()?;
        // Draft path: selected_folder_path is empty → look up in drafts
        if self.selected_folder_path.is_empty() {
            return self
                .state
                .drafts
                .iter()
                .find(|r| &r.id == req_id)
                .cloned();
        }
        let mut folder = self
            .state
            .folders
            .iter()
            .find(|f| f.id == self.selected_folder_path[0])?;
        for id in &self.selected_folder_path[1..] {
            folder = folder.subfolders.iter().find(|f| &f.id == id)?;
        }
        folder.requests.iter().find(|r| &r.id == req_id).cloned()
    }

    fn update_current_request<F>(&mut self, updater: F)
    where
        F: FnOnce(&mut Request),
    {
        if let Some(req_id) = self.selected_request_id.clone() {
            // Draft case: update the draft in state.drafts.
            if self.selected_folder_path.is_empty() {
                if let Some(r) = self.state.drafts.iter_mut().find(|r| r.id == req_id) {
                    updater(r);
                    self.save_state();
                }
                return;
            }
            if let Some(folder) = self.get_current_folder_mut() {
                if let Some(request) = folder.requests.iter_mut().find(|r| r.id == req_id) {
                    updater(request);
                }
            }
            self.save_state();
        }
    }

    fn commit_editing(&mut self) {
        let name = self.editing_name.clone();
        let method = self.editing_method.clone();
        let url = self.editing_url.clone();
        let body = self.editing_body.clone();
        let headers = self.editing_headers.clone();
        let params = self.editing_params.clone();
        let cookies = self.editing_cookies.clone();
        let body_ext = self.editing_body_ext.clone();
        let auth = self.editing_auth.clone();
        let extractors = self.editing_extractors.clone();
        self.update_current_request(|req| {
            req.name = name;
            req.method = method;
            req.url = url;
            req.body = body;
            req.headers = headers;
            req.query_params = params;
            req.cookies = cookies;
            req.body_ext = body_ext;
            req.auth = auth;
            req.extractors = extractors;
        });
    }

    fn send_request(&mut self) {
        self.commit_editing();
        let env = self.active_environment().cloned();
        if let Some(request) = self.get_current_request() {
            self.is_loading = true;
            self.response_text = "Loading...".to_string();
            self.response_status = "Sending request...".to_string();
            self.response_time = String::new();
            self.response_headers.clear();
            self.request_promise = Some(Promise::spawn_thread("request", move || {
                Self::execute_request(&request, env.as_ref())
            }));
        }
    }

    fn active_environment(&self) -> Option<&Environment> {
        let id = self.state.active_env_id.as_ref()?;
        self.state.environments.iter().find(|e| &e.id == id)
    }

    /// Run the current request's extractors against the just-received
    /// response and write each result into the active environment. A
    /// toast summarizes how many values were captured so the user has
    /// feedback that chaining actually happened.
    fn apply_response_extractors(&mut self) {
        let Some(req) = self.get_current_request() else { return };
        if req.extractors.is_empty() {
            return;
        }
        let Some(env_id) = self.state.active_env_id.clone() else {
            return;
        };

        let body = self.response_text.clone();
        let headers = self.response_headers.clone();
        let status = self.response_status.clone();

        let mut writes: Vec<(String, String)> = Vec::new();
        let mut missed: Vec<String> = Vec::new();
        for ex in &req.extractors {
            if !ex.enabled {
                continue;
            }
            let var = ex.variable.trim();
            if var.is_empty() {
                continue;
            }
            let value = match ex.source {
                ExtractorSource::Body => extract::eval_body_path(&body, ex.expression.trim()),
                ExtractorSource::Header => headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(ex.expression.trim()))
                    .map(|(_, v)| v.clone()),
                // Leading `HTTP/1.1 ` is stripped in response_status already;
                // we still just write whatever is there verbatim.
                ExtractorSource::Status => Some(
                    status
                        .split_whitespace()
                        .next()
                        .unwrap_or(&status)
                        .to_string(),
                ),
            };
            match value {
                Some(v) => writes.push((var.to_string(), v)),
                None => missed.push(var.to_string()),
            }
        }

        if writes.is_empty() && missed.is_empty() {
            return;
        }

        if let Some(env) = self
            .state
            .environments
            .iter_mut()
            .find(|e| e.id == env_id)
        {
            for (var, val) in &writes {
                match env.variables.iter_mut().find(|kv| kv.key == *var) {
                    Some(existing) => existing.value = val.clone(),
                    None => env.variables.push(KvRow::new(var, val)),
                }
            }
        }
        self.save_state();

        let msg = match (writes.len(), missed.len()) {
            (n, 0) if n > 0 => format!("Extracted {} value(s)", n),
            (0, m) if m > 0 => format!("Extractor missed: {}", missed.join(", ")),
            (n, m) => format!("Extracted {}, missed {}", n, m),
        };
        self.show_toast(msg);
    }

    fn push_history_entry(&mut self) {
        let Some(req) = self.get_current_request() else {
            return;
        };
        let mut preview = self.response_text.clone();
        if preview.len() > 256 {
            preview.truncate(256);
            preview.push_str("…");
        }
        let time_ms = self
            .response_time
            .trim_end_matches("ms")
            .parse::<u64>()
            .unwrap_or(0);
        let entry = HistoryEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            method: req.method,
            url: req.url,
            status: self.response_status.clone(),
            time_ms,
            response_preview: preview,
        };
        self.state.history.insert(0, entry);
        const MAX: usize = 200;
        if self.state.history.len() > MAX {
            self.state.history.truncate(MAX);
        }
        self.save_state();
    }

    fn execute_request(request: &Request, env: Option<&Environment>) -> ResponseData {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let t_prepare_start = std::time::Instant::now();

        // Apply environment substitution to all string fields up-front.
        let final_url_base = substitute_vars(&request.url, env);
        let sub_params = substitute_kvs(&request.query_params, env);
        let sub_headers = substitute_kvs(&request.headers, env);
        let sub_cookies = substitute_kvs(&request.cookies, env);
        let sub_body = substitute_vars(&request.body, env);
        let sub_auth = match &request.auth {
            Auth::None => Auth::None,
            Auth::Bearer { token } => Auth::Bearer {
                token: substitute_vars(token, env),
            },
            Auth::Basic { username, password } => Auth::Basic {
                username: substitute_vars(username, env),
                password: substitute_vars(password, env),
            },
        };

        rt.block_on(async {
            let client = reqwest::Client::new();
            // reqwest requires an absolute URL; a missing scheme surfaces
            // as the opaque "builder error". Match Postman behavior and
            // auto-prepend https:// so the user's `api.example.com/foo`
            // just works.
            let final_url_base = ensure_url_scheme(&final_url_base);
            let final_url = curl::build_full_url(&final_url_base, &sub_params);

            let mut req_builder = match request.method {
                HttpMethod::GET => client.get(&final_url),
                HttpMethod::POST => client.post(&final_url),
                HttpMethod::PUT => client.put(&final_url),
                HttpMethod::DELETE => client.delete(&final_url),
                HttpMethod::PATCH => client.patch(&final_url),
                HttpMethod::HEAD => client.head(&final_url),
                HttpMethod::OPTIONS => client.request(reqwest::Method::OPTIONS, &final_url),
            };

            let mut cookie_parts: Vec<String> = Vec::new();
            for h in &sub_headers {
                if !h.enabled || h.key.trim().is_empty() {
                    continue;
                }
                if h.key.eq_ignore_ascii_case("cookie") {
                    cookie_parts.push(h.value.clone());
                    continue;
                }
                req_builder = req_builder.header(&h.key, &h.value);
            }
            for c in &sub_cookies {
                if c.enabled && !c.key.is_empty() {
                    cookie_parts.push(format!("{}={}", c.key, c.value));
                }
            }
            if !cookie_parts.is_empty() {
                req_builder = req_builder.header("Cookie", cookie_parts.join("; "));
            }

            match &sub_auth {
                Auth::Bearer { token } if !token.is_empty() => {
                    req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
                }
                Auth::Basic { username, password } if !username.is_empty() => {
                    let encoded = base64::engine::general_purpose::STANDARD
                        .encode(format!("{}:{}", username, password));
                    req_builder =
                        req_builder.header("Authorization", format!("Basic {}", encoded));
                }
                _ => {}
            }

            // Body handling — depends on body_ext mode.
            match &request.body_ext {
                None => {
                    if !sub_body.is_empty() {
                        req_builder = req_builder.body(sub_body.clone());
                    }
                }
                Some(BodyExt::FormUrlEncoded { fields }) => {
                    let pairs: Vec<(String, String)> = substitute_kvs(fields, env)
                        .into_iter()
                        .filter(|f| f.enabled && !f.key.is_empty())
                        .map(|f| (f.key, f.value))
                        .collect();
                    req_builder = req_builder.form(&pairs);
                }
                Some(BodyExt::MultipartForm { fields }) => {
                    let mut form = reqwest::multipart::Form::new();
                    for f in substitute_kvs(fields, env) {
                        if f.enabled && !f.key.is_empty() {
                            form = form.text(f.key, f.value);
                        }
                    }
                    req_builder = req_builder.multipart(form);
                }
                Some(BodyExt::GraphQL { variables }) => {
                    let vars_value: serde_json::Value =
                        serde_json::from_str(&substitute_vars(variables, env))
                            .unwrap_or(serde_json::json!({}));
                    let body_json = serde_json::json!({
                        "query": sub_body,
                        "variables": vars_value,
                    });
                    req_builder = req_builder.json(&body_json);
                }
            }

            // Build (but don't consume) the request so we can size the
            // outgoing headers + body before firing it. reqwest adds a
            // couple of default headers (Host, Accept, etc.) that also
            // count toward request size.
            let built = match req_builder.build() {
                Ok(r) => r,
                Err(e) => {
                    return ResponseData {
                        body: format!("Error: {}", e),
                        status: "Failed".to_string(),
                        time: "0ms".to_string(),
                        headers: vec![],
                        response_headers_bytes: 0,
                        response_body_bytes: 0,
                        request_headers_bytes: 0,
                        request_body_bytes: 0,
                        prepare_ms: 0,
                        waiting_ms: 0,
                        download_ms: 0,
                        total_ms: 0,
                    };
                }
            };
            let req_line = format!(
                "{} {} HTTP/1.1\r\n",
                built.method(),
                built
                    .url()
                    .as_str(),
            );
            let request_headers_bytes = req_line.len()
                + built
                    .headers()
                    .iter()
                    .map(|(k, v)| k.as_str().len() + 2 + v.as_bytes().len() + 2)
                    .sum::<usize>()
                + 2; // trailing CRLF between headers and body
            let request_body_bytes = built
                .body()
                .and_then(|b| b.as_bytes())
                .map(|b| b.len())
                .unwrap_or(0);

            let t_send = std::time::Instant::now();
            let prepare_ms = t_send.saturating_duration_since(t_prepare_start).as_millis() as u64;
            match client.execute(built).await {
                Ok(response) => {
                    let t_headers = std::time::Instant::now();
                    let waiting_ms = t_headers.saturating_duration_since(t_send).as_millis() as u64;
                    let status = format!(
                        "{} {}",
                        response.status().as_u16(),
                        response.status().canonical_reason().unwrap_or("")
                    );

                    // Status line + headers + trailing CRLF.
                    let status_line = format!(
                        "HTTP/1.1 {} {}\r\n",
                        response.status().as_u16(),
                        response.status().canonical_reason().unwrap_or(""),
                    );
                    let response_headers_bytes = status_line.len()
                        + response
                            .headers()
                            .iter()
                            .map(|(k, v)| k.as_str().len() + 2 + v.as_bytes().len() + 2)
                            .sum::<usize>()
                        + 2;

                    let headers: Vec<(String, String)> = response
                        .headers()
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.to_string(),
                                v.to_str().unwrap_or("<non-ascii>").to_string(),
                            )
                        })
                        .collect();

                    let body = response
                        .text()
                        .await
                        .unwrap_or_else(|e| format!("Error reading body: {}", e));
                    let t_done = std::time::Instant::now();
                    let download_ms = t_done.saturating_duration_since(t_headers).as_millis() as u64;
                    let total_ms = t_done.saturating_duration_since(t_prepare_start).as_millis() as u64;
                    let time = format!("{} ms", total_ms);
                    let response_body_bytes = body.len();
                    let formatted_body =
                        match serde_json::from_str::<serde_json::Value>(&body) {
                            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or(body),
                            Err(_) => body,
                        };

                    ResponseData {
                        body: formatted_body,
                        status,
                        time,
                        headers,
                        response_headers_bytes,
                        response_body_bytes,
                        request_headers_bytes,
                        request_body_bytes,
                        prepare_ms,
                        waiting_ms,
                        download_ms,
                        total_ms,
                    }
                }
                Err(e) => ResponseData {
                    body: format_request_error(&e),
                    status: "Failed".to_string(),
                    time: "0ms".to_string(),
                    headers: vec![],
                    response_headers_bytes: 0,
                    response_body_bytes: 0,
                    request_headers_bytes,
                    request_body_bytes,
                    prepare_ms,
                    waiting_ms: 0,
                    download_ms: 0,
                    total_ms: prepare_ms,
                },
            }
        })
    }

    fn load_request_for_editing(&mut self) {
        if let Some(r) = self.get_current_request() {
            self.editing_url = r.url;
            self.editing_body = r.body;
            self.editing_name = r.name;
            self.editing_method = r.method.clone();
            self.editing_headers = r.headers;
            self.editing_params = r.query_params;
            self.editing_cookies = r.cookies;
            self.editing_body_ext = r.body_ext;
            self.editing_auth = r.auth;
            self.editing_extractors = r.extractors;
            self.editing_request_id_for_history = Some(r.id);
            // Capture method for history entry too
            let _ = r.method;
        }
    }

    fn show_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), 2.5));
    }

    fn open_request(&mut self, folder_path: Vec<String>, request_id: String) {
        if let Some(existing) = self.state.open_tabs.iter().position(|t| t.request_id == request_id) {
            let tab = self.state.open_tabs[existing].clone();
            self.selected_folder_path = tab.folder_path;
            self.selected_request_id = Some(tab.request_id);
        } else {
            self.state.open_tabs.push(OpenTab {
                folder_path: folder_path.clone(),
                request_id: request_id.clone(),
            });
            self.selected_folder_path = folder_path;
            self.selected_request_id = Some(request_id);
        }
        self.load_request_for_editing();
        self.response_text.clear();
        self.response_status.clear();
        self.response_time.clear();
        self.response_headers.clear();
    }

    fn close_tab(&mut self, idx: usize) {
        if idx >= self.state.open_tabs.len() {
            return;
        }
        let closing = self.state.open_tabs.remove(idx);
        // If it was a draft, discard the draft entirely (user chose to
        // close without saving).
        if closing.folder_path.is_empty() {
            self.state.drafts.retain(|d| d.id != closing.request_id);
        }
        let was_active = self.selected_request_id.as_deref() == Some(closing.request_id.as_str());
        if was_active {
            if self.state.open_tabs.is_empty() {
                self.clear_selection();
            } else {
                let new_idx = idx.min(self.state.open_tabs.len() - 1);
                let tab = self.state.open_tabs[new_idx].clone();
                self.selected_folder_path = tab.folder_path;
                self.selected_request_id = Some(tab.request_id);
                self.load_request_for_editing();
                self.response_text.clear();
                self.response_status.clear();
                self.response_time.clear();
                self.response_headers.clear();
            }
        }
    }

    fn close_other_tabs(&mut self, keep_idx: usize) {
        if keep_idx >= self.state.open_tabs.len() {
            return;
        }
        let keep = self.state.open_tabs.remove(keep_idx);
        // Discard any draft requests whose tabs are about to be closed.
        let keep_id = keep.request_id.clone();
        let draft_ids: Vec<String> = self
            .state
            .open_tabs
            .iter()
            .filter(|t| t.folder_path.is_empty())
            .map(|t| t.request_id.clone())
            .collect();
        self.state
            .drafts
            .retain(|d| d.id == keep_id || !draft_ids.contains(&d.id));
        self.state.open_tabs.clear();
        self.state.open_tabs.push(keep.clone());
        self.selected_folder_path = keep.folder_path;
        self.selected_request_id = Some(keep.request_id);
        self.load_request_for_editing();
    }

    fn close_all_tabs(&mut self) {
        // Discard all drafts (they're only alive because they had a tab).
        let draft_ids: Vec<String> = self
            .state
            .open_tabs
            .iter()
            .filter(|t| t.folder_path.is_empty())
            .map(|t| t.request_id.clone())
            .collect();
        self.state.drafts.retain(|d| !draft_ids.contains(&d.id));
        self.state.open_tabs.clear();
        self.clear_selection();
    }

    fn prune_stale_tabs(&mut self) {
        // Collect the data we need before the retain closure (so we don't
        // borrow `self.state.open_tabs` mutably at the same time).
        let folders = self.state.folders.clone();
        let drafts = self.state.drafts.clone();
        self.state.open_tabs.retain(|t| {
            find_request_info(&folders, &drafts, &t.folder_path, &t.request_id).is_some()
        });
        if let Some(rid) = self.selected_request_id.clone() {
            if !self.state.open_tabs.iter().any(|t| t.request_id == rid) {
                if let Some(first) = self.state.open_tabs.first().cloned() {
                    self.selected_folder_path = first.folder_path;
                    self.selected_request_id = Some(first.request_id);
                    self.load_request_for_editing();
                } else {
                    self.clear_selection();
                }
            }
        }
    }

    fn clear_selection(&mut self) {
        self.selected_folder_path.clear();
        self.selected_request_id = None;
        self.editing_name.clear();
        self.editing_url.clear();
        self.editing_body.clear();
        self.editing_headers.clear();
        self.editing_params.clear();
        self.editing_cookies.clear();
        self.editing_auth = Auth::None;
        self.response_text.clear();
        self.response_status.clear();
        self.response_time.clear();
        self.response_headers.clear();
    }

    fn rename_request(&mut self, request_id: &str, new_name: String) {
        fn go(folders: &mut Vec<Folder>, id: &str, name: &str) -> bool {
            for f in folders {
                for r in f.requests.iter_mut() {
                    if r.id == id {
                        r.name = name.to_string();
                        return true;
                    }
                }
                if go(&mut f.subfolders, id, name) {
                    return true;
                }
            }
            false
        }
        if go(&mut self.state.folders, request_id, &new_name) {
            if self.selected_request_id.as_deref() == Some(request_id) {
                self.editing_name = new_name;
            }
            self.save_state();
        }
    }

    fn do_import_file(&mut self) {
        let path = rfd::FileDialog::new()
            .add_filter("Collections", &["json", "yaml", "yml"])
            .add_filter("All files", &["*"])
            .pick_file();
        let Some(path) = path else { return };
        match io::import_from_file(&path) {
            Ok(folders) => {
                let n = folders.len();
                self.state.folders.extend(folders);
                self.save_state();
                self.show_toast(format!("Imported {} folder(s)", n));
            }
            Err(e) => {
                self.show_toast(format!("Import failed: {}", e));
            }
        }
    }

    fn do_export_all(&mut self, format: io::Format) {
        let (ext, label) = match format {
            io::Format::Json => ("json", "JSON"),
            io::Format::Yaml => ("yaml", "YAML"),
        };
        let path = rfd::FileDialog::new()
            .add_filter(label, &[ext])
            .set_file_name(&format!("rusty-requester.{}", ext))
            .save_file();
        let Some(path) = path else { return };
        match io::export_string(&self.state.folders, format) {
            Ok(content) => match std::fs::write(&path, content) {
                Ok(_) => self.show_toast(format!("Exported as {}", label)),
                Err(e) => self.show_toast(format!("Write failed: {}", e)),
            },
            Err(e) => self.show_toast(format!("Export failed: {}", e)),
        }
    }

    fn do_export_folder(&mut self, folder_id: &str, format: io::Format) {
        fn find<'a>(folders: &'a [Folder], id: &str) -> Option<&'a Folder> {
            for f in folders {
                if f.id == id {
                    return Some(f);
                }
                if let Some(sub) = find(&f.subfolders, id) {
                    return Some(sub);
                }
            }
            None
        }
        let Some(folder) = find(&self.state.folders, folder_id).cloned() else {
            return;
        };
        let (ext, label) = match format {
            io::Format::Json => ("json", "JSON"),
            io::Format::Yaml => ("yaml", "YAML"),
        };
        let suggested = format!(
            "{}.{}",
            sanitize_filename(&folder.name).unwrap_or_else(|| "collection".to_string()),
            ext
        );
        let path = rfd::FileDialog::new()
            .add_filter(label, &[ext])
            .set_file_name(&suggested)
            .save_file();
        let Some(path) = path else { return };
        match io::export_string(std::slice::from_ref(&folder), format) {
            Ok(content) => match std::fs::write(&path, content) {
                Ok(_) => self.show_toast(format!("Exported '{}' as {}", folder.name, label)),
                Err(e) => self.show_toast(format!("Write failed: {}", e)),
            },
            Err(e) => self.show_toast(format!("Export failed: {}", e)),
        }
    }
}



impl eframe::App for ApiClient {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle deferred file-dialog actions from the previous frame.
        // These would otherwise block the main thread before the menu
        // popup that triggered them had a chance to close.
        if self.pending_import {
            self.pending_import = false;
            self.do_import_file();
        }
        if self.pending_export_json {
            self.pending_export_json = false;
            self.do_export_all(io::Format::Json);
        }
        if self.pending_export_yaml {
            self.pending_export_yaml = false;
            self.do_export_all(io::Format::Yaml);
        }

        let (cmd_enter, cmd_k, cmd_s, f2) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(egui::Key::Enter),
                i.modifiers.command && i.key_pressed(egui::Key::K),
                i.modifiers.command && i.key_pressed(egui::Key::S),
                i.key_pressed(egui::Key::F2),
            )
        });
        if cmd_enter && self.selected_request_id.is_some() && !self.is_loading {
            self.send_request();
        }
        if cmd_k {
            self.focus_search_next_frame = true;
        }
        // Cmd/Ctrl+S — if the active tab is a draft, open the Save-draft
        // modal to pick a destination collection. Saved requests are
        // auto-persisted to disk on every edit so this shortcut is a no-op
        // for them (other than a confirmation toast).
        if cmd_s && self.selected_request_id.is_some() {
            if self.selected_folder_path.is_empty() {
                let draft_id = self.selected_request_id.clone().unwrap();
                if let Some(idx) = self
                    .state
                    .open_tabs
                    .iter()
                    .position(|t| t.is_draft() && t.request_id == draft_id)
                {
                    self.begin_save_draft(idx);
                }
            } else {
                self.show_toast("Saved");
            }
        }
        // F2 — rename the active request (VS Code / Finder convention)
        if f2 && self.renaming_request_id.is_none() {
            if let Some(id) = self.selected_request_id.clone() {
                if let Some(req) = self.get_current_request() {
                    self.renaming_request_id = Some(id);
                    self.rename_request_text = req.name;
                    self.request_rename_focus_pending = true;
                }
            }
        }

        if self.app_icon.is_none() {
            if let Some(ci) = load_icon_color_image() {
                self.app_icon = Some(ctx.load_texture(
                    "app_icon",
                    ci,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        // Set the macOS Dock / Cmd+Tab icon once the window is up. Doing this
        // from the first `update()` (rather than from the eframe creation
        // callback) ensures NSApp is fully initialized by winit/eframe before
        // we override `applicationIconImage` and force a Regular activation
        // policy. Without this, the running process inherits the parent
        // terminal's icon in Cmd+Tab.
        // Set the icon image once the window is up. Activation policy was
        // already set in main() before eframe took over the run loop —
        // setActivationPolicy(Regular) is restricted by macOS once NSApp.run
        // is processing events.
        if !self.macos_icon_set {
            self.macos_icon_set = true;
            if let Err(e) = set_macos_app_icon_image(APP_ICON_BYTES) {
                eprintln!("[icon] set_macos_app_icon_image failed: {}", e);
            }
        }

        if let Some(promise) = &self.request_promise {
            if let Some(r) = promise.ready() {
                self.response_text = r.body.clone();
                self.response_status = r.status.clone();
                self.response_time = r.time.clone();
                self.response_headers = r.headers.clone();
                self.response_headers_bytes = r.response_headers_bytes;
                self.response_body_bytes = r.response_body_bytes;
                self.request_headers_bytes = r.request_headers_bytes;
                self.request_body_bytes = r.request_body_bytes;
                self.response_prepare_ms = r.prepare_ms;
                self.response_waiting_ms = r.waiting_ms;
                self.response_download_ms = r.download_ms;
                self.response_total_ms = r.total_ms;
                self.is_loading = false;
                self.request_promise = None;
                self.push_history_entry();
                self.apply_response_extractors();
            } else {
                ctx.request_repaint();
            }
        }

        theme::apply_style(ctx);

        // Paint the ENTIRE window background with C_BG before any panels
        // render. egui leaves a ~50-60 logical-pixel "gutter" between
        // `SidePanel::left` and `CentralPanel` (for separator/resize
        // reservation) that neither panel's fill covers — without this
        // base-layer paint, that gutter surfaces as egui's default
        // near-black. Using the background layer ensures panels draw on top
        // of our fill, not the other way around.
        {
            let screen_rect = ctx.screen_rect();
            ctx.layer_painter(egui::LayerId::background()).rect_filled(
                screen_rect,
                egui::Rounding::ZERO,
                C_BG,
            );
        }

        self.render_sidebar(ctx);
        self.render_snippet_panel(ctx);
        self.render_central(ctx);
        self.render_paste_modal(ctx);
        self.render_env_modal(ctx);
        self.render_save_draft_modal(ctx);
        self.render_toast(ctx);
    }
}

impl ApiClient {
    fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .default_width(320.0)
            .width_range(260.0..=640.0)
            .resizable(true)
            // Disabled — egui adds a gutter/reserve region around the
            // separator line that used to surface as a dark strip between
            // sidebar and central panel. The base-layer bg paint in
            // update() plus matching panel colors make the boundary invisible
            // without the separator.
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(C_PANEL)
                    // Asymmetric: less right-padding so sidebar content
                    // butts close to the (invisible) boundary with central.
                    .inner_margin(egui::Margin {
                        left: 10.0,
                        right: 4.0,
                        top: 10.0,
                        bottom: 10.0,
                    }),
            )
            .show(ctx, |ui| {
                // Defensive floor — paint the full sidebar rect before any
                // children, so scroll tracks / code editors etc. don't
                // surface with egui-default near-black fills.
                ui.painter()
                    .rect_filled(ui.max_rect(), egui::Rounding::ZERO, C_PANEL);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if let Some(tex) = &self.app_icon {
                        ui.add(
                            egui::Image::from_texture(tex)
                                .fit_to_exact_size(egui::vec2(24.0, 24.0))
                                .rounding(egui::Rounding::same(5.0)),
                        );
                    }
                    ui.label(
                        egui::RichText::new("Rusty Requester")
                            .size(15.0)
                            .strong()
                            .color(C_TEXT),
                    );
                });
                ui.add_space(6.0);
                self.render_environment_picker(ui);
                ui.add_space(8.0);
                // Tab toggle. Use fixed-size Buttons (not `selectable_label`)
                // so the width doesn't shift when the selected state changes.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let v = self.sidebar_view;
                    let tab_size = egui::vec2(130.0, 24.0);

                    let coll_label = format!("Collections ({})", self.state.folders.len());
                    let coll_selected = v == SidebarView::Collections;
                    let coll_btn = egui::Button::new(
                        egui::RichText::new(coll_label)
                            .size(12.0)
                            .color(if coll_selected { C_ACCENT } else { C_MUTED }),
                    )
                    .fill(if coll_selected {
                        C_ACCENT.linear_multiply(0.15)
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .stroke(egui::Stroke::NONE)
                    .rounding(egui::Rounding::same(5.0))
                    .min_size(tab_size);
                    if ui.add(coll_btn).clicked() {
                        self.sidebar_view = SidebarView::Collections;
                    }

                    let hist_label = format!("History ({})", self.state.history.len());
                    let hist_selected = v == SidebarView::History;
                    let hist_btn = egui::Button::new(
                        egui::RichText::new(hist_label)
                            .size(12.0)
                            .color(if hist_selected { C_ACCENT } else { C_MUTED }),
                    )
                    .fill(if hist_selected {
                        C_ACCENT.linear_multiply(0.15)
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .stroke(egui::Stroke::NONE)
                    .rounding(egui::Rounding::same(5.0))
                    .min_size(tab_size);
                    if ui.add(hist_btn).clicked() {
                        self.sidebar_view = SidebarView::History;
                    }
                });
                ui.add_space(8.0);
                if self.sidebar_view == SidebarView::History {
                    self.render_history_view(ui);
                    return;
                }

                if ui
                    .add_sized(
                        [ui.available_width(), 32.0],
                        egui::Button::new(
                            egui::RichText::new("➕  New Collection")
                                .size(13.0)
                                .color(egui::Color32::WHITE)
                                .strong(),
                        )
                        .fill(C_ACCENT)
                        .rounding(egui::Rounding::same(8.0))
                        .stroke(egui::Stroke::NONE),
                    )
                    .clicked()
                {
                    self.state.folders.push(Folder {
                        id: Uuid::new_v4().to_string(),
                        name: format!("Collection {}", self.state.folders.len() + 1),
                        requests: vec![],
                        subfolders: vec![],
                    });
                    self.save_state();
                }

                ui.add_space(6.0);

                let mut action_import_file = false;
                let mut action_paste_curl = false;
                let mut action_export_json = false;
                let mut action_export_yaml = false;

                ui.horizontal(|ui| {
                    let btn_w = (ui.available_width() - 6.0) / 2.0;
                    ui.menu_button(
                        egui::RichText::new("📥 Import").size(12.0).color(C_TEXT),
                        |ui| {
                            ui.set_min_width(200.0);
                            if ui.button("Import collection file...").clicked() {
                                action_import_file = true;
                                ui.close_menu();
                            }
                            if ui.button("Paste cURL command...").clicked() {
                                action_paste_curl = true;
                                ui.close_menu();
                            }
                        },
                    )
                    .response
                    .on_hover_text("Import JSON / YAML / Postman / cURL");
                    let _ = btn_w;

                    ui.menu_button(
                        egui::RichText::new("📤 Export").size(12.0).color(C_TEXT),
                        |ui| {
                            ui.set_min_width(200.0);
                            let enabled = !self.state.folders.is_empty();
                            if ui
                                .add_enabled(enabled, egui::Button::new("Export all as JSON..."))
                                .clicked()
                            {
                                action_export_json = true;
                                ui.close_menu();
                            }
                            if ui
                                .add_enabled(enabled, egui::Button::new("Export all as YAML..."))
                                .clicked()
                            {
                                action_export_yaml = true;
                                ui.close_menu();
                            }
                        },
                    );
                });

                // Defer file-dialog actions to the next frame so the menu
                // popup has a chance to close visibly before `rfd` blocks
                // the main thread. The blocking dialog otherwise freezes
                // the menu in its "open" state on screen.
                if action_import_file {
                    self.pending_import = true;
                }
                if action_paste_curl {
                    self.show_paste_modal = true;
                    self.paste_curl_text.clear();
                    self.paste_error.clear();
                }
                if action_export_json {
                    self.pending_export_json = true;
                }
                if action_export_yaml {
                    self.pending_export_yaml = true;
                }

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    // Hand-drawn magnifying-glass icon — avoids emoji-font
                    // fallback that looks pixelated in egui's default font.
                    let (icon_rect, _) = ui.allocate_exact_size(
                        egui::vec2(18.0, 24.0),
                        egui::Sense::hover(),
                    );
                    paint_search_icon(ui.painter(), icon_rect.center(), C_MUTED);

                    let clear_w = if self.search_query.is_empty() { 0.0 } else { 22.0 };
                    let search_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .desired_width(ui.available_width() - clear_w)
                            .hint_text("Search (⌘K)"),
                    );
                    if self.focus_search_next_frame {
                        self.focus_search_next_frame = false;
                        search_resp.request_focus();
                    }
                    if !self.search_query.is_empty()
                        && close_x_button(ui, "Clear search").clicked()
                    {
                        self.search_query.clear();
                    }
                });
                if !self.search_query.is_empty() {
                    let total = count_matches(&self.state.folders, &self.search_query.to_lowercase());
                    ui.label(
                        egui::RichText::new(format!("{} match(es)", total))
                            .size(11.0)
                            .color(C_MUTED),
                    );
                }

                ui.add_space(6.0);

                // Scroll with trackpad / wheel. Bar is hidden entirely —
                // `VisibleWhenNeeded` was causing visible width shifts when
                // the bar faded in/out on pointer hover, which showed as the
                // sidebar "resizing on pointer move".
                egui::ScrollArea::vertical()
                    .id_salt("sidebar_scroll")
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(
                        egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                    )
                    .show(ui, |ui| {
                        // Section header — small uppercase label so users can
                        // distinguish *collections* (top-level) from
                        // *folders* (nested) at a glance.
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("COLLECTIONS")
                                .size(10.5)
                                .strong()
                                .color(C_MUTED),
                        );
                        ui.add_space(4.0);

                        let folders = self.state.folders.clone();
                        let query = self.search_query.to_lowercase();
                        for folder in &folders {
                            if !query.is_empty() && !folder_matches(folder, &query) {
                                continue;
                            }
                            self.render_folder(ui, folder, vec![folder.id.clone()], 0);
                        }
                    });
            });
    }

    fn render_environment_picker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Env").size(11.0).color(C_MUTED));
            let active_name = self
                .state
                .active_env_id
                .as_ref()
                .and_then(|id| self.state.environments.iter().find(|e| &e.id == id))
                .map(|e| e.name.clone())
                .unwrap_or_else(|| "No env".to_string());
            egui::ComboBox::from_id_salt("env_picker")
                .selected_text(active_name)
                .width(ui.available_width() - 70.0)
                .show_ui(ui, |ui| {
                    let mut new_id: Option<Option<String>> = None;
                    if ui
                        .selectable_label(self.state.active_env_id.is_none(), "No env")
                        .clicked()
                    {
                        new_id = Some(None);
                    }
                    for env in &self.state.environments {
                        let selected = self.state.active_env_id.as_deref() == Some(&env.id);
                        if ui.selectable_label(selected, &env.name).clicked() {
                            new_id = Some(Some(env.id.clone()));
                        }
                    }
                    if let Some(v) = new_id {
                        self.state.active_env_id = v;
                        self.save_state();
                    }
                });
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("⚙").size(13.0))
                        .min_size(egui::vec2(28.0, 24.0)),
                )
                .on_hover_text("Manage environments")
                .clicked()
            {
                self.show_env_modal = true;
                if self.selected_env_for_edit.is_none() {
                    self.selected_env_for_edit =
                        self.state.environments.first().map(|e| e.id.clone());
                }
            }
        });
    }

    fn render_history_view(&mut self, ui: &mut egui::Ui) {
        if self.state.history.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("No requests sent yet.")
                        .color(C_MUTED)
                        .size(12.0),
                );
            });
            return;
        }
        let mut clear = false;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} entries (newest first)", self.state.history.len()))
                    .size(11.0)
                    .color(C_MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(egui::RichText::new("Clear").size(11.0).color(C_RED))
                    .clicked()
                {
                    clear = true;
                }
            });
        });
        ui.add_space(4.0);

        if clear {
            self.state.history.clear();
            self.save_state();
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("history_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let entries = self.state.history.clone();
                for entry in &entries {
                    let mc = method_color(&entry.method);
                    let sc = status_color(&entry.status);
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 50.0),
                        egui::Sense::click(),
                    );
                    if ui.is_rect_visible(rect) {
                        let bg = if resp.hovered() {
                            C_ELEVATED
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        ui.painter()
                            .rect_filled(rect, egui::Rounding::same(6.0), bg);

                        // Method as colored text (no pill bg)
                        let method_left = rect.left() + 10.0;
                        let method_y = rect.top() + 17.0;
                        let method_font =
                            egui::FontId::new(10.0, egui::FontFamily::Proportional);
                        ui.painter().text(
                            egui::pos2(method_left, method_y),
                            egui::Align2::LEFT_CENTER,
                            format!("{}", entry.method),
                            method_font,
                            mc,
                        );

                        // Status + time on the right
                        let info = format!("{}  ·  {}ms", entry.status, entry.time_ms);
                        ui.painter().text(
                            egui::pos2(method_left + 46.0, method_y),
                            egui::Align2::LEFT_CENTER,
                            info,
                            egui::FontId::new(11.0, egui::FontFamily::Proportional),
                            sc,
                        );

                        // URL beneath
                        let url_font = egui::FontId::new(11.5, egui::FontFamily::Proportional);
                        let max_w = rect.width() - 16.0;
                        let elided = elide(&entry.url, max_w, &url_font, ui);
                        ui.painter().text(
                            egui::pos2(rect.left() + 8.0, rect.top() + 33.0),
                            egui::Align2::LEFT_TOP,
                            elided,
                            url_font,
                            C_TEXT,
                        );
                    }
                    ui.add_space(2.0);
                }
            });
    }

    fn render_env_modal(&mut self, ctx: &egui::Context) {
        if !self.show_env_modal {
            return;
        }
        let mut open = self.show_env_modal;
        let mut close_modal = false;
        let mut create_env = false;
        let mut delete_id: Option<String> = None;

        egui::Window::new("Environments")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .default_height(420.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left column: env list
                    ui.vertical(|ui| {
                        ui.set_min_width(160.0);
                        ui.set_max_width(160.0);
                        ui.label(
                            egui::RichText::new("Environments")
                                .size(11.0)
                                .strong()
                                .color(C_MUTED),
                        );
                        ui.add_space(4.0);
                        let envs = self.state.environments.clone();
                        for env in &envs {
                            let selected =
                                self.selected_env_for_edit.as_deref() == Some(env.id.as_str());
                            if ui
                                .selectable_label(selected, &env.name)
                                .clicked()
                            {
                                self.selected_env_for_edit = Some(env.id.clone());
                            }
                        }
                        ui.add_space(6.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("+ New environment")
                                        .size(11.0)
                                        .color(C_ACCENT),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::new(1.0, C_BORDER)),
                            )
                            .clicked()
                        {
                            create_env = true;
                        }
                    });

                    ui.separator();

                    // Right column: editor for selected env
                    ui.vertical(|ui| {
                        let id = self.selected_env_for_edit.clone();
                        let env_idx = id.as_ref().and_then(|id| {
                            self.state.environments.iter().position(|e| &e.id == id)
                        });
                        if let Some(idx) = env_idx {
                            let mut name = self.state.environments[idx].name.clone();
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Name")
                                        .size(11.0)
                                        .color(C_MUTED),
                                );
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut name)
                                            .desired_width(ui.available_width() - 80.0),
                                    )
                                    .changed()
                                {
                                    self.state.environments[idx].name = name.clone();
                                    self.save_state();
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Delete").color(C_RED).size(11.0),
                                        )
                                        .fill(egui::Color32::TRANSPARENT)
                                        .stroke(egui::Stroke::new(1.0, C_RED)),
                                    )
                                    .clicked()
                                {
                                    delete_id = Some(self.state.environments[idx].id.clone());
                                }
                            });
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Variables  (use {{name}} in URL/headers/body)")
                                    .size(11.0)
                                    .color(C_MUTED),
                            );
                            ui.add_space(4.0);
                            let mut vars =
                                self.state.environments[idx].variables.clone();
                            let changed = render_kv_table(ui, "Variables", &mut vars, false);
                            if changed {
                                self.state.environments[idx].variables = vars;
                                self.save_state();
                            }
                        } else {
                            ui.label(
                                egui::RichText::new(
                                    "Select an environment on the left, or create a new one.",
                                )
                                .color(C_MUTED),
                            );
                        }
                    });
                });

                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Done").color(egui::Color32::WHITE).strong(),
                            )
                            .fill(C_ACCENT)
                            .min_size(egui::vec2(80.0, 28.0)),
                        )
                        .clicked()
                    {
                        close_modal = true;
                    }
                });
            });
        self.show_env_modal = open;

        if create_env {
            let new = Environment {
                id: Uuid::new_v4().to_string(),
                name: format!("Environment {}", self.state.environments.len() + 1),
                variables: vec![],
            };
            let new_id = new.id.clone();
            self.state.environments.push(new);
            self.selected_env_for_edit = Some(new_id);
            self.save_state();
        }
        if let Some(id) = delete_id {
            self.state.environments.retain(|e| e.id != id);
            if self.state.active_env_id.as_deref() == Some(&id) {
                self.state.active_env_id = None;
            }
            if self.selected_env_for_edit.as_deref() == Some(&id) {
                self.selected_env_for_edit = self.state.environments.first().map(|e| e.id.clone());
            }
            self.save_state();
        }
        if close_modal {
            self.show_env_modal = false;
        }
    }

    fn render_central(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(C_BG)
                    .inner_margin(egui::Margin::symmetric(0.0, 0.0)),
            )
            .show(ctx, |ui| {
            // Defensive floor: paint the full panel rect with C_BG BEFORE
            // any children render. Without this, if any child widget (scroll
            // track, code editor, etc.) leaves a sub-region with an
            // un-themed dark/transparent fill, the OS default (near-black)
            // bleeds through as a "black strip".
            ui.painter()
                .rect_filled(ui.max_rect(), egui::Rounding::ZERO, C_BG);
            self.render_tabs_bar(ui);
            if self.selected_request_id.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.25);
                        if let Some(tex) = &self.app_icon {
                            ui.add(
                                egui::Image::from_texture(tex)
                                    .fit_to_exact_size(egui::vec2(96.0, 96.0))
                                    .rounding(egui::Rounding::same(18.0)),
                            );
                        }
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new("Rusty Requester")
                                .size(22.0)
                                .strong()
                                .color(C_TEXT),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(
                                "Pick a request from the sidebar, or create a new one.",
                            )
                            .size(13.0)
                            .color(C_MUTED),
                        );
                    });
                });
                return;
            }

            egui::Frame::none()
                .inner_margin(egui::Margin {
                    // Tight left margin — sidebar ends here, we want central
                    // content to start right after without a visible dead
                    // zone. The background layer paints C_BG underneath so
                    // the tiny sliver between panels is invisible.
                    left: 10.0,
                    right: 16.0,
                    top: 10.0,
                    bottom: 8.0,
                })
                .show(ui, |ui| {
                    self.render_url_bar(ui);
                    ui.add_space(8.0);

                    // Vertical split with a draggable handle between
                    // the request-editor section (top) and the response
                    // section (bottom) — Postman-style.
                    let drag_handle_h: f32 = 5.0;
                    let min_req = 160.0_f32;
                    let min_resp = 140.0_f32;
                    let total_h = ui.available_height();
                    let max_req =
                        (total_h - drag_handle_h - min_resp).max(min_req);
                    self.request_split_px =
                        self.request_split_px.clamp(min_req, max_req);
                    let req_h = self.request_split_px;

                    // Top: request editor
                    ui.allocate_ui(
                        egui::vec2(ui.available_width(), req_h),
                        |ui| {
                            self.render_request_tabs(ui);
                        },
                    );

                    // Drag handle — 5px-tall invisible strip with a
                    // 1px accent line painted in the middle. Cursor
                    // changes to ResizeRow on hover.
                    let handle_resp = ui.allocate_response(
                        egui::vec2(ui.available_width(), drag_handle_h),
                        egui::Sense::drag(),
                    );
                    if handle_resp.hovered() || handle_resp.dragged() {
                        ui.output_mut(|o| {
                            o.cursor_icon = egui::CursorIcon::ResizeRow;
                        });
                    }
                    if ui.is_rect_visible(handle_resp.rect) {
                        let line_y = handle_resp.rect.center().y;
                        let line_color = if handle_resp.hovered()
                            || handle_resp.dragged()
                        {
                            C_ACCENT
                        } else {
                            C_BORDER
                        };
                        ui.painter().line_segment(
                            [
                                egui::pos2(handle_resp.rect.left() + 20.0, line_y),
                                egui::pos2(handle_resp.rect.right() - 20.0, line_y),
                            ],
                            egui::Stroke::new(1.0, line_color),
                        );
                    }
                    if handle_resp.dragged() {
                        self.request_split_px = (self.request_split_px
                            + handle_resp.drag_delta().y)
                            .clamp(min_req, max_req);
                    }

                    // Bottom: response section fills the rest.
                    self.render_response(ui);
                });
        });
    }

    fn render_tabs_bar(&mut self, ui: &mut egui::Ui) {
        // Always render the tabs bar — the "+" button lives here so users
        // can create a new unsaved draft request at any time.
        let bar_height = 38.0;
        egui::Frame::none()
            .fill(C_PANEL_DARK)
            .inner_margin(egui::Margin {
                left: 10.0,
                right: 10.0,
                top: 4.0,
                bottom: 0.0,
            })
            .show(ui, |ui| {
                ui.set_min_height(bar_height);
                ui.set_max_height(bar_height);

                let mut activate: Option<usize> = None;
                let mut close: Option<usize> = None;
                let mut close_others: Option<usize> = None;
                let mut close_all = false;
                let mut new_draft = false;
                let mut save_draft: Option<usize> = None;

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    egui::ScrollArea::horizontal()
                        .id_salt("tabs_bar_scroll")
                        .auto_shrink([false, false])
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                        )
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                let tabs_snapshot = self.state.open_tabs.clone();
                                for (i, tab) in tabs_snapshot.iter().enumerate() {
                                    let info = find_request_info(
                                        &self.state.folders,
                                        &self.state.drafts,
                                        &tab.folder_path,
                                        &tab.request_id,
                                    );
                                    let (method, name, url) = info.clone().unwrap_or((
                                        HttpMethod::GET,
                                        "(missing)".to_string(),
                                        String::new(),
                                    ));
                                    let is_active = self.selected_request_id.as_deref()
                                        == Some(tab.request_id.as_str());

                                    let action = render_single_tab(
                                        ui,
                                        i,
                                        &method,
                                        &name,
                                        &url,
                                        is_active,
                                        tab.is_draft(),
                                    );
                                    match action {
                                        TabAction::Activate => activate = Some(i),
                                        TabAction::Close => close = Some(i),
                                        TabAction::CloseOthers => close_others = Some(i),
                                        TabAction::CloseAll => close_all = true,
                                        TabAction::SaveDraft => save_draft = Some(i),
                                        TabAction::None => {}
                                    }
                                }

                                // "+" button — creates a new Untitled draft.
                                ui.add_space(2.0);
                                let (plus_rect, plus_resp) = ui.allocate_exact_size(
                                    egui::vec2(30.0, 28.0),
                                    egui::Sense::click(),
                                );
                                if ui.is_rect_visible(plus_rect) {
                                    let hovered = plus_resp.hovered();
                                    // Neutral hover — subtle elevated grey,
                                    // matching Postman's "new tab" button.
                                    if hovered {
                                        ui.painter().rect_filled(
                                            plus_rect,
                                            egui::Rounding::same(4.0),
                                            C_ELEVATED,
                                        );
                                    }
                                    let color = if hovered { C_TEXT } else { C_MUTED };
                                    ui.painter().text(
                                        plus_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "+",
                                        egui::FontId::new(18.0, egui::FontFamily::Proportional),
                                        color,
                                    );
                                }
                                if plus_resp
                                    .on_hover_text("New request (unsaved)")
                                    .clicked()
                                {
                                    new_draft = true;
                                }
                            });
                        });
                });

                if let Some(i) = activate {
                    if let Some(tab) = self.state.open_tabs.get(i).cloned() {
                        self.selected_folder_path = tab.folder_path;
                        self.selected_request_id = Some(tab.request_id);
                        self.load_request_for_editing();
                        self.response_text.clear();
                        self.response_status.clear();
                        self.response_time.clear();
                        self.response_headers.clear();
                    }
                }
                if let Some(i) = close {
                    self.close_tab(i);
                }
                if let Some(i) = close_others {
                    self.close_other_tabs(i);
                }
                if close_all {
                    self.close_all_tabs();
                }
                if new_draft {
                    self.new_draft_request();
                }
                if let Some(idx) = save_draft {
                    self.begin_save_draft(idx);
                }
            });
    }

    /// Open the "Save to folder" modal for the draft at tab index `idx`.
    fn begin_save_draft(&mut self, idx: usize) {
        let Some(tab) = self.state.open_tabs.get(idx).cloned() else {
            return;
        };
        if !tab.folder_path.is_empty() {
            return; // not actually a draft
        }
        let Some(draft) = self.state.drafts.iter().find(|d| d.id == tab.request_id).cloned()
        else {
            return;
        };
        self.save_draft_open = true;
        self.save_draft_tab_idx = Some(idx);
        self.save_draft_name = if draft.name.is_empty() || draft.name == "Untitled" {
            "New Request".to_string()
        } else {
            draft.name
        };
        // Default to the first top-level collection if any.
        self.save_draft_target_path = self
            .state
            .folders
            .first()
            .map(|f| vec![f.id.clone()])
            .unwrap_or_default();
        self.save_draft_search.clear();
        self.save_draft_new_folder_name = None;
    }

    fn render_save_draft_modal(&mut self, ctx: &egui::Context) {
        if !self.save_draft_open {
            return;
        }
        let mut open = self.save_draft_open;
        let mut do_save = false;
        let mut do_cancel = false;
        let mut create_folder: Option<(Vec<String>, String)> = None;

        egui::Window::new(
            egui::RichText::new("SAVE REQUEST").size(12.0).strong().color(C_MUTED),
        )
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(560.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_min_width(540.0);

                // Name
                ui.label(egui::RichText::new("Request name").size(11.0).color(C_MUTED));
                let name_resp = ui.add(
                    egui::TextEdit::singleline(&mut self.save_draft_name)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(10.0);

                // Breadcrumb
                let breadcrumb = self.save_draft_breadcrumb();
                ui.label(
                    egui::RichText::new(format!("Save to  {}", breadcrumb))
                        .size(12.0)
                        .color(C_TEXT),
                );
                ui.add_space(6.0);

                // Search
                ui.add(
                    egui::TextEdit::singleline(&mut self.save_draft_search)
                        .hint_text("Search for collection or folder")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.0);

                // Folder tree (scrollable)
                egui::Frame::none()
                    .fill(C_PANEL_DARK)
                    .stroke(egui::Stroke::new(1.0, C_BORDER))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(4.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::ScrollArea::vertical()
                            .id_salt("save_draft_tree")
                            .max_height(260.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let folders = self.state.folders.clone();
                                let query = self.save_draft_search.to_lowercase();
                                for f in &folders {
                                    Self::render_save_tree_row(
                                        ui,
                                        f,
                                        &mut vec![],
                                        &mut self.save_draft_target_path,
                                        &query,
                                    );
                                }
                            });
                    });

                ui.add_space(8.0);

                // New folder — either show the "+ New folder" button or,
                // if the user clicked it, an inline input with Create/Cancel.
                let mut cancel_new_folder = false;
                if let Some(name) = self.save_draft_new_folder_name.as_mut() {
                    let target_path_snapshot = self.save_draft_target_path.clone();
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(name)
                                .hint_text("New folder name")
                                .desired_width(260.0),
                        );
                        let enabled = !name.trim().is_empty();
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Button::new(
                                    egui::RichText::new("Create").color(egui::Color32::WHITE),
                                )
                                .fill(if enabled { C_ACCENT } else { C_ELEVATED })
                                .min_size(egui::vec2(72.0, 26.0)),
                            )
                            .clicked()
                        {
                            create_folder =
                                Some((target_path_snapshot.clone(), name.trim().to_string()));
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_new_folder = true;
                        }
                    });
                } else if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("+ New folder").size(12.0).color(C_ACCENT),
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE),
                    )
                    .clicked()
                {
                    self.save_draft_new_folder_name = Some(String::new());
                }
                if cancel_new_folder {
                    self.save_draft_new_folder_name = None;
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                // Save / Cancel
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        do_cancel = true;
                    }
                    let can_save = !self.save_draft_target_path.is_empty()
                        && !self.save_draft_name.trim().is_empty();
                    let save_btn = egui::Button::new(
                        egui::RichText::new("Save")
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .fill(if can_save { C_ACCENT } else { C_ELEVATED })
                    .min_size(egui::vec2(80.0, 28.0));
                    if ui.add_enabled(can_save, save_btn).clicked() {
                        do_save = true;
                    }
                });

                ui.input(|i| {
                    if i.key_pressed(egui::Key::Escape) {
                        do_cancel = true;
                    }
                });

                if !name_resp.has_focus()
                    && self.save_draft_tab_idx.is_some()
                    && !do_save
                    && !do_cancel
                    && self.save_draft_search.is_empty()
                    && self.save_draft_new_folder_name.is_none()
                {
                    name_resp.request_focus();
                }
            });
        self.save_draft_open = open;

        // Create the new folder outside the UI closure to avoid borrow
        // conflicts (we were holding `&self.state.folders` immutably while
        // iterating the tree).
        if let Some((parent_path, folder_name)) = create_folder {
            let new_id = Uuid::new_v4().to_string();
            let new_folder = Folder {
                id: new_id.clone(),
                name: folder_name,
                requests: vec![],
                subfolders: vec![],
            };
            let inserted = if parent_path.is_empty() {
                self.state.folders.push(new_folder);
                true
            } else if let Some(parent) = self.folder_at_path_mut(&parent_path) {
                parent.subfolders.push(new_folder);
                true
            } else {
                false
            };
            if inserted {
                let mut new_path = parent_path.clone();
                new_path.push(new_id);
                self.save_draft_target_path = new_path;
                self.save_draft_new_folder_name = None;
                self.save_state();
            }
        }

        if do_save {
            self.commit_save_draft();
        }
        if do_cancel || !self.save_draft_open {
            self.save_draft_open = false;
            self.save_draft_tab_idx = None;
            self.save_draft_target_path.clear();
            self.save_draft_name.clear();
            self.save_draft_search.clear();
            self.save_draft_new_folder_name = None;
        }
    }

    /// Human-readable breadcrumb of the currently-selected destination
    /// folder path, e.g. "Personal / API v2 / Search".
    fn save_draft_breadcrumb(&self) -> String {
        if self.save_draft_target_path.is_empty() {
            return "(select a folder)".to_string();
        }
        let mut parts: Vec<String> = Vec::new();
        let mut current: Option<&Folder> = None;
        for (depth, id) in self.save_draft_target_path.iter().enumerate() {
            let found = if depth == 0 {
                self.state.folders.iter().find(|f| &f.id == id)
            } else {
                current.and_then(|c| c.subfolders.iter().find(|f| &f.id == id))
            };
            match found {
                Some(f) => {
                    parts.push(f.name.clone());
                    current = Some(f);
                }
                None => break,
            }
        }
        parts.join(" / ")
    }

    /// Recursively render one row of the save-draft folder tree. `path`
    /// is the running path from root down to (not including) `folder`;
    /// `target` is the currently-selected destination that this row will
    /// update when clicked.
    fn render_save_tree_row(
        ui: &mut egui::Ui,
        folder: &Folder,
        path: &mut Vec<String>,
        target: &mut Vec<String>,
        query: &str,
    ) {
        let mut this_path = path.clone();
        this_path.push(folder.id.clone());

        // Filter: show row if self or any descendant matches the query.
        let matches_self = query.is_empty() || folder.name.to_lowercase().contains(query);
        let matches_descendant = !query.is_empty()
            && folder.subfolders.iter().any(|f| Self::subtree_has_match(f, query));
        if !matches_self && !matches_descendant {
            return;
        }

        let depth = path.len();
        let is_selected = *target == this_path;
        let row_h = 28.0;
        let indent = 14.0 * depth as f32 + 8.0;

        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_h),
            egui::Sense::click(),
        );
        if ui.is_rect_visible(rect) {
            let bg = if is_selected {
                C_ACCENT.linear_multiply(0.18)
            } else if resp.hovered() {
                C_ELEVATED
            } else {
                egui::Color32::TRANSPARENT
            };
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(4.0), bg);
            let icon_x = rect.left() + indent;
            let text_y = rect.center().y;
            // Folder icon — painter-drawn (two stacked rounded rects) so
            // we don't depend on unicode glyphs egui's bundled font
            // doesn't ship (e.g. `▸` rendered as a tofu square).
            let icon_body = egui::Rect::from_min_size(
                egui::pos2(icon_x, text_y - 4.0),
                egui::vec2(14.0, 10.0),
            );
            let icon_tab = egui::Rect::from_min_size(
                egui::pos2(icon_x, text_y - 7.0),
                egui::vec2(6.0, 3.5),
            );
            let icon_color = if is_selected { C_ACCENT } else { C_MUTED };
            ui.painter()
                .rect_filled(icon_tab, egui::Rounding::same(1.5), icon_color);
            ui.painter()
                .rect_filled(icon_body, egui::Rounding::same(2.0), icon_color);
            ui.painter().text(
                egui::pos2(icon_x + 20.0, text_y),
                egui::Align2::LEFT_CENTER,
                &folder.name,
                egui::FontId::proportional(13.0),
                C_TEXT,
            );
        }
        if resp.clicked() {
            *target = this_path.clone();
        }

        // Recurse into subfolders.
        if !folder.subfolders.is_empty() {
            path.push(folder.id.clone());
            for sub in &folder.subfolders {
                Self::render_save_tree_row(ui, sub, path, target, query);
            }
            path.pop();
        }
    }

    fn subtree_has_match(folder: &Folder, query: &str) -> bool {
        if folder.name.to_lowercase().contains(query) {
            return true;
        }
        folder.subfolders.iter().any(|f| Self::subtree_has_match(f, query))
    }

    /// Mutable lookup of a folder at an arbitrary path (top-level collection
    /// at path[0], nested subfolders after). Returns None if the path
    /// doesn't resolve.
    fn folder_at_path_mut(&mut self, path: &[String]) -> Option<&mut Folder> {
        if path.is_empty() {
            return None;
        }
        let mut cur = self.state.folders.iter_mut().find(|f| f.id == path[0])?;
        for id in &path[1..] {
            cur = cur.subfolders.iter_mut().find(|f| &f.id == id)?;
        }
        Some(cur)
    }

    /// Move the draft referenced by the modal into the selected folder path.
    fn commit_save_draft(&mut self) {
        let Some(idx) = self.save_draft_tab_idx else { return; };
        let target_path = self.save_draft_target_path.clone();
        if target_path.is_empty() {
            return;
        }
        let Some(tab) = self.state.open_tabs.get(idx).cloned() else { return; };
        if !tab.folder_path.is_empty() {
            return;
        }
        let draft_id = tab.request_id.clone();
        let draft_pos = self.state.drafts.iter().position(|d| d.id == draft_id);
        let Some(pos) = draft_pos else { return; };
        let mut req = self.state.drafts.remove(pos);
        req.name = self.save_draft_name.trim().to_string();

        let inserted = if let Some(folder) = self.folder_at_path_mut(&target_path) {
            folder.requests.push(req);
            true
        } else {
            false
        };
        if inserted {
            self.state.open_tabs[idx].folder_path = target_path.clone();
            self.selected_folder_path = target_path;
            self.selected_request_id = Some(draft_id);
            self.load_request_for_editing();
            self.save_state();
            self.show_toast("Request saved");
        }

        self.save_draft_open = false;
        self.save_draft_tab_idx = None;
        self.save_draft_target_path.clear();
        self.save_draft_name.clear();
        self.save_draft_search.clear();
        self.save_draft_new_folder_name = None;
    }

    /// Create an "Untitled" draft request, add it as a tab, and activate it.
    /// The draft lives in `state.drafts` (persisted) until the user either
    /// closes the tab or explicitly saves it to a folder via the tab's
    /// right-click menu.
    fn new_draft_request(&mut self) {
        let draft = Request {
            id: Uuid::new_v4().to_string(),
            name: "Untitled".to_string(),
            method: HttpMethod::GET,
            url: String::new(),
            query_params: vec![],
            headers: vec![],
            cookies: vec![],
            body: String::new(),
            body_ext: None,
            auth: Auth::None,
            extractors: vec![],
        };
        let id = draft.id.clone();
        self.state.drafts.push(draft);
        self.state.open_tabs.push(OpenTab {
            folder_path: vec![],
            request_id: id.clone(),
        });
        self.selected_folder_path = vec![];
        self.selected_request_id = Some(id);
        self.load_request_for_editing();
        self.response_text.clear();
        self.response_status.clear();
        self.response_time.clear();
        self.response_headers.clear();
        self.save_state();
    }

    fn render_snippet_panel(&mut self, ctx: &egui::Context) {
        if !self.show_snippet_panel {
            return;
        }
        let req = match self.get_current_request() {
            Some(r) => r,
            None => {
                self.show_snippet_panel = false;
                return;
            }
        };
        let snippet = render_snippet(&req, self.snippet_lang);
        let mut copy_clicked = false;
        let mut close_clicked = false;

        egui::SidePanel::right("snippet_panel")
            .resizable(true)
            .default_width(380.0)
            .width_range(280.0..=600.0)
            .frame(
                egui::Frame::none()
                    .fill(C_PANEL)
                    .inner_margin(egui::Margin::symmetric(10.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Code snippet")
                            .size(14.0)
                            .strong()
                            .color(C_TEXT),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if close_x_button(ui, "Close panel").clicked() {
                            close_clicked = true;
                        }
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("snippet_lang")
                        .selected_text(self.snippet_lang.label())
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for &lang in &[
                                SnippetLang::Curl,
                                SnippetLang::Python,
                                SnippetLang::JavaScript,
                                SnippetLang::HttpieShell,
                            ] {
                                ui.selectable_value(&mut self.snippet_lang, lang, lang.label());
                            }
                        });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Copy")
                                        .color(egui::Color32::WHITE)
                                        .strong(),
                                )
                                .fill(C_ACCENT)
                                .min_size(egui::vec2(70.0, 26.0)),
                            )
                            .clicked()
                        {
                            copy_clicked = true;
                        }
                    });
                });
                ui.add_space(8.0);
                egui::Frame::none()
                    .fill(C_PANEL_DARK)
                    .stroke(egui::Stroke::new(1.0, C_BORDER))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        let avail_h = ui.available_height();
                        egui::ScrollArea::vertical()
                            .id_salt("snippet_scroll")
                            .auto_shrink([false, false])
                            .max_height(avail_h)
                            .show(ui, |ui| {
                                let mut text = snippet.clone();
                                let lang = self.snippet_lang;
                                let mut layouter =
                                    move |ui: &egui::Ui, s: &str, wrap_width: f32| {
                                        let mut job =
                                            build_snippet_layout_job(s, lang, wrap_width);
                                        job.wrap.max_width = wrap_width;
                                        ui.fonts(|f| f.layout_job(job))
                                    };
                                ui.add(
                                    egui::TextEdit::multiline(&mut text)
                                        .code_editor()
                                        .interactive(false)
                                        .desired_width(f32::INFINITY)
                                        .layouter(&mut layouter),
                                );
                            });
                    });
            });

        if copy_clicked {
            ctx.output_mut(|o| o.copied_text = snippet);
            self.show_toast("Snippet copied");
        }
        if close_clicked {
            self.show_snippet_panel = false;
        }
    }

    fn render_url_bar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(C_PANEL)
            .inner_margin(12.0)
            .rounding(10.0)
            .stroke(egui::Stroke::new(1.0, C_BORDER))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mc = method_color(&self.editing_method);
                    egui::ComboBox::from_id_salt("method_combo")
                        .selected_text(
                            egui::RichText::new(format!("{}", self.editing_method))
                                .color(mc)
                                .strong()
                                .size(13.0),
                        )
                        .width(90.0)
                        .show_ui(ui, |ui| {
                            for method in [
                                HttpMethod::GET,
                                HttpMethod::POST,
                                HttpMethod::PUT,
                                HttpMethod::DELETE,
                                HttpMethod::PATCH,
                                HttpMethod::HEAD,
                                HttpMethod::OPTIONS,
                            ] {
                                let mc2 = method_color(&method);
                                if ui
                                    .selectable_value(
                                        &mut self.editing_method,
                                        method.clone(),
                                        egui::RichText::new(format!("{}", method))
                                            .color(mc2)
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    let m = self.editing_method.clone();
                                    self.update_current_request(|req| req.method = m);
                                }
                            }
                        });

                    // Reserve space for Send + Code buttons (~180 px)
                    let btn_space = 180.0;
                    let avail = (ui.available_width() - btn_space).max(200.0);
                    let url_edit = ui.add(
                        egui::TextEdit::singleline(&mut self.editing_url)
                            .desired_width(avail)
                            .hint_text("https://api.example.com/endpoint  (or paste a cURL command)")
                            .font(egui::TextStyle::Monospace),
                    );
                    if url_edit.changed() {
                        let trimmed = self.editing_url.trim_start();
                        let looks_like_curl = trimmed.starts_with("curl ")
                            || trimmed.starts_with("curl\t")
                            || trimmed.starts_with("curl\n")
                            || trimmed == "curl";
                        if looks_like_curl && trimmed.len() > 5 {
                            match curl::parse_curl(&self.editing_url) {
                                Ok(parsed) => {
                                    self.editing_method = parsed.method;
                                    self.editing_url = parsed.url;
                                    self.editing_params = parsed.query_params;
                                    self.editing_headers = parsed.headers;
                                    self.editing_cookies = parsed.cookies;
                                    self.editing_body = parsed.body;
                                    self.editing_auth = parsed.auth;
                                    self.commit_editing();
                                    self.show_toast("Imported cURL into request");
                                }
                                Err(_) => {
                                    let url = self.editing_url.clone();
                                    self.update_current_request(|req| req.url = url);
                                }
                            }
                        } else {
                            let url = self.editing_url.clone();
                            self.update_current_request(|req| req.url = url);
                        }
                    }

                    let send_pressed = url_edit.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    let send_btn = egui::Button::new(
                        egui::RichText::new(if self.is_loading { "Sending..." } else { "Send" })
                            .size(13.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(C_PURPLE)
                    .min_size(egui::vec2(80.0, 28.0));

                    let send_click = ui
                        .add_enabled(!self.is_loading, send_btn)
                        .on_hover_text("Send (⌘/Ctrl + Enter)")
                        .clicked();

                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("</> Code").size(12.0))
                                .fill(C_BORDER)
                                .min_size(egui::vec2(74.0, 28.0)),
                        )
                        .on_hover_text("Toggle code-snippet panel")
                        .clicked()
                    {
                        self.commit_editing();
                        self.show_snippet_panel = !self.show_snippet_panel;
                    }

                    if send_click || send_pressed {
                        self.send_request();
                    }
                });
            });
    }

    fn render_request_tabs(&mut self, ui: &mut egui::Ui) {
        let params_label = if self.editing_params.is_empty() {
            "Params".to_string()
        } else {
            format!("Params ({})", self.editing_params.len())
        };
        let headers_label = if self.editing_headers.is_empty() {
            "Headers".to_string()
        } else {
            format!("Headers ({})", self.editing_headers.len())
        };
        let cookies_label = if self.editing_cookies.is_empty() {
            "Cookies".to_string()
        } else {
            format!("Cookies ({})", self.editing_cookies.len())
        };
        let body_label = if self.editing_body.is_empty() {
            "Body".to_string()
        } else {
            "Body •".to_string()
        };
        let auth_label = match self.editing_auth {
            Auth::None => "Auth".to_string(),
            Auth::Bearer { .. } => "Auth (Bearer)".to_string(),
            Auth::Basic { .. } => "Auth (Basic)".to_string(),
        };
        let active_extractors = self
            .editing_extractors
            .iter()
            .filter(|e| e.enabled && !e.variable.trim().is_empty())
            .count();
        let tests_label = if active_extractors == 0 {
            "Tests".to_string()
        } else {
            format!("Tests ({})", active_extractors)
        };

        ui.horizontal(|ui| {
            tab_button(ui, &mut self.request_tab, RequestTab::Params, &params_label);
            tab_button(ui, &mut self.request_tab, RequestTab::Headers, &headers_label);
            tab_button(ui, &mut self.request_tab, RequestTab::Cookies, &cookies_label);
            tab_button(ui, &mut self.request_tab, RequestTab::Body, &body_label);
            tab_button(ui, &mut self.request_tab, RequestTab::Auth, &auth_label);
            tab_button(ui, &mut self.request_tab, RequestTab::Tests, &tests_label);
        });

        // Use whatever vertical space we've been given by the caller
        // (render_central allocates the request section with a fixed
        // height so the user can drag-resize the split).
        egui::Frame::none()
            .fill(C_PANEL)
            .inner_margin(12.0)
            .rounding(10.0)
            .stroke(egui::Stroke::new(1.0, C_BORDER))
            .show(ui, |ui| {
                let avail = ui.available_height();
                egui::ScrollArea::vertical()
                    .id_salt("request_tab_scroll")
                    .max_height(avail)
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.request_tab {
                        RequestTab::Params => self.render_params_tab(ui),
                        RequestTab::Headers => self.render_headers_tab(ui),
                        RequestTab::Cookies => self.render_cookies_tab(ui),
                        RequestTab::Body => self.render_body_tab(ui),
                        RequestTab::Auth => self.render_auth_tab(ui),
                        RequestTab::Tests => self.render_tests_tab(ui),
                    });
            });
    }

    fn render_params_tab(&mut self, ui: &mut egui::Ui) {
        let changed = render_kv_table(ui, "Query Params", &mut self.editing_params, true);
        if changed {
            let params = self.editing_params.clone();
            self.update_current_request(|r| r.query_params = params);
        }
    }

    fn render_headers_tab(&mut self, ui: &mut egui::Ui) {
        let changed = render_kv_table(ui, "Headers", &mut self.editing_headers, true);
        if changed {
            let headers = self.editing_headers.clone();
            self.update_current_request(|r| r.headers = headers);
        }
    }

    fn render_body_tab(&mut self, ui: &mut egui::Ui) {
        let current_mode = match &self.editing_body_ext {
            None => BodyMode::Raw,
            Some(BodyExt::FormUrlEncoded { .. }) => BodyMode::FormUrlEncoded,
            Some(BodyExt::MultipartForm { .. }) => BodyMode::MultipartForm,
            Some(BodyExt::GraphQL { .. }) => BodyMode::GraphQL,
        };
        let mut new_mode = current_mode;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Body type").size(11.0).color(C_MUTED));
            for &m in &[
                BodyMode::Raw,
                BodyMode::FormUrlEncoded,
                BodyMode::MultipartForm,
                BodyMode::GraphQL,
            ] {
                ui.selectable_value(&mut new_mode, m, m.label());
            }
        });
        if new_mode != current_mode {
            self.editing_body_ext = match new_mode {
                BodyMode::Raw => None,
                BodyMode::FormUrlEncoded => Some(BodyExt::FormUrlEncoded { fields: vec![] }),
                BodyMode::MultipartForm => Some(BodyExt::MultipartForm { fields: vec![] }),
                BodyMode::GraphQL => Some(BodyExt::GraphQL {
                    variables: String::new(),
                }),
            };
            let body_ext = self.editing_body_ext.clone();
            self.update_current_request(|r| r.body_ext = body_ext);
        }
        ui.add_space(8.0);

        match new_mode {
            BodyMode::Raw => self.render_body_raw(ui),
            BodyMode::FormUrlEncoded => self.render_body_form(ui, false),
            BodyMode::MultipartForm => self.render_body_form(ui, true),
            BodyMode::GraphQL => self.render_body_graphql(ui),
        }
    }

    fn render_body_raw(&mut self, ui: &mut egui::Ui) {
        let mut prettify = false;
        let mut minify = false;
        ui.horizontal(|ui| {
            if ui
                .small_button(egui::RichText::new("Prettify JSON").size(11.0))
                .on_hover_text("Format body as pretty JSON")
                .clicked()
            {
                prettify = true;
            }
            if ui
                .small_button(egui::RichText::new("Minify").size(11.0))
                .on_hover_text("Collapse JSON to one line")
                .clicked()
            {
                minify = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let size_label = if self.editing_body.is_empty() {
                    "empty".to_string()
                } else {
                    format!("{} bytes", self.editing_body.len())
                };
                ui.label(
                    egui::RichText::new(size_label)
                        .size(11.0)
                        .color(C_MUTED),
                );
            });
        });
        if prettify {
            match serde_json::from_str::<serde_json::Value>(&self.editing_body) {
                Ok(v) => match serde_json::to_string_pretty(&v) {
                    Ok(s) => {
                        self.editing_body = s;
                        let body = self.editing_body.clone();
                        self.update_current_request(|r| r.body = body);
                        self.show_toast("Body prettified");
                    }
                    Err(e) => self.show_toast(format!("Prettify failed: {}", e)),
                },
                Err(_) => self.show_toast("Body is not valid JSON"),
            }
        }
        if minify {
            match serde_json::from_str::<serde_json::Value>(&self.editing_body) {
                Ok(v) => match serde_json::to_string(&v) {
                    Ok(s) => {
                        self.editing_body = s;
                        let body = self.editing_body.clone();
                        self.update_current_request(|r| r.body = body);
                        self.show_toast("Body minified");
                    }
                    Err(e) => self.show_toast(format!("Minify failed: {}", e)),
                },
                Err(_) => self.show_toast("Body is not valid JSON"),
            }
        }
        ui.add_space(4.0);
        if ui
            .add_sized(
                [ui.available_width(), ui.available_height() - 4.0],
                egui::TextEdit::multiline(&mut self.editing_body)
                    .code_editor()
                    .hint_text("Request body (JSON, text, ...)")
                    .font(egui::TextStyle::Monospace),
            )
            .changed()
        {
            let body = self.editing_body.clone();
            self.update_current_request(|r| r.body = body);
        }
    }

    fn render_body_form(&mut self, ui: &mut egui::Ui, multipart: bool) {
        ui.label(
            egui::RichText::new(if multipart {
                "multipart/form-data fields (text only)"
            } else {
                "x-www-form-urlencoded fields"
            })
            .size(11.0)
            .color(C_MUTED),
        );
        ui.add_space(4.0);
        // Take ownership of the inner Vec<KvRow>, render the table, write back.
        let mut fields = match &self.editing_body_ext {
            Some(BodyExt::FormUrlEncoded { fields }) | Some(BodyExt::MultipartForm { fields }) => {
                fields.clone()
            }
            _ => vec![],
        };
        let changed = render_kv_table(ui, "Fields", &mut fields, false);
        if changed {
            let new_ext = if multipart {
                BodyExt::MultipartForm { fields }
            } else {
                BodyExt::FormUrlEncoded { fields }
            };
            self.editing_body_ext = Some(new_ext);
            let body_ext = self.editing_body_ext.clone();
            self.update_current_request(|r| r.body_ext = body_ext);
        }
    }

    fn render_body_graphql(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Sent as JSON `{ query, variables }` with application/json.")
                .size(11.0)
                .color(C_MUTED),
        );
        ui.add_space(4.0);

        let avail_h = ui.available_height();
        let query_h = (avail_h * 0.6).max(80.0);
        let vars_h = (avail_h - query_h - 30.0).max(60.0);

        ui.label(egui::RichText::new("Query").size(11.0).strong().color(C_TEXT));
        if ui
            .add_sized(
                [ui.available_width(), query_h],
                egui::TextEdit::multiline(&mut self.editing_body)
                    .code_editor()
                    .hint_text("query MyQuery { ... }")
                    .font(egui::TextStyle::Monospace),
            )
            .changed()
        {
            let body = self.editing_body.clone();
            self.update_current_request(|r| r.body = body);
        }

        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Variables (JSON)")
                .size(11.0)
                .strong()
                .color(C_TEXT),
        );
        let mut vars = match &self.editing_body_ext {
            Some(BodyExt::GraphQL { variables }) => variables.clone(),
            _ => String::new(),
        };
        if ui
            .add_sized(
                [ui.available_width(), vars_h],
                egui::TextEdit::multiline(&mut vars)
                    .code_editor()
                    .hint_text("{ \"id\": 123 }")
                    .font(egui::TextStyle::Monospace),
            )
            .changed()
        {
            self.editing_body_ext = Some(BodyExt::GraphQL { variables: vars });
            let body_ext = self.editing_body_ext.clone();
            self.update_current_request(|r| r.body_ext = body_ext);
        }
    }

    fn render_cookies_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Cookies are merged into a Cookie header on send.")
                .size(11.0)
                .color(C_MUTED),
        );
        ui.add_space(4.0);
        let changed = render_kv_table(ui, "Cookies", &mut self.editing_cookies, false);
        if changed {
            let cookies = self.editing_cookies.clone();
            self.update_current_request(|r| r.cookies = cookies);
        }
    }

    fn render_auth_tab(&mut self, ui: &mut egui::Ui) {
        let mut kind = AuthKind::from(&self.editing_auth);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Type").color(C_ACCENT));
            egui::ComboBox::from_id_salt("auth_kind")
                .selected_text(match kind {
                    AuthKind::None => "No Auth",
                    AuthKind::Bearer => "Bearer Token",
                    AuthKind::Basic => "Basic Auth",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut kind, AuthKind::None, "No Auth");
                    ui.selectable_value(&mut kind, AuthKind::Bearer, "Bearer Token");
                    ui.selectable_value(&mut kind, AuthKind::Basic, "Basic Auth");
                });
        });

        let current_kind = AuthKind::from(&self.editing_auth);
        if kind != current_kind {
            self.editing_auth = match kind {
                AuthKind::None => Auth::None,
                AuthKind::Bearer => Auth::Bearer {
                    token: match &self.editing_auth {
                        Auth::Bearer { token } => token.clone(),
                        _ => String::new(),
                    },
                },
                AuthKind::Basic => match &self.editing_auth {
                    Auth::Basic { username, password } => Auth::Basic {
                        username: username.clone(),
                        password: password.clone(),
                    },
                    _ => Auth::Basic {
                        username: String::new(),
                        password: String::new(),
                    },
                },
            };
            let auth = self.editing_auth.clone();
            self.update_current_request(|r| r.auth = auth);
        }

        ui.add_space(8.0);
        let mut changed = false;
        match &mut self.editing_auth {
            Auth::None => {
                ui.label(
                    egui::RichText::new("No authentication will be sent.")
                        .color(C_MUTED)
                        .size(12.0),
                );
            }
            Auth::Bearer { token } => {
                ui.label(egui::RichText::new("Token").color(C_ACCENT));
                if ui
                    .add(
                        egui::TextEdit::singleline(token)
                            .desired_width(ui.available_width())
                            .password(false)
                            .hint_text("eyJhbGciOi..."),
                    )
                    .changed()
                {
                    changed = true;
                }
            }
            Auth::Basic { username, password } => {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Username").color(C_ACCENT));
                    if ui
                        .add(
                            egui::TextEdit::singleline(username)
                                .desired_width(ui.available_width() - 100.0),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Password").color(C_ACCENT));
                    if ui
                        .add(
                            egui::TextEdit::singleline(password)
                                .desired_width(ui.available_width() - 100.0)
                                .password(true),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
            }
        }
        if changed {
            let auth = self.editing_auth.clone();
            self.update_current_request(|r| r.auth = auth);
        }
    }

    /// Post-response extractors — rules that pull a value out of the
    /// response (JSON-path into body, header name, or status code) and
    /// write it into the currently-active environment as a variable.
    /// Next request can reference it with `{{name}}`.
    fn render_tests_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Extract values from the response into environment variables.")
                .size(12.0)
                .color(C_MUTED),
        );
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(
                "Body path: dot + bracket syntax, e.g. `data.token` or `items[0].id`.",
            )
            .size(11.5)
            .color(C_MUTED),
        );
        ui.add_space(8.0);

        // Ensure a trailing blank ghost row so users can just start typing.
        if self
            .editing_extractors
            .last()
            .map(|e| !e.variable.is_empty() || !e.expression.is_empty())
            .unwrap_or(true)
        {
            self.editing_extractors.push(ResponseExtractor {
                enabled: true,
                variable: String::new(),
                source: ExtractorSource::Body,
                expression: String::new(),
            });
        }

        let avail = ui.available_width();
        let cb_w = 22.0;
        let var_w = 180.0;
        let src_w = 110.0;
        let del_w = 22.0;
        let pad = 6.0;
        let expr_w = (avail - cb_w - var_w - src_w - del_w - pad * 4.0).max(180.0);
        let row_h = 24.0;

        ui.horizontal(|ui| {
            ui.add_space(cb_w + pad);
            ui.label(egui::RichText::new("VARIABLE").size(10.0).color(C_MUTED));
            ui.add_space(var_w - 46.0);
            ui.label(egui::RichText::new("SOURCE").size(10.0).color(C_MUTED));
            ui.add_space(src_w - 36.0);
            ui.label(egui::RichText::new("EXPRESSION").size(10.0).color(C_MUTED));
        });
        ui.add_space(2.0);
        ui.separator();
        ui.add_space(4.0);

        let mut changed = false;
        let mut to_remove: Option<usize> = None;
        let row_count = self.editing_extractors.len();
        let id_salt = egui::Id::new("extractors_table");
        for (i, ex) in self.editing_extractors.iter_mut().enumerate() {
            let is_ghost =
                i == row_count - 1 && ex.variable.is_empty() && ex.expression.is_empty();
            ui.horizontal(|ui| {
                if is_ghost {
                    ui.add_space(cb_w);
                } else if ui.add(egui::Checkbox::new(&mut ex.enabled, "")).changed() {
                    changed = true;
                }
                ui.add_space(pad);

                let color = if ex.enabled { C_TEXT } else { C_MUTED };
                if ui
                    .add_sized(
                        [var_w, row_h],
                        egui::TextEdit::singleline(&mut ex.variable)
                            .id(id_salt.with((i, "var")))
                            .hint_text(if is_ghost { "var_name" } else { "" })
                            .text_color(color),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.add_space(pad);

                egui::ComboBox::from_id_salt(id_salt.with((i, "src")))
                    .selected_text(ex.source.label())
                    .width(src_w)
                    .show_ui(ui, |ui| {
                        for s in [
                            ExtractorSource::Body,
                            ExtractorSource::Header,
                            ExtractorSource::Status,
                        ] {
                            if ui
                                .selectable_label(ex.source == s, s.label())
                                .clicked()
                            {
                                if ex.source != s {
                                    ex.source = s;
                                    changed = true;
                                }
                            }
                        }
                    });
                ui.add_space(pad);

                let hint = match ex.source {
                    ExtractorSource::Body => "data.token",
                    ExtractorSource::Header => "X-Request-Id",
                    ExtractorSource::Status => "(ignored)",
                };
                let expr_enabled = !matches!(ex.source, ExtractorSource::Status);
                if ui
                    .add_sized(
                        [expr_w, row_h],
                        egui::TextEdit::singleline(&mut ex.expression)
                            .id(id_salt.with((i, "expr")))
                            .hint_text(if expr_enabled && is_ghost { hint } else { "" })
                            .interactive(expr_enabled)
                            .text_color(color),
                    )
                    .changed()
                {
                    changed = true;
                }

                ui.add_space(pad);
                if is_ghost {
                    ui.add_space(del_w);
                } else if close_x_button(ui, "Remove extractor").clicked() {
                    to_remove = Some(i);
                }
            });
            ui.add_space(2.0);
        }

        if let Some(i) = to_remove {
            self.editing_extractors.remove(i);
            changed = true;
        }
        if changed {
            let ext = self.editing_extractors.clone();
            self.update_current_request(|r| r.extractors = ext);
        }

        if self.state.active_env_id.is_none() {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(
                    "No active environment — extracted values will be discarded. \
                     Pick or create an environment from the sidebar.",
                )
                .size(11.5)
                .color(C_ORANGE),
            );
        }
    }

    fn render_response(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Response")
                    .size(15.0)
                    .strong()
                    .color(C_TEXT),
            );
            ui.add_space(12.0);

            let bullet = || {
                egui::RichText::new("•")
                    .color(C_MUTED.linear_multiply(0.7))
                    .size(12.0)
            };
            let info_text = |s: String| egui::RichText::new(s).color(C_MUTED).size(12.0);

            if !self.response_status.is_empty() {
                // Status badge — colored pill, e.g. "200 OK" or "404 Not Found"
                let sc = status_color(&self.response_status);
                egui::Frame::none()
                    .fill(sc.linear_multiply(0.18))
                    .rounding(egui::Rounding::same(5.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&self.response_status)
                                .color(sc)
                                .strong()
                                .size(12.0),
                        );
                    });
            }
            if !self.response_time.is_empty() {
                ui.add_space(4.0);
                ui.label(bullet());
                ui.add_space(2.0);
                let time_resp = ui.label(info_text(self.response_time.clone()));
                let prep = self.response_prepare_ms;
                let wait = self.response_waiting_ms;
                let dl = self.response_download_ms;
                let total = self.response_total_ms;
                time_resp.on_hover_ui(move |ui| {
                    render_time_breakdown(ui, prep, wait, dl, total);
                });
            }
            let total_resp_bytes =
                self.response_headers_bytes + self.response_body_bytes;
            if total_resp_bytes > 0 {
                ui.add_space(4.0);
                ui.label(bullet());
                ui.add_space(2.0);
                let size_resp = ui.label(info_text(format_bytes(total_resp_bytes)));
                // Hover popover with breakdown — response size + request size,
                // mirroring Postman's globe-icon tooltip.
                let resp_h_bytes = self.response_headers_bytes;
                let resp_b_bytes = self.response_body_bytes;
                let req_h_bytes = self.request_headers_bytes;
                let req_b_bytes = self.request_body_bytes;
                size_resp.on_hover_ui(move |ui| {
                    render_size_breakdown(
                        ui,
                        resp_h_bytes,
                        resp_b_bytes,
                        req_h_bytes,
                        req_b_bytes,
                    );
                });
            }
            if self.response_status.is_empty() {
                ui.label(
                    egui::RichText::new("— no response yet")
                        .color(C_MUTED)
                        .size(12.0)
                        .italics(),
                );
            }
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let body_label = "Body".to_string();
            let headers_label = if self.response_headers.is_empty() {
                "Headers".to_string()
            } else {
                format!("Headers ({})", self.response_headers.len())
            };
            tab_button(ui, &mut self.response_tab, ResponseTab::Body, &body_label);
            tab_button(
                ui,
                &mut self.response_tab,
                ResponseTab::Headers,
                &headers_label,
            );
        });
        ui.add_space(4.0);

        // Fill all remaining vertical space.
        let remaining_height = ui.available_height().max(120.0);

        // Loading state — show a centered spinner + timer while the
        // in-flight request is pending. This replaces both the empty
        // state and the stale previous response while a send is in
        // progress, so the UI is obviously "working".
        if self.is_loading {
            let full_w = ui.available_width();
            let full_h = ui.available_height().max(120.0);
            egui::Frame::none()
                .fill(C_PANEL_DARK)
                .inner_margin(16.0)
                .rounding(10.0)
                .stroke(egui::Stroke::new(1.0, C_BORDER))
                .show(ui, |ui| {
                    let margin = 32.0;
                    ui.set_width(full_w - margin);
                    ui.set_min_height(full_h - margin);
                    let inner_h = ui.available_height();
                    ui.vertical_centered(|ui| {
                        ui.add_space((inner_h * 0.30).max(24.0));
                        ui.add(egui::Spinner::new().size(26.0).color(C_ACCENT));
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("Sending request…")
                                .size(13.0)
                                .color(C_TEXT),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Press Esc to stay — response will appear here")
                                .size(11.5)
                                .color(C_MUTED),
                        );
                    });
                });
            // Keep the frame animating while we wait.
            ui.ctx().request_repaint();
            return;
        }

        // Postman-style empty state — if we haven't sent a request yet,
        // show a few hint lines instead of an empty body frame.
        if self.response_status.is_empty() {
            let full_w = ui.available_width();
            let full_h = ui.available_height().max(120.0);
            egui::Frame::none()
                .fill(C_PANEL_DARK)
                .inner_margin(16.0)
                .rounding(10.0)
                .stroke(egui::Stroke::new(1.0, C_BORDER))
                .show(ui, |ui| {
                    // Force the Frame to expand to the full size of the
                    // response panel. Without these, the Frame shrinks to
                    // the hint text, leaving dead grey space on the right
                    // and below (Frame auto-sizes to content).
                    let margin = 32.0; // 16.0 × 2
                    ui.set_width(full_w - margin);
                    ui.set_min_height(full_h - margin);
                    let inner_h = ui.available_height();
                    ui.vertical_centered(|ui| {
                        ui.add_space((inner_h * 0.32).max(24.0));
                        ui.label(
                            egui::RichText::new("No response yet")
                                .size(15.0)
                                .color(C_TEXT),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Send a request to see the response here.")
                                .size(12.5)
                                .color(C_MUTED),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Shortcut: Cmd / Ctrl + Enter")
                                .size(12.0)
                                .color(C_MUTED),
                        );
                    });
                });
            return;
        }

        let full_w = ui.available_width();
        let full_h = ui.available_height().max(120.0);
        egui::Frame::none()
            .fill(C_PANEL_DARK)
            .inner_margin(12.0)
            .rounding(10.0)
            .stroke(egui::Stroke::new(1.0, C_BORDER))
            .show(ui, |ui| {
                // Force Frame to fill the full response panel rather than
                // auto-sizing to content (which leaves right/bottom gaps).
                let margin = 24.0; // 12.0 × 2
                ui.set_width(full_w - margin);
                ui.set_min_height(full_h - margin);
                egui::ScrollArea::vertical()
                    .id_salt("response_scroll")
                    .max_height(remaining_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.response_tab {
                        ResponseTab::Body => {
                            // Three view modes (like Postman's body toolbar):
                            //   • JSON — syntax-highlighted pretty code
                            //   • Tree — collapsible tree view
                            //   • Raw  — verbatim text
                            // JSON/Tree fall back to Raw when the body isn't
                            // valid JSON.
                            let parsed: Option<serde_json::Value> =
                                serde_json::from_str(&self.response_text).ok();
                            let is_json = parsed.is_some();
                            let mut copy_clicked = false;
                            let mut toggle_search = false;
                            ui.horizontal(|ui| {
                                let mut view = self.body_view;
                                body_view_pill(ui, &mut view, BodyView::Json, "JSON");
                                body_view_pill(ui, &mut view, BodyView::Tree, "Tree");
                                body_view_pill(ui, &mut view, BodyView::Raw, "Raw");
                                self.body_view = view;
                                if matches!(self.body_view, BodyView::Tree) && is_json {
                                    ui.add_space(8.0);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.body_tree_filter)
                                            .hint_text("Filter keys / values")
                                            .desired_width(200.0),
                                    );
                                }
                                // Right-side icon buttons — search + copy.
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if icon_button(ui, "Copy response body", paint_copy_icon)
                                            .clicked()
                                        {
                                            copy_clicked = true;
                                        }
                                        ui.add_space(2.0);
                                        if icon_button(ui, "Search in body", paint_search_icon)
                                            .clicked()
                                        {
                                            toggle_search = true;
                                        }
                                    },
                                );
                            });
                            if toggle_search {
                                self.body_search_visible = !self.body_search_visible;
                                if !self.body_search_visible {
                                    self.body_search_query.clear();
                                }
                            }
                            if copy_clicked {
                                ui.ctx()
                                    .output_mut(|o| o.copied_text = self.response_text.clone());
                                self.show_toast("Copied response body");
                            }
                            if self.body_search_visible {
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.body_search_query)
                                            .hint_text("Find in body…")
                                            .desired_width(ui.available_width() - 40.0),
                                    );
                                    if icon_button(ui, "Close search", |p, c, col| {
                                        paint_x(p, c, 5.0, col, 1.5)
                                    })
                                    .clicked()
                                    {
                                        self.body_search_visible = false;
                                        self.body_search_query.clear();
                                    }
                                });
                            }
                            ui.add_space(6.0);

                            let effective_view = if !is_json
                                && !matches!(self.body_view, BodyView::Raw)
                            {
                                BodyView::Raw
                            } else {
                                self.body_view
                            };

                            // Make the editor background extend to the
                            // outer response panel — no inner border, so
                            // short payloads don't look like a small
                            // floating card with empty space below.
                            match effective_view {
                                BodyView::Json => {
                                    // `&mut &str` gives egui a read-only
                                    // buffer — the user can click, position
                                    // the caret, select text, scroll and
                                    // copy (⌘C) just like in Postman, but
                                    // edits are dropped because the buffer
                                    // can't be mutated.
                                    let mut buf: &str = &self.response_text;
                                    let search = self.body_search_query.clone();
                                    let mut layouter = move |ui: &egui::Ui,
                                                             s: &str,
                                                             wrap_width: f32| {
                                        let mut job =
                                            build_json_layout_job_with_search(s, &search);
                                        job.wrap.max_width = wrap_width;
                                        ui.fonts(|f| f.layout_job(job))
                                    };
                                    ui.add_sized(
                                        egui::vec2(
                                            ui.available_width(),
                                            ui.available_height(),
                                        ),
                                        egui::TextEdit::multiline(&mut buf)
                                            .frame(false)
                                            .desired_width(f32::INFINITY)
                                            .font(egui::TextStyle::Monospace)
                                            .layouter(&mut layouter),
                                    );
                                }
                                BodyView::Tree => {
                                    if let Some(v) = parsed {
                                        let filter = self.body_tree_filter.to_lowercase();
                                        render_json_tree(
                                            ui,
                                            egui::Id::new("resp_tree_root"),
                                            "",
                                            &v,
                                            &filter,
                                            0,
                                        );
                                    }
                                }
                                BodyView::Raw => {
                                    ui.add_sized(
                                        egui::vec2(
                                            ui.available_width(),
                                            ui.available_height(),
                                        ),
                                        egui::TextEdit::multiline(&mut self.response_text.as_str())
                                            .frame(false)
                                            .desired_width(f32::INFINITY)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                }
                            }
                        }
                        ResponseTab::Headers => {
                            if self.response_headers.is_empty() {
                                ui.label(
                                    egui::RichText::new("No response headers yet.")
                                        .color(C_MUTED),
                                );
                            } else {
                                egui::Grid::new("resp_headers_grid")
                                    .num_columns(2)
                                    .spacing([20.0, 4.0])
                                    .striped(true)
                                    .show(ui, |ui| {
                                        for (k, v) in &self.response_headers {
                                            ui.label(
                                                egui::RichText::new(k).color(C_ACCENT).strong(),
                                            );
                                            ui.label(
                                                egui::RichText::new(v)
                                                    .font(egui::FontId::monospace(12.0)),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            }
                        }
                    });
            });
    }

    fn render_paste_modal(&mut self, ctx: &egui::Context) {
        if !self.show_paste_modal {
            return;
        }
        let mut open = self.show_paste_modal;
        let mut do_import: Option<Request> = None;
        egui::Window::new("Import from cURL")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .default_height(340.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Paste a cURL command below and click Import.")
                        .color(C_MUTED)
                        .size(12.0),
                );
                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.paste_curl_text)
                        .code_editor()
                        .desired_rows(10)
                        .desired_width(f32::INFINITY)
                        .hint_text("curl -X POST 'https://api.example.com' -H 'Content-Type: application/json' -d '{\"k\":\"v\"}'"),
                );
                if !self.paste_error.is_empty() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&self.paste_error).color(C_RED));
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Import")
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .fill(C_PURPLE)
                            .min_size(egui::vec2(90.0, 28.0)),
                        )
                        .clicked()
                    {
                        match curl::parse_curl(&self.paste_curl_text) {
                            Ok(req) => do_import = Some(req),
                            Err(e) => self.paste_error = e,
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_paste_modal = false;
                    }
                });
            });
        self.show_paste_modal = open;

        if let Some(mut req) = do_import {
            // Ensure we have a folder to put it in
            if self.state.folders.is_empty() {
                self.state.folders.push(Folder {
                    id: Uuid::new_v4().to_string(),
                    name: "Imported".to_string(),
                    requests: vec![],
                    subfolders: vec![],
                });
            }

            let target_path = if !self.selected_folder_path.is_empty() {
                self.selected_folder_path.clone()
            } else {
                vec![self.state.folders[0].id.clone()]
            };
            self.selected_folder_path = target_path;

            // Name based on method + host/path
            if req.name == "Imported from cURL" {
                let short = short_name_from_url(&req.url);
                req.name = format!("{} {}", req.method, short);
            }
            let new_id = req.id.clone();
            if let Some(folder) = self.get_current_folder_mut() {
                folder.requests.push(req);
            }
            self.save_state();
            let p = self.selected_folder_path.clone();
            self.open_request(p, new_id);
            self.show_paste_modal = false;
            self.show_toast("Request imported");
        }
    }

    fn render_toast(&mut self, ctx: &egui::Context) {
        let Some((msg, ttl)) = self.toast.clone() else {
            return;
        };
        let dt = ctx.input(|i| i.unstable_dt);
        let new_ttl = ttl - dt;
        if new_ttl <= 0.0 {
            self.toast = None;
            return;
        }
        self.toast = Some((msg.clone(), new_ttl));
        ctx.request_repaint();
        egui::Area::new(egui::Id::new("toast"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(C_PANEL)
                    .stroke(egui::Stroke::new(1.0, C_ACCENT))
                    .rounding(10.0)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(msg).color(C_TEXT).size(13.0));
                    });
            });
    }

    fn render_folder(
        &mut self,
        ui: &mut egui::Ui,
        folder: &Folder,
        path: Vec<String>,
        depth: usize,
    ) {
        let is_renaming = self.renaming_folder_id.as_ref() == Some(&folder.id);
        let query = self.search_query.to_lowercase();
        let searching = !query.is_empty();

        // Visually separate collections (top-level) from folders (nested):
        //   • collections render without a folder glyph — just the chevron
        //     and a strong-weight name.
        //   • subfolders render with a small painter-drawn folder icon
        //     between the chevron and the name, so users can tell them
        //     apart at a glance (matching Postman / Insomnia).
        // A custom `.icon(...)` fn replaces CollapsingHeader's default
        // triangle too, because egui's default icon uses a glyph that
        // renders as tofu on some font setups — painter shapes are
        // always font-safe.
        let header_text = if is_renaming {
            String::new()
        } else {
            folder.name.clone()
        };
        let is_collection = depth == 0;
        let name_prefix = if is_collection || is_renaming {
            String::new()
        } else {
            // Leading space lets the folder icon we'll paint sit in front
            // of the header text without overlapping.
            "   ".to_string()
        };
        let mut header = egui::CollapsingHeader::new(
            egui::RichText::new(format!("{}{}", name_prefix, header_text))
                .size(13.0)
                .color(C_TEXT)
                .strong(),
        )
        .id_salt(&folder.id)
        .default_open(true)
        .icon(paint_folder_chevron);
        if searching {
            header = header.open(Some(true));
        }
        // Buttons-as-context: use right-click on the folder header to
        // add a request, add a subfolder, rename, duplicate, delete, etc.
        // Track what was chosen so we can apply it after the header closure.
        let mut action_add_request = false;
        let header_response = header.show(ui, |ui| {
            let mut to_delete: Option<String> = None;
            let mut to_duplicate: Option<String> = None;
            for req in &folder.requests {
                if searching
                    && !request_matches(req, &query)
                    && !folder.name.to_lowercase().contains(&query)
                {
                    continue;
                }
                let is_selected = self.selected_request_id.as_ref() == Some(&req.id);
                let mc = method_color(&req.method);
                // Shadow the outer `is_renaming` (which is for the folder
                // header) with one keyed on this request's id. Without this
                // shadow the row would fall back to the folder's flag and
                // the rename TextEdit would never appear.
                let is_renaming =
                    self.renaming_request_id.as_deref() == Some(req.id.as_str());

                // Compact Postman-style row
                let row_h = 26.0;
                let (rect, resp) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), row_h),
                    egui::Sense::click(),
                );

                if ui.is_rect_visible(rect) {
                    let bg = if is_selected {
                        C_ACCENT.linear_multiply(0.18)
                    } else if resp.hovered() {
                        C_ELEVATED
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    ui.painter()
                        .rect_filled(rect, egui::Rounding::same(5.0), bg);

                    if is_selected {
                        let bar = egui::Rect::from_min_size(
                            rect.min,
                            egui::vec2(3.0, rect.height()),
                        );
                        ui.painter()
                            .rect_filled(bar, egui::Rounding::same(2.0), C_ACCENT);
                    }

                    // Method as bold colored TEXT (no pill background).
                    // Reserve a fixed-width slot so all names align
                    // regardless of method length (GET vs OPTIONS).
                    let method_slot_w = 46.0;
                    let method_left = rect.left() + 10.0;
                    ui.painter().text(
                        egui::pos2(method_left, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        format!("{}", req.method),
                        egui::FontId::new(10.0, egui::FontFamily::Proportional),
                        mc,
                    );

                    if !is_renaming {
                        let name_x = method_left + method_slot_w;
                        let name_pos = egui::pos2(name_x, rect.center().y);
                        let font = egui::FontId::new(12.5, egui::FontFamily::Proportional);
                        let max_w = rect.right() - name_x - 6.0;
                        let display_name = elide(&req.name, max_w, &font, ui);
                        ui.painter().text(
                            name_pos,
                            egui::Align2::LEFT_CENTER,
                            display_name,
                            font,
                            C_TEXT,
                        );
                    }
                }

                // Inline rename TextEdit overlay (clearly visible against the row)
                if is_renaming {
                    // Same geometry as the name: method_left (10) + method_slot_w (46)
                    let name_start = rect.left() + 10.0 + 46.0;
                    let edit_rect = egui::Rect::from_min_max(
                        egui::pos2(name_start - 2.0, rect.top() + 3.0),
                        egui::pos2(rect.right() - 4.0, rect.bottom() - 3.0),
                    );
                    // Visible background + accent border so the input is obvious.
                    ui.painter()
                        .rect_filled(edit_rect, egui::Rounding::same(4.0), C_PANEL_DARK);
                    ui.painter().rect_stroke(
                        edit_rect,
                        egui::Rounding::same(4.0),
                        egui::Stroke::new(1.5, C_ACCENT),
                    );
                    let inner = edit_rect.shrink2(egui::vec2(6.0, 2.0));
                    let edit_resp = ui.put(
                        inner,
                        egui::TextEdit::singleline(&mut self.rename_request_text)
                            .desired_width(inner.width())
                            .frame(false)
                            .text_color(C_TEXT)
                            .font(egui::FontId::new(13.0, egui::FontFamily::Proportional)),
                    );
                    if self.request_rename_focus_pending {
                        self.request_rename_focus_pending = false;
                        edit_resp.request_focus();
                    }
                    let (enter, escape) = ui.input(|i| {
                        (
                            i.key_pressed(egui::Key::Enter),
                            i.key_pressed(egui::Key::Escape),
                        )
                    });
                    if enter && edit_resp.has_focus() {
                        let id = req.id.clone();
                        let new_name = self.rename_request_text.trim().to_string();
                        if !new_name.is_empty() {
                            self.rename_request(&id, new_name);
                        }
                        self.renaming_request_id = None;
                    } else if escape || (edit_resp.lost_focus() && !enter) {
                        self.renaming_request_id = None;
                    }
                } else {
                    // Hand-rolled double-click: if user clicks the same
                    // request row twice within DOUBLE_CLICK_SECS, treat as
                    // a double-click and start rename. Single click opens
                    // the request after a small grace period.
                    if resp.clicked() {
                        const DOUBLE_CLICK_SECS: f64 = 0.4;
                        let now = ui.input(|i| i.time);
                        let is_double = self
                            .last_request_click
                            .as_ref()
                            .map(|(id, t)| id == &req.id && (now - t) < DOUBLE_CLICK_SECS)
                            .unwrap_or(false);
                        if is_double {
                            self.renaming_request_id = Some(req.id.clone());
                            self.rename_request_text = req.name.clone();
                            self.request_rename_focus_pending = true;
                            self.last_request_click = None;
                        } else {
                            self.open_request(path.clone(), req.id.clone());
                            self.last_request_click = Some((req.id.clone(), now));
                        }
                    }
                    let req_id_for_menu = req.id.clone();
                    let req_name_for_menu = req.name.clone();
                    resp.context_menu(|ui| {
                        if ui.button("Rename").clicked() {
                            self.renaming_request_id = Some(req_id_for_menu.clone());
                            self.rename_request_text = req_name_for_menu.clone();
                            self.request_rename_focus_pending = true;
                            ui.close_menu();
                        }
                        if ui.button("Duplicate").clicked() {
                            to_duplicate = Some(req_id_for_menu.clone());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Delete").clicked() {
                            to_delete = Some(req_id_for_menu.clone());
                            ui.close_menu();
                        }
                    });
                }

                ui.add_space(1.0);
            }

            if let Some(dup_id) = to_duplicate {
                self.selected_folder_path = path.clone();
                let mut new_req_opt = None;
                if let Some(f) = self.get_current_folder_mut() {
                    if let Some(original) = f.requests.iter().find(|r| r.id == dup_id).cloned() {
                        let mut copy = original;
                        copy.id = Uuid::new_v4().to_string();
                        copy.name = format!("{} (copy)", copy.name);
                        new_req_opt = Some(copy.id.clone());
                        f.requests.push(copy);
                    }
                }
                self.save_state();
                if let Some(new_id) = new_req_opt {
                    let p = path.clone();
                    self.open_request(p, new_id);
                    self.show_toast("Request duplicated");
                }
            }

            if let Some(del_id) = to_delete {
                self.selected_folder_path = path.clone();
                if let Some(f) = self.get_current_folder_mut() {
                    f.requests.retain(|r| r.id != del_id);
                }
                self.save_state();
                self.prune_stale_tabs();
            }

            for subfolder in &folder.subfolders {
                if searching && !folder_matches(subfolder, &query) {
                    continue;
                }
                let mut subpath = path.clone();
                subpath.push(subfolder.id.clone());
                self.render_folder(ui, subfolder, subpath, depth + 1);
            }
        });

        // For nested folders, paint a small folder glyph in the 3-space
        // gap we reserved at the front of the header label. Collections
        // (depth == 0) intentionally get no folder icon so it's obvious
        // which rows are top-level.
        if !is_collection && !is_renaming {
            let rect = header_response.header_response.rect;
            // Chevron sits at ~rect.left() + 8; reserve ~12px for it then
            // a small gap before the folder glyph.
            let icon_center = egui::pos2(rect.left() + 24.0, rect.center().y);
            paint_folder_icon(&ui.painter_at(rect), icon_center, C_MUTED);
        }

        if is_renaming {
            let rect = header_response.header_response.rect;
            let mut rename_rect = rect;
            rename_rect.min.x += 25.0;
            let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rename_rect));
            child_ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.rename_folder_text)
                        .desired_width(150.0)
                        .font(egui::TextStyle::Body),
                );
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.rename_folder(&folder.id, self.rename_folder_text.clone());
                    self.renaming_folder_id = None;
                }
                if ui.button("✓").clicked() {
                    self.rename_folder(&folder.id, self.rename_folder_text.clone());
                    self.renaming_folder_id = None;
                }
                if ui.button("✖").clicked() {
                    self.renaming_folder_id = None;
                }
            });
        } else {
            let folder_id = folder.id.clone();
            let folder_name = folder.name.clone();
            let noun = if depth == 0 { "collection" } else { "folder" };
            let mut start_rename = false;
            let mut delete_folder = false;
            let mut export_json = false;
            let mut export_yaml = false;
            let mut add_subfolder = false;
            header_response.header_response.context_menu(|ui| {
                if ui.button("Add request").clicked() {
                    action_add_request = true;
                    ui.close_menu();
                }
                if ui.button("Add subfolder").clicked() {
                    add_subfolder = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Rename").clicked() {
                    start_rename = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export as JSON...").clicked() {
                    export_json = true;
                    ui.close_menu();
                }
                if ui.button("Export as YAML...").clicked() {
                    export_yaml = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button(format!("Delete {}", noun)).clicked() {
                    delete_folder = true;
                    ui.close_menu();
                }
            });

            // "Add request" — create a new request inside THIS folder and
            // open it as a tab.
            if action_add_request {
                self.selected_folder_path = path.clone();
                let count = folder.requests.len() + 1;
                let new_req = Request {
                    id: Uuid::new_v4().to_string(),
                    name: format!("Request {}", count),
                    method: HttpMethod::GET,
                    url: "https://api.example.com".to_string(),
                    query_params: vec![],
                    headers: vec![],
                    cookies: vec![],
                    body: String::new(),
                    body_ext: None,
                    auth: Auth::None,
                    extractors: vec![],
                };
                let new_id = new_req.id.clone();
                if let Some(f) = self.get_current_folder_mut() {
                    f.requests.push(new_req);
                }
                self.save_state();
                self.open_request(path.clone(), new_id);
            }
            if start_rename {
                self.renaming_folder_id = Some(folder_id.clone());
                self.rename_folder_text = folder_name;
            }
            if add_subfolder {
                self.selected_folder_path = path.clone();
                let subcount = folder.subfolders.len() + 1;
                if let Some(f) = self.get_current_folder_mut() {
                    f.subfolders.push(Folder {
                        id: Uuid::new_v4().to_string(),
                        name: format!("Folder {}", subcount),
                        requests: vec![],
                        subfolders: vec![],
                    });
                }
                self.save_state();
            }
            if export_json {
                self.do_export_folder(&folder_id, io::Format::Json);
            }
            if export_yaml {
                self.do_export_folder(&folder_id, io::Format::Yaml);
            }
            if delete_folder {
                self.delete_folder(&folder_id);
            }
        }
    }

    fn rename_folder(&mut self, folder_id: &str, new_name: String) {
        fn go(folders: &mut Vec<Folder>, id: &str, name: String) -> bool {
            for f in folders {
                if f.id == id {
                    f.name = name;
                    return true;
                }
                if go(&mut f.subfolders, id, name.clone()) {
                    return true;
                }
            }
            false
        }
        if go(&mut self.state.folders, folder_id, new_name) {
            self.save_state();
        }
    }

    fn delete_folder(&mut self, folder_id: &str) {
        fn go(folders: &mut Vec<Folder>, id: &str) -> bool {
            if let Some(pos) = folders.iter().position(|f| f.id == id) {
                folders.remove(pos);
                return true;
            }
            for f in folders {
                if go(&mut f.subfolders, id) {
                    return true;
                }
            }
            false
        }
        if go(&mut self.state.folders, folder_id) {
            self.save_state();
            self.prune_stale_tabs();
        }
    }
}


/// Ensure the URL has a scheme so reqwest can parse it. Users
/// commonly paste `api.example.com/foo` expecting it to work.
///
/// Defaults to `http://` — matches curl's historical default and
/// Postman's behavior for schemeless URLs. This works for internal /
/// dev hosts that don't terminate TLS on :443; users hitting a
/// public API can always type `https://` themselves, but an internal
/// `dev.foo.int` served on port 80 would otherwise connection-refuse
/// because we'd be dialing :443.
fn ensure_url_scheme(url: &str) -> String {
    let t = url.trim();
    if t.is_empty() {
        return t.to_string();
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ws://")
        || lower.starts_with("wss://")
    {
        return t.to_string();
    }
    format!("http://{}", t)
}

/// Flatten a reqwest error's cause chain into a readable multi-line
/// string. reqwest's top-level `Display` often just says "builder
/// error" or "error sending request"; the actual reason (invalid URL,
/// DNS failure, etc.) is hidden in `source()`.
fn format_request_error(err: &reqwest::Error) -> String {
    use std::error::Error;
    let mut msg = format!("Error: {}", err);
    let mut source: Option<&(dyn Error + 'static)> = err.source();
    while let Some(s) = source {
        msg.push_str(&format!("\n  caused by: {}", s));
        source = s.source();
    }
    msg
}

fn main() -> Result<(), eframe::Error> {
    // Force NSApp into Regular activation policy BEFORE eframe starts the
    // macOS run loop. Once NSApp.run has begun processing events, macOS
    // rejects the setActivationPolicy(Regular) call, which is why calling
    // this from update() / CreationContext didn't work — the process stayed
    // Accessory/Prohibited and Cmd+Tab showed the parent terminal's icon.
    if let Err(e) = set_macos_activation_policy_regular() {
        eprintln!("[icon] activation policy set failed: {}", e);
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([900.0, 600.0])
        .with_title("Rusty Requester");
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        // Don't restore window geometry from a previous run — eframe's
        // default `persist_window = true` was reopening the app at the last
        // dragged size, which could be much smaller than the `with_inner_size`
        // default (including below `min_inner_size`).
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "Rusty Requester",
        options,
        Box::new(|_cc| Ok(Box::new(ApiClient::default()))),
    )
}
