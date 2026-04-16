#![allow(unexpected_cfgs)]

use eframe::egui;

pub const APP_ICON_BYTES: &[u8] = include_bytes!("../assets/icon.png");

pub fn load_icon_color_image() -> Option<egui::ColorImage> {
    let img = image::load_from_memory(APP_ICON_BYTES).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

pub fn load_window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(APP_ICON_BYTES).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

/// Set `NSApp.activationPolicy = Regular`. This **must** be called before
/// `NSApp.run` starts processing events — i.e. before `eframe::run_native`.
/// macOS rejects the policy change once event processing has begun, which is
/// why a CLI-launched process stays Accessory and inherits the parent
/// terminal's icon in Cmd+Tab.
///
/// Set `RUSTY_REQUESTER_LOG_ICON=1` to print diagnostics.
#[cfg(target_os = "macos")]
pub fn set_macos_activation_policy_regular() -> Result<(), &'static str> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    let log = std::env::var("RUSTY_REQUESTER_LOG_ICON").is_ok();
    unsafe {
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        if log {
            eprintln!("[icon] (early) NSApp.sharedApplication = {:p}", app);
        }
        if app.is_null() {
            return Err("sharedApplication returned null");
        }
        // 0 = NSApplicationActivationPolicyRegular
        let ok: bool = msg_send![app, setActivationPolicy: 0_i64];
        if log {
            eprintln!("[icon] (early) setActivationPolicy(Regular) -> {}", ok);
        }
        if !ok {
            return Err("setActivationPolicy(Regular) returned NO");
        }
    }
    Ok(())
}

/// Set `NSApp.applicationIconImage` from PNG bytes. Safe to call after
/// `NSApp.run` has begun (icon image changes aren't restricted, only
/// activation policy is).
#[cfg(target_os = "macos")]
pub fn set_macos_app_icon_image(bytes: &[u8]) -> Result<(), &'static str> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    let log = std::env::var("RUSTY_REQUESTER_LOG_ICON").is_ok();
    unsafe {
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        if log {
            eprintln!("[icon] NSApp.sharedApplication = {:p}", app);
        }
        if app.is_null() {
            return Err("sharedApplication returned null");
        }

        let data: *mut Object = msg_send![
            class!(NSData),
            dataWithBytes: bytes.as_ptr() as *const std::ffi::c_void
            length: bytes.len() as u64
        ];
        if log {
            eprintln!("[icon] NSData(len={}) = {:p}", bytes.len(), data);
        }
        if data.is_null() {
            return Err("NSData allocation returned null");
        }

        let image_alloc: *mut Object = msg_send![class!(NSImage), alloc];
        if image_alloc.is_null() {
            return Err("NSImage alloc returned null");
        }
        let image: *mut Object = msg_send![image_alloc, initWithData: data];
        if log {
            eprintln!("[icon] NSImage initWithData -> {:p}", image);
        }
        if image.is_null() {
            return Err("NSImage initWithData returned null (PNG decode failed?)");
        }

        let _: () = msg_send![app, setApplicationIconImage: image];
        if log {
            eprintln!("[icon] setApplicationIconImage called");
        }

        let _: () = msg_send![app, activateIgnoringOtherApps: true];
        if log {
            eprintln!("[icon] activateIgnoringOtherApps(true)");
        }
    }
    Ok(())
}

/// Merge the title bar into the window chrome — Postman / Arc /
/// Ghostty style. Sets on every open NSWindow:
///   * `titlebarAppearsTransparent = YES` — no title bar fill
///   * `titleVisibility = NSWindowTitleHidden` — no title text
///   * `styleMask |= NSFullSizeContentViewWindowMask` — content
///     extends under the title bar so our bg paints the whole window
///
/// Safe to call after `NSApp.run` has started; must be called on
/// every window (we only have one, but `for w in NSApp.windows`
/// handles that and is robust to future multi-window support).
///
/// Returns `Err` with a short message if any cocoa call fails.
#[cfg(target_os = "macos")]
pub fn set_macos_titlebar_transparent() -> Result<(), &'static str> {
    use objc::runtime::{Object, YES};
    use objc::{class, msg_send, sel, sel_impl};

    let log = std::env::var("RUSTY_REQUESTER_LOG_ICON").is_ok();
    unsafe {
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        if app.is_null() {
            return Err("sharedApplication returned null");
        }
        let windows: *mut Object = msg_send![app, windows];
        if windows.is_null() {
            return Err("NSApp.windows returned null");
        }
        let count: usize = msg_send![windows, count];
        if log {
            eprintln!("[titlebar] NSApp.windows count = {}", count);
        }
        for i in 0..count {
            let window: *mut Object = msg_send![windows, objectAtIndex: i];
            if window.is_null() {
                continue;
            }
            // Order matters here: enable fullSizeContentView FIRST,
            // then hide the title, then make the bar transparent.
            // Otherwise AppKit may refuse to re-layout content into
            // the expanded region until the next window refresh.
            let current_mask: u64 = msg_send![window, styleMask];
            // NSWindowStyleMaskFullSizeContentView = 1 << 15
            let new_mask: u64 = current_mask | (1_u64 << 15);
            let _: () = msg_send![window, setStyleMask: new_mask];
            // NSWindowTitleHidden = 1
            let _: () = msg_send![window, setTitleVisibility: 1_i64];
            let _: () = msg_send![window, setTitlebarAppearsTransparent: YES];
            if log {
                eprintln!(
                    "[titlebar] window[{}] styleMask: {:#x} -> {:#x}",
                    i, current_mask, new_mask
                );
            }
        }
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn set_macos_activation_policy_regular() -> Result<(), &'static str> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn set_macos_app_icon_image(_bytes: &[u8]) -> Result<(), &'static str> {
    Ok(())
}

/// Non-macOS stub — the caller is gated on `cfg(target_os = "macos")`
/// so this is genuinely unreachable on other platforms. `#[allow]`
/// keeps CI's `-D warnings` happy without wrapping the call site in
/// more cfgs; same pattern as the other macOS stubs above.
#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
pub fn set_macos_titlebar_transparent() -> Result<(), &'static str> {
    Ok(())
}
