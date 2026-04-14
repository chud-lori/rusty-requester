use crate::model::{BodyView, Environment, Folder, HttpMethod, KvRow, Request};
use crate::theme::*;
use eframe::egui;
use serde_json::Value;

/// Replace `{{var}}` tokens in `text` with values from the active environment.
pub fn substitute_vars(text: &str, env: Option<&Environment>) -> String {
    let Some(env) = env else {
        return text.to_string();
    };
    let mut out = text.to_string();
    for v in &env.variables {
        if v.enabled && !v.key.is_empty() {
            let pat = format!("{{{{{}}}}}", v.key);
            out = out.replace(&pat, &v.value);
        }
    }
    out
}

/// Apply `{{var}}` substitution to every key/value in a list (returns a new vec).
pub fn substitute_kvs(rows: &[KvRow], env: Option<&Environment>) -> Vec<KvRow> {
    rows.iter()
        .map(|r| KvRow {
            enabled: r.enabled,
            key: substitute_vars(&r.key, env),
            value: substitute_vars(&r.value, env),
            description: r.description.clone(),
        })
        .collect()
}

/// Draw a small magnifying-glass search icon centred on `center`.
/// The circle has `radius`, handle extends diagonally down-right.
pub fn paint_search_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    color: egui::Color32,
) {
    let stroke = egui::Stroke::new(1.5, color);
    let radius = 4.5;
    painter.circle_stroke(center, radius, stroke);
    // Handle — from lower-right of circle, extending down-right
    let start = egui::pos2(
        center.x + radius * 0.70,
        center.y + radius * 0.70,
    );
    let end = egui::pos2(
        center.x + radius + 2.5,
        center.y + radius + 2.5,
    );
    painter.line_segment([start, end], stroke);
}

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

/// Returns `(method, name, url)` for the request referenced by
/// `folder_path` + `request_id`. Empty `folder_path` means "look in drafts".
pub fn find_request_info(
    folders: &[Folder],
    drafts: &[Request],
    folder_path: &[String],
    request_id: &str,
) -> Option<(HttpMethod, String, String)> {
    if folder_path.is_empty() {
        return drafts
            .iter()
            .find(|r| r.id == request_id)
            .map(|r| (r.method.clone(), r.name.clone(), r.url.clone()));
    }
    let mut folder = folders.iter().find(|f| f.id == folder_path[0])?;
    for id in &folder_path[1..] {
        folder = folder.subfolders.iter().find(|f| &f.id == id)?;
    }
    folder
        .requests
        .iter()
        .find(|r| r.id == request_id)
        .map(|r| (r.method.clone(), r.name.clone(), r.url.clone()))
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
    // Stable ID salt for this table — the table title plus a role so that
    // two kv_tables on the same page (e.g. Params and Headers) don't
    // collide.  Widget IDs are then derived from (salt, row_index, field)
    // so they stay stable when the ghost row transitions to a real row
    // (which adds a checkbox in front of the TextEdits). Without stable
    // IDs, egui reassigns auto-IDs and the user's focus is lost mid-type.
    let id_salt = egui::Id::new(("kv_table", title));
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
                            .id(id_salt.with((i, "key")))
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
                            .id(id_salt.with((i, "value")))
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
                                .id(id_salt.with((i, "desc")))
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

    // No "+ Add row" button: a blank ghost row is always kept at the
    // bottom (see `rows.push(KvRow::empty())` above), and a new one
    // auto-appears the moment the user types into it. This matches
    // Postman's behavior.

    changed
}

#[derive(Clone, Copy)]
pub enum TabAction {
    None,
    Activate,
    Close,
    CloseOthers,
    CloseAll,
    SaveDraft,
}

pub fn render_single_tab(
    ui: &mut egui::Ui,
    idx: usize,
    method: &HttpMethod,
    name: &str,
    url: &str,
    is_active: bool,
    is_draft: bool,
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

        // Method as bold colored text (no filled pill), matching the
        // sidebar-row convention.
        let mc = method_color(method);
        let method_str = format!("{}", method);
        let method_font = egui::FontId::new(10.0, egui::FontFamily::Proportional);
        let method_slot_w = 42.0;
        let pad_left = rect.left() + 12.0;
        let mid_y = rect.center().y;
        ui.painter().text(
            egui::pos2(pad_left, mid_y),
            egui::Align2::LEFT_CENTER,
            method_str,
            method_font,
            mc,
        );

        let name_x = pad_left + method_slot_w;
        let name_color = if is_active { C_TEXT } else { C_MUTED };
        let name_font = egui::FontId::new(12.0, egui::FontFamily::Proportional);
        // Reserve space for close button and (optional) unsaved dot.
        let right_reserve: f32 = if is_draft { 40.0 } else { 28.0 };
        let max_w = (rect.right() - right_reserve) - name_x;
        let display = elide(name, max_w.max(0.0), &name_font, ui);
        ui.painter().text(
            egui::pos2(name_x, mid_y),
            egui::Align2::LEFT_CENTER,
            display,
            name_font,
            name_color,
        );

        // Draft / unsaved indicator — small amber dot between the name
        // and the close button, matching the Postman convention.
        if is_draft {
            let dot_x = rect.right() - 30.0;
            ui.painter().circle_filled(
                egui::pos2(dot_x, mid_y),
                3.5,
                C_ORANGE,
            );
        }
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

    // Hover preview — shows the full name and URL when the pointer
    // lingers on the tab, like Postman's tab tooltip.
    let tip_name = name.to_string();
    let tip_url = url.to_string();
    let tip_is_draft = is_draft;
    resp.clone().on_hover_ui(move |ui| {
        ui.set_max_width(360.0);
        ui.label(
            egui::RichText::new(&tip_name)
                .size(13.0)
                .strong()
                .color(C_TEXT),
        );
        ui.add_space(4.0);
        if tip_url.is_empty() {
            ui.label(
                egui::RichText::new("(no URL)")
                    .color(C_MUTED)
                    .size(11.0)
                    .italics(),
            );
        } else {
            ui.label(
                egui::RichText::new(&tip_url)
                    .color(C_MUTED)
                    .size(11.0),
            );
        }
        if tip_is_draft {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("● Unsaved draft")
                    .color(C_ORANGE)
                    .size(11.0)
                    .strong(),
            );
        }
    });

    resp.context_menu(|ui| {
        if is_draft {
            if ui.button("Save to folder...").clicked() {
                action = TabAction::SaveDraft;
                ui.close_menu();
            }
            ui.separator();
        }
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

/// Paints a collapsing-header chevron (▶ / ▼) using primitive shapes so it
/// stays font-safe. Plug into `egui::CollapsingHeader::icon(...)`.
/// `openness` ∈ [0, 1]: 0 = collapsed (▶ right), 1 = expanded (▼ down).
pub fn paint_folder_chevron(ui: &mut egui::Ui, openness: f32, response: &egui::Response) {
    let rect = response.rect;
    let center = rect.center();
    let r = 4.5;
    // Start with a right-pointing triangle, rotate by 90° × openness.
    let angle = openness * std::f32::consts::FRAC_PI_2;
    let rot = |p: egui::Vec2| -> egui::Vec2 {
        let (s, c) = angle.sin_cos();
        egui::vec2(p.x * c - p.y * s, p.x * s + p.y * c)
    };
    let p0 = center + rot(egui::vec2(-r * 0.6, -r));
    let p1 = center + rot(egui::vec2(-r * 0.6, r));
    let p2 = center + rot(egui::vec2(r, 0.0));
    ui.painter().add(egui::Shape::convex_polygon(
        vec![p0, p1, p2],
        C_MUTED,
        egui::Stroke::NONE,
    ));
}

/// Paints a thin plus icon (two crossed lines) at the given center.
pub fn paint_plus_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.4, color);
    let half = 5.0;
    painter.line_segment(
        [
            egui::pos2(center.x - half, center.y),
            egui::pos2(center.x + half, center.y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x, center.y - half),
            egui::pos2(center.x, center.y + half),
        ],
        stroke,
    );
}

/// Paints a horizontal three-dots (overflow-menu) icon at the given center.
pub fn paint_dots_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let r = 1.5;
    let gap = 4.0;
    for dx in &[-gap, 0.0, gap] {
        painter.circle_filled(egui::pos2(center.x + dx, center.y), r, color);
    }
}

/// Paints a "copy" icon — two overlapping rounded rects — at the given
/// center point. Font-free.
pub fn paint_copy_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let size = egui::vec2(9.0, 11.0);
    let stroke = egui::Stroke::new(1.3, color);
    // Back rect (upper-left), front rect (lower-right).
    let back = egui::Rect::from_center_size(center + egui::vec2(-1.5, -1.5), size);
    let front = egui::Rect::from_center_size(center + egui::vec2(1.5, 1.5), size);
    painter.rect_stroke(back, egui::Rounding::same(1.5), stroke);
    // Mask front's intersection with back so it visually sits on top.
    painter.rect_filled(front, egui::Rounding::same(1.5), painter.ctx().style().visuals.panel_fill);
    painter.rect_stroke(front, egui::Rounding::same(1.5), stroke);
}

/// Compact square icon button — 24×24 clickable area with an icon
/// drawn by `paint` via an egui::Painter reference. Used in toolbars
/// next to the response body view pills.
pub fn icon_button(
    ui: &mut egui::Ui,
    hover_text: &str,
    paint: impl FnOnce(&egui::Painter, egui::Pos2, egui::Color32),
) -> egui::Response {
    let size = egui::vec2(26.0, 22.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let color = if resp.hovered() { C_TEXT } else { C_MUTED };
        if resp.hovered() {
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(4.0), C_ELEVATED);
        }
        paint(&ui.painter(), rect.center(), color);
    }
    resp.on_hover_text(hover_text)
}

/// Paints a tiny folder glyph (two stacked rounded rects) at the given
/// center point. Font-free — matches `paint_folder_chevron`'s rationale.
pub fn paint_folder_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let body = egui::Rect::from_center_size(
        center + egui::vec2(0.0, 1.0),
        egui::vec2(14.0, 10.0),
    );
    let tab = egui::Rect::from_min_size(
        egui::pos2(body.left(), body.top() - 3.0),
        egui::vec2(6.0, 3.5),
    );
    painter.rect_filled(tab, egui::Rounding::same(1.5), color);
    painter.rect_filled(body, egui::Rounding::same(2.0), color);
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

/// Small segmented-control pill used in the response Body tab to switch
/// between Raw text and the JSON tree view. Styled like the main tab
/// bar but compact (height ~22, no underline).
pub fn body_view_pill(
    ui: &mut egui::Ui,
    current: &mut BodyView,
    value: BodyView,
    label: &str,
) {
    let is_active = *current == value;
    let fg = if is_active { C_ACCENT } else { C_MUTED };
    let bg = if is_active {
        C_ACCENT.linear_multiply(0.15)
    } else {
        egui::Color32::TRANSPARENT
    };
    let resp = ui.add(
        egui::Button::new(
            egui::RichText::new(label)
                .size(12.0)
                .color(fg)
                .strong(),
        )
        .fill(bg)
        .stroke(egui::Stroke::new(
            1.0,
            if is_active { C_ACCENT } else { C_BORDER },
        ))
        .min_size(egui::vec2(64.0, 22.0))
        .rounding(egui::Rounding::same(6.0)),
    );
    if resp.clicked() {
        *current = value;
    }
}

/// Render a JSON `Value` as an interactive, collapsible tree. Objects
/// and arrays become expandable nodes; leaves are rendered inline as
/// `key: value` with syntax colors.  `filter` (lowercase) hides any
/// subtree that has no matching key/value; an empty filter shows all.
pub fn render_json_tree(
    ui: &mut egui::Ui,
    id: egui::Id,
    key: &str,
    value: &Value,
    filter: &str,
    depth: usize,
) {
    if !filter.is_empty() && !subtree_matches(key, value, filter) {
        return;
    }

    match value {
        Value::Object(map) => {
            let summary = format!("{{...}} ({} key{})", map.len(), if map.len() == 1 { "" } else { "s" });
            json_header(ui, id, key, &summary, depth, |ui| {
                for (k, v) in map {
                    render_json_tree(ui, id.with(k), k, v, filter, depth + 1);
                }
            });
        }
        Value::Array(items) => {
            let summary = format!("[...] ({} item{})", items.len(), if items.len() == 1 { "" } else { "s" });
            json_header(ui, id, key, &summary, depth, |ui| {
                for (i, v) in items.iter().enumerate() {
                    let sub_key = format!("[{}]", i);
                    render_json_tree(ui, id.with(i), &sub_key, v, filter, depth + 1);
                }
            });
        }
        _ => {
            ui.horizontal(|ui| {
                ui.add_space(16.0 * depth as f32 + 18.0);
                if !key.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("{}:", key))
                            .color(C_ACCENT)
                            .font(egui::FontId::monospace(12.5)),
                    );
                }
                let (color, text) = json_leaf_style(value);
                ui.label(
                    egui::RichText::new(text)
                        .color(color)
                        .font(egui::FontId::monospace(12.5)),
                );
            });
        }
    }
}

fn json_header(
    ui: &mut egui::Ui,
    id: egui::Id,
    key: &str,
    summary: &str,
    depth: usize,
    body: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.add_space(16.0 * depth as f32);
        let header_text = if key.is_empty() {
            summary.to_string()
        } else {
            format!("{}  {}", key, summary)
        };
        let head = egui::CollapsingHeader::new(
            egui::RichText::new(header_text)
                .color(if key.is_empty() { C_MUTED } else { C_TEXT })
                .font(egui::FontId::monospace(12.5)),
        )
        .id_salt(id)
        .default_open(depth < 2)
        .icon(paint_folder_chevron);
        head.show(ui, body);
    });
}

fn json_leaf_style(v: &Value) -> (egui::Color32, String) {
    match v {
        Value::String(s) => (
            egui::Color32::from_rgb(230, 219, 116),
            format!("\"{}\"", s),
        ),
        Value::Number(n) => (egui::Color32::from_rgb(174, 129, 255), n.to_string()),
        Value::Bool(b) => (egui::Color32::from_rgb(249, 38, 114), b.to_string()),
        Value::Null => (C_MUTED, "null".to_string()),
        _ => (C_TEXT, v.to_string()),
    }
}

fn subtree_matches(key: &str, value: &Value, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    if key.to_lowercase().contains(filter) {
        return true;
    }
    match value {
        Value::Object(map) => map.iter().any(|(k, v)| subtree_matches(k, v, filter)),
        Value::Array(items) => items.iter().any(|v| subtree_matches("", v, filter)),
        Value::String(s) => s.to_lowercase().contains(filter),
        Value::Number(n) => n.to_string().contains(filter),
        Value::Bool(b) => b.to_string().contains(filter),
        Value::Null => "null".contains(filter),
    }
}

/// Gantt-style phase breakdown for the response-time hover popover,
/// similar to Postman. We only measure three phases honestly from
/// reqwest (Prepare / Waiting-TTFB / Download); finer phases like DNS
/// lookup and TCP handshake are rolled into Waiting because the high-
/// level client doesn't expose them.
pub fn render_time_breakdown(
    ui: &mut egui::Ui,
    prepare_ms: u64,
    waiting_ms: u64,
    download_ms: u64,
    total_ms: u64,
) {
    ui.set_min_width(320.0);

    // Title + total.
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Response Time")
                .strong()
                .color(C_TEXT)
                .size(13.0),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{} ms", total_ms))
                    .strong()
                    .color(C_TEXT)
                    .size(13.0),
            );
        });
    });
    ui.add_space(6.0);

    let scale = total_ms.max(1) as f32;
    let row = |ui: &mut egui::Ui,
                label: &str,
                start_ms: u64,
                dur_ms: u64,
                color: egui::Color32| {
        let row_h = 18.0;
        let bar_total_w: f32 = 140.0;
        ui.horizontal(|ui| {
            ui.add_sized(
                egui::vec2(130.0, row_h),
                egui::Label::new(egui::RichText::new(label).color(C_MUTED).size(12.0)),
            );
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(bar_total_w, row_h), egui::Sense::hover());
            let offset = (start_ms as f32 / scale) * bar_total_w;
            let width = ((dur_ms as f32 / scale) * bar_total_w).max(2.0);
            let bar = egui::Rect::from_min_size(
                egui::pos2(rect.left() + offset, rect.center().y - 4.0),
                egui::vec2(width, 8.0),
            );
            ui.painter()
                .rect_filled(bar, egui::Rounding::same(2.0), color);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{} ms", dur_ms))
                        .color(C_MUTED)
                        .size(12.0),
                );
            });
        });
    };

    // Phase start offsets in ms (cumulative).
    let prepare_start = 0;
    let waiting_start = prepare_ms;
    let download_start = prepare_ms + waiting_ms;

    row(
        ui,
        "Prepare",
        prepare_start,
        prepare_ms,
        egui::Color32::from_rgb(120, 120, 120),
    );
    row(
        ui,
        "Waiting (TTFB)",
        waiting_start,
        waiting_ms,
        egui::Color32::from_rgb(74, 129, 232),
    );
    row(
        ui,
        "Download",
        download_start,
        download_ms,
        egui::Color32::from_rgb(130, 200, 120),
    );

    ui.add_space(2.0);
    ui.separator();
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(
            "DNS / TCP / TLS phases are rolled into Waiting — reqwest's \
             high-level client doesn't expose them individually.",
        )
        .size(10.5)
        .color(C_MUTED),
    );
}

/// Human-readable byte size — B / KB / MB. Single-decimal precision.
pub fn format_bytes(n: usize) -> String {
    if n < 1024 {
        format!("{} B", n)
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    }
}

/// Draws the Postman-style size-breakdown tooltip: Response Size on top,
/// Request Size below, each with Headers + Body rows. Meant to be
/// handed to `Response::on_hover_ui`.
pub fn render_size_breakdown(
    ui: &mut egui::Ui,
    response_headers_bytes: usize,
    response_body_bytes: usize,
    request_headers_bytes: usize,
    request_body_bytes: usize,
) {
    ui.set_min_width(240.0);
    let section = |ui: &mut egui::Ui,
                    title: &str,
                    total: usize,
                    headers: usize,
                    body: usize,
                    accent: egui::Color32| {
        // Section header — colored badge + title + total.
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(18.0, 18.0),
                egui::Sense::hover(),
            );
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(4.0), accent.linear_multiply(0.25));
            ui.label(
                egui::RichText::new(title)
                    .strong()
                    .color(C_TEXT)
                    .size(13.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format_bytes(total))
                        .strong()
                        .color(C_TEXT)
                        .size(13.0),
                );
            });
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.label(egui::RichText::new("Headers").color(C_MUTED).size(12.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format_bytes(headers))
                        .color(C_MUTED)
                        .size(12.0),
                );
            });
        });
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.label(egui::RichText::new("Body").color(C_MUTED).size(12.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format_bytes(body))
                        .color(C_MUTED)
                        .size(12.0),
                );
            });
        });
    };

    let resp_total = response_headers_bytes + response_body_bytes;
    let req_total = request_headers_bytes + request_body_bytes;
    // Arrow-down for Response (inbound), arrow-up for Request (outbound).
    section(
        ui,
        "Response Size",
        resp_total,
        response_headers_bytes,
        response_body_bytes,
        egui::Color32::from_rgb(74, 129, 232),
    );
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);
    section(
        ui,
        "Request Size",
        req_total,
        request_headers_bytes,
        request_body_bytes,
        egui::Color32::from_rgb(212, 175, 55),
    );
}

pub fn short_name_from_url(url: &str) -> String {
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let cutoff = stripped.find('?').unwrap_or(stripped.len());
    stripped[..cutoff].trim_end_matches('/').to_string()
}

#[allow(dead_code)]
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
