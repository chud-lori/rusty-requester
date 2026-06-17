use super::*;

impl ApiClient {
    pub(crate) fn render_backup_modal(&mut self, ctx: &egui::Context) {
        if !self.show_backup_modal {
            return;
        }

        let backup_dir = backup::backup_dir_for(&self.storage_path);
        let backups = backup::list_backups(&self.storage_path);
        let mut open = self.show_backup_modal;
        let mut create_backup = false;
        let mut restore_path: Option<PathBuf> = None;
        let mut cancel_restore = false;

        egui::Window::new("Workspace Backups")
            .collapsible(false)
            .resizable(true)
            .default_width(680.0)
            .min_width(520.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Workspace Backups").size(16.0).strong());
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Location").color(muted()));
                        ui.monospace(backup_dir.display().to_string());
                    });
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("Create backup now").clicked() {
                            create_backup = true;
                        }
                        ui.label(
                            egui::RichText::new("Restore creates a current-state backup first.")
                                .color(muted()),
                        );
                    });
                    ui.separator();

                    match backups {
                        Ok(backups) if backups.is_empty() => {
                            ui.add_space(16.0);
                            ui.centered_and_justified(|ui| {
                                ui.label(egui::RichText::new("No backups yet").color(muted()));
                            });
                        }
                        Ok(backups) => {
                            egui::ScrollArea::vertical()
                                .max_height(360.0)
                                .show(ui, |ui| {
                                    for entry in backups {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&entry.file_name).strong(),
                                                );
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(format_backup_time(
                                                            entry.created_at,
                                                        ))
                                                        .color(muted()),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(format_file_size(
                                                            entry.size_bytes,
                                                        ))
                                                        .color(muted()),
                                                    );
                                                });
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.button("Restore").clicked() {
                                                        self.confirm_restore_backup_path =
                                                            Some(entry.path.clone());
                                                    }
                                                },
                                            );
                                        });
                                        ui.separator();
                                    }
                                });
                        }
                        Err(e) => {
                            ui.label(
                                egui::RichText::new(format!("Could not read backups: {}", e))
                                    .color(C_RED),
                            );
                        }
                    }

                    if let Some(path) = self.confirm_restore_backup_path.clone() {
                        ui.add_space(8.0);
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Restore this backup?").strong());
                            ui.label(
                                egui::RichText::new(
                                    path.file_name()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("selected backup"),
                                )
                                .color(muted()),
                            );
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button("Restore workspace").clicked() {
                                    restore_path = Some(path.clone());
                                }
                                if ui.button("Cancel").clicked() {
                                    cancel_restore = true;
                                }
                            });
                        });
                    }
                });
            });

        self.show_backup_modal = open;
        if !open || cancel_restore {
            self.confirm_restore_backup_path = None;
        }
        if create_backup {
            self.create_workspace_backup_now();
        }
        if let Some(path) = restore_path {
            self.restore_workspace_backup_now(&path);
        }
    }
}

fn format_backup_time(time: Option<SystemTime>) -> String {
    let Some(time) = time else {
        return "Unknown time".to_string();
    };
    let Ok(duration) = time.duration_since(UNIX_EPOCH) else {
        return "Before Unix epoch".to_string();
    };
    format!("{} seconds since epoch", duration.as_secs())
}

fn format_file_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / KB)
    } else {
        format!("{:.1} MB", bytes as f64 / MB)
    }
}
