use super::*;

impl ApiClient {
    pub(crate) fn render_snippet_panel(&mut self, ctx: &egui::Context) {
        if !self.show_snippet_panel {
            return;
        }
        let req = match self.get_current_request() {
            Some(r) => r,
            None => {
                self.show_snippet_panel = false;
                return;
            }
        };
        let snippet = render_snippet_redacted(&req, self.snippet_lang);
        let raw_snippet = render_snippet(&req, self.snippet_lang);
        let mut copy_clicked = false;
        let mut copy_raw_clicked = false;
        let mut close_clicked = false;

        egui::SidePanel::right("snippet_panel")
            .resizable(true)
            .default_width(380.0)
            .width_range(280.0..=600.0)
            .frame(
                egui::Frame::none()
                    .fill(bg())
                    .inner_margin(egui::Margin::symmetric(10.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Code snippet")
                            .size(14.0)
                            .strong()
                            .color(text()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if close_x_button(ui, "Close panel").clicked() {
                            close_clicked = true;
                        }
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("snippet_lang")
                        .selected_text(self.snippet_lang.label())
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for &lang in &[
                                SnippetLang::Curl,
                                SnippetLang::Python,
                                SnippetLang::JavaScript,
                                SnippetLang::HttpieShell,
                            ] {
                                ui.selectable_value(&mut self.snippet_lang, lang, lang.label());
                            }
                        });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_btn(ui, egui_phosphor::regular::COPY, "Copy redacted snippet")
                            .clicked()
                        {
                            copy_clicked = true;
                        }
                        if ui
                            .button("Copy raw")
                            .on_hover_text("Copy snippet with original values")
                            .clicked()
                        {
                            copy_raw_clicked = true;
                        }
                        // "Copied!" flash sits to the LEFT of the button
                        // (layout is right-to-left). Visible for ~1.5s
                        // after click, then fades out. Gives inline
                        // feedback — the bottom-right toast gets hidden
                        // behind this side panel so it wasn't noticed.
                        if let Some(t0) = self.snippet_copied_at {
                            let now = ui.ctx().input(|i| i.time);
                            let age = now - t0;
                            if age < 1.5 {
                                ui.add_space(6.0);
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
                                self.snippet_copied_at = None;
                            }
                        }
                    });
                });
                ui.add_space(8.0);
                egui::Frame::none()
                    .fill(panel_dark())
                    .stroke(egui::Stroke::new(1.0, border()))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        let avail_h = ui.available_height();
                        egui::ScrollArea::vertical()
                            .id_salt("snippet_scroll")
                            .auto_shrink([false, false])
                            .max_height(avail_h)
                            .show(ui, |ui| {
                                // Two-column layout so wrapped lines
                                // stay inside the content column instead
                                // of snapping back to x=0 and colliding
                                // with the next logical line's gutter.
                                let gutter_w = 32.0;
                                let line_count = snippet.split('\n').count().max(1);
                                ui.horizontal_top(|ui| {
                                    // Left — line numbers. One label per
                                    // logical line; if the content wraps
                                    // to multiple visual rows the gutter
                                    // will trail shorter than the content,
                                    // which matches Postman's behavior.
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
                                    // Right — content; TextEdit owns its
                                    // own wrap inside the remaining width.
                                    let mut text = snippet.clone();
                                    let lang = self.snippet_lang;
                                    let mut layouter =
                                        move |ui: &egui::Ui, s: &str, wrap_width: f32| {
                                            let mut job =
                                                build_snippet_layout_job_content_only(s, lang);
                                            job.wrap.max_width = wrap_width;
                                            ui.fonts(|f| f.layout_job(job))
                                        };
                                    ui.add(
                                        egui::TextEdit::multiline(&mut text)
                                            .code_editor()
                                            .frame(false)
                                            .interactive(false)
                                            .desired_width(f32::INFINITY)
                                            .layouter(&mut layouter),
                                    );
                                });
                            });
                    });
            });

        if copy_clicked {
            ctx.output_mut(|o| o.copied_text = snippet);
            self.snippet_copied_at = Some(ctx.input(|i| i.time));
        }
        if copy_raw_clicked {
            ctx.output_mut(|o| o.copied_text = raw_snippet);
            self.snippet_copied_at = Some(ctx.input(|i| i.time));
        }
        if close_clicked {
            self.show_snippet_panel = false;
        }
    }
}
