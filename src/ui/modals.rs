//! Modal dialogs + floating UI: Environments manager, Save-draft
//! folder picker (with new-folder inline creator), cURL paste dialog,
//! right-side code snippet panel, toast notifications. Plus a couple
//! of state helpers tightly coupled to the save-draft modal
//! (folder-path lookup, subtree search) and `new_draft_request`.

use crate::io::curl;
use crate::model::*;
use crate::snippet::{build_snippet_layout_job_content_only, render_snippet, SnippetLang};
use crate::theme::*;
use crate::widgets::*;
use crate::ApiClient;
use eframe::egui;
use uuid::Uuid;

impl ApiClient {
    pub(crate) fn render_env_modal(&mut self, ctx: &egui::Context) {
        if !self.show_env_modal {
            return;
        }
        let mut open = self.show_env_modal;
        let mut close_modal = false;
        let mut create_env = false;
        let mut delete_id: Option<String> = None;

        egui::Window::new("Environments")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .default_height(420.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left column: env list
                    ui.vertical(|ui| {
                        ui.set_min_width(160.0);
                        ui.set_max_width(160.0);
                        ui.label(
                            egui::RichText::new("Environments")
                                .size(11.0)
                                .strong()
                                .color(C_MUTED),
                        );
                        ui.add_space(4.0);
                        let envs = self.state.environments.clone();
                        for env in &envs {
                            let selected =
                                self.selected_env_for_edit.as_deref() == Some(env.id.as_str());
                            if ui
                                .selectable_label(selected, &env.name)
                                .clicked()
                            {
                                self.selected_env_for_edit = Some(env.id.clone());
                            }
                        }
                        ui.add_space(6.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("+ New environment")
                                        .size(11.0)
                                        .color(C_ACCENT),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::new(1.0, C_BORDER)),
                            )
                            .clicked()
                        {
                            create_env = true;
                        }
                    });

                    ui.separator();

                    // Right column: editor for selected env
                    ui.vertical(|ui| {
                        let id = self.selected_env_for_edit.clone();
                        let env_idx = id.as_ref().and_then(|id| {
                            self.state.environments.iter().position(|e| &e.id == id)
                        });
                        if let Some(idx) = env_idx {
                            let mut name = self.state.environments[idx].name.clone();
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Name")
                                        .size(11.0)
                                        .color(C_MUTED),
                                );
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut name)
                                            .desired_width(ui.available_width() - 80.0),
                                    )
                                    .changed()
                                {
                                    self.state.environments[idx].name = name.clone();
                                    self.save_state();
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Delete").color(C_RED).size(11.0),
                                        )
                                        .fill(egui::Color32::TRANSPARENT)
                                        .stroke(egui::Stroke::new(1.0, C_RED)),
                                    )
                                    .clicked()
                                {
                                    delete_id = Some(self.state.environments[idx].id.clone());
                                }
                            });
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Variables  (use {{name}} in URL/headers/body)")
                                    .size(11.0)
                                    .color(C_MUTED),
                            );
                            ui.add_space(4.0);
                            let mut vars =
                                self.state.environments[idx].variables.clone();
                            let changed = render_kv_table(ui, "Variables", &mut vars, false);
                            if changed {
                                self.state.environments[idx].variables = vars;
                                self.save_state();
                            }
                        } else {
                            ui.label(
                                egui::RichText::new(
                                    "Select an environment on the left, or create a new one.",
                                )
                                .color(C_MUTED),
                            );
                        }
                    });
                });

                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Done").color(egui::Color32::WHITE).strong(),
                            )
                            .fill(C_ACCENT)
                            .min_size(egui::vec2(80.0, 28.0)),
                        )
                        .clicked()
                    {
                        close_modal = true;
                    }
                });
            });
        self.show_env_modal = open;

        if create_env {
            let new = Environment {
                id: Uuid::new_v4().to_string(),
                name: format!("Environment {}", self.state.environments.len() + 1),
                variables: vec![],
            };
            let new_id = new.id.clone();
            self.state.environments.push(new);
            self.selected_env_for_edit = Some(new_id);
            self.save_state();
        }
        if let Some(id) = delete_id {
            self.state.environments.retain(|e| e.id != id);
            if self.state.active_env_id.as_deref() == Some(&id) {
                self.state.active_env_id = None;
            }
            if self.selected_env_for_edit.as_deref() == Some(&id) {
                self.selected_env_for_edit = self.state.environments.first().map(|e| e.id.clone());
            }
            self.save_state();
        }
        if close_modal {
            self.show_env_modal = false;
        }
    }

    pub(crate) fn begin_save_draft(&mut self, idx: usize) {
        let Some(tab) = self.state.open_tabs.get(idx).cloned() else {
            return;
        };
        if !tab.folder_path.is_empty() {
            return; // not actually a draft
        }
        let Some(draft) = self.state.drafts.iter().find(|d| d.id == tab.request_id).cloned()
        else {
            return;
        };
        self.save_draft_open = true;
        self.save_draft_tab_idx = Some(idx);
        self.save_draft_name = if draft.name.is_empty() || draft.name == "Untitled" {
            "New Request".to_string()
        } else {
            draft.name
        };
        // Default to the first top-level collection if any.
        self.save_draft_target_path = self
            .state
            .folders
            .first()
            .map(|f| vec![f.id.clone()])
            .unwrap_or_default();
        self.save_draft_search.clear();
        self.save_draft_new_folder_name = None;
    }

    pub(crate) fn render_save_draft_modal(&mut self, ctx: &egui::Context) {
        if !self.save_draft_open {
            return;
        }
        let mut open = self.save_draft_open;
        let mut do_save = false;
        let mut do_cancel = false;
        let mut create_folder: Option<(Vec<String>, String)> = None;

        egui::Window::new(
            egui::RichText::new("SAVE REQUEST").size(12.0).strong().color(C_MUTED),
        )
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(560.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_min_width(540.0);

                // Name
                ui.label(egui::RichText::new("Request name").size(11.0).color(C_MUTED));
                let name_resp = ui.add(
                    egui::TextEdit::singleline(&mut self.save_draft_name)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(10.0);

                // Breadcrumb
                let breadcrumb = self.save_draft_breadcrumb();
                ui.label(
                    egui::RichText::new(format!("Save to  {}", breadcrumb))
                        .size(12.0)
                        .color(C_TEXT),
                );
                ui.add_space(6.0);

                // Search
                ui.add(
                    egui::TextEdit::singleline(&mut self.save_draft_search)
                        .hint_text("Search for collection or folder")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.0);

                // Folder tree (scrollable)
                egui::Frame::none()
                    .fill(C_PANEL_DARK)
                    .stroke(egui::Stroke::new(1.0, C_BORDER))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(4.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::ScrollArea::vertical()
                            .id_salt("save_draft_tree")
                            .max_height(260.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let folders = self.state.folders.clone();
                                let query = self.save_draft_search.to_lowercase();
                                for f in &folders {
                                    Self::render_save_tree_row(
                                        ui,
                                        f,
                                        &mut vec![],
                                        &mut self.save_draft_target_path,
                                        &query,
                                    );
                                }
                            });
                    });

                ui.add_space(8.0);

                // New folder — either show the "+ New folder" button or,
                // if the user clicked it, an inline input with Create/Cancel.
                let mut cancel_new_folder = false;
                if let Some(name) = self.save_draft_new_folder_name.as_mut() {
                    let target_path_snapshot = self.save_draft_target_path.clone();
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(name)
                                .hint_text("New folder name")
                                .desired_width(260.0),
                        );
                        let enabled = !name.trim().is_empty();
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Button::new(
                                    egui::RichText::new("Create").color(egui::Color32::WHITE),
                                )
                                .fill(if enabled { C_ACCENT } else { C_ELEVATED })
                                .min_size(egui::vec2(72.0, 26.0)),
                            )
                            .clicked()
                        {
                            create_folder =
                                Some((target_path_snapshot.clone(), name.trim().to_string()));
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_new_folder = true;
                        }
                    });
                } else if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("+ New folder").size(12.0).color(C_ACCENT),
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE),
                    )
                    .clicked()
                {
                    self.save_draft_new_folder_name = Some(String::new());
                }
                if cancel_new_folder {
                    self.save_draft_new_folder_name = None;
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                // Save / Cancel
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        do_cancel = true;
                    }
                    let can_save = !self.save_draft_target_path.is_empty()
                        && !self.save_draft_name.trim().is_empty();
                    let save_btn = egui::Button::new(
                        egui::RichText::new("Save")
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .fill(if can_save { C_ACCENT } else { C_ELEVATED })
                    .min_size(egui::vec2(80.0, 28.0));
                    if ui.add_enabled(can_save, save_btn).clicked() {
                        do_save = true;
                    }
                });

                ui.input(|i| {
                    if i.key_pressed(egui::Key::Escape) {
                        do_cancel = true;
                    }
                });

                if !name_resp.has_focus()
                    && self.save_draft_tab_idx.is_some()
                    && !do_save
                    && !do_cancel
                    && self.save_draft_search.is_empty()
                    && self.save_draft_new_folder_name.is_none()
                {
                    name_resp.request_focus();
                }
            });
        self.save_draft_open = open;

        // Create the new folder outside the UI closure to avoid borrow
        // conflicts (we were holding `&self.state.folders` immutably while
        // iterating the tree).
        if let Some((parent_path, folder_name)) = create_folder {
            let new_id = Uuid::new_v4().to_string();
            let new_folder = Folder {
                id: new_id.clone(),
                name: folder_name,
                requests: vec![],
                subfolders: vec![],
            };
            let inserted = if parent_path.is_empty() {
                self.state.folders.push(new_folder);
                true
            } else if let Some(parent) = self.folder_at_path_mut(&parent_path) {
                parent.subfolders.push(new_folder);
                true
            } else {
                false
            };
            if inserted {
                let mut new_path = parent_path.clone();
                new_path.push(new_id);
                self.save_draft_target_path = new_path;
                self.save_draft_new_folder_name = None;
                self.save_state();
            }
        }

        if do_save {
            self.commit_save_draft();
        }
        if do_cancel || !self.save_draft_open {
            self.save_draft_open = false;
            self.save_draft_tab_idx = None;
            self.save_draft_target_path.clear();
            self.save_draft_name.clear();
            self.save_draft_search.clear();
            self.save_draft_new_folder_name = None;
        }
    }

    /// Human-readable breadcrumb of the currently-selected destination
    /// folder path, e.g. "Personal / API v2 / Search".
    fn save_draft_breadcrumb(&self) -> String {
        if self.save_draft_target_path.is_empty() {
            return "(select a folder)".to_string();
        }
        let mut parts: Vec<String> = Vec::new();
        let mut current: Option<&Folder> = None;
        for (depth, id) in self.save_draft_target_path.iter().enumerate() {
            let found = if depth == 0 {
                self.state.folders.iter().find(|f| &f.id == id)
            } else {
                current.and_then(|c| c.subfolders.iter().find(|f| &f.id == id))
            };
            match found {
                Some(f) => {
                    parts.push(f.name.clone());
                    current = Some(f);
                }
                None => break,
            }
        }
        parts.join(" / ")
    }

    /// Recursively render one row of the save-draft folder tree. `path`
    /// is the running path from root down to (not including) `folder`;
    /// `target` is the currently-selected destination that this row will
    /// update when clicked.
    fn render_save_tree_row(
        ui: &mut egui::Ui,
        folder: &Folder,
        path: &mut Vec<String>,
        target: &mut Vec<String>,
        query: &str,
    ) {
        let mut this_path = path.clone();
        this_path.push(folder.id.clone());

        // Filter: show row if self or any descendant matches the query.
        let matches_self = query.is_empty() || folder.name.to_lowercase().contains(query);
        let matches_descendant = !query.is_empty()
            && folder.subfolders.iter().any(|f| Self::subtree_has_match(f, query));
        if !matches_self && !matches_descendant {
            return;
        }

        let depth = path.len();
        let is_selected = *target == this_path;
        let row_h = 28.0;
        let indent = 14.0 * depth as f32 + 8.0;

        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_h),
            egui::Sense::click(),
        );
        if ui.is_rect_visible(rect) {
            let bg = if is_selected {
                C_ACCENT.linear_multiply(0.18)
            } else if resp.hovered() {
                C_ELEVATED
            } else {
                egui::Color32::TRANSPARENT
            };
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(4.0), bg);
            let icon_x = rect.left() + indent;
            let text_y = rect.center().y;
            // Folder icon — painter-drawn (two stacked rounded rects) so
            // we don't depend on unicode glyphs egui's bundled font
            // doesn't ship (e.g. `▸` rendered as a tofu square).
            let icon_body = egui::Rect::from_min_size(
                egui::pos2(icon_x, text_y - 4.0),
                egui::vec2(14.0, 10.0),
            );
            let icon_tab = egui::Rect::from_min_size(
                egui::pos2(icon_x, text_y - 7.0),
                egui::vec2(6.0, 3.5),
            );
            let icon_color = if is_selected { C_ACCENT } else { C_MUTED };
            ui.painter()
                .rect_filled(icon_tab, egui::Rounding::same(1.5), icon_color);
            ui.painter()
                .rect_filled(icon_body, egui::Rounding::same(2.0), icon_color);
            ui.painter().text(
                egui::pos2(icon_x + 20.0, text_y),
                egui::Align2::LEFT_CENTER,
                &folder.name,
                egui::FontId::proportional(13.0),
                C_TEXT,
            );
        }
        if resp.clicked() {
            *target = this_path.clone();
        }

        // Recurse into subfolders.
        if !folder.subfolders.is_empty() {
            path.push(folder.id.clone());
            for sub in &folder.subfolders {
                Self::render_save_tree_row(ui, sub, path, target, query);
            }
            path.pop();
        }
    }

    fn subtree_has_match(folder: &Folder, query: &str) -> bool {
        if folder.name.to_lowercase().contains(query) {
            return true;
        }
        folder.subfolders.iter().any(|f| Self::subtree_has_match(f, query))
    }

    /// Mutable lookup of a folder at an arbitrary path (top-level collection
    /// at path[0], nested subfolders after). Returns None if the path
    /// doesn't resolve.
    fn folder_at_path_mut(&mut self, path: &[String]) -> Option<&mut Folder> {
        if path.is_empty() {
            return None;
        }
        let mut cur = self.state.folders.iter_mut().find(|f| f.id == path[0])?;
        for id in &path[1..] {
            cur = cur.subfolders.iter_mut().find(|f| &f.id == id)?;
        }
        Some(cur)
    }

    /// Move the draft referenced by the modal into the selected folder path.
    fn commit_save_draft(&mut self) {
        let Some(idx) = self.save_draft_tab_idx else { return; };
        let target_path = self.save_draft_target_path.clone();
        if target_path.is_empty() {
            return;
        }
        let Some(tab) = self.state.open_tabs.get(idx).cloned() else { return; };
        if !tab.folder_path.is_empty() {
            return;
        }
        let draft_id = tab.request_id.clone();
        let draft_pos = self.state.drafts.iter().position(|d| d.id == draft_id);
        let Some(pos) = draft_pos else { return; };
        let mut req = self.state.drafts.remove(pos);
        req.name = self.save_draft_name.trim().to_string();

        let inserted = if let Some(folder) = self.folder_at_path_mut(&target_path) {
            folder.requests.push(req);
            true
        } else {
            false
        };
        if inserted {
            self.state.open_tabs[idx].folder_path = target_path.clone();
            self.selected_folder_path = target_path;
            self.selected_request_id = Some(draft_id);
            self.load_request_for_editing();
            self.save_state();
            self.show_toast("Request saved");
        }

        self.save_draft_open = false;
        self.save_draft_tab_idx = None;
        self.save_draft_target_path.clear();
        self.save_draft_name.clear();
        self.save_draft_search.clear();
        self.save_draft_new_folder_name = None;
    }

    /// Create an "Untitled" draft request, add it as a tab, and activate it.
    /// The draft lives in `state.drafts` (persisted) until the user either
    /// closes the tab or explicitly saves it to a folder via the tab's
    /// right-click menu.
    pub(crate) fn new_draft_request(&mut self) {
        let draft = Request {
            id: Uuid::new_v4().to_string(),
            name: "Untitled".to_string(),
            method: HttpMethod::GET,
            url: String::new(),
            query_params: vec![],
            headers: vec![],
            cookies: vec![],
            body: String::new(),
            body_ext: None,
            auth: Auth::None,
            extractors: vec![],
        };
        let id = draft.id.clone();
        self.state.drafts.push(draft);
        self.state.open_tabs.push(OpenTab {
            folder_path: vec![],
            request_id: id.clone(),
        });
        self.selected_folder_path = vec![];
        self.selected_request_id = Some(id);
        self.load_request_for_editing();
        self.response_text.clear();
        self.response_status.clear();
        self.response_time.clear();
        self.response_headers.clear();
        self.save_state();
    }

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
        let snippet = render_snippet(&req, self.snippet_lang);
        let mut copy_clicked = false;
        let mut close_clicked = false;

        egui::SidePanel::right("snippet_panel")
            .resizable(true)
            .default_width(380.0)
            .width_range(280.0..=600.0)
            .frame(
                egui::Frame::none()
                    .fill(C_PANEL)
                    .inner_margin(egui::Margin::symmetric(10.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Code snippet")
                            .size(14.0)
                            .strong()
                            .color(C_TEXT),
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
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Copy")
                                        .color(egui::Color32::WHITE)
                                        .strong(),
                                )
                                .fill(C_ACCENT)
                                .min_size(egui::vec2(70.0, 26.0)),
                            )
                            .clicked()
                        {
                            copy_clicked = true;
                        }
                    });
                });
                ui.add_space(8.0);
                egui::Frame::none()
                    .fill(C_PANEL_DARK)
                    .stroke(egui::Stroke::new(1.0, C_BORDER))
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
                                let line_count =
                                    snippet.split('\n').count().max(1);
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
                                                    egui::RichText::new(format!(
                                                        "{:>3}",
                                                        i
                                                    ))
                                                    .color(egui::Color32::from_rgb(100, 105, 115))
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
            self.show_toast("Snippet copied");
        }
        if close_clicked {
            self.show_snippet_panel = false;
        }
    }

    pub(crate) fn render_paste_modal(&mut self, ctx: &egui::Context) {
        if !self.show_paste_modal {
            return;
        }
        let mut open = self.show_paste_modal;
        let mut do_import: Option<Request> = None;
        egui::Window::new("Import from cURL")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .default_height(340.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Paste a cURL command below and click Import.")
                        .color(C_MUTED)
                        .size(12.0),
                );
                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.paste_curl_text)
                        .code_editor()
                        .desired_rows(10)
                        .desired_width(f32::INFINITY)
                        .hint_text("curl -X POST 'https://api.example.com' -H 'Content-Type: application/json' -d '{\"k\":\"v\"}'"),
                );
                if !self.paste_error.is_empty() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&self.paste_error).color(C_RED));
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Import")
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .fill(C_PURPLE)
                            .min_size(egui::vec2(90.0, 28.0)),
                        )
                        .clicked()
                    {
                        match curl::parse_curl(&self.paste_curl_text) {
                            Ok(req) => do_import = Some(req),
                            Err(e) => self.paste_error = e,
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_paste_modal = false;
                    }
                });
            });
        self.show_paste_modal = open;

        if let Some(mut req) = do_import {
            // Ensure we have a folder to put it in
            if self.state.folders.is_empty() {
                self.state.folders.push(Folder {
                    id: Uuid::new_v4().to_string(),
                    name: "Imported".to_string(),
                    requests: vec![],
                    subfolders: vec![],
                });
            }

            let target_path = if !self.selected_folder_path.is_empty() {
                self.selected_folder_path.clone()
            } else {
                vec![self.state.folders[0].id.clone()]
            };
            self.selected_folder_path = target_path;

            // Name based on method + host/path
            if req.name == "Imported from cURL" {
                let short = short_name_from_url(&req.url);
                req.name = format!("{} {}", req.method, short);
            }
            let new_id = req.id.clone();
            if let Some(folder) = self.get_current_folder_mut() {
                folder.requests.push(req);
            }
            self.save_state();
            let p = self.selected_folder_path.clone();
            self.open_request(p, new_id);
            self.show_paste_modal = false;
            self.show_toast("Request imported");
        }
    }

    /// App-wide settings modal — request timeout, body size cap, proxy,
    /// TLS verification. Changes take effect immediately (we rebuild
    /// the shared `reqwest::Client` on save).
    pub(crate) fn render_settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_settings_modal {
            return;
        }
        let mut open = self.show_settings_modal;
        let mut do_save = false;
        let mut do_cancel = false;

        egui::Window::new(
            egui::RichText::new("SETTINGS").size(12.0).strong().color(C_MUTED),
        )
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_min_width(420.0);

                // Request timeout
                ui.label(egui::RichText::new("Request timeout (seconds)").size(11.5).color(C_MUTED));
                ui.add(
                    egui::DragValue::new(&mut self.editing_settings.timeout_sec)
                        .range(0..=3600)
                        .speed(1.0)
                        .suffix(" s"),
                );
                ui.label(
                    egui::RichText::new("0 disables the timeout (requests can hang forever).")
                        .size(10.5)
                        .color(C_MUTED),
                );
                ui.add_space(10.0);

                // Max body size
                ui.label(egui::RichText::new("Max response body (MB)").size(11.5).color(C_MUTED));
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
                    .color(C_MUTED),
                );
                ui.add_space(10.0);

                // Proxy
                ui.label(egui::RichText::new("Proxy URL").size(11.5).color(C_MUTED));
                ui.add(
                    egui::TextEdit::singleline(&mut self.editing_settings.proxy_url)
                        .hint_text("http://proxy:8080 (leave empty for direct)")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(10.0);

                // Verify TLS
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
                    .color(C_MUTED),
                );

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
                    .fill(C_ACCENT)
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
        }
    }

    pub(crate) fn render_toast(&mut self, ctx: &egui::Context) {
        let Some((msg, ttl)) = self.toast.clone() else {
            return;
        };
        let dt = ctx.input(|i| i.unstable_dt);
        let new_ttl = ttl - dt;
        if new_ttl <= 0.0 {
            self.toast = None;
            return;
        }
        self.toast = Some((msg.clone(), new_ttl));
        ctx.request_repaint();
        egui::Area::new(egui::Id::new("toast"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(C_PANEL)
                    .stroke(egui::Stroke::new(1.0, C_ACCENT))
                    .rounding(10.0)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(msg).color(C_TEXT).size(13.0));
                    });
            });
    }

}
