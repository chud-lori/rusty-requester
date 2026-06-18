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
            .resizable(true)
            .default_width(860.0)
            .default_height(620.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .open(&mut open)
            .show(ctx, |ui| {
                let git_ready = !sync.git_workspace_dir.trim().is_empty();
                let is_git_repo = git_ready
                    && std::path::Path::new(sync.git_workspace_dir.trim())
                        .join(".git")
                        .is_dir();
                let busy = self.sync_in_flight.is_some();
                let openapi_ready = !sync.openapi_spec_path.trim().is_empty();

                ui.set_min_size(egui::vec2(780.0, 540.0));
                ui.horizontal(|ui| {
                    collection_settings_nav(ui, &folder.name, git_ready, is_git_repo, openapi_ready);

                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("collection_settings_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_width(560.0);

                            setting_card(ui, "Collection directory", "Reviewable files", |ui| {
                                ui.label(
                                    egui::RichText::new(
                                        "Export this collection into a folder or repository as \
                                         compact .rr files. The folder path is also used for Git \
                                         pull/push actions below.",
                                    )
                                    .size(10.5)
                                    .color(muted()),
                                );
                                ui.add_space(8.0);
                                field_row(ui, "Directory", |ui| {
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut sync.git_workspace_dir)
                                            .hint_text(hint("/path/to/collection-repo"))
                                            .desired_width(ui.available_width() - 106.0),
                                    );
                                    changed |= response.changed();
                                    if ui.button("Choose...").clicked() {
                                        choose_git_dir = true;
                                    }
                                });
                                if git_ready && !is_git_repo {
                                    ui.add_space(6.0);
                                    status_note(
                                        ui,
                                        "Remote pull/push needs the repository root containing .git.",
                                        C_ORANGE,
                                    );
                                }
                                ui.add_space(10.0);
                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .add_enabled(
                                            git_ready && !busy,
                                            egui::Button::new("Import from folder"),
                                        )
                                        .clicked()
                                    {
                                        import_workspace = true;
                                    }
                                    if ui
                                        .add_enabled(
                                            git_ready && !busy,
                                            egui::Button::new("Export to folder"),
                                        )
                                        .clicked()
                                    {
                                        export_workspace = true;
                                    }
                                });
                            });

                            ui.add_space(10.0);
                            setting_card(ui, "Secrets and review rules", "Export safety", |ui| {
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
                                        "Keep this off for shared repos. A private repo is access \
                                         control, not encryption.",
                                    )
                                    .size(10.5)
                                    .color(muted()),
                                );
                                ui.add_space(10.0);
                                ui.label(
                                    egui::RichText::new("Mask rules")
                                        .size(11.0)
                                        .strong()
                                        .color(text()),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        "Comma-separated key/header/env-name patterns. Keep readable \
                                         wins over built-in and custom mask rules.",
                                    )
                                    .size(10.5)
                                    .color(muted()),
                                );
                                ui.add_space(8.0);
                                field_row(ui, "Always mask", |ui| {
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut sync.mask_key_patterns)
                                            .hint_text(hint("x-api-key, token, authorization"))
                                            .desired_width(ui.available_width()),
                                    );
                                    changed |= response.changed();
                                });
                                ui.add_space(6.0);
                                field_row(ui, "Keep readable", |ui| {
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut sync.allow_key_patterns)
                                            .hint_text(hint("platform, env, app-version"))
                                            .desired_width(ui.available_width()),
                                    );
                                    changed |= response.changed();
                                });
                            });

                            ui.add_space(10.0);
                            setting_card(ui, "Git remote", "Pull and push", |ui| {
                                ui.label(
                                    egui::RichText::new(
                                        "Uses your local Git credentials. Private GitHub, GitLab, and \
                                         Bitbucket repositories work when the repository is already \
                                         authenticated on this machine.",
                                    )
                                    .size(10.5)
                                    .color(muted()),
                                );
                                ui.add_space(8.0);
                                field_row(ui, "Commit", |ui| {
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut sync.git_commit_message)
                                            .hint_text(hint("Sync Rusty Requester collection"))
                                            .desired_width(ui.available_width()),
                                    );
                                    changed |= response.changed();
                                });
                                ui.add_space(10.0);
                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .add_enabled(
                                            is_git_repo && !busy,
                                            egui::Button::new("Refresh changes"),
                                        )
                                        .clicked()
                                    {
                                        refresh_git_status = true;
                                    }
                                    if ui
                                        .add_enabled(
                                            is_git_repo && !busy,
                                            egui::Button::new("Pull from remote"),
                                        )
                                        .clicked()
                                    {
                                        pull_remote = true;
                                    }
                                    if ui
                                        .add_enabled(
                                            is_git_repo && !busy,
                                            egui::Button::new("Commit and push"),
                                        )
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
                                            egui::ScrollArea::vertical()
                                                .max_height(110.0)
                                                .show(ui, |ui| {
                                                    ui.monospace(&self.collection_git_status);
                                                });
                                        });
                                }
                            });

                            ui.add_space(10.0);
                            setting_card(ui, "OpenAPI refresh", "Regenerate requests", |ui| {
                                field_row(ui, "Spec file", |ui| {
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut sync.openapi_spec_path)
                                            .hint_text(hint("/path/to/openapi.yaml"))
                                            .desired_width(ui.available_width() - 106.0),
                                    );
                                    changed |= response.changed();
                                    if ui.button("Choose...").clicked() {
                                        choose_openapi_file = true;
                                    }
                                });
                                ui.add_space(10.0);
                                if ui
                                    .add_enabled(
                                        openapi_ready && !busy,
                                        egui::Button::new("Refresh collection from OpenAPI"),
                                    )
                                    .clicked()
                                {
                                    refresh_openapi = true;
                                }
                            });

                            if let Some(sync) = &self.sync_in_flight {
                                ui.add_space(12.0);
                                egui::Frame::none()
                                    .fill(elevated())
                                    .stroke(egui::Stroke::new(1.0, border()))
                                    .rounding(egui::Rounding::same(6.0))
                                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spinner();
                                            ui.label(
                                                egui::RichText::new(&sync.label)
                                                    .size(11.0)
                                                    .color(muted()),
                                            );
                                        });
                                    });
                            }
                        });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
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

fn collection_settings_nav(
    ui: &mut egui::Ui,
    collection_name: &str,
    git_ready: bool,
    is_git_repo: bool,
    openapi_ready: bool,
) {
    ui.vertical(|ui| {
        ui.set_width(190.0);
        ui.label(
            egui::RichText::new(collection_name)
                .size(18.0)
                .strong()
                .color(text()),
        );
        ui.label(
            egui::RichText::new("Collection settings")
                .size(10.5)
                .color(muted()),
        );
        ui.add_space(14.0);

        nav_status(ui, "Directory", git_ready, "Folder selected", "Not linked");
        nav_status(
            ui,
            "Secrets",
            true,
            "Masked by default",
            "Masked by default",
        );
        nav_status(
            ui,
            "Git remote",
            is_git_repo,
            "Git repo ready",
            "Needs .git folder",
        );
        nav_status(
            ui,
            "OpenAPI",
            openapi_ready,
            "Spec selected",
            "No spec selected",
        );

        ui.add_space(14.0);
        egui::Frame::none()
            .fill(with_alpha(accent(), 18))
            .stroke(egui::Stroke::new(1.0, with_alpha(accent(), 70)))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Git providers are not stored here.")
                        .size(10.5)
                        .strong()
                        .color(text()),
                );
                ui.label(
                    egui::RichText::new("Remote access uses your local Git credentials.")
                        .size(10.0)
                        .color(muted()),
                );
            });
    });
}

fn nav_status(ui: &mut egui::Ui, title: &str, ready: bool, ready_text: &str, empty_text: &str) {
    let color = if ready { C_GREEN } else { muted() };
    ui.add_space(2.0);
    ui.label(egui::RichText::new(title).size(11.5).strong().color(text()));
    ui.label(
        egui::RichText::new(if ready { ready_text } else { empty_text })
            .size(10.0)
            .color(color),
    );
    ui.add_space(8.0);
}

fn setting_card(
    ui: &mut egui::Ui,
    title: &str,
    eyebrow: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::none()
        .fill(panel_dark())
        .stroke(egui::Stroke::new(1.0, border()))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(title).size(14.0).strong().color(text()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(eyebrow)
                            .size(10.0)
                            .strong()
                            .color(muted()),
                    );
                });
            });
            ui.add_space(8.0);
            add_contents(ui);
        });
}

fn field_row(ui: &mut egui::Ui, label: &str, add_field: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_height(30.0);
        ui.add_sized(
            egui::vec2(96.0, 24.0),
            egui::Label::new(egui::RichText::new(label).size(11.0).color(muted())),
        );
        add_field(ui);
    });
}

fn status_note(ui: &mut egui::Ui, text_value: &str, color: egui::Color32) {
    ui.label(egui::RichText::new(text_value).size(10.5).color(color));
}
