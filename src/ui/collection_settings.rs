use crate::theme::*;
use crate::{find_folder_by_id, find_folder_by_id_mut, ApiClient};
use eframe::egui;

impl ApiClient {
    pub(crate) fn render_collection_settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_collection_settings_modal {
            return;
        }

        let Some(folder_id) = self.collection_settings_folder_id.clone() else {
            self.show_collection_settings_modal = false;
            return;
        };
        let Some(folder) = find_folder_by_id(&self.state.folders, &folder_id).cloned() else {
            self.show_collection_settings_modal = false;
            self.show_toast("Collection not found");
            return;
        };

        let mut open = self.show_collection_settings_modal;
        let mut sync = folder.sync.clone();
        let mut changed = false;
        let mut choose_git_dir = false;
        let mut choose_openapi_file = false;
        let mut import_workspace = false;
        let mut export_workspace = false;
        let mut pull_remote = false;
        let mut push_remote = false;
        let mut refresh_git_status = false;
        let mut refresh_openapi = false;

        egui::Window::new("Collection Settings")
            .collapsible(false)
            .resizable(false)
            .default_width(660.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(620.0);
                ui.label(
                    egui::RichText::new(&folder.name)
                        .size(16.0)
                        .strong()
                        .color(text()),
                );
                ui.label(
                    egui::RichText::new(
                        "Link this collection to a local folder or Git repository. Private \
                         GitHub/GitLab/Bitbucket repos work through your existing local Git \
                         credentials; Rusty Requester does not store Git provider tokens.",
                    )
                    .size(10.5)
                    .color(muted()),
                );
                ui.add_space(12.0);

                ui.label(
                    egui::RichText::new("Collection directory")
                        .size(12.0)
                        .strong()
                        .color(text()),
                );
                ui.label(
                    egui::RichText::new(
                        "Exports this collection as reviewable workspace files. Requests are \
                         saved as compact .rr text files for readable pull requests.",
                    )
                    .size(10.5)
                    .color(muted()),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Directory").size(11.0).color(muted()));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut sync.git_workspace_dir)
                            .hint_text(hint("/path/to/collection-repo"))
                            .desired_width(ui.available_width() - 96.0),
                    );
                    changed |= response.changed();
                    if ui.button("Choose...").clicked() {
                        choose_git_dir = true;
                    }
                });

                let git_ready = !sync.git_workspace_dir.trim().is_empty();
                let is_git_repo = git_ready
                    && std::path::Path::new(sync.git_workspace_dir.trim())
                        .join(".git")
                        .is_dir();
                if git_ready && !is_git_repo {
                    ui.label(
                        egui::RichText::new(
                            "Remote pull/push needs the repository root containing .git.",
                        )
                        .size(10.5)
                        .color(C_ORANGE),
                    );
                }

                ui.add_space(6.0);
                if ui
                    .checkbox(
                        &mut sync.include_secrets_in_git_workspace,
                        "Include secrets in collection exports",
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.label(
                    egui::RichText::new(
                        "Keep this off for shared repos. A private repo is access control, not \
                         encryption or a guarantee that secrets are safe to commit.",
                    )
                    .size(10.5)
                    .color(muted()),
                );

                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Mask rules")
                        .size(12.0)
                        .strong()
                        .color(text()),
                );
                ui.label(
                    egui::RichText::new(
                        "Comma-separated key/header/env-name patterns. Keep readable wins over \
                         built-in and custom mask rules.",
                    )
                    .size(10.5)
                    .color(muted()),
                );
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Always mask").size(11.0).color(muted()));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut sync.mask_key_patterns)
                            .hint_text(hint("x-api-key, token, authorization"))
                            .desired_width(ui.available_width()),
                    );
                    changed |= response.changed();
                });
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Keep readable")
                            .size(11.0)
                            .color(muted()),
                    );
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut sync.allow_key_patterns)
                            .hint_text(hint("platform, env, app-version"))
                            .desired_width(ui.available_width()),
                    );
                    changed |= response.changed();
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let busy = self.sync_in_flight.is_some();
                    if ui
                        .add_enabled(git_ready && !busy, egui::Button::new("Import from folder"))
                        .clicked()
                    {
                        import_workspace = true;
                    }
                    if ui
                        .add_enabled(git_ready && !busy, egui::Button::new("Export to folder"))
                        .clicked()
                    {
                        export_workspace = true;
                    }
                });

                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("Git remote")
                        .size(12.0)
                        .strong()
                        .color(text()),
                );
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Commit").size(11.0).color(muted()));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut sync.git_commit_message)
                            .hint_text(hint("Sync Rusty Requester collection"))
                            .desired_width(ui.available_width()),
                    );
                    changed |= response.changed();
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let busy = self.sync_in_flight.is_some();
                    if ui
                        .add_enabled(is_git_repo && !busy, egui::Button::new("Refresh changes"))
                        .clicked()
                    {
                        refresh_git_status = true;
                    }
                    if ui
                        .add_enabled(is_git_repo && !busy, egui::Button::new("Pull from remote"))
                        .clicked()
                    {
                        pull_remote = true;
                    }
                    if ui
                        .add_enabled(is_git_repo && !busy, egui::Button::new("Commit and push"))
                        .clicked()
                    {
                        push_remote = true;
                    }
                });
                if !self.collection_git_status.is_empty() {
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(elevated())
                        .stroke(egui::Stroke::new(1.0, border()))
                        .rounding(egui::Rounding::same(6.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                        .show(ui, |ui| {
                            ui.set_max_height(120.0);
                            egui::ScrollArea::vertical()
                                .max_height(120.0)
                                .show(ui, |ui| {
                                    ui.monospace(&self.collection_git_status);
                                });
                        });
                }

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

                ui.label(
                    egui::RichText::new("OpenAPI refresh")
                        .size(12.0)
                        .strong()
                        .color(text()),
                );
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Spec file").size(11.0).color(muted()));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut sync.openapi_spec_path)
                            .hint_text(hint("/path/to/openapi.yaml"))
                            .desired_width(ui.available_width() - 96.0),
                    );
                    changed |= response.changed();
                    if ui.button("Choose...").clicked() {
                        choose_openapi_file = true;
                    }
                });
                ui.add_space(6.0);
                if ui
                    .add_enabled(
                        !sync.openapi_spec_path.trim().is_empty() && self.sync_in_flight.is_none(),
                        egui::Button::new("Refresh collection from OpenAPI"),
                    )
                    .clicked()
                {
                    refresh_openapi = true;
                }

                if let Some(sync) = &self.sync_in_flight {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(egui::RichText::new(&sync.label).size(11.0).color(muted()));
                    });
                }

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        self.show_collection_settings_modal = false;
                    }
                });
            });

        self.show_collection_settings_modal = open && self.show_collection_settings_modal;

        if changed {
            if let Some(folder) = find_folder_by_id_mut(&mut self.state.folders, &folder_id) {
                folder.sync = sync;
                self.save_state();
            }
        }
        if choose_git_dir {
            self.choose_collection_git_workspace_dir(&folder_id);
        }
        if choose_openapi_file {
            self.choose_collection_openapi_spec_file(&folder_id);
        }
        if import_workspace {
            self.import_collection_workspace_from_config(&folder_id);
        }
        if export_workspace {
            self.export_collection_workspace_from_config(&folder_id);
        }
        if pull_remote {
            self.pull_collection_workspace_from_config(&folder_id);
        }
        if push_remote {
            self.push_collection_workspace_from_config(&folder_id);
        }
        if refresh_git_status {
            self.refresh_collection_git_status_from_config(&folder_id);
        }
        if refresh_openapi {
            self.refresh_collection_openapi_from_config(&folder_id);
        }
    }
}
