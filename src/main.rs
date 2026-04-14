mod extract;
mod icon;
mod io;
mod model;
mod net;
mod snippet;
mod theme;
mod ui;
mod widgets;

use eframe::egui;
use icon::{
    load_icon_color_image, load_window_icon, set_macos_activation_policy_regular,
    set_macos_app_icon_image, APP_ICON_BYTES,
};
use model::*;
use poll_promise::Promise;
use snippet::SnippetLang;
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
                net::execute_request(&request, env.as_ref())
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

    // Per-folder export was removed from the UI (the overflow menu was
    // pared down to Add request / Add folder / Rename / Duplicate /
    // Delete), but the machinery stays wired in case we re-expose it
    // (e.g. from the top-level Export menu with a folder picker).
    #[allow(dead_code)]
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

    /// Recursively deep-clones a folder (with fresh UUIDs for the folder,
    /// every request, and every nested subfolder) and inserts the copy
    /// next to the original. Naming convention mirrors the per-request
    /// duplicate — appends " (copy)" to the name.
    pub(crate) fn duplicate_folder(&mut self, folder_id: &str) {
        fn clone_deep(src: &Folder) -> Folder {
            Folder {
                id: Uuid::new_v4().to_string(),
                name: src.name.clone(),
                requests: src
                    .requests
                    .iter()
                    .map(|r| Request {
                        id: Uuid::new_v4().to_string(),
                        ..r.clone()
                    })
                    .collect(),
                subfolders: src.subfolders.iter().map(clone_deep).collect(),
            }
        }
        fn go(folders: &mut Vec<Folder>, id: &str) -> bool {
            if let Some(pos) = folders.iter().position(|f| f.id == id) {
                let mut dup = clone_deep(&folders[pos]);
                dup.name = format!("{} (copy)", dup.name);
                folders.insert(pos + 1, dup);
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
        }
    }
}


fn main() -> Result<(), eframe::Error> {
    // Cheap `--version` / `-V` flag so anyone (user or `install.sh`)
    // can confirm which build is actually on disk without opening the
    // UI — the single most useful weapon against LaunchServices-cache
    // confusion.
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("rusty-requester {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => {}
        }
    }

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
