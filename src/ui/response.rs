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
    pub(crate) fn render_response(&mut self, ui: &mut egui::Ui) {
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

}
