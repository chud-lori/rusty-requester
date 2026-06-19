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
        let mut collection_name = folder.name.clone();
        let mut changed = false;
        let mut name_changed = false;
        let mut actions = CollectionSettingsActions::default();

        egui::Window::new("Collection settings")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .fixed_size(egui::vec2(820.0, 520.0))
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
                        render_header(
                            ui,
                            &mut collection_name,
                            &mut name_changed,
                            &mut self.show_collection_settings_modal,
                        );
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);

                        let content_height = (ui.available_height() - 44.0).max(320.0);
                        ui.horizontal_top(|ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(172.0, content_height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_width(172.0);
                                    ui.set_min_height(content_height);
                                    ui.set_clip_rect(ui.max_rect());
                                    render_section_picker(
                                        ui,
                                        &mut self.collection_settings_section,
                                        git_ready,
                                        is_git_repo,
                                        openapi_ready,
                                    );
                                },
                            );

                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);

                            let detail_width = ui.available_width().min(600.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(detail_width, content_height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_width(detail_width);
                                    ui.set_min_height(content_height);
                                    egui::ScrollArea::vertical()
                                        .id_salt("collection_settings_detail")
                                        .auto_shrink([false, false])
                                        .max_height(content_height - 4.0)
                                        .show(ui, |ui| {
                                            ui.set_width(detail_width - 14.0);
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
                                },
                            );
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

        if changed || name_changed {
            if let Some(folder) = find_folder_by_id_mut(&mut self.state.folders, &folder_id) {
                if changed {
                    folder.sync = sync;
                }
                if name_changed {
                    let trimmed = collection_name.trim();
                    if !trimmed.is_empty() {
                        folder.name = trimmed.to_string();
                    }
                }
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

fn render_header(
    ui: &mut egui::Ui,
    collection_name: &mut String,
    name_changed: &mut bool,
    modal_open: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Collection settings")
                    .size(18.0)
                    .strong()
                    .color(text()),
            );
            ui.label(egui::RichText::new("Name").size(11.0).color(muted()));
            ui.add_space(4.0);
            let response = framed_text_field(
                ui,
                ui.available_width().min(420.0),
                collection_name,
                "Collection name",
            );
            *name_changed |= response.changed();
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
        ui.set_width(172.0);
        ui.label(egui::RichText::new("Sections").size(10.5).color(muted()));
        ui.add_space(6.0);
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
    let desired = egui::vec2((ui.available_width() - 6.0).max(120.0), 42.0);
    let (allocated_rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let rect = allocated_rect.shrink2(egui::vec2(2.0, 1.0));
    if response.clicked() {
        *selected = section;
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand);

    let fill = if active {
        with_alpha(accent(), 22)
    } else {
        egui::Color32::TRANSPARENT
    };
    let stroke = if active {
        egui::Stroke::new(1.0, with_alpha(accent(), 110))
    } else {
        egui::Stroke::new(1.0, egui::Color32::TRANSPARENT)
    };
    ui.painter()
        .rect(rect, egui::Rounding::same(6.0), fill, stroke);
    if active {
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 1.0, rect.top() + 7.0),
                egui::pos2(rect.left() + 1.0, rect.bottom() - 7.0),
            ],
            egui::Stroke::new(3.0, accent()),
        );
    }
    let status_color = if ready { C_GREEN } else { muted() };
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.top() + 7.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(11.5),
        text(),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.top() + 25.0),
        egui::Align2::LEFT_TOP,
        status,
        egui::FontId::proportional(10.0),
        status_color,
    );
    ui.add_space(4.0);
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
    path_field_group(
        ui,
        "Directory",
        git_workspace_dir,
        "/path/to/collection-repo",
        changed,
        || actions.choose_git_dir = true,
    );
    if git_ready && !is_git_repo {
        ui.add_space(8.0);
        status_note(
            ui,
            "Remote pull/push needs the repository root containing .git.",
            C_ORANGE,
        );
    } else if git_ready {
        ui.add_space(8.0);
        status_note(
            ui,
            "Directory linked. Click Export now to write reviewable files.",
            C_GREEN,
        );
    }
    ui.add_space(12.0);
    action_buttons(ui, |ui| {
        if fixed_button(ui, git_ready && !busy, "Import from folder", 142.0).clicked() {
            actions.import_workspace = true;
        }
        if fixed_button(ui, git_ready && !busy, "Export now", 112.0).clicked() {
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
    text_field_group(
        ui,
        "Always mask",
        &mut sync.mask_key_patterns,
        "x-api-key, token, authorization",
        changed,
    );
    ui.add_space(8.0);
    text_field_group(
        ui,
        "Keep readable",
        &mut sync.allow_key_patterns,
        "platform, env, app-version",
        changed,
    );
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
    ui.add_space(8.0);
    info_note(
        ui,
        "Private remotes use your local Git credentials. No provider tokens are stored.",
    );
    ui.add_space(12.0);
    text_field_group(
        ui,
        "Commit",
        git_commit_message,
        "Sync Rusty Requester collection",
        changed,
    );
    ui.add_space(14.0);
    action_buttons(ui, |ui| {
        if fixed_button(ui, is_git_repo && !busy, "Refresh changes", 130.0).clicked() {
            actions.refresh_git_status = true;
        }
        if fixed_button(ui, is_git_repo && !busy, "Pull from remote", 130.0).clicked() {
            actions.pull_remote = true;
        }
        if fixed_button(ui, is_git_repo && !busy, "Commit and push", 130.0).clicked() {
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
    path_field_group(
        ui,
        "Spec file",
        openapi_spec_path,
        "/path/to/openapi.yaml",
        changed,
        || actions.choose_openapi_file = true,
    );
    ui.add_space(14.0);
    action_buttons(ui, |ui| {
        if fixed_button(
            ui,
            openapi_ready && !busy,
            "Refresh collection from OpenAPI",
            230.0,
        )
        .clicked()
        {
            actions.refresh_openapi = true;
        }
    });
}

fn section_header(ui: &mut egui::Ui, title: &str, description: &str) {
    ui.label(egui::RichText::new(title).size(15.0).strong().color(text()));
    ui.add_space(4.0);
    ui.add(egui::Label::new(egui::RichText::new(description).size(11.0).color(muted())).wrap());
}

fn path_field_group(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    placeholder: &str,
    changed: &mut bool,
    mut choose: impl FnMut(),
) {
    ui.label(egui::RichText::new(label).size(11.0).color(muted()));
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        let button_w = 96.0;
        let input_w = (ui.available_width() - button_w - 10.0).max(220.0);
        let response = framed_text_field(ui, input_w, value, placeholder);
        *changed |= response.changed();
        if ui
            .add_sized([button_w, 34.0], egui::Button::new("Choose..."))
            .clicked()
        {
            choose();
        }
    });
}

fn text_field_group(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    placeholder: &str,
    changed: &mut bool,
) {
    ui.label(egui::RichText::new(label).size(11.0).color(muted()));
    ui.add_space(5.0);
    let response = framed_text_field(ui, ui.available_width(), value, placeholder);
    *changed |= response.changed();
}

fn framed_text_field(
    ui: &mut egui::Ui,
    width: f32,
    value: &mut String,
    placeholder: &str,
) -> egui::Response {
    let frame_width = width.max(120.0);
    egui::Frame::none()
        .fill(elevated())
        .stroke(egui::Stroke::new(1.0, border()))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::symmetric(10.0, 5.0))
        .show(ui, |ui| {
            ui.set_width((frame_width - 20.0).max(80.0));
            ui.add_sized(
                [ui.available_width(), 22.0],
                egui::TextEdit::singleline(value)
                    .hint_text(hint(placeholder))
                    .frame(false)
                    .desired_width(f32::INFINITY),
            )
        })
        .inner
}

fn action_buttons(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.set_min_height(32.0);
        add_contents(ui);
    });
}

fn fixed_button(ui: &mut egui::Ui, enabled: bool, label: &str, width: f32) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        ui.add_sized([width, 30.0], egui::Button::new(label))
    })
    .inner
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
        .fill(elevated())
        .stroke(egui::Stroke::new(1.0, border()))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(9.0, 7.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text_value).size(10.0).color(muted()));
        });
}

fn status_note(ui: &mut egui::Ui, text_value: &str, color: egui::Color32) {
    ui.label(egui::RichText::new(text_value).size(10.5).color(color));
}
