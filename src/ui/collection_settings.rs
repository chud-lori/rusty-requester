use crate::theme::*;
use crate::{find_folder_by_id, find_folder_by_id_mut, ApiClient};
use eframe::egui;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum CollectionSettingsSection {
    #[default]
    Directory,
    Secrets,
    Git,
    OpenApi,
}

#[derive(Default)]
struct CollectionSettingsActions {
    choose_git_dir: bool,
    choose_openapi_file: bool,
    import_workspace: bool,
    export_workspace: bool,
    pull_remote: bool,
    push_remote: bool,
    refresh_git_status: bool,
    refresh_openapi: bool,
}

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
        let mut actions = CollectionSettingsActions::default();

        egui::Window::new("Collection settings")
            .title_bar(false)
            .collapsible(false)
            .resizable(true)
            .default_size(egui::vec2(900.0, 620.0))
            .min_width(660.0)
            .min_height(500.0)
            .max_width(1040.0)
            .max_height(760.0)
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

                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(18.0, 14.0))
                    .show(ui, |ui| {
                        render_header(ui, &folder.name, &mut self.show_collection_settings_modal);
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);

                        let footer_height = 46.0;
                        let content_height = (ui.available_height() - footer_height).max(320.0);
                        let content_width = ui.available_width().max(1.0);
                        let (content_rect, _) = ui.allocate_exact_size(
                            egui::vec2(content_width, content_height),
                            egui::Sense::hover(),
                        );
                        let nav_width = 210.0_f32.min((content_rect.width() * 0.34).max(180.0));
                        let gap = 16.0;
                        let nav_rect = egui::Rect::from_min_max(
                            content_rect.min,
                            egui::pos2(content_rect.left() + nav_width, content_rect.bottom()),
                        );
                        let detail_rect = egui::Rect::from_min_max(
                            egui::pos2(content_rect.left() + nav_width + gap, content_rect.top()),
                            content_rect.max,
                        );
                        let divider_x = content_rect.left() + nav_width + gap * 0.5;
                        ui.painter().line_segment(
                            [
                                egui::pos2(divider_x, content_rect.top()),
                                egui::pos2(divider_x, content_rect.bottom()),
                            ],
                            egui::Stroke::new(1.0, border()),
                        );

                        let mut nav_ui = ui.new_child(egui::UiBuilder::new().max_rect(nav_rect));
                        nav_ui.set_clip_rect(nav_rect);
                        render_section_picker(
                            &mut nav_ui,
                            &mut self.collection_settings_section,
                            git_ready,
                            is_git_repo,
                            openapi_ready,
                        );

                        let mut detail_ui =
                            ui.new_child(egui::UiBuilder::new().max_rect(detail_rect));
                        detail_ui.set_clip_rect(detail_rect);
                        egui::ScrollArea::vertical()
                            .id_salt("collection_settings_detail")
                            .auto_shrink([false, false])
                            .max_height(content_height)
                            .show(&mut detail_ui, |ui| {
                                ui.set_width(ui.available_width().max(360.0));
                                match self.collection_settings_section {
                                    CollectionSettingsSection::Directory => {
                                        directory_section(
                                            ui,
                                            &mut sync.git_workspace_dir,
                                            git_ready,
                                            is_git_repo,
                                            busy,
                                            &mut changed,
                                            &mut actions,
                                        );
                                    }
                                    CollectionSettingsSection::Secrets => {
                                        secrets_section(ui, &mut sync, &mut changed);
                                    }
                                    CollectionSettingsSection::Git => {
                                        git_section(
                                            ui,
                                            &mut sync.git_commit_message,
                                            is_git_repo,
                                            busy,
                                            &self.collection_git_status,
                                            &mut changed,
                                            &mut actions,
                                        );
                                    }
                                    CollectionSettingsSection::OpenApi => {
                                        openapi_section(
                                            ui,
                                            &mut sync.openapi_spec_path,
                                            openapi_ready,
                                            busy,
                                            &mut changed,
                                            &mut actions,
                                        );
                                    }
                                }

                                if let Some(sync) = &self.sync_in_flight {
                                    ui.add_space(12.0);
                                    sync_status(ui, &sync.label);
                                }
                            });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                self.show_collection_settings_modal = false;
                            }
                        });
                    });
            });

        self.show_collection_settings_modal = open && self.show_collection_settings_modal;

        if changed {
            if let Some(folder) = find_folder_by_id_mut(&mut self.state.folders, &folder_id) {
                folder.sync = sync;
                self.save_state();
            }
        }
        if actions.choose_git_dir {
            self.choose_collection_git_workspace_dir(&folder_id);
        }
        if actions.choose_openapi_file {
            self.choose_collection_openapi_spec_file(&folder_id);
        }
        if actions.import_workspace {
            self.import_collection_workspace_from_config(&folder_id);
        }
        if actions.export_workspace {
            self.export_collection_workspace_from_config(&folder_id);
        }
        if actions.pull_remote {
            self.pull_collection_workspace_from_config(&folder_id);
        }
        if actions.push_remote {
            self.push_collection_workspace_from_config(&folder_id);
        }
        if actions.refresh_git_status {
            self.refresh_collection_git_status_from_config(&folder_id);
        }
        if actions.refresh_openapi {
            self.refresh_collection_openapi_from_config(&folder_id);
        }
    }
}

fn render_header(ui: &mut egui::Ui, collection_name: &str, modal_open: &mut bool) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Collection settings")
                    .size(18.0)
                    .strong()
                    .color(text()),
            );
            ui.label(
                egui::RichText::new(collection_name)
                    .size(11.0)
                    .color(muted()),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(egui_phosphor::regular::X)
                            .size(18.0)
                            .color(muted()),
                    )
                    .frame(false),
                )
                .on_hover_text("Close")
                .clicked()
            {
                *modal_open = false;
            }
        });
    });
}

fn render_section_picker(
    ui: &mut egui::Ui,
    selected: &mut CollectionSettingsSection,
    git_ready: bool,
    is_git_repo: bool,
    openapi_ready: bool,
) {
    ui.vertical(|ui| {
        ui.set_width(200.0);
        section_button(
            ui,
            selected,
            CollectionSettingsSection::Directory,
            "Directory",
            if git_ready {
                "Folder selected"
            } else {
                "Not linked"
            },
            git_ready,
        );
        section_button(
            ui,
            selected,
            CollectionSettingsSection::Secrets,
            "Secrets",
            "Masked by default",
            true,
        );
        section_button(
            ui,
            selected,
            CollectionSettingsSection::Git,
            "Git remote",
            if is_git_repo {
                "Repository ready"
            } else {
                "Needs .git folder"
            },
            is_git_repo,
        );
        section_button(
            ui,
            selected,
            CollectionSettingsSection::OpenApi,
            "OpenAPI",
            if openapi_ready {
                "Spec selected"
            } else {
                "No spec selected"
            },
            openapi_ready,
        );

        ui.add_space(12.0);
        info_note(
            ui,
            "Git providers are not stored here. Private remotes use your local Git credentials.",
        );
    });
}

fn section_button(
    ui: &mut egui::Ui,
    selected: &mut CollectionSettingsSection,
    section: CollectionSettingsSection,
    title: &str,
    status: &str,
    ready: bool,
) {
    let active = *selected == section;
    let desired = egui::vec2(ui.available_width(), 54.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    if response.clicked() {
        *selected = section;
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand);

    let fill = if active { elevated() } else { panel_dark() };
    let stroke = if active {
        egui::Stroke::new(1.0, accent())
    } else {
        egui::Stroke::new(1.0, border())
    };
    ui.painter()
        .rect(rect, egui::Rounding::same(7.0), fill, stroke);
    if active {
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 1.0, rect.top() + 8.0),
                egui::pos2(rect.left() + 1.0, rect.bottom() - 8.0),
            ],
            egui::Stroke::new(3.0, accent()),
        );
    }
    let status_color = if ready { C_GREEN } else { muted() };
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, rect.top() + 11.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(12.0),
        text(),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, rect.top() + 31.0),
        egui::Align2::LEFT_TOP,
        status,
        egui::FontId::proportional(10.5),
        status_color,
    );
    ui.add_space(8.0);
}

fn directory_section(
    ui: &mut egui::Ui,
    git_workspace_dir: &mut String,
    git_ready: bool,
    is_git_repo: bool,
    busy: bool,
    changed: &mut bool,
    actions: &mut CollectionSettingsActions,
) {
    section_header(
        ui,
        "Collection directory",
        "Export this collection as reviewable .rr files. Use a Git repository root if you want pull and push actions.",
    );
    ui.add_space(12.0);
    field_row(ui, "Directory", |ui| {
        let response = ui.add(
            egui::TextEdit::singleline(git_workspace_dir)
                .hint_text(hint("/path/to/collection-repo"))
                .desired_width((ui.available_width() - 112.0).max(220.0)),
        );
        *changed |= response.changed();
        if ui.button("Choose...").clicked() {
            actions.choose_git_dir = true;
        }
    });
    if git_ready && !is_git_repo {
        ui.add_space(8.0);
        status_note(
            ui,
            "Remote pull/push needs the repository root containing .git.",
            C_ORANGE,
        );
    }
    ui.add_space(14.0);
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(git_ready && !busy, egui::Button::new("Import from folder"))
            .clicked()
        {
            actions.import_workspace = true;
        }
        if ui
            .add_enabled(git_ready && !busy, egui::Button::new("Export to folder"))
            .clicked()
        {
            actions.export_workspace = true;
        }
    });
}

fn secrets_section(ui: &mut egui::Ui, sync: &mut crate::model::SyncConfig, changed: &mut bool) {
    section_header(
        ui,
        "Secrets and review rules",
        "Control which values stay readable in exported .rr files and which values are masked before sharing.",
    );
    ui.add_space(12.0);
    if ui
        .checkbox(
            &mut sync.include_secrets_in_git_workspace,
            "Include secrets in collection exports",
        )
        .changed()
    {
        *changed = true;
    }
    ui.label(
        egui::RichText::new(
            "Keep this off for shared repos. A private repo is access control, not encryption.",
        )
        .size(10.5)
        .color(muted()),
    );
    ui.add_space(14.0);
    field_row(ui, "Always mask", |ui| {
        let response = ui.add(
            egui::TextEdit::singleline(&mut sync.mask_key_patterns)
                .hint_text(hint("x-api-key, token, authorization"))
                .desired_width(ui.available_width()),
        );
        *changed |= response.changed();
    });
    ui.add_space(8.0);
    field_row(ui, "Keep readable", |ui| {
        let response = ui.add(
            egui::TextEdit::singleline(&mut sync.allow_key_patterns)
                .hint_text(hint("platform, env, app-version"))
                .desired_width(ui.available_width()),
        );
        *changed |= response.changed();
    });
}

fn git_section(
    ui: &mut egui::Ui,
    git_commit_message: &mut String,
    is_git_repo: bool,
    busy: bool,
    git_status: &str,
    changed: &mut bool,
    actions: &mut CollectionSettingsActions,
) {
    section_header(
        ui,
        "Git remote",
        "Pull, review, commit, and push through your existing local Git credentials. Rusty Requester does not store provider tokens.",
    );
    ui.add_space(12.0);
    field_row(ui, "Commit", |ui| {
        let response = ui.add(
            egui::TextEdit::singleline(git_commit_message)
                .hint_text(hint("Sync Rusty Requester collection"))
                .desired_width(ui.available_width()),
        );
        *changed |= response.changed();
    });
    ui.add_space(14.0);
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(is_git_repo && !busy, egui::Button::new("Refresh changes"))
            .clicked()
        {
            actions.refresh_git_status = true;
        }
        if ui
            .add_enabled(is_git_repo && !busy, egui::Button::new("Pull from remote"))
            .clicked()
        {
            actions.pull_remote = true;
        }
        if ui
            .add_enabled(is_git_repo && !busy, egui::Button::new("Commit and push"))
            .clicked()
        {
            actions.push_remote = true;
        }
    });
    if !is_git_repo {
        ui.add_space(10.0);
        status_note(
            ui,
            "Choose a collection directory that contains a .git folder to enable remote actions.",
            C_ORANGE,
        );
    }
    if !git_status.is_empty() {
        ui.add_space(12.0);
        egui::Frame::none()
            .fill(elevated())
            .stroke(egui::Stroke::new(1.0, border()))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        ui.monospace(git_status);
                    });
            });
    }
}

fn openapi_section(
    ui: &mut egui::Ui,
    openapi_spec_path: &mut String,
    openapi_ready: bool,
    busy: bool,
    changed: &mut bool,
    actions: &mut CollectionSettingsActions,
) {
    section_header(
        ui,
        "OpenAPI refresh",
        "Regenerate requests from a local OpenAPI JSON or YAML file without replacing your whole app workspace.",
    );
    ui.add_space(12.0);
    field_row(ui, "Spec file", |ui| {
        let response = ui.add(
            egui::TextEdit::singleline(openapi_spec_path)
                .hint_text(hint("/path/to/openapi.yaml"))
                .desired_width((ui.available_width() - 112.0).max(220.0)),
        );
        *changed |= response.changed();
        if ui.button("Choose...").clicked() {
            actions.choose_openapi_file = true;
        }
    });
    ui.add_space(14.0);
    if ui
        .add_enabled(
            openapi_ready && !busy,
            egui::Button::new("Refresh collection from OpenAPI"),
        )
        .clicked()
    {
        actions.refresh_openapi = true;
    }
}

fn section_header(ui: &mut egui::Ui, title: &str, description: &str) {
    ui.label(egui::RichText::new(title).size(16.0).strong().color(text()));
    ui.add_space(4.0);
    ui.label(egui::RichText::new(description).size(11.0).color(muted()));
}

fn field_row(ui: &mut egui::Ui, label: &str, add_field: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_height(30.0);
        ui.add_sized(
            egui::vec2(92.0, 24.0),
            egui::Label::new(egui::RichText::new(label).size(11.0).color(muted())),
        );
        add_field(ui);
    });
}

fn sync_status(ui: &mut egui::Ui, label: &str) {
    egui::Frame::none()
        .fill(elevated())
        .stroke(egui::Stroke::new(1.0, border()))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(egui::RichText::new(label).size(11.0).color(muted()));
            });
        });
}

fn info_note(ui: &mut egui::Ui, text_value: &str) {
    egui::Frame::none()
        .fill(with_alpha(accent(), 16))
        .stroke(egui::Stroke::new(1.0, with_alpha(accent(), 60)))
        .rounding(egui::Rounding::same(7.0))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text_value).size(10.5).color(text()));
        });
}

fn status_note(ui: &mut egui::Ui, text_value: &str, color: egui::Color32) {
    ui.label(egui::RichText::new(text_value).size(10.5).color(color));
}
