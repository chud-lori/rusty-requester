use super::*;

impl ApiClient {
    pub(crate) fn render_env_modal(&mut self, ctx: &egui::Context) {
        if !self.show_env_modal {
            return;
        }
        let mut open = self.show_env_modal;
        let mut create_env = false;
        let mut delete_id: Option<String> = None;
        let mut add_missing_key: Option<(String, KvRow)> = None;
        let mut copy_summary: Option<String> = None;

        self.normalize_env_compare_selection();

        egui::Window::new("Environments")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(760.0)
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
                                .color(muted()),
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
                                        .color(accent()),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::new(1.0, border())),
                            )
                            .clicked()
                        {
                            create_env = true;
                        }
                    });

                    ui.separator();

                    // Right column: editor for selected env + compare.
                    ui.vertical(|ui| {
                        let id = self.selected_env_for_edit.clone();
                        let env_idx = id.as_ref().and_then(|id| {
                            self.state.environments.iter().position(|e| &e.id == id)
                        });
                        if let Some(idx) = env_idx {
                            let mut name = self.state.environments[idx].name.clone();
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Name").size(11.0).color(muted()));
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
                                .color(muted()),
                            );
                            ui.add_space(4.0);
                            let mut vars = self.state.environments[idx].variables.clone();
                            let changed = render_kv_table(ui, "Variables", &mut vars, false);
                            if changed {
                                self.state.environments[idx].variables = vars;
                                self.save_state();
                            }

                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(8.0);
                        } else {
                            ui.label(
                                egui::RichText::new(
                                    "Select an environment on the left, or create a new one.",
                                )
                                .color(muted()),
                            );
                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(8.0);
                        }

                        ui.label(
                            egui::RichText::new("Compare environments")
                                .size(11.0)
                                .strong()
                                .color(muted()),
                        );
                        ui.add_space(4.0);

                        if self.state.environments.len() < 2 {
                            ui.label(
                                egui::RichText::new(
                                    "Create at least two environments to compare variables.",
                                )
                                .color(muted()),
                            );
                        } else {
                            ui.horizontal(|ui| {
                                env_combo(
                                    ui,
                                    "env_compare_source",
                                    "Source",
                                    &self.state.environments,
                                    &mut self.env_compare_source_id,
                                );
                                env_combo(
                                    ui,
                                    "env_compare_target",
                                    "Target",
                                    &self.state.environments,
                                    &mut self.env_compare_target_id,
                                );
                                if ui.button("Swap").clicked() {
                                    std::mem::swap(
                                        &mut self.env_compare_source_id,
                                        &mut self.env_compare_target_id,
                                    );
                                }
                            });
                            ui.add_space(6.0);

                            let source_env = self
                                .env_compare_source_id
                                .as_ref()
                                .and_then(|id| self.state.environments.iter().find(|e| &e.id == id))
                                .cloned();
                            let target_env = self
                                .env_compare_target_id
                                .as_ref()
                                .and_then(|id| self.state.environments.iter().find(|e| &e.id == id))
                                .cloned();

                            if let (Some(source), Some(target)) = (source_env, target_env) {
                                let diff = compare_environments(&source, &target);
                                ui.horizontal_wrapped(|ui| {
                                    diff_badge(ui, "Added", diff.added.len(), C_GREEN);
                                    diff_badge(ui, "Missing", diff.missing.len(), C_ORANGE);
                                    diff_badge(ui, "Changed", diff.changed.len(), accent());
                                    diff_badge(ui, "Unchanged", diff.unchanged.len(), muted());
                                    if ui.button("Copy safe summary").clicked() {
                                        copy_summary =
                                            Some(safe_summary(&source.name, &target.name, &diff));
                                    }
                                });
                                ui.add_space(6.0);

                                if source.id == target.id {
                                    ui.label(
                                        egui::RichText::new(
                                            "Pick two different environments for a useful diff.",
                                        )
                                        .color(muted()),
                                    );
                                } else if diff.is_empty() {
                                    ui.label(
                                        egui::RichText::new(
                                            "No variable keys in either environment.",
                                        )
                                        .color(muted()),
                                    );
                                } else {
                                    egui::ScrollArea::vertical()
                                        .id_salt("env_compare_scroll")
                                        .max_height(220.0)
                                        .show(ui, |ui| {
                                            render_env_diff_group(
                                                ui,
                                                "Missing from target",
                                                &diff.missing,
                                                C_ORANGE,
                                                Some((&target.id, &source.name)),
                                                &mut add_missing_key,
                                            );
                                            render_env_diff_group(
                                                ui,
                                                "Added in target",
                                                &diff.added,
                                                C_GREEN,
                                                None,
                                                &mut add_missing_key,
                                            );
                                            render_env_diff_group(
                                                ui,
                                                "Changed",
                                                &diff.changed,
                                                accent(),
                                                None,
                                                &mut add_missing_key,
                                            );
                                            render_env_diff_group(
                                                ui,
                                                "Unchanged",
                                                &diff.unchanged,
                                                muted(),
                                                None,
                                                &mut add_missing_key,
                                            );
                                        });
                                }
                            }
                        }
                    });
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
            self.normalize_env_compare_selection();
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
            if self.env_compare_source_id.as_deref() == Some(&id) {
                self.env_compare_source_id = None;
            }
            if self.env_compare_target_id.as_deref() == Some(&id) {
                self.env_compare_target_id = None;
            }
            self.normalize_env_compare_selection();
            self.save_state();
        }
        if let Some((target_id, row)) = add_missing_key {
            if let Some(env) = self
                .state
                .environments
                .iter_mut()
                .find(|env| env.id == target_id)
            {
                env.variables.push(row);
                self.save_state();
                self.show_toast("Missing environment key added");
            }
        }
        if let Some(summary) = copy_summary {
            ctx.copy_text(summary);
            self.show_toast("Environment diff summary copied");
        }
    }

    fn normalize_env_compare_selection(&mut self) {
        let ids: Vec<String> = self
            .state
            .environments
            .iter()
            .map(|env| env.id.clone())
            .collect();
        if ids.is_empty() {
            self.env_compare_source_id = None;
            self.env_compare_target_id = None;
            return;
        }

        let preferred_source = self
            .env_compare_source_id
            .clone()
            .filter(|id| ids.contains(id))
            .or_else(|| {
                self.state
                    .active_env_id
                    .clone()
                    .filter(|id| ids.contains(id))
            })
            .or_else(|| {
                self.selected_env_for_edit
                    .clone()
                    .filter(|id| ids.contains(id))
            })
            .unwrap_or_else(|| ids[0].clone());

        let preferred_target = self
            .env_compare_target_id
            .clone()
            .filter(|id| ids.contains(id) && id != &preferred_source)
            .or_else(|| ids.iter().find(|id| *id != &preferred_source).cloned())
            .unwrap_or_else(|| preferred_source.clone());

        self.env_compare_source_id = Some(preferred_source);
        self.env_compare_target_id = Some(preferred_target);
    }
}

fn env_combo(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    envs: &[Environment],
    selected_id: &mut Option<String>,
) {
    let selected_name = selected_id
        .as_ref()
        .and_then(|id| envs.iter().find(|env| &env.id == id))
        .map(|env| env.name.as_str())
        .unwrap_or("Select");

    ui.label(egui::RichText::new(label).size(11.0).color(muted()));
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected_name)
        .width(170.0)
        .show_ui(ui, |ui| {
            for env in envs {
                let selected = selected_id.as_deref() == Some(env.id.as_str());
                if ui.selectable_label(selected, &env.name).clicked() {
                    *selected_id = Some(env.id.clone());
                }
            }
        });
}

fn diff_badge(ui: &mut egui::Ui, label: &str, count: usize, color: egui::Color32) {
    ui.label(
        egui::RichText::new(format!("{} {}", label, count))
            .size(11.0)
            .color(color)
            .strong(),
    );
}

fn render_env_diff_group(
    ui: &mut egui::Ui,
    title: &str,
    entries: &[EnvDiffEntry],
    color: egui::Color32,
    add_target: Option<(&str, &str)>,
    add_missing_key: &mut Option<(String, KvRow)>,
) {
    if entries.is_empty() {
        return;
    }

    egui::CollapsingHeader::new(
        egui::RichText::new(format!("{} ({})", title, entries.len()))
            .size(12.0)
            .strong()
            .color(color),
    )
    .default_open(!matches!(title, "Unchanged"))
    .show(ui, |ui| {
        for entry in entries {
            egui::Frame::none()
                .fill(with_alpha(
                    if matches!(title, "Unchanged") {
                        border()
                    } else {
                        color
                    },
                    if is_light() { 12 } else { 18 },
                ))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.add_sized(
                            [150.0, 18.0],
                            egui::Label::new(
                                egui::RichText::new(&entry.key)
                                    .monospace()
                                    .size(12.0)
                                    .color(text()),
                            ),
                        );
                        ui.label(
                            egui::RichText::new(env_entry_value(entry))
                                .size(12.0)
                                .color(muted()),
                        );
                        if let (Some((target_id, source_name)), Some(source)) =
                            (add_target, entry.source.as_ref())
                        {
                            if ui.button("Add to target").clicked() {
                                *add_missing_key = Some((
                                    target_id.to_string(),
                                    KvRow {
                                        enabled: source.enabled,
                                        key: entry.key.clone(),
                                        value: source.value.clone(),
                                        description: format!("Added from {}", source_name),
                                    },
                                ));
                            }
                        }
                    });
                });
            ui.add_space(3.0);
        }
    });
}

fn env_entry_value(entry: &EnvDiffEntry) -> String {
    match (&entry.source, &entry.target) {
        (Some(source), Some(target)) => format!(
            "{}{} -> {}{}",
            display_value(&entry.key, &source.value),
            enabled_suffix(source.enabled),
            display_value(&entry.key, &target.value),
            enabled_suffix(target.enabled)
        ),
        (Some(source), None) => format!(
            "{}{}",
            display_value(&entry.key, &source.value),
            enabled_suffix(source.enabled)
        ),
        (None, Some(target)) => format!(
            "{}{}",
            display_value(&entry.key, &target.value),
            enabled_suffix(target.enabled)
        ),
        (None, None) => String::new(),
    }
}

fn enabled_suffix(enabled: bool) -> &'static str {
    if enabled {
        ""
    } else {
        " (disabled)"
    }
}
