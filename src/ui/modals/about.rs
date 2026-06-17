use super::*;

impl ApiClient {
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
                .color(muted()),
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
                    .link(egui::RichText::new(label).size(12.5).color(accent()))
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
                        .color(text()),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(concat!("Version ", env!("CARGO_PKG_VERSION")))
                        .size(12.0)
                        .color(text()),
                );
                ui.label(
                    egui::RichText::new(concat!("Build: ", env!("CARGO_PKG_VERSION"), " (native)"))
                        .size(11.5)
                        .color(muted()),
                );

                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("A native, offline, lightweight API client.")
                        .size(12.0)
                        .color(text()),
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
                        .color(muted()),
                );
                ui.add_space(10.0);
            });
        });
        if !open {
            self.show_about_modal = false;
        }
    }
}
