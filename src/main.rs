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
    
    // UI state
    new_header_key: String,
    new_header_value: String,
    response_text: String,
    response_status: String,
    response_time: String,
    is_loading: bool,
    
    // Edit states
    editing_url: String,
    editing_body: String,
    editing_name: String,
    editing_method: HttpMethod,
    editing_headers: Vec<(String, String)>,
    
    storage_path: PathBuf,
    
    // Promise for async requests
    request_promise: Option<Promise<(String, String, String)>>,
    
    // Rename states
    renaming_folder_id: Option<String>,
    rename_folder_text: String,
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
            
            // Create a promise that will execute the request in a background thread
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
                    
                    // Try to format as JSON if possible
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
}

impl eframe::App for ApiClient {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if we have a completed request
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
        
        // Custom styling
        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill = egui::Color32::from_rgb(26, 27, 38);
        style.visuals.panel_fill = egui::Color32::from_rgb(26, 27, 38);
        style.visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 230));
        ctx.set_style(style);
        
        egui::SidePanel::left("sidebar")
            .min_width(280.0)
            .resizable(true)
            .show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("📁 Collections").size(18.0).color(egui::Color32::from_rgb(139, 233, 253)));
            });
            ui.add_space(5.0);
            ui.separator();
            ui.add_space(10.0);
            
            if ui.add_sized([ui.available_width(), 32.0], egui::Button::new(egui::RichText::new("➕ New Folder").size(14.0))
                .fill(egui::Color32::from_rgb(56, 189, 100))
                .stroke(egui::Stroke::NONE)).clicked() {
                self.state.folders.push(Folder {
                    id: Uuid::new_v4().to_string(),
                    name: format!("Folder {}", self.state.folders.len() + 1),
                    requests: vec![],
                    subfolders: vec![],
                });
                self.save_state();
            }
            
            ui.add_space(10.0);
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                let folders = self.state.folders.clone();
                for folder in &folders {
                    self.render_folder(ui, folder, vec![folder.id.clone()]);
                }
            });
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.selected_request_id.is_some() {
                ui.add_space(10.0);
                
                // Request name section with delete button
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(40, 42, 54))
                    .inner_margin(12.0)
                    .rounding(6.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Name:").size(14.0).color(egui::Color32::from_rgb(139, 233, 253)));
                            if ui.add(egui::TextEdit::singleline(&mut self.editing_name)
                                .desired_width(ui.available_width() - 80.0)
                                .font(egui::TextStyle::Body)).changed() {
                                let name = self.editing_name.clone();
                                self.update_current_request(|req| {
                                    req.name = name;
                                });
                            }
                            
                            let delete_button = egui::Button::new(egui::RichText::new("🗑 Delete").size(13.0))
                                .fill(egui::Color32::from_rgb(255, 85, 85))
                                .stroke(egui::Stroke::NONE);
                            
                            if ui.add(delete_button).clicked() {
                                // Delete current request
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
                
                ui.add_space(10.0);
                
                // Request URL section
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(40, 42, 54))
                    .inner_margin(12.0)
                    .rounding(6.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let method_color = match self.editing_method {
                                HttpMethod::GET => egui::Color32::from_rgb(80, 250, 123),
                                HttpMethod::POST => egui::Color32::from_rgb(255, 184, 108),
                                HttpMethod::PUT => egui::Color32::from_rgb(139, 233, 253),
                                HttpMethod::DELETE => egui::Color32::from_rgb(255, 121, 198),
                                HttpMethod::PATCH => egui::Color32::from_rgb(189, 147, 249),
                                _ => egui::Color32::from_rgb(98, 114, 164),
                            };
                            
                            egui::ComboBox::from_label("")
                                .selected_text(egui::RichText::new(format!("{}", self.editing_method)).color(method_color).strong().size(14.0))
                                .width(80.0)
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
                            
                            if ui.add(egui::TextEdit::singleline(&mut self.editing_url)
                                .desired_width(ui.available_width() - 100.0)
                                .hint_text("https://api.example.com/endpoint")
                                .font(egui::TextStyle::Monospace)).changed() {
                                let url = self.editing_url.clone();
                                self.update_current_request(|req| {
                                    req.url = url;
                                });
                            }
                            
                            let send_button = egui::Button::new(egui::RichText::new("Send").size(14.0).strong())
                                .fill(egui::Color32::from_rgb(189, 147, 249))
                                .min_size(egui::vec2(80.0, 30.0));
                            
                            if ui.add(send_button).clicked() {
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
                
                ui.add_space(10.0);
                
                // Headers and Body sections in tabs-like style
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    
                    if ui.add(egui::Button::new(egui::RichText::new("Headers").size(13.0))
                        .fill(egui::Color32::from_rgb(40, 42, 54))).clicked() {
                        // Headers section is always visible below
                    }
                    
                    if ui.add(egui::Button::new(egui::RichText::new("Body").size(13.0))
                        .fill(egui::Color32::from_rgb(40, 42, 54))).clicked() {
                        // Body section is always visible below
                    }
                });
                
                ui.add_space(5.0);
                
                // Request details section with better height allocation
                let available_height = ui.available_height();
                let request_section_height = (available_height * 0.35).max(150.0);
                
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(40, 42, 54))
                    .inner_margin(12.0)
                    .rounding(6.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(request_section_height)
                            .show(ui, |ui| {
                                ui.collapsing(egui::RichText::new("📝 Headers").size(14.0), |ui| {
                                    let mut to_remove = None;
                                    for (i, (key, value)) in self.editing_headers.iter_mut().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.add(egui::TextEdit::singleline(key)
                                                .desired_width(150.0)
                                                .hint_text("Key"));
                                            ui.add(egui::TextEdit::singleline(value)
                                                .desired_width(ui.available_width() - 40.0)
                                                .hint_text("Value"));
                                            if ui.button("🗑").clicked() {
                                                to_remove = Some(i);
                                            }
                                        });
                                    }
                                    
                                    if let Some(i) = to_remove {
                                        self.editing_headers.remove(i);
                                        let headers = self.editing_headers.clone();
                                        self.update_current_request(|req| {
                                            req.headers = headers;
                                        });
                                    }
                                    
                                    ui.add_space(5.0);
                                    ui.horizontal(|ui| {
                                        ui.add(egui::TextEdit::singleline(&mut self.new_header_key)
                                            .desired_width(150.0)
                                            .hint_text("New key"));
                                        ui.add(egui::TextEdit::singleline(&mut self.new_header_value)
                                            .desired_width(ui.available_width() - 90.0)
                                            .hint_text("New value"));
                                        if ui.button(egui::RichText::new("➕ Add").size(12.0)).clicked() && !self.new_header_key.is_empty() {
                                            self.editing_headers.push((
                                                self.new_header_key.clone(),
                                                self.new_header_value.clone(),
                                            ));
                                            self.new_header_key.clear();
                                            self.new_header_value.clear();
                                            
                                            let headers = self.editing_headers.clone();
                                            self.update_current_request(|req| {
                                                req.headers = headers;
                                            });
                                        }
                                    });
                                });
                                
                                ui.add_space(8.0);
                                
                                ui.collapsing(egui::RichText::new("📄 Body").size(14.0), |ui| {
                                    if ui.add(egui::TextEdit::multiline(&mut self.editing_body)
                                        .desired_width(ui.available_width())
                                        .desired_rows(6)
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
                
                ui.add_space(15.0);
                
                // Response section with much more space
                ui.label(egui::RichText::new("Response").size(16.0).strong().color(egui::Color32::from_rgb(139, 233, 253)));
                ui.add_space(5.0);
                
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(40, 42, 54))
                    .inner_margin(12.0)
                    .rounding(6.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let status_color = if self.response_status.starts_with("2") {
                                egui::Color32::from_rgb(80, 250, 123)
                            } else if self.response_status.starts_with("3") {
                                egui::Color32::from_rgb(255, 184, 108)
                            } else if self.response_status.starts_with("4") || self.response_status.starts_with("5") {
                                egui::Color32::from_rgb(255, 85, 85)
                            } else {
                                egui::Color32::from_rgb(98, 114, 164)
                            };
                            
                            ui.label(egui::RichText::new("Status:").size(13.0).strong());
                            ui.label(egui::RichText::new(&self.response_status).color(status_color).strong().size(13.0));
                            
                            ui.separator();
                            
                            ui.label(egui::RichText::new("⏱").size(13.0));
                            ui.label(egui::RichText::new(&self.response_time).color(egui::Color32::from_rgb(139, 233, 253)).size(13.0));
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(egui::RichText::new("📋 Copy").size(12.0)).clicked() {
                                    ui.output_mut(|o| o.copied_text = self.response_text.clone());
                                }
                            });
                        });
                    });
                
                ui.add_space(5.0);
                
                // Response body - takes remaining space
                let remaining_height = ui.available_height() - 10.0;
                
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 31, 41))
                    .inner_margin(12.0)
                    .rounding(6.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 71, 90)))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(remaining_height)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.response_text.as_str())
                                        .code_editor()
                                        .desired_width(f32::INFINITY)
                                        .font(egui::TextStyle::Monospace)
                                );
                            });
                    });
                
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new("Select or create a request to get started").size(16.0).color(egui::Color32::from_rgb(98, 114, 164)));
                });
            }
        });
    }
}

impl ApiClient {
    fn render_folder(&mut self, ui: &mut egui::Ui, folder: &Folder, path: Vec<String>) {
        let is_renaming = self.renaming_folder_id.as_ref() == Some(&folder.id);
        
        let header_response = egui::CollapsingHeader::new(
            if is_renaming {
                egui::RichText::new("").size(13.0)
            } else {
                egui::RichText::new(format!("📁 {}", folder.name)).size(13.0)
            }
        )
        .id_salt(&folder.id)
        .default_open(true)
        .show(ui, |ui| {
            ui.add_space(5.0);
            
            if ui.add_sized([ui.available_width(), 28.0], egui::Button::new(egui::RichText::new("➕ New Request").size(12.0))
                .fill(egui::Color32::from_rgb(68, 71, 90))
                .stroke(egui::Stroke::NONE)).clicked() {
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
            
            ui.add_space(5.0);
            
            for req in &folder.requests {
                let is_selected = self.selected_request_id.as_ref() == Some(&req.id);
                
                let method_color = match req.method {
                    HttpMethod::GET => egui::Color32::from_rgb(80, 250, 123),
                    HttpMethod::POST => egui::Color32::from_rgb(255, 184, 108),
                    HttpMethod::PUT => egui::Color32::from_rgb(139, 233, 253),
                    HttpMethod::DELETE => egui::Color32::from_rgb(255, 121, 198),
                    HttpMethod::PATCH => egui::Color32::from_rgb(189, 147, 249),
                    _ => egui::Color32::from_rgb(98, 114, 164),
                };
                
                let bg_color = if is_selected {
                    egui::Color32::from_rgb(68, 71, 90)
                } else {
                    egui::Color32::from_rgb(40, 42, 54)
                };
                
                let button = egui::Button::new(
                    egui::RichText::new(format!("{} {}", req.method, req.name))
                        .size(12.0)
                )
                .fill(bg_color)
                .stroke(if is_selected {
                    egui::Stroke::new(1.0, method_color)
                } else {
                    egui::Stroke::NONE
                });
                
                if ui.add_sized([ui.available_width(), 32.0], button).clicked() {
                    self.selected_folder_path = path.clone();
                    self.selected_request_id = Some(req.id.clone());
                    self.load_request_for_editing();
                    self.response_text.clear();
                    self.response_status.clear();
                    self.response_time.clear();
                }
                
                ui.add_space(3.0);
            }
            
            for subfolder in &folder.subfolders {
                let mut subpath = path.clone();
                subpath.push(subfolder.id.clone());
                self.render_folder(ui, subfolder, subpath);
            }
        });
        
        // Show rename field and buttons outside the collapsing header
        if is_renaming {
            // Position the rename UI where the header text would be
            let rect = header_response.header_response.rect;
            let mut rename_rect = rect;
            rename_rect.min.x += 25.0; // Offset for the collapse arrow and icon
            
            let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rename_rect));
            child_ui.horizontal(|ui| {
                let text_edit = egui::TextEdit::singleline(&mut self.rename_folder_text)
                    .desired_width(150.0)
                    .font(egui::TextStyle::Body);
                
                let response = ui.add(text_edit);
                
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // Save the rename
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
            // Add context menu for renaming
            header_response.header_response.context_menu(|ui| {
                if ui.button("✏ Rename").clicked() {
                    self.renaming_folder_id = Some(folder.id.clone());
                    self.rename_folder_text = folder.name.clone();
                    ui.close_menu();
                }
            });
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
