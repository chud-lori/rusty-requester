use crate::model::Theme;
use crate::theme::*;
use crate::{in_app_update_supported, spawn_update_check, ApiClient, UpdateCheckOutcome};
use eframe::egui;

impl ApiClient {
    /// App-wide settings modal. Edits are staged in `editing_settings`
    /// and only persisted when Save is pressed.
    pub(crate) fn render_settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_settings_modal {
            return;
        }
        let mut open = self.show_settings_modal;
        let mut do_save = false;
        let mut do_cancel = false;
        let mut do_check_updates = false;
        let mut do_inline_update_now = false;
        let mut do_open_releases_url: Option<String> = None;

        egui::Window::new(
            egui::RichText::new("SETTINGS")
                .size(12.0)
                .strong()
                .color(muted()),
        )
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(440.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_width(420.0);

            ui.label(
                egui::RichText::new("Request timeout (seconds)")
                    .size(11.5)
                    .color(muted()),
            );
            ui.add(
                egui::DragValue::new(&mut self.editing_settings.timeout_sec)
                    .range(0..=3600)
                    .speed(1.0)
                    .suffix(" s"),
            );
            ui.label(
                egui::RichText::new("0 disables the timeout (requests can hang forever).")
                    .size(10.5)
                    .color(muted()),
            );
            ui.add_space(10.0);

            ui.label(
                egui::RichText::new("Max response body (MB)")
                    .size(11.5)
                    .color(muted()),
            );
            ui.add(
                egui::DragValue::new(&mut self.editing_settings.max_body_mb)
                    .range(0..=2048)
                    .speed(1.0)
                    .suffix(" MB"),
            );
            ui.label(
                egui::RichText::new(
                    "Responses larger than this are truncated with a banner. \
                         0 disables the cap (huge payloads may OOM the app).",
                )
                .size(10.5)
                .color(muted()),
            );
            ui.add_space(10.0);

            ui.label(egui::RichText::new("Proxy URL").size(11.5).color(muted()));
            ui.add(
                egui::TextEdit::singleline(&mut self.editing_settings.proxy_url)
                    .hint_text(hint("http://proxy:8080 (leave empty for direct)"))
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(10.0);

            ui.checkbox(
                &mut self.editing_settings.verify_tls,
                "Verify TLS certificates",
            );
            ui.label(
                egui::RichText::new(
                    "Unchecked = accept self-signed / expired certs. \
                         Dangerous on the public internet; useful for internal dev APIs.",
                )
                .size(10.5)
                .color(muted()),
            );

            ui.add_space(10.0);
            ui.label(egui::RichText::new("Theme").size(11.0).color(muted()));
            ui.horizontal(|ui| {
                let mut t = self.editing_settings.theme;
                ui.selectable_value(&mut t, Theme::Dark, "Dark");
                ui.selectable_value(&mut t, Theme::Light, "Light");
                ui.selectable_value(&mut t, Theme::Postman, "Postman");
                self.editing_settings.theme = t;
            });
            ui.label(
                egui::RichText::new(
                    "Light theme flips egui's chrome (panels, text, borders). \
                         HTTP method colors and status pills stay the same across \
                         themes.",
                )
                .size(10.5)
                .color(muted()),
            );

            ui.add_space(10.0);
            ui.checkbox(
                &mut self.editing_settings.workspace_sync_enabled,
                "Enable Workspace Sync",
            );
            ui.label(
                egui::RichText::new(
                    "Optional local workflow for Git workspace import/export and \
                         OpenAPI refresh. Off by default; no background Git or \
                         network sync runs automatically.",
                )
                .size(10.5)
                .color(muted()),
            );

            ui.add_space(10.0);
            ui.checkbox(
                &mut self.editing_settings.check_updates_on_launch,
                "Check for updates on launch",
            );
            ui.label(
                egui::RichText::new(
                    "Silent GET to github.com/.../releases/latest once per \
                         launch. No account, no telemetry — disable for \
                         zero outbound traffic.",
                )
                .size(10.5)
                .color(muted()),
            );
            ui.add_space(4.0);
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Check for updates now")
                            .size(11.0)
                            .color(text()),
                    )
                    .fill(elevated())
                    .min_size(egui::vec2(0.0, 26.0)),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                do_check_updates = true;
            }

            let available = self.update_available.clone();
            let inline_state = if available.is_some() {
                "available"
            } else if matches!(self.manual_update_check, UpdateCheckOutcome::Checking) {
                "checking"
            } else if matches!(self.manual_update_check, UpdateCheckOutcome::Failed(_)) {
                "failed"
            } else if matches!(self.manual_update_check, UpdateCheckOutcome::NoUpdate) {
                "uptodate"
            } else {
                "idle"
            };
            match inline_state {
                "checking" => {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(
                            egui::RichText::new("Checking GitHub…")
                                .size(11.0)
                                .color(muted()),
                        );
                    });
                }
                "uptodate" => {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "{}  You're on the latest version (v{})",
                            egui_phosphor::regular::CHECK,
                            env!("CARGO_PKG_VERSION"),
                        ))
                        .size(11.0)
                        .color(muted()),
                    );
                }
                "failed" => {
                    let reason = match &self.manual_update_check {
                        UpdateCheckOutcome::Failed(reason) => reason.as_str(),
                        _ => "Update check failed",
                    };
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "{}  Could not check for updates: {}",
                            egui_phosphor::regular::WARNING,
                            reason
                        ))
                        .size(11.0)
                        .color(C_ORANGE),
                    );
                }
                "available" => {
                    let tag = available.unwrap_or_default();
                    let version = tag.trim_start_matches('v');
                    let release_url = format!(
                        "https://github.com/chud-lori/rusty-requester/releases/tag/{}",
                        tag,
                    );
                    let in_app_ok = in_app_update_supported();
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(elevated())
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(76, 175, 80)))
                        .rounding(egui::Rounding::same(6.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Update available: v{}  (you're on v{})",
                                    version,
                                    env!("CARGO_PKG_VERSION"),
                                ))
                                .size(12.0)
                                .strong()
                                .color(text()),
                            );
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if in_app_ok
                                    && ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("Update now")
                                                    .color(egui::Color32::WHITE)
                                                    .strong()
                                                    .size(11.0),
                                            )
                                            .fill(accent())
                                            .min_size(egui::vec2(110.0, 26.0)),
                                        )
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                {
                                    do_inline_update_now = true;
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Release notes")
                                                .color(text())
                                                .size(11.0),
                                        )
                                        .fill(elevated())
                                        .min_size(egui::vec2(110.0, 26.0)),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    do_open_releases_url = Some(release_url);
                                }
                            });
                        });
                }
                _ => {}
            }

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    do_cancel = true;
                }
                let save_btn = egui::Button::new(
                    egui::RichText::new("Save")
                        .color(egui::Color32::WHITE)
                        .strong(),
                )
                .fill(accent())
                .min_size(egui::vec2(80.0, 28.0));
                if ui.add(save_btn).clicked() {
                    do_save = true;
                }
            });

            ui.input(|i| {
                if i.key_pressed(egui::Key::Escape) {
                    do_cancel = true;
                }
            });
        });
        self.show_settings_modal = open;

        if do_save {
            self.state.settings = self.editing_settings.clone();
            self.http_client = crate::net::build_client(&self.state.settings);
            self.save_state();
            self.show_toast("Settings saved");
            self.show_settings_modal = false;
        }
        if do_cancel || !self.show_settings_modal {
            self.show_settings_modal = false;
            self.editing_settings = self.state.settings.clone();
        }
        if do_check_updates {
            self.update_check_rx = Some(spawn_update_check(&self.http_runtime));
            self.state.settings.dismissed_update_version = None;
            self.editing_settings.dismissed_update_version = None;
            self.manual_update_check = UpdateCheckOutcome::Checking;
            self.save_state();
            self.show_toast("Checking for updates…");
        }
        if do_inline_update_now {
            self.state.settings = self.editing_settings.clone();
            self.http_client = crate::net::build_client(&self.state.settings);
            self.save_state();
            self.show_settings_modal = false;
            self.spawn_update();
        }
        if let Some(url) = do_open_releases_url {
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(&url).spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", &url])
                .spawn();
        }
    }
}
