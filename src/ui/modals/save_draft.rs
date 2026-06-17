use super::*;

impl ApiClient {
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
        self.save_draft_name_focus_pending = true;
    }

    pub(crate) fn render_save_draft_modal(&mut self, ctx: &egui::Context) {
        if !self.save_draft_open {
            return;
        }
        let mut open = self.save_draft_open;
        let mut do_save = false;
        let mut do_cancel = false;
        let mut create_folder: Option<(Vec<String>, String)> = None;

        egui::Window::new("save_request_modal")
            .open(&mut open)
            .title_bar(false)
            .collapsible(false)
            .resizable(true)
            .default_width(720.0)
            .default_height(640.0)
            .min_width(660.0)
            .min_height(520.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_min_width(640.0);
                egui::Frame::none()
                    .fill(elevated())
                    .inner_margin(egui::Margin {
                        left: 18.0,
                        right: 12.0,
                        top: 12.0,
                        bottom: 12.0,
                    })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("Save request")
                                        .size(16.0)
                                        .strong()
                                        .color(text()),
                                );
                                ui.label(
                                    egui::RichText::new("Choose a collection or folder")
                                        .size(11.0)
                                        .color(muted()),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new(egui_phosphor::regular::X)
                                                    .size(15.0)
                                                    .color(muted()),
                                            )
                                            .fill(egui::Color32::TRANSPARENT)
                                            .stroke(egui::Stroke::NONE)
                                            .min_size(egui::vec2(30.0, 30.0)),
                                        )
                                        .on_hover_text("Close")
                                        .clicked()
                                    {
                                        do_cancel = true;
                                    }
                                },
                            );
                        });
                    });
                ui.painter().line_segment(
                    [
                        egui::pos2(ui.min_rect().left(), ui.min_rect().top()),
                        egui::pos2(ui.min_rect().right(), ui.min_rect().top()),
                    ],
                    egui::Stroke::new(1.0, border()),
                );
                ui.add_space(14.0);

                // Name
                ui.label(
                    egui::RichText::new("Request name")
                        .size(11.0)
                        .color(muted()),
                );
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
                        .color(text()),
                );
                ui.add_space(6.0);

                // Search
                ui.add(
                    egui::TextEdit::singleline(&mut self.save_draft_search)
                        .hint_text(hint("Search for collection or folder"))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.0);

                // Folder tree (scrollable)
                egui::Frame::none()
                    .fill(panel_dark())
                    .stroke(egui::Stroke::new(1.0, border()))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(4.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::ScrollArea::vertical()
                            .id_salt("save_draft_tree")
                            .max_height(340.0)
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
                                .hint_text(hint("New folder name"))
                                .desired_width(260.0),
                        );
                        let enabled = !name.trim().is_empty();
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Button::new(
                                    egui::RichText::new("Create").color(egui::Color32::WHITE),
                                )
                                .fill(if enabled { accent() } else { elevated() })
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
                                .color(accent()),
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

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

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
                    .fill(if can_save { accent() } else { elevated() })
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

                if self.save_draft_name_focus_pending {
                    name_resp.request_focus();
                    self.save_draft_name_focus_pending = false;
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
                accent().linear_multiply(0.18)
            } else if resp.hovered() {
                elevated()
            } else {
                egui::Color32::TRANSPARENT
            };
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(4.0), bg);
            let icon_x = rect.left() + indent;
            let text_y = rect.center().y;
            let icon_color = if is_selected { accent() } else { muted() };
            ui.painter().text(
                egui::pos2(icon_x + 7.0, text_y),
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::FOLDER_SIMPLE,
                egui::FontId::proportional(14.0),
                icon_color,
            );
            ui.painter().text(
                egui::pos2(icon_x + 22.0, text_y),
                egui::Align2::LEFT_CENTER,
                &folder.name,
                egui::FontId::proportional(13.0),
                text(),
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
            description: String::new(),
            method: HttpMethod::GET,
            url: String::new(),
            query_params: vec![],
            path_params: vec![],
            headers: vec![],
            cookies: vec![],
            body: String::new(),
            body_ext: None,
            auth: Auth::None,
            extractors: vec![],
            assertions: vec![],
            source: None,
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
                    .color(text())
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
                            .fill(accent()),
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
        // Esc dismisses the modal as Cancel — don't lose the draft,
        // don't save, just put the user back where they were.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.confirm_close_draft_idx = None;
        }
    }
}
