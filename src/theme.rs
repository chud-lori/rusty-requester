use crate::model::HttpMethod;
use eframe::egui;

// ====== Dark palette, rust only as accent ======
// Neutral near-black surfaces (no warm tint across the chrome), with
// rust orange reserved for the interactive accent — selected tab
// underline, "New Collection" button, Send button, focus ring, etc.
// Keep sidebar and central panel on the *same* base so there's no
// visible seam between them. `C_PANEL_DARK` stays a shade lower for
// sunken surfaces (response body frame, code editors).
pub const C_BG: egui::Color32 = egui::Color32::from_rgb(22, 24, 29); // #16181D app bg
pub const C_PANEL: egui::Color32 = egui::Color32::from_rgb(22, 24, 29); // == C_BG, on purpose
pub const C_PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(15, 16, 20); // #0F1014 sunken
pub const C_ELEVATED: egui::Color32 = egui::Color32::from_rgb(42, 45, 52); // #2A2D34 hover / active row
pub const C_BORDER: egui::Color32 = egui::Color32::from_rgb(52, 55, 63); // #34373F subtle divider
pub const C_ACCENT: egui::Color32 = egui::Color32::from_rgb(206, 66, 43); // #CE422B rust orange — THE accent
pub const C_PURPLE: egui::Color32 = egui::Color32::from_rgb(186, 120, 80); // #BA7850 burnt sienna — PATCH
pub const C_GREEN: egui::Color32 = egui::Color32::from_rgb(134, 172, 113); // #86AC71 patina green — GET
pub const C_ORANGE: egui::Color32 = egui::Color32::from_rgb(245, 158, 11); // #F59E0B amber — POST
pub const C_PINK: egui::Color32 = egui::Color32::from_rgb(183, 65, 14); // #B7410E deep rust — PUT
pub const C_RED: egui::Color32 = egui::Color32::from_rgb(220, 38, 38); // #DC2626 crimson — DELETE / errors
pub const C_MUTED: egui::Color32 = egui::Color32::from_rgb(126, 131, 145); // #7E8391 neutral muted text
pub const C_TEXT: egui::Color32 = egui::Color32::from_rgb(224, 226, 232); // #E0E2E8 neutral light
pub const C_HINT: egui::Color32 = egui::Color32::from_rgb(80, 84, 95); // #50545F dim placeholder

/// Styled placeholder text for TextEdit hint_text — dim color so
/// it's unambiguously a placeholder, not a real value.
pub fn hint(text: &str) -> egui::RichText {
    egui::RichText::new(text).color(C_HINT)
}

pub fn method_color(m: &HttpMethod) -> egui::Color32 {
    match m {
        HttpMethod::GET => C_GREEN,    // patina green — safe read
        HttpMethod::POST => C_ORANGE,  // amber gold — create
        HttpMethod::PUT => C_PINK,     // deep rust — update
        HttpMethod::DELETE => C_RED,   // crimson — destructive
        HttpMethod::PATCH => C_PURPLE, // burnt sienna — partial
        _ => C_MUTED,                  // warm grey — HEAD / OPTIONS
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
    // Every background-colour field egui exposes, coerced into our palette.
    // Leaving any of these on their dark-default can surface as pure-black
    // strips behind scroll bars, code editors, or grid zebra stripes.
    style.visuals.extreme_bg_color = C_PANEL_DARK;
    style.visuals.faint_bg_color = C_ELEVATED;
    style.visuals.code_bg_color = C_PANEL_DARK;
    style.visuals.override_text_color = Some(C_TEXT);
    style.visuals.selection.bg_fill = C_ACCENT.gamma_multiply(0.4);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, C_ACCENT);
    style.visuals.hyperlink_color = C_ACCENT;
    style.visuals.widgets.noninteractive.bg_fill = C_BG;
    style.visuals.widgets.noninteractive.weak_bg_fill = C_BG;
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, C_BORDER);
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, C_TEXT);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.inactive.bg_fill = C_ELEVATED;
    style.visuals.widgets.inactive.weak_bg_fill = C_ELEVATED;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.hovered.bg_fill = C_BORDER;
    style.visuals.widgets.hovered.weak_bg_fill = C_BORDER;
    // No hover stroke — a 1px stroke on hover was causing the sidebar
    // right-edge (SidePanel resize zone) to cascade 1-pixel layout shifts
    // as the pointer moved, making the whole panel visibly "jitter".
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
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

    // Slightly longer animations — softer fades on hover, panel collapse,
    // tab switches, etc. egui default is ~0.083s (snappy but abrupt).
    style.animation_time = 0.18;

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.menu_margin = egui::Margin::same(6.0);
    // Floating scrollbar — overlays content semi-transparently, so:
    //   1. It doesn't reserve its own width column (prevents the sidebar
    //      "auto-resizing" when the pointer enters its scroll area and
    //      the `VisibleWhenNeeded` scrollbar fades in).
    //   2. There's no visible darker "track" strip between the panels
    //      (the track was using `extreme_bg_color`).
    // Thin floating scrollbar — floats over content so it doesn't
    // reserve its own width column (otherwise toggling visibility
    // shifts layout). ~4px bar matches Postman/VS Code aesthetics.
    style.spacing.scroll.bar_width = 4.0;
    style.spacing.scroll.floating = true;
    style.spacing.scroll.handle_min_length = 20.0;
    style.spacing.scroll.bar_inner_margin = 2.0;
    style.spacing.scroll.bar_outer_margin = 0.0;

    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(20.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        ),
    ]
    .into();

    ctx.set_style(style);

    // Phosphor icon font — register once so icon glyphs resolve
    // inside any Proportional text (labels, buttons, RichText).
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}
