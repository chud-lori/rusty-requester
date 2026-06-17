use crate::theme::*;
use crate::ApiClient;
use eframe::egui;

impl ApiClient {
    pub(crate) fn render_sync_modal(&mut self, ctx: &egui::Context) {
        if !self.show_sync_modal {
            return;
        }

        let mut open = self.show_sync_modal;
        let mut choose_git_dir = false;
        let mut choose_openapi_file = false;
        let mut pull_git_workspace = false;
        let mut push_git_workspace = false;
        let mut pull_github_workspace = false;
        let mut push_github_workspace = false;
        let mut refresh_openapi = false;
        let mut sync_config_changed = false;

        egui::Window::new("Workspace Sync")
            .collapsible(false)
            .resizable(false)
            .default_width(620.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(580.0);

                if !self.state.settings.workspace_sync_enabled {
                    ui.label(
                        egui::RichText::new("Workspace Sync is disabled")
                            .size(15.0)
                            .strong()
                            .color(text()),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Enable it in Settings before reading or writing sync files.",
                        )
                        .size(11.0)
                        .color(muted()),
                    );
                    return;
                }

                ui.label(
                    egui::RichText::new("Git workspace")
                        .size(12.0)
                        .strong()
                        .color(text()),
                );
                ui.label(
                    egui::RichText::new(
                        "Manual pull/push for the deterministic workspace directory. \
                         Exports are masked by default for safer commits.",
                    )
                    .size(10.5)
                    .color(muted()),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Directory").size(11.0).color(muted()));
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.state.sync.git_workspace_dir)
                            .hint_text(hint("/path/to/api-workspace"))
                            .desired_width(ui.available_width() - 92.0),
                    );
                    sync_config_changed |= resp.changed();
                    if ui.button("Choose…").clicked() {
                        choose_git_dir = true;
                    }
                });
                let git_ready = !self.state.sync.git_workspace_dir.trim().is_empty();
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(
                            &mut self.state.sync.include_secrets_in_git_workspace,
                            "Include secrets in Git workspace export",
                        )
                        .changed()
                    {
                        sync_config_changed = true;
                    }
                });
                ui.label(
                    egui::RichText::new(
                        "Keep unchecked for shared repos. Including secrets is for private/local-only sync.",
                    )
                    .size(10.5)
                    .color(muted()),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let sync_busy = self.sync_in_flight.is_some();
                    if ui
                        .add_enabled(
                            git_ready && !sync_busy,
                            egui::Button::new("Pull from Git workspace"),
                        )
                        .clicked()
                    {
                        pull_git_workspace = true;
                    }
                    if ui
                        .add_enabled(
                            git_ready && !sync_busy,
                            egui::Button::new("Push Git workspace"),
                        )
                        .clicked()
                    {
                        push_git_workspace = true;
                    }
                });
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("GitHub / Git remote")
                        .size(12.0)
                        .strong()
                        .color(text()),
                );
                ui.label(
                    egui::RichText::new(
                        "Uses your local Git setup in this repository. Private repos work through \
                         your existing SSH key or Git credential helper; Rusty Requester stores no GitHub token.",
                    )
                    .size(10.5)
                    .color(muted()),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Commit").size(11.0).color(muted()));
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.state.sync.git_commit_message)
                            .hint_text(hint("Sync Rusty Requester workspace"))
                            .desired_width(ui.available_width()),
                    );
                    sync_config_changed |= resp.changed();
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let sync_busy = self.sync_in_flight.is_some();
                    if ui
                        .add_enabled(
                            git_ready && !sync_busy,
                            egui::Button::new("Pull from GitHub"),
                        )
                        .clicked()
                    {
                        pull_github_workspace = true;
                    }
                    if ui
                        .add_enabled(git_ready && !sync_busy, egui::Button::new("Push to GitHub"))
                        .clicked()
                    {
                        push_github_workspace = true;
                    }
                });

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

                ui.label(
                    egui::RichText::new("OpenAPI refresh")
                        .size(12.0)
                        .strong()
                        .color(text()),
                );
                ui.label(
                    egui::RichText::new(
                        "Refresh generated OpenAPI requests from a local JSON/YAML spec while \
                         preserving saved auth, tests, extractors, and user rows where possible.",
                    )
                    .size(10.5)
                    .color(muted()),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Spec file").size(11.0).color(muted()));
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.state.sync.openapi_spec_path)
                            .hint_text(hint("/path/to/openapi.yaml"))
                            .desired_width(ui.available_width() - 92.0),
                    );
                    sync_config_changed |= resp.changed();
                    if ui.button("Choose…").clicked() {
                        choose_openapi_file = true;
                    }
                });
                let spec_ready = !self.state.sync.openapi_spec_path.trim().is_empty();
                ui.add_space(6.0);
                let sync_busy = self.sync_in_flight.is_some();
                if ui
                    .add_enabled(
                        spec_ready && !sync_busy,
                        egui::Button::new("Refresh from OpenAPI spec"),
                    )
                    .clicked()
                {
                    refresh_openapi = true;
                }

                ui.add_space(14.0);
                if let Some(sync) = &self.sync_in_flight {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(
                            egui::RichText::new(&sync.label)
                                .size(11.0)
                                .color(muted()),
                        );
                    });
                    ui.add_space(8.0);
                }
                ui.separator();
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        self.show_sync_modal = false;
                    }
                });
            });

        self.show_sync_modal = open && self.show_sync_modal;
        if sync_config_changed {
            self.save_state();
        }
        if choose_git_dir {
            self.choose_git_workspace_dir();
        }
        if choose_openapi_file {
            self.choose_openapi_spec_file();
        }
        if pull_git_workspace {
            self.import_git_workspace_from_config();
        }
        if push_git_workspace {
            self.export_git_workspace_from_config();
        }
        if pull_github_workspace {
            self.pull_github_workspace_from_config();
        }
        if push_github_workspace {
            self.push_github_workspace_from_config();
        }
        if refresh_openapi {
            self.refresh_openapi_from_config();
        }
    }
}
