use super::*;

impl ApiClient {
    pub(crate) fn render_runner_modal(&mut self, ctx: &egui::Context) {
        if !self.show_runner_modal {
            return;
        }

        if let Some(scope_id) = self.runner_scope_folder_id.as_deref() {
            if crate::find_folder_by_id(&self.state.folders, scope_id).is_none() {
                self.runner_scope_folder_id = None;
            }
        }
        let request_count =
            count_runner_requests(&self.state.folders, self.runner_scope_folder_id.as_deref());
        let scope_options = runner_scope_options(&self.state.folders);
        let data_rows_label = runner_data_rows_label(&self.runner_data_rows);
        let preset_options: Vec<(String, String)> = self
            .state
            .runner_presets
            .iter()
            .map(|p| (p.id.clone(), p.name.clone()))
            .collect();
        if self
            .runner_selected_preset_id
            .as_ref()
            .is_some_and(|id| !preset_options.iter().any(|(preset_id, _)| preset_id == id))
        {
            self.runner_selected_preset_id = None;
            self.runner_preset_rename_input.clear();
        }
        let selected_preset = self
            .runner_selected_preset_id
            .as_ref()
            .and_then(|id| self.state.runner_presets.iter().find(|p| &p.id == id))
            .cloned();
        let selected_preset_label = selected_preset
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Choose preset".to_string());
        let mut open = self.show_runner_modal;
        let mut run_requested = false;
        let mut cancel_requested = false;
        let mut export_csv_requested = false;
        let mut export_html_requested = false;
        let mut copy_summary_requested: Option<String> = None;
        let mut save_preset_requested = false;
        let mut load_preset_id: Option<String> = None;
        let mut rename_preset_id: Option<String> = None;
        let mut duplicate_preset_id: Option<String> = None;
        let mut delete_preset_id: Option<String> = None;
        let runner_busy = self.runner_in_flight.is_some();
        if self
            .runner_selected_result
            .is_some_and(|index| index >= self.runner_results.len())
        {
            self.runner_selected_result = None;
        }
        let mut selected_result = self.runner_selected_result;

        egui::Window::new("Collection Runner")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(780.0)
            .default_height(560.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(230.0);
                        ui.label(
                            egui::RichText::new("Scope")
                                .size(11.0)
                                .strong()
                                .color(muted()),
                        );
                        ui.add_space(4.0);
                        egui::Frame::none()
                            .fill(elevated())
                            .stroke(egui::Stroke::new(1.0, border()))
                            .rounding(6.0)
                            .inner_margin(egui::Margin::same(10.0))
                            .show(ui, |ui| {
                                ui.add_enabled_ui(!runner_busy, |ui| {
                                    if ui
                                        .selectable_label(
                                            self.runner_scope_folder_id.is_none(),
                                            egui::RichText::new(format!(
                                                "All collections ({})",
                                                count_runner_requests(&self.state.folders, None)
                                            ))
                                            .color(text()),
                                        )
                                        .clicked()
                                    {
                                        self.runner_scope_folder_id = None;
                                    }
                                    ui.add_space(3.0);
                                    egui::ScrollArea::vertical()
                                        .id_salt("runner_scope_picker")
                                        .max_height(150.0)
                                        .auto_shrink([false, true])
                                        .show(ui, |ui| {
                                            for option in &scope_options {
                                                ui.horizontal(|ui| {
                                                    ui.add_space((option.depth as f32) * 12.0);
                                                    let selected =
                                                        self.runner_scope_folder_id.as_deref()
                                                            == Some(option.id.as_str());
                                                    if ui
                                                        .selectable_label(
                                                            selected,
                                                            egui::RichText::new(format!(
                                                                "{} ({})",
                                                                option.name, option.request_count
                                                            ))
                                                            .color(text()),
                                                        )
                                                        .clicked()
                                                    {
                                                        self.runner_scope_folder_id =
                                                            Some(option.id.clone());
                                                    }
                                                });
                                            }
                                        });
                                });
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} request{} selected",
                                        request_count,
                                        if request_count == 1 { "" } else { "s" }
                                    ))
                                    .size(11.0)
                                    .color(muted()),
                                );
                            });

                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new("Presets")
                                .size(11.0)
                                .strong()
                                .color(muted()),
                        );
                        ui.add_space(4.0);
                        egui::Frame::none()
                            .fill(elevated())
                            .stroke(egui::Stroke::new(1.0, border()))
                            .rounding(6.0)
                            .inner_margin(egui::Margin::same(10.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(
                                        "Presets save scope, active environment selection, and data rows as shown. Data row values may include secrets.",
                                    )
                                    .size(11.0)
                                    .color(muted()),
                                );
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut self.runner_preset_name_input,
                                        )
                                        .desired_width(130.0)
                                        .hint_text(hint("Preset name")),
                                    );
                                    if ui
                                        .add_enabled(
                                            !runner_busy,
                                            egui::Button::new("Save current"),
                                        )
                                        .on_hover_text(
                                            "Saves the current scope, active environment id, and visible data rows",
                                        )
                                        .clicked()
                                    {
                                        save_preset_requested = true;
                                    }
                                });

                                ui.add_space(6.0);
                                egui::ComboBox::from_id_salt("runner_preset_picker")
                                    .selected_text(selected_preset_label)
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for (id, name) in &preset_options {
                                            let selected =
                                                self.runner_selected_preset_id.as_deref()
                                                    == Some(id.as_str());
                                            if ui.selectable_label(selected, name).clicked() {
                                                self.runner_selected_preset_id = Some(id.clone());
                                                self.runner_preset_rename_input = name.clone();
                                            }
                                        }
                                    });

                                if let Some(preset) = &selected_preset {
                                    if self.runner_preset_rename_input.is_empty() {
                                        self.runner_preset_rename_input = preset.name.clone();
                                    }
                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add_enabled(!runner_busy, egui::Button::new("Load"))
                                            .clicked()
                                        {
                                            load_preset_id = Some(preset.id.clone());
                                        }
                                        if ui
                                            .add_enabled(!runner_busy, egui::Button::new("Duplicate"))
                                            .clicked()
                                        {
                                            duplicate_preset_id = Some(preset.id.clone());
                                        }
                                        if ui
                                            .add_enabled(!runner_busy, egui::Button::new("Delete"))
                                            .clicked()
                                        {
                                            delete_preset_id = Some(preset.id.clone());
                                        }
                                    });
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.runner_preset_rename_input,
                                            )
                                            .desired_width(130.0)
                                            .hint_text(hint("Rename preset")),
                                        );
                                        if ui
                                            .add_enabled(!runner_busy, egui::Button::new("Rename"))
                                            .clicked()
                                        {
                                            rename_preset_id = Some(preset.id.clone());
                                        }
                                    });
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new(runner_preset_summary(preset))
                                            .size(11.0)
                                            .color(muted()),
                                    );
                                } else if preset_options.is_empty() {
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new("No presets saved")
                                            .size(11.0)
                                            .color(muted()),
                                    );
                                }
                            });

                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new("Data rows")
                                .size(11.0)
                                .strong()
                                .color(muted()),
                        );
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::multiline(&mut self.runner_data_rows)
                                .code_editor()
                                .desired_rows(12)
                                .desired_width(f32::INFINITY)
                                .hint_text(hint(
                                    "CSV:\nusername,password\nalice,secret\n\nJSON:\n[{\"username\":\"alice\"}]",
                                )),
                        );
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(data_rows_label).size(11.0).color(muted()));

                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    request_count > 0 && !runner_busy,
                                    egui::Button::new(
                                        egui::RichText::new("Run")
                                            .color(egui::Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(C_PURPLE)
                                    .min_size(egui::vec2(96.0, 30.0)),
                                )
                                .clicked()
                            {
                                run_requested = true;
                            }
                            if ui
                                .add_enabled(
                                    runner_busy,
                                    egui::Button::new(
                                        egui::RichText::new("Cancel").color(text()),
                                    )
                                    .fill(elevated())
                                    .min_size(egui::vec2(82.0, 30.0)),
                                )
                                .clicked()
                            {
                                cancel_requested = true;
                            }
                        });
                        if request_count == 0 {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Create or import a collection to run it.")
                                    .size(11.0)
                                    .color(muted()),
                            );
                        }
                    });

                    ui.separator();

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Results")
                                    .size(11.0)
                                    .strong()
                                    .color(muted()),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let can_export =
                                        !runner_busy && !self.runner_results.is_empty();
                                    if ui
                                        .add_enabled(can_export, egui::Button::new("Export HTML"))
                                        .on_hover_text("Save an escaped HTML report")
                                        .clicked()
                                    {
                                        export_html_requested = true;
                                    }
                                    if ui
                                        .add_enabled(can_export, egui::Button::new("Export CSV"))
                                        .on_hover_text("Save a CSV report")
                                        .clicked()
                                    {
                                        export_csv_requested = true;
                                    }
                                    let can_copy = selected_result
                                        .and_then(|index| self.runner_results.get(index))
                                        .is_some();
                                    if ui
                                        .add_enabled(can_copy, egui::Button::new("Copy Summary"))
                                        .on_hover_text("Copy a secret-safe summary for the selected result")
                                        .clicked()
                                    {
                                        if let Some(row) = selected_result
                                            .and_then(|index| self.runner_results.get(index))
                                        {
                                            copy_summary_requested =
                                                Some(runner_detail::summary_text(row));
                                        }
                                    }
                                },
                            );
                        });
                        if !self.runner_status.is_empty() {
                            ui.add_space(3.0);
                            ui.label(
                                egui::RichText::new(&self.runner_status)
                                    .size(11.0)
                                    .color(muted()),
                            );
                        }
                        ui.add_space(8.0);

                        egui::ScrollArea::both()
                            .id_salt("runner_results_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("runner_results_grid")
                                    .striped(true)
                                    .min_col_width(72.0)
                                    .spacing(egui::vec2(12.0, 6.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Collection")
                                                .strong()
                                                .color(muted()),
                                        );
                                        ui.label(
                                            egui::RichText::new("Request").strong().color(muted()),
                                        );
                                        ui.label(
                                            egui::RichText::new("Method").strong().color(muted()),
                                        );
                                        ui.label(
                                            egui::RichText::new("Status").strong().color(muted()),
                                        );
                                        ui.label(
                                            egui::RichText::new("Time").strong().color(muted()),
                                        );
                                        ui.label(
                                            egui::RichText::new("Note").strong().color(muted()),
                                        );
                                        ui.end_row();

                                        if self.runner_results.is_empty() {
                                            ui.label(egui::RichText::new("No runs yet").color(muted()));
                                            ui.label("");
                                            ui.label("");
                                            ui.label("");
                                            ui.label("");
                                            ui.label("");
                                            ui.end_row();
                                        } else {
                                            for (index, row) in self.runner_results.iter().enumerate()
                                            {
                                                let selected = selected_result == Some(index);
                                                let mut choose = false;
                                                choose |= runner_result_cell(
                                                    ui,
                                                    selected,
                                                    &row.collection,
                                                );
                                                choose |= runner_result_cell(
                                                    ui,
                                                    selected,
                                                    &row.request,
                                                );
                                                choose |= runner_result_cell(
                                                    ui,
                                                    selected,
                                                    &row.method.to_string(),
                                                );
                                                choose |=
                                                    runner_result_cell(ui, selected, &row.status);
                                                choose |= runner_result_cell(
                                                    ui,
                                                    selected,
                                                    &runner_detail::duration_text(row.duration_ms),
                                                );
                                                choose |=
                                                    runner_result_cell(ui, selected, &row.note);
                                                if choose {
                                                    selected_result = Some(index);
                                                }
                                                ui.end_row();
                                            }
                                        }
                                    });
                            });
                        if let Some(row) =
                            selected_result.and_then(|index| self.runner_results.get(index))
                        {
                            ui.add_space(10.0);
                            render_runner_detail_panel(ui, row);
                        }
                    });
                });
            });

        self.show_runner_modal = open;
        self.runner_selected_result = selected_result;

        if save_preset_requested {
            self.save_current_runner_preset();
        }
        if let Some(id) = load_preset_id {
            self.load_runner_preset(&id);
        }
        if let Some(id) = rename_preset_id {
            self.rename_runner_preset(&id);
        }
        if let Some(id) = duplicate_preset_id {
            self.duplicate_runner_preset(&id);
        }
        if let Some(id) = delete_preset_id {
            self.delete_runner_preset(&id);
        }
        if run_requested {
            self.start_collection_runner();
        }
        if cancel_requested {
            if let Some(run) = self.runner_in_flight.take() {
                run.handle.abort();
                self.runner_status = "Runner cancelled.".to_string();
            }
        }
        if export_csv_requested {
            self.export_runner_report("csv");
        }
        if export_html_requested {
            self.export_runner_report("html");
        }
        if let Some(summary) = copy_summary_requested {
            ctx.copy_text(summary);
            self.show_toast("Runner summary copied");
        }
    }

    fn save_current_runner_preset(&mut self) {
        let name = trimmed_or_default(
            &self.runner_preset_name_input,
            format!("Runner Preset {}", self.state.runner_presets.len() + 1),
        );
        let preset = self.current_runner_preset(name);
        self.runner_selected_preset_id = Some(preset.id.clone());
        self.runner_preset_rename_input = preset.name.clone();
        self.runner_preset_name_input.clear();
        self.state.runner_presets.push(preset);
        self.save_state();
        self.show_toast("Runner preset saved");
    }

    fn load_runner_preset(&mut self, preset_id: &str) {
        let Some(preset) = self
            .state
            .runner_presets
            .iter()
            .find(|p| p.id == preset_id)
            .cloned()
        else {
            return;
        };

        let mut notes = Vec::new();
        match &preset.scope {
            RunnerPresetScope::All => {
                self.runner_scope_folder_id = None;
            }
            RunnerPresetScope::Folder {
                folder_id,
                folder_name,
            } => {
                if crate::find_folder_by_id(&self.state.folders, folder_id).is_some() {
                    self.runner_scope_folder_id = Some(folder_id.clone());
                } else {
                    self.runner_scope_folder_id = None;
                    let label = if folder_name.is_empty() {
                        "saved folder".to_string()
                    } else {
                        format!("'{}'", folder_name)
                    };
                    notes.push(format!(
                        "folder scope {} was not found; using all collections",
                        label
                    ));
                }
            }
        }

        if let Some(env_id) = &preset.env_id {
            if self.state.environments.iter().any(|env| &env.id == env_id) {
                self.state.active_env_id = Some(env_id.clone());
            } else {
                self.state.active_env_id = None;
                let label = if preset.env_name.is_empty() {
                    "saved environment".to_string()
                } else {
                    format!("'{}'", preset.env_name)
                };
                notes.push(format!(
                    "environment {} was not found; using no environment",
                    label
                ));
            }
        } else {
            self.state.active_env_id = None;
        }

        self.runner_data_rows = preset.data_rows.clone();
        self.runner_selected_preset_id = Some(preset.id.clone());
        self.runner_preset_rename_input = preset.name.clone();
        self.runner_status = if notes.is_empty() {
            format!("Loaded runner preset '{}'.", preset.name)
        } else {
            format!(
                "Loaded runner preset '{}': {}.",
                preset.name,
                notes.join("; ")
            )
        };
        self.save_state();
    }

    fn rename_runner_preset(&mut self, preset_id: &str) {
        let name = self.runner_preset_rename_input.trim();
        if name.is_empty() {
            self.runner_status = "Preset name cannot be empty.".to_string();
            return;
        }
        let Some(preset) = self
            .state
            .runner_presets
            .iter_mut()
            .find(|p| p.id == preset_id)
        else {
            return;
        };
        preset.name = name.to_string();
        self.save_state();
        self.show_toast("Runner preset renamed");
    }

    fn duplicate_runner_preset(&mut self, preset_id: &str) {
        let Some(mut preset) = self
            .state
            .runner_presets
            .iter()
            .find(|p| p.id == preset_id)
            .cloned()
        else {
            return;
        };
        preset.id = Uuid::new_v4().to_string();
        preset.name = unique_runner_preset_name(&self.state.runner_presets, &preset.name);
        self.runner_selected_preset_id = Some(preset.id.clone());
        self.runner_preset_rename_input = preset.name.clone();
        self.state.runner_presets.push(preset);
        self.save_state();
        self.show_toast("Runner preset duplicated");
    }

    fn delete_runner_preset(&mut self, preset_id: &str) {
        let before = self.state.runner_presets.len();
        self.state
            .runner_presets
            .retain(|preset| preset.id != preset_id);
        if self.state.runner_presets.len() == before {
            return;
        }
        if self.runner_selected_preset_id.as_deref() == Some(preset_id) {
            self.runner_selected_preset_id = None;
            self.runner_preset_rename_input.clear();
        }
        self.save_state();
        self.show_toast("Runner preset deleted");
    }

    fn current_runner_preset(&self, name: String) -> RunnerPreset {
        let scope = match &self.runner_scope_folder_id {
            Some(folder_id) => RunnerPresetScope::Folder {
                folder_id: folder_id.clone(),
                folder_name: crate::find_folder_by_id(&self.state.folders, folder_id)
                    .map(|folder| folder.name.clone())
                    .unwrap_or_default(),
            },
            None => RunnerPresetScope::All,
        };
        let env = self.active_environment();
        RunnerPreset {
            id: Uuid::new_v4().to_string(),
            name,
            scope,
            env_id: env.map(|env| env.id.clone()),
            env_name: env.map(|env| env.name.clone()).unwrap_or_default(),
            data_rows: self.runner_data_rows.clone(),
        }
    }

    fn export_runner_report(&mut self, format: &str) {
        if self.runner_results.is_empty() {
            return;
        }
        let filename = sanitize_filename("collection-run-report")
            .unwrap_or_else(|| "collection-run-report".to_string());
        let path = rfd::FileDialog::new()
            .add_filter(format.to_ascii_uppercase(), &[format])
            .set_file_name(format!("{}.{}", filename, format))
            .save_file();
        let Some(path) = path else { return };
        let content = match format {
            "html" => build_runner_report_html(&self.runner_results),
            _ => build_runner_report_csv(&self.runner_results),
        };
        match std::fs::write(&path, content) {
            Ok(_) => self.show_toast("Runner report exported"),
            Err(e) => self.show_toast(format!("Report export failed: {}", e)),
        }
    }

    fn start_collection_runner(&mut self) {
        self.commit_editing();
        let data_rows = match parse_runner_data_rows(&self.runner_data_rows) {
            Ok(rows) => rows,
            Err(err) => {
                self.runner_status = format!("Data rows error: {:?}", err);
                return;
            }
        };
        if self.runner_in_flight.is_some() {
            self.runner_status = "Runner is already running.".to_string();
            return;
        }
        let scope_id = self.runner_scope_folder_id.clone();
        let request_count =
            runner::collect_requests(&self.state.folders, scope_id.as_deref()).len();
        if request_count == 0 {
            self.runner_status = "No requests to run.".to_string();
            return;
        }
        let iteration_count = data_rows.len().max(1);
        let folders = self.state.folders.clone();
        let settings = self.state.settings.clone();
        let env = self.active_environment().cloned();
        let options = runner::RunnerOptions {
            folder_id: scope_id,
            data_rows,
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = self.http_runtime.spawn(async move {
            runner::run_collection_with_progress(&folders, options, &settings, env, tx).await;
        });
        self.runner_in_flight = Some(crate::InFlightCollectionRun { handle, rx });
        self.runner_results.clear();
        self.runner_selected_result = None;
        self.runner_status = format!(
            "Running {} request{} across {} iteration{}...",
            request_count,
            if request_count == 1 { "" } else { "s" },
            iteration_count,
            if iteration_count == 1 { "" } else { "s" }
        );
    }
}

struct RunnerScopeOption {
    id: String,
    name: String,
    depth: usize,
    request_count: usize,
}

fn count_runner_requests(folders: &[Folder], folder_id: Option<&str>) -> usize {
    runner::collect_requests(folders, folder_id).len()
}

fn runner_scope_options(folders: &[Folder]) -> Vec<RunnerScopeOption> {
    let mut options = Vec::new();
    for folder in folders {
        collect_runner_scope_options(folder, 0, &mut options);
    }
    options
}

fn collect_runner_scope_options(
    folder: &Folder,
    depth: usize,
    options: &mut Vec<RunnerScopeOption>,
) {
    let request_count =
        runner::collect_requests(std::slice::from_ref(folder), Some(&folder.id)).len();
    options.push(RunnerScopeOption {
        id: folder.id.clone(),
        name: folder.name.clone(),
        depth,
        request_count,
    });
    for child in &folder.subfolders {
        collect_runner_scope_options(child, depth + 1, options);
    }
}

fn runner_data_row_count(data_rows: &str) -> usize {
    parse_runner_data_rows(data_rows)
        .map(|rows| rows.len())
        .unwrap_or(0)
}

fn parse_runner_data_rows(data_rows: &str) -> Result<Vec<runner::DataRow>, runner::DataParseError> {
    let format = if data_rows.trim_start().starts_with(['[', '{']) {
        "json"
    } else {
        "csv"
    };
    runner::parse_data_rows(data_rows, format)
}

fn runner_data_rows_label(data_rows: &str) -> String {
    let count = runner_data_row_count(data_rows);
    if count == 0 {
        "Optional CSV or JSON rows".to_string()
    } else {
        format!("{} data row{}", count, if count == 1 { "" } else { "s" })
    }
}

fn runner_preset_summary(preset: &RunnerPreset) -> String {
    let scope = match &preset.scope {
        RunnerPresetScope::All => "all collections".to_string(),
        RunnerPresetScope::Folder {
            folder_name,
            folder_id,
        } => {
            if folder_name.is_empty() {
                format!("folder {}", folder_id)
            } else {
                format!("folder '{}'", folder_name)
            }
        }
    };
    let env = preset
        .env_id
        .as_ref()
        .map(|_| {
            if preset.env_name.is_empty() {
                "saved environment".to_string()
            } else {
                format!("environment '{}'", preset.env_name)
            }
        })
        .unwrap_or_else(|| "no environment".to_string());
    let rows = runner_data_rows_label(&preset.data_rows);
    format!("Loads {}, {}, {}", scope, env, rows.to_ascii_lowercase())
}

fn trimmed_or_default(value: &str, default: String) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default
    } else {
        trimmed.to_string()
    }
}

fn unique_runner_preset_name(presets: &[RunnerPreset], base: &str) -> String {
    let base = base.trim();
    let base = if base.is_empty() {
        "Runner Preset"
    } else {
        base
    };
    let mut candidate = format!("{} Copy", base);
    let mut suffix = 2usize;
    while presets.iter().any(|preset| preset.name == candidate) {
        candidate = format!("{} Copy {}", base, suffix);
        suffix += 1;
    }
    candidate
}

fn build_runner_report_csv(rows: &[RunnerResultRow]) -> String {
    let mut out = String::from("collection,request,method,url,status,duration_ms,note\n");
    for row in rows {
        out.push_str(&csv_cell(&row.collection));
        out.push(',');
        out.push_str(&csv_cell(&row.request));
        out.push(',');
        out.push_str(&csv_cell(&row.method.to_string()));
        out.push(',');
        out.push_str(&csv_cell(&row.url));
        out.push(',');
        out.push_str(&csv_cell(&row.status));
        out.push(',');
        if let Some(ms) = row.duration_ms {
            out.push_str(&ms.to_string());
        }
        out.push(',');
        out.push_str(&csv_cell(&row.note));
        out.push('\n');
    }
    out
}

fn build_runner_report_html(rows: &[RunnerResultRow]) -> String {
    let mut out = String::from(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Collection Runner Report</title>\
         <style>body{font-family:-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;margin:32px;color:#1f2937}\
         table{border-collapse:collapse;width:100%;font-size:13px}th,td{border:1px solid #d8dee8;padding:8px;text-align:left;vertical-align:top}\
         th{background:#f3f6fa}.pass{color:#18864b}.fail{color:#b42318}</style></head><body>\
         <h1>Collection Runner Report</h1><table><thead><tr>\
         <th>Collection</th><th>Request</th><th>Method</th><th>URL</th><th>Status</th><th>Time</th><th>Note</th>\
         </tr></thead><tbody>",
    );
    for row in rows {
        let status_class = if row.status.starts_with('2') || row.status.starts_with('3') {
            "pass"
        } else {
            "fail"
        };
        out.push_str("<tr><td>");
        out.push_str(&crate::privacy::escape_html(&row.collection));
        out.push_str("</td><td>");
        out.push_str(&crate::privacy::escape_html(&row.request));
        out.push_str("</td><td>");
        out.push_str(&crate::privacy::escape_html(&row.method.to_string()));
        out.push_str("</td><td>");
        out.push_str(&crate::privacy::escape_html(&row.url));
        out.push_str("</td><td class=\"");
        out.push_str(status_class);
        out.push_str("\">");
        out.push_str(&crate::privacy::escape_html(&row.status));
        out.push_str("</td><td>");
        out.push_str(
            &row.duration_ms
                .map(|ms| format!("{} ms", ms))
                .unwrap_or_else(|| "-".to_string()),
        );
        out.push_str("</td><td>");
        out.push_str(&crate::privacy::escape_html(&row.note));
        out.push_str("</td></tr>");
    }
    out.push_str("</tbody></table></body></html>");
    out
}

fn runner_result_cell(ui: &mut egui::Ui, selected: bool, value: &str) -> bool {
    let resp = ui
        .selectable_label(selected, egui::RichText::new(value).color(text()))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let keyboard_select = resp.has_focus()
        && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    resp.clicked() || keyboard_select
}

fn render_runner_detail_panel(ui: &mut egui::Ui, row: &RunnerResultRow) {
    egui::Frame::none()
        .fill(elevated())
        .stroke(egui::Stroke::new(1.0, border()))
        .rounding(6.0)
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Result Detail")
                        .size(11.0)
                        .strong()
                        .color(muted()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("body, headers, cookies, and extracted values hidden")
                            .size(11.0)
                            .color(muted()),
                    );
                });
            });
            ui.add_space(8.0);

            egui::Grid::new("runner_result_detail_grid")
                .num_columns(2)
                .spacing(egui::vec2(12.0, 4.0))
                .show(ui, |ui| {
                    runner_detail_kv(ui, "Method", &row.method.to_string());
                    runner_detail_kv(ui, "URL", &row.url);
                    runner_detail_kv(ui, "Collection", &row.collection);
                    runner_detail_kv(ui, "Data row", &row.row_label);
                    runner_detail_kv(ui, "Request", &row.request_name);
                    runner_detail_kv(ui, "Status", &row.status);
                    runner_detail_kv(
                        ui,
                        "Timing",
                        &format!(
                            "{} ms total, {} ms prepare, {} ms waiting, {} ms download",
                            row.timing.total_ms,
                            row.timing.prepare_ms,
                            row.timing.waiting_ms,
                            row.timing.download_ms
                        ),
                    );
                    runner_detail_kv(
                        ui,
                        "Extracted",
                        &format!("{} value(s) hidden", row.extracted_count),
                    );
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(260.0);
                    ui.label(
                        egui::RichText::new("Assertions")
                            .size(11.0)
                            .strong()
                            .color(muted()),
                    );
                    ui.add_space(4.0);
                    if row.assertions.is_empty() {
                        ui.label(egui::RichText::new("No assertions ran").color(muted()));
                    } else {
                        for assertion in &row.assertions {
                            let (label, color) = match assertion.outcome {
                                runner_detail::RunnerAssertionOutcome::Pass => ("PASS", C_GREEN),
                                runner_detail::RunnerAssertionOutcome::Fail => ("FAIL", C_RED),
                                runner_detail::RunnerAssertionOutcome::Error => ("ERROR", C_ORANGE),
                            };
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(label)
                                        .font(egui::FontId::monospace(11.0))
                                        .strong()
                                        .color(color),
                                );
                                ui.label(
                                    egui::RichText::new(format!("#{}", assertion.index + 1))
                                        .font(egui::FontId::monospace(11.0))
                                        .color(muted()),
                                );
                                if let Some(message) = &assertion.message {
                                    ui.label(
                                        egui::RichText::new(message)
                                            .font(egui::FontId::monospace(11.0))
                                            .color(text()),
                                    );
                                }
                            });
                        }
                    }
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Extractor Misses")
                            .size(11.0)
                            .strong()
                            .color(muted()),
                    );
                    ui.add_space(4.0);
                    if row.extractor_misses.is_empty() {
                        ui.label(egui::RichText::new("No extractor misses").color(muted()));
                    } else {
                        for miss in &row.extractor_misses {
                            ui.label(
                                egui::RichText::new(miss)
                                    .font(egui::FontId::monospace(11.0))
                                    .color(text()),
                            );
                        }
                    }
                });
            });
        });
}

fn runner_detail_kv(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).size(11.0).color(muted()));
    ui.label(
        egui::RichText::new(if value.is_empty() { "-" } else { value })
            .size(12.0)
            .color(text()),
    );
    ui.end_row();
}

fn csv_cell(value: &str) -> String {
    let safe = spreadsheet_safe_cell(value);
    if safe.contains(',') || safe.contains('"') || safe.contains('\n') {
        format!("\"{}\"", safe.replace('"', "\"\""))
    } else {
        safe
    }
}

fn spreadsheet_safe_cell(value: &str) -> String {
    match value.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{}", value),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod runner_report_tests {
    use super::*;

    #[test]
    fn csv_cell_neutralizes_spreadsheet_formulas() {
        assert_eq!(csv_cell("=cmd|' /C calc'!A0"), "'=cmd|' /C calc'!A0");
        assert_eq!(csv_cell("+SUM(A1:A2)"), "'+SUM(A1:A2)");
        assert_eq!(csv_cell("@user"), "'@user");
    }
}
