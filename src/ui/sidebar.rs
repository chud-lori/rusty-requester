//! Left sidebar: collection / folder tree, environment picker, history
//! view, and the top-level "new collection / import / export / search"
//! toolbar. All methods are `impl ApiClient` so they have direct
//! access to state.

use crate::model::*;
use crate::theme::*;
use crate::widgets::*;
use crate::ApiClient;
use eframe::egui;
use uuid::Uuid;

impl ApiClient {
    pub(crate) fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .default_width(320.0)
            .width_range(260.0..=640.0)
            .resizable(true)
            // Re-enabled — we now paint the sidebar with `panel_dark()`
            // (elevated card) which is BRIGHTER than the central canvas
            // in both themes. That gives a visible hierarchy (card lifts
            // off the canvas) so the separator line is no longer the
            // only cue between panels.
            .show_separator_line(true)
            .frame(
                egui::Frame::none()
                    .fill(panel_dark())
                    // Asymmetric: less right-padding so sidebar content
                    // butts close to the (invisible) boundary with central.
                    // On macOS, extra top padding so the traffic-light
                    // window controls — now overlaid on our chrome via
                    // `set_macos_titlebar_transparent` — don't collide
                    // with the "Rusty Requester" header row.
                    .inner_margin(egui::Margin {
                        left: 10.0,
                        right: 4.0,
                        top: if cfg!(target_os = "macos") {
                            32.0
                        } else {
                            10.0
                        },
                        bottom: 10.0,
                    }),
            )
            .show(ctx, |ui| {
                // Defensive floor — paint the full sidebar rect before any
                // children, so scroll tracks / code editors etc. don't
                // surface with egui-default near-black fills. Uses
                // `panel_dark()` so sub-regions match the sidebar card
                // color, not the central-panel canvas.
                ui.painter()
                    .rect_filled(ui.max_rect(), egui::Rounding::ZERO, panel_dark());
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if let Some(tex) = &self.app_icon {
                        ui.add(
                            egui::Image::from_texture(tex)
                                .fit_to_exact_size(egui::vec2(24.0, 24.0))
                                .rounding(egui::Rounding::same(5.0)),
                        );
                    }
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Rusty Requester")
                                .size(15.0)
                                .strong()
                                .color(text()),
                        );
                        // Baked-in build version + optional
                        // "update available" pill — stays visible
                        // for the whole session so the user never
                        // forgets a pending update.
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            ui.label(
                                egui::RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                                    .size(10.5)
                                    .color(muted()),
                            );
                            if let Some(latest) = self.update_available.clone() {
                                // Hide the pill once the user has
                                // explicitly dismissed THIS version —
                                // avoids pill-fatigue for people who
                                // defer updates. Reappears when a
                                // newer tag shows up.
                                let suppressed =
                                    self.state.settings.dismissed_update_version.as_deref()
                                        == Some(latest.as_str());
                                if !suppressed {
                                    // Phosphor arrow glyph — egui's bundled
                                    // font lacks U+2191 (↑) and renders it
                                    // as a tofu square.
                                    let pill = egui::Button::new(
                                        egui::RichText::new(format!(
                                            "{} {}",
                                            egui_phosphor::regular::ARROW_UP,
                                            latest
                                        ))
                                        .size(10.0)
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                    )
                                    .fill(accent())
                                    .stroke(egui::Stroke::NONE)
                                    .rounding(egui::Rounding::same(4.0))
                                    .min_size(egui::vec2(0.0, 18.0));
                                    if ui
                                        .add(pill)
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .on_hover_text("Click to see update instructions")
                                        .clicked()
                                    {
                                        self.show_update_modal = true;
                                    }
                                    // Small ✕ to dismiss just this
                                    // version's pill without opening
                                    // the modal. Uses Phosphor X.
                                    let dismiss = egui::Button::new(
                                        egui::RichText::new(egui_phosphor::regular::X)
                                            .size(10.0)
                                            .color(muted()),
                                    )
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::NONE)
                                    .min_size(egui::vec2(14.0, 18.0));
                                    if ui
                                        .add(dismiss)
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .on_hover_text("Dismiss (reappears on next version)")
                                        .clicked()
                                    {
                                        self.state.settings.dismissed_update_version =
                                            Some(latest.clone());
                                        self.save_state();
                                    }
                                }
                            }
                        });
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("⚙").size(14.0).color(muted()),
                                )
                                .min_size(egui::vec2(26.0, 24.0))
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Settings (timeout, body cap, proxy, TLS)")
                            .clicked()
                        {
                            self.editing_settings = self.state.settings.clone();
                            self.show_settings_modal = true;
                        }
                    });
                });
                ui.add_space(6.0);
                self.render_environment_picker(ui);
                ui.add_space(8.0);
                // Tab toggle. Use fixed-size Buttons (not `selectable_label`)
                // so the width doesn't shift when the selected state changes.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let v = self.sidebar_view;
                    let tab_size = egui::vec2(130.0, 24.0);

                    let coll_label = format!("Collections ({})", self.state.folders.len());
                    let coll_selected = v == SidebarView::Collections;
                    // Selected pill: brighter text (theme-aware) on a
                    // tinted accent background. Accent on TEXT was at
                    // ~4:1 contrast and felt cramped.
                    let coll_btn = egui::Button::new(
                        egui::RichText::new(coll_label)
                            .size(12.0)
                            .strong()
                            .color(if coll_selected { text() } else { muted() }),
                    )
                    .fill(if coll_selected {
                        accent().linear_multiply(0.18)
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .stroke(egui::Stroke::NONE)
                    .rounding(egui::Rounding::same(5.0))
                    .min_size(tab_size);
                    if ui
                        .add(coll_btn)
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        self.sidebar_view = SidebarView::Collections;
                    }

                    let hist_label = format!("History ({})", self.state.history.len());
                    let hist_selected = v == SidebarView::History;
                    let hist_btn = egui::Button::new(
                        egui::RichText::new(hist_label)
                            .size(12.0)
                            .strong()
                            .color(if hist_selected { text() } else { muted() }),
                    )
                    .fill(if hist_selected {
                        accent().linear_multiply(0.18)
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .stroke(egui::Stroke::NONE)
                    .rounding(egui::Rounding::same(5.0))
                    .min_size(tab_size);
                    if ui
                        .add(hist_btn)
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        self.sidebar_view = SidebarView::History;
                    }
                });
                ui.add_space(8.0);
                if self.sidebar_view == SidebarView::History {
                    self.render_history_view(ui);
                    return;
                }

                if ui
                    .add_sized(
                        [ui.available_width(), 32.0],
                        egui::Button::new(
                            egui::RichText::new("➕  New Collection")
                                .size(13.0)
                                .color(egui::Color32::WHITE)
                                .strong(),
                        )
                        .fill(accent())
                        .rounding(egui::Rounding::same(8.0))
                        .stroke(egui::Stroke::NONE),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.state.folders.push(Folder {
                        id: Uuid::new_v4().to_string(),
                        name: format!("Collection {}", self.state.folders.len() + 1),
                        requests: vec![],
                        subfolders: vec![],
                        description: String::new(),
                    });
                    self.save_state();
                }

                ui.add_space(6.0);

                let mut action_import_file = false;
                let mut action_paste_curl = false;
                let mut action_export_json = false;
                let mut action_export_yaml = false;

                ui.horizontal(|ui| {
                    let btn_w = (ui.available_width() - 6.0) / 2.0;
                    ui.menu_button(
                        egui::RichText::new("📥 Import").size(12.0).color(text()),
                        |ui| {
                            ui.set_min_width(200.0);
                            if ui.button("Import collection file...").clicked() {
                                action_import_file = true;
                                ui.close_menu();
                            }
                            if ui.button("Paste cURL command...").clicked() {
                                action_paste_curl = true;
                                ui.close_menu();
                            }
                        },
                    )
                    .response
                    .on_hover_text("Import JSON / YAML / Postman / cURL");
                    let _ = btn_w;

                    ui.menu_button(
                        egui::RichText::new("📤 Export").size(12.0).color(text()),
                        |ui| {
                            ui.set_min_width(200.0);
                            let enabled = !self.state.folders.is_empty();
                            if ui
                                .add_enabled(enabled, egui::Button::new("Export all as JSON..."))
                                .clicked()
                            {
                                action_export_json = true;
                                ui.close_menu();
                            }
                            if ui
                                .add_enabled(enabled, egui::Button::new("Export all as YAML..."))
                                .clicked()
                            {
                                action_export_yaml = true;
                                ui.close_menu();
                            }
                        },
                    );
                });

                // Defer file-dialog actions to the next frame so the menu
                // popup has a chance to close visibly before `rfd` blocks
                // the main thread. The blocking dialog otherwise freezes
                // the menu in its "open" state on screen.
                if action_import_file {
                    self.pending_import = true;
                }
                if action_paste_curl {
                    self.show_paste_modal = true;
                    self.paste_curl_text.clear();
                    self.paste_error.clear();
                }
                if action_export_json {
                    self.pending_export_json = true;
                }
                if action_export_yaml {
                    self.pending_export_yaml = true;
                }

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let (icon_rect, _) =
                        ui.allocate_exact_size(egui::vec2(18.0, 24.0), egui::Sense::hover());
                    ui.painter().text(
                        icon_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        egui_phosphor::regular::MAGNIFYING_GLASS,
                        egui::FontId::proportional(15.0),
                        muted(),
                    );

                    // Always reserve the close-button slot (visible or as
                    // a phantom spacer) so the TextEdit's desired_width
                    // doesn't change when the user starts typing. Without
                    // this, the horizontal row claims more space than it
                    // had, and the resizable SidePanel expands to the
                    // right on every keystroke.
                    let clear_w = 22.0;
                    let spacing = ui.spacing().item_spacing.x;
                    let search_w = (ui.available_width() - clear_w - spacing).max(80.0);
                    let search_resp = ui.add_sized(
                        [search_w, 24.0],
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text(hint("Search (⌘K)")),
                    );
                    if self.focus_search_next_frame {
                        self.focus_search_next_frame = false;
                        search_resp.request_focus();
                    }
                    if self.search_query.is_empty() {
                        // Phantom slot — same footprint as `close_x_button`
                        // so the layout is stable across empty/typed states.
                        ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                    } else if close_x_button(ui, "Clear search").clicked() {
                        self.search_query.clear();
                    }
                });
                if !self.search_query.is_empty() {
                    let total =
                        count_matches(&self.state.folders, &self.search_query.to_lowercase());
                    ui.label(
                        egui::RichText::new(format!("{} match(es)", total))
                            .size(11.0)
                            .color(muted()),
                    );
                }

                ui.add_space(6.0);

                // Thin floating scrollbar — `floating = true` in the
                // global style (theme.rs) keeps it from reserving a
                // width column, so we can show the bar without the
                // "sidebar resizing on pointer move" jitter we used
                // to hit with `VisibleWhenNeeded` + non-floating.
                egui::ScrollArea::vertical()
                    .id_salt("sidebar_scroll")
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(
                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                    )
                    .show(ui, |ui| {
                        // Section header — small uppercase label so users can
                        // distinguish *collections* (top-level) from
                        // *folders* (nested) at a glance.
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("COLLECTIONS")
                                .size(10.5)
                                .strong()
                                .color(muted()),
                        );
                        ui.add_space(4.0);

                        let folders = self.state.folders.clone();
                        let query = self.search_query.to_lowercase();
                        for folder in &folders {
                            if !query.is_empty() && !folder_matches(folder, &query) {
                                continue;
                            }
                            self.render_folder(ui, folder, vec![folder.id.clone()], 0);
                        }
                    });
            });
    }

    fn render_environment_picker(&mut self, ui: &mut egui::Ui) {
        // Small uppercase section header, then the picker + gear on a row
        // of its own — much easier to scan than the cramped inline label.
        ui.label(
            egui::RichText::new("ENVIRONMENT")
                .size(10.5)
                .strong()
                .color(muted()),
        );
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            let active_name = self
                .state
                .active_env_id
                .as_ref()
                .and_then(|id| self.state.environments.iter().find(|e| &e.id == id))
                .map(|e| e.name.clone())
                .unwrap_or_else(|| "No environment".to_string());
            egui::ComboBox::from_id_salt("env_picker")
                .selected_text(egui::RichText::new(active_name).size(12.5).color(text()))
                .width(ui.available_width() - 40.0)
                .show_ui(ui, |ui| {
                    let mut new_id: Option<Option<String>> = None;
                    if ui
                        .selectable_label(self.state.active_env_id.is_none(), "No environment")
                        .clicked()
                    {
                        new_id = Some(None);
                    }
                    for env in &self.state.environments {
                        let selected = self.state.active_env_id.as_deref() == Some(&env.id);
                        if ui.selectable_label(selected, &env.name).clicked() {
                            new_id = Some(Some(env.id.clone()));
                        }
                    }
                    if let Some(v) = new_id {
                        self.state.active_env_id = v;
                        self.save_state();
                    }
                });
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("⚙").size(13.0).color(muted()))
                        .min_size(egui::vec2(28.0, 26.0))
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::new(1.0, border())),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Manage environments")
                .clicked()
            {
                self.show_env_modal = true;
                if self.selected_env_for_edit.is_none() {
                    self.selected_env_for_edit =
                        self.state.environments.first().map(|e| e.id.clone());
                }
            }
        });
    }

    fn render_history_view(&mut self, ui: &mut egui::Ui) {
        if self.state.history.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("No requests sent yet.")
                        .color(muted())
                        .size(12.0),
                );
            });
            return;
        }
        let mut clear = false;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{} entries (newest first)",
                    self.state.history.len()
                ))
                .size(11.0)
                .color(muted()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(egui::RichText::new("Clear").size(11.0).color(C_RED))
                    .clicked()
                {
                    clear = true;
                }
            });
        });
        ui.add_space(4.0);

        if clear {
            self.state.history.clear();
            self.save_state();
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("history_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let entries = self.state.history.clone();
                for entry in &entries {
                    let mc = method_color(&entry.method);
                    let sc = status_color(&entry.status);
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 50.0),
                        egui::Sense::click(),
                    );
                    if ui.is_rect_visible(rect) {
                        let bg = if resp.hovered() {
                            elevated()
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        ui.painter()
                            .rect_filled(rect, egui::Rounding::same(6.0), bg);

                        // Method as colored text (no pill bg)
                        let method_left = rect.left() + 10.0;
                        let method_y = rect.top() + 17.0;
                        let method_font = egui::FontId::new(10.0, egui::FontFamily::Proportional);
                        ui.painter().text(
                            egui::pos2(method_left, method_y),
                            egui::Align2::LEFT_CENTER,
                            format!("{}", entry.method),
                            method_font,
                            mc,
                        );

                        // Status + time on the right
                        let info = format!("{}  ·  {}ms", entry.status, entry.time_ms);
                        ui.painter().text(
                            egui::pos2(method_left + 46.0, method_y),
                            egui::Align2::LEFT_CENTER,
                            info,
                            egui::FontId::new(11.0, egui::FontFamily::Proportional),
                            sc,
                        );

                        // URL beneath
                        let url_font = egui::FontId::new(11.5, egui::FontFamily::Proportional);
                        let max_w = rect.width() - 16.0;
                        let elided = elide(&entry.url, max_w, &url_font, ui);
                        ui.painter().text(
                            egui::pos2(rect.left() + 8.0, rect.top() + 33.0),
                            egui::Align2::LEFT_TOP,
                            elided,
                            url_font,
                            text(),
                        );
                    }
                    ui.add_space(2.0);
                }
            });
    }

    pub(crate) fn render_folder(
        &mut self,
        ui: &mut egui::Ui,
        folder: &Folder,
        path: Vec<String>,
        depth: usize,
    ) {
        let is_renaming = self.renaming_folder_id.as_ref() == Some(&folder.id);
        let query = self.search_query.to_lowercase();
        let searching = !query.is_empty();

        // Visually separate collections (top-level) from folders (nested):
        //   • collections render without a folder glyph — just the chevron
        //     and a strong-weight name.
        //   • subfolders render with a small painter-drawn folder icon
        //     between the chevron and the name, so users can tell them
        //     apart at a glance (matching Postman / Insomnia).
        // A custom `.icon(...)` fn replaces CollapsingHeader's default
        // triangle too, because egui's default icon uses a glyph that
        // renders as tofu on some font setups — painter shapes are
        // always font-safe.
        let header_text = if is_renaming {
            String::new()
        } else {
            folder.name.clone()
        };
        let is_collection = depth == 0;
        let name_prefix = if is_collection || is_renaming {
            String::new()
        } else {
            // Reserve ~20px for the painter-drawn folder icon. Three
            // space chars were too narrow at 13pt, so the first letter
            // of the name overlapped the icon (see `paint_folder_icon`
            // call below — its center sits at rect.left() + 24).
            "      ".to_string()
        };
        let mut header = egui::CollapsingHeader::new(
            egui::RichText::new(format!("{}{}", name_prefix, header_text))
                .size(13.0)
                .color(text())
                .strong(),
        )
        .id_salt(&folder.id)
        .default_open(true)
        .icon(paint_folder_chevron);
        if searching {
            header = header.open(Some(true));
        }
        // Buttons-as-context: use right-click on the folder header to
        // add a request, add a subfolder, rename, duplicate, delete, etc.
        // Track what was chosen so we can apply it after the header closure.
        let mut action_add_request = false;
        let header_response = header.show(ui, |ui| {
            let mut to_delete: Option<String> = None;
            let mut to_duplicate: Option<String> = None;
            for (i, req) in folder.requests.iter().enumerate() {
                if searching
                    && !request_matches(req, &query)
                    && !folder.name.to_lowercase().contains(&query)
                {
                    continue;
                }
                let is_selected = self.selected_request_id.as_ref() == Some(&req.id);
                let mc = method_color(&req.method);
                // Shadow the outer `is_renaming` (which is for the folder
                // header) with one keyed on this request's id. Without this
                // shadow the row would fall back to the folder's flag and
                // the rename TextEdit would never appear.
                let is_renaming = self.renaming_request_id.as_deref() == Some(req.id.as_str());

                // Compact Postman-style row — also a drag source +
                // drop target for in-folder reordering. Cross-folder
                // drag isn't supported yet (would need extra path
                // matching + fallback if dropped in dead space).
                let row_h = 26.0;
                let (rect, resp) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), row_h),
                    egui::Sense::click_and_drag(),
                );
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                // Mark this row as the drag source the moment a drag
                // starts. `dnd_set_drag_payload` carries the row's
                // identity to the eventual drop site.
                if resp.drag_started() {
                    egui::DragAndDrop::set_payload::<DragPayload>(
                        ui.ctx(),
                        DragPayload {
                            folder_path: path.clone(),
                            request_id: req.id.clone(),
                            from_index: i,
                        },
                    );
                }
                // While being dragged, paint a faint accent border so
                // users see what's moving.
                if egui::DragAndDrop::has_payload_of_type::<DragPayload>(ui.ctx()) && resp.dragged()
                {
                    ui.painter().rect_stroke(
                        rect,
                        egui::Rounding::same(5.0),
                        egui::Stroke::new(1.5, accent()),
                    );
                }
                // While *another* row is being dragged and the pointer
                // is over us, draw the drop indicator (a thin accent
                // line above this row) and on release, perform the
                // reorder.
                if resp.hovered() && egui::DragAndDrop::has_payload_of_type::<DragPayload>(ui.ctx())
                {
                    let any_dragged = ui.ctx().input(|i| i.pointer.any_down());
                    if any_dragged {
                        ui.painter().line_segment(
                            [
                                egui::pos2(rect.left() + 4.0, rect.top() + 1.0),
                                egui::pos2(rect.right() - 4.0, rect.top() + 1.0),
                            ],
                            egui::Stroke::new(2.0, accent()),
                        );
                    }
                }
                if let Some(payload) = resp.dnd_release_payload::<DragPayload>() {
                    if payload.folder_path == path && payload.from_index != i {
                        self.reorder_request_in_folder(&path, payload.from_index, i);
                    }
                }

                if ui.is_rect_visible(rect) {
                    let bg = if is_selected {
                        accent().linear_multiply(0.18)
                    } else if resp.hovered() {
                        elevated()
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    ui.painter()
                        .rect_filled(rect, egui::Rounding::same(5.0), bg);

                    if is_selected {
                        let bar =
                            egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height()));
                        ui.painter()
                            .rect_filled(bar, egui::Rounding::same(2.0), accent());
                    }

                    // Method as bold colored TEXT (no pill background).
                    // Reserve a fixed-width slot so all names align
                    // regardless of method length (GET vs OPTIONS).
                    let method_slot_w = 46.0;
                    let method_left = rect.left() + 10.0;
                    ui.painter().text(
                        egui::pos2(method_left, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        format!("{}", req.method),
                        egui::FontId::new(10.0, egui::FontFamily::Proportional),
                        mc,
                    );

                    if !is_renaming {
                        let name_x = method_left + method_slot_w;
                        let name_pos = egui::pos2(name_x, rect.center().y);
                        let font = egui::FontId::new(12.5, egui::FontFamily::Proportional);
                        let max_w = rect.right() - name_x - 6.0;
                        let display_name = elide(&req.name, max_w, &font, ui);
                        ui.painter().text(
                            name_pos,
                            egui::Align2::LEFT_CENTER,
                            display_name,
                            font,
                            text(),
                        );
                    }
                }

                // Inline rename TextEdit overlay (clearly visible against the row)
                if is_renaming {
                    // Same geometry as the name: method_left (10) + method_slot_w (46)
                    let name_start = rect.left() + 10.0 + 46.0;
                    let edit_rect = egui::Rect::from_min_max(
                        egui::pos2(name_start - 2.0, rect.top() + 3.0),
                        egui::pos2(rect.right() - 4.0, rect.bottom() - 3.0),
                    );
                    // Visible background + accent border so the input is obvious.
                    ui.painter()
                        .rect_filled(edit_rect, egui::Rounding::same(4.0), panel_dark());
                    ui.painter().rect_stroke(
                        edit_rect,
                        egui::Rounding::same(4.0),
                        egui::Stroke::new(1.5, accent()),
                    );
                    let inner = edit_rect.shrink2(egui::vec2(6.0, 2.0));
                    let edit_resp = ui.put(
                        inner,
                        egui::TextEdit::singleline(&mut self.rename_request_text)
                            .desired_width(inner.width())
                            .frame(false)
                            .text_color(text())
                            .font(egui::FontId::new(13.0, egui::FontFamily::Proportional)),
                    );
                    if self.request_rename_focus_pending {
                        self.request_rename_focus_pending = false;
                        edit_resp.request_focus();
                    }
                    let (enter, escape) = ui.input(|i| {
                        (
                            i.key_pressed(egui::Key::Enter),
                            i.key_pressed(egui::Key::Escape),
                        )
                    });
                    // Canonical egui "Enter-to-submit" pattern: check
                    // `lost_focus() && enter` together. egui's singleline
                    // TextEdit de-focuses in the SAME frame Enter fires, so
                    // the earlier `enter && has_focus()` check would see
                    // `has_focus() == false` and silently drop the commit —
                    // the rename appeared to do nothing. (Issue #16.)
                    if edit_resp.lost_focus() && enter {
                        let id = req.id.clone();
                        let new_name = self.rename_request_text.trim().to_string();
                        if !new_name.is_empty() {
                            self.rename_request(&id, new_name);
                        }
                        self.renaming_request_id = None;
                    } else if escape || (edit_resp.lost_focus() && !enter) {
                        self.renaming_request_id = None;
                    }
                } else {
                    // Hand-rolled double-click: if user clicks the same
                    // request row twice within DOUBLE_CLICK_SECS, treat as
                    // a double-click and start rename. Single click opens
                    // the request after a small grace period.
                    if resp.clicked() {
                        const DOUBLE_CLICK_SECS: f64 = 0.4;
                        let now = ui.input(|i| i.time);
                        let is_double = self
                            .last_request_click
                            .as_ref()
                            .map(|(id, t)| id == &req.id && (now - t) < DOUBLE_CLICK_SECS)
                            .unwrap_or(false);
                        if is_double {
                            self.renaming_request_id = Some(req.id.clone());
                            self.rename_request_text = req.name.clone();
                            self.request_rename_focus_pending = true;
                            self.last_request_click = None;
                        } else {
                            self.open_request(path.clone(), req.id.clone());
                            self.last_request_click = Some((req.id.clone(), now));
                        }
                    }
                    let req_id_for_menu = req.id.clone();
                    let req_name_for_menu = req.name.clone();
                    resp.context_menu(|ui| {
                        if ui.button("Rename").clicked() {
                            self.renaming_request_id = Some(req_id_for_menu.clone());
                            self.rename_request_text = req_name_for_menu.clone();
                            self.request_rename_focus_pending = true;
                            ui.close_menu();
                        }
                        if ui.button("Duplicate").clicked() {
                            to_duplicate = Some(req_id_for_menu.clone());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Delete").clicked() {
                            to_delete = Some(req_id_for_menu.clone());
                            ui.close_menu();
                        }
                    });
                }

                ui.add_space(1.0);
            }

            if let Some(dup_id) = to_duplicate {
                self.selected_folder_path = path.clone();
                let mut new_req_opt = None;
                if let Some(f) = self.get_current_folder_mut() {
                    if let Some(original) = f.requests.iter().find(|r| r.id == dup_id).cloned() {
                        let mut copy = original;
                        copy.id = Uuid::new_v4().to_string();
                        copy.name = format!("{} (copy)", copy.name);
                        new_req_opt = Some(copy.id.clone());
                        f.requests.push(copy);
                    }
                }
                self.save_state();
                if let Some(new_id) = new_req_opt {
                    let p = path.clone();
                    self.open_request(p, new_id);
                    self.show_toast("Request duplicated");
                }
            }

            if let Some(del_id) = to_delete {
                self.selected_folder_path = path.clone();
                if let Some(f) = self.get_current_folder_mut() {
                    f.requests.retain(|r| r.id != del_id);
                }
                self.save_state();
                self.prune_stale_tabs();
            }

            for subfolder in &folder.subfolders {
                if searching && !folder_matches(subfolder, &query) {
                    continue;
                }
                let mut subpath = path.clone();
                subpath.push(subfolder.id.clone());
                self.render_folder(ui, subfolder, subpath, depth + 1);
            }
        });

        // For nested folders, paint a small folder glyph in the leading
        // space we reserved at the front of the header label. The icon
        // goes right after the chevron (~16px wide) and the 6-char
        // prefix keeps the name from overlapping it. Collections
        // (depth == 0) intentionally get no folder icon so it's obvious
        // which rows are top-level.
        if !is_collection && !is_renaming {
            let rect = header_response.header_response.rect;
            let icon_center = egui::pos2(rect.left() + 22.0, rect.center().y);
            ui.painter_at(rect).text(
                icon_center,
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::FOLDER_SIMPLE,
                egui::FontId::proportional(14.0),
                muted(),
            );
        }

        if is_renaming {
            // Header rect is tight to the label text. Stretch the rename
            // area to the sidebar's full width so the TextEdit + buttons
            // actually fit. Also: `✓`/`✖` unicode glyphs aren't in egui's
            // bundled font on some systems (tofu rectangles) — use
            // Phosphor CHECK / X which we already ship.
            let header_rect = header_response.header_response.rect;
            let right_edge = ui.max_rect().right();
            let rename_rect = egui::Rect::from_min_max(
                egui::pos2(header_rect.left() + 25.0, header_rect.top()),
                egui::pos2(right_edge - 4.0, header_rect.bottom()),
            );
            let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rename_rect));
            child_ui.horizontal(|ui| {
                let btn_size = egui::vec2(22.0, 22.0);
                // Reserve space for the two action buttons, everything
                // else goes to the TextEdit.
                let edit_width = (rename_rect.width() - (btn_size.x * 2.0) - 12.0).max(80.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.rename_folder_text)
                        .desired_width(edit_width)
                        .font(egui::TextStyle::Body),
                );
                let (enter, escape) = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::Enter),
                        i.key_pressed(egui::Key::Escape),
                    )
                });
                if response.lost_focus() && enter {
                    self.rename_folder(&folder.id, self.rename_folder_text.clone());
                    self.renaming_folder_id = None;
                } else if escape {
                    self.renaming_folder_id = None;
                }
                if ui
                    .add_sized(btn_size, egui::Button::new(egui_phosphor::regular::CHECK))
                    .clicked()
                {
                    self.rename_folder(&folder.id, self.rename_folder_text.clone());
                    self.renaming_folder_id = None;
                }
                if ui
                    .add_sized(btn_size, egui::Button::new(egui_phosphor::regular::X))
                    .clicked()
                {
                    self.renaming_folder_id = None;
                }
            });
        } else {
            let folder_id = folder.id.clone();
            let folder_name = folder.name.clone();
            let noun = if depth == 0 { "collection" } else { "folder" };
            let mut start_rename = false;
            let mut delete_folder = false;
            let mut duplicate_folder = false;
            let mut add_subfolder = false;

            // Inline `+` (add request) and `...` (overflow menu) on the
            // right edge of the header — the toolbar pattern from Postman.
            // `header_rect` is tight to the text width, so we pin to the
            // *ui's* right edge for the x position and use the header_rect
            // only for the y center.
            let header_rect = header_response.header_response.rect;
            let right_edge = ui.max_rect().right();
            let btn_size = egui::vec2(22.0, 22.0);
            let plus_rect = egui::Rect::from_min_size(
                egui::pos2(
                    right_edge - btn_size.x - 4.0,
                    header_rect.center().y - btn_size.y / 2.0,
                ),
                btn_size,
            );
            let dots_rect = egui::Rect::from_min_size(
                egui::pos2(plus_rect.left() - btn_size.x - 2.0, plus_rect.top()),
                btn_size,
            );

            let plus_id = ui.id().with(("folder_plus", &folder_id));
            let plus_resp = ui
                .interact(plus_rect, plus_id, egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if plus_resp.hovered() {
                ui.painter()
                    .rect_filled(plus_rect, egui::Rounding::same(4.0), elevated());
            }
            let plus_color = if plus_resp.hovered() { text() } else { muted() };
            ui.painter().text(
                plus_rect.center(),
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::PLUS,
                egui::FontId::proportional(14.0),
                plus_color,
            );
            plus_resp.clone().on_hover_text("Add request");
            if plus_resp.clicked() {
                action_add_request = true;
            }

            let dots_id = ui.id().with(("folder_dots", &folder_id));
            let dots_resp = ui
                .interact(dots_rect, dots_id, egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if dots_resp.hovered() {
                ui.painter()
                    .rect_filled(dots_rect, egui::Rounding::same(4.0), elevated());
            }
            let dots_color = if dots_resp.hovered() { text() } else { muted() };
            ui.painter().text(
                dots_rect.center(),
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::DOTS_THREE,
                egui::FontId::proportional(14.0),
                dots_color,
            );
            dots_resp.clone().on_hover_text("More options");

            let popup_id = ui.id().with(("folder_menu", &folder_id));
            if dots_resp.clicked() {
                ui.memory_mut(|m| m.toggle_popup(popup_id));
            }
            let mut open_overview = false;
            egui::popup::popup_below_widget(
                ui,
                popup_id,
                &dots_resp,
                egui::PopupCloseBehavior::CloseOnClick,
                |ui| {
                    ui.set_min_width(180.0);
                    if ui.button("Open overview").clicked() {
                        open_overview = true;
                    }
                    ui.separator();
                    if ui.button("Add request").clicked() {
                        action_add_request = true;
                    }
                    if ui
                        .button(format!(
                            "Add {}",
                            if depth == 0 { "folder" } else { "subfolder" }
                        ))
                        .clicked()
                    {
                        add_subfolder = true;
                    }
                    ui.separator();
                    if ui.button("Rename").clicked() {
                        start_rename = true;
                    }
                    if ui.button("Duplicate").clicked() {
                        duplicate_folder = true;
                    }
                    ui.separator();
                    if ui
                        .button(egui::RichText::new(format!("Delete {}", noun)).color(C_RED))
                        .clicked()
                    {
                        delete_folder = true;
                    }
                },
            );

            // Keep the right-click context menu in sync.
            header_response.header_response.context_menu(|ui| {
                if ui.button("Open overview").clicked() {
                    open_overview = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Add request").clicked() {
                    action_add_request = true;
                    ui.close_menu();
                }
                if ui
                    .button(format!(
                        "Add {}",
                        if depth == 0 { "folder" } else { "subfolder" }
                    ))
                    .clicked()
                {
                    add_subfolder = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Rename").clicked() {
                    start_rename = true;
                    ui.close_menu();
                }
                if ui.button("Duplicate").clicked() {
                    duplicate_folder = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui
                    .button(egui::RichText::new(format!("Delete {}", noun)).color(C_RED))
                    .clicked()
                {
                    delete_folder = true;
                    ui.close_menu();
                }
            });

            // "Add request" — create a new request inside THIS folder and
            // open it as a tab.
            if action_add_request {
                self.selected_folder_path = path.clone();
                let count = folder.requests.len() + 1;
                let new_req = Request {
                    id: Uuid::new_v4().to_string(),
                    name: format!("Request {}", count),
                    method: HttpMethod::GET,
                    url: "https://api.example.com".to_string(),
                    query_params: vec![],
                    headers: vec![],
                    cookies: vec![],
                    body: String::new(),
                    body_ext: None,
                    auth: Auth::None,
                    extractors: vec![],
                    assertions: vec![],
                };
                let new_id = new_req.id.clone();
                if let Some(f) = self.get_current_folder_mut() {
                    f.requests.push(new_req);
                }
                self.save_state();
                self.open_request(path.clone(), new_id);
            }
            if start_rename {
                self.renaming_folder_id = Some(folder_id.clone());
                self.rename_folder_text = folder_name;
            }
            if add_subfolder {
                self.selected_folder_path = path.clone();
                let subcount = folder.subfolders.len() + 1;
                if let Some(f) = self.get_current_folder_mut() {
                    f.subfolders.push(Folder {
                        id: Uuid::new_v4().to_string(),
                        name: format!("Folder {}", subcount),
                        requests: vec![],
                        subfolders: vec![],
                        description: String::new(),
                    });
                }
                self.save_state();
            }
            if duplicate_folder {
                self.duplicate_folder(&folder_id);
            }
            if open_overview {
                self.open_folder_overview(&folder_id);
            }
            if delete_folder {
                self.delete_folder(&folder_id);
            }
        }
    }

    /// Move the request at `from_index` to `to_index` within the
    /// folder at `path`. No-op if the path doesn't resolve or the
    /// indices are out-of-bounds. Persists immediately.
    pub(crate) fn reorder_request_in_folder(
        &mut self,
        path: &[String],
        from_index: usize,
        to_index: usize,
    ) {
        let Some(folder) = self.folder_at_path_mut(path) else {
            return;
        };
        if from_index >= folder.requests.len() || to_index >= folder.requests.len() {
            return;
        }
        if from_index == to_index {
            return;
        }
        let item = folder.requests.remove(from_index);
        let insert_at = if to_index > from_index {
            to_index - 1
        } else {
            to_index
        };
        folder.requests.insert(insert_at, item);
        self.save_state();
    }
}

/// In-flight payload during a request-row drag. Carries enough info
/// for the drop site to identify the source row + same-folder check.
/// `request_id` is informational (could be used for cross-folder
/// drops in the future); same-folder reorder uses `from_index`.
#[derive(Clone, Debug)]
pub(crate) struct DragPayload {
    pub folder_path: Vec<String>,
    #[allow(dead_code)]
    pub request_id: String,
    pub from_index: usize,
}
