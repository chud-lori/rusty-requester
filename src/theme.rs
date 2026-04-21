use crate::model::{HttpMethod, Theme};
use eframe::egui;

// ====== Dark palette — "Editor Dark" ======
// Neutral warm-charcoal surfaces (no pure blacks, no blue tint) with a
// coral-red accent reserved for interactive elements — selected tab
// underline, "New Collection" button, Send button, focus ring, etc.
// Layers follow the modern "elevated = brighter" convention: bg is the
// darkest canvas, `panel_dark` is brighter (cards / response body lift
// off the canvas), `elevated` is brightest (inputs stand out).
pub const C_BG: egui::Color32 = egui::Color32::from_rgb(26, 26, 26); // #1A1A1A app canvas (dark mode)
pub const C_PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(37, 37, 37); // #252525 elevated card / sidebar
pub const C_ELEVATED: egui::Color32 = egui::Color32::from_rgb(51, 51, 51); // #333333 inputs / hover
pub const C_BORDER: egui::Color32 = egui::Color32::from_rgb(64, 64, 64); // #404040 subtle divider
pub const C_PURPLE: egui::Color32 = egui::Color32::from_rgb(186, 120, 80); // #BA7850 burnt sienna — PATCH
pub const C_GREEN: egui::Color32 = egui::Color32::from_rgb(134, 172, 113); // #86AC71 patina green — GET
pub const C_ORANGE: egui::Color32 = egui::Color32::from_rgb(245, 158, 11); // #F59E0B amber — POST
pub const C_PINK: egui::Color32 = egui::Color32::from_rgb(183, 65, 14); // #B7410E deep rust — PUT
pub const C_RED: egui::Color32 = egui::Color32::from_rgb(220, 38, 38); // #DC2626 crimson — DELETE / errors
pub const C_MUTED: egui::Color32 = egui::Color32::from_rgb(156, 163, 175); // #9CA3AF neutral muted text (WCAG AA ~5.5:1 on C_BG)
pub const C_TEXT: egui::Color32 = egui::Color32::from_rgb(243, 244, 246); // #F3F4F6 near-white, non-vibrating

// Theme-aware accent — a warm rust-red in both modes, softened from
// the original saturated `#CE422B` that was "bleeding" on dark. Dark
// value leans slightly warmer / more orange to keep the "rusty" feel
// (the first coral-red #EF5350 swing overshot into pink territory).
// Light value is deeper red for WCAG AA against white button text.
const C_ACCENT_DARK: egui::Color32 = egui::Color32::from_rgb(216, 85, 57); // #D85539 warm rust — dark-mode accent
const C_ACCENT_LIGHT: egui::Color32 = egui::Color32::from_rgb(196, 60, 40); // #C43C28 deep rust — light-mode accent
/// Postman accent — sampled directly from the current Postman web UI
/// Send button (`#3A82E6`). The app moved away from the classic brand
/// orange to this blue for primary actions; matched here so the
/// Postman theme mirrors the live product.
const C_ACCENT_POSTMAN: egui::Color32 = egui::Color32::from_rgb(58, 130, 230); // #3A82E6

pub fn accent() -> egui::Color32 {
    match current_theme() {
        Theme::Dark => C_ACCENT_DARK,
        Theme::Light => C_ACCENT_LIGHT,
        Theme::Postman => C_ACCENT_POSTMAN,
    }
}

/// Legacy constant kept as a compile-time fallback for call sites
/// that can't call a function (e.g. SVG embeds, `const` contexts).
/// Prefer `accent()` everywhere the theme can be read at render time.
pub const C_ACCENT: egui::Color32 = C_ACCENT_DARK;

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
    // Both Light and Postman render on pale surfaces and need the
    // darker, WCAG-readable variants; only the Dark theme keeps the
    // brighter/saturated originals.
    let dark_mode = matches!(current_theme(), Theme::Dark);
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
    let dark_mode = matches!(current_theme(), Theme::Dark);
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

/// Light palette — "Paper Light", tuned for visible hierarchy. The
/// previous try had the canvas near-white (`#F4F5F7`) with both
/// panels and inputs also pure white — cards bled into the
/// background, inputs looked like white pills floating on white.
///
/// Fixes:
///   - Canvas deepened to `#F0F2F5` — clearly cooler-gray, so the
///     pure-white sidebar / response panels pop as defined cards.
///   - Inputs use the "Filled" approach (`#F3F4F6`) — light gray
///     boxes on the white card, with a subtle border for structure.
///   - Secondary text darkened to `#6B7280` (medium slate) so
///     placeholders and labels no longer feel washed out.
pub const LIGHT_PALETTE: Palette = Palette {
    // #E9ECEF — deeper cool gray so the pure-white cards pop clearly
    // as defined containers. Earlier `#F0F2F5` was still too close
    // to white; panels bled into the canvas.
    bg: egui::Color32::from_rgb(233, 236, 239),
    panel_dark: egui::Color32::from_rgb(255, 255, 255),
    elevated: egui::Color32::from_rgb(243, 244, 246),
    border: egui::Color32::from_rgb(209, 213, 219),
    text: egui::Color32::from_rgb(45, 55, 72),
    muted: egui::Color32::from_rgb(107, 114, 128),
};

/// Postman palette — pure-white "main view" (central canvas for the
/// request editor and response body) lifted above a warm light-gray
/// sidebar (`#EDEDEE`). The sidebar is painted with `panel_dark`, so
/// here we intentionally invert the usual "panel_dark is brighter
/// than bg" convention: bg is the brightest layer (main content card)
/// and panel_dark is a shade darker (navbar / tool rails) to match
/// the Postman app's chrome-vs-content hierarchy.
pub const POSTMAN_PALETTE: Palette = Palette {
    bg: egui::Color32::from_rgb(255, 255, 255), // #FFFFFF main canvas
    panel_dark: egui::Color32::from_rgb(249, 249, 249), // #F9F9F9 sidebar / chrome
    elevated: egui::Color32::from_rgb(244, 244, 244), // #F4F4F4 hover / faint bg
    border: egui::Color32::from_rgb(230, 230, 230), // #E6E6E6 divider
    text: egui::Color32::from_rgb(19, 19, 19),  // #131313 near-black
    muted: egui::Color32::from_rgb(107, 107, 107), // #6B6B6B neutral gray
};

pub fn palette_for(theme: Theme) -> Palette {
    match theme {
        Theme::Dark => DARK_PALETTE,
        Theme::Light => LIGHT_PALETTE,
        Theme::Postman => POSTMAN_PALETTE,
    }
}

/// Active theme tracker — updated on every `apply_style` call,
/// read by `active_palette()` so widgets can pull theme-aware
/// colors without threading a `Theme` arg through every function.
/// `0` = `Theme::Dark`, `1` = `Theme::Light`, `2` = `Theme::Postman`
/// (default 0).
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
        Theme::Postman => 2u8,
    };
    ACTIVE_THEME.store(val, std::sync::atomic::Ordering::Relaxed);
}

/// Current active theme. Scoped to the crate because most callers want
/// a palette or a boolean test, not the enum — but we use it locally in
/// `accent()` / `method_color()` to branch on `Postman` specifically.
pub(crate) fn current_theme() -> Theme {
    match ACTIVE_THEME.load(std::sync::atomic::Ordering::Relaxed) {
        1 => Theme::Light,
        2 => Theme::Postman,
        _ => Theme::Dark,
    }
}

pub fn active_palette() -> Palette {
    palette_for(current_theme())
}

/// `true` when a light-based theme (Light or Postman) is active.
/// Syntax-highlight call sites use this to branch palettes (Monokai on
/// dark, GitHub-ish on light/Postman); widget code uses it to pick the
/// "paper" render path over the "dark canvas" one. The Postman theme
/// intentionally rides this same paper path — e.g. `hint_color()`
/// gives the Postman canvas the same paper-grey placeholder treatment
/// as the Light theme, since both are near-white surfaces.
pub fn is_light() -> bool {
    !matches!(current_theme(), Theme::Dark)
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
    // Light-based themes (Light, Postman) want dark widgets on a light
    // panel — flip `dark_mode` so egui's internal defaults pick sensible
    // colors for things we don't override (scroll thumbs, tooltips, etc).
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

    // Per-theme text sizes. The Postman theme is tuned a step smaller
    // than Dark/Light to match the reading density of the Postman web
    // UI (12-13px body, 17px headings). Weight is handled separately
    // by shipping `Inter-Light.ttf` for the font itself — this block
    // is purely about point-size, not weight compensation.
    let (heading, body, mono, button, small) = if matches!(theme, Theme::Postman) {
        (17.0, 12.5, 12.0, 12.0, 10.5)
    } else {
        (20.0, 13.5, 12.5, 13.0, 11.0)
    };
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(heading, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(body, FontFamily::Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(mono, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(button, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(small, FontFamily::Proportional),
        ),
    ]
    .into();

    ctx.set_style(style);

    // Phosphor icon font — register once so icon glyphs resolve
    // inside any Proportional text (labels, buttons, RichText).
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    // Inter — Postman web uses Inter for its UI. We ship the `Light`
    // (300) weight rather than `Regular` (400) because egui renders via
    // ab_glyph with grayscale antialiasing only — no hinting, no
    // subpixel rendering — which makes every TTF look ~1 weight-step
    // heavier than it does in a browser. Light-in-egui ≈ Regular-in-
    // browser, which is the weight Postman web actually shows.
    // PUA codepoints have been stripped from the bundled TTF so Inter
    // doesn't shadow phosphor's icon glyphs at U+E000–U+F8FF.
    if matches!(theme, Theme::Postman) {
        fonts.font_data.insert(
            "Inter".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/Inter-Light.ttf")),
        );
        let prop = fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default();
        prop.insert(0, "Inter".to_owned());
        // egui_phosphor installs itself at index 1 of Proportional. With
        // Inter now at index 0, phosphor gets pushed to index 2 — behind
        // egui's default sans, which has a glyph at `U+E1FE` (phosphor's
        // DOTS_THREE codepoint) that renders as a capital "M". Move
        // phosphor back up to index 1 so its PUA icons win before the
        // default sans ever gets a shot.
        if let Some(pos) = prop.iter().position(|s| s == "phosphor") {
            let phosphor = prop.remove(pos);
            prop.insert(1, phosphor);
        }
    }
    ctx.set_fonts(fonts);
}
