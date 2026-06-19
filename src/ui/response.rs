//! Response panel — status/time/size info bar with hover tooltips,
//! body view modes (JSON / Tree / Raw) with syntax highlighting,
//! search & copy toolbar, headers grid, loading spinner, and the
//! empty state.

use crate::model::*;
use crate::snippet::build_json_layout_job_content_only_with_search_active;
use crate::theme::*;
use crate::widgets::*;
use crate::ApiClient;
use eframe::egui;
use std::collections::HashMap;

impl ApiClient {
    /// Stable response header metadata. This renders inside the single
    /// response surface so status/time/size do not become a separate
    /// stacked card above the body.
    fn render_response_meta_row(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Response")
                    .size(11.5)
                    .color(muted())
                    .strong(),
            );
            ui.add_space(10.0);

            if self.response_status.is_empty() {
                ui.label(
                    egui::RichText::new("No response yet")
                        .size(12.5)
                        .color(muted())
                        .italics(),
                );
                return;
            }

            if self.is_loading {
                ui.add(egui::Spinner::new().size(14.0).color(accent()));
                ui.label(
                    egui::RichText::new("Sending request...")
                        .size(12.0)
                        .color(muted()),
                );
                return;
            }

            let sc = status_color(&self.response_status);
            egui::Frame::none()
                .fill(with_alpha(sc, if is_light() { 22 } else { 34 }))
                .rounding(egui::Rounding::same(5.0))
                .inner_margin(egui::Margin::symmetric(7.0, 2.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(&self.response_status)
                            .color(sc)
                            .strong()
                            .size(12.0),
                    );
                });

            if !self.response_time.is_empty() {
                let prep = self.response_prepare_ms;
                let wait = self.response_waiting_ms;
                let dl = self.response_download_ms;
                let total = self.response_total_ms;
                render_response_metric(ui, &self.response_time).on_hover_ui(move |ui| {
                    render_time_breakdown(ui, prep, wait, dl, total);
                });
            }

            let total_resp_bytes = self.response_headers_bytes + self.response_body_bytes;
            if total_resp_bytes > 0 {
                let resp_h = self.response_headers_bytes;
                let resp_b = self.response_body_bytes;
                let req_h = self.request_headers_bytes;
                let req_b = self.request_body_bytes;
                render_response_metric(ui, &format_bytes(total_resp_bytes)).on_hover_ui(
                    move |ui| {
                        render_size_breakdown(ui, resp_h, resp_b, req_h, req_b);
                    },
                );
            }
        });
    }

    /// Structured SSE event log — one expandable row per event. Newest
    /// event auto-scrolled into view while the stream is live so users
    /// don't have to scroll manually to watch incoming data.
    fn render_events_view(&mut self, ui: &mut egui::Ui) {
        if self.streaming_events.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new(egui_phosphor::regular::BROADCAST)
                        .size(48.0)
                        .color(muted().linear_multiply(0.6)),
                );
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(if self.is_loading {
                        "Waiting for events…"
                    } else {
                        "No events received."
                    })
                    .size(13.0)
                    .color(muted()),
                );
            });
            return;
        }

        let total = self.streaming_events.len();
        let live = self.is_loading;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(live)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for (idx, ev) in self.streaming_events.iter().enumerate() {
                    render_event_row(ui, idx, ev, total);
                }
                if live {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let time = ui.ctx().input(|i| i.time);
                        let pulse = ((time * 2.0).sin() * 0.5 + 0.5) as f32;
                        let dot = accent().linear_multiply(0.4 + 0.6 * pulse);
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 4.0, dot);
                        ui.label(egui::RichText::new("Listening…").size(11.5).color(muted()));
                    });
                }
            });
    }

    /// Response diff view — unified `+/-` line-diff between the
    /// previous response body and the current one. Added lines are
    /// green with a `+` gutter, removed lines red with `-`, same
    /// lines muted. Header summary shows `+A −B`.
    fn render_diff_view(&mut self, ui: &mut egui::Ui) {
        let Some(before) = self.previous_response_text.as_deref() else {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new(
                        "Send the request again to see a diff against this response.",
                    )
                    .size(13.0)
                    .color(muted()),
                );
            });
            return;
        };
        let after = self.response_text.as_str();
        let d = crate::diff::diff_lines(before, after);
        let (added, removed) = crate::diff::summarize(&d);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("+{}", added))
                    .color(C_GREEN)
                    .strong()
                    .monospace()
                    .size(12.0),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("−{}", removed))
                    .color(C_RED)
                    .strong()
                    .monospace()
                    .size(12.0),
            );
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("vs previous response")
                    .color(muted())
                    .size(11.0),
            );
        });
        ui.add_space(4.0);

        if added == 0 && removed == 0 {
            ui.label(
                egui::RichText::new("No differences — the response bodies are identical.")
                    .color(muted())
                    .size(12.0),
            );
            return;
        }

        let font = egui::FontId::monospace(12.0);
        egui::ScrollArea::vertical()
            .id_salt("diff_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for line in &d {
                    let (prefix, fg, bg) = match line.op {
                        crate::diff::Op::Same => (" ", text(), egui::Color32::TRANSPARENT),
                        crate::diff::Op::Added => ("+", C_GREEN, C_GREEN.linear_multiply(0.12)),
                        crate::diff::Op::Removed => ("−", C_RED, C_RED.linear_multiply(0.12)),
                    };
                    let row_h = 16.0;
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), row_h),
                        egui::Sense::hover(),
                    );
                    if ui.is_rect_visible(rect) {
                        if bg != egui::Color32::TRANSPARENT {
                            ui.painter().rect_filled(rect, egui::Rounding::ZERO, bg);
                        }
                        ui.painter().text(
                            egui::pos2(rect.left() + 4.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            prefix,
                            font.clone(),
                            fg,
                        );
                        ui.painter().text(
                            egui::pos2(rect.left() + 18.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &line.text,
                            font.clone(),
                            fg,
                        );
                    }
                }
            });
    }

    /// Error / cancel state — replaces the code editor with a
    /// centered illustration, status headline, error detail pill,
    /// and a helper hint line. Modeled after Postman's "Could not
    /// send request" screen. `cancelled` toggles the wording +
    /// color (amber for user cancel, red for network/DNS/TLS
    /// failure).
    fn render_failed_state(&mut self, ui: &mut egui::Ui, cancelled: bool) {
        let full_w = ui.available_width();
        let full_h = ui.available_height().max(160.0);
        egui::Frame::none()
            .fill(panel_dark())
            .inner_margin(16.0)
            .rounding(10.0)
            .stroke(egui::Stroke::new(1.0, border()))
            .show(ui, |ui| {
                let margin = 32.0;
                ui.set_width(full_w - margin);
                ui.set_min_height(full_h - margin);
                let inner_h = ui.available_height();

                // Headline wording flips between "Cancelled" and the
                // generic "Could not send request" (Postman parity).
                let headline = if cancelled {
                    "Request cancelled"
                } else {
                    "Could not send request"
                };
                // Error tint: amber for user-initiated cancel, red
                // for network failure. Amber reads as "you did this
                // on purpose" so it doesn't alarm.
                let tint = if cancelled { C_ORANGE } else { C_RED };

                ui.vertical_centered(|ui| {
                    ui.add_space((inner_h * 0.18).max(24.0));
                    // Large Phosphor icon as the error-state illustration.
                    ui.label(
                        egui::RichText::new(if cancelled {
                            egui_phosphor::regular::PROHIBIT
                        } else {
                            egui_phosphor::regular::WIFI_SLASH
                        })
                        .size(64.0)
                        .color(tint.linear_multiply(0.6)),
                    );
                    ui.add_space(14.0);

                    ui.label(egui::RichText::new(headline).size(14.5).color(muted()));
                    ui.add_space(12.0);

                    // Error detail pill: red-tinted bar with the
                    // leading line of `response_text` (which holds
                    // reqwest's flattened error chain for failures
                    // and our own message for cancels).
                    let detail = first_line(&self.response_text);
                    let prefix = if cancelled { "Cancelled:" } else { "Error:" };
                    render_error_pill(ui, tint, prefix, &detail);

                    ui.add_space(14.0);
                    // Quiet hint line. Different suggestions per
                    // cause — a cancel doesn't need "check the URL",
                    // but a failure does.
                    let hint = if cancelled {
                        "Press Send to try again."
                    } else {
                        "Double-check the URL, host reachability, TLS, and your proxy."
                    };
                    ui.label(egui::RichText::new(hint).size(11.5).color(muted()));
                });
            });
    }

    pub(crate) fn render_response(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        // Stable response header first, then one content surface. The
        // Body-only JSON/Tree/Raw selector stays secondary to the
        // primary Body/Headers tabs.
        let mut copy_clicked = false;
        let mut toggle_search = false;
        let mut save_clicked = false;
        let mut inline_room = true;
        let is_json_body = !self.response_text.is_empty()
            && serde_json::from_str::<serde_json::Value>(&self.response_text).is_ok();
        let body_active = matches!(self.response_tab, ResponseTab::Body);
        let is_html_body =
            crate::html_preview::is_html(&self.response_headers, &self.response_text);
        let is_sse_body = crate::sse::is_event_stream(&self.response_headers)
            || !self.streaming_events.is_empty();
        let has_diff_snapshot = self.previous_response_text.is_some();

        self.render_response_meta_row(ui);
        ui.add_space(6.0);

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

            // Right side: action icons (Body tab only).
            // When the panel is narrow (snippet panel open + small
            // window) the right-to-left block would overlap the tab
            // labels, since `ui.horizontal` doesn't reserve space
            // between left and right children. We measure remaining
            // width and defer rendering to a second row when there
            // isn't enough room — drawing nothing inside the inner
            // block keeps it from claiming row height.
            //
            // Threshold ~150 px = 3 icon buttons + paddings. Status
            // chips have moved to the meta band above.
            inline_room = ui.available_width() >= 150.0;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !inline_room {
                    return;
                }
                if body_active {
                    if icon_btn(
                        ui,
                        egui_phosphor::regular::DOWNLOAD_SIMPLE,
                        "Save raw response body to file",
                    )
                    .clicked()
                    {
                        save_clicked = true;
                    }
                    ui.add_space(2.0);
                    if icon_btn(ui, egui_phosphor::regular::COPY, "Copy raw response body")
                        .clicked()
                    {
                        copy_clicked = true;
                    }
                    // Inline "Copied!" flash, visible for ~1.5s after
                    // the click. The bottom-right toast is easy to miss
                    // when focus is on the response pane; a label right
                    // next to the button isn't.
                    if let Some(t0) = self.response_copied_at {
                        let now = ui.ctx().input(|i| i.time);
                        let age = now - t0;
                        if age < 1.5 {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} Copied",
                                    egui_phosphor::regular::CHECK
                                ))
                                .color(C_GREEN)
                                .size(12.0),
                            );
                            ui.ctx().request_repaint();
                        } else {
                            self.response_copied_at = None;
                        }
                    }
                    ui.add_space(2.0);
                    if icon_btn(
                        ui,
                        egui_phosphor::regular::MAGNIFYING_GLASS,
                        "Search in body",
                    )
                    .clicked()
                    {
                        toggle_search = true;
                    }
                }
            });
        });
        // Overflow row — only when the inline block didn't fit.
        // Renders the same right-side content pushed to the panel's
        // right edge so the visual rhythm is preserved.
        if !inline_room {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if body_active {
                        if icon_btn(
                            ui,
                            egui_phosphor::regular::DOWNLOAD_SIMPLE,
                            "Save raw response body to file",
                        )
                        .clicked()
                        {
                            save_clicked = true;
                        }
                        ui.add_space(2.0);
                        if icon_btn(ui, egui_phosphor::regular::COPY, "Copy raw response body")
                            .clicked()
                        {
                            copy_clicked = true;
                        }
                        if let Some(t0) = self.response_copied_at {
                            let now = ui.ctx().input(|i| i.time);
                            let age = now - t0;
                            if age < 1.5 {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} Copied",
                                        egui_phosphor::regular::CHECK
                                    ))
                                    .color(C_GREEN)
                                    .size(12.0),
                                );
                                ui.ctx().request_repaint();
                            } else {
                                self.response_copied_at = None;
                            }
                        }
                        ui.add_space(2.0);
                        if icon_btn(
                            ui,
                            egui_phosphor::regular::MAGNIFYING_GLASS,
                            "Search in body",
                        )
                        .clicked()
                        {
                            toggle_search = true;
                        }
                    }
                });
            });
        }

        let body_active = matches!(self.response_tab, ResponseTab::Body);
        if body_active {
            ui.add_space(3.0);
            let requested_view = self.body_view;
            let effective_view = effective_body_view(
                requested_view,
                is_json_body,
                is_html_body,
                is_sse_body,
                has_diff_snapshot,
            );
            let mut view = requested_view;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                render_body_view_selector(
                    ui,
                    &mut view,
                    effective_view,
                    is_sse_body,
                    is_html_body,
                    has_diff_snapshot,
                    self.streaming_events.len(),
                );
                if view != requested_view {
                    self.body_view = view;
                }

                let selected_effective = effective_body_view(
                    self.body_view,
                    is_json_body,
                    is_html_body,
                    is_sse_body,
                    has_diff_snapshot,
                );
                if matches!(selected_effective, BodyView::Tree) && is_json_body {
                    ui.add_space(8.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.body_tree_filter)
                            .hint_text(hint("Filter keys / values"))
                            .desired_width(170.0),
                    );
                }
            });

            if self.body_search_visible {
                ui.add_space(4.0);
                let horizontal_margin = 8.0;
                let find_w = response_find_row_width(ui.available_width());
                let content_w = response_find_inner_width(find_w, horizontal_margin);
                ui.allocate_ui_with_layout(
                    egui::vec2(find_w, 32.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        egui::Frame::none()
                            .fill(if is_light() {
                                egui::Color32::from_rgb(245, 247, 250)
                            } else {
                                egui::Color32::from_rgb(22, 25, 31)
                            })
                            .stroke(egui::Stroke::new(1.0, with_alpha(border(), 185)))
                            .rounding(egui::Rounding::same(9.0))
                            .inner_margin(egui::Margin::symmetric(horizontal_margin, 4.0))
                            .show(ui, |ui| {
                                ui.set_min_width(content_w);
                                ui.set_max_width(content_w);
                                ui.spacing_mut().item_spacing.x = 6.0;
                                ui.label(
                                    egui::RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS)
                                        .size(13.0)
                                        .color(muted()),
                                );
                                let close_w = 22.0;
                                if self.body_search_query != self.body_search_last_query {
                                    self.body_search_last_query = self.body_search_query.clone();
                                    self.body_search_active_match = 0;
                                }
                                let match_count = count_case_insensitive_matches(
                                    &self.response_text,
                                    &self.body_search_query,
                                );
                                if self.body_search_active_match >= match_count {
                                    self.body_search_active_match = 0;
                                }
                                let count_text = response_find_count_text(
                                    &self.body_search_query,
                                    match_count,
                                    self.body_search_active_match,
                                );
                                let count_w = response_find_count_width(&count_text);
                                let gap_w = if count_text.is_empty() {
                                    ui.spacing().item_spacing.x
                                } else {
                                    ui.spacing().item_spacing.x * 2.0
                                };
                                let icon_w = 14.0;
                                let spacing_w = ui.spacing().item_spacing.x;
                                let input_w = response_find_input_width(
                                    content_w - icon_w - spacing_w,
                                    count_w,
                                    close_w,
                                    gap_w,
                                );
                                let search_resp = ui.add_sized(
                                    [input_w, 22.0],
                                    egui::TextEdit::singleline(&mut self.body_search_query)
                                        .hint_text(hint("Find"))
                                        .frame(false),
                                );
                                if self.body_search_focus_pending {
                                    self.body_search_focus_pending = false;
                                    search_resp.request_focus();
                                }
                                if search_resp.has_focus() {
                                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                        self.body_search_visible = false;
                                        self.body_search_query.clear();
                                        self.body_search_last_query.clear();
                                        self.body_search_active_match = 0;
                                    }
                                    if ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        && match_count > 0
                                    {
                                        let backward = ui.input(|i| i.modifiers.shift);
                                        self.body_search_active_match = next_search_match_index(
                                            self.body_search_active_match,
                                            match_count,
                                            backward,
                                        );
                                    }
                                }

                                if !count_text.is_empty() {
                                    ui.add_sized(
                                        [count_w, 20.0],
                                        egui::Label::new(
                                            egui::RichText::new(count_text)
                                                .size(11.0)
                                                .color(muted()),
                                        ),
                                    );
                                }
                                if close_x_button(ui, "Close search").clicked() {
                                    self.body_search_visible = false;
                                    self.body_search_query.clear();
                                    self.body_search_last_query.clear();
                                    self.body_search_active_match = 0;
                                }
                            });
                    },
                );
            }
        }

        if toggle_search {
            if self.body_search_visible {
                self.body_search_visible = false;
                self.body_search_query.clear();
                self.body_search_last_query.clear();
                self.body_search_active_match = 0;
            } else {
                self.body_search_visible = true;
                self.body_search_focus_pending = true;
            }
        }
        if copy_clicked {
            ui.ctx()
                .output_mut(|o| o.copied_text = self.response_text.clone());
            self.response_copied_at = Some(ui.ctx().input(|i| i.time));
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
                .fill(panel_dark())
                .inner_margin(16.0)
                .rounding(10.0)
                .stroke(egui::Stroke::new(1.0, border()))
                .show(ui, |ui| {
                    let margin = 32.0;
                    ui.set_width(full_w - margin);
                    ui.set_min_height(full_h - margin);
                    let inner_h = ui.available_height();
                    ui.vertical_centered(|ui| {
                        ui.add_space((inner_h * 0.30).max(24.0));
                        ui.add(egui::Spinner::new().size(26.0).color(accent()));
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("Sending request…")
                                .size(13.0)
                                .color(text()),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Press Esc to stay — response will appear here")
                                .size(11.5)
                                .color(muted()),
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
                .fill(panel_dark())
                .inner_margin(16.0)
                .rounding(10.0)
                .stroke(egui::Stroke::new(1.0, border()))
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
                                .color(text()),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Send a request to see the response here.")
                                .size(12.5)
                                .color(muted()),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Shortcut: Cmd / Ctrl + Enter")
                                .size(12.0)
                                .color(muted()),
                        );
                    });
                });
            return;
        }

        // Dedicated panel for request failures (network error, DNS,
        // TLS, timeout) and user cancellation. Replaces the code
        // editor — showing a red pill with the error on a centered
        // illustration is far more scannable than "Error: builder
        // error… caused by…" rendered as plain text.
        let is_failed = self.response_status == "Failed";
        let is_cancelled = self.response_status == "Cancelled";
        if is_failed || is_cancelled {
            self.render_failed_state(ui, is_cancelled);
            return;
        }

        let parsed: Option<serde_json::Value> = serde_json::from_str(&self.response_text).ok();
        let is_json = parsed.is_some();
        let effective_view = effective_body_view(
            self.body_view,
            is_json,
            crate::html_preview::is_html(&self.response_headers, &self.response_text),
            crate::sse::is_event_stream(&self.response_headers)
                || !self.streaming_events.is_empty(),
            self.previous_response_text.is_some(),
        );

        if body_active {
            render_response_summary(
                ui,
                &self.response_status,
                &self.response_headers,
                &self.response_text,
                is_json,
                self.body_view,
            );
        }

        let full_w = ui.available_width();
        let full_h = ui.available_height().max(120.0);
        egui::Frame::none()
            .fill(panel_dark())
            .inner_margin(12.0)
            .rounding(10.0)
            .stroke(egui::Stroke::new(1.0, border()))
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
                            match effective_view {
                                BodyView::Json => {
                                    // Postman-style folding viewer.
                                    // Gutter has two sub-columns: a
                                    // chevron (▼ open / ▶ closed) for
                                    // every line that opens a multi-
                                    // line `{` or `[` block, and the
                                    // 1-based line number. Clicking the
                                    // chevron toggles `folded_response_lines`.
                                    // The displayed text is rebuilt
                                    // every frame from the original
                                    // body + the fold set: hidden lines
                                    // are skipped, and the opener line
                                    // gets `…}` or `…]` appended so
                                    // collapsed blocks read as a single
                                    // logical entry. TextEdit still
                                    // owns selection / copy / search
                                    // over the visible text.
                                    let gutter_w = 60.0;
                                    let chevron_w = 16.0;
                                    let row_h = 17.0;
                                    let pairs = compute_json_fold_pairs(&self.response_text);
                                    let display = build_folded_display(
                                        &self.response_text,
                                        &pairs,
                                        &self.folded_response_lines,
                                    );
                                    let mut toggle_fold: Option<u32> = None;
                                    ui.horizontal_top(|ui| {
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 0.0;
                                            for d in &display {
                                                ui.horizontal(|ui| {
                                                    ui.spacing_mut().item_spacing.x = 0.0;
                                                    let chev_color =
                                                        egui::Color32::from_rgb(120, 125, 135);
                                                    match d.fold {
                                                        FoldState::Open | FoldState::Closed => {
                                                            let glyph = if matches!(
                                                                d.fold,
                                                                FoldState::Open
                                                            ) {
                                                                "▼"
                                                            } else {
                                                                "▶"
                                                            };
                                                            let resp = ui.add_sized(
                                                                [chevron_w, row_h],
                                                                egui::Button::new(
                                                                    egui::RichText::new(glyph)
                                                                        .color(chev_color)
                                                                        .font(
                                                                            egui::FontId::monospace(
                                                                                10.0,
                                                                            ),
                                                                        ),
                                                                )
                                                                .frame(false),
                                                            );
                                                            if resp.clicked() {
                                                                toggle_fold = Some(d.line_no);
                                                            }
                                                        }
                                                        FoldState::None => {
                                                            ui.add_sized(
                                                                [chevron_w, row_h],
                                                                egui::Label::new(""),
                                                            );
                                                        }
                                                    }
                                                    ui.add_sized(
                                                        [gutter_w - chevron_w, row_h],
                                                        egui::Label::new(
                                                            egui::RichText::new(format!(
                                                                "{:>3}",
                                                                d.line_no
                                                            ))
                                                            .color(egui::Color32::from_rgb(
                                                                100, 105, 115,
                                                            ))
                                                            .font(egui::FontId::monospace(12.5)),
                                                        ),
                                                    );
                                                });
                                            }
                                        });
                                        ui.add_space(6.0);
                                        let displayed_text: String = display
                                            .iter()
                                            .map(|d| d.content.as_str())
                                            .collect::<Vec<_>>()
                                            .join("\n");
                                        let mut buf: &str = &displayed_text;
                                        let search = self.body_search_query.clone();
                                        let active_match = self.body_search_active_match;
                                        let mut layouter =
                                            move |ui: &egui::Ui, s: &str, wrap_width: f32| {
                                                let mut job =
                                                    build_json_layout_job_content_only_with_search_active(
                                                        s,
                                                        &search,
                                                        Some(active_match),
                                                    );
                                                job.wrap.max_width = wrap_width;
                                                ui.fonts(|f| f.layout_job(job))
                                            };
                                        ui.add(
                                            egui::TextEdit::multiline(&mut buf)
                                                .frame(false)
                                                .desired_width(f32::INFINITY)
                                                .font(egui::TextStyle::Monospace)
                                                .layouter(&mut layouter),
                                        );
                                    });
                                    if let Some(line_no) = toggle_fold {
                                        if !self.folded_response_lines.remove(&line_no) {
                                            self.folded_response_lines.insert(line_no);
                                        }
                                    }
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
                                BodyView::Preview => {
                                    // Strip scripts/styles + decode
                                    // entities on every frame. Cheap
                                    // for typical HTML error pages
                                    // (< 100 KB); for larger bodies
                                    // we'd want to cache, but those
                                    // aren't the common case. Font
                                    // intentionally proportional (not
                                    // monospace) — this is the
                                    // reader-mode view.
                                    let stripped =
                                        crate::html_preview::strip_to_text(&self.response_text);
                                    let mut buf: &str = &stripped;
                                    ui.add_sized(
                                        egui::vec2(
                                            ui.available_width(),
                                            ui.available_height().max(120.0),
                                        ),
                                        egui::TextEdit::multiline(&mut buf)
                                            .frame(false)
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                                BodyView::Events => {
                                    self.render_events_view(ui);
                                }
                                BodyView::Diff => {
                                    self.render_diff_view(ui);
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
                                    egui::RichText::new("No response headers yet.").color(muted()),
                                );
                            } else {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Response headers")
                                            .size(12.5)
                                            .strong()
                                            .color(text()),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} total",
                                            self.response_headers.len()
                                        ))
                                        .size(11.0)
                                        .color(muted()),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if icon_btn(
                                                ui,
                                                egui_phosphor::regular::COPY,
                                                "Copy all response headers",
                                            )
                                            .clicked()
                                            {
                                                let headers_text = self
                                                    .response_headers
                                                    .iter()
                                                    .map(|(k, v)| format!("{}: {}", k, v))
                                                    .collect::<Vec<_>>()
                                                    .join("\n");
                                                ui.ctx().copy_text(headers_text);
                                            }
                                        },
                                    );
                                });
                                ui.add_space(6.0);

                                // Softer header pane: keys use `muted()`
                                // instead of saturated accent() (the rust
                                // red on every row felt harsh), and we
                                // drop the built-in `.striped(true)`
                                // zebra in favor of a single 1-px
                                // `border()` divider between rows —
                                // closer to Postman's calmer table look.
                                egui::Grid::new("resp_headers_grid")
                                    .num_columns(2)
                                    .spacing([20.0, 6.0])
                                    .show(ui, |ui| {
                                        for (k, v) in &self.response_headers {
                                            ui.label(
                                                egui::RichText::new(k).color(muted()).strong(),
                                            );
                                            ui.label(
                                                egui::RichText::new(v)
                                                    .font(egui::FontId::monospace(12.0))
                                                    .color(text()),
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

fn render_response_metric(ui: &mut egui::Ui, value: &str) -> egui::Response {
    ui.add_space(2.0);
    ui.label(egui::RichText::new("·").size(12.0).color(muted()));
    ui.add_space(2.0);
    ui.add(
        egui::Label::new(
            egui::RichText::new(value)
                .size(12.0)
                .color(muted())
                .strong(),
        )
        .sense(egui::Sense::hover()),
    )
}

fn response_find_width(available_width: f32) -> f32 {
    available_width.clamp(0.0, 360.0)
}

fn response_find_row_width(available_width: f32) -> f32 {
    response_find_width((available_width - 12.0).max(0.0))
}

fn response_find_inner_width(frame_width: f32, horizontal_margin: f32) -> f32 {
    (frame_width - horizontal_margin * 2.0).max(0.0)
}

fn response_find_count_text(query: &str, match_count: usize, active_match: usize) -> String {
    if query.is_empty() {
        String::new()
    } else if match_count == 0 {
        "0/0".to_string()
    } else {
        format!("{}/{}", active_match + 1, match_count)
    }
}

fn response_find_count_width(count_text: &str) -> f32 {
    if count_text.is_empty() {
        0.0
    } else {
        (count_text.chars().count() as f32 * 7.0 + 14.0).clamp(42.0, 96.0)
    }
}

fn response_find_input_width(
    available_width: f32,
    count_width: f32,
    close_width: f32,
    gap_width: f32,
) -> f32 {
    (available_width - count_width - close_width - gap_width).max(0.0)
}

fn count_case_insensitive_matches(text: &str, query: &str) -> usize {
    if query.is_empty() {
        return 0;
    }
    let text = text.to_lowercase();
    let query = query.to_lowercase();
    text.match_indices(&query).count()
}

fn next_search_match_index(current: usize, match_count: usize, backward: bool) -> usize {
    if match_count == 0 {
        0
    } else if backward {
        current.checked_sub(1).unwrap_or(match_count - 1)
    } else {
        (current + 1) % match_count
    }
}

fn effective_body_view(
    selected: BodyView,
    is_json: bool,
    is_html: bool,
    is_sse: bool,
    has_diff_snapshot: bool,
) -> BodyView {
    let mut effective = selected;
    if !is_json && matches!(effective, BodyView::Json | BodyView::Tree) {
        effective = BodyView::Raw;
    }
    if matches!(effective, BodyView::Preview) && !is_html {
        effective = BodyView::Raw;
    }
    if matches!(effective, BodyView::Events) && !is_sse {
        effective = BodyView::Raw;
    }
    if is_sse && matches!(effective, BodyView::Json | BodyView::Tree) {
        effective = BodyView::Raw;
    }
    if matches!(effective, BodyView::Diff) && !has_diff_snapshot {
        effective = BodyView::Raw;
    }
    effective
}

fn render_body_view_selector(
    ui: &mut egui::Ui,
    current: &mut BodyView,
    effective: BodyView,
    _is_sse: bool,
    _is_html: bool,
    _has_diff_snapshot: bool,
    _event_count: usize,
) {
    // Specialized renderers still exist for restored state and fallback
    // compatibility, but M1 exposes only the core Body views here.
    let _retained_modes = [BodyView::Preview, BodyView::Events, BodyView::Diff];
    ui.horizontal(|ui| {
        for view in [BodyView::Json, BodyView::Tree, BodyView::Raw] {
            let selected = effective == view;
            if ui
                .selectable_label(selected, body_view_label(view, 0))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                *current = view;
            }
        }
    });
}

fn body_view_label(view: BodyView, event_count: usize) -> String {
    match view {
        BodyView::Json => "JSON".to_string(),
        BodyView::Tree => "Tree".to_string(),
        BodyView::Preview => "Preview".to_string(),
        BodyView::Events => format!("Events ({})", event_count),
        BodyView::Diff => "Diff".to_string(),
        BodyView::Raw => "Raw".to_string(),
    }
}

/// Per-displayed-row state for the JSON folding viewer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FoldState {
    /// Line is not the opener of any multi-line block.
    None,
    /// Line opens a multi-line block and is currently expanded.
    Open,
    /// Line opens a multi-line block and is currently collapsed.
    Closed,
}

struct DisplayLine {
    /// Original 1-based line number in `response_text`. Used for
    /// gutter rendering and as the stable key in `folded_response_lines`.
    line_no: u32,
    /// What the layouter should render for this row. For folded
    /// openers, this is `<opener line> …}` (or `…]`); for hidden
    /// inner lines we don't push a `DisplayLine` at all.
    content: String,
    fold: FoldState,
}

/// Walk the JSON body once and pair every `{`/`[` opener with its
/// matching `}`/`]` closer. Returns `opener_line → closer_line` (both
/// 1-based) only for pairs that span more than one line — single-line
/// `{}` or `[]` aren't worth a fold chevron.
///
/// String-aware (skips braces inside `"…"`); does not validate JSON,
/// just trusts that the body is well-formed enough for fold ranges to
/// be meaningful. Pretty-printed serde output always is.
fn compute_json_fold_pairs(text: &str) -> HashMap<u32, u32> {
    let mut pairs: HashMap<u32, u32> = HashMap::new();
    let mut stack: Vec<u32> = Vec::new();
    let mut line: u32 = 1;
    let mut in_string = false;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            match c {
                b'\\' => {
                    // Skip the escaped char so e.g. `\"` doesn't end
                    // the string. Newline mid-string isn't legal in
                    // JSON, but if the body has one anyway we still
                    // want to track it.
                    if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                        line += 1;
                    }
                    i += 2;
                    continue;
                }
                b'"' => in_string = false,
                b'\n' => line += 1,
                _ => {}
            }
        } else {
            match c {
                b'\n' => line += 1,
                b'"' => in_string = true,
                b'{' | b'[' => stack.push(line),
                b'}' | b']' => {
                    if let Some(open_line) = stack.pop() {
                        if line > open_line {
                            pairs.insert(open_line, line);
                        }
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    pairs
}

/// Apply the user's fold set to the original text and emit the list
/// of rows the gutter + layouter should render. Folded openers get a
/// `…}` / `…]` placeholder appended so the row visually summarises
/// the hidden block (same idea as Postman / VSCode); the inner lines
/// are simply omitted.
fn build_folded_display(
    text: &str,
    pairs: &HashMap<u32, u32>,
    folded: &std::collections::HashSet<u32>,
) -> Vec<DisplayLine> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<DisplayLine> = Vec::with_capacity(lines.len());
    let mut skip_until: Option<u32> = None;
    for (idx, line) in lines.iter().enumerate() {
        let line_no = (idx + 1) as u32;
        if let Some(end) = skip_until {
            if line_no <= end {
                continue;
            }
            skip_until = None;
        }
        let is_opener = pairs.contains_key(&line_no);
        if is_opener && folded.contains(&line_no) {
            let closer = pairs[&line_no];
            let trimmed = line.trim_end();
            let suffix = if trimmed.ends_with('{') {
                " …}"
            } else if trimmed.ends_with('[') {
                " …]"
            } else {
                " …"
            };
            out.push(DisplayLine {
                line_no,
                content: format!("{}{}", line, suffix),
                fold: FoldState::Closed,
            });
            skip_until = Some(closer);
        } else {
            out.push(DisplayLine {
                line_no,
                content: line.to_string(),
                fold: if is_opener {
                    FoldState::Open
                } else {
                    FoldState::None
                },
            });
        }
    }
    out
}

/// Render one SSE event as a collapsible row: a chip header with
/// `#N event-type · time · ids` and the data payload beneath. JSON
/// payloads get pretty-printed with monospace formatting; other text
/// renders verbatim. Expanded by default for the latest event.
fn render_event_row(ui: &mut egui::Ui, idx: usize, ev: &crate::sse::SseEvent, total: usize) {
    let event_label = ev.event_type.as_deref().unwrap_or("message");
    let default_open = idx + 1 == total; // latest is expanded
    let id = egui::Id::new(("sse_event", idx, ev.timestamp_ms));

    egui::Frame::none()
        .fill(panel_dark())
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .stroke(egui::Stroke::new(1.0, border()))
        .show(ui, |ui| {
            egui::CollapsingHeader::new(header_richtext(idx + 1, event_label, ev))
                .id_salt(id)
                .default_open(default_open)
                .show(ui, |ui| {
                    ui.add_space(2.0);
                    if let Some(id_val) = &ev.id {
                        ui.label(
                            egui::RichText::new(format!("id: {}", id_val))
                                .color(muted())
                                .size(11.0)
                                .monospace(),
                        );
                    }
                    if let Some(retry) = ev.retry_ms {
                        ui.label(
                            egui::RichText::new(format!("retry: {} ms", retry))
                                .color(muted())
                                .size(11.0)
                                .monospace(),
                        );
                    }
                    let data_pretty = match serde_json::from_str::<serde_json::Value>(&ev.data) {
                        Ok(v) => {
                            serde_json::to_string_pretty(&v).unwrap_or_else(|_| ev.data.clone())
                        }
                        Err(_) => ev.data.clone(),
                    };
                    let mut data_ref: &str = &data_pretty;
                    let h = data_pretty.lines().count().clamp(1, 14) as f32 * 16.0 + 10.0;
                    ui.add_sized(
                        egui::vec2(ui.available_width(), h),
                        egui::TextEdit::multiline(&mut data_ref)
                            .frame(false)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let copy = ui
                                .small_button(
                                    egui::RichText::new("Copy data").size(11.0).color(muted()),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand);
                            if copy.clicked() {
                                ui.ctx().copy_text(ev.data.clone());
                            }
                        });
                    });
                });
        });
}

fn render_response_summary(
    ui: &mut egui::Ui,
    status: &str,
    headers: &[(String, String)],
    body: &str,
    is_json: bool,
    view: BodyView,
) {
    if body.is_empty() {
        return;
    }

    let Some(summary) = classify_response_summary(status, headers, body, is_json, view) else {
        return;
    };

    egui::Frame::none()
        .fill(summary.tint.linear_multiply(0.12))
        .stroke(egui::Stroke::new(1.0, summary.tint.linear_multiply(0.45)))
        .rounding(egui::Rounding::same(7.0))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.label(
                    egui::RichText::new(if summary.is_error {
                        egui_phosphor::regular::WARNING
                    } else {
                        egui_phosphor::regular::INFO
                    })
                    .size(15.0)
                    .color(summary.tint),
                );
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(summary.headline)
                            .size(12.5)
                            .strong()
                            .color(text()),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(summary.detail)
                            .size(11.5)
                            .color(muted()),
                    );
                });
            });
        });
    ui.add_space(8.0);
}

struct ResponseSummary {
    headline: String,
    detail: String,
    tint: egui::Color32,
    is_error: bool,
}

fn classify_response_summary(
    status: &str,
    headers: &[(String, String)],
    body: &str,
    is_json: bool,
    view: BodyView,
) -> Option<ResponseSummary> {
    if body.is_empty() {
        return None;
    }

    let status_code = status
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<u16>().ok());
    let is_error = status_code.map(|code| code >= 400).unwrap_or(false);
    let is_html = crate::html_preview::is_html(headers, body);
    let content_type = content_type(headers).unwrap_or("unknown");
    let is_structured_view = matches!(view, BodyView::Json | BodyView::Tree);

    if !is_error && !is_html && is_json {
        return None;
    }

    let lower_body = body.to_ascii_lowercase();
    let looks_like_cloudflare = lower_body.contains("cloudflare")
        || lower_body.contains("challenge-platform")
        || lower_body.contains("just a moment");
    let tint = if is_error {
        status_color(status)
    } else if is_html {
        C_ORANGE
    } else {
        muted()
    };
    let headline = if looks_like_cloudflare {
        "Cloudflare challenge returned instead of API data"
    } else if is_error && is_html {
        "Request returned an HTML error page"
    } else if is_error {
        "Request completed with an error status"
    } else if is_html {
        "HTML response detected"
    } else if is_structured_view {
        "Response is not valid JSON"
    } else {
        "Non-JSON response"
    };
    let detail = if looks_like_cloudflare {
        format!(
            "Status: {}. Prefer official API auth, service tokens, or allowlisting; browser session cookies are sensitive.",
            status
        )
    } else if is_html {
        format!(
            "Content-Type: {}. Raw keeps the original body.",
            content_type
        )
    } else if !is_json {
        format!(
            "Content-Type: {}. JSON and Tree views are unavailable; Raw shows the original body.",
            content_type
        )
    } else {
        "Inspect the response body and headers for details.".to_string()
    };

    Some(ResponseSummary {
        headline: headline.to_string(),
        detail,
        tint,
        is_error,
    })
}

fn content_type(headers: &[(String, String)]) -> Option<&str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.as_str())
}

/// Collapsing-header label for an SSE event row: `#12 event-type ·
/// HH:MM:SS.mmm`. The event type is accent-colored so streams with
/// mixed events (e.g. `message` vs `error`) are scannable at a glance.
fn header_richtext(n: usize, event_type: &str, ev: &crate::sse::SseEvent) -> egui::RichText {
    let ts = format_event_ts(ev.timestamp_ms);
    egui::RichText::new(format!("#{}  {}  ·  {}", n, event_type, ts))
        .size(12.5)
        .color(text())
}

fn format_event_ts(ms: u64) -> String {
    let secs = (ms / 1000) % 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let millis = ms % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, millis)
}

/// Pull the first non-empty line from a multi-line error chain. The
/// rest of the text still renders elsewhere if the user wants it
/// (e.g. in the Headers tab or via Raw view); the pill only needs
/// the human-readable summary line.
fn first_line(s: &str) -> String {
    s.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(s)
        .trim_start_matches("Error:")
        .trim_start_matches("error sending request for url")
        .trim()
        .to_string()
}

/// Error detail pill. Red/amber-tinted rounded bar with an icon,
/// a bold prefix (`Error:` / `Cancelled:`), then the detail.
///
/// Pre-allocates a fixed-width region sized to approximately fit the
/// text, then renders the Frame inside. Needed because egui's
/// `ui.horizontal` inside a Frame claims the full available width as
/// its *allocated* rect — so `vertical_centered` sees a full-width
/// widget and the visual pill ends up flush-left even though it's
/// drawing smaller content. Sizing the outer allocation explicitly
/// lets the parent `vertical_centered` center it correctly.
fn render_error_pill(ui: &mut egui::Ui, tint: egui::Color32, prefix: &str, detail: &str) {
    let max_chars = 90;
    let trimmed = if detail.chars().count() > max_chars {
        let cut: String = detail.chars().take(max_chars).collect();
        format!("{}…", cut)
    } else {
        detail.to_string()
    };
    // Rough width estimate — icon + spacings + glyph widths at
    // 12 px monospace (≈7 px per char) + Frame margins. Caps at the
    // panel's usable width so very long errors still fit on screen.
    let approx_chars = prefix.chars().count() + 1 + trimmed.chars().count();
    let pill_w = (18.0 + 8.0 + approx_chars as f32 * 7.3 + 24.0).min(ui.available_width() - 32.0);

    ui.allocate_ui_with_layout(
        egui::vec2(pill_w, 0.0),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            egui::Frame::none()
                .fill(tint.linear_multiply(0.22))
                .stroke(egui::Stroke::new(1.0, tint.linear_multiply(0.55)))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(egui_phosphor::regular::WARNING)
                                .size(15.0)
                                .color(tint),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(prefix)
                                .color(tint)
                                .strong()
                                .font(egui::FontId::monospace(12.5)),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(&trimmed)
                                .color(text())
                                .font(egui::FontId::monospace(12.0)),
                        );
                    });
                });
        },
    );
}

// Hand-drawn paint_unplugged_plug and paint_warning_icon removed —
// replaced by Phosphor icon font glyphs (WIFI_SLASH, PROHIBIT, WARNING).

#[cfg(test)]
mod tests {
    use super::*;

    fn pretty(v: serde_json::Value) -> String {
        serde_json::to_string_pretty(&v).unwrap()
    }

    #[test]
    fn fold_pairs_simple_object() {
        // Pretty-printed `{ "a": 1 }` is three lines:
        //   1: {
        //   2:   "a": 1
        //   3: }
        // The opener is line 1, closer is line 3.
        let text = pretty(serde_json::json!({ "a": 1 }));
        let pairs = compute_json_fold_pairs(&text);
        assert_eq!(pairs.get(&1), Some(&3));
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn fold_pairs_nested_object_and_array() {
        let text = pretty(serde_json::json!({
            "outer": {
                "inner": [1, 2, 3]
            }
        }));
        let pairs = compute_json_fold_pairs(&text);
        // Three openers — outer object, inner object, inner array —
        // each must point at its own closer line. We don't pin exact
        // line numbers because pretty-print formatting could shift
        // by one across serde versions; we just check that every
        // opener has a strictly-greater closer and the count is right.
        assert_eq!(pairs.len(), 3);
        for (open, close) in &pairs {
            assert!(
                close > open,
                "closer {} must come after opener {}",
                close,
                open
            );
        }
    }

    #[test]
    fn fold_pairs_single_line_blocks_excluded() {
        // Single-line `{}` and `[]` have nothing to fold — the closer
        // sits on the same line as the opener, so we don't record
        // the pair.
        let pairs = compute_json_fold_pairs("{}");
        assert!(pairs.is_empty());

        let pairs = compute_json_fold_pairs("{ \"empty\": [] }");
        assert!(pairs.is_empty());
    }

    #[test]
    fn fold_pairs_braces_inside_strings_ignored() {
        // The `{` and `]` inside the string value must NOT be parsed
        // as structure. Without string-awareness we'd mis-match the
        // brace stack and produce wrong pairs.
        let text = "{\n  \"note\": \"contains { and ] chars\"\n}";
        let pairs = compute_json_fold_pairs(text);
        assert_eq!(pairs.get(&1), Some(&3));
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn fold_pairs_escaped_quote_does_not_close_string() {
        // `\"` inside a string must not terminate it — otherwise we'd
        // start treating subsequent `{` as structure mid-string.
        let text = "{\n  \"q\": \"he said \\\"hi\\\" then {\"\n}";
        let pairs = compute_json_fold_pairs(text);
        assert_eq!(pairs.get(&1), Some(&3));
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn fold_pairs_empty_text() {
        let pairs = compute_json_fold_pairs("");
        assert!(pairs.is_empty());
    }

    #[test]
    fn effective_body_view_preserves_json_preference_for_json_body() {
        assert!(matches!(
            effective_body_view(BodyView::Json, true, false, false, false),
            BodyView::Json
        ));
        assert!(matches!(
            effective_body_view(BodyView::Tree, true, false, false, false),
            BodyView::Tree
        ));
    }

    #[test]
    fn effective_body_view_falls_back_to_raw_for_html_when_json_selected() {
        assert!(matches!(
            effective_body_view(BodyView::Json, false, true, false, false),
            BodyView::Raw
        ));
        assert!(matches!(
            effective_body_view(BodyView::Tree, false, true, false, false),
            BodyView::Raw
        ));
    }

    #[test]
    fn effective_body_view_falls_back_to_raw_for_unavailable_optional_views() {
        assert!(matches!(
            effective_body_view(BodyView::Preview, false, false, false, false),
            BodyView::Raw
        ));
        assert!(matches!(
            effective_body_view(BodyView::Events, false, false, false, false),
            BodyView::Raw
        ));
        assert!(matches!(
            effective_body_view(BodyView::Diff, true, false, false, false),
            BodyView::Raw
        ));
    }

    #[test]
    fn effective_body_view_falls_back_to_raw_for_sse_when_json_selected() {
        assert!(matches!(
            effective_body_view(BodyView::Json, true, false, true, false),
            BodyView::Raw
        ));
        assert!(matches!(
            effective_body_view(BodyView::Tree, true, false, true, false),
            BodyView::Raw
        ));
    }

    #[test]
    fn effective_body_view_keeps_requested_diff_when_snapshot_exists() {
        assert!(matches!(
            effective_body_view(BodyView::Diff, true, false, false, true),
            BodyView::Diff
        ));
    }

    #[test]
    fn response_summary_is_quiet_for_json_success() {
        let summary = classify_response_summary(
            "200 OK",
            &[("content-type".to_string(), "application/json".to_string())],
            "{\"ok\":true}",
            true,
            BodyView::Json,
        );
        assert!(summary.is_none());
    }

    #[test]
    fn response_summary_calls_out_cloudflare_challenge() {
        let summary = classify_response_summary(
            "403 Forbidden",
            &[("content-type".to_string(), "text/html".to_string())],
            "<!doctype html><title>Just a moment...</title><script src=\"/cdn-cgi/challenge-platform/h/b/orchestrate/jsch/v1\"></script>Cloudflare",
            false,
            BodyView::Json,
        )
        .unwrap();
        assert_eq!(
            summary.headline,
            "Cloudflare challenge returned instead of API data"
        );
        assert!(summary.detail.contains("official API auth"));
        assert!(summary.detail.contains("session cookies are sensitive"));
        assert!(summary.is_error);
    }

    #[test]
    fn response_summary_reports_invalid_json_view() {
        let summary = classify_response_summary(
            "200 OK",
            &[("content-type".to_string(), "text/plain".to_string())],
            "ok",
            false,
            BodyView::Json,
        )
        .unwrap();
        assert_eq!(summary.headline, "Response is not valid JSON");
        assert!(summary
            .detail
            .contains("JSON and Tree views are unavailable"));
        assert!(!summary.is_error);
    }

    #[test]
    fn response_summary_explains_plain_text_after_auto_switch() {
        let summary = classify_response_summary(
            "200 OK",
            &[("content-type".to_string(), "text/plain".to_string())],
            "ok",
            false,
            BodyView::Raw,
        )
        .unwrap();
        assert_eq!(summary.headline, "Non-JSON response");
        assert!(summary.detail.contains("Raw shows the original body"));
    }

    #[test]
    fn response_find_width_is_bounded_for_narrow_and_wide_toolbars() {
        assert_eq!(response_find_width(120.0), 120.0);
        assert_eq!(response_find_width(240.0), 240.0);
        assert_eq!(response_find_width(600.0), 360.0);
    }

    #[test]
    fn response_find_row_width_leaves_right_gutter() {
        assert_eq!(response_find_row_width(10.0), 0.0);
        assert_eq!(response_find_row_width(240.0), 228.0);
        assert_eq!(response_find_row_width(600.0), 360.0);
        assert!(response_find_row_width(240.0) <= 240.0 - 12.0);
    }

    #[test]
    fn response_find_inner_width_subtracts_frame_padding() {
        assert_eq!(response_find_inner_width(360.0, 8.0), 344.0);
        assert_eq!(response_find_inner_width(12.0, 8.0), 0.0);
    }

    #[test]
    fn response_find_outer_width_includes_frame_padding() {
        let outer_w = response_find_width(600.0);
        let content_w = response_find_inner_width(outer_w, 8.0);
        assert_eq!(content_w + 16.0, outer_w);
    }

    #[test]
    fn response_find_input_width_reserves_count_and_close_controls() {
        assert_eq!(response_find_input_width(220.0, 0.0, 22.0, 6.0), 192.0);
        assert_eq!(response_find_input_width(220.0, 42.0, 22.0, 12.0), 144.0);
        assert_eq!(response_find_input_width(90.0, 42.0, 22.0, 12.0), 14.0);
        assert_eq!(response_find_input_width(40.0, 42.0, 22.0, 12.0), 0.0);
    }

    #[test]
    fn response_find_count_text_shows_active_and_total() {
        assert_eq!(response_find_count_text("", 0, 0), "");
        assert_eq!(response_find_count_text("ok", 0, 0), "0/0");
        assert_eq!(response_find_count_text("ok", 3, 0), "1/3");
        assert_eq!(response_find_count_text("ok", 3, 2), "3/3");
    }

    #[test]
    fn response_find_enter_navigation_wraps() {
        assert_eq!(next_search_match_index(0, 0, false), 0);
        assert_eq!(next_search_match_index(0, 3, false), 1);
        assert_eq!(next_search_match_index(2, 3, false), 0);
        assert_eq!(next_search_match_index(0, 3, true), 2);
        assert_eq!(next_search_match_index(2, 3, true), 1);
    }
}
