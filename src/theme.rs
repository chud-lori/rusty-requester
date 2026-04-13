use crate::model::HttpMethod;
use eframe::egui;

// ====== Rust-forge palette ======
// Warm, oxidised-metal tones that match the "Rusty" brand:
// dark warm browns for surfaces, rust orange for the primary accent,
// copper / amber / patina-green for method badges.
pub const C_BG: egui::Color32 = egui::Color32::from_rgb(24, 15, 10);        // #180F0A deep warm
pub const C_PANEL: egui::Color32 = egui::Color32::from_rgb(40, 26, 18);     // #281A12 warm dark panel
pub const C_PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(16, 10, 7); // #100A07 deepest
pub const C_ELEVATED: egui::Color32 = egui::Color32::from_rgb(58, 39, 27);  // #3A271B hover / tab row
pub const C_BORDER: egui::Color32 = egui::Color32::from_rgb(82, 57, 40);    // #523928 warm border
pub const C_ACCENT: egui::Color32 = egui::Color32::from_rgb(206, 66, 43);   // #CE422B rust orange (brand)
pub const C_PURPLE: egui::Color32 = egui::Color32::from_rgb(186, 120, 80);  // #BA7850 burnt sienna — PATCH
pub const C_GREEN: egui::Color32 = egui::Color32::from_rgb(134, 172, 113); // #86AC71 patina green — GET
pub const C_ORANGE: egui::Color32 = egui::Color32::from_rgb(245, 158, 11); // #F59E0B amber gold — POST
pub const C_PINK: egui::Color32 = egui::Color32::from_rgb(183, 65, 14);    // #B7410E deep rust — PUT
pub const C_RED: egui::Color32 = egui::Color32::from_rgb(220, 38, 38);     // #DC2626 crimson — DELETE / errors
pub const C_MUTED: egui::Color32 = egui::Color32::from_rgb(138, 115, 95);  // #8A735F warm muted text
pub const C_TEXT: egui::Color32 = egui::Color32::from_rgb(245, 230, 208); // #F5E6D0 warm cream

pub fn pill_text_color(bg: egui::Color32) -> egui::Color32 {
    let r = bg.r() as f32 / 255.0;
    let g = bg.g() as f32 / 255.0;
    let b = bg.b() as f32 / 255.0;
    let luma = 0.299 * r + 0.587 * g + 0.114 * b;
    if luma > 0.55 {
        egui::Color32::from_rgb(15, 17, 26)
    } else {
        egui::Color32::WHITE
    }
}

pub fn method_color(m: &HttpMethod) -> egui::Color32 {
    match m {
        HttpMethod::GET => C_GREEN,   // patina green — safe read
        HttpMethod::POST => C_ORANGE, // amber gold — create
        HttpMethod::PUT => C_PINK,    // deep rust — update
        HttpMethod::DELETE => C_RED,  // crimson — destructive
        HttpMethod::PATCH => C_PURPLE, // burnt sienna — partial
        _ => C_MUTED,                 // warm grey — HEAD / OPTIONS
    }
}

pub fn status_color(status: &str) -> egui::Color32 {
    if status.starts_with('2') {
        C_GREEN
    } else if status.starts_with('3') {
        C_ORANGE
    } else if status.starts_with('4') || status.starts_with('5') {
        C_RED
    } else {
        C_MUTED
    }
}

pub fn apply_style(ctx: &egui::Context) {
    use egui::{FontFamily, FontId, TextStyle};
    let mut style = (*ctx.style()).clone();
    style.visuals.window_fill = C_BG;
    style.visuals.panel_fill = C_BG;
    style.visuals.extreme_bg_color = C_PANEL_DARK;
    style.visuals.override_text_color = Some(C_TEXT);
    style.visuals.selection.bg_fill = C_ACCENT.gamma_multiply(0.4);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, C_ACCENT);
    style.visuals.hyperlink_color = C_ACCENT;
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, C_BORDER);
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, C_TEXT);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.inactive.bg_fill = C_ELEVATED;
    style.visuals.widgets.inactive.weak_bg_fill = C_ELEVATED;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.hovered.bg_fill = C_BORDER;
    style.visuals.widgets.hovered.weak_bg_fill = C_BORDER;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, C_ACCENT.gamma_multiply(0.4));
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.active.bg_fill = C_BORDER;
    style.visuals.widgets.active.weak_bg_fill = C_BORDER;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, C_ACCENT);
    style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.open.bg_fill = C_BORDER;
    style.visuals.widgets.open.rounding = egui::Rounding::same(8.0);
    style.visuals.menu_rounding = egui::Rounding::same(10.0);
    style.visuals.window_rounding = egui::Rounding::same(12.0);
    style.visuals.window_stroke = egui::Stroke::new(1.0, C_BORDER);

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.menu_margin = egui::Margin::same(6.0);
    style.spacing.scroll.bar_width = 10.0;
    style.spacing.scroll.floating = false;

    style.text_styles = [
        (TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
    ]
    .into();

    ctx.set_style(style);
}
