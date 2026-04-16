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

/// Square X/close button with a red-tinted hover state.
pub fn close_x_button(ui: &mut egui::Ui, hover_text: &str) -> egui::Response {
    let size = egui::vec2(20.0, 20.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        if hovered {
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(4.0), C_RED.linear_multiply(0.35));
        }
        let color = if hovered { C_RED } else { muted() };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::regular::X,
            egui::FontId::proportional(13.0),
            color,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(hover_text)
}

pub fn tab_button<T: PartialEq + Copy>(ui: &mut egui::Ui, current: &mut T, value: T, label: &str) {
    let selected = *current == value;
    // Active tab: brighter text (theme-aware) — not the accent. The
    // accent goes on the underline only. Tinting text AND underline
    // with the accent was at ~4:1 contrast on dark bg (below WCAG AA)
    // and consistently felt cramped to users. VS Code / GitHub / Linear
    // all leave active-tab text plain + colored indicator.
    let rich = if selected {
        egui::RichText::new(label).color(text()).strong().size(13.0)
    } else {
        egui::RichText::new(label).color(muted()).size(13.0)
    };
    let btn = egui::Button::new(rich)
        .fill(egui::Color32::TRANSPARENT)
        .stroke(egui::Stroke::NONE)
        .rounding(egui::Rounding::same(6.0))
        .min_size(egui::vec2(90.0, 30.0));
    let resp = ui.add(btn).on_hover_cursor(egui::CursorIcon::PointingHand);
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
        let mid = (lo + hi).div_ceil(2);
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
            .color(muted()),
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
    // Small trailing gap so the × delete button doesn't sit flush
    // against the panel's right border.
    let right_margin = 12.0;
    let usable = avail - right_margin;
    let (val_w, desc_w) = if show_description {
        let total = (usable - cb_w - key_w - del_w - cell_pad * 4.0).max(200.0);
        (total * 0.55, total * 0.45)
    } else {
        let val = usable - cb_w - key_w - del_w - cell_pad * 3.0;
        (val.max(150.0), 0.0)
    };

    ui.horizontal(|ui| {
        ui.add_space(cb_w + cell_pad);
        ui.label(egui::RichText::new("KEY").size(10.0).color(muted()));
        ui.add_space(key_w - 20.0);
        ui.label(egui::RichText::new("VALUE").size(10.0).color(muted()));
        if show_description {
            ui.add_space(val_w - 30.0);
            ui.label(egui::RichText::new("DESCRIPTION").size(10.0).color(muted()));
        }
    });
    ui.add_space(2.0);
    ui.painter().line_segment(
        [
            egui::pos2(ui.cursor().left(), ui.cursor().top()),
            egui::pos2(ui.cursor().left() + ui.available_width(), ui.cursor().top()),
        ],
        egui::Stroke::new(1.0, border().linear_multiply(0.6)),
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
            panel_dark().linear_multiply(0.6)
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

                    let text_color = if row.enabled { text() } else { muted() };

                    let key_resp = ui.add_sized(
                        [key_w, row_h],
                        egui::TextEdit::singleline(&mut row.key)
                            .id(id_salt.with((i, "key")))
                            .hint_text(if is_last_blank { hint("Key") } else { hint("") })
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
                            .hint_text(if is_last_blank {
                                hint("Value")
                            } else {
                                hint("")
                            })
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
                                .hint_text(if is_last_blank {
                                    hint("Description")
                                } else {
                                    hint("")
                                })
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
    Duplicate,
    TogglePin,
}

#[allow(clippy::too_many_arguments)]
pub fn render_single_tab(
    ui: &mut egui::Ui,
    idx: usize,
    method: &HttpMethod,
    name: &str,
    url: &str,
    is_active: bool,
    is_draft: bool,
    is_pinned: bool,
) -> TabAction {
    let tab_height = 32.0;
    let tab_width = 180.0;
    let (rect, mut resp) =
        ui.allocate_exact_size(egui::vec2(tab_width, tab_height), egui::Sense::click());
    resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

    let mut action = TabAction::None;

    if ui.is_rect_visible(rect) {
        // Active tab has an elevated fill so it visibly "lifts" out
        // of the flat tab strip (strip and content share `bg()`).
        // Hovered inactives get a faint elevation too, but less
        // saturated. Inactive = transparent so the strip bg shows
        // through (matches Postman's flat chrome).
        let tab_bg = if is_active {
            elevated()
        } else if resp.hovered() {
            elevated().linear_multiply(0.5)
        } else {
            egui::Color32::TRANSPARENT
        };
        let rounding = egui::Rounding {
            nw: 8.0,
            ne: 8.0,
            sw: 0.0,
            se: 0.0,
        };
        ui.painter().rect_filled(rect, rounding, tab_bg);

        if is_active {
            let top = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 2.0));
            ui.painter().rect_filled(top, rounding, C_ACCENT);
        }

        // Method as bold colored text. `egui::TextStyle::Button` is
        // the same style base used by the URL-bar method combobox, so
        // tabs and URL bar show visually identical method labels (just
        // different sizes for the context).
        let mc = method_color(method);
        let method_str = format!("{}", method);
        let method_font = egui::FontId::new(10.5, egui::FontFamily::Proportional);
        let method_slot_w = 42.0;
        let mut pad_left = rect.left() + 12.0;
        let mid_y = rect.center().y;

        // Pin glyph (before the method) when the tab is pinned — small
        // accent-colored Phosphor pin. Shrinks the method label slot by
        // ~14 px so the rest of the layout just shifts right.
        if is_pinned {
            ui.painter().text(
                egui::pos2(pad_left, mid_y),
                egui::Align2::LEFT_CENTER,
                egui_phosphor::regular::PUSH_PIN_SIMPLE,
                egui::FontId::proportional(11.5),
                C_ACCENT,
            );
            pad_left += 14.0;
        }
        // Paint the text twice — second pass slightly offset — for a
        // faux-bold effect that matches RichText::strong() in the
        // URL bar combobox (which uses the same color).
        for dx in &[0.0_f32, 0.4] {
            ui.painter().text(
                egui::pos2(pad_left + dx, mid_y),
                egui::Align2::LEFT_CENTER,
                &method_str,
                method_font.clone(),
                mc,
            );
        }

        let name_x = pad_left + method_slot_w;
        let name_color = if is_active { text() } else { muted() };
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
            ui.painter()
                .circle_filled(egui::pos2(dot_x, mid_y), 3.5, C_ORANGE);
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
        let color = if hovered { C_RED } else { muted() };
        ui.painter().text(
            close_rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::regular::X,
            egui::FontId::proportional(12.0),
            color,
        );
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
                .color(text()),
        );
        ui.add_space(4.0);
        if tip_url.is_empty() {
            ui.label(
                egui::RichText::new("(no URL)")
                    .color(muted())
                    .size(11.0)
                    .italics(),
            );
        } else {
            ui.label(egui::RichText::new(&tip_url).color(muted()).size(11.0));
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
        if ui.button("Duplicate tab").clicked() {
            action = TabAction::Duplicate;
            ui.close_menu();
        }
        let pin_label = if is_pinned { "Unpin tab" } else { "Pin tab" };
        if ui.button(pin_label).clicked() {
            action = TabAction::TogglePin;
            ui.close_menu();
        }
        ui.separator();
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
        muted(),
        egui::Stroke::NONE,
    ));
}

/// Extension trait that lets any clickable `egui::Response` opt into
/// the pointing-hand cursor with a single short method call, so we
/// don't repeat `.on_hover_cursor(CursorIcon::PointingHand)` dozens
/// of times. Web browsers show the hand cursor on links/buttons by
/// default; egui doesn't — this just restores that expectation.
/// Retained (even though most call sites currently inline
/// `.on_hover_cursor(...)`) so new buttons can opt in with `.hand()`.
#[allow(dead_code)]
pub trait HandCursor {
    fn hand(self) -> Self;
}
#[allow(dead_code)]
impl HandCursor for egui::Response {
    fn hand(self) -> Self {
        self.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

/// Compact square icon button using a Phosphor icon glyph.
/// 22×20 clickable area — fits in toolbars alongside pills and filters.
pub fn icon_btn(ui: &mut egui::Ui, icon: &str, hover_text: &str) -> egui::Response {
    let size = egui::vec2(22.0, 20.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let color = if resp.hovered() { text() } else { muted() };
        if resp.hovered() {
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(4.0), elevated());
        }
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(15.0),
            color,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(hover_text)
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

/// Compact underlined text toggle used in the response Body toolbar
/// to switch between JSON / Tree / Raw views. No background, no
/// border — just colored text with an accent underline when active,
/// matching Postman's Pretty/Raw/Preview style (less visual weight
/// than a chunky pill, scans as a tabbed toggle).
pub fn body_view_pill(ui: &mut egui::Ui, current: &mut BodyView, value: BodyView, label: &str) {
    let is_active = *current == value;
    // Same rationale as `tab_button`: accent on underline only, text
    // stays readable at full contrast.
    let fg = if is_active { text() } else { muted() };
    let resp = ui
        .add(
            egui::Button::new(egui::RichText::new(label).size(11.5).color(fg).strong())
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE)
                .min_size(egui::vec2(42.0, 20.0))
                .rounding(egui::Rounding::same(3.0)),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if is_active {
        let rect = resp.rect;
        let y = rect.bottom() - 1.0;
        let pad = 6.0;
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + pad, y),
                egui::pos2(rect.right() - pad, y),
            ],
            egui::Stroke::new(1.5, C_ACCENT),
        );
    }
    if resp.clicked() {
        *current = value;
    }
}

/// Render a JSON `Value` as an interactive, collapsible tree. Objects
/// and arrays become expandable nodes; leaves are rendered inline as
/// `key: value` with syntax colors. `filter` (lowercase) hides any
/// subtree that has no matching key/value; an empty filter shows all.
///
/// Right-click any row to copy its dot/bracket path
/// (e.g. `data.items[0].id`) to the clipboard — slots directly into
/// extractors and assertions.
pub fn render_json_tree(
    ui: &mut egui::Ui,
    id: egui::Id,
    key: &str,
    value: &Value,
    filter: &str,
    depth: usize,
) {
    render_json_tree_inner(ui, id, key, value, filter, depth, "");
}

fn render_json_tree_inner(
    ui: &mut egui::Ui,
    id: egui::Id,
    key: &str,
    value: &Value,
    filter: &str,
    depth: usize,
    parent_path: &str,
) {
    if !filter.is_empty() && !subtree_matches(key, value, filter) {
        return;
    }

    let current_path = compose_json_path(parent_path, key);

    match value {
        Value::Object(map) => {
            let summary = format!(
                "{{...}} ({} key{})",
                map.len(),
                if map.len() == 1 { "" } else { "s" }
            );
            json_header_with_menu(ui, id, key, &summary, depth, &current_path, |ui| {
                for (k, v) in map {
                    render_json_tree_inner(ui, id.with(k), k, v, filter, depth + 1, &current_path);
                }
            });
        }
        Value::Array(items) => {
            let summary = format!(
                "[...] ({} item{})",
                items.len(),
                if items.len() == 1 { "" } else { "s" }
            );
            json_header_with_menu(ui, id, key, &summary, depth, &current_path, |ui| {
                for (i, v) in items.iter().enumerate() {
                    let sub_key = format!("[{}]", i);
                    render_json_tree_inner(
                        ui,
                        id.with(i),
                        &sub_key,
                        v,
                        filter,
                        depth + 1,
                        &current_path,
                    );
                }
            });
        }
        _ => {
            // Allocate a clickable row so we can attach a right-click
            // context menu to the leaf.
            let row_h = 18.0;
            let (rect, resp) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), row_h),
                egui::Sense::click(),
            );
            if ui.is_rect_visible(rect) {
                if resp.hovered() {
                    ui.painter()
                        .rect_filled(rect, egui::Rounding::same(3.0), elevated());
                }
                let mut x = rect.left() + 16.0 * depth as f32 + 18.0;
                let y = rect.center().y;
                let font = egui::FontId::monospace(12.5);
                if !key.is_empty() {
                    let key_text = format!("{}:", key);
                    let key_galley =
                        ui.painter()
                            .layout_no_wrap(key_text.clone(), font.clone(), C_ACCENT);
                    ui.painter().galley(
                        egui::pos2(x, y - key_galley.size().y / 2.0),
                        key_galley.clone(),
                        C_ACCENT,
                    );
                    x += key_galley.size().x + 6.0;
                }
                let (color, text) = json_leaf_style(value);
                ui.painter().text(
                    egui::pos2(x, y),
                    egui::Align2::LEFT_CENTER,
                    text,
                    font,
                    color,
                );
            }
            attach_path_menu(&resp, &current_path);
        }
    }
}

/// "data" + "items" → "data.items"; "data.items" + "[0]" → "data.items[0]";
/// empty parent + key "data" → "data"; empty key (root) → "".
fn compose_json_path(parent: &str, key: &str) -> String {
    if key.is_empty() {
        return parent.to_string();
    }
    if parent.is_empty() {
        // Top-level array index "[0]" stays "[0]"; top-level object key "data" stays "data".
        return key.to_string();
    }
    if key.starts_with('[') {
        format!("{}{}", parent, key)
    } else {
        format!("{}.{}", parent, key)
    }
}

/// Same as the original `json_header` but the entire header response
/// gets a right-click menu offering "Copy path".
fn json_header_with_menu(
    ui: &mut egui::Ui,
    id: egui::Id,
    key: &str,
    summary: &str,
    depth: usize,
    path: &str,
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
                .color(if key.is_empty() { muted() } else { text() })
                .font(egui::FontId::monospace(12.5)),
        )
        .id_salt(id)
        .default_open(depth < 2)
        .icon(paint_folder_chevron);
        let resp = head.show(ui, body);
        attach_path_menu(&resp.header_response, path);
    });
}

/// Attach a right-click "Copy path" menu to any clickable Response.
/// Empty paths (root) show the option as disabled to avoid copying
/// "" silently.
fn attach_path_menu(resp: &egui::Response, path: &str) {
    let path = path.to_string();
    let label = if path.is_empty() {
        "Copy path (root)".to_string()
    } else {
        format!("Copy path  ·  {}", short_path(&path))
    };
    resp.context_menu(|ui| {
        if ui.button(&label).clicked() {
            ui.ctx().output_mut(|o| o.copied_text = path.clone());
            ui.close_menu();
        }
    });
}

fn short_path(p: &str) -> String {
    const MAX: usize = 48;
    if p.len() <= MAX {
        p.to_string()
    } else {
        format!("…{}", &p[p.len() - (MAX - 1)..])
    }
}

// Original `json_header` is superseded by `json_header_with_menu`,
// which adds the right-click "Copy path" menu. Removed to avoid
// dead-code drift; restore from git history if needed.
#[allow(dead_code)]
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
                .color(if key.is_empty() { muted() } else { text() })
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
        Value::String(s) => (egui::Color32::from_rgb(230, 219, 116), format!("\"{}\"", s)),
        Value::Number(n) => (egui::Color32::from_rgb(174, 129, 255), n.to_string()),
        Value::Bool(b) => (egui::Color32::from_rgb(249, 38, 114), b.to_string()),
        Value::Null => (muted(), "null".to_string()),
        _ => (text(), v.to_string()),
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
                .color(text())
                .size(13.0),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{} ms", total_ms))
                    .strong()
                    .color(text())
                    .size(13.0),
            );
        });
    });
    ui.add_space(6.0);

    let scale = total_ms.max(1) as f32;
    let row = |ui: &mut egui::Ui, label: &str, start_ms: u64, dur_ms: u64, color: egui::Color32| {
        let row_h = 18.0;
        let bar_total_w: f32 = 140.0;
        ui.horizontal(|ui| {
            ui.add_sized(
                egui::vec2(130.0, row_h),
                egui::Label::new(egui::RichText::new(label).color(muted()).size(12.0)),
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
                        .color(muted())
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
        .color(muted()),
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
            let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
            ui.painter().rect_filled(
                rect,
                egui::Rounding::same(4.0),
                accent.linear_multiply(0.25),
            );
            ui.label(egui::RichText::new(title).strong().color(text()).size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format_bytes(total))
                        .strong()
                        .color(text())
                        .size(13.0),
                );
            });
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.label(egui::RichText::new("Headers").color(muted()).size(12.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format_bytes(headers))
                        .color(muted())
                        .size(12.0),
                );
            });
        });
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.label(egui::RichText::new("Body").color(muted()).size(12.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format_bytes(body))
                        .color(muted())
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
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_').to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Mask a secret token for display — shows first 8 and last 8 chars,
/// with a "···" separator. Strings under 20 chars render as a
/// row of asterisks so nothing leaks.
pub fn mask_token(token: &str) -> String {
    let n = token.chars().count();
    if n <= 20 {
        return "*".repeat(n.min(16));
    }
    let start: String = token.chars().take(8).collect();
    let end: String = token.chars().rev().take(8).collect();
    let end: String = end.chars().rev().collect();
    format!("{} ··· {}", start, end)
}
