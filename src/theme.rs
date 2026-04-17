use crate::model::{HttpMethod, Theme};
use eframe::egui;

// ====== Dark palette, rust only as accent ======
// Neutral near-black surfaces (no warm tint across the chrome), with
// rust orange reserved for the interactive accent — selected tab
// underline, "New Collection" button, Send button, focus ring, etc.
// Keep sidebar and central panel on the *same* base so there's no
// visible seam between them. `C_PANEL_DARK` stays a shade lower for
// sunken surfaces (response body frame, code editors).
pub const C_BG: egui::Color32 = egui::Color32::from_rgb(22, 24, 29); // #16181D app bg
pub const C_PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(15, 16, 20); // #0F1014 sunken
pub const C_ELEVATED: egui::Color32 = egui::Color32::from_rgb(42, 45, 52); // #2A2D34 hover / active row
pub const C_BORDER: egui::Color32 = egui::Color32::from_rgb(52, 55, 63); // #34373F subtle divider
pub const C_ACCENT: egui::Color32 = egui::Color32::from_rgb(206, 66, 43); // #CE422B rust orange — THE accent
pub const C_PURPLE: egui::Color32 = egui::Color32::from_rgb(186, 120, 80); // #BA7850 burnt sienna — PATCH
pub const C_GREEN: egui::Color32 = egui::Color32::from_rgb(134, 172, 113); // #86AC71 patina green — GET
pub const C_ORANGE: egui::Color32 = egui::Color32::from_rgb(245, 158, 11); // #F59E0B amber — POST
pub const C_PINK: egui::Color32 = egui::Color32::from_rgb(183, 65, 14); // #B7410E deep rust — PUT
pub const C_RED: egui::Color32 = egui::Color32::from_rgb(220, 38, 38); // #DC2626 crimson — DELETE / errors
pub const C_MUTED: egui::Color32 = egui::Color32::from_rgb(143, 148, 162); // #8F94A2 neutral muted text (WCAG AA ~5.3:1 on C_BG)
pub const C_TEXT: egui::Color32 = egui::Color32::from_rgb(224, 226, 232); // #E0E2E8 neutral light

/// Theme-aware placeholder color. Earlier `#50545F` was too dark on
/// the dark bg (dropped below WCAG AA ~2.5:1) and egui's default
/// `weak_text_color` landed too pale on light bg. These values hit
/// ~4.5:1 in both modes — visibly a placeholder but readable.
pub fn hint_color() -> egui::Color32 {
    if is_light() {
        egui::Color32::from_rgb(140, 148, 158) // #8C949E — medium grey on paper
    } else {
        egui::Color32::from_rgb(138, 142, 152) // #8A8E98 — lifted grey on dark
    }
}

/// Styled placeholder text for TextEdit hint_text — dim but legible
/// in either theme.
pub fn hint(text: &str) -> egui::RichText {
    egui::RichText::new(text).color(hint_color())
}

/// Light-mode versions of the method colors, picked for ~4.5:1+
/// contrast against the light palette bg (#F6F7F9). The dark-mode
/// originals are ~2–3:1 on a light surface and unreadable; these
/// darker variants stay chromatically similar (green stays green,
/// amber stays amber, etc.) but readable. Matches the pattern
/// Postman uses in its light theme.
const C_GREEN_LIGHT: egui::Color32 = egui::Color32::from_rgb(47, 110, 61); //  #2F6E3D
const C_ORANGE_LIGHT: egui::Color32 = egui::Color32::from_rgb(166, 98, 12); //  #A6620C
const C_PINK_LIGHT: egui::Color32 = egui::Color32::from_rgb(147, 55, 18); //  #933712
const C_RED_LIGHT: egui::Color32 = egui::Color32::from_rgb(168, 31, 31); //  #A81F1F
const C_PURPLE_LIGHT: egui::Color32 = egui::Color32::from_rgb(130, 82, 40); //  #825228

pub fn method_color(m: &HttpMethod) -> egui::Color32 {
    let dark_mode = matches!(ACTIVE_THEME.load(std::sync::atomic::Ordering::Relaxed), 0);
    match m {
        HttpMethod::GET => {
            if dark_mode {
                C_GREEN
            } else {
                C_GREEN_LIGHT
            }
        }
        HttpMethod::POST => {
            if dark_mode {
                C_ORANGE
            } else {
                C_ORANGE_LIGHT
            }
        }
        HttpMethod::PUT => {
            if dark_mode {
                C_PINK
            } else {
                C_PINK_LIGHT
            }
        }
        HttpMethod::DELETE => {
            if dark_mode {
                C_RED
            } else {
                C_RED_LIGHT
            }
        }
        HttpMethod::PATCH => {
            if dark_mode {
                C_PURPLE
            } else {
                C_PURPLE_LIGHT
            }
        }
        _ => muted(), // theme-aware neutral for HEAD / OPTIONS
    }
}

pub fn status_color(status: &str) -> egui::Color32 {
    let dark_mode = matches!(ACTIVE_THEME.load(std::sync::atomic::Ordering::Relaxed), 0);
    if status.starts_with('2') {
        if dark_mode {
            C_GREEN
        } else {
            C_GREEN_LIGHT
        }
    } else if status.starts_with('3') {
        if dark_mode {
            C_ORANGE
        } else {
            C_ORANGE_LIGHT
        }
    } else if status.starts_with('4') || status.starts_with('5') {
        if dark_mode {
            C_RED
        } else {
            C_RED_LIGHT
        }
    } else {
        muted()
    }
}

/// Color palette for the egui chrome (panel fills, text, borders, widget
/// backgrounds). Saturated accents (method colors, status pills,
/// `C_ACCENT`) are not in the palette — they stay constant across
/// themes because they're already tuned to read on either background.
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: egui::Color32,
    pub panel_dark: egui::Color32,
    pub elevated: egui::Color32,
    pub border: egui::Color32,
    pub text: egui::Color32,
    /// Reserved for call sites that want a theme-aware muted color.
    /// `C_MUTED` is still used directly across the codebase (it reads
    /// fine on both backgrounds); this field exists so future refactors
    /// can thread a lighter muted through light mode without changing
    /// the palette shape.
    #[allow(dead_code)]
    pub muted: egui::Color32,
}

/// Dark palette — the project's original / default. Keeps the
/// pre-light-theme values so existing screenshots stay accurate.
pub const DARK_PALETTE: Palette = Palette {
    bg: C_BG,
    panel_dark: C_PANEL_DARK,
    elevated: C_ELEVATED,
    border: C_BORDER,
    text: C_TEXT,
    muted: C_MUTED,
};

/// Light palette — tuned against Postman's light mode. Earlier
/// iterations landed too grey (`#D9DCE3`) and made every KV input
/// render as a visible grey pill — the "8-bit" look. Postman's
/// canvas is essentially white; structure comes from borders and
/// content, not from chunky input fills.
///   * `bg`         #FCFCFD — canvas (faint off-white, not clinical)
///   * `panel_dark` #F3F4F7 — sidebar / sunken (response body, code)
///   * `elevated`   #EDEFF2 — inputs / hover / active rows
///   * `border`     #D1D5DB — clear dividers (GitHub-ish)
///   * `text`       #1F2328 — body text, high contrast on bg
///   * `muted`      #656D76 — secondary labels
pub const LIGHT_PALETTE: Palette = Palette {
    bg: egui::Color32::from_rgb(252, 252, 253),
    panel_dark: egui::Color32::from_rgb(243, 244, 247),
    elevated: egui::Color32::from_rgb(237, 239, 242),
    border: egui::Color32::from_rgb(209, 213, 219),
    text: egui::Color32::from_rgb(31, 35, 40),
    muted: egui::Color32::from_rgb(101, 109, 118),
};

pub fn palette_for(theme: Theme) -> Palette {
    match theme {
        Theme::Dark => DARK_PALETTE,
        Theme::Light => LIGHT_PALETTE,
    }
}

/// Active theme tracker — updated on every `apply_style` call,
/// read by `active_palette()` so widgets can pull theme-aware
/// colors without threading a `Theme` arg through every function.
/// `0` = `Theme::Dark`, `1` = `Theme::Light` (default 0).
///
/// Why an atomic instead of passing `&Palette` everywhere: hundreds
/// of widget call sites use `C_TEXT`/`C_MUTED`/`C_ELEVATED`
/// directly. Threading theme through every render function would
/// touch every file in `src/ui/`. A global read is O(1) atomic load,
/// trivially correct, and keeps the call-site diff to one-token
/// renames (`C_TEXT` → `text()`).
static ACTIVE_THEME: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

fn set_active_theme(theme: Theme) {
    let val = match theme {
        Theme::Dark => 0u8,
        Theme::Light => 1u8,
    };
    ACTIVE_THEME.store(val, std::sync::atomic::Ordering::Relaxed);
}

pub fn active_palette() -> Palette {
    match ACTIVE_THEME.load(std::sync::atomic::Ordering::Relaxed) {
        1 => LIGHT_PALETTE,
        _ => DARK_PALETTE,
    }
}

/// `true` when light theme is active. Syntax-highlight call sites use
/// this to branch palettes (Monokai on dark, GitHub-ish on light).
pub fn is_light() -> bool {
    ACTIVE_THEME.load(std::sync::atomic::Ordering::Relaxed) == 1
}

/// Theme-aware "text" color — the replacement for `C_TEXT` at call
/// sites that should flip between themes.
pub fn text() -> egui::Color32 {
    active_palette().text
}

/// Theme-aware "muted" color — the replacement for `C_MUTED` at
/// call sites that should flip between themes.
pub fn muted() -> egui::Color32 {
    active_palette().muted
}

/// Theme-aware panel / surface color. Replaces `C_BG` at sites
/// that paint their own surface. Currently only used at a small
/// number of sites (main panel floor, central panel); the `#[allow]`
/// lets us keep the palette API shaped even when callers haven't
/// migrated yet.
#[allow(dead_code)]
pub fn bg() -> egui::Color32 {
    active_palette().bg
}

/// Theme-aware elevated-surface color (inputs, chips, hover rects).
/// Replaces `C_ELEVATED` where the darker elevated surface should
/// flip to a lighter one in light mode.
pub fn elevated() -> egui::Color32 {
    active_palette().elevated
}

/// Theme-aware border color. Replaces `C_BORDER`.
pub fn border() -> egui::Color32 {
    active_palette().border
}

/// Theme-aware sunken-surface color (code editor, response body
/// frame). Replaces `C_PANEL_DARK`.
pub fn panel_dark() -> egui::Color32 {
    active_palette().panel_dark
}

pub fn apply_style(ctx: &egui::Context, theme: Theme) {
    use egui::{FontFamily, FontId, TextStyle};
    // Record the active theme so widget code can pull theme-aware
    // colors via `active_palette()` without threading `Theme` args.
    set_active_theme(theme);
    let p = palette_for(theme);
    let mut style = (*ctx.style()).clone();
    style.visuals.window_fill = p.bg;
    style.visuals.panel_fill = p.bg;
    // Every background-colour field egui exposes, coerced into our palette.
    // Leaving any of these on their dark-default can surface as pure-black
    // strips behind scroll bars, code editors, or grid zebra stripes.
    style.visuals.extreme_bg_color = p.panel_dark;
    style.visuals.faint_bg_color = p.elevated;
    style.visuals.code_bg_color = p.panel_dark;
    style.visuals.override_text_color = Some(p.text);
    style.visuals.selection.bg_fill = C_ACCENT.gamma_multiply(0.4);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, C_ACCENT);
    style.visuals.hyperlink_color = C_ACCENT;
    style.visuals.widgets.noninteractive.bg_fill = p.bg;
    style.visuals.widgets.noninteractive.weak_bg_fill = p.bg;
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, p.text);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.inactive.bg_fill = p.elevated;
    style.visuals.widgets.inactive.weak_bg_fill = p.elevated;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.hovered.bg_fill = p.border;
    style.visuals.widgets.hovered.weak_bg_fill = p.border;
    // No hover stroke — a 1px stroke on hover was causing the sidebar
    // right-edge (SidePanel resize zone) to cascade 1-pixel layout shifts
    // as the pointer moved, making the whole panel visibly "jitter".
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.active.bg_fill = p.border;
    style.visuals.widgets.active.weak_bg_fill = p.border;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, C_ACCENT);
    style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);
    // `widgets.open.bg_fill` is what egui uses for the title-bar
    // band on open Windows. Previously set to `p.border` which made
    // the band darker than the window body — looked like a separate
    // "header section" in light mode. Keep it aligned with the
    // window body so the modal reads as a single surface.
    style.visuals.widgets.open.bg_fill = p.bg;
    style.visuals.widgets.open.rounding = egui::Rounding::same(8.0);
    style.visuals.menu_rounding = egui::Rounding::same(10.0);
    style.visuals.window_rounding = egui::Rounding::same(12.0);
    style.visuals.window_stroke = egui::Stroke::new(1.0, p.border);
    // Light theme wants dark widgets on a light panel — flip
    // `dark_mode` so egui's internal defaults pick sensible colors
    // for things we don't override (scroll thumbs, tooltips, etc.).
    style.visuals.dark_mode = matches!(theme, Theme::Dark);

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
