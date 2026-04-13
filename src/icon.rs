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

#[cfg(not(target_os = "macos"))]
pub fn set_macos_activation_policy_regular() -> Result<(), &'static str> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn set_macos_app_icon_image(_bytes: &[u8]) -> Result<(), &'static str> {
    Ok(())
}
