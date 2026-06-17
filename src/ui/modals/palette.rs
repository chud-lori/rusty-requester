use super::*;

impl ApiClient {
    /// Command palette (⌘P) — fuzzy-find across every request in
    /// every collection and jump to it. Modal Area-based popup with
    /// keyboard-only navigation (↑/↓, Enter to activate, Esc to
    /// dismiss). Matches VS Code / Sublime / fzf-style UX.
    pub(crate) fn render_command_palette(&mut self, ctx: &egui::Context) {
        if !self.show_command_palette {
            return;
        }
        // Build the match list — (path, request, display labels).
        let entries = collect_palette_entries(&self.state.folders);
        let query_lc = self.palette_query.to_lowercase();
        let matches: Vec<&PaletteEntry> = entries
            .iter()
            .filter(|e| {
                if query_lc.is_empty() {
                    true
                } else {
                    fuzzy_contains(&e.haystack_lc, &query_lc)
                }
            })
            .take(200)
            .collect();

        // Clamp selection to the visible matches (in case the user
        // typed and the filter shrunk the list).
        if self.palette_selected >= matches.len() {
            self.palette_selected = matches.len().saturating_sub(1);
        }

        let (enter, esc, arrow_up, arrow_down) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        if esc {
            self.show_command_palette = false;
            return;
        }
        if arrow_down && !matches.is_empty() {
            self.palette_selected = (self.palette_selected + 1) % matches.len();
        }
        if arrow_up && !matches.is_empty() {
            self.palette_selected = if self.palette_selected == 0 {
                matches.len() - 1
            } else {
                self.palette_selected - 1
            };
        }
        let mut activate: Option<(Vec<String>, String)> = None;
        if enter {
            if let Some(e) = matches.get(self.palette_selected) {
                activate = Some((e.folder_path.clone(), e.request_id.clone()));
            }
        }

        // Darken the background to draw focus.
        // No dim backdrop — VS Code / Raycast / Spotlight all forgo
        // one and rely on a shadowed floating panel to imply depth.
        // Earlier attempts with an `alpha` overlay fought the palette
        // for the same egui `Order::Middle` layer (dimming its content)
        // and generally looked heavy.

        let mut open = true;
        egui::Window::new(
            egui::RichText::new("COMMAND PALETTE")
                .size(11.0)
                .strong()
                .color(muted()),
        )
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .fixed_size(egui::vec2(560.0, 420.0))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 80.0))
        .frame(palette_frame(self.effective_theme()))
        .show(ctx, |ui| {
            let query_resp = ui.add(
                egui::TextEdit::singleline(&mut self.palette_query)
                    .hint_text(hint("Search requests by name, URL, or method…"))
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Body),
            );
            if self.palette_focus_pending {
                self.palette_focus_pending = false;
                query_resp.request_focus();
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!(
                    "{} result{}",
                    matches.len(),
                    if matches.len() == 1 { "" } else { "s" }
                ))
                .size(10.5)
                .color(muted()),
            );
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("palette_scroll")
                .max_height(320.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, m) in matches.iter().enumerate() {
                        let is_sel = i == self.palette_selected;
                        // Row height bumped from 34 → 44 so the
                        // breadcrumb has breathing room. Previously the
                        // secondary line sat flush against the bottom
                        // edge of the selection fill.
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 44.0),
                            egui::Sense::click(),
                        );
                        if ui.is_rect_visible(rect) {
                            // Softer accent-tinted fill for selected
                            // (translucent over the panel) plus a
                            // thin accent bar on the left. The prior
                            // `linear_multiply(0.18)` produced a dark
                            // saturated red block that read as a
                            // destructive state, not a selection.
                            let bg = if is_sel {
                                egui::Color32::from_rgba_unmultiplied(206, 66, 43, 36)
                            } else if resp.hovered() {
                                elevated()
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.painter()
                                .rect_filled(rect, egui::Rounding::same(5.0), bg);
                            if is_sel {
                                let bar = egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(3.0, rect.height()),
                                );
                                ui.painter().rect_filled(
                                    bar,
                                    egui::Rounding {
                                        nw: 5.0,
                                        sw: 5.0,
                                        ne: 0.0,
                                        se: 0.0,
                                    },
                                    accent(),
                                );
                            }
                            // Method
                            let mc = method_color(&m.method);
                            ui.painter().text(
                                egui::pos2(rect.left() + 10.0, rect.top() + 11.0),
                                egui::Align2::LEFT_TOP,
                                format!("{}", m.method),
                                egui::FontId::new(10.5, egui::FontFamily::Proportional),
                                mc,
                            );
                            ui.painter().text(
                                egui::pos2(rect.left() + 60.0, rect.top() + 8.0),
                                egui::Align2::LEFT_TOP,
                                &m.name,
                                egui::FontId::new(13.0, egui::FontFamily::Proportional),
                                text(),
                            );
                            ui.painter().text(
                                egui::pos2(rect.left() + 60.0, rect.top() + 26.0),
                                egui::Align2::LEFT_TOP,
                                &m.breadcrumb,
                                egui::FontId::new(10.5, egui::FontFamily::Proportional),
                                muted(),
                            );
                        }
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            activate = Some((m.folder_path.clone(), m.request_id.clone()));
                        }
                    }
                });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                // Phosphor arrow glyphs — the bundled Inter font lacks
                // U+2191 / U+2193 and used to render as "tofu" squares.
                ui.label(
                    egui::RichText::new(format!(
                        "{} {}  navigate    Enter  open    Esc  dismiss",
                        egui_phosphor::regular::ARROW_UP,
                        egui_phosphor::regular::ARROW_DOWN,
                    ))
                    .size(10.5)
                    .color(muted()),
                );
            });
        });
        if !open {
            self.show_command_palette = false;
        }
        if let Some((path, req_id)) = activate {
            self.show_command_palette = false;
            self.open_request(path, req_id);
        }
    }

    /// Actions palette (⇧⌘P) — the counterpart to `render_command_palette`.
    /// Same overlay chrome, but the list is `actions::PaletteAction::ALL`
    /// and Enter dispatches through `run_action` instead of opening a
    /// request.
    pub(crate) fn render_actions_palette(&mut self, ctx: &egui::Context) {
        if !self.show_actions_palette {
            return;
        }
        use crate::actions::PaletteAction;
        let query_lc = self.actions_palette_query.to_lowercase();
        let matches: Vec<&PaletteAction> = PaletteAction::ALL
            .iter()
            .filter(|a| {
                if query_lc.is_empty() {
                    true
                } else {
                    fuzzy_contains(&a.haystack_lc(), &query_lc)
                }
            })
            .collect();

        if self.actions_palette_selected >= matches.len() {
            self.actions_palette_selected = matches.len().saturating_sub(1);
        }

        let (enter, esc, arrow_up, arrow_down) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        if esc {
            self.show_actions_palette = false;
            return;
        }
        if arrow_down && !matches.is_empty() {
            self.actions_palette_selected = (self.actions_palette_selected + 1) % matches.len();
        }
        if arrow_up && !matches.is_empty() {
            self.actions_palette_selected = if self.actions_palette_selected == 0 {
                matches.len() - 1
            } else {
                self.actions_palette_selected - 1
            };
        }
        let mut activate: Option<PaletteAction> = None;
        if enter {
            if let Some(a) = matches.get(self.actions_palette_selected) {
                activate = Some(**a);
            }
        }

        // Dim backdrop matches the command palette for visual parity.
        // See `render_command_palette` for why there's no backdrop.

        let mut open = true;
        egui::Window::new(
            egui::RichText::new("ACTIONS")
                .size(11.0)
                .strong()
                .color(muted()),
        )
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .fixed_size(egui::vec2(560.0, 420.0))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 80.0))
        .frame(palette_frame(self.effective_theme()))
        .show(ctx, |ui| {
            let query_resp = ui.add(
                egui::TextEdit::singleline(&mut self.actions_palette_query)
                    .hint_text(hint("Run an action…"))
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Body),
            );
            if self.actions_palette_focus_pending {
                self.actions_palette_focus_pending = false;
                query_resp.request_focus();
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!(
                    "{} action{}",
                    matches.len(),
                    if matches.len() == 1 { "" } else { "s" }
                ))
                .size(10.5)
                .color(muted()),
            );
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("actions_palette_scroll")
                .max_height(320.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, a) in matches.iter().enumerate() {
                        let is_sel = i == self.actions_palette_selected;
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 32.0),
                            egui::Sense::click(),
                        );
                        if ui.is_rect_visible(rect) {
                            // Match the command-palette treatment —
                            // softer tint + left accent bar for
                            // selection (was a saturated red block).
                            let bg = if is_sel {
                                egui::Color32::from_rgba_unmultiplied(206, 66, 43, 36)
                            } else if resp.hovered() {
                                elevated()
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.painter()
                                .rect_filled(rect, egui::Rounding::same(5.0), bg);
                            if is_sel {
                                let bar = egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(3.0, rect.height()),
                                );
                                ui.painter().rect_filled(
                                    bar,
                                    egui::Rounding {
                                        nw: 5.0,
                                        sw: 5.0,
                                        ne: 0.0,
                                        se: 0.0,
                                    },
                                    accent(),
                                );
                            }
                            ui.painter().text(
                                egui::pos2(rect.left() + 14.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                a.label(),
                                egui::FontId::new(13.0, egui::FontFamily::Proportional),
                                text(),
                            );
                            if let Some(sc) = a.shortcut() {
                                ui.painter().text(
                                    egui::pos2(rect.right() - 14.0, rect.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    sc,
                                    egui::FontId::new(11.0, egui::FontFamily::Monospace),
                                    muted(),
                                );
                            }
                        }
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            activate = Some(**a);
                        }
                    }
                });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} {}  navigate    Enter  run    Esc  dismiss",
                        egui_phosphor::regular::ARROW_UP,
                        egui_phosphor::regular::ARROW_DOWN,
                    ))
                    .size(10.5)
                    .color(muted()),
                );
            });
        });
        if !open {
            self.show_actions_palette = false;
        }
        if let Some(action) = activate {
            self.show_actions_palette = false;
            self.run_action(action);
        }
    }
}

/// One row in the command palette result list.
struct PaletteEntry {
    folder_path: Vec<String>,
    request_id: String,
    name: String,
    method: HttpMethod,
    /// "Personal / api-v2 / GET" — shown as the secondary line in
    /// the palette so users see where the request lives.
    breadcrumb: String,
    /// Lowercased haystack used by the fuzzy matcher (name + URL +
    /// method + breadcrumb concatenated). Cached so we don't
    /// re-allocate per keystroke.
    haystack_lc: String,
}

fn collect_palette_entries(folders: &[Folder]) -> Vec<PaletteEntry> {
    let mut out = Vec::new();
    for folder in folders {
        walk_palette(
            folder,
            vec![folder.id.clone()],
            folder.name.clone(),
            &mut out,
        );
    }
    out
}

fn walk_palette(
    folder: &Folder,
    path: Vec<String>,
    breadcrumb: String,
    out: &mut Vec<PaletteEntry>,
) {
    for r in &folder.requests {
        let haystack = format!("{} {} {} {}", r.name, r.url, r.method, breadcrumb).to_lowercase();
        out.push(PaletteEntry {
            folder_path: path.clone(),
            request_id: r.id.clone(),
            name: r.name.clone(),
            method: r.method.clone(),
            breadcrumb: format!("{} · {}", breadcrumb, r.url),
            haystack_lc: haystack,
        });
    }
    for sub in &folder.subfolders {
        let mut sub_path = path.clone();
        sub_path.push(sub.id.clone());
        let sub_breadcrumb = format!("{} / {}", breadcrumb, sub.name);
        walk_palette(sub, sub_path, sub_breadcrumb, out);
    }
}

/// Tiny "subsequence" fuzzy matcher — every char of `needle` (already
/// lowercase) must appear somewhere in `haystack` in order. Same
/// algorithm fzf falls back to. Cheap, no scoring, good enough for
/// palette filtering.
fn fuzzy_contains(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let mut chars = needle.chars();
    let mut want = match chars.next() {
        Some(c) => c,
        None => return true,
    };
    for c in haystack.chars() {
        if c == want {
            match chars.next() {
                Some(next) => want = next,
                None => return true,
            }
        }
    }
    false
}

/// Frame styling for ⌘P / ⇧⌘P palette windows. Uses `elevated()`
/// instead of the default `bg()` so the palette visibly floats above
/// the darkened backdrop — without this they blend into the app
/// chrome and look "greyed-out" rather than focused.
fn palette_frame(theme: Theme) -> egui::Frame {
    // VS Code / Raycast-style palette frame: sits directly on top of
    // the unmodified UI (no backdrop), separated only by a slight
    // elevation + subtle border + punchy drop shadow. Fill is a shade
    // brighter-than-panel in dark mode, near-white in light.
    let (fill, border) = match theme {
        Theme::Dark => (
            // `#252830` — one notch brighter than `elevated()` (#2A2D34)
            // looks muddy against other dark chrome, so we nudge a
            // touch cooler. This matches VS Code's "Quick Input" bg.
            egui::Color32::from_rgb(37, 40, 48),
            egui::Color32::from_rgb(60, 64, 72),
        ),
        Theme::Light => (
            egui::Color32::from_rgb(253, 253, 254),
            egui::Color32::from_rgb(208, 212, 220),
        ),
        Theme::Postman => (
            egui::Color32::from_rgb(255, 255, 255),
            egui::Color32::from_rgb(221, 221, 224),
        ),
    };
    egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, border))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(14.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 10.0),
            blur: 28.0,
            spread: 0.0,
            // Heavier shadow than a regular modal — the palette has
            // no backdrop so the shadow alone carries the "floating"
            // read.
            color: egui::Color32::from_black_alpha(match theme {
                Theme::Dark => 180,
                Theme::Light => 80,
                Theme::Postman => 60,
            }),
        })
}
