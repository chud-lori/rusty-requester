mod actions;
mod assertion;
mod backup;
mod cookies;
mod diff;
mod env_compare;
mod extract;
mod html_preview;
mod icon;
mod io;
#[cfg(target_os = "macos")]
mod macos_menu;
mod model;
mod net;
mod oauth;
mod privacy;
mod runner;
mod runner_detail;
mod secret_scanner;
mod snippet;
mod sse;
mod sync;
mod theme;
mod ui;
mod widgets;

use eframe::egui;
use icon::{
    load_icon_color_image, load_window_icon, set_macos_activation_policy_regular,
    set_macos_app_icon_image, APP_ICON_BYTES,
};
use io::curl;
use model::*;
/// In-flight send: tokio task + result receiver. `handle.abort()` is
/// what powers the Cancel button — dropping the future mid-`.await`
/// also drops the underlying hyper connection.
struct InFlightRequest {
    handle: tokio::task::JoinHandle<()>,
    rx: std::sync::mpsc::Receiver<net::RequestUpdate>,
}

struct InFlightCollectionRun {
    handle: tokio::task::JoinHandle<()>,
    rx: std::sync::mpsc::Receiver<runner::CollectionRunEvent>,
}

#[derive(Clone)]
pub(crate) struct ExportSecretWarning {
    format: io::Format,
    findings: Vec<secret_scanner::SecretFinding>,
}

#[derive(Clone, Copy)]
pub(crate) struct ExportDecision {
    format: io::Format,
    redacted: bool,
}

/// Result of a successful OAuth2 flow, ready to be copied into the
/// active request's `Auth::OAuth2` state.
pub struct OAuth2TokenUpdate {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Option<i64>,
}
use snippet::SnippetLang;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use widgets::*;

/// Result of the most recent manual update check, used to render the
/// inline status block in the Settings modal. `Idle` = no inline UI;
/// the launch-time auto-check leaves this alone (the sidebar pill
/// handles that surface).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum UpdateCheckOutcome {
    #[default]
    Idle,
    Checking,
    NoUpdate,
    Failed(String),
}

pub(crate) enum UpdateCheckMessage {
    Available(String),
    Current,
    Failed(String),
}

/// In-memory snapshot of a request's last response so switching tabs
/// doesn't wipe what the user was reading. Keyed by `request_id` on
/// `ApiClient.response_cache`; NOT persisted to `data.json` (response
/// bodies can be large, and "last response" is a session concern).
#[derive(Clone, Default)]
struct CachedResponse {
    text: String,
    status: String,
    time: String,
    headers: Vec<(String, String)>,
    headers_bytes: usize,
    body_bytes: usize,
    prepare_ms: u64,
    waiting_ms: u64,
    download_ms: u64,
    total_ms: u64,
    previous_text: Option<String>,
    streaming_events: Vec<crate::sse::SseEvent>,
    assertion_results: Vec<Option<AssertionResult>>,
}

type RunnerResultRow = runner_detail::RunnerResultRow;

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

    /// Previous response body, retained just long enough to power the
    /// **Diff** view. Snapshot taken right before a new response
    /// overwrites `response_text`; `None` means there's no prior
    /// response to compare against.
    previous_response_text: Option<String>,

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
    editing_assertions: Vec<ResponseAssertion>,
    /// Transient per-send outcomes — populated by
    /// `apply_response_assertions`, indexed parallel to
    /// `editing_assertions`. `None` means "not yet run" (e.g. before
    /// the first send or after an assertion was added).
    assertion_results: Vec<Option<AssertionResult>>,
    editing_request_id_for_history: Option<String>,
    /// Saved-request baseline from the last explicit save. When the live
    /// request differs, the tab strip shows the same amber dot drafts use.
    saved_request_dirty_baselines: HashMap<String, Request>,

    storage_path: PathBuf,

    /// Active in-flight request. Holds a tokio `JoinHandle` so we can
    /// `abort()` on Cancel, and a `mpsc::Receiver` to pick up the
    /// `ResponseData` once the task completes. `None` when idle.
    request_in_flight: Option<InFlightRequest>,
    /// Accumulated SSE events for the current response, if the last
    /// (or in-flight) request has an `text/event-stream` body. Empty
    /// for non-SSE responses. Powers the Events body-view mode.
    streaming_events: Vec<crate::sse::SseEvent>,

    renaming_folder_id: Option<String>,
    rename_folder_text: String,

    request_tab: RequestTab,
    response_tab: ResponseTab,

    show_paste_modal: bool,
    paste_curl_text: String,
    paste_error: String,
    show_runner_modal: bool,
    show_backup_modal: bool,
    show_sync_modal: bool,
    show_collection_settings_modal: bool,
    collection_settings_folder_id: Option<String>,
    collection_git_status: String,
    confirm_restore_backup_path: Option<PathBuf>,
    runner_scope_folder_id: Option<String>,
    runner_data_rows: String,
    runner_selected_preset_id: Option<String>,
    runner_preset_name_input: String,
    runner_preset_rename_input: String,
    runner_results: Vec<RunnerResultRow>,
    runner_selected_result: Option<usize>,
    runner_status: String,
    runner_in_flight: Option<InFlightCollectionRun>,
    sync_in_flight: Option<sync::InFlightSync>,

    show_snippet_panel: bool,
    snippet_lang: SnippetLang,
    /// Wall-clock (egui `input.time`) at which the snippet copy button
    /// was last pressed. Drives the transient "Copied!" label that
    /// flashes next to the button. `None` = never copied this session,
    /// or flash already expired.
    snippet_copied_at: Option<f64>,
    /// Same idea for the response-body copy button. Kept separate so
    /// copying one doesn't flash the other.
    response_copied_at: Option<f64>,

    /// Per-request last-response cache. When the user switches tabs we
    /// stash the outgoing request's live response fields here, and pull
    /// them back when the tab re-activates. In-memory only — not
    /// persisted (keeps `data.json` lean, matches Postman's
    /// session-scoped behavior).
    response_cache: HashMap<String, CachedResponse>,

    sidebar_view: SidebarView,
    show_env_modal: bool,
    selected_env_for_edit: Option<String>,
    env_compare_source_id: Option<String>,
    env_compare_target_id: Option<String>,

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
    /// One-shot flag: when a user clicks a tab, we want the sidebar
    /// tree to expand all parent folders and scroll the matching row
    /// into view. Stores `(folder_path, request_id)` of the row to
    /// reveal; cleared by the sidebar once it's done scrolling.
    reveal_in_sidebar_pending: Option<(Vec<String>, String)>,

    /// Pending file-dialog actions — executed at the top of the next
    /// `update()` frame rather than immediately inside the menu closure,
    /// so the popup has a chance to close visibly before `rfd` blocks the
    /// main thread on the OS file picker.
    pending_import: bool,
    pending_export_json: bool,
    pending_export_yaml: bool,
    export_secret_warning: Option<ExportSecretWarning>,
    pending_export_decision: Option<ExportDecision>,
    /// Text to copy to the clipboard at the top of the next frame.
    /// Used by palette-dispatched actions (e.g. Copy as cURL) that
    /// don't have direct access to `egui::Context` for `ctx.copy_text`.
    pending_clipboard: Option<String>,
    /// One-shot toast shown on the first `update()` frame when the
    /// startup state-load flagged something the user should know —
    /// e.g. "`data.json` was corrupted — backed up to …". Cleared
    /// after it fires.
    startup_warning: Option<String>,
    /// Background update-check receiver. Set on startup via
    /// `spawn_update_check`; drained on each `update()` frame.
    update_check_rx: Option<std::sync::mpsc::Receiver<UpdateCheckMessage>>,
    /// Latest version string found by the update check, cached so the
    /// banner stays visible across frames. `None` = no update / not
    /// checked yet / current version is latest.
    update_available: Option<String>,
    /// Toggles the update-instructions modal — surfaced via the
    /// sidebar pill when an update is available.
    show_update_modal: bool,
    /// True while a one-click in-app update is in progress. Drives the
    /// "Updating…" modal that tails `update.log` so the user sees what
    /// the installer is doing in real time. The wrapper script (see
    /// `spawn_update`) writes its stdout/stderr to `update.log` and a
    /// terminal `update.status` file; `install.sh` will kill our
    /// process partway through (it has to, to replace the binary), the
    /// wrapper survives via `nohup` + `disown` and relaunches us.
    updating_in_progress: bool,
    /// Last ~4 KB of `update.log`, refreshed every ~300 ms while
    /// `updating_in_progress`. Shown in the modal as a scrolling
    /// monospace tail. Cleared between update attempts.
    update_log_tail: String,
    /// Last time we polled `update.log`, as `ctx.input(|i| i.time)`
    /// seconds. Throttles the re-read so we're not hammering the FS on
    /// every frame.
    update_log_last_poll: f64,
    /// Set on launch when the previous run kicked off an in-app
    /// update. `Some((true, "0.18.8"))` → green "Updated to vX.Y.Z"
    /// banner with a "View log" button. `Some((false, reason))` → red
    /// "Update failed" banner. Cleared after the user dismisses or
    /// auto-times-out.
    post_update_notice: Option<(bool, String)>,
    /// `ctx.input.time` when the post-update banner first appeared.
    /// Used to auto-dismiss the success banner after ~10s so a
    /// successful update doesn't leave a sticky pill on screen
    /// forever (failures stay until the user clicks Dismiss).
    post_update_notice_started_at: Option<f64>,
    /// Drives the inline update-status block in the Settings modal so
    /// users who click "Check for updates now" see the result inline
    /// instead of having to close Settings to find the update button.
    /// Set to `Checking` when the manual button fires; the frame-level
    /// drain in `update()` transitions it to `NoUpdate` (channel
    /// disconnected without a payload) or leaves it `Checking` while
    /// `update_available` picks up the new tag.
    manual_update_check: UpdateCheckOutcome,

    /// In-flight OAuth 2.0 flow. `Some` while the user is completing
    /// the authorize-redirect dance in their browser; drained each
    /// frame. `Ok(tokens)` → copy into the active Auth::OAuth2 state
    /// and clear; `Err(msg)` → toast the error and clear.
    oauth_flow_rx: Option<std::sync::mpsc::Receiver<Result<OAuth2TokenUpdate, String>>>,
    /// Human-readable status line shown under the Auth tab while a
    /// flow is in progress ("Waiting for browser redirect…", etc.).
    oauth_flow_status: Option<String>,
    /// "Get New Token" was clicked this frame — render_auth_tab can't
    /// call `start_oauth_flow` directly because it's already holding
    /// a mutable borrow of `editing_auth`. Caller flushes this flag
    /// after the match ends.
    oauth_start_requested: bool,

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
    /// One-shot: when true on the next render, focus the name field then
    /// clear. Set when the modal opens. Without the one-shot we'd refocus
    /// every frame and the user could never click into the search field.
    save_draft_name_focus_pending: bool,

    /// "Save changes?" confirmation modal — shown when closing a draft
    /// (unsaved) tab so the user can save, discard, or cancel.
    confirm_close_draft_idx: Option<usize>,

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
    /// icon button in the body toolbar, or Cmd/Ctrl+F).
    body_search_visible: bool,
    /// Set for one frame when Cmd/Ctrl+F fires — tells the response
    /// view to call `request_focus()` on the search TextEdit so the
    /// user can start typing immediately. Cleared after consumption.
    body_search_focus_pending: bool,
    /// Per-current-response set of collapsed JSON object/array opener
    /// lines (1-based line numbers in `response_text`). Postman-style
    /// folding: clicking the chevron next to a `{` or `[` adds its
    /// line here; the JSON view skips the contents until the matching
    /// closer. Cleared every time `response_text` changes.
    folded_response_lines: HashSet<u32>,

    /// Long-lived HTTP client built from `state.settings`. Rebuilt on
    /// app startup and whenever the Settings modal saves. Reused across
    /// every send so we don't reopen TCP/TLS pools per request.
    http_client: reqwest::Client,
    /// Long-lived tokio runtime — also reused across sends.
    http_runtime: tokio::runtime::Runtime,

    /// Settings modal state.
    show_settings_modal: bool,
    /// Working copy of settings while the modal is open. Committed to
    /// `state.settings` on Save.
    editing_settings: AppSettings,
    /// About modal (Help → About Rusty Requester).
    show_about_modal: bool,
    /// When `Some`, the central panel shows a collection/folder
    /// overview ("homepage") for that folder ID instead of the
    /// active request editor. Set by the folder `⋯` menu →
    /// "Open overview". Cleared when the user picks any request.
    viewing_folder_id: Option<String>,
    /// Working copy of the folder description while the overview
    /// is open — written back to `Folder.description` on blur.
    editing_folder_desc: String,

    // --- Command palette (⌘P) ---------------------------------------
    show_command_palette: bool,
    palette_query: String,
    palette_selected: usize,
    palette_focus_pending: bool,

    // --- Actions palette (⇧⌘P) ---------------------------------------
    /// Parallel palette for triggering app actions (toggle snippet
    /// panel, duplicate tab, clear history, etc.) rather than
    /// navigating to requests. Shares the same overlay chrome as the
    /// command palette but dispatches into `run_action` on Enter.
    show_actions_palette: bool,
    actions_palette_query: String,
    actions_palette_selected: usize,
    actions_palette_focus_pending: bool,
    /// Have we installed the macOS NSMenu yet? We defer the install
    /// to the first `update()` frame because doing it before
    /// `eframe::run_native` lets winit overwrite our menu with its
    /// default Services/Hide/Quit stub.
    #[cfg(target_os = "macos")]
    macos_menu_installed: bool,
}

impl Default for ApiClient {
    fn default() -> Self {
        let storage_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rusty-requester")
            .join("data.json");

        // Load outcome distinguishes first-launch from corrupted-file.
        // A corrupted file has already been renamed to a `.broken.<ts>`
        // backup by `load_state`; we surface the path via
        // `startup_warning` so the user sees a toast on first frame.
        let (state, startup_warning) = match Self::load_state(&storage_path) {
            LoadOutcome::Ok(s) => (*s, None),
            LoadOutcome::Fresh => (Self::fresh_state(), None),
            LoadOutcome::Corrupted { backup_path, error } => {
                eprintln!(
                    "rusty-requester: data.json was corrupted ({}). Backed up to {}.",
                    error,
                    backup_path.display()
                );
                (
                    Self::fresh_state(),
                    Some(format!(
                        "data.json was corrupted — backed up to {}",
                        backup_path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("a .broken.* file")
                    )),
                )
            }
        };

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
            previous_response_text: None,
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
            editing_assertions: vec![],
            assertion_results: vec![],
            editing_request_id_for_history: None,
            saved_request_dirty_baselines: HashMap::new(),
            storage_path,
            request_in_flight: None,
            streaming_events: Vec::new(),
            renaming_folder_id: None,
            rename_folder_text: String::new(),
            request_tab: RequestTab::Params,
            response_tab: ResponseTab::Body,
            show_paste_modal: false,
            paste_curl_text: String::new(),
            paste_error: String::new(),
            show_runner_modal: false,
            show_backup_modal: false,
            show_sync_modal: false,
            show_collection_settings_modal: false,
            collection_settings_folder_id: None,
            collection_git_status: String::new(),
            confirm_restore_backup_path: None,
            runner_scope_folder_id: None,
            runner_data_rows: String::new(),
            runner_selected_preset_id: None,
            runner_preset_name_input: String::new(),
            runner_preset_rename_input: String::new(),
            runner_results: Vec::new(),
            runner_selected_result: None,
            runner_status: String::new(),
            runner_in_flight: None,
            sync_in_flight: None,
            show_snippet_panel: false,
            snippet_lang: SnippetLang::Curl,
            snippet_copied_at: None,
            response_copied_at: None,
            response_cache: HashMap::new(),
            sidebar_view: SidebarView::Collections,
            show_env_modal: false,
            selected_env_for_edit: None,
            env_compare_source_id: None,
            env_compare_target_id: None,
            toast: None,
            focus_search_next_frame: false,
            app_icon: None,
            macos_icon_set: false,
            renaming_request_id: None,
            rename_request_text: String::new(),
            request_rename_focus_pending: false,
            last_request_click: None,
            reveal_in_sidebar_pending: None,
            pending_import: false,
            pending_export_json: false,
            pending_export_yaml: false,
            export_secret_warning: None,
            pending_export_decision: None,
            pending_clipboard: None,
            startup_warning: None,
            update_check_rx: None,
            update_available: None,
            show_update_modal: false,
            updating_in_progress: false,
            update_log_tail: String::new(),
            update_log_last_poll: 0.0,
            post_update_notice: None,
            post_update_notice_started_at: None,
            manual_update_check: UpdateCheckOutcome::Idle,
            oauth_flow_rx: None,
            oauth_flow_status: None,
            oauth_start_requested: false,
            save_draft_open: false,
            save_draft_tab_idx: None,
            save_draft_target_path: Vec::new(),
            save_draft_name: String::new(),
            save_draft_search: String::new(),
            save_draft_new_folder_name: None,
            save_draft_name_focus_pending: false,
            confirm_close_draft_idx: None,
            request_split_px: 320.0,
            body_view: BodyView::Json,
            body_tree_filter: String::new(),
            body_search_query: String::new(),
            body_search_visible: false,
            body_search_focus_pending: false,
            folded_response_lines: HashSet::new(),
            http_client: net::build_client(&AppSettings::default()),
            http_runtime: net::build_runtime(),
            show_settings_modal: false,
            editing_settings: AppSettings::default(),
            show_about_modal: false,
            viewing_folder_id: None,
            editing_folder_desc: String::new(),
            show_command_palette: false,
            show_actions_palette: false,
            actions_palette_query: String::new(),
            actions_palette_selected: 0,
            actions_palette_focus_pending: false,
            palette_query: String::new(),
            palette_selected: 0,
            palette_focus_pending: false,
            #[cfg(target_os = "macos")]
            macos_menu_installed: false,
        };
        // Attach the startup warning (if any). Consumed by the first
        // `update()` call via `show_toast`.
        this.startup_warning = startup_warning;
        // Rebuild the HTTP client from the deserialized settings — the
        // initial one above was a placeholder, because we couldn't read
        // `state.settings` before `state` was moved into `this`.
        this.http_client = net::build_client(&this.state.settings);
        // Fire a background check against GitHub's latest-release API.
        // One HTTP call per launch; silent failure if offline or if
        // GitHub rate-limits us — an API client crashing on startup
        // because its update check hiccuped would be absurd. Users
        // who want strict offline operation can disable this in
        // Settings → Check for updates on launch.
        if this.state.settings.check_updates_on_launch {
            this.update_check_rx = Some(spawn_update_check(&this.http_runtime));
        }
        // Restore active tab — if state has a saved `active_tab_id`,
        // activate that tab now. Otherwise fall back to the first open tab.
        let active_tab =
            restored_active_tab(&this.state.open_tabs, this.state.active_tab_id.as_deref());
        if let Some(tab) = active_tab {
            this.selected_folder_path = tab.folder_path;
            this.selected_request_id = Some(tab.request_id.clone());
            this.state.active_tab_id = Some(tab.request_id);
            this.load_request_for_editing();
        }
        this
    }
}

fn restored_active_tab(open_tabs: &[OpenTab], active_tab_id: Option<&str>) -> Option<OpenTab> {
    let by_id =
        active_tab_id.and_then(|id| open_tabs.iter().find(|tab| tab.request_id == id).cloned());
    by_id.or_else(|| open_tabs.first().cloned())
}

/// Outcome of loading `data.json` at startup. `Fresh` means the file
/// didn't exist yet (first launch); `Corrupted` means it existed but
/// couldn't be parsed and has been sidelined to a backup file — the
/// caller should surface the backup path so the user knows where
/// their old data went.
enum LoadOutcome {
    Ok(Box<AppState>),
    Fresh,
    Corrupted { backup_path: PathBuf, error: String },
}

impl ApiClient {
    /// Default empty state for first launch (no `data.json` yet) or
    /// after a corrupted-file recovery. One starter collection so the
    /// sidebar isn't a blank void.
    fn fresh_state() -> AppState {
        AppState {
            folders: vec![Folder {
                id: Uuid::new_v4().to_string(),
                name: "My Requests".to_string(),
                requests: vec![],
                subfolders: vec![],
                description: String::new(),
                sync: SyncConfig::default(),
            }],
            environments: vec![],
            active_env_id: None,
            history: vec![],
            drafts: vec![],
            open_tabs: vec![],
            active_tab_id: None,
            settings: AppSettings::default(),
            runner_presets: vec![],
            sync: SyncConfig::default(),
        }
    }

    fn load_state(path: &PathBuf) -> LoadOutcome {
        let data = match fs::read_to_string(path) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return LoadOutcome::Fresh,
            Err(_) => return LoadOutcome::Fresh, // treat other IO errors as fresh; worst case is an empty workspace
        };
        match serde_json::from_str::<AppState>(&data) {
            Ok(state) => LoadOutcome::Ok(Box::new(state)),
            Err(e) => {
                // Move the broken file aside so we never silently clobber
                // the user's data on the next save. Timestamped so
                // repeated corruptions don't overwrite each other.
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let backup = path.with_extension(format!("json.broken.{}", ts));
                let _ = fs::rename(path, &backup);
                LoadOutcome::Corrupted {
                    backup_path: backup,
                    error: e.to_string(),
                }
            }
        }
    }

    /// Atomic write: serialize → write to `<path>.tmp` → `fsync` → rename
    /// over the real file. This prevents a crash / power cut mid-write
    /// from leaving a truncated `data.json` the next launch can't parse.
    /// Rename is atomic on POSIX; on Windows it's close enough for our
    /// needs since `rename` there uses `MoveFileEx` with replace-existing.
    fn save_state(&mut self) {
        use std::io::Write;
        // Sync the active-tab id into state so the workspace restores to
        // this tab on next launch.
        self.state.active_tab_id = self.selected_request_id.clone();
        if let Some(parent) = self.storage_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let Ok(json) = serde_json::to_string_pretty(&self.state) else {
            return;
        };
        let tmp = self.storage_path.with_extension("json.tmp");
        let Ok(mut f) = fs::File::create(&tmp) else {
            return;
        };
        if f.write_all(json.as_bytes()).is_err() {
            let _ = fs::remove_file(&tmp);
            return;
        }
        // fsync so the rename target's data is durable on disk before
        // we swap it into place. Without this, a crash after the rename
        // can still leave a zero-length file.
        let _ = f.sync_all();
        drop(f);
        let _ = fs::rename(&tmp, &self.storage_path);
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
            return self.state.drafts.iter().find(|r| &r.id == req_id).cloned();
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

    fn get_saved_request_by_id(&self, request_id: &str) -> Option<Request> {
        fn go(folders: &[Folder], request_id: &str) -> Option<Request> {
            for folder in folders {
                if let Some(req) = folder.requests.iter().find(|r| r.id == request_id) {
                    return Some(req.clone());
                }
                if let Some(req) = go(&folder.subfolders, request_id) {
                    return Some(req);
                }
            }
            None
        }
        go(&self.state.folders, request_id)
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
            let mut changed_pair: Option<(Request, Request)> = None;
            if let Some(folder) = self.get_current_folder_mut() {
                if let Some(request) = folder.requests.iter_mut().find(|r| r.id == req_id) {
                    let before = request.clone();
                    updater(request);
                    changed_pair = Some((before, request.clone()));
                }
            }
            if let Some((before, after)) = changed_pair {
                self.track_saved_request_change(&req_id, before, &after);
            }
            self.save_state();
        }
    }

    fn track_saved_request_change(&mut self, request_id: &str, before: Request, after: &Request) {
        if before == *after {
            return;
        }
        if let Some(baseline) = self.saved_request_dirty_baselines.get(request_id) {
            if baseline == after {
                self.saved_request_dirty_baselines.remove(request_id);
            }
        } else {
            self.saved_request_dirty_baselines
                .insert(request_id.to_string(), before);
        }
    }

    fn request_tab_has_unsaved_changes(&self, tab: &OpenTab) -> bool {
        tab.is_draft()
            || self
                .saved_request_dirty_baselines
                .contains_key(&tab.request_id)
    }

    fn mark_saved_request_clean(&mut self, request_id: &str) {
        self.saved_request_dirty_baselines.remove(request_id);
    }

    fn commit_editing(&mut self) {
        let name = self.editing_name.clone();
        let method = self.editing_method.clone();
        // editing_url holds the full URL (base + query string);
        // the model stores base and query_params separately.
        let (base_url, _) = curl::split_url(&self.editing_url);
        let body = self.editing_body.clone();
        let headers = self.editing_headers.clone();
        let params = self.editing_params.clone();
        let cookies = self.editing_cookies.clone();
        let body_ext = self.editing_body_ext.clone();
        let auth = self.editing_auth.clone();
        let extractors = self.editing_extractors.clone();
        let assertions = self.editing_assertions.clone();
        self.update_current_request(|req| {
            req.name = name;
            req.method = method;
            req.url = base_url;
            req.body = body;
            req.headers = headers;
            req.query_params = params;
            req.cookies = cookies;
            req.body_ext = body_ext;
            req.auth = auth;
            req.extractors = extractors;
            req.assertions = assertions;
        });
    }

    fn send_request(&mut self) {
        // Already sending — treat second Send as a no-op. Cancel has
        // its own button; we don't want double-firing to abort.
        if self.request_in_flight.is_some() {
            return;
        }
        self.commit_editing();
        let env = self.active_environment().cloned();
        if let Some(request) = self.get_current_request() {
            // Snapshot the previous response body for the Diff view,
            // but only if it looks like a real response (not the
            // "Loading..." placeholder or an empty slate).
            if !self.response_text.is_empty() && self.response_text != "Loading..." {
                self.previous_response_text = Some(self.response_text.clone());
            }
            self.is_loading = true;
            self.response_text = "Loading...".to_string();
            self.folded_response_lines.clear();
            self.response_status = "Sending request...".to_string();
            self.response_time = String::new();
            self.response_headers.clear();
            self.streaming_events.clear();

            let client = self.http_client.clone();
            let max_body_bytes =
                (self.state.settings.max_body_mb as usize).saturating_mul(1024 * 1024);
            let (tx, rx) = std::sync::mpsc::channel::<net::RequestUpdate>();
            // Spawn on our long-lived runtime so Cancel can abort
            // the JoinHandle; dropping the future also drops the
            // in-flight hyper connection. Result flows back via a
            // std::sync::mpsc::channel that `update()` polls. For
            // SSE responses the task emits a Progress update per
            // event and a Final at the end; non-SSE just sends one
            // Final.
            let tx_progress = tx.clone();
            let handle = self.http_runtime.spawn(async move {
                let r = net::execute_request_async(
                    client,
                    request,
                    env,
                    max_body_bytes,
                    Some(tx_progress),
                )
                .await;
                let _ = tx.send(net::RequestUpdate::Final(r));
            });
            self.request_in_flight = Some(InFlightRequest { handle, rx });
        }
    }

    /// Copy a ResponseData snapshot into the ApiClient's response
    /// fields. Shared by both Progress (streaming SSE) and Final
    /// (terminal) updates — so the UI sees the same flow regardless.
    fn apply_response_snapshot(&mut self, r: &ResponseData) {
        self.response_text = r.body.clone();
        self.folded_response_lines.clear();
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
    }

    /// Abort the in-flight request (if any). Drops the tokio task so
    /// the hyper/TCP connection unwinds immediately; surfaces a
    /// "Cancelled" status so the user sees their click took effect.
    fn cancel_request(&mut self) {
        if let Some(f) = self.request_in_flight.take() {
            f.handle.abort();
            self.is_loading = false;
            self.response_status = "Cancelled".to_string();
            self.response_text = "Request was cancelled by the user.".to_string();
            self.folded_response_lines.clear();
            self.response_time = String::new();
            self.show_toast("Request cancelled");
        }
    }

    fn active_environment(&self) -> Option<&Environment> {
        let id = self.state.active_env_id.as_ref()?;
        self.state.environments.iter().find(|e| &e.id == id)
    }

    /// Kick off an OAuth 2.0 Authorization Code + PKCE flow using the
    /// config currently showing on the Auth tab. Spawns a background
    /// thread that opens the browser, waits for the redirect, and
    /// exchanges the code for a token — result flows back via
    /// `oauth_flow_rx` and is merged into `editing_auth` on the next
    /// `update()` frame. Returns immediately; UI shows "Waiting…"
    /// until the flow completes.
    fn start_oauth_flow(&mut self) {
        let Auth::OAuth2(state) = &self.editing_auth else {
            return;
        };
        if state.config.auth_url.trim().is_empty()
            || state.config.token_url.trim().is_empty()
            || state.config.client_id.trim().is_empty()
        {
            self.show_toast("Fill in Auth URL, Token URL, and Client ID first");
            return;
        }
        let config = state.config.clone();
        let client = self.http_client.clone();
        let rt_handle = self.http_runtime.handle().clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.oauth_flow_rx = Some(rx);
        self.oauth_flow_status = Some("Opening browser…".to_string());

        std::thread::spawn(move || {
            let flow = match oauth::begin_flow(&config) {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.send(Err(format!("Flow setup failed: {}", e)));
                    return;
                }
            };
            let auth_url = flow.authorize_url(&config);
            if let Err(e) = webbrowser_open(&auth_url) {
                let _ = tx.send(Err(format!("Could not open browser: {}", e)));
                return;
            }
            // Block up to 2 min for the user to complete the redirect.
            let code = match flow.wait_for_redirect(std::time::Duration::from_secs(120)) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Err(format!("{}", e)));
                    return;
                }
            };
            let redirect_uri = flow.redirect_uri().to_string();
            // Run the token exchange on the shared tokio runtime so
            // we don't build a throwaway runtime per flow.
            let exchange = rt_handle.block_on(oauth::exchange_code(
                &client,
                &config,
                &code,
                &flow.verifier,
                &redirect_uri,
            ));
            match exchange {
                Ok(tr) => {
                    let expires_at = tr.expires_in_secs.map(|s| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        now + s
                    });
                    let _ = tx.send(Ok(OAuth2TokenUpdate {
                        access_token: tr.access_token,
                        refresh_token: tr.refresh_token,
                        expires_at,
                    }));
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("{}", e)));
                }
            }
        });
    }

    /// Called each `update()` frame — if the OAuth flow has
    /// completed, drain the result into `editing_auth` (success) or
    /// a toast (failure). No-op when no flow is in flight.
    fn poll_oauth_flow(&mut self) {
        if self.oauth_flow_rx.is_none() {
            return;
        }
        let Some(rx) = &self.oauth_flow_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(tokens)) => {
                if let Auth::OAuth2(ref mut s) = self.editing_auth {
                    s.access_token = tokens.access_token;
                    s.refresh_token = tokens.refresh_token;
                    s.expires_at = tokens.expires_at;
                }
                let auth = self.editing_auth.clone();
                self.update_current_request(|r| r.auth = auth);
                self.oauth_flow_rx = None;
                self.oauth_flow_status = None;
                self.show_toast("OAuth token obtained");
            }
            Ok(Err(msg)) => {
                self.oauth_flow_rx = None;
                self.oauth_flow_status = None;
                self.show_toast(format!("OAuth failed: {}", msg));
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                // Flow still running — bump status if we haven't yet.
                if self.oauth_flow_status.as_deref() == Some("Opening browser…") {
                    self.oauth_flow_status = Some("Waiting for browser redirect…".to_string());
                }
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.oauth_flow_rx = None;
                self.oauth_flow_status = None;
            }
        }
    }

    /// Run the current request's extractors against the just-received
    /// response and write each result into the active environment. A
    /// toast summarizes how many values were captured so the user has
    /// feedback that chaining actually happened.
    /// Fold `Set-Cookie` response cookies into the active environment's
    /// jar (replacing matching name/domain/path entries, pruning
    /// expired ones). No-op if there's no active env — cookies are
    /// silently dropped. Most users want them per-env so we don't
    /// fall back to anything global.
    fn merge_cookies_into_env(&mut self, cookies: Vec<StoredCookie>) {
        if cookies.is_empty() {
            return;
        }
        let Some(env_id) = self.state.active_env_id.clone() else {
            return;
        };
        let Some(env) = self.state.environments.iter_mut().find(|e| e.id == env_id) else {
            return;
        };
        for c in cookies {
            cookies::upsert(&mut env.cookies, c);
        }
        cookies::prune(&mut env.cookies);
        self.save_state();
    }

    fn apply_response_extractors(&mut self) {
        let Some(req) = self.get_current_request() else {
            return;
        };
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

        if let Some(env) = self.state.environments.iter_mut().find(|e| e.id == env_id) {
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

    /// Run each enabled assertion against the latest response and
    /// store the outcome in `assertion_results` (parallel to
    /// `editing_assertions`). The Tests tab shows per-row badges for
    /// success; toast only when a failure/error needs attention.
    fn apply_response_assertions(&mut self) {
        if self.editing_assertions.is_empty() {
            self.assertion_results.clear();
            return;
        }
        let status = self.response_status.clone();
        let body = self.response_text.clone();
        let headers = self.response_headers.clone();

        self.assertion_results = self
            .editing_assertions
            .iter()
            .map(|a| {
                if !a.enabled {
                    return None;
                }
                // Skip the trailing ghost row the editor auto-appends so
                // users can type a new assertion — empty expression AND
                // empty expected means "not configured yet".
                if a.expression.trim().is_empty() && a.expected.trim().is_empty() {
                    return None;
                }
                Some(assertion::evaluate(a, &status, &body, &headers))
            })
            .collect();

        let (_pass, fail, err) = self
            .assertion_results
            .iter()
            .fold((0, 0, 0), |acc, r| match r {
                Some(AssertionResult::Pass) => (acc.0 + 1, acc.1, acc.2),
                Some(AssertionResult::Fail(_)) => (acc.0, acc.1 + 1, acc.2),
                Some(AssertionResult::Error(_)) => (acc.0, acc.1, acc.2 + 1),
                None => acc,
            });
        if fail > 0 || err > 0 {
            self.show_toast(match (fail, err) {
                (f, 0) => format!("{} assertion{} failed", f, if f == 1 { "" } else { "s" }),
                (0, e) => format!("{} assertion{} errored", e, if e == 1 { "" } else { "s" }),
                (f, e) => format!(
                    "{} assertion{} failed, {} errored",
                    f,
                    if f == 1 { "" } else { "s" },
                    e
                ),
            });
        }
    }

    fn push_history_entry(&mut self) {
        let Some(req) = self.get_current_request() else {
            return;
        };
        let mut preview = self.response_text.clone();
        if preview.len() > 256 {
            // Walk back to the nearest UTF-8 char boundary ≤ 256 — a
            // raw truncate(256) panics when byte 256 lands mid-codepoint
            // (e.g. a response body with an emoji straddling the cut).
            let mut cut = 256;
            while !preview.is_char_boundary(cut) {
                cut -= 1;
            }
            preview.truncate(cut);
            preview.push('…');
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
            self.editing_url = curl::build_full_url(&r.url, &r.query_params);
            self.editing_body = r.body;
            self.editing_name = r.name;
            self.editing_method = r.method.clone();
            self.editing_headers = r.headers;
            self.editing_params = r.query_params;
            self.editing_cookies = r.cookies;
            self.editing_body_ext = r.body_ext;
            self.editing_auth = r.auth;
            self.editing_extractors = r.extractors;
            self.editing_assertions = r.assertions;
            self.assertion_results = vec![None; self.editing_assertions.len()];
            self.editing_request_id_for_history = Some(r.id);
        }
        // Diff snapshot is request-scoped — don't leak a previous
        // request's body into a different request's Diff view.
        self.previous_response_text = None;
    }

    fn show_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), 2.5));
    }

    /// Kick off the one-click in-app updater. Writes a tiny bash
    /// runner script to `~/.../rusty-requester/update-runner.sh`,
    /// spawns it detached (`nohup … & disown`) so it survives
    /// `install.sh` killing our process partway through, and flips
    /// `updating_in_progress` so the "Updating…" modal takes over.
    ///
    /// The runner:
    /// 1. Appends a `### STARTED …` banner to `update.log`.
    /// 2. Pipes `install.sh | bash` into the log (stdout + stderr).
    /// 3. On success — writes `update.status` with `ok\n<version>` and
    ///    relaunches the app (`open -a RustyRequester` on macOS,
    ///    `rusty-requester &` on Linux).
    /// 4. On failure — writes `update.status` with `fail\n<reason>`.
    ///
    /// Caller is responsible for closing the existing update modal
    /// (we set `show_update_modal = false` so the live-tail modal
    /// owns the screen).
    pub(crate) fn spawn_update(&mut self) {
        if !in_app_update_supported() {
            self.show_toast("In-app update only supported on macOS and Linux");
            return;
        }
        // `update_available` stores the GitHub tag (e.g. "v0.18.8");
        // strip the leading "v" so downstream strings ("Updated to
        // vX.Y.Z", "X.Y.Z -> Y.Y.Z") don't double-up the prefix.
        let target_version = self
            .update_available
            .as_deref()
            .map(|s| s.trim_start_matches('v').to_string())
            .unwrap_or_default();
        if target_version.is_empty() {
            self.show_toast("No update available");
            return;
        }
        let log_path = update_log_path();
        let status_path = update_status_path();
        let runner_path = update_runner_path();
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Clear any stale status from a previous run; if the wrapper
        // dies before writing, we want "no status" rather than the
        // previous attempt's outcome lingering.
        let _ = std::fs::remove_file(&status_path);

        let relaunch_cmd = if cfg!(target_os = "macos") {
            "open -a RustyRequester >/dev/null 2>&1 || true".to_string()
        } else {
            // Linux: install.sh drops the binary at ~/.local/bin/
            // (which may or may not be on PATH). Try the absolute
            // path first, then the bare name.
            "( \"$HOME/.local/bin/rusty-requester\" >/dev/null 2>&1 & ) || \
             ( rusty-requester >/dev/null 2>&1 & ) || true"
                .to_string()
        };

        let current = env!("CARGO_PKG_VERSION");
        let script = format!(
            "#!/usr/bin/env bash\n\
             set -u\n\
             exec >> '{log}' 2>&1\n\
             echo ''\n\
             echo \"### STARTED $(date '+%Y-%m-%d %H:%M:%S')\"\n\
             echo \"### Updating rusty-requester v{current} -> {target}\"\n\
             echo ''\n\
             if curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | bash; then\n\
                 echo ''\n\
                 echo \"### DONE_OK $(date '+%Y-%m-%d %H:%M:%S')\"\n\
                 printf 'ok\\n{target}\\n' > '{status}'\n\
                 sleep 1\n\
                 {relaunch}\n\
             else\n\
                 echo ''\n\
                 echo \"### DONE_FAIL $(date '+%Y-%m-%d %H:%M:%S')\"\n\
                 printf 'fail\\ninstall.sh returned non-zero — see log for details\\n' > '{status}'\n\
             fi\n",
            log = log_path.display(),
            status = status_path.display(),
            current = current,
            target = target_version,
            relaunch = relaunch_cmd,
        );

        if let Err(e) = std::fs::write(&runner_path, &script) {
            self.show_toast(format!("Could not write update runner: {}", e));
            return;
        }
        // Make the runner executable (best-effort on Unix).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&runner_path) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&runner_path, perms);
            }
        }

        // Spawn detached via `sh -c "nohup bash … & disown"` — the
        // outer `sh` exits immediately, the bash process gets
        // reparented to init and survives our death.
        let detach_cmd = format!(
            "nohup bash '{}' </dev/null >/dev/null 2>&1 & disown",
            runner_path.display()
        );
        match std::process::Command::new("sh")
            .arg("-c")
            .arg(&detach_cmd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(_) => {
                self.updating_in_progress = true;
                self.show_update_modal = false;
                self.update_log_tail.clear();
                self.update_log_last_poll = 0.0;
            }
            Err(e) => {
                self.show_toast(format!("Could not start updater: {}", e));
            }
        }
    }

    /// Tail `update.log` while the in-app updater is running. Throttled
    /// to ~3 reads/sec so we're not blocking the UI thread on FS
    /// syscalls each frame. Also watches `update.status` — if the
    /// wrapper finishes before `install.sh` can kill us (e.g. update
    /// failed early), we surface the result inline in the modal
    /// instead of waiting for a relaunch that won't happen.
    pub(crate) fn poll_update_progress(&mut self, ctx: &egui::Context) {
        if !self.updating_in_progress {
            return;
        }
        let now = ctx.input(|i| i.time);
        if now - self.update_log_last_poll >= 0.3 {
            self.update_log_last_poll = now;
            self.update_log_tail = read_log_tail(&update_log_path(), 4096);
            // If the wrapper already wrote a status file, the update
            // is done (success or failure) — flip out of "in progress"
            // so the modal switches to the post-update banner. On
            // success this branch is rarely hit because install.sh
            // usually kills us first, but on early failure (no
            // network, curl 404) the wrapper completes while we're
            // still alive.
            if let Some(outcome) = consume_update_status() {
                self.updating_in_progress = false;
                self.post_update_notice = Some(outcome);
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(300));
    }

    /// Activate collection-overview mode for `folder_id`. Shows the
    /// folder's homepage (title / stats / description) in the central
    /// panel instead of the request editor. Clears any selected
    /// request so the editor doesn't fight for space.
    pub(crate) fn open_folder_overview(&mut self, folder_id: &str) {
        let desc = find_folder_desc(&self.state.folders, folder_id).unwrap_or_default();
        self.viewing_folder_id = Some(folder_id.to_string());
        self.editing_folder_desc = desc;
        self.selected_request_id = None;
    }

    fn open_request(&mut self, folder_path: Vec<String>, request_id: String) {
        // Any request activation leaves the collection overview mode.
        self.viewing_folder_id = None;

        let outgoing_id = self.selected_request_id.clone();

        if let Some(existing) = self
            .state
            .open_tabs
            .iter()
            .position(|t| t.request_id == request_id)
        {
            let tab = self.state.open_tabs[existing].clone();
            self.selected_folder_path = tab.folder_path;
            self.selected_request_id = Some(tab.request_id);
        } else {
            self.state.open_tabs.push(OpenTab {
                folder_path: folder_path.clone(),
                request_id: request_id.clone(),
                pinned: false,
            });
            self.selected_folder_path = folder_path;
            self.selected_request_id = Some(request_id);
        }
        self.state.active_tab_id = self.selected_request_id.clone();

        // Re-clicking the already-active tab is a no-op — don't wipe
        // the live response by restoring from an empty cache. Only
        // stash/restore when we actually switched requests.
        let actually_switched = outgoing_id.as_deref() != self.selected_request_id.as_deref();
        if !actually_switched {
            self.load_request_for_editing();
            return;
        }

        if let Some(prev) = outgoing_id {
            self.stash_response_for(&prev);
        }
        self.load_request_for_editing();
        let new_id = self.selected_request_id.clone();
        self.restore_response_for(new_id.as_deref());
    }

    /// Copy current live response state into `response_cache` under the
    /// given request id.
    fn stash_response_for(&mut self, request_id: &str) {
        // Don't cache a blank/loading slot — an empty entry does
        // nothing useful and just gets overwritten on next restore.
        if self.response_text.is_empty() && self.response_status.is_empty() {
            self.response_cache.remove(request_id);
            return;
        }
        let snap = CachedResponse {
            text: self.response_text.clone(),
            status: self.response_status.clone(),
            time: self.response_time.clone(),
            headers: self.response_headers.clone(),
            headers_bytes: self.response_headers_bytes,
            body_bytes: self.response_body_bytes,
            prepare_ms: self.response_prepare_ms,
            waiting_ms: self.response_waiting_ms,
            download_ms: self.response_download_ms,
            total_ms: self.response_total_ms,
            previous_text: self.previous_response_text.clone(),
            streaming_events: self.streaming_events.clone(),
            assertion_results: self.assertion_results.clone(),
        };
        self.response_cache.insert(request_id.to_string(), snap);
    }

    /// Restore live response state from `response_cache` for the given
    /// request id, or clear everything if there's no cached entry.
    fn restore_response_for(&mut self, request_id: Option<&str>) {
        let Some(id) = request_id else {
            self.clear_response_fields();
            return;
        };
        if let Some(snap) = self.response_cache.get(id).cloned() {
            self.response_text = snap.text;
            self.folded_response_lines.clear();
            self.response_status = snap.status;
            self.response_time = snap.time;
            self.response_headers = snap.headers;
            self.response_headers_bytes = snap.headers_bytes;
            self.response_body_bytes = snap.body_bytes;
            self.response_prepare_ms = snap.prepare_ms;
            self.response_waiting_ms = snap.waiting_ms;
            self.response_download_ms = snap.download_ms;
            self.response_total_ms = snap.total_ms;
            self.previous_response_text = snap.previous_text;
            self.streaming_events = snap.streaming_events;
            self.assertion_results = snap.assertion_results;
        } else {
            self.clear_response_fields();
        }
    }

    fn clear_response_fields(&mut self) {
        self.response_text.clear();
        self.folded_response_lines.clear();
        self.response_status.clear();
        self.response_time.clear();
        self.response_headers.clear();
        self.response_headers_bytes = 0;
        self.response_body_bytes = 0;
        self.response_prepare_ms = 0;
        self.response_waiting_ms = 0;
        self.response_download_ms = 0;
        self.response_total_ms = 0;
        self.previous_response_text = None;
        self.streaming_events.clear();
        self.assertion_results.clear();
    }

    fn close_tab(&mut self, idx: usize) {
        if idx >= self.state.open_tabs.len() {
            return;
        }
        // Pinned tabs are sticky — user has to unpin or use the X
        // button directly (X still works; this only guards ⌘W and
        // menu-driven closes).
        if self.state.open_tabs[idx].pinned {
            self.show_toast("Tab is pinned — unpin to close");
            return;
        }
        // If it's a draft with content, show the "Save changes?" modal
        // instead of discarding immediately.
        if let Some(tab) = self.state.open_tabs.get(idx) {
            if tab.folder_path.is_empty() {
                let has_content = self
                    .state
                    .drafts
                    .iter()
                    .find(|d| d.id == tab.request_id)
                    .map(|d| !d.url.is_empty() || !d.body.is_empty() || !d.headers.is_empty())
                    .unwrap_or(false);
                if has_content {
                    self.confirm_close_draft_idx = Some(idx);
                    return;
                }
            }
        }
        self.close_tab_force(idx);
    }

    /// Close a tab without any confirmation prompt.
    fn close_tab_force(&mut self, idx: usize) {
        if idx >= self.state.open_tabs.len() {
            return;
        }
        let closing = self.state.open_tabs.remove(idx);
        // If it was a draft, discard the draft entirely.
        if closing.folder_path.is_empty() {
            self.state.drafts.retain(|d| d.id != closing.request_id);
        }
        let was_active = self.selected_request_id.as_deref() == Some(closing.request_id.as_str());
        // Closed tab's cached response is no longer reachable — drop
        // it so the cache doesn't leak memory across long sessions.
        self.response_cache.remove(&closing.request_id);
        if was_active {
            if self.state.open_tabs.is_empty() {
                self.clear_selection();
            } else {
                let new_idx = idx.min(self.state.open_tabs.len() - 1);
                let tab = self.state.open_tabs[new_idx].clone();
                self.selected_folder_path = tab.folder_path;
                self.selected_request_id = Some(tab.request_id);
                self.load_request_for_editing();
                let new_id = self.selected_request_id.clone();
                self.restore_response_for(new_id.as_deref());
            }
        }
    }

    fn close_other_tabs(&mut self, keep_idx: usize) {
        if keep_idx >= self.state.open_tabs.len() {
            return;
        }
        let keep_id = self.state.open_tabs[keep_idx].request_id.clone();
        // Collect tabs to drop: everything except `keep_idx` and
        // anything pinned. Pinned tabs are preserved by design.
        let to_drop_draft_ids: Vec<String> = self
            .state
            .open_tabs
            .iter()
            .enumerate()
            .filter(|(i, t)| *i != keep_idx && !t.pinned && t.folder_path.is_empty())
            .map(|(_, t)| t.request_id.clone())
            .collect();
        self.state
            .drafts
            .retain(|d| !to_drop_draft_ids.contains(&d.id));
        self.state
            .open_tabs
            .retain(|t| t.request_id == keep_id || t.pinned);
        // Make the kept tab active (it may have moved after retain).
        if let Some(keep) = self
            .state
            .open_tabs
            .iter()
            .find(|t| t.request_id == keep_id)
            .cloned()
        {
            self.selected_folder_path = keep.folder_path;
            self.selected_request_id = Some(keep.request_id);
            self.load_request_for_editing();
        }
    }

    fn close_all_tabs(&mut self) {
        // Discard drafts whose tabs are about to close — pinned tabs
        // (and their drafts) are preserved.
        let draft_ids: Vec<String> = self
            .state
            .open_tabs
            .iter()
            .filter(|t| !t.pinned && t.folder_path.is_empty())
            .map(|t| t.request_id.clone())
            .collect();
        self.state.drafts.retain(|d| !draft_ids.contains(&d.id));
        self.state.open_tabs.retain(|t| t.pinned);
        if self.state.open_tabs.is_empty() {
            self.clear_selection();
        } else {
            // Activate the first remaining (pinned) tab.
            let first = self.state.open_tabs[0].clone();
            self.selected_folder_path = first.folder_path;
            self.selected_request_id = Some(first.request_id);
            self.load_request_for_editing();
        }
    }

    /// Flattened list of `(folder_path, request_id)` for every
    /// request in every collection, depth-first. Used by ↑/↓ arrow
    /// navigation so the user can step through requests without
    /// mouse-clicking. Respects the current sidebar search filter.
    fn flat_request_list(&self) -> Vec<(Vec<String>, String)> {
        let q = self.search_query.to_lowercase();
        let mut out = Vec::new();
        for folder in &self.state.folders {
            collect_flat_requests(folder, &mut Vec::new(), &mut out, &q);
        }
        out
    }

    /// Move sidebar selection to the next (`down = true`) or previous
    /// request in the flat list, wrapping at the ends. No-op when the
    /// list is empty.
    fn arrow_navigate_sidebar(&mut self, down: bool) {
        let flat = self.flat_request_list();
        if flat.is_empty() {
            return;
        }
        let current_idx = self.selected_request_id.as_ref().and_then(|id| {
            flat.iter()
                .position(|(p, rid)| rid == id && *p == self.selected_folder_path)
        });
        let next_idx = match current_idx {
            None => {
                if down {
                    0
                } else {
                    flat.len() - 1
                }
            }
            Some(i) => {
                if down {
                    (i + 1) % flat.len()
                } else {
                    (i + flat.len() - 1) % flat.len()
                }
            }
        };
        let (path, id) = flat[next_idx].clone();
        self.open_request(path, id);
    }

    /// Duplicate the request at `idx` as a new draft tab. For saved
    /// requests, a fresh draft copy is added to `state.drafts` with a
    /// new UUID; for drafts, the source draft itself is cloned.
    fn duplicate_tab(&mut self, idx: usize) {
        let Some(tab) = self.state.open_tabs.get(idx).cloned() else {
            return;
        };
        let src: Option<Request> = if tab.folder_path.is_empty() {
            self.state
                .drafts
                .iter()
                .find(|d| d.id == tab.request_id)
                .cloned()
        } else {
            let path = tab.folder_path.clone();
            let mut req: Option<Request> = None;
            if let Some(folder) = self.folder_at_path_mut(&path) {
                req = folder
                    .requests
                    .iter()
                    .find(|r| r.id == tab.request_id)
                    .cloned();
            }
            req
        };
        let Some(mut req) = src else { return };
        req.id = Uuid::new_v4().to_string();
        if !req.name.is_empty() {
            req.name = format!("{} (copy)", req.name);
        }
        let new_id = req.id.clone();
        self.state.drafts.push(req);
        self.state.open_tabs.push(OpenTab {
            folder_path: vec![],
            request_id: new_id.clone(),
            pinned: false,
        });
        self.selected_folder_path = vec![];
        self.selected_request_id = Some(new_id);
        self.load_request_for_editing();
        self.save_state();
        self.show_toast("Tab duplicated");
    }

    /// Dispatch a `PaletteAction` — the single entry point the
    /// actions palette uses on Enter/click. Most branches reuse
    /// existing helpers; a few translate a menu-dispatch event into
    /// the equivalent state flip.
    fn run_action(&mut self, action: actions::PaletteAction) {
        use actions::PaletteAction as A;
        match action {
            A::NewRequest => self.new_draft_request(),
            A::DuplicateTab => {
                if let Some(idx) = self.active_tab_index() {
                    self.duplicate_tab(idx);
                }
            }
            A::CloseTab => {
                if let Some(idx) = self.active_tab_index() {
                    self.close_tab(idx);
                }
            }
            A::TogglePin => {
                if let Some(idx) = self.active_tab_index() {
                    if let Some(tab) = self.state.open_tabs.get_mut(idx) {
                        tab.pinned = !tab.pinned;
                        self.save_state();
                    }
                }
            }
            A::SaveDraft => {
                if let Some(idx) = self.active_tab_index() {
                    if self.state.open_tabs[idx].folder_path.is_empty() {
                        self.begin_save_draft(idx);
                    } else {
                        self.show_toast("Already saved");
                    }
                }
            }
            A::CopyAsCurl => {
                if let Some(req) = self.get_current_request() {
                    let s = curl::to_curl_redacted(&req);
                    // egui's Context::copy_text is frame-scoped; show a
                    // toast and use arboard-like behavior via ctx next
                    // frame would be ideal, but render_toast is the
                    // immediate confirmation users expect.
                    // Use egui's clipboard by writing to the context.
                    self.pending_clipboard = Some(s);
                    self.show_toast("Copied redacted cURL to clipboard");
                }
            }
            A::ToggleSnippetPanel => self.show_snippet_panel = !self.show_snippet_panel,
            A::OpenEnvironments => {
                self.show_env_modal = true;
                if self.selected_env_for_edit.is_none() {
                    self.selected_env_for_edit =
                        self.state.environments.first().map(|e| e.id.clone());
                }
            }
            A::OpenSettings => {
                self.editing_settings = self.state.settings.clone();
                self.show_settings_modal = true;
            }
            A::PasteCurl => {
                self.show_paste_modal = true;
                self.paste_curl_text.clear();
                self.paste_error.clear();
            }
            A::OpenCollectionRunner => self.show_runner_modal = true,
            A::OpenSync => self.open_sync_or_settings(),
            A::ImportCollection => self.pending_import = true,
            A::CreateBackup => self.create_workspace_backup_now(),
            A::OpenBackups => self.show_backup_modal = true,
            A::ExportJson => self.pending_export_json = true,
            A::ExportYaml => self.pending_export_yaml = true,
            A::ClearHistory => {
                self.state.history.clear();
                self.save_state();
                self.show_toast("History cleared");
            }
            A::ToggleSidebarHistory => {
                self.sidebar_view = match self.sidebar_view {
                    SidebarView::Collections => SidebarView::History,
                    SidebarView::History => SidebarView::Collections,
                };
            }
            A::ShowAbout => self.show_about_modal = true,
        }
    }

    /// Tab index of the active tab, or `None` if no request is open.
    fn active_tab_index(&self) -> Option<usize> {
        let req_id = self.selected_request_id.as_ref()?;
        self.state
            .open_tabs
            .iter()
            .position(|t| &t.request_id == req_id)
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

    pub(crate) fn open_sync_or_settings(&mut self) {
        if self.state.settings.workspace_sync_enabled {
            self.show_sync_modal = true;
        } else {
            self.editing_settings = self.state.settings.clone();
            self.show_settings_modal = true;
            self.show_toast("Enable Workspace Sync in Settings first");
        }
    }

    pub(crate) fn open_collection_settings(&mut self, folder_id: &str) {
        if crate::find_folder_by_id(&self.state.folders, folder_id).is_none() {
            self.show_toast("Collection not found");
            return;
        }
        self.collection_settings_folder_id = Some(folder_id.to_string());
        self.collection_git_status.clear();
        self.show_collection_settings_modal = true;
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
        let before = self.get_saved_request_by_id(request_id);
        if go(&mut self.state.folders, request_id, &new_name) {
            if let (Some(before), Some(after)) = (before, self.get_saved_request_by_id(request_id))
            {
                self.track_saved_request_change(request_id, before, &after);
            }
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
                if let Err(e) = backup::create_backup(&self.storage_path) {
                    self.show_toast(format!("Import aborted: backup failed: {}", e));
                    return;
                }
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

    pub(crate) fn create_workspace_backup_now(&mut self) {
        match backup::create_backup(&self.storage_path) {
            Ok(Some(path)) => self.show_toast(format!(
                "Backup created: {}",
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("workspace backup")
            )),
            Ok(None) => self.show_toast("No saved workspace yet"),
            Err(e) => self.show_toast(format!("Backup failed: {}", e)),
        }
    }

    pub(crate) fn restore_workspace_backup_now(&mut self, backup_path: &Path) {
        match backup::restore_backup(&self.storage_path, backup_path) {
            Ok(_) => match Self::load_state(&self.storage_path) {
                LoadOutcome::Ok(state) => {
                    self.state = *state;
                    if let Some(tab) = restored_active_tab(
                        &self.state.open_tabs,
                        self.state.active_tab_id.as_deref(),
                    ) {
                        self.selected_folder_path = tab.folder_path;
                        self.selected_request_id = Some(tab.request_id.clone());
                        self.state.active_tab_id = Some(tab.request_id);
                        self.load_request_for_editing();
                    } else {
                        self.selected_folder_path.clear();
                        self.selected_request_id = None;
                    }
                    self.confirm_restore_backup_path = None;
                    self.show_backup_modal = false;
                    self.show_toast("Workspace restored");
                }
                LoadOutcome::Fresh => self.show_toast("Restore produced an empty workspace"),
                LoadOutcome::Corrupted { error, .. } => {
                    self.show_toast(format!("Restore failed: {}", error));
                }
            },
            Err(e) => self.show_toast(format!("Restore failed: {}", e)),
        }
    }

    fn request_export_all(&mut self, format: io::Format) {
        let findings = secret_scanner::scan_folders(&self.state.folders);
        if findings.is_empty() {
            self.pending_export_decision = Some(ExportDecision {
                format,
                redacted: false,
            });
        } else {
            self.export_secret_warning = Some(ExportSecretWarning { format, findings });
        }
    }

    fn do_export_all(&mut self, format: io::Format, redacted: bool) {
        let (ext, label) = match format {
            io::Format::Json => ("json", "JSON"),
            io::Format::Yaml => ("yaml", "YAML"),
        };
        let suggested = if redacted {
            format!("rusty-requester.redacted.{}", ext)
        } else {
            format!("rusty-requester.{}", ext)
        };
        let path = rfd::FileDialog::new()
            .add_filter(label, &[ext])
            .set_file_name(suggested)
            .save_file();
        let Some(path) = path else { return };
        let export_result = if redacted {
            io::export_string_redacted(&self.state.folders, format)
        } else {
            io::export_string(&self.state.folders, format)
        };
        match export_result {
            Ok(content) => match std::fs::write(&path, content) {
                Ok(_) if redacted => self.show_toast(format!("Exported redacted {}", label)),
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
        let findings = secret_scanner::scan_folders(std::slice::from_ref(&folder));
        let redacted = !findings.is_empty();
        let export_result = if redacted {
            io::export_string_redacted(std::slice::from_ref(&folder), format)
        } else {
            io::export_string(std::slice::from_ref(&folder), format)
        };
        match export_result {
            Ok(content) => match std::fs::write(&path, content) {
                Ok(_) if redacted => {
                    self.show_toast(format!("Exported '{}' as redacted {}", folder.name, label))
                }
                Ok(_) => self.show_toast(format!("Exported '{}' as {}", folder.name, label)),
                Err(e) => self.show_toast(format!("Write failed: {}", e)),
            },
            Err(e) => self.show_toast(format!("Export failed: {}", e)),
        }
    }
}

#[cfg(target_os = "macos")]
impl ApiClient {
    /// Drain macOS NSMenu events and map each item ID to an action.
    fn dispatch_macos_menu_events(&mut self) {
        use macos_menu as m;
        for id in m::drain_events() {
            match id.as_str() {
                m::MENU_NEW_REQUEST => self.new_draft_request(),
                m::MENU_NEW_COLLECTION => {
                    self.state.folders.push(Folder {
                        id: Uuid::new_v4().to_string(),
                        name: format!("Collection {}", self.state.folders.len() + 1),
                        requests: vec![],
                        subfolders: vec![],
                        description: String::new(),
                        sync: SyncConfig::default(),
                    });
                    self.save_state();
                }
                m::MENU_CLOSE_TAB => {
                    if let Some(req_id) = &self.selected_request_id {
                        let idx = self
                            .state
                            .open_tabs
                            .iter()
                            .position(|t| &t.request_id == req_id);
                        if let Some(i) = idx {
                            self.close_tab(i);
                        }
                    }
                }
                m::MENU_IMPORT => self.pending_import = true,
                m::MENU_PASTE_CURL => {
                    self.show_paste_modal = true;
                    self.paste_curl_text.clear();
                    self.paste_error.clear();
                }
                m::MENU_CREATE_BACKUP => self.create_workspace_backup_now(),
                m::MENU_BACKUPS => self.show_backup_modal = true,
                m::MENU_WORKSPACE_SYNC => self.open_sync_or_settings(),
                m::MENU_EXPORT_JSON => self.pending_export_json = true,
                m::MENU_EXPORT_YAML => self.pending_export_yaml = true,
                m::MENU_TOGGLE_SNIPPET => self.show_snippet_panel = !self.show_snippet_panel,
                m::MENU_COMMAND_PALETTE => {
                    self.show_command_palette = true;
                    self.palette_query.clear();
                    self.palette_selected = 0;
                    self.palette_focus_pending = true;
                }
                m::MENU_SEND => self.send_request(),
                m::MENU_COLLECTION_RUNNER => self.show_runner_modal = true,
                m::MENU_SETTINGS => {
                    self.editing_settings = self.state.settings.clone();
                    self.show_settings_modal = true;
                }
                m::MENU_ENVIRONMENTS => {
                    self.show_env_modal = true;
                    if self.selected_env_for_edit.is_none() {
                        self.selected_env_for_edit =
                            self.state.environments.first().map(|e| e.id.clone());
                    }
                }
                m::MENU_ABOUT => {
                    eprintln!("[menu] MENU_ABOUT fired — opening custom About modal");
                    self.show_about_modal = true;
                }
                m::MENU_GITHUB | m::MENU_REPORT_ISSUE => {
                    let url = if id == m::MENU_GITHUB {
                        "https://github.com/chud-lori/rusty-requester"
                    } else {
                        "https://github.com/chud-lori/rusty-requester/issues"
                    };
                    if let Err(e) = webbrowser_open(url) {
                        eprintln!("[menu] open url failed: {}", e);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Open a URL in the user's default browser. Used by Help → GitHub,
/// Report an issue, and the OAuth "Get New Token" flow. Shells out
/// to the platform's native URL handler instead of pulling in the
/// `webbrowser` crate.
fn webbrowser_open(url: &str) -> Result<(), std::io::Error> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
    }
    #[cfg(target_os = "windows")]
    {
        // `cmd /c start ""` — the empty "" is the window title
        // placeholder; without it `start` consumes the URL as the
        // title and the actual URL becomes the command.
        std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .spawn()
            .map(|_| ())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!(
                "no browser-opener known for this platform; URL was: {}",
                url
            ),
        ))
    }
}

impl eframe::App for ApiClient {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Fire the startup warning (if any) exactly once. Using a
        // toast means the user sees it but doesn't need to dismiss a
        // modal just to start working.
        if let Some(msg) = self.startup_warning.take() {
            self.show_toast(msg);
        }

        // If the previous run kicked off an in-app update, the wrapper
        // script will have written `update.status`. Consume it once
        // (file is deleted) and surface the result via the
        // post-update banner. Skipped on subsequent frames because
        // the file is gone.
        if self.post_update_notice.is_none() && !self.updating_in_progress {
            if let Some(outcome) = consume_update_status() {
                self.post_update_notice = Some(outcome);
            }
        }

        self.poll_update_progress(ctx);
        self.poll_oauth_flow();

        // Drain the update-check channel. Only fires once (the tokio
        // task sends at most one message then the sender drops); after
        // that the receiver disconnects and `try_recv` is a ~no-op until
        // we clear `update_check_rx`. We also feed the result into
        // `manual_update_check` so the Settings inline status block can
        // show "Checking… → Up to date / Update available" without the
        // user having to close the modal.
        if let Some(rx) = &self.update_check_rx {
            match rx.try_recv() {
                Ok(message) => {
                    match message {
                        UpdateCheckMessage::Available(new_version) => {
                            self.update_available = Some(new_version);
                            if matches!(self.manual_update_check, UpdateCheckOutcome::Checking) {
                                self.manual_update_check = UpdateCheckOutcome::Idle;
                            }
                        }
                        UpdateCheckMessage::Current => {
                            if matches!(self.manual_update_check, UpdateCheckOutcome::Checking) {
                                self.manual_update_check = UpdateCheckOutcome::NoUpdate;
                            }
                        }
                        UpdateCheckMessage::Failed(reason) => {
                            if matches!(self.manual_update_check, UpdateCheckOutcome::Checking) {
                                self.manual_update_check = UpdateCheckOutcome::Failed(reason);
                            }
                        }
                    }
                    self.update_check_rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if matches!(self.manual_update_check, UpdateCheckOutcome::Checking) {
                        self.manual_update_check =
                            UpdateCheckOutcome::Failed("Update check stopped".to_string());
                    }
                    self.update_check_rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }

        // Install the macOS menu bar on the first frame (after winit
        // has wired up NSApp). See `macos_menu_installed` doc comment
        // for why this can't run in `main()`.
        #[cfg(target_os = "macos")]
        if !self.macos_menu_installed {
            self.macos_menu_installed = true;
            macos_menu::install();
            eprintln!("[menu] macOS NSMenu installed from first update()");
            // Merge the title bar into the window chrome (fullSizeContentView
            // + titlebarAppearsTransparent + titleVisibilityHidden). Doing
            // this AFTER the window is fully instantiated — setting it via
            // `ViewportBuilder` in 0.29 was creating a painted-over dead
            // zone; the raw `objc` path works because we're setting the
            // NSWindow flags directly once the winit-created window exists.
            if let Err(e) = icon::set_macos_titlebar_transparent() {
                eprintln!("[titlebar] could not merge title bar: {}", e);
            }
        }

        // Dispatch any macOS system-menu selections made since the last
        // frame. On Linux/Windows this is a no-op — the menu lives in
        // a `TopBottomPanel` inside the window instead.
        #[cfg(target_os = "macos")]
        self.dispatch_macos_menu_events();

        // Handle deferred file-dialog actions from the previous frame.
        // These would otherwise block the main thread before the menu
        // popup that triggered them had a chance to close.
        if self.pending_import {
            self.pending_import = false;
            self.do_import_file();
        }
        if self.pending_export_json {
            self.pending_export_json = false;
            self.request_export_all(io::Format::Json);
        }
        if self.pending_export_yaml {
            self.pending_export_yaml = false;
            self.request_export_all(io::Format::Yaml);
        }
        if let Some(decision) = self.pending_export_decision.take() {
            self.do_export_all(decision.format, decision.redacted);
        }
        if let Some(text) = self.pending_clipboard.take() {
            ctx.copy_text(text);
        }

        let (
            cmd_enter,
            cmd_k,
            cmd_p,
            cmd_shift_p,
            cmd_s,
            cmd_n,
            cmd_w,
            cmd_d,
            cmd_f,
            f2,
            arrow_up,
            arrow_down,
        ) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(egui::Key::Enter),
                i.modifiers.command && i.key_pressed(egui::Key::K),
                i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::P),
                i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::P),
                i.modifiers.command && i.key_pressed(egui::Key::S),
                i.modifiers.command && i.key_pressed(egui::Key::N),
                i.modifiers.command && i.key_pressed(egui::Key::W),
                i.modifiers.command && i.key_pressed(egui::Key::D),
                // egui's `modifiers.command` maps to Cmd on macOS and
                // Ctrl on Linux / Windows, so one bind covers all three.
                i.modifiers.command && i.key_pressed(egui::Key::F),
                i.key_pressed(egui::Key::F2),
                !i.modifiers.command && !i.modifiers.alt && i.key_pressed(egui::Key::ArrowUp),
                !i.modifiers.command && !i.modifiers.alt && i.key_pressed(egui::Key::ArrowDown),
            )
        });
        // Arrow navigation is gated on "no widget wants keyboard
        // input" — so typing in a TextEdit, Body editor, or search
        // box isn't hijacked. When nothing's focused, ↑/↓ step
        // through the flat request list in the sidebar.
        let can_arrow_nav = !ctx.wants_keyboard_input()
            && !self.show_command_palette
            && !self.show_actions_palette
            && !self.show_env_modal
            && !self.show_settings_modal
            && !self.show_paste_modal
            && !self.show_runner_modal
            && !self.show_backup_modal
            && !self.show_sync_modal
            && !self.show_about_modal
            && self.export_secret_warning.is_none()
            && !self.save_draft_open
            && self.confirm_close_draft_idx.is_none()
            && self.renaming_request_id.is_none()
            && self.renaming_folder_id.is_none();
        if can_arrow_nav && arrow_up {
            self.arrow_navigate_sidebar(false);
        }
        if can_arrow_nav && arrow_down {
            self.arrow_navigate_sidebar(true);
        }
        if cmd_enter && self.selected_request_id.is_some() && !self.is_loading {
            self.send_request();
        }
        // Cmd/Ctrl+F — Find in response body. Opens the inline search
        // bar, switches to the Body tab if we're on Headers, and
        // focuses the input so the user can type immediately. Pressing
        // it again while already open just re-focuses the input
        // (convenient if focus drifted elsewhere). Escape closes, same
        // as the magnifying-glass button.
        if cmd_f {
            self.response_tab = ResponseTab::Body;
            self.body_search_visible = true;
            self.body_search_focus_pending = true;
        }
        // Cmd+N — new request (cross-platform; macOS also fires via menu accelerator).
        if cmd_n {
            self.new_draft_request();
        }
        // Cmd+W — close active tab.
        if cmd_w {
            if let Some(req_id) = &self.selected_request_id {
                let idx = self
                    .state
                    .open_tabs
                    .iter()
                    .position(|t| &t.request_id == req_id);
                if let Some(i) = idx {
                    self.close_tab(i);
                }
            }
        }
        // Cmd+D — duplicate active tab.
        if cmd_d {
            if let Some(req_id) = &self.selected_request_id {
                let idx = self
                    .state
                    .open_tabs
                    .iter()
                    .position(|t| &t.request_id == req_id);
                if let Some(i) = idx {
                    self.duplicate_tab(i);
                }
            }
        }
        if cmd_k {
            self.focus_search_next_frame = true;
        }
        if cmd_p {
            self.show_command_palette = true;
            self.palette_query.clear();
            self.palette_selected = 0;
            self.palette_focus_pending = true;
        }
        if cmd_shift_p {
            self.show_actions_palette = true;
            self.actions_palette_query.clear();
            self.actions_palette_selected = 0;
            self.actions_palette_focus_pending = true;
        }
        // Cmd/Ctrl+S — drafts open the Save-draft modal; saved requests
        // clear their dirty indicator after writing the current state.
        if cmd_s {
            if let Some(req_id) = self.selected_request_id.clone() {
                if self.selected_folder_path.is_empty() {
                    if let Some(idx) = self
                        .state
                        .open_tabs
                        .iter()
                        .position(|t| t.is_draft() && t.request_id == req_id)
                    {
                        self.begin_save_draft(idx);
                    }
                } else {
                    self.mark_saved_request_clean(&req_id);
                    self.save_state();
                    self.show_toast("Saved");
                }
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
                self.app_icon =
                    Some(ctx.load_texture("app_icon", ci, egui::TextureOptions::LINEAR));
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

        // Poll the in-flight send. `try_recv` is non-blocking; the
        // tokio task sends RequestUpdate messages (Progress during SSE
        // streaming, Final at the end). We drain all pending messages
        // per frame so the latest state always wins.
        while let Some(f) = &self.request_in_flight {
            match f.rx.try_recv() {
                Ok(net::RequestUpdate::Progress {
                    snapshot,
                    new_events,
                }) => {
                    self.apply_response_snapshot(&snapshot);
                    self.streaming_events.extend(new_events);
                    // Keep animating; don't clear is_loading — the
                    // stream is still live.
                    ctx.request_repaint();
                }
                Ok(net::RequestUpdate::Final(r)) => {
                    self.apply_response_snapshot(&r);
                    self.is_loading = false;
                    let cookies_to_merge = r.set_cookies.clone();
                    self.request_in_flight = None;
                    self.merge_cookies_into_env(cookies_to_merge);
                    self.push_history_entry();
                    self.apply_response_extractors();
                    self.apply_response_assertions();
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Still in flight — keep the UI animating.
                    ctx.request_repaint();
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Sender dropped without a Final — task was
                    // aborted (Cancel) or panicked. Clean up
                    // silently; status was already set by cancel.
                    self.request_in_flight = None;
                    self.is_loading = false;
                    break;
                }
            }
        }

        while let Some(f) = &self.runner_in_flight {
            match f.rx.try_recv() {
                Ok(runner::CollectionRunEvent::RequestFinished(progress)) => {
                    self.runner_results
                        .push(runner_detail::row_from_progress(&progress));
                    if self.runner_selected_result.is_none() {
                        self.runner_selected_result = Some(0);
                    }
                    self.runner_status = format!(
                        "Running: {} / {} request runs complete.",
                        progress.completed_requests, progress.total_requests
                    );
                    ctx.request_repaint();
                }
                Ok(runner::CollectionRunEvent::Finished(result)) => {
                    self.runner_results = runner_detail::rows_from_result(&result);
                    if self
                        .runner_selected_result
                        .is_some_and(|index| index >= self.runner_results.len())
                    {
                        self.runner_selected_result = None;
                    }
                    self.runner_status = format!(
                        "Finished: {} request run{}, {} passed, {} failed, {} errored.",
                        result.total_requests,
                        if result.total_requests == 1 { "" } else { "s" },
                        result.passed_assertions,
                        result.failed_assertions,
                        result.errored_assertions
                    );
                    self.runner_in_flight = None;
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint();
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.runner_status = "Runner stopped before returning results.".to_string();
                    self.runner_in_flight = None;
                    break;
                }
            }
        }

        self.poll_sync_job(ctx);

        let display_theme = self.effective_theme();
        theme::apply_style(ctx, display_theme);

        // Paint the ENTIRE window background before any panels render.
        // egui leaves a ~50-60 logical-pixel "gutter" between
        // `SidePanel::left` and `CentralPanel` (for separator/resize
        // reservation) that neither panel's fill covers — without this
        // base-layer paint, that gutter surfaces as egui's default
        // near-black. Using the background layer ensures panels draw on top
        // of our fill, not the other way around. Theme-aware so light
        // mode doesn't get a black gutter strip.
        {
            let screen_rect = ctx.screen_rect();
            let bg = theme::palette_for(display_theme).bg;
            ctx.layer_painter(egui::LayerId::background()).rect_filled(
                screen_rect,
                egui::Rounding::ZERO,
                bg,
            );
        }

        // On macOS the menu lives in the system menu bar (installed
        // via `macos_menu::install`). On other platforms we render an
        // in-window bar across the top.
        #[cfg(not(target_os = "macos"))]
        self.render_menu_bar(ctx);
        self.render_sidebar(ctx);
        self.render_snippet_panel(ctx);
        self.render_central(ctx);
        self.render_paste_modal(ctx);
        self.render_runner_modal(ctx);
        self.render_backup_modal(ctx);
        self.render_sync_modal(ctx);
        self.render_collection_settings_modal(ctx);
        self.render_env_modal(ctx);
        self.render_settings_modal(ctx);
        self.render_update_modal(ctx);
        self.render_updating_modal(ctx);
        self.render_export_secret_warning_modal(ctx);
        self.render_post_update_banner(ctx);
        self.render_save_draft_modal(ctx);
        self.render_confirm_close_draft(ctx);
        self.render_about_modal(ctx);
        self.render_command_palette(ctx);
        self.render_actions_palette(ctx);
        self.render_toast(ctx);
    }
}

impl ApiClient {
    pub(crate) fn effective_theme(&self) -> Theme {
        if self.show_settings_modal {
            self.editing_settings.theme
        } else {
            self.state.settings.theme
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

    /// Recursively deep-clones a folder (with fresh UUIDs for the folder,
    /// every request, and every nested subfolder) and inserts the copy
    /// next to the original. Naming convention mirrors the per-request
    /// duplicate — appends " (copy)" to the name.
    pub(crate) fn duplicate_folder(&mut self, folder_id: &str) {
        fn clone_deep(src: &Folder) -> Folder {
            Folder {
                id: Uuid::new_v4().to_string(),
                name: src.name.clone(),
                description: src.description.clone(),
                sync: src.sync.clone(),
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

/// Walk a folder tree and return a clone of the folder with this id
/// (or any descendant). Used by the overview view to pull current
/// metadata without holding a long-lived borrow.
pub(crate) fn find_folder_by_id<'a>(folders: &'a [Folder], id: &str) -> Option<&'a Folder> {
    for f in folders {
        if f.id == id {
            return Some(f);
        }
        if let Some(got) = find_folder_by_id(&f.subfolders, id) {
            return Some(got);
        }
    }
    None
}

/// Mutable counterpart of `find_folder_by_id` — used when saving the
/// description from the overview back into the tree.
pub(crate) fn find_folder_by_id_mut<'a>(
    folders: &'a mut [Folder],
    id: &str,
) -> Option<&'a mut Folder> {
    for f in folders {
        if f.id == id {
            return Some(f);
        }
        if let Some(got) = find_folder_by_id_mut(&mut f.subfolders, id) {
            return Some(got);
        }
    }
    None
}

fn find_folder_desc(folders: &[Folder], id: &str) -> Option<String> {
    find_folder_by_id(folders, id).map(|f| f.description.clone())
}

/// Depth-first collector for `flat_request_list` — pushes
/// `(folder_path, request_id)` for each request matching the search
/// query. Empty query accepts everything.
fn collect_flat_requests(
    folder: &Folder,
    path: &mut Vec<String>,
    out: &mut Vec<(Vec<String>, String)>,
    query: &str,
) {
    path.push(folder.id.clone());
    for r in &folder.requests {
        if query.is_empty() || widgets::request_matches(r, query) {
            out.push((path.clone(), r.id.clone()));
        }
    }
    for sub in &folder.subfolders {
        collect_flat_requests(sub, path, out, query);
    }
    path.pop();
}

const STATIC_LATEST_URL: &str = "https://chud-lori.github.io/rusty-requester/latest.json";
const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/chud-lori/rusty-requester/releases/latest";

/// Check the static GitHub Pages metadata first, then fall back to
/// GitHub's latest-release API. The static file avoids GitHub REST API
/// rate limits for normal update checks; the API fallback covers older
/// Pages deployments or temporary CDN misses.
pub(crate) fn spawn_update_check(
    rt: &tokio::runtime::Runtime,
) -> std::sync::mpsc::Receiver<UpdateCheckMessage> {
    let (tx, rx) = std::sync::mpsc::channel();
    let current = env!("CARGO_PKG_VERSION").to_string();
    rt.spawn(async move {
        // Small dedicated client, 5s timeout. Don't use the app's
        // shared `reqwest::Client` — it's tuned with the user's proxy
        // / TLS settings, which shouldn't affect GitHub API reachability.
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .user_agent(format!("rusty-requester/{}", current))
            .build();
        let Ok(client) = client else {
            let _ = tx.send(UpdateCheckMessage::Failed(
                "Could not create update client".to_string(),
            ));
            return;
        };
        let tag = match fetch_static_latest_tag(&client).await {
            Ok(tag) => tag,
            Err(static_reason) => match fetch_github_latest_tag(&client).await {
                Ok(tag) => tag,
                Err(api_reason) => {
                    let _ = tx.send(UpdateCheckMessage::Failed(format!(
                        "{}; fallback failed: {}",
                        static_reason, api_reason
                    )));
                    return;
                }
            },
        };
        // `tag_name` is `v0.13.0`; strip the `v` to compare with
        // `CARGO_PKG_VERSION`.
        let latest = tag.trim_start_matches('v');
        if is_newer_semver(latest, &current) {
            let _ = tx.send(UpdateCheckMessage::Available(tag.to_string()));
        } else {
            let _ = tx.send(UpdateCheckMessage::Current);
        }
    });
    rx
}

async fn fetch_static_latest_tag(client: &reqwest::Client) -> Result<String, String> {
    let resp = client
        .get(STATIC_LATEST_URL)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| "Could not reach update metadata".to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Update metadata returned {}", resp.status()));
    }
    let json = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|_| "Could not parse update metadata".to_string())?;
    latest_tag_from_json(&json, "Update metadata")
}

async fn fetch_github_latest_tag(client: &reqwest::Client) -> Result<String, String> {
    let resp = client
        .get(GITHUB_LATEST_RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "Could not reach GitHub releases".to_string())?;
    if !resp.status().is_success() {
        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            return Err("GitHub API rate limit reached".to_string());
        }
        return Err(format!("GitHub returned {}", resp.status()));
    }
    let json = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|_| "Could not parse GitHub release response".to_string())?;
    latest_tag_from_json(&json, "GitHub response")
}

fn latest_tag_from_json(json: &serde_json::Value, source: &str) -> Result<String, String> {
    if let Some(tag) = json.get("tag").and_then(|v| v.as_str()) {
        return Ok(tag.to_string());
    }
    if let Some(tag) = json.get("tag_name").and_then(|v| v.as_str()) {
        return Ok(tag.to_string());
    }
    if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
        return Ok(format!("v{}", version.trim_start_matches('v')));
    }
    Err(format!("{} did not include a release tag", source))
}

/// Compare two `X.Y.Z` version strings. Returns true when `a` is
/// strictly newer than `b`. Unparseable components are treated as 0
/// — good enough for tag comparison; never used for security checks.
fn is_newer_semver(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> (u64, u64, u64) {
        let mut parts = s.split('.');
        let major = parts.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        let minor = parts.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        // Patch may have a pre-release tag glued on — strip it.
        let patch_raw = parts.next().unwrap_or("0");
        let patch = patch_raw
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|x| x.parse().ok())
            .unwrap_or(0);
        (major, minor, patch)
    };
    parse(a) > parse(b)
}

/// Scratch directory for the one-click updater: log, status file,
/// runner script. Lives under the OS temp dir (`$TMPDIR` on macOS,
/// `/tmp` on Linux) instead of next to `data.json` because none of
/// these files need to survive a reboot — the log is interesting
/// while the user is debugging a fresh failure, but stale logs from
/// successful updates last week are just clutter. Caller is
/// responsible for `create_dir_all`.
pub(crate) fn update_scratch_dir() -> PathBuf {
    std::env::temp_dir().join("rusty-requester")
}

pub(crate) fn update_log_path() -> PathBuf {
    update_scratch_dir().join("update.log")
}

pub(crate) fn update_status_path() -> PathBuf {
    update_scratch_dir().join("update.status")
}

pub(crate) fn update_runner_path() -> PathBuf {
    update_scratch_dir().join("update-runner.sh")
}

/// True on platforms where the in-app one-click updater is supported.
/// Windows users see only the "Copy command" fallback path because we
/// don't have a clean detached-spawn + auto-relaunch recipe there
/// (and `install.sh` itself doesn't target Windows).
pub(crate) fn in_app_update_supported() -> bool {
    cfg!(any(target_os = "macos", target_os = "linux"))
}

/// Open `update.log` in the OS's default text-handler. Best effort —
/// silently ignores spawn failures (worst case: user opens it
/// themselves; the modal already shows the path).
pub(crate) fn open_update_log_in_os() {
    let path = update_log_path();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&path).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(&path)
        .spawn();
}

/// Read the last `cap` bytes of `path` as UTF-8 (lossy), returning an
/// empty string if the file doesn't exist yet. Used by the update
/// modal's live tail — install.sh is verbose, so we cap to a few KB
/// to keep the modal responsive.
fn read_log_tail(path: &std::path::Path, cap: u64) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(path) else {
        return String::new();
    };
    let Ok(meta) = f.metadata() else {
        return String::new();
    };
    let len = meta.len();
    let start = len.saturating_sub(cap);
    if f.seek(SeekFrom::Start(start)).is_err() {
        return String::new();
    }
    let mut buf = Vec::with_capacity(cap as usize);
    if f.read_to_end(&mut buf).is_err() {
        return String::new();
    }
    // Drop the leading partial line when we seeked into the middle of
    // one — keeps the tail readable.
    let s = String::from_utf8_lossy(&buf).into_owned();
    if start > 0 {
        if let Some(nl) = s.find('\n') {
            return s[nl + 1..].to_string();
        }
    }
    s
}

/// Read and consume `update.status` if present. Format written by the
/// wrapper script:
///
/// ```text
/// ok\n0.18.8\n         → success, new version
/// fail\n<reason>\n     → failure, free-form reason
/// ```
///
/// Returns `Some((true, version))` / `Some((false, reason))` /
/// `None` if no recent update activity. Always deletes the file
/// after reading so we don't show the same banner twice.
fn consume_update_status() -> Option<(bool, String)> {
    let path = update_status_path();
    let raw = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let mut lines = raw.lines();
    let kind = lines.next()?.trim();
    let detail = lines.next().unwrap_or("").trim().to_string();
    match kind {
        // Override the predicted version with the actually-running
        // version — install.sh fetches "latest", which may have moved
        // since our pre-update check.
        "ok" => Some((true, env!("CARGO_PKG_VERSION").to_string())),
        "fail" => Some((
            false,
            if detail.is_empty() {
                "unknown error".to_string()
            } else {
                detail
            },
        )),
        _ => None,
    }
}

/// Write panic info to a log file next to `data.json` so users can
/// attach it to a bug report. Chains to the default panic hook so the
/// usual stderr output + process-exit behavior still happens.
fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let log_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rusty-requester")
            .join("panic.log");
        let _ = std::fs::create_dir_all(log_path.parent().unwrap_or(&log_path));
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let payload = format!(
            "=== rusty-requester panic @ unix-ts {} ===\n\
             version: {}\n\
             info: {}\n\
             backtrace:\n{}\n\n",
            ts,
            env!("CARGO_PKG_VERSION"),
            info,
            std::backtrace::Backtrace::force_capture()
        );
        // Append, never overwrite — multiple crashes in a session
        // should all land in the same file in order.
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            use std::io::Write;
            let _ = f.write_all(payload.as_bytes());
        }
        eprintln!("rusty-requester: panic logged to {}", log_path.display());
        prev(info);
    }));
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

    // Postmortem panic log — on any panic anywhere in the app, append
    // location + message + backtrace to a file next to `data.json` so
    // users can share it on a bug report. `save_state` runs on every
    // edit, so the user's workspace is already persisted; this hook
    // covers the diagnostic gap. Cheap (~0 overhead until a panic
    // actually fires).
    install_panic_hook();

    // Force NSApp into Regular activation policy BEFORE eframe starts the
    // macOS run loop. Once NSApp.run has begun processing events, macOS
    // rejects the setActivationPolicy(Regular) call, which is why calling
    // this from update() / CreationContext didn't work — the process stayed
    // Accessory/Prohibited and Cmd+Tab showed the parent terminal's icon.
    if let Err(e) = set_macos_activation_policy_regular() {
        eprintln!("[icon] activation policy set failed: {}", e);
    }

    // Set NSApp's applicationIconImage BEFORE the menu bar is
    // installed (and before the first About menu click). NSApp's stock
    // About panel reads this property at render time; if we delay it
    // until the first `update()` frame, the initial panel shows the
    // generic file-bundle icon.
    #[cfg(target_os = "macos")]
    if let Err(e) = set_macos_app_icon_image(APP_ICON_BYTES) {
        eprintln!("[icon] early app-icon set failed: {}", e);
    }

    // The macOS menu bar is installed from the first `update()` frame,
    // NOT here — winit's NSApp delegate overwrites any menu we set up
    // before `eframe::run_native` starts its run loop, which is why
    // you'd see the default Services/Hide/Quit stub instead of our
    // custom menu bar.

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([900.0, 600.0])
        .with_title("Rusty Requester")
        // Wayland app_id / X11 WM_CLASS. Must match the `.desktop`
        // file's `StartupWMClass=` so GNOME / KDE can associate the
        // running window with the installed launcher and show the
        // right icon in the dock / Activities. Without this, Ubuntu
        // under Wayland shows a generic cog as the window icon even
        // though `_NET_WM_ICON` is set (issue #18).
        .with_app_id("rusty-requester");
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    // NOTE: `.with_fullsize_content_view(true)` was tried to make the
    // macOS title bar blend with the app chrome (Postman / Arc style),
    // but eframe 0.29 renders a dark empty strip above the content
    // instead of actually extending content under the title bar.
    // Reverted — we keep the native macOS title bar for now. Revisit
    // when we bump to egui 0.31+ where this combo works properly.

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

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(folder_path: Vec<&str>, request_id: &str, pinned: bool) -> OpenTab {
        OpenTab {
            folder_path: folder_path.into_iter().map(str::to_string).collect(),
            request_id: request_id.to_string(),
            pinned,
        }
    }

    #[test]
    fn restored_active_tab_prefers_persisted_active_id() {
        let open_tabs = vec![
            tab(vec!["collection-a"], "first", false),
            tab(vec!["collection-b"], "second", true),
        ];

        let restored = restored_active_tab(&open_tabs, Some("second")).unwrap();

        assert_eq!(restored.request_id, "second");
        assert_eq!(restored.folder_path, vec!["collection-b".to_string()]);
        assert!(restored.pinned);
    }

    #[test]
    fn restored_active_tab_falls_back_to_first_open_tab_when_saved_id_is_stale() {
        let open_tabs = vec![
            tab(vec!["collection-a"], "first", false),
            tab(vec!["collection-b"], "second", false),
        ];

        let restored = restored_active_tab(&open_tabs, Some("missing")).unwrap();

        assert_eq!(restored.request_id, "first");
        assert_eq!(restored.folder_path, vec!["collection-a".to_string()]);
    }

    #[test]
    fn restored_active_tab_falls_back_to_first_open_tab_without_saved_id() {
        let open_tabs = vec![
            tab(vec![], "draft", false),
            tab(vec!["collection-b"], "saved", false),
        ];

        let restored = restored_active_tab(&open_tabs, None).unwrap();

        assert_eq!(restored.request_id, "draft");
        assert!(restored.folder_path.is_empty());
    }

    #[test]
    fn restored_active_tab_returns_none_for_empty_workspace() {
        assert!(restored_active_tab(&[], Some("missing")).is_none());
    }

    #[test]
    fn latest_tag_from_json_accepts_static_metadata_shape() {
        let json = serde_json::json!({
            "version": "0.27.2",
            "tag": "v0.27.2",
            "release_url": "https://github.com/chud-lori/rusty-requester/releases/tag/v0.27.2"
        });

        assert_eq!(
            latest_tag_from_json(&json, "Update metadata").unwrap(),
            "v0.27.2"
        );
    }

    #[test]
    fn latest_tag_from_json_accepts_github_release_shape() {
        let json = serde_json::json!({
            "tag_name": "v0.27.2"
        });

        assert_eq!(
            latest_tag_from_json(&json, "GitHub response").unwrap(),
            "v0.27.2"
        );
    }

    #[test]
    fn latest_tag_from_json_prefixes_version_when_tag_missing() {
        let json = serde_json::json!({
            "version": "0.27.2"
        });

        assert_eq!(
            latest_tag_from_json(&json, "Update metadata").unwrap(),
            "v0.27.2"
        );
    }
}
