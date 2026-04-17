//! Response panel — status/time/size info bar with hover tooltips,
//! body view modes (JSON / Tree / Raw) with syntax highlighting,
//! search & copy toolbar, headers grid, loading spinner, and the
//! empty state.

use crate::model::*;
use crate::snippet::build_json_layout_job_content_only_with_search;
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
                    .color(muted())
                    .size(11.5)
                    .italics(),
            );
            return;
        }
        let bullet_sep = |ui: &mut egui::Ui| {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("•")
                    .color(muted().linear_multiply(0.7))
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
                    .color(muted())
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
                    .color(muted())
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
                        let dot = C_ACCENT.linear_multiply(0.4 + 0.6 * pulse);
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
                // The Events pill is exclusive to SSE responses —
                // when present we auto-select it and hide the generic
                // JSON/Tree pills (the per-event body is already
                // pretty-printed). Streamed SSE bodies aren't JSON-
                // parseable anyway.
                let is_sse_body = crate::sse::is_event_stream(&self.response_headers)
                    || !self.streaming_events.is_empty();
                if is_sse_body {
                    body_view_pill(
                        ui,
                        &mut view,
                        BodyView::Events,
                        &format!("Events ({})", self.streaming_events.len()),
                    );
                } else {
                    body_view_pill(ui, &mut view, BodyView::Json, "JSON");
                    body_view_pill(ui, &mut view, BodyView::Tree, "Tree");
                }
                // The Preview pill only surfaces for HTML responses.
                // We detect HTML via Content-Type (authoritative) +
                // body sniff (fallback for header-less responses).
                let is_html_body =
                    crate::html_preview::is_html(&self.response_headers, &self.response_text);
                if is_html_body {
                    body_view_pill(ui, &mut view, BodyView::Preview, "Preview");
                }
                // Diff pill surfaces only when a prior response exists.
                let has_diff_snapshot = self.previous_response_text.is_some();
                if has_diff_snapshot {
                    body_view_pill(ui, &mut view, BodyView::Diff, "Diff");
                }
                body_view_pill(ui, &mut view, BodyView::Raw, "Raw");
                // If the user had a pill selected that no longer
                // applies to this response, fall back to a sensible
                // default.
                if matches!(view, BodyView::Preview) && !is_html_body {
                    view = BodyView::Raw;
                }
                if matches!(view, BodyView::Events) && !is_sse_body {
                    view = BodyView::Raw;
                }
                if is_sse_body && matches!(view, BodyView::Json | BodyView::Tree) {
                    view = BodyView::Events;
                }
                if matches!(view, BodyView::Diff) && !has_diff_snapshot {
                    view = BodyView::Raw;
                }
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
                    if icon_btn(
                        ui,
                        egui_phosphor::regular::DOWNLOAD_SIMPLE,
                        "Save response to file",
                    )
                    .clicked()
                    {
                        save_clicked = true;
                    }
                    ui.add_space(2.0);
                    if icon_btn(ui, egui_phosphor::regular::COPY, "Copy response body").clicked() {
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
        if body_active && self.body_search_visible {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.body_search_query)
                        .hint_text("Find in body…")
                        .desired_width(ui.available_width() - 40.0),
                );
                if icon_btn(ui, egui_phosphor::regular::X, "Close search").clicked() {
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
                        ui.add(egui::Spinner::new().size(26.0).color(C_ACCENT));
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
                                    // Two-column layout — separate
                                    // gutter column on the left, content
                                    // on the right. Matches the snippet
                                    // panel and Postman's response view:
                                    // vertical scroll only, content
                                    // wraps inside its own column so
                                    // wrapped rows never collide with
                                    // the line numbers. Earlier nested
                                    // horizontal ScrollArea allowed
                                    // diagonal drag-scrolling that felt
                                    // broken.
                                    let gutter_w = 44.0;
                                    let line_count =
                                        self.response_text.split('\n').count().max(1);
                                    ui.horizontal_top(|ui| {
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 0.0;
                                            for i in 1..=line_count {
                                                ui.add_sized(
                                                    [gutter_w, 17.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(format!("{:>3}", i))
                                                            .color(egui::Color32::from_rgb(
                                                                100, 105, 115,
                                                            ))
                                                            .font(egui::FontId::monospace(12.5)),
                                                    ),
                                                );
                                            }
                                        });
                                        ui.add_space(6.0);
                                        let mut buf: &str = &self.response_text;
                                        let search = self.body_search_query.clone();
                                        let mut layouter =
                                            move |ui: &egui::Ui, s: &str, wrap_width: f32| {
                                                let mut job =
                                                    build_json_layout_job_content_only_with_search(
                                                        s, &search,
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
