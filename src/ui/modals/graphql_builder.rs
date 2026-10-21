use super::*;
use crate::graphql::{self, OperationKind};

impl ApiClient {
    pub(crate) fn render_graphql_builder_modal(&mut self, ctx: &egui::Context) {
        if !self.show_graphql_builder {
            return;
        }
        let mut open = self.show_graphql_builder;
        let mut do_parse = false;
        let mut load_file = false;
        let mut chosen: Option<(OperationKind, String)> = None;

        egui::Window::new("GraphQL schema builder")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(620.0)
            .default_height(520.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Paste schema SDL or introspection JSON, then pick a field to build an operation.",
                    )
                    .color(muted())
                    .size(12.0),
                );
                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.graphql_schema_text)
                        .code_editor()
                        .desired_rows(8)
                        .desired_width(f32::INFINITY)
                        .hint_text(hint("type Query { user(id: ID!): User }")),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Parse schema")
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .fill(accent())
                            .min_size(egui::vec2(110.0, 28.0)),
                        )
                        .clicked()
                    {
                        do_parse = true;
                    }
                    if ui.button("Load from file…").clicked() {
                        load_file = true;
                    }
                });

                if !self.graphql_builder_error.is_empty() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&self.graphql_builder_error).color(C_RED));
                }

                if let Some(schema) = self.graphql_schema.as_ref() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Filter").size(11.0).color(muted()));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.graphql_builder_filter)
                                .desired_width(220.0)
                                .hint_text(hint("field name")),
                        );
                    });
                    ui.add_space(4.0);

                    let filter = self.graphql_builder_filter.to_lowercase();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (kind, title) in [
                            (OperationKind::Query, "Query"),
                            (OperationKind::Mutation, "Mutation"),
                        ] {
                            let fields: Vec<String> = schema
                                .root_fields(kind)
                                .iter()
                                .map(|f| f.name.clone())
                                .filter(|n| filter.is_empty() || n.to_lowercase().contains(&filter))
                                .collect();
                            if fields.is_empty() {
                                continue;
                            }
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(title).size(11.0).strong().color(text()));
                            for name in fields {
                                if menu_item(ui, egui_phosphor::regular::PLUS, &name).clicked() {
                                    chosen = Some((kind, name.clone()));
                                }
                            }
                        }
                    });
                }
            });
        self.show_graphql_builder = open;

        if load_file {
            self.pending_graphql_schema_load = true;
        }

        if do_parse {
            match parse_schema_text(&self.graphql_schema_text) {
                Ok(schema) => {
                    self.graphql_schema = Some(schema);
                    self.graphql_builder_error.clear();
                }
                Err(e) => {
                    self.graphql_schema = None;
                    self.graphql_builder_error = e;
                }
            }
        }

        if let Some((kind, name)) = chosen {
            let op = self
                .graphql_schema
                .as_ref()
                .and_then(|s| s.build_operation(kind, &name));
            if let Some(op) = op {
                self.editing_body = op.query.clone();
                self.editing_body_ext = Some(BodyExt::GraphQL {
                    variables: op.variables.clone(),
                });
                let body = op.query;
                let vars = op.variables;
                self.update_current_request(|r| {
                    r.body = body;
                    r.body_ext = Some(BodyExt::GraphQL { variables: vars });
                });
                self.show_graphql_builder = false;
                self.show_toast(format!("Inserted {} operation", name));
            }
        }
    }
}

/// Heuristic: an introspection result starts with `{` (JSON); anything
/// else is treated as SDL.
fn parse_schema_text(text: &str) -> Result<graphql::Schema, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Paste a schema first.".to_string());
    }
    if trimmed.starts_with('{') {
        graphql::parse_introspection(trimmed)
    } else {
        graphql::parse_sdl(trimmed)
    }
}
