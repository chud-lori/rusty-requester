use super::*;

impl ApiClient {
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
                    .fill(bg())
                    .stroke(egui::Stroke::new(1.0, accent()))
                    .rounding(10.0)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.set_min_width(160.0);
                        ui.add(
                            egui::Label::new(egui::RichText::new(msg).color(text()).size(13.0))
                                .extend(),
                        );
                    });
            });
    }
}
