use crate::model::{Folder, HttpMethod, KvRow};
use crate::theme::*;
use eframe::egui;

pub fn paint_x(
    painter: &egui::Painter,
    center: egui::Pos2,
    half: f32,
    color: egui::Color32,
    width: f32,
) {
    let stroke = egui::Stroke::new(width, color);
    painter.line_segment(
        [
            egui::pos2(center.x - half, center.y - half),
            egui::pos2(center.x + half, center.y + half),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x - half, center.y + half),
            egui::pos2(center.x + half, center.y - half),
        ],
        stroke,
    );
}

pub fn close_x_button(ui: &mut egui::Ui, hover_text: &str) -> egui::Response {
    let size = egui::vec2(20.0, 20.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        if hovered {
            ui.painter().rect_filled(
                rect,
                egui::Rounding::same(4.0),
                C_RED.linear_multiply(0.35),
            );
        }
        let color = if hovered { C_RED } else { C_MUTED };
        paint_x(ui.painter(), rect.center(), 4.0, color, 1.5);
    }
    resp.on_hover_text(hover_text)
}

pub fn tab_button<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    current: &mut T,
    value: T,
    label: &str,
) {
    let selected = *current == value;
    let (text_color, text) = if selected {
        (
            C_ACCENT,
            egui::RichText::new(label).color(C_ACCENT).strong().size(13.0),
        )
    } else {
        (C_MUTED, egui::RichText::new(label).color(C_MUTED).size(13.0))
    };
    let _ = text_color;
    let btn = egui::Button::new(text)
        .fill(egui::Color32::TRANSPARENT)
        .stroke(egui::Stroke::NONE)
        .rounding(egui::Rounding::same(6.0))
        .min_size(egui::vec2(90.0, 30.0));
    let resp = ui.add(btn);
    if selected {
        let rect = resp.rect;
        let y = rect.bottom() - 1.0;
        let pad = 10.0;
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + pad, y),
                egui::pos2(rect.right() - pad, y),
            ],
            egui::Stroke::new(2.5, C_ACCENT),
        );
    }
    if resp.clicked() {
        *current = value;
    }
}

pub fn elide(text: &str, max_width: f32, font: &egui::FontId, ui: &egui::Ui) -> String {
    if max_width <= 0.0 {
        return String::new();
    }
    let measure = |s: &str| {
        ui.fonts(|f| f.layout_no_wrap(s.to_string(), font.clone(), egui::Color32::WHITE))
            .size()
            .x
    };
    if measure(text) <= max_width {
        return text.to_string();
    }
    let ellipsis = "…";
    let mut lo = 0usize;
    let mut hi = text.chars().count();
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let candidate: String = text.chars().take(mid).collect::<String>() + ellipsis;
        if measure(&candidate) <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    text.chars().take(lo).collect::<String>() + ellipsis
}

pub fn find_request_info(
    folders: &[Folder],
    folder_path: &[String],
    request_id: &str,
) -> Option<(HttpMethod, String)> {
    if folder_path.is_empty() {
        return None;
    }
    let mut folder = folders.iter().find(|f| f.id == folder_path[0])?;
    for id in &folder_path[1..] {
        folder = folder.subfolders.iter().find(|f| &f.id == id)?;
    }
    folder
        .requests
        .iter()
        .find(|r| r.id == request_id)
        .map(|r| (r.method.clone(), r.name.clone()))
}

pub fn render_kv_table(
    ui: &mut egui::Ui,
    title: &str,
    rows: &mut Vec<KvRow>,
    show_description: bool,
) -> bool {
    let mut changed = false;

    ui.label(
        egui::RichText::new(title)
            .size(11.0)
            .strong()
            .color(C_MUTED),
    );
    ui.add_space(4.0);

    if rows.last().map(|r| !r.is_blank()).unwrap_or(true) {
        rows.push(KvRow::empty());
    }

    let avail = ui.available_width();
    let cb_w = 22.0;
    let del_w = 22.0;
    let key_w = 200.0;
    let row_h = 24.0;
    let cell_pad = 6.0;
    let (val_w, desc_w) = if show_description {
        let total = (avail - cb_w - key_w - del_w - cell_pad * 4.0).max(200.0);
        (total * 0.55, total * 0.45)
    } else {
        let val = avail - cb_w - key_w - del_w - cell_pad * 3.0;
        (val.max(150.0), 0.0)
    };

    ui.horizontal(|ui| {
        ui.add_space(cb_w + cell_pad);
        ui.label(egui::RichText::new("KEY").size(10.0).color(C_MUTED));
        ui.add_space(key_w - 20.0);
        ui.label(egui::RichText::new("VALUE").size(10.0).color(C_MUTED));
        if show_description {
            ui.add_space(val_w - 30.0);
            ui.label(egui::RichText::new("DESCRIPTION").size(10.0).color(C_MUTED));
        }
    });
    ui.add_space(2.0);
    ui.painter().line_segment(
        [
            egui::pos2(ui.cursor().left(), ui.cursor().top()),
            egui::pos2(
                ui.cursor().left() + ui.available_width(),
                ui.cursor().top(),
            ),
        ],
        egui::Stroke::new(1.0, C_BORDER.linear_multiply(0.6)),
    );
    ui.add_space(4.0);

    let mut to_remove: Option<usize> = None;
    let row_count = rows.len();
    for (i, row) in rows.iter_mut().enumerate() {
        let is_blank = row.is_blank();
        let is_last_blank = is_blank && i == row_count - 1;
        let bg = if is_last_blank {
            C_PANEL_DARK.linear_multiply(0.6)
        } else {
            egui::Color32::TRANSPARENT
        };
        egui::Frame::none()
            .fill(bg)
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(2.0, 2.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if is_last_blank {
                        ui.add_space(cb_w);
                    } else {
                        let cb = ui.add(egui::Checkbox::new(&mut row.enabled, ""));
                        if cb.changed() {
                            changed = true;
                        }
                    }
                    ui.add_space(cell_pad);

                    let text_color = if row.enabled { C_TEXT } else { C_MUTED };

                    let key_resp = ui.add_sized(
                        [key_w, row_h],
                        egui::TextEdit::singleline(&mut row.key)
                            .hint_text(if is_last_blank { "Key" } else { "" })
                            .text_color(text_color),
                    );
                    if key_resp.changed() {
                        changed = true;
                    }
                    ui.add_space(cell_pad);

                    let val_resp = ui.add_sized(
                        [val_w, row_h],
                        egui::TextEdit::singleline(&mut row.value)
                            .hint_text(if is_last_blank { "Value" } else { "" })
                            .text_color(text_color),
                    );
                    if val_resp.changed() {
                        changed = true;
                    }

                    if show_description {
                        ui.add_space(cell_pad);
                        let desc_resp = ui.add_sized(
                            [desc_w, row_h],
                            egui::TextEdit::singleline(&mut row.description)
                                .hint_text(if is_last_blank { "Description" } else { "" })
                                .text_color(text_color),
                        );
                        if desc_resp.changed() {
                            changed = true;
                        }
                    }

                    ui.add_space(cell_pad);
                    if is_last_blank {
                        ui.add_space(del_w);
                    } else if close_x_button(ui, "Remove row").clicked() {
                        to_remove = Some(i);
                    }
                });
            });
        ui.add_space(2.0);
    }

    if let Some(i) = to_remove {
        rows.remove(i);
        changed = true;
    }

    ui.add_space(6.0);
    if ui
        .add(
            egui::Button::new(egui::RichText::new("+ Add row").size(12.0).color(C_ACCENT))
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::new(1.0, C_BORDER))
                .min_size(egui::vec2(100.0, 26.0)),
        )
        .clicked()
    {
        if let Some(last) = rows.last_mut() {
            last.enabled = true;
        }
        rows.push(KvRow::empty());
        changed = true;
    }

    changed
}

#[derive(Clone, Copy)]
pub enum TabAction {
    None,
    Activate,
    Close,
    CloseOthers,
    CloseAll,
}

pub fn render_single_tab(
    ui: &mut egui::Ui,
    idx: usize,
    method: &HttpMethod,
    name: &str,
    is_active: bool,
) -> TabAction {
    let tab_height = 32.0;
    let tab_width = 180.0;
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(tab_width, tab_height),
        egui::Sense::click(),
    );

    let mut action = TabAction::None;

    if ui.is_rect_visible(rect) {
        let bg = if is_active {
            C_BG
        } else if resp.hovered() {
            C_ELEVATED
        } else {
            C_PANEL
        };
        let rounding = egui::Rounding {
            nw: 8.0,
            ne: 8.0,
            sw: 0.0,
            se: 0.0,
        };
        ui.painter().rect_filled(rect, rounding, bg);

        if is_active {
            let top = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 2.0));
            ui.painter().rect_filled(top, rounding, C_ACCENT);
        }

        let mc = method_color(method);
        let method_str = format!("{}", method);
        let method_font = egui::FontId::new(10.0, egui::FontFamily::Proportional);
        let pad_left = rect.left() + 12.0;
        let mid_y = rect.center().y;

        let pill_w = 48.0;
        let pill_h = 18.0;
        let pill_rect = egui::Rect::from_min_size(
            egui::pos2(pad_left, mid_y - pill_h / 2.0),
            egui::vec2(pill_w, pill_h),
        );
        ui.painter()
            .rect_filled(pill_rect, egui::Rounding::same(4.0), mc);
        ui.painter().text(
            pill_rect.center(),
            egui::Align2::CENTER_CENTER,
            method_str,
            method_font,
            pill_text_color(mc),
        );

        let name_x = pill_rect.right() + 8.0;
        let name_color = if is_active { C_TEXT } else { C_MUTED };
        let name_font = egui::FontId::new(12.0, egui::FontFamily::Proportional);
        let max_w = (rect.right() - 28.0) - name_x;
        let display = elide(name, max_w.max(0.0), &name_font, ui);
        ui.painter().text(
            egui::pos2(name_x, mid_y),
            egui::Align2::LEFT_CENTER,
            display,
            name_font,
            name_color,
        );
    }

    let close_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - 22.0, rect.center().y - 9.0),
        egui::vec2(18.0, 18.0),
    );
    let close_resp = ui.interact(
        close_rect,
        ui.make_persistent_id(format!("tab_close_{}", idx)),
        egui::Sense::click(),
    );
    if ui.is_rect_visible(close_rect) {
        let hovered = close_resp.hovered();
        if hovered {
            ui.painter().rect_filled(
                close_rect,
                egui::Rounding::same(4.0),
                C_RED.linear_multiply(0.35),
            );
        }
        let color = if hovered { C_RED } else { C_MUTED };
        paint_x(ui.painter(), close_rect.center(), 4.0, color, 1.5);
    }

    if close_resp.clicked() {
        action = TabAction::Close;
    } else if resp.clicked() {
        let click_pos = ui.input(|i| i.pointer.interact_pos()).unwrap_or_default();
        if !close_rect.contains(click_pos) {
            action = TabAction::Activate;
        }
    }

    resp.context_menu(|ui| {
        if ui.button("Close").clicked() {
            action = TabAction::Close;
            ui.close_menu();
        }
        if ui.button("Close others").clicked() {
            action = TabAction::CloseOthers;
            ui.close_menu();
        }
        if ui.button("Close all").clicked() {
            action = TabAction::CloseAll;
            ui.close_menu();
        }
    });

    action
}

pub fn folder_matches(folder: &Folder, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    if folder.name.to_lowercase().contains(q) {
        return true;
    }
    if folder.requests.iter().any(|r| request_matches(r, q)) {
        return true;
    }
    folder.subfolders.iter().any(|sub| folder_matches(sub, q))
}

pub fn request_matches(r: &crate::model::Request, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    r.name.to_lowercase().contains(q)
        || r.url.to_lowercase().contains(q)
        || format!("{}", r.method).to_lowercase().contains(q)
}

pub fn count_matches(folders: &[Folder], q: &str) -> usize {
    let mut n = 0;
    for f in folders {
        for r in &f.requests {
            if request_matches(r, q) {
                n += 1;
            }
        }
        n += count_matches(&f.subfolders, q);
    }
    n
}

pub fn short_name_from_url(url: &str) -> String {
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let cutoff = stripped.find('?').unwrap_or(stripped.len());
    stripped[..cutoff].trim_end_matches('/').to_string()
}

pub fn sanitize_filename(name: &str) -> Option<String> {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let trimmed = cleaned.trim_matches('_').to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}
