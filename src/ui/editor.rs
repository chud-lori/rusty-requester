//! Central panel — tab bar at top, URL bar, request-editor tabs
//! (Params / Headers / Cookies / Body / Auth / Tests) stacked above a
//! drag-resizable split with the response panel.

use crate::io::curl;
use crate::model::*;
use crate::theme::*;
use crate::widgets::*;
use crate::ApiClient;
use eframe::egui;

impl ApiClient {
    pub(crate) fn render_central(&mut self, ctx: &egui::Context) {
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

}
