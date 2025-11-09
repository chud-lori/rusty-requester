use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
use poll_promise::Promise;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Request {
    id: String,
    name: String,
    method: HttpMethod,
    url: String,
    headers: Vec<(String, String)>,
    body: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
enum HttpMethod {
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

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Folder {
    id: String,
    name: String,
    requests: Vec<Request>,
    subfolders: Vec<Folder>,
}

#[derive(Serialize, Deserialize)]
struct AppState {
    folders: Vec<Folder>,
}

struct ApiClient {
    state: AppState,
    selected_folder_path: Vec<String>,
    selected_request_id: Option<String>,

    new_header_key: String,
    new_header_value: String,
    response_text: String,
    response_status: String,
    response_time: String,
    is_loading: bool,

    editing_url: String,
    editing_body: String,
    editing_name: String,
    editing_method: HttpMethod,
    editing_headers: Vec<(String, String)>,

    storage_path: PathBuf,
    request_promise: Option<Promise<(String, String, String)>>,
    renaming_folder_id: Option<String>,
    rename_folder_text: String,
}

impl Default for ApiClient {
    fn default() -> Self {
        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".rusty-requester")
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
            response_text: String::new(),
            response_status: String::new(),
            response_time: String::new(),
            is_loading: false,
            editing_url: String::new(),
            editing_body: String::new(),
            editing_name: String::new(),
            editing_method: HttpMethod::GET,
            editing_headers: vec![],
            storage_path,
            request_promise: None,
            renaming_folder_id: None,
            rename_folder_text: String::new(),
        }
    }
}

impl ApiClient {
    fn load_state(path: &PathBuf) -> Option<AppState> {
        if let Ok(data) = fs::read_to_string(path) {
            serde_json::from_str(&data).ok()
        } else {
            None
        }
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

        let mut folder = self.state.folders.iter_mut()
            .find(|f| f.id == self.selected_folder_path[0])?;

        for id in &self.selected_folder_path[1..] {
            folder = folder.subfolders.iter_mut().find(|f| &f.id == id)?;
        }

        Some(folder)
    }

    fn get_current_request(&self) -> Option<Request> {
        let req_id = self.selected_request_id.as_ref()?;

        if self.selected_folder_path.is_empty() {
            return None;
        }

        let mut folder = self.state.folders.iter()
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
        if let Some(req_id) = &self.selected_request_id.clone() {
            if let Some(folder) = self.get_current_folder_mut() {
                if let Some(request) = folder.requests.iter_mut().find(|r| &r.id == req_id) {
                    updater(request);
                    self.save_state();
                }
            }
        }
    }

    fn send_request(&mut self) {
        if let Some(request) = self.get_current_request() {
            self.is_loading = true;
            self.response_text = "Loading...".to_string();
            self.response_status = "Sending request...".to_string();
            self.response_time = String::new();

            self.request_promise = Some(Promise::spawn_thread("request", move || {
                Self::execute_request(&request)
            }));
        }
    }

    fn execute_request(request: &Request) -> (String, String, String) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let client = reqwest::Client::new();

            let mut req_builder = match request.method {
                HttpMethod::GET => client.get(&request.url),
                HttpMethod::POST => client.post(&request.url),
                HttpMethod::PUT => client.put(&request.url),
                HttpMethod::DELETE => client.delete(&request.url),
                HttpMethod::PATCH => client.patch(&request.url),
                HttpMethod::HEAD => client.head(&request.url),
                HttpMethod::OPTIONS => client.request(reqwest::Method::OPTIONS, &request.url),
            };

            for (key, value) in &request.headers {
                req_builder = req_builder.header(key, value);
            }

            if !request.body.is_empty() {
                req_builder = req_builder.body(request.body.clone());
            }

            let start = std::time::Instant::now();
            match req_builder.send().await {
                Ok(response) => {
                    let elapsed = start.elapsed();
                    let status = format!("{} {}", response.status().as_u16(),
                                       response.status().canonical_reason().unwrap_or(""));
                    let time = format!("{}ms", elapsed.as_millis());

                    let body = response.text().await.unwrap_or_else(|e| format!("Error reading body: {}", e));

                    let formatted_body = if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&body) {
                        serde_json::to_string_pretty(&json_value).unwrap_or(body)
                    } else {
                        body
                    };

                    (formatted_body, status, time)
                }
                Err(e) => (format!("Error: {}", e), "Failed".to_string(), "0ms".to_string()),
            }
        })
    }

    fn load_request_for_editing(&mut self) {
        if let Some(request) = self.get_current_request() {
            self.editing_url = request.url.clone();
            self.editing_body = request.body.clone();
            self.editing_name = request.name.clone();
            self.editing_method = request.method.clone();
            self.editing_headers = request.headers.clone();
        }
    }

    fn rename_folder(&mut self, folder_id: &str, new_name: String) {
        fn find_and_rename(folders: &mut Vec<Folder>, id: &str, name: String) -> bool {
            for folder in folders {
                if folder.id == id {
                    folder.name = name;
                    return true;
                }
                if find_and_rename(&mut folder.subfolders, id, name.clone()) {
                    return true;
                }
            }
            false
        }

        if find_and_rename(&mut self.state.folders, folder_id, new_name) {
            self.save_state();
        }
    }

    fn delete_folder(&mut self, folder_id: &str) {
        fn find_and_delete(folders: &mut Vec<Folder>, id: &str) -> bool {
            if let Some(pos) = folders.iter().position(|f| f.id == id) {
                folders.remove(pos);
                return true;
            }

            for folder in folders {
                if find_and_delete(&mut folder.subfolders, id) {
                    return true;
                }
            }
            false
        }

        if find_and_delete(&mut self.state.folders, folder_id) {
            self.save_state();
            self.selected_request_id = None;
            self.selected_folder_path.clear();
        }
    }

    fn render_folder(&mut self, ui: &mut egui::Ui, folder: &Folder, path: Vec<String>, _idx: usize) {
        let is_renaming = self.renaming_folder_id.as_ref() == Some(&folder.id);

        egui::Frame::none()
            .fill(egui::Color32::from_rgb(28, 28, 28))
            .inner_margin(0.0)
            .rounding(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(12.0);

                    if is_renaming {
                        let response = ui.add(egui::TextEdit::singleline(&mut self.rename_folder_text)
                            .desired_width(150.0));

                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.rename_folder(&folder.id, self.rename_folder_text.clone());
                            self.renaming_folder_id = None;
                        }

                        if ui.add(egui::Button::new("OK").fill(egui::Color32::from_rgb(72, 187, 120)).rounding(4.0)).clicked() {
                            self.rename_folder(&folder.id, self.rename_folder_text.clone());
                            self.renaming_folder_id = None;
                        }

                        if ui.add(egui::Button::new("X").fill(egui::Color32::from_rgb(248, 113, 113)).rounding(4.0)).clicked() {
                            self.renaming_folder_id = None;
                        }
                    } else {
                        ui.label(egui::RichText::new(format!("[ ] {}", folder.name)).size(14.0).strong());

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(8.0);
                            ui.menu_button("•••", |ui| {
                                if ui.button("Rename").clicked() {
                                    self.renaming_folder_id = Some(folder.id.clone());
                                    self.rename_folder_text = folder.name.clone();
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.add(egui::Button::new(egui::RichText::new("Delete").color(egui::Color32::from_rgb(248, 113, 113)))).clicked() {
                                    self.delete_folder(&folder.id);
                                    ui.close_menu();
                                }
                            });
                        });
                    }
                });

                ui.add_space(8.0);

                if ui.add_sized([ui.available_width() - 24.0, 32.0],
                    egui::Button::new(egui::RichText::new("+ New Request").size(12.0))
                        .fill(egui::Color32::from_rgb(35, 35, 35))
                        .rounding(6.0)).clicked() {
                    let new_req = Request {
                        id: Uuid::new_v4().to_string(),
                        name: format!("Request {}", folder.requests.len() + 1),
                        method: HttpMethod::GET,
                        url: "https://api.example.com".to_string(),
                        headers: vec![],
                        body: String::new(),
                    };

                    self.selected_folder_path = path.clone();

                    if let Some(f) = self.get_current_folder_mut() {
                        f.requests.push(new_req);
                        self.save_state();
                    }
                }

                ui.add_space(8.0);

                for req in &folder.requests {
                    let is_selected = self.selected_request_id.as_ref() == Some(&req.id);

                    let method_color = match req.method {
                        HttpMethod::GET => egui::Color32::from_rgb(72, 187, 120),
                        HttpMethod::POST => egui::Color32::from_rgb(251, 146, 60),
                        HttpMethod::PUT => egui::Color32::from_rgb(96, 165, 250),
                        HttpMethod::DELETE => egui::Color32::from_rgb(248, 113, 113),
                        HttpMethod::PATCH => egui::Color32::from_rgb(167, 139, 250),
                        _ => egui::Color32::from_rgb(156, 163, 175),
                    };

                    let bg_color = if is_selected {
                        egui::Color32::from_rgb(45, 140, 230)
                    } else {
                        egui::Color32::from_rgb(35, 35, 35)
                    };

                    ui.add_space(4.0);

                    if ui.add_sized([ui.available_width() - 24.0, 36.0],
                        egui::Button::new(egui::RichText::new(format!("{} {}", req.method, req.name)).size(13.0)
                            .color(if is_selected { egui::Color32::WHITE } else { method_color }))
                            .fill(bg_color)
                            .rounding(6.0)).clicked() {
                        self.selected_folder_path = path.clone();
                        self.selected_request_id = Some(req.id.clone());
                        self.load_request_for_editing();
                        self.response_text.clear();
                        self.response_status.clear();
                        self.response_time.clear();
                    }
                }

                ui.add_space(12.0);

                for subfolder in &folder.subfolders {
                    let mut subpath = path.clone();
                    subpath.push(subfolder.id.clone());
                    ui.add_space(4.0);
                    self.render_folder(ui, subfolder, subpath, 0);
                }
            });
    }
}

impl eframe::App for ApiClient {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(promise) = &self.request_promise {
            if let Some(result) = promise.ready() {
                let (body, status, time) = result.clone();
                self.response_text = body;
                self.response_status = status;
                self.response_time = time;
                self.is_loading = false;
                self.request_promise = None;
            }
        }

        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill = egui::Color32::from_rgb(18, 18, 18);
        style.visuals.panel_fill = egui::Color32::from_rgb(18, 18, 18);
        style.visuals.override_text_color = Some(egui::Color32::from_rgb(230, 230, 230));
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(35, 35, 35);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(45, 45, 45);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(55, 55, 55);
        ctx.set_style(style);

        egui::SidePanel::left("sidebar")
            .min_width(280.0)
            .max_width(400.0)
            .resizable(true)
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(22, 22, 22)))
            .show(ctx, |ui| {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Collections").size(20.0).strong().color(egui::Color32::from_rgb(100, 200, 255)));
            });
            ui.add_space(12.0);

            if ui.add_sized([ui.available_width(), 36.0], egui::Button::new(egui::RichText::new("+ New Collection").size(14.0))
                .fill(egui::Color32::from_rgb(45, 140, 230))
                .rounding(8.0)).clicked() {
                self.state.folders.push(Folder {
                    id: Uuid::new_v4().to_string(),
                    name: format!("Collection {}", self.state.folders.len() + 1),
                    requests: vec![],
                    subfolders: vec![],
                });
                self.save_state();
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                let folders = self.state.folders.clone();
                for (idx, folder) in folders.iter().enumerate() {
                    self.render_folder(ui, folder, vec![folder.id.clone()], idx);
                    ui.add_space(4.0);
                }
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(18, 18, 18)).inner_margin(16.0))
            .show(ctx, |ui| {
            if self.selected_request_id.is_some() {
                ui.add_space(8.0);

                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(28, 28, 28))
                    .inner_margin(16.0)
                    .rounding(10.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Request Name").size(12.0).color(egui::Color32::from_rgb(150, 150, 150)));
                            ui.add_space(8.0);
                            if ui.add(egui::TextEdit::singleline(&mut self.editing_name)
                                .desired_width(ui.available_width() - 100.0)).changed() {
                                let name = self.editing_name.clone();
                                self.update_current_request(|req| {
                                    req.name = name;
                                });
                            }

                            if ui.add(egui::Button::new(egui::RichText::new("Delete Request").size(13.0))
                                .fill(egui::Color32::from_rgb(220, 60, 60))
                                .rounding(6.0)).clicked() {
                                if let Some(req_id) = self.selected_request_id.clone() {
                                    if let Some(folder) = self.get_current_folder_mut() {
                                        folder.requests.retain(|r| r.id != req_id);
                                        self.save_state();
                                        self.selected_request_id = None;
                                        self.editing_name.clear();
                                        self.editing_url.clear();
                                        self.editing_body.clear();
                                        self.editing_headers.clear();
                                        self.response_text.clear();
                                        self.response_status.clear();
                                        self.response_time.clear();
                                    }
                                }
                            }
                        });
                    });

                ui.add_space(12.0);

                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(28, 28, 28))
                    .inner_margin(16.0)
                    .rounding(10.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let method_color = match self.editing_method {
                                HttpMethod::GET => egui::Color32::from_rgb(72, 187, 120),
                                HttpMethod::POST => egui::Color32::from_rgb(251, 146, 60),
                                HttpMethod::PUT => egui::Color32::from_rgb(96, 165, 250),
                                HttpMethod::DELETE => egui::Color32::from_rgb(248, 113, 113),
                                HttpMethod::PATCH => egui::Color32::from_rgb(167, 139, 250),
                                _ => egui::Color32::from_rgb(156, 163, 175),
                            };

                            egui::ComboBox::from_id_salt("method_selector")
                                .selected_text(egui::RichText::new(format!("{}", self.editing_method)).color(method_color).strong().size(14.0))
                                .width(90.0)
                                .show_ui(ui, |ui| {
                                    for method in [HttpMethod::GET, HttpMethod::POST, HttpMethod::PUT,
                                                 HttpMethod::DELETE, HttpMethod::PATCH, HttpMethod::HEAD, HttpMethod::OPTIONS] {
                                        if ui.selectable_value(&mut self.editing_method, method.clone(), format!("{}", method)).clicked() {
                                            let m = self.editing_method.clone();
                                            self.update_current_request(|req| {
                                                req.method = m;
                                            });
                                        }
                                    }
                                });

                            ui.add_space(4.0);

                            if ui.add(egui::TextEdit::singleline(&mut self.editing_url)
                                .desired_width(ui.available_width() - 110.0)
                                .hint_text("https://api.example.com/endpoint")
                                .font(egui::TextStyle::Monospace)).changed() {
                                let url = self.editing_url.clone();
                                self.update_current_request(|req| {
                                    req.url = url;
                                });
                            }

                            ui.add_space(4.0);

                            if ui.add(egui::Button::new(egui::RichText::new("Send").size(14.0).strong())
                                .fill(egui::Color32::from_rgb(45, 140, 230))
                                .min_size(egui::vec2(90.0, 36.0))
                                .rounding(8.0)).clicked() {
                                let body = self.editing_body.clone();
                                let headers = self.editing_headers.clone();
                                self.update_current_request(|req| {
                                    req.body = body;
                                    req.headers = headers;
                                });
                                self.send_request();
                            }
                        });
                    });

                ui.add_space(12.0);

                let available_height = ui.available_height();
                let request_section_height = (available_height * 0.30).max(120.0);

                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(28, 28, 28))
                    .inner_margin(16.0)
                    .rounding(10.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(request_section_height)
                            .show(ui, |ui| {
                                ui.collapsing(egui::RichText::new("Headers").size(14.0).strong(), |ui| {
                                    ui.add_space(8.0);
                                    let mut to_remove = None;
                                    for (i, (key, value)) in self.editing_headers.iter_mut().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.add(egui::TextEdit::singleline(key).desired_width(180.0).hint_text("Header name"));
                                            ui.add(egui::TextEdit::singleline(value).desired_width(ui.available_width() - 50.0).hint_text("Header value"));
                                            if ui.add(egui::Button::new("X").fill(egui::Color32::from_rgb(220, 60, 60)).rounding(4.0)).clicked() {
                                                to_remove = Some(i);
                                            }
                                        });
                                        ui.add_space(4.0);
                                    }

                                    if let Some(i) = to_remove {
                                        self.editing_headers.remove(i);
                                        let headers = self.editing_headers.clone();
                                        self.update_current_request(|req| {
                                            req.headers = headers;
                                        });
                                    }

                                    ui.add_space(8.0);
                                    ui.horizontal(|ui| {
                                        ui.add(egui::TextEdit::singleline(&mut self.new_header_key).desired_width(180.0).hint_text("New header"));
                                        ui.add(egui::TextEdit::singleline(&mut self.new_header_value).desired_width(ui.available_width() - 100.0).hint_text("Value"));
                                        if ui.add(egui::Button::new(egui::RichText::new("+ Add").size(12.0))
                                            .fill(egui::Color32::from_rgb(45, 140, 230))
                                            .rounding(6.0)).clicked() && !self.new_header_key.is_empty() {
                                            self.editing_headers.push((self.new_header_key.clone(), self.new_header_value.clone()));
                                            self.new_header_key.clear();
                                            self.new_header_value.clear();
                                            let headers = self.editing_headers.clone();
                                            self.update_current_request(|req| {
                                                req.headers = headers;
                                            });
                                        }
                                    });
                                });

                                ui.add_space(12.0);

                                ui.collapsing(egui::RichText::new("Body").size(14.0).strong(), |ui| {
                                    ui.add_space(8.0);
                                    if ui.add(egui::TextEdit::multiline(&mut self.editing_body)
                                        .desired_width(ui.available_width())
                                        .desired_rows(8)
                                        .code_editor()
                                        .font(egui::TextStyle::Monospace)).changed() {
                                        let body = self.editing_body.clone();
                                        self.update_current_request(|req| {
                                            req.body = body;
                                        });
                                    }
                                });
                            });
                    });

                ui.add_space(16.0);

                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(28, 28, 28))
                    .inner_margin(0.0)
                    .rounding(10.0)
                    .show(ui, |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(35, 35, 35))
                            .inner_margin(egui::vec2(16.0, 12.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Response").size(16.0).strong().color(egui::Color32::from_rgb(100, 200, 255)));
                                    ui.add_space(16.0);

                                    let status_color = if self.response_status.starts_with("2") {
                                        egui::Color32::from_rgb(72, 187, 120)
                                    } else if self.response_status.starts_with("3") {
                                        egui::Color32::from_rgb(251, 146, 60)
                                    } else if self.response_status.starts_with("4") || self.response_status.starts_with("5") {
                                        egui::Color32::from_rgb(248, 113, 113)
                                    } else {
                                        egui::Color32::from_rgb(156, 163, 175)
                                    };

                                    egui::Frame::none()
                                        .fill(status_color.linear_multiply(0.2))
                                        .inner_margin(egui::vec2(12.0, 6.0))
                                        .rounding(6.0)
                                        .show(ui, |ui| {
                                            ui.label(egui::RichText::new(&self.response_status).color(status_color).strong().size(13.0));
                                        });

                                    ui.add_space(12.0);

                                    egui::Frame::none()
                                        .fill(egui::Color32::from_rgb(45, 45, 45))
                                        .inner_margin(egui::vec2(12.0, 6.0))
                                        .rounding(6.0)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new("Time:").size(12.0));
                                                ui.label(egui::RichText::new(&self.response_time).color(egui::Color32::from_rgb(180, 180, 180)).size(13.0));
                                            });
                                        });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.add(egui::Button::new(egui::RichText::new("Copy").size(13.0))
                                            .fill(egui::Color32::from_rgb(45, 140, 230))
                                            .rounding(6.0)
                                            .min_size(egui::vec2(80.0, 28.0))).clicked() {
                                            ui.output_mut(|o| o.copied_text = self.response_text.clone());
                                        }
                                    });
                                });
                            });

                        let remaining_height = ui.available_height() - 16.0;

                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(22, 22, 22))
                            .inner_margin(0.0)
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .max_height(remaining_height)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.add_space(16.0);
                                        ui.horizontal(|ui| {
                                            ui.add_space(16.0);
                                            ui.add(
                                                egui::TextEdit::multiline(&mut self.response_text.as_str())
                                                    .code_editor()
                                                    .desired_width(ui.available_width() - 16.0)
                                                    .font(egui::TextStyle::Monospace)
                                            );
                                        });
                                        ui.add_space(16.0);
                                    });
                            });
                    });

            } else {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(egui::RichText::new("RUSTY REQUESTER").size(32.0).strong().color(egui::Color32::from_rgb(100, 200, 255)));
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Select a request to get started").size(18.0).color(egui::Color32::from_rgb(120, 120, 120)));
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("or create a new one from the sidebar").size(14.0).color(egui::Color32::from_rgb(100, 100, 100)));
                    });
                });
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Rusty Requester",
        options,
        Box::new(|_cc| Ok(Box::new(ApiClient::default()))),
    )
}
