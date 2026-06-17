use super::*;

impl ApiClient {
    /// In-window menu bar along the top of the window. Cross-platform
    /// (macOS + Linux) — egui doesn't drive the native macOS NSMenu
    /// bar, so we render our own strip here. Keeps the same items
    /// regardless of OS so behavior is consistent.
    #[cfg(not(target_os = "macos"))]
    pub(crate) fn render_menu_bar(&mut self, ctx: &egui::Context) {
        // Actions flow out of the closures via mutable flags so we can
        // react after the panel closes (avoids borrow conflicts with
        // `self` inside the menu closures).
        let mut action_new_request = false;
        let mut action_new_collection = false;
        let mut action_import = false;
        let mut action_paste_curl = false;
        let mut action_create_backup = false;
        let mut action_open_backups = false;
        let mut action_open_sync = false;
        let mut action_export_json = false;
        let mut action_export_yaml = false;
        let mut action_open_runner = false;
        let mut action_toggle_snippet = false;
        let mut action_open_settings = false;
        let mut action_open_env = false;
        let mut action_show_about = false;
        let mut action_quit = false;

        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(panel_dark())
                    .inner_margin(egui::Margin::symmetric(4.0, 2.0)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("New Request").clicked() {
                            action_new_request = true;
                            ui.close_menu();
                        }
                        if ui.button("New Collection").clicked() {
                            action_new_collection = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Import collection file…").clicked() {
                            action_import = true;
                            ui.close_menu();
                        }
                        if ui.button("Paste cURL command…").clicked() {
                            action_paste_curl = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Create Workspace Backup").clicked() {
                            action_create_backup = true;
                            ui.close_menu();
                        }
                        if ui.button("Backups…").clicked() {
                            action_open_backups = true;
                            ui.close_menu();
                        }
                        if ui.button("Workspace Sync…").clicked() {
                            action_open_sync = true;
                            ui.close_menu();
                        }
                        if ui.button("Export all as JSON…").clicked() {
                            action_export_json = true;
                            ui.close_menu();
                        }
                        if ui.button("Export all as YAML…").clicked() {
                            action_export_yaml = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            action_quit = true;
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("View", |ui| {
                        if ui.button("Toggle code snippet panel").clicked() {
                            action_toggle_snippet = true;
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Request", |ui| {
                        if ui.button("Collection Runner…").clicked() {
                            action_open_runner = true;
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Settings", |ui| {
                        if ui.button("Preferences…").clicked() {
                            action_open_settings = true;
                            ui.close_menu();
                        }
                        if ui.button("Environments…").clicked() {
                            action_open_env = true;
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Help", |ui| {
                        if ui.button("About Rusty Requester").clicked() {
                            action_show_about = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Open GitHub repo").clicked() {
                            ctx.output_mut(|o| {
                                o.open_url = Some(egui::output::OpenUrl {
                                    url: "https://github.com/chud-lori/rusty-requester".to_string(),
                                    new_tab: true,
                                });
                            });
                            ui.close_menu();
                        }
                        if ui.button("Report an issue").clicked() {
                            ctx.output_mut(|o| {
                                o.open_url = Some(egui::output::OpenUrl {
                                    url: "https://github.com/chud-lori/rusty-requester/issues"
                                        .to_string(),
                                    new_tab: true,
                                });
                            });
                            ui.close_menu();
                        }
                    });
                });
            });

        // Apply actions after the panel closes.
        if action_new_request {
            self.new_draft_request();
        }
        if action_new_collection {
            self.state.folders.push(crate::model::Folder {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("Collection {}", self.state.folders.len() + 1),
                requests: vec![],
                subfolders: vec![],
                description: String::new(),
                sync: SyncConfig::default(),
            });
            self.save_state();
        }
        if action_import {
            self.pending_import = true;
        }
        if action_paste_curl {
            self.show_paste_modal = true;
            self.paste_curl_text.clear();
            self.paste_error.clear();
        }
        if action_create_backup {
            self.create_workspace_backup_now();
        }
        if action_open_backups {
            self.show_backup_modal = true;
        }
        if action_open_sync {
            self.open_sync_or_settings();
        }
        if action_export_json {
            self.pending_export_json = true;
        }
        if action_export_yaml {
            self.pending_export_yaml = true;
        }
        if action_toggle_snippet {
            self.show_snippet_panel = !self.show_snippet_panel;
        }
        if action_open_runner {
            self.show_runner_modal = true;
        }
        if action_open_settings {
            self.editing_settings = self.state.settings.clone();
            self.show_settings_modal = true;
        }
        if action_open_env {
            self.show_env_modal = true;
            if self.selected_env_for_edit.is_none() {
                self.selected_env_for_edit = self.state.environments.first().map(|e| e.id.clone());
            }
        }
        if action_show_about {
            self.show_about_modal = true;
        }
        if action_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
