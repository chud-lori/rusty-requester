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
