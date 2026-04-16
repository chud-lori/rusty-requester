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
                            if ui.selectable_label(selected, &env.name).clicked() {
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
                                ui.label(egui::RichText::new("Name").size(11.0).color(C_MUTED));
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
                                egui::RichText::new(
                                    "Variables  (use {{name}} in URL/headers/body)",
                                )
                                .size(11.0)
                                .color(C_MUTED),
                            );
                            ui.add_space(4.0);
                            let mut vars = self.state.environments[idx].variables.clone();
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
                                egui::RichText::new("Done")
                                    .color(egui::Color32::WHITE)
                                    .strong(),
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
                cookies: vec![],
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
        let Some(draft) = self
            .state
            .drafts
            .iter()
            .find(|d| d.id == tab.request_id)
            .cloned()
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
            egui::RichText::new("SAVE REQUEST")
                .size(12.0)
                .strong()
                .color(C_MUTED),
        )
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(560.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_width(540.0);

            // Name
            ui.label(
                egui::RichText::new("Request name")
                    .size(11.0)
                    .color(C_MUTED),
            );
            let name_resp = ui.add(
                egui::TextEdit::singleline(&mut self.save_draft_name).desired_width(f32::INFINITY),
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
                        egui::RichText::new("+ New folder")
                            .size(12.0)
                            .color(C_ACCENT),
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
                description: String::new(),
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
            && folder
                .subfolders
                .iter()
                .any(|f| Self::subtree_has_match(f, query));
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
            let icon_body =
                egui::Rect::from_min_size(egui::pos2(icon_x, text_y - 4.0), egui::vec2(14.0, 10.0));
            let icon_tab =
                egui::Rect::from_min_size(egui::pos2(icon_x, text_y - 7.0), egui::vec2(6.0, 3.5));
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
        folder
            .subfolders
            .iter()
            .any(|f| Self::subtree_has_match(f, query))
    }

    /// Mutable lookup of a folder at an arbitrary path (top-level collection
    /// at path[0], nested subfolders after). Returns None if the path
    /// doesn't resolve.
    pub(crate) fn folder_at_path_mut(&mut self, path: &[String]) -> Option<&mut Folder> {
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
        let Some(idx) = self.save_draft_tab_idx else {
            return;
        };
        let target_path = self.save_draft_target_path.clone();
        if target_path.is_empty() {
            return;
        }
        let Some(tab) = self.state.open_tabs.get(idx).cloned() else {
            return;
        };
        if !tab.folder_path.is_empty() {
            return;
        }
        let draft_id = tab.request_id.clone();
        let draft_pos = self.state.drafts.iter().position(|d| d.id == draft_id);
        let Some(pos) = draft_pos else {
            return;
        };
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
            assertions: vec![],
        };
        let id = draft.id.clone();
        self.state.drafts.push(draft);
        self.state.open_tabs.push(OpenTab {
            folder_path: vec![],
            request_id: id.clone(),
            pinned: false,
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
                        if icon_btn(ui, egui_phosphor::regular::COPY, "Copy snippet").clicked() {
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
                    description: String::new(),
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

    /// "Save changes?" confirmation when closing a draft (unsaved) tab.
    pub(crate) fn render_confirm_close_draft(&mut self, ctx: &egui::Context) {
        let Some(idx) = self.confirm_close_draft_idx else {
            return;
        };
        // Gather info about the draft for the message.
        let draft_url = self
            .state
            .open_tabs
            .get(idx)
            .and_then(|tab| self.state.drafts.iter().find(|d| d.id == tab.request_id))
            .map(|d| d.url.clone())
            .unwrap_or_default();

        let display_url = if draft_url.is_empty() {
            "Untitled request".to_string()
        } else if draft_url.chars().count() > 45 {
            let cut: String = draft_url.chars().take(45).collect();
            format!("{}...", cut)
        } else {
            draft_url
        };

        let mut open = true;
        egui::Window::new("Save changes?")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([380.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "{} has unsaved changes. Save these changes to avoid losing your work.",
                        display_url
                    ))
                    .color(C_TEXT)
                    .size(13.0),
                );
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui.button("Don't save").clicked() {
                        self.close_tab_force(idx);
                        self.confirm_close_draft_idx = None;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let save_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Save changes").color(egui::Color32::WHITE),
                            )
                            .fill(C_ACCENT),
                        );
                        if save_btn.clicked() {
                            self.confirm_close_draft_idx = None;
                            self.begin_save_draft(idx);
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_close_draft_idx = None;
                        }
                    });
                });
            });
        if !open {
            self.confirm_close_draft_idx = None;
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
            egui::RichText::new("SETTINGS")
                .size(12.0)
                .strong()
                .color(C_MUTED),
        )
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(440.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_width(420.0);

            // Request timeout
            ui.label(
                egui::RichText::new("Request timeout (seconds)")
                    .size(11.5)
                    .color(C_MUTED),
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
                    .color(C_MUTED),
            );
            ui.add_space(10.0);

            // Max body size
            ui.label(
                egui::RichText::new("Max response body (MB)")
                    .size(11.5)
                    .color(C_MUTED),
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

            ui.add_space(10.0);
            // Theme — Dark (default) / Light.
            ui.label(egui::RichText::new("Theme").size(11.0).color(C_MUTED));
            ui.horizontal(|ui| {
                let mut t = self.editing_settings.theme;
                ui.selectable_value(&mut t, Theme::Dark, "Dark");
                ui.selectable_value(&mut t, Theme::Light, "Light");
                self.editing_settings.theme = t;
            });
            ui.label(
                egui::RichText::new(
                    "Light theme flips egui's chrome (panels, text, borders). \
                         HTTP method colors and status pills stay the same across \
                         themes.",
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
        let mut action_export_json = false;
        let mut action_export_yaml = false;
        let mut action_toggle_snippet = false;
        let mut action_open_settings = false;
        let mut action_open_env = false;
        let mut action_show_about = false;
        let mut action_quit = false;

        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(C_PANEL_DARK)
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
        if action_export_json {
            self.pending_export_json = true;
        }
        if action_export_yaml {
            self.pending_export_yaml = true;
        }
        if action_toggle_snippet {
            self.show_snippet_panel = !self.show_snippet_panel;
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

    /// About dialog — minimal Postman-style layout: icon, name,
    /// version lines, one-line tagline, three stacked plain links,
    /// copyright. Used by both the macOS app/Help menu and the
    /// in-window Linux menu bar.
    pub(crate) fn render_about_modal(&mut self, ctx: &egui::Context) {
        if !self.show_about_modal {
            return;
        }
        let mut open = self.show_about_modal;
        egui::Window::new(
            egui::RichText::new("ABOUT")
                .size(12.0)
                .strong()
                .color(C_MUTED),
        )
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_width(360.0);

            let open_url = |ctx: &egui::Context, url: &str| {
                ctx.output_mut(|o| {
                    o.open_url = Some(egui::output::OpenUrl {
                        url: url.to_string(),
                        new_tab: true,
                    });
                });
            };
            let link_row = |ui: &mut egui::Ui, label: &str, url: &str, ctx: &egui::Context| {
                if ui
                    .link(egui::RichText::new(label).size(12.5).color(C_ACCENT))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    open_url(ctx, url);
                }
            };

            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                if let Some(tex) = &self.app_icon {
                    ui.add(
                        egui::Image::from_texture(tex)
                            .fit_to_exact_size(egui::vec2(80.0, 80.0))
                            .rounding(egui::Rounding::same(14.0)),
                    );
                }
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("Rusty Requester")
                        .size(19.0)
                        .strong()
                        .color(C_TEXT),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(concat!("Version ", env!("CARGO_PKG_VERSION")))
                        .size(12.0)
                        .color(C_TEXT),
                );
                ui.label(
                    egui::RichText::new(concat!("Build: ", env!("CARGO_PKG_VERSION"), " (native)"))
                        .size(11.5)
                        .color(C_MUTED),
                );

                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("A native, offline, lightweight API client.")
                        .size(12.0)
                        .color(C_TEXT),
                );

                ui.add_space(12.0);
                link_row(
                    ui,
                    "GitHub Repository",
                    "https://github.com/chud-lori/rusty-requester",
                    ctx,
                );
                ui.add_space(4.0);
                link_row(
                    ui,
                    "Report an issue",
                    "https://github.com/chud-lori/rusty-requester/issues",
                    ctx,
                );
                ui.add_space(4.0);
                link_row(
                    ui,
                    "Creator: Lori (@chud-lori)",
                    "https://github.com/chud-lori",
                    ctx,
                );

                ui.add_space(14.0);
                ui.label(
                    egui::RichText::new("MIT Licensed · © Lori (@chud-lori)")
                        .size(11.0)
                        .color(C_MUTED),
                );
                ui.add_space(10.0);
            });
        });
        if !open {
            self.show_about_modal = false;
        }
    }

    /// Command palette (⌘P) — fuzzy-find across every request in
    /// every collection and jump to it. Modal Area-based popup with
    /// keyboard-only navigation (↑/↓, Enter to activate, Esc to
    /// dismiss). Matches VS Code / Sublime / fzf-style UX.
    pub(crate) fn render_command_palette(&mut self, ctx: &egui::Context) {
        if !self.show_command_palette {
            return;
        }
        // Build the match list — (path, request, display labels).
        let entries = collect_palette_entries(&self.state.folders);
        let query_lc = self.palette_query.to_lowercase();
        let matches: Vec<&PaletteEntry> = entries
            .iter()
            .filter(|e| {
                if query_lc.is_empty() {
                    true
                } else {
                    fuzzy_contains(&e.haystack_lc, &query_lc)
                }
            })
            .take(200)
            .collect();

        // Clamp selection to the visible matches (in case the user
        // typed and the filter shrunk the list).
        if self.palette_selected >= matches.len() {
            self.palette_selected = matches.len().saturating_sub(1);
        }

        let (enter, esc, arrow_up, arrow_down) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        if esc {
            self.show_command_palette = false;
            return;
        }
        if arrow_down && !matches.is_empty() {
            self.palette_selected = (self.palette_selected + 1) % matches.len();
        }
        if arrow_up && !matches.is_empty() {
            self.palette_selected = if self.palette_selected == 0 {
                matches.len() - 1
            } else {
                self.palette_selected - 1
            };
        }
        let mut activate: Option<(Vec<String>, String)> = None;
        if enter {
            if let Some(e) = matches.get(self.palette_selected) {
                activate = Some((e.folder_path.clone(), e.request_id.clone()));
            }
        }

        // Darken the background to draw focus.
        // No dim backdrop — VS Code / Raycast / Spotlight all forgo
        // one and rely on a shadowed floating panel to imply depth.
        // Earlier attempts with an `alpha` overlay fought the palette
        // for the same egui `Order::Middle` layer (dimming its content)
        // and generally looked heavy.

        let mut open = true;
        egui::Window::new(
            egui::RichText::new("COMMAND PALETTE")
                .size(11.0)
                .strong()
                .color(C_MUTED),
        )
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .fixed_size(egui::vec2(560.0, 420.0))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 80.0))
        .frame(palette_frame(self.state.settings.theme))
        .show(ctx, |ui| {
            let query_resp = ui.add(
                egui::TextEdit::singleline(&mut self.palette_query)
                    .hint_text("Search requests by name, URL, or method…")
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Body),
            );
            if self.palette_focus_pending {
                self.palette_focus_pending = false;
                query_resp.request_focus();
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!(
                    "{} result{}",
                    matches.len(),
                    if matches.len() == 1 { "" } else { "s" }
                ))
                .size(10.5)
                .color(C_MUTED),
            );
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("palette_scroll")
                .max_height(320.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, m) in matches.iter().enumerate() {
                        let is_sel = i == self.palette_selected;
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 34.0),
                            egui::Sense::click(),
                        );
                        if ui.is_rect_visible(rect) {
                            let bg = if is_sel {
                                C_ACCENT.linear_multiply(0.18)
                            } else if resp.hovered() {
                                C_ELEVATED
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.painter()
                                .rect_filled(rect, egui::Rounding::same(5.0), bg);
                            // Method
                            let mc = method_color(&m.method);
                            ui.painter().text(
                                egui::pos2(rect.left() + 10.0, rect.top() + 10.0),
                                egui::Align2::LEFT_TOP,
                                format!("{}", m.method),
                                egui::FontId::new(10.5, egui::FontFamily::Proportional),
                                mc,
                            );
                            ui.painter().text(
                                egui::pos2(rect.left() + 60.0, rect.top() + 7.0),
                                egui::Align2::LEFT_TOP,
                                &m.name,
                                egui::FontId::new(13.0, egui::FontFamily::Proportional),
                                C_TEXT,
                            );
                            ui.painter().text(
                                egui::pos2(rect.left() + 60.0, rect.top() + 22.0),
                                egui::Align2::LEFT_TOP,
                                &m.breadcrumb,
                                egui::FontId::new(10.5, egui::FontFamily::Proportional),
                                C_MUTED,
                            );
                        }
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            activate = Some((m.folder_path.clone(), m.request_id.clone()));
                        }
                    }
                });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("↑ ↓  navigate    Enter  open    Esc  dismiss")
                        .size(10.5)
                        .color(C_MUTED),
                );
            });
        });
        if !open {
            self.show_command_palette = false;
        }
        if let Some((path, req_id)) = activate {
            self.show_command_palette = false;
            self.open_request(path, req_id);
        }
    }

    /// Actions palette (⇧⌘P) — the counterpart to `render_command_palette`.
    /// Same overlay chrome, but the list is `actions::PaletteAction::ALL`
    /// and Enter dispatches through `run_action` instead of opening a
    /// request.
    pub(crate) fn render_actions_palette(&mut self, ctx: &egui::Context) {
        if !self.show_actions_palette {
            return;
        }
        use crate::actions::PaletteAction;
        let query_lc = self.actions_palette_query.to_lowercase();
        let matches: Vec<&PaletteAction> = PaletteAction::ALL
            .iter()
            .filter(|a| {
                if query_lc.is_empty() {
                    true
                } else {
                    fuzzy_contains(&a.haystack_lc(), &query_lc)
                }
            })
            .collect();

        if self.actions_palette_selected >= matches.len() {
            self.actions_palette_selected = matches.len().saturating_sub(1);
        }

        let (enter, esc, arrow_up, arrow_down) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        if esc {
            self.show_actions_palette = false;
            return;
        }
        if arrow_down && !matches.is_empty() {
            self.actions_palette_selected = (self.actions_palette_selected + 1) % matches.len();
        }
        if arrow_up && !matches.is_empty() {
            self.actions_palette_selected = if self.actions_palette_selected == 0 {
                matches.len() - 1
            } else {
                self.actions_palette_selected - 1
            };
        }
        let mut activate: Option<PaletteAction> = None;
        if enter {
            if let Some(a) = matches.get(self.actions_palette_selected) {
                activate = Some(**a);
            }
        }

        // Dim backdrop matches the command palette for visual parity.
        // See `render_command_palette` for why there's no backdrop.

        let mut open = true;
        egui::Window::new(
            egui::RichText::new("ACTIONS")
                .size(11.0)
                .strong()
                .color(C_MUTED),
        )
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .fixed_size(egui::vec2(560.0, 420.0))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 80.0))
        .frame(palette_frame(self.state.settings.theme))
        .show(ctx, |ui| {
            let query_resp = ui.add(
                egui::TextEdit::singleline(&mut self.actions_palette_query)
                    .hint_text("Run an action…")
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Body),
            );
            if self.actions_palette_focus_pending {
                self.actions_palette_focus_pending = false;
                query_resp.request_focus();
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!(
                    "{} action{}",
                    matches.len(),
                    if matches.len() == 1 { "" } else { "s" }
                ))
                .size(10.5)
                .color(C_MUTED),
            );
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("actions_palette_scroll")
                .max_height(320.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, a) in matches.iter().enumerate() {
                        let is_sel = i == self.actions_palette_selected;
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 32.0),
                            egui::Sense::click(),
                        );
                        if ui.is_rect_visible(rect) {
                            let bg = if is_sel {
                                C_ACCENT.linear_multiply(0.18)
                            } else if resp.hovered() {
                                C_ELEVATED
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.painter()
                                .rect_filled(rect, egui::Rounding::same(5.0), bg);
                            ui.painter().text(
                                egui::pos2(rect.left() + 14.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                a.label(),
                                egui::FontId::new(13.0, egui::FontFamily::Proportional),
                                C_TEXT,
                            );
                            if let Some(sc) = a.shortcut() {
                                ui.painter().text(
                                    egui::pos2(rect.right() - 14.0, rect.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    sc,
                                    egui::FontId::new(11.0, egui::FontFamily::Monospace),
                                    C_MUTED,
                                );
                            }
                        }
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            activate = Some(**a);
                        }
                    }
                });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("↑ ↓  navigate    Enter  run    Esc  dismiss")
                        .size(10.5)
                        .color(C_MUTED),
                );
            });
        });
        if !open {
            self.show_actions_palette = false;
        }
        if let Some(action) = activate {
            self.show_actions_palette = false;
            self.run_action(action);
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

/// One row in the command palette result list.
struct PaletteEntry {
    folder_path: Vec<String>,
    request_id: String,
    name: String,
    method: HttpMethod,
    /// "Personal / api-v2 / GET" — shown as the secondary line in
    /// the palette so users see where the request lives.
    breadcrumb: String,
    /// Lowercased haystack used by the fuzzy matcher (name + URL +
    /// method + breadcrumb concatenated). Cached so we don't
    /// re-allocate per keystroke.
    haystack_lc: String,
}

fn collect_palette_entries(folders: &[Folder]) -> Vec<PaletteEntry> {
    let mut out = Vec::new();
    for folder in folders {
        walk_palette(
            folder,
            vec![folder.id.clone()],
            folder.name.clone(),
            &mut out,
        );
    }
    out
}

fn walk_palette(
    folder: &Folder,
    path: Vec<String>,
    breadcrumb: String,
    out: &mut Vec<PaletteEntry>,
) {
    for r in &folder.requests {
        let haystack = format!("{} {} {} {}", r.name, r.url, r.method, breadcrumb).to_lowercase();
        out.push(PaletteEntry {
            folder_path: path.clone(),
            request_id: r.id.clone(),
            name: r.name.clone(),
            method: r.method.clone(),
            breadcrumb: format!("{} · {}", breadcrumb, r.url),
            haystack_lc: haystack,
        });
    }
    for sub in &folder.subfolders {
        let mut sub_path = path.clone();
        sub_path.push(sub.id.clone());
        let sub_breadcrumb = format!("{} / {}", breadcrumb, sub.name);
        walk_palette(sub, sub_path, sub_breadcrumb, out);
    }
}

/// Tiny "subsequence" fuzzy matcher — every char of `needle` (already
/// lowercase) must appear somewhere in `haystack` in order. Same
/// algorithm fzf falls back to. Cheap, no scoring, good enough for
/// palette filtering.
fn fuzzy_contains(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let mut chars = needle.chars();
    let mut want = match chars.next() {
        Some(c) => c,
        None => return true,
    };
    for c in haystack.chars() {
        if c == want {
            match chars.next() {
                Some(next) => want = next,
                None => return true,
            }
        }
    }
    false
}

/// Frame styling for ⌘P / ⇧⌘P palette windows. Uses `C_ELEVATED`
/// instead of the default `C_BG` so the palette visibly floats above
/// the darkened backdrop — without this they blend into the app
/// chrome and look "greyed-out" rather than focused.
fn palette_frame(theme: Theme) -> egui::Frame {
    // VS Code / Raycast-style palette frame: sits directly on top of
    // the unmodified UI (no backdrop), separated only by a slight
    // elevation + subtle border + punchy drop shadow. Fill is a shade
    // brighter-than-panel in dark mode, near-white in light.
    let (fill, border) = match theme {
        Theme::Dark => (
            // `#252830` — one notch brighter than `C_ELEVATED` (#2A2D34)
            // looks muddy against other dark chrome, so we nudge a
            // touch cooler. This matches VS Code's "Quick Input" bg.
            egui::Color32::from_rgb(37, 40, 48),
            egui::Color32::from_rgb(60, 64, 72),
        ),
        Theme::Light => (
            egui::Color32::from_rgb(253, 253, 254),
            egui::Color32::from_rgb(208, 212, 220),
        ),
    };
    egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, border))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(14.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 10.0),
            blur: 28.0,
            spread: 0.0,
            // Heavier shadow than a regular modal — the palette has
            // no backdrop so the shadow alone carries the "floating"
            // read.
            color: egui::Color32::from_black_alpha(match theme {
                Theme::Dark => 180,
                Theme::Light => 80,
            }),
        })
}
