mod curl;

use base64::Engine;
use eframe::egui;
use poll_promise::Promise;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Request {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub query_params: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
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
enum AuthKind {
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub requests: Vec<Request>,
    #[serde(default)]
    pub subfolders: Vec<Folder>,
}

#[derive(Serialize, Deserialize, Default)]
struct AppState {
    folders: Vec<Folder>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RequestTab {
    Params,
    Headers,
    Body,
    Auth,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResponseTab {
    Body,
    Headers,
}

struct ResponseData {
    body: String,
    status: String,
    time: String,
    headers: Vec<(String, String)>,
}

struct ApiClient {
    state: AppState,
    selected_folder_path: Vec<String>,
    selected_request_id: Option<String>,

    new_header_key: String,
    new_header_value: String,
    new_param_key: String,
    new_param_value: String,

    response_text: String,
    response_status: String,
    response_time: String,
    response_headers: Vec<(String, String)>,
    is_loading: bool,

    editing_url: String,
    editing_body: String,
    editing_name: String,
    editing_method: HttpMethod,
    editing_headers: Vec<(String, String)>,
    editing_params: Vec<(String, String)>,
    editing_auth: Auth,

    storage_path: PathBuf,

    request_promise: Option<Promise<ResponseData>>,

    renaming_folder_id: Option<String>,
    rename_folder_text: String,

    request_tab: RequestTab,
    response_tab: ResponseTab,

    show_paste_modal: bool,
    paste_curl_text: String,
    paste_error: String,

    toast: Option<(String, f32)>,
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
        });

        Self {
            state,
            selected_folder_path: vec![],
            selected_request_id: None,
            new_header_key: String::new(),
            new_header_value: String::new(),
            new_param_key: String::new(),
            new_param_value: String::new(),
            response_text: String::new(),
            response_status: String::new(),
            response_time: String::new(),
            response_headers: vec![],
            is_loading: false,
            editing_url: String::new(),
            editing_body: String::new(),
            editing_name: String::new(),
            editing_method: HttpMethod::GET,
            editing_headers: vec![],
            editing_params: vec![],
            editing_auth: Auth::None,
            storage_path,
            request_promise: None,
            renaming_folder_id: None,
            rename_folder_text: String::new(),
            request_tab: RequestTab::Params,
            response_tab: ResponseTab::Body,
            show_paste_modal: false,
            paste_curl_text: String::new(),
            paste_error: String::new(),
            toast: None,
        }
    }
}

impl ApiClient {
    fn load_state(path: &PathBuf) -> Option<AppState> {
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save_state(&self) {
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
        if self.selected_folder_path.is_empty() {
            return None;
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
        let auth = self.editing_auth.clone();
        self.update_current_request(|req| {
            req.name = name;
            req.method = method;
            req.url = url;
            req.body = body;
            req.headers = headers;
            req.query_params = params;
            req.auth = auth;
        });
    }

    fn send_request(&mut self) {
        self.commit_editing();
        if let Some(request) = self.get_current_request() {
            self.is_loading = true;
            self.response_text = "Loading...".to_string();
            self.response_status = "Sending request...".to_string();
            self.response_time = String::new();
            self.response_headers.clear();
            self.request_promise = Some(Promise::spawn_thread("request", move || {
                Self::execute_request(&request)
            }));
        }
    }

    fn execute_request(request: &Request) -> ResponseData {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = reqwest::Client::new();
            let final_url = curl::build_full_url(&request.url, &request.query_params);

            let mut req_builder = match request.method {
                HttpMethod::GET => client.get(&final_url),
                HttpMethod::POST => client.post(&final_url),
                HttpMethod::PUT => client.put(&final_url),
                HttpMethod::DELETE => client.delete(&final_url),
                HttpMethod::PATCH => client.patch(&final_url),
                HttpMethod::HEAD => client.head(&final_url),
                HttpMethod::OPTIONS => client.request(reqwest::Method::OPTIONS, &final_url),
            };

            for (key, value) in &request.headers {
                if key.trim().is_empty() {
                    continue;
                }
                req_builder = req_builder.header(key, value);
            }

            match &request.auth {
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

            if !request.body.is_empty() {
                req_builder = req_builder.body(request.body.clone());
            }

            let start = std::time::Instant::now();
            match req_builder.send().await {
                Ok(response) => {
                    let elapsed = start.elapsed();
                    let status = format!(
                        "{} {}",
                        response.status().as_u16(),
                        response.status().canonical_reason().unwrap_or("")
                    );
                    let time = format!("{}ms", elapsed.as_millis());

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
                    }
                }
                Err(e) => ResponseData {
                    body: format!("Error: {}", e),
                    status: "Failed".to_string(),
                    time: "0ms".to_string(),
                    headers: vec![],
                },
            }
        })
    }

    fn load_request_for_editing(&mut self) {
        if let Some(r) = self.get_current_request() {
            self.editing_url = r.url;
            self.editing_body = r.body;
            self.editing_name = r.name;
            self.editing_method = r.method;
            self.editing_headers = r.headers;
            self.editing_params = r.query_params;
            self.editing_auth = r.auth;
        }
    }

    fn show_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), 2.5));
    }
}

// ====== Colors ======
const C_BG: egui::Color32 = egui::Color32::from_rgb(26, 27, 38);
const C_PANEL: egui::Color32 = egui::Color32::from_rgb(40, 42, 54);
const C_PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(30, 31, 41);
const C_BORDER: egui::Color32 = egui::Color32::from_rgb(68, 71, 90);
const C_ACCENT: egui::Color32 = egui::Color32::from_rgb(139, 233, 253);
const C_PURPLE: egui::Color32 = egui::Color32::from_rgb(189, 147, 249);
const C_GREEN: egui::Color32 = egui::Color32::from_rgb(80, 250, 123);
const C_ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 184, 108);
const C_PINK: egui::Color32 = egui::Color32::from_rgb(255, 121, 198);
const C_RED: egui::Color32 = egui::Color32::from_rgb(255, 85, 85);
const C_MUTED: egui::Color32 = egui::Color32::from_rgb(98, 114, 164);
const C_TEXT: egui::Color32 = egui::Color32::from_rgb(220, 220, 230);

fn method_color(m: &HttpMethod) -> egui::Color32 {
    match m {
        HttpMethod::GET => C_GREEN,
        HttpMethod::POST => C_ORANGE,
        HttpMethod::PUT => C_ACCENT,
        HttpMethod::DELETE => C_PINK,
        HttpMethod::PATCH => C_PURPLE,
        _ => C_MUTED,
    }
}

fn status_color(status: &str) -> egui::Color32 {
    if status.starts_with('2') {
        C_GREEN
    } else if status.starts_with('3') {
        C_ORANGE
    } else if status.starts_with('4') || status.starts_with('5') {
        C_RED
    } else {
        C_MUTED
    }
}

impl eframe::App for ApiClient {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(promise) = &self.request_promise {
            if let Some(r) = promise.ready() {
                self.response_text = r.body.clone();
                self.response_status = r.status.clone();
                self.response_time = r.time.clone();
                self.response_headers = r.headers.clone();
                self.is_loading = false;
                self.request_promise = None;
            } else {
                ctx.request_repaint();
            }
        }

        // Apply theme
        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill = C_BG;
        style.visuals.panel_fill = C_BG;
        style.visuals.override_text_color = Some(C_TEXT);
        style.visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, C_BORDER);
        style.visuals.widgets.inactive.bg_fill = C_PANEL;
        style.visuals.widgets.hovered.bg_fill = C_BORDER;
        style.visuals.widgets.active.bg_fill = C_BORDER;
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        ctx.set_style(style);

        self.render_sidebar(ctx);
        self.render_central(ctx);
        self.render_paste_modal(ctx);
        self.render_toast(ctx);
    }
}

impl ApiClient {
    fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .min_width(280.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("📁 Collections")
                            .size(18.0)
                            .color(C_ACCENT),
                    );
                });
                ui.add_space(5.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui
                        .add_sized(
                            [ui.available_width() * 0.55, 30.0],
                            egui::Button::new(egui::RichText::new("➕ New Folder").size(13.0))
                                .fill(egui::Color32::from_rgb(56, 170, 100))
                                .stroke(egui::Stroke::NONE),
                        )
                        .clicked()
                    {
                        self.state.folders.push(Folder {
                            id: Uuid::new_v4().to_string(),
                            name: format!("Folder {}", self.state.folders.len() + 1),
                            requests: vec![],
                            subfolders: vec![],
                        });
                        self.save_state();
                    }

                    if ui
                        .add_sized(
                            [ui.available_width(), 30.0],
                            egui::Button::new(egui::RichText::new("📥 Import cURL").size(13.0))
                                .fill(C_PANEL)
                                .stroke(egui::Stroke::NONE),
                        )
                        .clicked()
                    {
                        self.show_paste_modal = true;
                        self.paste_curl_text.clear();
                        self.paste_error.clear();
                    }
                });

                ui.add_space(10.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let folders = self.state.folders.clone();
                    for folder in &folders {
                        self.render_folder(ui, folder, vec![folder.id.clone()]);
                    }
                });
            });
    }

    fn render_central(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.selected_request_id.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Select or create a request to get started")
                            .size(16.0)
                            .color(C_MUTED),
                    );
                });
                return;
            }

            ui.add_space(10.0);
            self.render_name_bar(ui);
            ui.add_space(8.0);
            self.render_url_bar(ui);
            ui.add_space(10.0);
            self.render_request_tabs(ui);
            ui.add_space(10.0);
            self.render_response(ui);
        });
    }

    fn render_name_bar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(C_PANEL)
            .inner_margin(10.0)
            .rounding(6.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Name")
                            .size(13.0)
                            .color(C_ACCENT),
                    );
                    let avail = ui.available_width();
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.editing_name)
                                .desired_width(avail - 90.0)
                                .font(egui::TextStyle::Body),
                        )
                        .changed()
                    {
                        let name = self.editing_name.clone();
                        self.update_current_request(|req| req.name = name);
                    }
                    let delete_btn = egui::Button::new(
                        egui::RichText::new("🗑 Delete").size(12.0).color(egui::Color32::WHITE),
                    )
                    .fill(C_RED)
                    .stroke(egui::Stroke::NONE);

                    if ui.add(delete_btn).clicked() {
                        if let Some(req_id) = self.selected_request_id.clone() {
                            if let Some(folder) = self.get_current_folder_mut() {
                                folder.requests.retain(|r| r.id != req_id);
                            }
                            self.save_state();
                            self.selected_request_id = None;
                            self.editing_name.clear();
                            self.editing_url.clear();
                            self.editing_body.clear();
                            self.editing_headers.clear();
                            self.editing_params.clear();
                            self.editing_auth = Auth::None;
                            self.response_text.clear();
                            self.response_status.clear();
                            self.response_time.clear();
                            self.response_headers.clear();
                        }
                    }
                });
            });
    }

    fn render_url_bar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(C_PANEL)
            .inner_margin(10.0)
            .rounding(6.0)
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

                    // Reserve space for Send + cURL + Paste buttons (~240 px)
                    let btn_space = 260.0;
                    let avail = (ui.available_width() - btn_space).max(200.0);
                    let url_edit = ui.add(
                        egui::TextEdit::singleline(&mut self.editing_url)
                            .desired_width(avail)
                            .hint_text("https://api.example.com/endpoint")
                            .font(egui::TextStyle::Monospace),
                    );
                    if url_edit.changed() {
                        let url = self.editing_url.clone();
                        self.update_current_request(|req| req.url = url);
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

                    let send_click = ui.add_enabled(!self.is_loading, send_btn).clicked();

                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("📋 cURL").size(12.0))
                                .fill(C_BORDER)
                                .min_size(egui::vec2(70.0, 28.0)),
                        )
                        .on_hover_text("Copy as cURL")
                        .clicked()
                    {
                        self.commit_editing();
                        if let Some(req) = self.get_current_request() {
                            let text = curl::to_curl(&req);
                            ui.output_mut(|o| o.copied_text = text);
                            self.show_toast("Copied as cURL");
                        }
                    }

                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("📥 Paste").size(12.0))
                                .fill(C_BORDER)
                                .min_size(egui::vec2(70.0, 28.0)),
                        )
                        .on_hover_text("Paste cURL into this request")
                        .clicked()
                    {
                        self.show_paste_modal = true;
                        self.paste_curl_text.clear();
                        self.paste_error.clear();
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

        ui.horizontal(|ui| {
            tab_button(ui, &mut self.request_tab, RequestTab::Params, &params_label);
            tab_button(ui, &mut self.request_tab, RequestTab::Headers, &headers_label);
            tab_button(ui, &mut self.request_tab, RequestTab::Body, &body_label);
            tab_button(ui, &mut self.request_tab, RequestTab::Auth, &auth_label);
        });

        egui::Frame::none()
            .fill(C_PANEL)
            .inner_margin(12.0)
            .rounding(6.0)
            .stroke(egui::Stroke::new(1.0, C_BORDER))
            .show(ui, |ui| {
                let section_height = (ui.available_height() * 0.38).clamp(140.0, 260.0);
                ui.set_min_height(section_height);
                ui.set_max_height(section_height);
                egui::ScrollArea::vertical()
                    .id_salt("request_tab_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.request_tab {
                        RequestTab::Params => self.render_params_tab(ui),
                        RequestTab::Headers => self.render_headers_tab(ui),
                        RequestTab::Body => self.render_body_tab(ui),
                        RequestTab::Auth => self.render_auth_tab(ui),
                    });
            });
    }

    fn render_params_tab(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let mut to_remove = None;
        for (i, (k, v)) in self.editing_params.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::TextEdit::singleline(k)
                            .desired_width(160.0)
                            .hint_text("Key"),
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(
                        egui::TextEdit::singleline(v)
                            .desired_width(ui.available_width() - 40.0)
                            .hint_text("Value"),
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui.small_button("🗑").clicked() {
                    to_remove = Some(i);
                }
            });
        }
        if let Some(i) = to_remove {
            self.editing_params.remove(i);
            changed = true;
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.new_param_key)
                    .desired_width(160.0)
                    .hint_text("New key"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.new_param_value)
                    .desired_width(ui.available_width() - 90.0)
                    .hint_text("New value"),
            );
            if ui.button("➕ Add").clicked() && !self.new_param_key.is_empty() {
                self.editing_params
                    .push((self.new_param_key.clone(), self.new_param_value.clone()));
                self.new_param_key.clear();
                self.new_param_value.clear();
                changed = true;
            }
        });
        if changed {
            let params = self.editing_params.clone();
            self.update_current_request(|r| r.query_params = params);
        }
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(format!(
                "Final URL: {}",
                curl::build_full_url(&self.editing_url, &self.editing_params)
            ))
            .size(11.0)
            .color(C_MUTED),
        );
    }

    fn render_headers_tab(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let mut to_remove = None;
        for (i, (k, v)) in self.editing_headers.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::TextEdit::singleline(k)
                            .desired_width(160.0)
                            .hint_text("Key"),
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(
                        egui::TextEdit::singleline(v)
                            .desired_width(ui.available_width() - 40.0)
                            .hint_text("Value"),
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui.small_button("🗑").clicked() {
                    to_remove = Some(i);
                }
            });
        }
        if let Some(i) = to_remove {
            self.editing_headers.remove(i);
            changed = true;
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.new_header_key)
                    .desired_width(160.0)
                    .hint_text("New key"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.new_header_value)
                    .desired_width(ui.available_width() - 90.0)
                    .hint_text("New value"),
            );
            if ui.button("➕ Add").clicked() && !self.new_header_key.is_empty() {
                self.editing_headers
                    .push((self.new_header_key.clone(), self.new_header_value.clone()));
                self.new_header_key.clear();
                self.new_header_value.clear();
                changed = true;
            }
        });
        if changed {
            let headers = self.editing_headers.clone();
            self.update_current_request(|r| r.headers = headers);
        }
    }

    fn render_body_tab(&mut self, ui: &mut egui::Ui) {
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

    fn render_response(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Response")
                    .size(15.0)
                    .strong()
                    .color(C_ACCENT),
            );
            ui.separator();
            if !self.response_status.is_empty() {
                ui.label(egui::RichText::new("Status:").size(12.0).strong());
                ui.label(
                    egui::RichText::new(&self.response_status)
                        .color(status_color(&self.response_status))
                        .strong()
                        .size(12.0),
                );
            }
            if !self.response_time.is_empty() {
                ui.separator();
                ui.label(egui::RichText::new("⏱").size(12.0));
                ui.label(
                    egui::RichText::new(&self.response_time)
                        .color(C_ACCENT)
                        .size(12.0),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📋 Copy").clicked() && !self.response_text.is_empty() {
                    ui.output_mut(|o| o.copied_text = self.response_text.clone());
                    self.show_toast("Response copied");
                }
            });
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

        let remaining_height = (ui.available_height() - 10.0).max(120.0);
        egui::Frame::none()
            .fill(C_PANEL_DARK)
            .inner_margin(12.0)
            .rounding(6.0)
            .stroke(egui::Stroke::new(1.0, C_BORDER))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("response_scroll")
                    .max_height(remaining_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.response_tab {
                        ResponseTab::Body => {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.response_text.as_str())
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Monospace),
                            );
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
            self.selected_request_id = Some(new_id);
            self.load_request_for_editing();
            self.response_text.clear();
            self.response_status.clear();
            self.response_time.clear();
            self.response_headers.clear();
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
                    .rounding(6.0)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(msg).color(C_TEXT).size(13.0));
                    });
            });
    }

    fn render_folder(&mut self, ui: &mut egui::Ui, folder: &Folder, path: Vec<String>) {
        let is_renaming = self.renaming_folder_id.as_ref() == Some(&folder.id);

        let header_response = egui::CollapsingHeader::new(if is_renaming {
            egui::RichText::new("").size(13.0)
        } else {
            egui::RichText::new(format!("📁 {}", folder.name)).size(13.0)
        })
        .id_salt(&folder.id)
        .default_open(true)
        .show(ui, |ui| {
            ui.add_space(4.0);

            if ui
                .add_sized(
                    [ui.available_width(), 26.0],
                    egui::Button::new(egui::RichText::new("➕ New Request").size(12.0))
                        .fill(C_BORDER)
                        .stroke(egui::Stroke::NONE),
                )
                .clicked()
            {
                let new_req = Request {
                    id: Uuid::new_v4().to_string(),
                    name: format!("Request {}", folder.requests.len() + 1),
                    method: HttpMethod::GET,
                    url: "https://api.example.com".to_string(),
                    query_params: vec![],
                    headers: vec![],
                    body: String::new(),
                    auth: Auth::None,
                };

                self.selected_folder_path = path.clone();

                if let Some(f) = self.get_current_folder_mut() {
                    f.requests.push(new_req);
                }
                self.save_state();
            }

            ui.add_space(4.0);

            let mut to_delete: Option<String> = None;
            for req in &folder.requests {
                let is_selected = self.selected_request_id.as_ref() == Some(&req.id);
                let mc = method_color(&req.method);

                let bg_color = if is_selected { C_BORDER } else { C_PANEL };

                ui.horizontal(|ui| {
                    let button = egui::Button::new(
                        egui::RichText::new(format!("{:<7} {}", req.method, req.name))
                            .size(12.0)
                            .color(if is_selected { egui::Color32::WHITE } else { C_TEXT }),
                    )
                    .fill(bg_color)
                    .stroke(if is_selected {
                        egui::Stroke::new(1.0, mc)
                    } else {
                        egui::Stroke::NONE
                    });

                    let resp = ui.add_sized([ui.available_width() - 26.0, 28.0], button);
                    if resp.clicked() {
                        self.selected_folder_path = path.clone();
                        self.selected_request_id = Some(req.id.clone());
                        self.load_request_for_editing();
                        self.response_text.clear();
                        self.response_status.clear();
                        self.response_time.clear();
                        self.response_headers.clear();
                    }
                    resp.context_menu(|ui| {
                        if ui.button("🗑 Delete").clicked() {
                            to_delete = Some(req.id.clone());
                            ui.close_menu();
                        }
                    });

                    if ui.small_button("✕").on_hover_text("Delete").clicked() {
                        to_delete = Some(req.id.clone());
                    }
                });

                ui.add_space(2.0);
            }

            if let Some(del_id) = to_delete {
                self.selected_folder_path = path.clone();
                if let Some(f) = self.get_current_folder_mut() {
                    f.requests.retain(|r| r.id != del_id);
                }
                if self.selected_request_id.as_deref() == Some(del_id.as_str()) {
                    self.selected_request_id = None;
                    self.editing_name.clear();
                    self.editing_url.clear();
                    self.editing_body.clear();
                    self.editing_headers.clear();
                    self.editing_params.clear();
                    self.editing_auth = Auth::None;
                }
                self.save_state();
            }

            for subfolder in &folder.subfolders {
                let mut subpath = path.clone();
                subpath.push(subfolder.id.clone());
                self.render_folder(ui, subfolder, subpath);
            }
        });

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
            let mut start_rename = false;
            let mut delete_folder = false;
            header_response.header_response.context_menu(|ui| {
                if ui.button("✏ Rename").clicked() {
                    start_rename = true;
                    ui.close_menu();
                }
                if ui.button("🗑 Delete folder").clicked() {
                    delete_folder = true;
                    ui.close_menu();
                }
            });
            if start_rename {
                self.renaming_folder_id = Some(folder_id.clone());
                self.rename_folder_text = folder_name;
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
            self.selected_folder_path.clear();
            self.selected_request_id = None;
        }
    }
}

fn tab_button<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    current: &mut T,
    value: T,
    label: &str,
) {
    let selected = *current == value;
    let text_color = if selected { C_TEXT } else { C_MUTED };
    let fill = if selected { C_BORDER } else { C_PANEL };
    let stroke = if selected {
        egui::Stroke::new(2.0, C_ACCENT)
    } else {
        egui::Stroke::new(1.0, C_BORDER)
    };
    let btn = egui::Button::new(egui::RichText::new(label).color(text_color).size(12.0))
        .fill(fill)
        .stroke(stroke)
        .min_size(egui::vec2(90.0, 26.0));
    if ui.add(btn).clicked() {
        *current = value;
    }
}

fn short_name_from_url(url: &str) -> String {
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let cutoff = stripped.find('?').unwrap_or(stripped.len());
    stripped[..cutoff].trim_end_matches('/').to_string()
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Rusty Requester",
        options,
        Box::new(|_cc| Ok(Box::new(ApiClient::default()))),
    )
}
