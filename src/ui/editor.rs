//! Central panel — tab bar at top, URL bar, request-editor tabs
//! (Params / Headers / Cookies / Body / Auth / Tests) stacked above a
//! drag-resizable split with the response panel.

use crate::io::curl;
use crate::model::*;
use crate::snippet::build_json_layout_job_content_only_with_search;
use crate::theme::*;
use crate::widgets::*;
use crate::ApiClient;
use eframe::egui;

impl ApiClient {
    pub(crate) fn render_central(&mut self, ctx: &egui::Context) {
        let theme_bg = crate::theme::palette_for(self.state.settings.theme).bg;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme_bg)
                    .inner_margin(egui::Margin::symmetric(0.0, 0.0)),
            )
            .show(ctx, |ui| {
                // Defensive floor: paint the full panel rect with the
                // theme bg BEFORE any children render. Without this, if
                // any child widget (scroll track, code editor, etc.)
                // leaves a sub-region with an un-themed dark/transparent
                // fill, the OS default (near-black) bleeds through as a
                // "black strip".
                ui.painter()
                    .rect_filled(ui.max_rect(), egui::Rounding::ZERO, theme_bg);
                self.render_tabs_bar(ui);

                // Collection overview — activated via sidebar folder's
                // `⋯` menu or right-click → "Open overview". Shows folder
                // title / stats / editable description instead of the
                // request editor. Dismissed automatically when the user
                // opens any request (see `open_request`).
                if self.viewing_folder_id.is_some() {
                    self.render_collection_overview(ui);
                    return;
                }

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
                                    .color(text()),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    "Pick a request from the sidebar, or create a new one.",
                                )
                                .size(13.0)
                                .color(muted()),
                            );
                        });
                    });
                    return;
                }

                egui::Frame::none()
                    .inner_margin(egui::Margin {
                        // Tight left margin — sidebar ends here, we want central
                        // content to start right after without a visible dead
                        // zone. The background layer paints bg() underneath so
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
                        let max_req = (total_h - drag_handle_h - min_resp).max(min_req);
                        self.request_split_px = self.request_split_px.clamp(min_req, max_req);
                        let req_h = self.request_split_px;

                        // Top: request editor
                        ui.allocate_ui(egui::vec2(ui.available_width(), req_h), |ui| {
                            self.render_request_tabs(ui);
                        });

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
                            let line_color = if handle_resp.hovered() || handle_resp.dragged() {
                                C_ACCENT
                            } else {
                                border()
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
        //
        // Bg matches the page (not `panel_dark`) — Postman-style flat
        // chrome. Active tab fill alone signals "selected", no darker
        // horizontal seam between the tab strip and the content below.
        let bar_height = 38.0;
        egui::Frame::none()
            .fill(crate::theme::bg())
            .inner_margin(egui::Margin {
                left: 10.0,
                right: 10.0,
                // macOS: extra top padding so the tab strip clears the
                // traffic-light window controls when the title bar is
                // merged into the chrome.
                top: if cfg!(target_os = "macos") { 28.0 } else { 4.0 },
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
                let mut duplicate: Option<usize> = None;
                let mut toggle_pin: Option<usize> = None;

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    // Reserve the right-side slot for a pinned `+` button
                    // so it's always visible, even when tabs overflow.
                    // The tab ScrollArea takes `available_width - plus_slot`
                    // and scrolls within that; the `+` sits outside the
                    // scroll area. Natural affordance that tabs scroll
                    // past the pinned button, no gradient needed.
                    let plus_slot = 36.0;
                    let scroll_width = (ui.available_width() - plus_slot).max(80.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(scroll_width, bar_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
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
                                                tab.pinned,
                                            );
                                            match action {
                                                TabAction::Activate => activate = Some(i),
                                                TabAction::Close => close = Some(i),
                                                TabAction::CloseOthers => close_others = Some(i),
                                                TabAction::CloseAll => close_all = true,
                                                TabAction::SaveDraft => save_draft = Some(i),
                                                TabAction::Duplicate => duplicate = Some(i),
                                                TabAction::TogglePin => toggle_pin = Some(i),
                                                TabAction::None => {}
                                            }
                                        }
                                    });
                                });
                        },
                    );

                    // Pinned `+` outside the ScrollArea — always visible
                    // regardless of how many tabs are open.
                    let (plus_rect, plus_resp) =
                        ui.allocate_exact_size(egui::vec2(30.0, 28.0), egui::Sense::click());
                    if ui.is_rect_visible(plus_rect) {
                        let hovered = plus_resp.hovered();
                        if hovered {
                            ui.painter().rect_filled(
                                plus_rect,
                                egui::Rounding::same(4.0),
                                crate::theme::elevated(),
                            );
                        }
                        let color = if hovered {
                            crate::theme::text()
                        } else {
                            crate::theme::muted()
                        };
                        ui.painter().text(
                            plus_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "+",
                            egui::FontId::new(18.0, egui::FontFamily::Proportional),
                            color,
                        );
                    }
                    if plus_resp
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("New request (unsaved)")
                        .clicked()
                    {
                        new_draft = true;
                    }
                });

                if let Some(i) = activate {
                    if let Some(tab) = self.state.open_tabs.get(i).cloned() {
                        // Route through `open_request` so the per-tab
                        // response cache stash/restore fires — the
                        // previous inline clear path wiped the response
                        // on every tab switch.
                        self.open_request(tab.folder_path, tab.request_id);
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
                if let Some(idx) = duplicate {
                    self.duplicate_tab(idx);
                }
                if let Some(idx) = toggle_pin {
                    if let Some(tab) = self.state.open_tabs.get_mut(idx) {
                        tab.pinned = !tab.pinned;
                        self.save_state();
                    }
                }
            });
    }

    /// Open the "Save to folder" modal for the draft at tab index `idx`.
    fn render_url_bar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(bg())
            .inner_margin(12.0)
            .rounding(10.0)
            .stroke(egui::Stroke::new(1.0, border()))
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

                    // Reserve space for Send + Code buttons (~180 px).
                    let btn_space = 180.0;
                    let avail = (ui.available_width() - btn_space).max(200.0);
                    let url_edit = ui.add(
                        egui::TextEdit::singleline(&mut self.editing_url)
                            .id_source("url_bar_edit")
                            .desired_width(avail)
                            .hint_text(hint(
                                "https://api.example.com/endpoint  (or paste a cURL command)",
                            ))
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
                                    self.editing_url =
                                        curl::build_full_url(&parsed.url, &parsed.query_params);
                                    self.editing_params = parsed.query_params;
                                    self.editing_headers = parsed.headers;
                                    self.editing_cookies = parsed.cookies;
                                    self.editing_body = parsed.body;
                                    self.editing_auth = parsed.auth;
                                    self.commit_editing();
                                    self.show_toast("Imported cURL into request");
                                }
                                Err(_) => {
                                    let (base, parsed_params) = curl::split_url(&self.editing_url);
                                    self.editing_params = parsed_params;
                                    let params = self.editing_params.clone();
                                    self.update_current_request(|req| {
                                        req.url = base;
                                        req.query_params = params;
                                    });
                                }
                            }
                        } else {
                            // Postman-style URL↔Params sync: parse
                            // query params from the URL bar into the
                            // Params table. editing_url keeps the full
                            // URL; commit_editing splits it for storage.
                            let (base, parsed_params) = curl::split_url(&self.editing_url);
                            self.editing_params = parsed_params;
                            let params = self.editing_params.clone();
                            self.update_current_request(|req| {
                                req.url = base;
                                req.query_params = params;
                            });
                        }
                    }

                    let send_pressed =
                        url_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    // The Send button flips to Cancel while a request
                    // is in flight. `is_loading` stays true for the
                    // entire duration of the tokio task; clicking
                    // Cancel calls `abort()` on the JoinHandle and
                    // drops the connection.
                    let (label, fill, tooltip) = if self.is_loading {
                        ("Cancel", C_RED, "Cancel the in-flight request")
                    } else {
                        // Brand-accent rust orange — matches "New
                        // Collection" + active-tab underline. Was
                        // C_PURPLE (burnt-sienna PATCH color) which
                        // made Send look like a different family
                        // from the rest of the primary CTAs.
                        ("Send", C_ACCENT, "Send (⌘/Ctrl + Enter)")
                    };
                    let send_btn = egui::Button::new(
                        egui::RichText::new(label)
                            .size(13.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(fill)
                    .min_size(egui::vec2(80.0, 28.0));

                    let send_click = ui
                        .add(send_btn)
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(tooltip)
                        .clicked();

                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("</> Code").size(12.0))
                                .fill(border())
                                .min_size(egui::vec2(74.0, 28.0)),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Toggle code-snippet panel")
                        .clicked()
                    {
                        self.commit_editing();
                        self.show_snippet_panel = !self.show_snippet_panel;
                    }

                    // One click handler for both states: Cancel when
                    // loading, Send otherwise. ⌘/Ctrl+Enter and
                    // Enter-in-URL-bar always Send (never Cancel).
                    if send_click {
                        if self.is_loading {
                            self.cancel_request();
                        } else {
                            self.send_request();
                        }
                    } else if send_pressed && !self.is_loading {
                        self.send_request();
                    }
                });
            });
    }

    fn render_request_tabs(&mut self, ui: &mut egui::Ui) {
        // Tab count labels — skip the trailing blank "ghost" row that
        // `render_kv_table` appends for the user to type into. Without
        // this filter, a brand-new request shows "Params (1)" even
        // though the row is empty.
        let params_count = self.editing_params.iter().filter(|r| !r.is_blank()).count();
        let headers_count = self
            .editing_headers
            .iter()
            .filter(|r| !r.is_blank())
            .count();
        let cookies_count = self
            .editing_cookies
            .iter()
            .filter(|r| !r.is_blank())
            .count();
        let params_label = if params_count == 0 {
            "Params".to_string()
        } else {
            format!("Params ({})", params_count)
        };
        let headers_label = if headers_count == 0 {
            "Headers".to_string()
        } else {
            format!("Headers ({})", headers_count)
        };
        let cookies_label = if cookies_count == 0 {
            "Cookies".to_string()
        } else {
            format!("Cookies ({})", cookies_count)
        };
        let body_label = if self.editing_body.is_empty() {
            "Body".to_string()
        } else {
            "Body •".to_string()
        };
        let auth_label = match &self.editing_auth {
            Auth::None => "Auth".to_string(),
            Auth::Bearer { .. } => "Auth (Bearer)".to_string(),
            Auth::Basic { .. } => "Auth (Basic)".to_string(),
            Auth::OAuth2(_) => "Auth (OAuth 2.0)".to_string(),
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
            tab_button(
                ui,
                &mut self.request_tab,
                RequestTab::Headers,
                &headers_label,
            );
            tab_button(
                ui,
                &mut self.request_tab,
                RequestTab::Cookies,
                &cookies_label,
            );
            tab_button(ui, &mut self.request_tab, RequestTab::Body, &body_label);
            tab_button(ui, &mut self.request_tab, RequestTab::Auth, &auth_label);
            tab_button(ui, &mut self.request_tab, RequestTab::Tests, &tests_label);
        });

        // Use whatever vertical space we've been given by the caller
        // (render_central allocates the request section with a fixed
        // height so the user can drag-resize the split).
        egui::Frame::none()
            .fill(bg())
            .inner_margin(12.0)
            .rounding(10.0)
            .stroke(egui::Stroke::new(1.0, border()))
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
            // Reverse sync: rebuild the URL bar's query string from
            // the table so both views stay in sync (Postman-style).
            let (base, _) = curl::split_url(&self.editing_url);
            self.editing_url = curl::build_full_url(&base, &self.editing_params);
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
        // Postman-style radio row — native radio circles, no saturated
        // fills or a redundant "Body type" label. The options label
        // themselves ("Raw", "x-www-form-urlencoded", ...) are
        // self-descriptive.
        ui.horizontal(|ui| {
            for &m in &[
                BodyMode::Raw,
                BodyMode::FormUrlEncoded,
                BodyMode::MultipartForm,
                BodyMode::GraphQL,
            ] {
                ui.radio_value(&mut new_mode, m, m.label());
                ui.add_space(6.0);
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
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Postman-style subtle action links on the right, not
                // tacked-on buttons.
                if ui
                    .link(egui::RichText::new("Beautify").size(11.5).color(C_ACCENT))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Pretty-print JSON")
                    .clicked()
                {
                    prettify = true;
                }
                ui.add_space(8.0);
                if ui
                    .link(egui::RichText::new("Minify").size(11.5).color(muted()))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Collapse JSON to one line")
                    .clicked()
                {
                    minify = true;
                }
                ui.add_space(12.0);
                let size_label = if self.editing_body.is_empty() {
                    "empty".to_string()
                } else {
                    format!("{} bytes", self.editing_body.len())
                };
                ui.label(egui::RichText::new(size_label).size(11.0).color(muted()))
                    .on_hover_text(
                        "Size in UTF-8 bytes — exactly what goes on the wire.\n\
                     ASCII chars are 1 byte each; accented / non-Latin chars \
                     can be 2-4 bytes.",
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
        // Two-column layout: left gutter shows one line number per
        // logical line of the body; right column is the editor itself.
        // Matches the snippet-panel pattern so wrapped long lines in
        // the body stay inside the editor column instead of colliding
        // with the gutter.
        let avail_h = (ui.available_height() - 4.0).max(80.0);
        let line_count = self.editing_body.split('\n').count().max(1);
        let mut body_changed = false;
        ui.horizontal_top(|ui| {
            let gutter_w = 34.0;
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.set_width(gutter_w);
                for i in 1..=line_count {
                    ui.add_sized(
                        [gutter_w, 17.0],
                        egui::Label::new(
                            egui::RichText::new(format!("{:>3}", i))
                                .color(egui::Color32::from_rgb(100, 105, 115))
                                .font(egui::FontId::monospace(12.5)),
                        ),
                    );
                }
            });
            ui.add_space(4.0);
            // Syntax highlight via the same JSON layouter the response
            // body uses — keys blue/green, strings yellow/navy, numbers
            // purple/orange, true/false/null pink/red. For non-JSON
            // raw text the layouter just tokenizes quoted strings and
            // numbers, which is a sensible fallback for most bodies.
            let mut layouter = |ui: &egui::Ui, s: &str, wrap_width: f32| {
                let mut job = build_json_layout_job_content_only_with_search(s, "");
                job.wrap.max_width = wrap_width;
                ui.fonts(|f| f.layout_job(job))
            };
            if ui
                .add_sized(
                    [ui.available_width(), avail_h],
                    egui::TextEdit::multiline(&mut self.editing_body)
                        .frame(false)
                        .hint_text(hint("Request body (JSON, text, ...)"))
                        .font(egui::TextStyle::Monospace)
                        .layouter(&mut layouter),
                )
                .changed()
            {
                body_changed = true;
            }
        });
        if body_changed {
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
            .color(muted()),
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
                .color(muted()),
        );
        ui.add_space(4.0);

        let avail_h = ui.available_height();
        let query_h = (avail_h * 0.6).max(80.0);
        let vars_h = (avail_h - query_h - 30.0).max(60.0);

        ui.label(
            egui::RichText::new("Query")
                .size(11.0)
                .strong()
                .color(text()),
        );
        if ui
            .add_sized(
                [ui.available_width(), query_h],
                egui::TextEdit::multiline(&mut self.editing_body)
                    .code_editor()
                    .hint_text(hint("query MyQuery { ... }"))
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
                .color(text()),
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
                    .hint_text(hint("{ \"id\": 123 }"))
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
                .color(muted()),
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
                    AuthKind::OAuth2 => "OAuth 2.0",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut kind, AuthKind::None, "No Auth");
                    ui.selectable_value(&mut kind, AuthKind::Bearer, "Bearer Token");
                    ui.selectable_value(&mut kind, AuthKind::Basic, "Basic Auth");
                    ui.selectable_value(&mut kind, AuthKind::OAuth2, "OAuth 2.0");
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
                AuthKind::OAuth2 => match &self.editing_auth {
                    Auth::OAuth2(s) => Auth::OAuth2(s.clone()),
                    _ => Auth::OAuth2(Box::default()),
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
                        .color(muted())
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
                            .hint_text(hint("eyJhbGciOi...")),
                    )
                    .changed()
                {
                    changed = true;
                }
                // JWT decoder — if the token looks like `header.payload.sig`
                // with 2 dots and each part is base64url-ish, render the
                // decoded header + payload below.
                if let Some((header_json, payload_json)) = try_decode_jwt(token) {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Decoded JWT")
                            .size(11.0)
                            .strong()
                            .color(muted()),
                    );
                    ui.add_space(4.0);
                    egui::CollapsingHeader::new(
                        egui::RichText::new("Header").color(text()).size(12.5),
                    )
                    .default_open(true)
                    .show(ui, |ui| {
                        let mut s: &str = &header_json;
                        ui.add(
                            egui::TextEdit::multiline(&mut s)
                                .code_editor()
                                .desired_rows(3)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
                    egui::CollapsingHeader::new(
                        egui::RichText::new("Payload").color(text()).size(12.5),
                    )
                    .default_open(true)
                    .show(ui, |ui| {
                        let mut s: &str = &payload_json;
                        ui.add(
                            egui::TextEdit::multiline(&mut s)
                                .code_editor()
                                .desired_rows(6)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
                    ui.label(
                        egui::RichText::new(
                            "Signature is not verified — we just base64-decode the header and payload.",
                        )
                        .size(10.5)
                        .color(muted()),
                    );
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
            Auth::OAuth2(s) => {
                // Config form — six fields. Stored persistently on
                // the request; `Get New Token` uses these to drive
                // the PKCE flow.
                fn field(
                    ui: &mut egui::Ui,
                    label: &str,
                    value: &mut String,
                    hint: &str,
                    password: bool,
                ) -> bool {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(label).color(C_ACCENT));
                        let mut edit = egui::TextEdit::singleline(value)
                            .desired_width(ui.available_width() - 140.0)
                            .hint_text(hint);
                        if password {
                            edit = edit.password(true);
                        }
                        ui.add(edit).changed()
                    })
                    .inner
                }
                changed |= field(
                    ui,
                    "Auth URL",
                    &mut s.config.auth_url,
                    "https://provider.example.com/oauth/authorize",
                    false,
                );
                changed |= field(
                    ui,
                    "Token URL",
                    &mut s.config.token_url,
                    "https://provider.example.com/oauth/token",
                    false,
                );
                changed |= field(ui, "Client ID", &mut s.config.client_id, "", false);
                changed |= field(
                    ui,
                    "Client secret",
                    &mut s.config.client_secret,
                    "(public / PKCE clients: leave empty)",
                    true,
                );
                changed |= field(
                    ui,
                    "Scope",
                    &mut s.config.scope,
                    "read:all write:all",
                    false,
                );
                changed |= field(
                    ui,
                    "Redirect URI",
                    &mut s.config.redirect_uri,
                    "http://127.0.0.1/callback",
                    false,
                );

                ui.add_space(8.0);
                // Status line + action button.
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                let (status_text, status_color) = if s.access_token.is_empty() {
                    ("No token yet — click Get New Token", muted())
                } else if let Some(exp) = s.expires_at {
                    let secs_left = exp - now;
                    if secs_left <= 0 {
                        ("Access token expired — click Get New Token", C_RED)
                    } else if secs_left < 60 {
                        ("Access token expires in <1 min", C_ORANGE)
                    } else {
                        ("Access token valid", C_GREEN)
                    }
                } else {
                    ("Access token stored (no expiry info)", C_GREEN)
                };
                // Collect intents from the closures (can't call
                // `self.start_oauth_flow()` while `s` holds a mutable
                // borrow of `self.editing_auth`).
                let busy = self.oauth_flow_rx.is_some();
                let flow_status = self.oauth_flow_status.clone();
                let has_token = !s.access_token.is_empty();
                let mut start_flow = false;
                let mut clear_token = false;
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(status_text)
                            .color(status_color)
                            .size(12.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_text = if busy { "Waiting…" } else { "Get New Token" };
                        let btn = egui::Button::new(
                            egui::RichText::new(btn_text)
                                .color(egui::Color32::WHITE)
                                .strong(),
                        )
                        .fill(C_ACCENT)
                        .min_size(egui::vec2(140.0, 28.0));
                        let resp = ui.add_enabled(!busy, btn);
                        if resp.clicked() {
                            start_flow = true;
                        }
                        if has_token && ui.small_button("Clear token").clicked() {
                            clear_token = true;
                        }
                    });
                });
                if clear_token {
                    s.access_token.clear();
                    s.refresh_token.clear();
                    s.expires_at = None;
                    changed = true;
                }
                if let Some(msg) = flow_status {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(msg).color(muted()).size(11.0));
                }
                // `start_flow` is handled after the match ends (below)
                // so we don't hold `&mut self.editing_auth` across the
                // call to `self.start_oauth_flow()`.
                if start_flow {
                    self.oauth_start_requested = true;
                }

                if !s.access_token.is_empty() {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("Access token (stored in data.json)")
                            .size(10.5)
                            .color(muted()),
                    );
                    // Masked preview — show first + last 8 chars.
                    let preview = mask_token(&s.access_token);
                    ui.label(
                        egui::RichText::new(preview)
                            .font(egui::FontId::monospace(11.5))
                            .color(text()),
                    );
                }
            }
        }
        if changed {
            let auth = self.editing_auth.clone();
            self.update_current_request(|r| r.auth = auth);
        }
        if std::mem::take(&mut self.oauth_start_requested) {
            self.start_oauth_flow();
        }
    }

    /// Post-response extractors — rules that pull a value out of the
    /// response (JSON-path into body, header name, or status code) and
    /// write it into the currently-active environment as a variable.
    /// Next request can reference it with `{{name}}`.
    fn render_tests_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("EXTRACTORS")
                .size(10.5)
                .strong()
                .color(muted()),
        );
        ui.add_space(3.0);
        ui.label(
            egui::RichText::new("Pull values from the response into environment variables.")
                .size(11.5)
                .color(muted()),
        );
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(
                "Body path: dot + bracket syntax, e.g. `data.token` or `items[0].id`.",
            )
            .size(11.0)
            .color(muted()),
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
            ui.label(egui::RichText::new("VARIABLE").size(10.0).color(muted()));
            ui.add_space(var_w - 46.0);
            ui.label(egui::RichText::new("SOURCE").size(10.0).color(muted()));
            ui.add_space(src_w - 36.0);
            ui.label(egui::RichText::new("EXPRESSION").size(10.0).color(muted()));
        });
        ui.add_space(2.0);
        ui.separator();
        ui.add_space(4.0);

        let mut changed = false;
        let mut to_remove: Option<usize> = None;
        let row_count = self.editing_extractors.len();
        let id_salt = egui::Id::new("extractors_table");
        for (i, ex) in self.editing_extractors.iter_mut().enumerate() {
            let is_ghost = i == row_count - 1 && ex.variable.is_empty() && ex.expression.is_empty();
            ui.horizontal(|ui| {
                if is_ghost {
                    ui.add_space(cb_w);
                } else if ui.add(egui::Checkbox::new(&mut ex.enabled, "")).changed() {
                    changed = true;
                }
                ui.add_space(pad);

                let color = if ex.enabled { text() } else { muted() };
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
                            if ui.selectable_label(ex.source == s, s.label()).clicked()
                                && ex.source != s
                            {
                                ex.source = s;
                                changed = true;
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

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);
        self.render_assertions_section(ui);
    }

    /// Pass/fail rules evaluated against the response after each
    /// send. The outcome is shown as a colored dot in the leftmost
    /// column of each row (green = pass, red = fail, yellow = error,
    /// grey = not yet run).
    fn render_assertions_section(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("ASSERTIONS")
                .size(10.5)
                .strong()
                .color(muted()),
        );
        ui.add_space(3.0);
        ui.label(
            egui::RichText::new(
                "Check the response matches your expectations. Body uses the \
                 same dot/bracket path as Extractors.",
            )
            .size(11.5)
            .color(muted()),
        );
        ui.add_space(8.0);

        // Trailing ghost row.
        if self
            .editing_assertions
            .last()
            .map(|a| !a.expression.is_empty() || !a.expected.is_empty())
            .unwrap_or(true)
        {
            self.editing_assertions.push(ResponseAssertion {
                enabled: true,
                source: AssertionSource::Status,
                expression: String::new(),
                op: AssertionOp::Equals,
                expected: String::new(),
            });
        }
        // Keep results vector in lock-step length with assertions.
        if self.assertion_results.len() < self.editing_assertions.len() {
            self.assertion_results
                .resize(self.editing_assertions.len(), None);
        } else if self.assertion_results.len() > self.editing_assertions.len() {
            self.assertion_results
                .truncate(self.editing_assertions.len());
        }

        let avail = ui.available_width();
        let dot_w = 14.0;
        let cb_w = 22.0;
        let src_w = 90.0;
        let expr_w = 160.0;
        let op_w = 110.0;
        let del_w = 22.0;
        let pad = 6.0;
        let right_margin = 12.0;
        let usable = avail - right_margin;
        let exp_w = (usable - dot_w - cb_w - src_w - expr_w - op_w - del_w - pad * 6.0).max(120.0);
        let row_h = 24.0;

        ui.horizontal(|ui| {
            ui.add_space(dot_w + cb_w + pad * 2.0);
            ui.label(egui::RichText::new("SOURCE").size(10.0).color(muted()));
            ui.add_space(src_w - 36.0);
            ui.label(egui::RichText::new("EXPRESSION").size(10.0).color(muted()));
            ui.add_space(expr_w - 60.0);
            ui.label(egui::RichText::new("OP").size(10.0).color(muted()));
            ui.add_space(op_w - 6.0);
            ui.label(egui::RichText::new("EXPECTED").size(10.0).color(muted()));
        });
        ui.add_space(2.0);
        ui.separator();
        ui.add_space(4.0);

        let mut changed = false;
        let mut to_remove: Option<usize> = None;
        let row_count = self.editing_assertions.len();
        let id_salt = egui::Id::new("assertions_table");

        for (i, asr) in self.editing_assertions.iter_mut().enumerate() {
            let is_ghost =
                i == row_count - 1 && asr.expression.is_empty() && asr.expected.is_empty();
            let result = self.assertion_results.get(i).cloned().flatten();
            ui.horizontal(|ui| {
                // Result dot
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(dot_w, row_h), egui::Sense::hover());
                if !is_ghost && asr.enabled {
                    let color = match &result {
                        Some(AssertionResult::Pass) => egui::Color32::from_rgb(130, 200, 120),
                        Some(AssertionResult::Fail(_)) => C_RED,
                        Some(AssertionResult::Error(_)) => C_ORANGE,
                        None => border(),
                    };
                    ui.painter().circle_filled(rect.center(), 4.0, color);
                    if let Some(r) = &result {
                        let tip = match r {
                            AssertionResult::Pass => "Passed".to_string(),
                            AssertionResult::Fail(why) => format!("Failed — {}", why),
                            AssertionResult::Error(why) => format!("Error — {}", why),
                        };
                        ui.interact(rect, id_salt.with((i, "dot")), egui::Sense::hover())
                            .on_hover_text(tip);
                    }
                }

                ui.add_space(pad);
                if is_ghost {
                    ui.add_space(cb_w);
                } else if ui.add(egui::Checkbox::new(&mut asr.enabled, "")).changed() {
                    changed = true;
                }
                ui.add_space(pad);

                let color = if asr.enabled { text() } else { muted() };

                // Source picker
                egui::ComboBox::from_id_salt(id_salt.with((i, "src")))
                    .selected_text(asr.source.label())
                    .width(src_w)
                    .show_ui(ui, |ui| {
                        for s in [
                            AssertionSource::Status,
                            AssertionSource::Header,
                            AssertionSource::Body,
                        ] {
                            if ui.selectable_label(asr.source == s, s.label()).clicked()
                                && asr.source != s
                            {
                                asr.source = s;
                                changed = true;
                            }
                        }
                    });
                ui.add_space(pad);

                let expr_enabled = !matches!(asr.source, AssertionSource::Status);
                let expr_hint = match asr.source {
                    AssertionSource::Body => "data.token",
                    AssertionSource::Header => "X-Request-Id",
                    AssertionSource::Status => "—",
                };
                if ui
                    .add_sized(
                        [expr_w, row_h],
                        egui::TextEdit::singleline(&mut asr.expression)
                            .id(id_salt.with((i, "expr")))
                            .hint_text(if expr_enabled && is_ghost {
                                expr_hint
                            } else {
                                ""
                            })
                            .interactive(expr_enabled)
                            .text_color(color),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.add_space(pad);

                // Operator picker
                egui::ComboBox::from_id_salt(id_salt.with((i, "op")))
                    .selected_text(asr.op.label())
                    .width(op_w)
                    .show_ui(ui, |ui| {
                        for op in [
                            AssertionOp::Equals,
                            AssertionOp::NotEquals,
                            AssertionOp::Contains,
                            AssertionOp::Matches,
                            AssertionOp::Exists,
                            AssertionOp::GreaterThan,
                            AssertionOp::LessThan,
                        ] {
                            if ui.selectable_label(asr.op == op, op.label()).clicked()
                                && asr.op != op
                            {
                                asr.op = op;
                                changed = true;
                            }
                        }
                    });
                ui.add_space(pad);

                let expected_enabled = asr.op.takes_expected();
                if ui
                    .add_sized(
                        [exp_w, row_h],
                        egui::TextEdit::singleline(&mut asr.expected)
                            .id(id_salt.with((i, "expected")))
                            .hint_text(if expected_enabled && is_ghost {
                                "expected value"
                            } else {
                                ""
                            })
                            .interactive(expected_enabled)
                            .text_color(color),
                    )
                    .changed()
                {
                    changed = true;
                }

                ui.add_space(pad);
                if is_ghost {
                    ui.add_space(del_w);
                } else if close_x_button(ui, "Remove assertion").clicked() {
                    to_remove = Some(i);
                }
            });
            ui.add_space(2.0);
        }

        if let Some(i) = to_remove {
            self.editing_assertions.remove(i);
            self.assertion_results.remove(i);
            changed = true;
        }
        if changed {
            let asrt = self.editing_assertions.clone();
            self.update_current_request(|r| r.assertions = asrt);
        }
    }

    /// Collection/folder "homepage": big title, quick stats, editable
    /// description, and a list of contained requests. Persists
    /// description changes back into the folder tree on every edit.
    fn render_collection_overview(&mut self, ui: &mut egui::Ui) {
        let folder_id = match &self.viewing_folder_id {
            Some(id) => id.clone(),
            None => return,
        };
        // Snapshot what we need; don't hold a borrow across the
        // whole render closure (we also want to write the description
        // back on change).
        let Some(folder_snapshot) =
            crate::find_folder_by_id(&self.state.folders, &folder_id).cloned()
        else {
            // Folder was deleted while we were viewing it — just
            // leave overview mode.
            self.viewing_folder_id = None;
            return;
        };
        let (req_count, sub_count) = count_requests_and_folders(&folder_snapshot);

        egui::Frame::none()
            .inner_margin(egui::Margin {
                left: 32.0,
                right: 32.0,
                top: 26.0,
                bottom: 10.0,
            })
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(("overview_scroll", folder_snapshot.id.clone()))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Title
                        ui.label(
                            egui::RichText::new(&folder_snapshot.name)
                                .size(26.0)
                                .strong()
                                .color(text()),
                        );
                        ui.add_space(6.0);

                        // Stats line — "N requests · M subfolders"
                        let stats = format!(
                            "{} request{} · {} subfolder{}",
                            req_count,
                            if req_count == 1 { "" } else { "s" },
                            sub_count,
                            if sub_count == 1 { "" } else { "s" },
                        );
                        ui.label(egui::RichText::new(stats).size(12.5).color(muted()));
                        ui.add_space(16.0);

                        // Description — editable multiline field. Auto-
                        // saves on every change; no "Save" button.
                        ui.label(
                            egui::RichText::new("DESCRIPTION")
                                .size(10.5)
                                .strong()
                                .color(muted()),
                        );
                        ui.add_space(4.0);
                        let desc_resp = ui.add(
                            egui::TextEdit::multiline(&mut self.editing_folder_desc)
                                .hint_text(
                                    "Write a description for this collection. \
                                     Explain what it's for, any auth quirks, \
                                     env setup, etc.",
                                )
                                .desired_rows(4)
                                .desired_width(f32::INFINITY),
                        );
                        if desc_resp.changed() {
                            let new_desc = self.editing_folder_desc.clone();
                            if let Some(folder) =
                                crate::find_folder_by_id_mut(&mut self.state.folders, &folder_id)
                            {
                                folder.description = new_desc;
                            }
                            self.save_state();
                        }
                        ui.add_space(20.0);

                        // Request list — click to jump into that request.
                        if !folder_snapshot.requests.is_empty() {
                            ui.label(
                                egui::RichText::new("REQUESTS")
                                    .size(10.5)
                                    .strong()
                                    .color(muted()),
                            );
                            ui.add_space(6.0);
                            let path = folder_path_from_root(&self.state.folders, &folder_id)
                                .unwrap_or_default();
                            for req in &folder_snapshot.requests {
                                let mc = method_color(&req.method);
                                let (rect, resp) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 28.0),
                                    egui::Sense::click(),
                                );
                                if ui.is_rect_visible(rect) {
                                    if resp.hovered() {
                                        ui.painter().rect_filled(
                                            rect,
                                            egui::Rounding::same(5.0),
                                            elevated(),
                                        );
                                    }
                                    ui.painter().text(
                                        egui::pos2(rect.left() + 8.0, rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        format!("{}", req.method),
                                        egui::FontId::new(10.5, egui::FontFamily::Proportional),
                                        mc,
                                    );
                                    ui.painter().text(
                                        egui::pos2(rect.left() + 60.0, rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        &req.name,
                                        egui::FontId::new(12.5, egui::FontFamily::Proportional),
                                        text(),
                                    );
                                }
                                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                                if resp.clicked() {
                                    self.open_request(path.clone(), req.id.clone());
                                }
                            }
                        } else {
                            ui.label(
                                egui::RichText::new(
                                    "This collection has no requests yet. \
                                     Right-click it in the sidebar to add one.",
                                )
                                .size(12.0)
                                .color(muted())
                                .italics(),
                            );
                        }
                    });
            });
    }
}

/// Count requests and subfolders recursively.
fn count_requests_and_folders(folder: &Folder) -> (usize, usize) {
    let mut reqs = folder.requests.len();
    let mut subs = folder.subfolders.len();
    for sub in &folder.subfolders {
        let (r, s) = count_requests_and_folders(sub);
        reqs += r;
        subs += s;
    }
    (reqs, subs)
}

/// Find the path of IDs from the top-level collections array down to
/// the folder with the given id. Returns `None` if not found.
fn folder_path_from_root(folders: &[Folder], target: &str) -> Option<Vec<String>> {
    fn dfs(folders: &[Folder], target: &str, path: &mut Vec<String>) -> bool {
        for f in folders {
            path.push(f.id.clone());
            if f.id == target {
                return true;
            }
            if dfs(&f.subfolders, target, path) {
                return true;
            }
            path.pop();
        }
        false
    }
    let mut path = Vec::new();
    if dfs(folders, target, &mut path) {
        Some(path)
    } else {
        None
    }
}

/// Attempt to decode a token as a JWT. Returns the pretty-printed
/// header + payload as `(header_json, payload_json)` on success. We
/// don't verify the signature — this is a dev convenience for
/// eyeballing claims, not a security check.
fn try_decode_jwt(token: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = token.trim().split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let decode_segment = |s: &str| -> Option<String> {
        use base64::Engine;
        let padded = {
            let pad = (4 - s.len() % 4) % 4;
            let mut t = s.to_string();
            for _ in 0..pad {
                t.push('=');
            }
            t
        };
        let bytes = base64::engine::general_purpose::URL_SAFE
            .decode(padded.as_bytes())
            .ok()?;
        let raw = String::from_utf8(bytes).ok()?;
        match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => serde_json::to_string_pretty(&v).ok(),
            Err(_) => None,
        }
    };
    let header = decode_segment(parts[0])?;
    let payload = decode_segment(parts[1])?;
    Some((header, payload))
}
