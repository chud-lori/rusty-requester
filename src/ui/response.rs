//! Response panel — status/time/size info bar with hover tooltips,
//! body view modes (JSON / Tree / Raw) with syntax highlighting,
//! search & copy toolbar, headers grid, loading spinner, and the
//! empty state.

use crate::model::*;
use crate::snippet::build_json_layout_job_with_search;
use crate::theme::*;
use crate::widgets::*;
use crate::ApiClient;
use eframe::egui;

impl ApiClient {
    /// Render `[status pill] · [time ms] · [total bytes]` inside a
    /// right-to-left layout, with hover tooltips (phase breakdown on
    /// time, request+response size breakdown on size). Rendered on
    /// the far-right of the Body/Headers tab row, Postman-style.
    fn render_response_status_chips(&self, ui: &mut egui::Ui) {
        if self.response_status.is_empty() {
            ui.label(
                egui::RichText::new("No response yet")
                    .color(C_MUTED)
                    .size(11.5)
                    .italics(),
            );
            return;
        }
        let bullet_sep = |ui: &mut egui::Ui| {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("•")
                    .color(C_MUTED.linear_multiply(0.7))
                    .size(12.0),
            );
            ui.add_space(4.0);
        };
        // In a right-to-left layout, items are laid out right-first,
        // so visually: status · time · size (size ends up leftmost).
        let total_resp_bytes = self.response_headers_bytes + self.response_body_bytes;
        if total_resp_bytes > 0 {
            let resp_h = self.response_headers_bytes;
            let resp_b = self.response_body_bytes;
            let req_h = self.request_headers_bytes;
            let req_b = self.request_body_bytes;
            ui.label(
                egui::RichText::new(format_bytes(total_resp_bytes))
                    .color(C_MUTED)
                    .size(12.0),
            )
            .on_hover_ui(move |ui| {
                render_size_breakdown(ui, resp_h, resp_b, req_h, req_b);
            });
            bullet_sep(ui);
        }
        if !self.response_time.is_empty() {
            let prep = self.response_prepare_ms;
            let wait = self.response_waiting_ms;
            let dl = self.response_download_ms;
            let total = self.response_total_ms;
            ui.label(
                egui::RichText::new(self.response_time.clone())
                    .color(C_MUTED)
                    .size(12.0),
            )
            .on_hover_ui(move |ui| {
                render_time_breakdown(ui, prep, wait, dl, total);
            });
            bullet_sep(ui);
        }
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

    pub(crate) fn render_response(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        // One unified toolbar row — Body / Headers tabs on the left,
        // then the body-view pills (JSON / Tree / Raw) inline with
        // them when Body is active, and the save / copy / search
        // icons pushed to the far right. Matches Postman's second
        // toolbar strip where format + action icons sit on the same
        // line as the section tabs.
        let mut copy_clicked = false;
        let mut toggle_search = false;
        let mut save_clicked = false;
        let is_json_body = !self.response_text.is_empty()
            && serde_json::from_str::<serde_json::Value>(&self.response_text).is_ok();
        let body_active = matches!(self.response_tab, ResponseTab::Body);

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

            if body_active {
                ui.add_space(18.0);
                let mut view = self.body_view;
                body_view_pill(ui, &mut view, BodyView::Json, "JSON");
                body_view_pill(ui, &mut view, BodyView::Tree, "Tree");
                body_view_pill(ui, &mut view, BodyView::Raw, "Raw");
                self.body_view = view;
                if matches!(self.body_view, BodyView::Tree) && is_json_body {
                    ui.add_space(8.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.body_tree_filter)
                            .hint_text("Filter keys / values")
                            .desired_width(160.0),
                    );
                }
            }

            // Right side: action icons (Body tab only) + status chips.
            // 6 px right edge padding so nothing sits flush against
            // the panel border.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                if body_active {
                    if icon_button(ui, "Save response to file", paint_save_icon).clicked() {
                        save_clicked = true;
                    }
                    ui.add_space(2.0);
                    if icon_button(ui, "Copy response body", paint_copy_icon).clicked() {
                        copy_clicked = true;
                    }
                    ui.add_space(2.0);
                    if icon_button(ui, "Search in body", paint_search_icon).clicked() {
                        toggle_search = true;
                    }
                    ui.add_space(12.0);
                }
                self.render_response_status_chips(ui);
            });
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
        if save_clicked {
            let ext = match self
                .response_headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                .map(|(_, v)| v.to_ascii_lowercase())
                .as_deref()
            {
                Some(v) if v.contains("json") => "json",
                Some(v) if v.contains("xml") => "xml",
                Some(v) if v.contains("html") => "html",
                Some(v) if v.contains("csv") => "csv",
                _ => "txt",
            };
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(format!("response.{}", ext))
                .save_file()
            {
                match std::fs::write(&path, &self.response_text) {
                    Ok(()) => self.show_toast(format!(
                        "Saved to {}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("file")
                    )),
                    Err(e) => self.show_toast(format!("Save failed: {}", e)),
                }
            }
        }
        if body_active && self.body_search_visible {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.body_search_query)
                        .hint_text("Find in body…")
                        .desired_width(ui.available_width() - 40.0),
                );
                if icon_button(ui, "Close search", |p, c, col| paint_x(p, c, 5.0, col, 1.5))
                    .clicked()
                {
                    self.body_search_visible = false;
                    self.body_search_query.clear();
                }
            });
        }
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
                let margin = 24.0;
                ui.set_width(full_w - margin);
                ui.set_min_height(full_h - margin);

                // Toolbar lives in the row above (inline with Body /
                // Headers tabs); this Frame just hosts the scrollable
                // content.
                egui::ScrollArea::vertical()
                    .id_salt("response_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.response_tab {
                        ResponseTab::Body => {
                            let parsed: Option<serde_json::Value> =
                                serde_json::from_str(&self.response_text).ok();
                            let is_json = parsed.is_some();
                            let effective_view =
                                if !is_json && !matches!(self.body_view, BodyView::Raw) {
                                    BodyView::Raw
                                } else {
                                    self.body_view
                                };

                            match effective_view {
                                BodyView::Json => {
                                    // `&mut &str` keeps the buffer
                                    // read-only while still letting the
                                    // user click to position the caret,
                                    // drag to select, and ⌘A / ⌘C as
                                    // expected — egui's TextEdit
                                    // handles those shortcuts itself
                                    // (the macOS Edit menu used to
                                    // intercept them; we removed that).
                                    let mut buf: &str = &self.response_text;
                                    let search = self.body_search_query.clone();
                                    let mut layouter =
                                        move |ui: &egui::Ui, s: &str, wrap_width: f32| {
                                            let mut job =
                                                build_json_layout_job_with_search(s, &search);
                                            job.wrap.max_width = wrap_width;
                                            ui.fonts(|f| f.layout_job(job))
                                        };
                                    ui.add_sized(
                                        egui::vec2(
                                            ui.available_width(),
                                            ui.available_height().max(120.0),
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
                                            ui.available_height().max(120.0),
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
                                    egui::RichText::new("No response headers yet.").color(C_MUTED),
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
        let _ = remaining_height;
    }
}
