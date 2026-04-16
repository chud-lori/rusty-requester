//! Native macOS `NSMenu` bar — built via the `muda` crate so the menu
//! lives on the top of the screen (the macOS-idiomatic place) instead
//! of inside the window. Item IDs are stable strings that `ApiClient`
//! checks each frame via `poll_menu_events`.
//!
//! Linux/Windows don't go through this module — they keep the in-window
//! egui menu bar in `ui::modals::render_menu_bar`.

use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    Menu, MenuItem, PredefinedMenuItem, Submenu,
};

pub const MENU_NEW_REQUEST: &str = "new_request";
pub const MENU_NEW_COLLECTION: &str = "new_collection";
pub const MENU_IMPORT: &str = "import";
pub const MENU_PASTE_CURL: &str = "paste_curl";
pub const MENU_EXPORT_JSON: &str = "export_json";
pub const MENU_EXPORT_YAML: &str = "export_yaml";
pub const MENU_TOGGLE_SNIPPET: &str = "toggle_snippet";
pub const MENU_COMMAND_PALETTE: &str = "command_palette";
pub const MENU_SEND: &str = "send";
pub const MENU_SETTINGS: &str = "settings";
pub const MENU_ENVIRONMENTS: &str = "environments";
pub const MENU_CLOSE_TAB: &str = "close_tab";
pub const MENU_ABOUT: &str = "about";
pub const MENU_GITHUB: &str = "github";
pub const MENU_REPORT_ISSUE: &str = "report_issue";

/// Install a system NSMenu bar on `NSApp`. Must be called after
/// `set_macos_activation_policy_regular()` has run (otherwise NSApp
/// is in accessory mode and the menu bar is hidden).
///
/// The `Menu` value must be kept alive for the life of the app; we
/// leak it on purpose via `Box::leak` so callers don't need to store
/// it (no state-plumbing into `ApiClient`).
pub fn install() {
    let cmd = Modifiers::META; // Command on macOS
    let shift = Modifiers::META.union(Modifiers::SHIFT);

    let menu = Menu::new();

    // --- App menu (the leftmost one, named after the app) ---------------
    //
    // The About entry's title starts with a zero-width space
    // (`\u{200B}`) so users see "About Rusty Requester" visually
    // while muda/AppKit's prefix check for `"About "` doesn't match
    // — otherwise the item gets auto-routed to NSApp's stock
    // `orderFrontStandardAboutPanel:` and our custom modal never
    // opens. Apple HIG wants the About item at the top of the app
    // menu; the zero-width-space hack is how we keep it there AND
    // keep our own handler firing.
    let app_submenu = Submenu::new("Rusty Requester", true);
    app_submenu
        .append_items(&[
            &MenuItem::with_id(MENU_ABOUT, "\u{200B}About Rusty Requester", true, None),
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(
                MENU_SETTINGS,
                "Preferences…",
                true,
                Some(Accelerator::new(Some(cmd), Code::Comma)),
            ),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(None),
        ])
        .unwrap();

    // --- File -----------------------------------------------------------
    let file = Submenu::new("File", true);
    file.append_items(&[
        &MenuItem::with_id(
            MENU_NEW_REQUEST,
            "New Request",
            true,
            Some(Accelerator::new(Some(cmd), Code::KeyN)),
        ),
        &MenuItem::with_id(MENU_NEW_COLLECTION, "New Collection", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(
            MENU_CLOSE_TAB,
            "Close Tab",
            true,
            Some(Accelerator::new(Some(cmd), Code::KeyW)),
        ),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(
            MENU_IMPORT,
            "Import collection file…",
            true,
            Some(Accelerator::new(Some(cmd), Code::KeyO)),
        ),
        &MenuItem::with_id(MENU_PASTE_CURL, "Paste cURL command…", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(MENU_EXPORT_JSON, "Export all as JSON…", true, None),
        &MenuItem::with_id(MENU_EXPORT_YAML, "Export all as YAML…", true, None),
    ])
    .unwrap();

    // No Edit submenu on purpose. muda's predefined Cut / Copy /
    // Paste / Select-All install AppKit's native Cmd+X/C/V/A
    // shortcuts that route through NSApp's first responder — which
    // egui's TextEdit isn't, so the events get *consumed* by the
    // menu and never reach the editor. Without this submenu, egui
    // handles all text-editing shortcuts itself (and they actually
    // work).

    // --- View ----------------------------------------------------------
    let view = Submenu::new("View", true);
    view.append_items(&[
        &MenuItem::with_id(
            MENU_COMMAND_PALETTE,
            "Command Palette…",
            true,
            Some(Accelerator::new(Some(cmd), Code::KeyP)),
        ),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(
            MENU_TOGGLE_SNIPPET,
            "Toggle code snippet panel",
            true,
            Some(Accelerator::new(Some(shift), Code::KeyC)),
        ),
    ])
    .unwrap();

    // --- Request -------------------------------------------------------
    let request = Submenu::new("Request", true);
    request
        .append_items(&[
            &MenuItem::with_id(
                MENU_SEND,
                "Send",
                true,
                Some(Accelerator::new(Some(cmd), Code::Enter)),
            ),
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(MENU_ENVIRONMENTS, "Environments…", true, None),
        ])
        .unwrap();

    // --- Help ----------------------------------------------------------
    //
    // About item uses the same zero-width-space-prefixed title as
    // the app menu's About so AppKit's auto-router can't hijack it
    // either.
    let help = Submenu::new("Help", true);
    help.append_items(&[
        &MenuItem::with_id(MENU_ABOUT, "\u{200B}About Rusty Requester", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(MENU_GITHUB, "Open GitHub repo", true, None),
        &MenuItem::with_id(MENU_REPORT_ISSUE, "Report an issue", true, None),
    ])
    .unwrap();

    menu.append_items(&[&app_submenu, &file, &view, &request, &help])
        .unwrap();

    menu.init_for_nsapp();

    // Keep the menu alive forever — NSApp holds a raw pointer to its
    // items, so dropping this would invalidate them.
    Box::leak(Box::new(menu));
}

/// Drain any menu events emitted since the last call. Returns a list
/// of item IDs the user activated. Safe to call every frame.
pub fn drain_events() -> Vec<String> {
    let mut out = Vec::new();
    while let Ok(ev) = muda::MenuEvent::receiver().try_recv() {
        out.push(ev.id().0.clone());
    }
    out
}
