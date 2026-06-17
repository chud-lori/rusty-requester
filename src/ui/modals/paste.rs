use super::*;

impl ApiClient {
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
                        .color(muted())
                        .size(12.0),
                );
                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.paste_curl_text)
                        .code_editor()
                        .desired_rows(10)
                        .desired_width(f32::INFINITY)
                        .hint_text(hint("curl -X POST 'https://api.example.com' -H 'Content-Type: application/json' -d '{\"k\":\"v\"}'")),
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
}
