use super::*;

impl ApiClient {
    /// Update-instructions modal — surfaced when the user clicks the
    /// "update available" pill. Gives them the exact `curl` one-liner
    /// (same `install.sh` that handles fresh installs — quit running
    /// app, download DMG, copy into /Applications, strip quarantine,
    /// re-register with Launch Services, relaunch). A single click
    /// copies it to the clipboard. Release notes link for the
    /// curious.
    pub(crate) fn render_update_modal(&mut self, ctx: &egui::Context) {
        if !self.show_update_modal {
            return;
        }
        let Some(latest) = self.update_available.clone() else {
            self.show_update_modal = false;
            return;
        };
        let mut open = self.show_update_modal;
        let curl_cmd =
            "curl -fsSL https://raw.githubusercontent.com/chud-lori/rusty-requester/main/install.sh | bash";
        let release_url = format!(
            "https://github.com/chud-lori/rusty-requester/releases/tag/{}",
            latest
        );
        let mut copy_curl = false;
        let mut open_releases = false;
        let mut start_in_app_update = false;
        let in_app_ok = in_app_update_supported();

        egui::Window::new(format!("Update to {}", latest))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([520.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Running v{}. A newer version ({}) is available.",
                        env!("CARGO_PKG_VERSION"),
                        latest
                    ))
                    .color(text())
                    .size(13.0),
                );
                ui.add_space(12.0);
                let update_bullet = |ui: &mut egui::Ui, label: &str| {
                    ui.horizontal(|ui| {
                        let (dot_rect, _) =
                            ui.allocate_exact_size(egui::vec2(12.0, 13.0), egui::Sense::hover());
                        ui.painter()
                            .circle_filled(dot_rect.center(), 2.0, muted());
                        ui.label(egui::RichText::new(label).color(muted()).size(10.5));
                    });
                };
                if in_app_ok {
                    // macOS / Linux: "Update now" handles everything,
                    // so the user doesn't need to see the curl
                    // one-liner. Just describe what'll happen.
                    ui.label(
                        egui::RichText::new("Clicking Update now will:")
                        .color(muted())
                        .size(10.5),
                    );
                    update_bullet(ui, "Download the new build in the background");
                    update_bullet(ui, "Quit and replace the running app automatically");
                    update_bullet(ui, "Strip Gatekeeper quarantine (macOS) and relaunch");
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(
                            "Your data (data.json — collections, history, OAuth tokens, env vars) \
                             is untouched. You'll see a live log while it runs.",
                        )
                        .color(muted())
                        .size(10.5),
                    );
                } else {
                    // Windows: show the install command so the user
                    // can paste it themselves.
                    ui.label(
                        egui::RichText::new(
                            "One-line installer — paste into your terminal to update:",
                        )
                        .color(muted())
                        .size(11.0),
                    );
                    ui.add_space(4.0);
                    egui::Frame::none()
                        .fill(panel_dark())
                        .stroke(egui::Stroke::new(1.0, border()))
                        .rounding(egui::Rounding::same(6.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut curl_cmd.to_string())
                                    .font(egui::FontId::monospace(11.5))
                                    .frame(false)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(2)
                                    .interactive(false),
                            );
                        });
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("The official one-line installer handles everything:")
                        .color(muted())
                        .size(10.5),
                    );
                    update_bullet(ui, "Quits the running app automatically");
                    update_bullet(ui, "Downloads the new build from GitHub Releases");
                    update_bullet(ui, "Replaces the installed binary");
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(
                            "Your data (data.json) is untouched. After it finishes, relaunch the app.",
                        )
                        .color(muted())
                        .size(10.5),
                    );
                }

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    // Primary action on macOS/Linux: run the install
                    // script in-process, tail its log, auto-relaunch.
                    // Windows users only see "Copy command" — see
                    // `in_app_update_supported` for why.
                    if in_app_ok
                        && ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Update now")
                                        .color(egui::Color32::WHITE)
                                        .strong(),
                                )
                                .fill(accent())
                                .min_size(egui::vec2(140.0, 30.0)),
                            )
                            .on_hover_text(
                                "Runs the install script in the background and \
                                 relaunches the app when done. You can watch the live \
                                 log in the next dialog.",
                            )
                            .clicked()
                    {
                        start_in_app_update = true;
                    }
                    // Windows-only fallback: no detached-spawn /
                    // auto-relaunch recipe there, so users still copy
                    // the install command and run it in PowerShell /
                    // WSL themselves. macOS / Linux users get the
                    // one-click "Update now" path above and never see
                    // this button.
                    if !in_app_ok
                        && ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Copy command")
                                        .color(egui::Color32::WHITE)
                                        .strong(),
                                )
                                .fill(accent())
                                .min_size(egui::vec2(140.0, 30.0)),
                            )
                            .clicked()
                    {
                        copy_curl = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("Release notes").color(text()))
                                .fill(elevated())
                                .min_size(egui::vec2(120.0, 30.0)),
                        )
                        .clicked()
                    {
                        open_releases = true;
                    }
                });
            });

        if start_in_app_update {
            self.spawn_update();
        }
        if copy_curl {
            ctx.output_mut(|o| o.copied_text = curl_cmd.to_string());
            self.show_toast("Update command copied — paste in your terminal");
        }
        if open_releases {
            // Best-effort platform-specific open. Silently ignores
            // errors — worst case the URL string is in the toast for
            // the user to copy manually.
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(&release_url).spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open")
                .arg(&release_url)
                .spawn();
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", &release_url])
                .spawn();
        }
        self.show_update_modal = open;
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_update_modal = false;
        }
    }

    pub(crate) fn render_export_secret_warning_modal(&mut self, ctx: &egui::Context) {
        let Some(warning) = self.export_secret_warning.clone() else {
            return;
        };

        let label = match warning.format {
            crate::io::Format::Json => "JSON",
            crate::io::Format::Yaml => "YAML",
        };
        let mut open = true;
        let mut decision: Option<ExportDecision> = None;
        let mut cancel = false;

        egui::Window::new("Likely Secrets in Export")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([560.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "This {} export appears to include {} likely secret{}.",
                        label,
                        warning.findings.len(),
                        if warning.findings.len() == 1 { "" } else { "s" }
                    ))
                    .color(text())
                    .size(13.0),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(
                        "Scanning happens only on this device. Redacted export replaces likely secret values with [REDACTED].",
                    )
                    .color(muted())
                    .size(10.5),
                );
                ui.add_space(10.0);

                egui::Frame::none()
                    .fill(panel_dark())
                    .stroke(egui::Stroke::new(1.0, border()))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        for finding in warning.findings.iter().take(6) {
                            ui.label(
                                egui::RichText::new(finding.summary())
                                    .color(muted())
                                    .size(10.5),
                            );
                        }
                        if warning.findings.len() > 6 {
                            ui.label(
                                egui::RichText::new(format!(
                                    "...and {} more",
                                    warning.findings.len() - 6
                                ))
                                .color(muted())
                                .size(10.5),
                            );
                        }
                    });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Export redacted")
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .fill(accent())
                            .rounding(egui::Rounding::same(6.0)),
                        )
                        .clicked()
                    {
                        decision = Some(ExportDecision {
                            format: warning.format,
                            redacted: true,
                        });
                    }
                    if ui.button("Export original").clicked() {
                        decision = Some(ExportDecision {
                            format: warning.format,
                            redacted: false,
                        });
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });

        if decision.is_some() || cancel || !open {
            self.export_secret_warning = None;
        }
        if let Some(decision) = decision {
            self.pending_export_decision = Some(decision);
        }
    }

    /// Live progress modal shown while the one-click in-app updater
    /// is running. Streams the last few KB of `update.log` so users
    /// can see download progress, file copies, gatekeeper prompts,
    /// etc. — same info they'd see if they ran the curl command in a
    /// terminal. `install.sh` will kill us partway through (normal —
    /// it has to swap the binary); the wrapper survives and relaunches
    /// the app. On the next launch the post-update banner shows
    /// success/failure.
    ///
    /// No close button while in progress — closing the modal can't
    /// stop the wrapper anyway, and would just hide useful info.
    pub(crate) fn render_updating_modal(&mut self, ctx: &egui::Context) {
        if !self.updating_in_progress {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
        let log_path = update_log_path();
        let tail = self.update_log_tail.clone();
        let phase = infer_update_phase(&tail);
        let mut view_log = false;

        egui::Window::new("Updating rusty-requester…")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([500.0, 280.0])
            .show(ctx, |ui| {
                ui.set_width(464.0);
                ui.add_space(6.0);
                render_update_progress_strip(ui, ctx.input(|i| i.time));
                ui.add_space(14.0);
                render_update_steps(ui, phase);
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new(
                        "The installer is running in the background. The app may close and \
                         relaunch automatically while the binary is replaced.",
                    )
                    .color(muted())
                    .size(11.5),
                );
                ui.add_space(8.0);
                egui::Frame::none()
                    .fill(panel_dark())
                    .stroke(egui::Stroke::new(1.0, border()))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(10.0, 9.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            egui::RichText::new(phase.label())
                                .color(text())
                                .size(12.0)
                                .strong(),
                        );
                        ui.add_space(3.0);
                        let status = latest_update_status(&tail)
                            .unwrap_or_else(|| "Starting installer...".to_string());
                        let status_text = truncate_middle(&status, 92);
                        let status_rect = ui
                            .allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), 18.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_clip_rect(ui.max_rect());
                                    ui.label(
                                        egui::RichText::new(status_text)
                                            .color(muted())
                                            .font(egui::FontId::monospace(10.5)),
                                    );
                                },
                            )
                            .response;
                        status_rect.on_hover_text(status);
                        ui.add_space(4.0);
                        let log_label =
                            truncate_middle(&format!("Log: {}", log_path.display()), 78);
                        ui.label(
                            egui::RichText::new(log_label)
                                .color(muted())
                                .size(10.0)
                                .monospace(),
                        )
                        .on_hover_text(log_path.display().to_string());
                    });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("View log file").color(text()))
                                .fill(elevated())
                                .min_size(egui::vec2(130.0, 26.0)),
                        )
                        .clicked()
                    {
                        view_log = true;
                    }
                });
            });
        if view_log {
            open_update_log_in_os();
        }
    }

    /// Small banner anchored bottom-right after an in-app update
    /// completes, surfaced once on the next launch (or inline if the
    /// installer finished without killing us). Two states:
    ///
    /// - **Success**: green border, "Updated to vX.Y.Z" with a
    ///   "View log" button.
    /// - **Failure**: red border, the wrapper's reason string, and a
    ///   "View log" button so the user can debug.
    ///
    /// Success banner auto-dismisses after ~10 seconds (the update
    /// already worked — user got the confirmation, no action needed).
    /// Failure banner stays until the user clicks Dismiss so they
    /// don't miss the error or the View-log button.
    pub(crate) fn render_post_update_banner(&mut self, ctx: &egui::Context) {
        let Some((success, detail)) = self.post_update_notice.clone() else {
            // Banner gone — reset the timer so a future banner re-arms.
            self.post_update_notice_started_at = None;
            return;
        };
        let now = ctx.input(|i| i.time);
        let started = *self.post_update_notice_started_at.get_or_insert(now);
        const SUCCESS_AUTO_DISMISS_SECS: f64 = 10.0;
        if success && now - started >= SUCCESS_AUTO_DISMISS_SECS {
            self.post_update_notice = None;
            self.post_update_notice_started_at = None;
            return;
        }
        if success {
            // Ensure the auto-dismiss fires even when the app is
            // otherwise idle (no input, no scheduled repaint).
            let remaining = SUCCESS_AUTO_DISMISS_SECS - (now - started);
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(remaining.max(0.0)));
        }
        let mut dismiss = false;
        let mut view_log = false;

        let (border_color, title) = if success {
            (
                egui::Color32::from_rgb(76, 175, 80),
                // Phosphor CHECK — egui's bundled font has no U+2713
                // glyph, so a literal `✓` renders as a tofu square
                // (`sidebar.rs:926` already calls this out).
                format!("{}  Updated to v{}", egui_phosphor::regular::CHECK, detail),
            )
        } else {
            (
                egui::Color32::from_rgb(244, 67, 54),
                "Update failed".to_string(),
            )
        };

        egui::Area::new(egui::Id::new("post_update_banner"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(bg())
                    .stroke(egui::Stroke::new(1.5, border_color))
                    .rounding(10.0)
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_max_width(320.0);
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(&title)
                                    .color(text())
                                    .size(13.0)
                                    .strong(),
                            );
                            if !success {
                                ui.add_space(2.0);
                                ui.label(egui::RichText::new(&detail).color(muted()).size(11.0));
                            }
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("View log")
                                                .color(text())
                                                .size(11.0),
                                        )
                                        .fill(elevated())
                                        .min_size(egui::vec2(80.0, 22.0)),
                                    )
                                    .clicked()
                                {
                                    view_log = true;
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Dismiss")
                                                .color(muted())
                                                .size(11.0),
                                        )
                                        .fill(egui::Color32::TRANSPARENT)
                                        .min_size(egui::vec2(70.0, 22.0)),
                                    )
                                    .clicked()
                                {
                                    dismiss = true;
                                }
                            });
                        });
                    });
            });

        if view_log {
            open_update_log_in_os();
        }
        if dismiss {
            self.post_update_notice = None;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum UpdatePhase {
    Download,
    Install,
    Relaunch,
}

impl UpdatePhase {
    fn label(self) -> &'static str {
        match self {
            UpdatePhase::Download => "Downloading release",
            UpdatePhase::Install => "Installing update",
            UpdatePhase::Relaunch => "Preparing relaunch",
        }
    }
}

fn infer_update_phase(tail: &str) -> UpdatePhase {
    let lower = tail.to_ascii_lowercase();
    if lower.contains("relaunch")
        || lower.contains("launch services")
        || lower.contains("register")
        || lower.contains("starting")
    {
        UpdatePhase::Relaunch
    } else if lower.contains("copy")
        || lower.contains("install")
        || lower.contains("quarantine")
        || lower.contains("chmod")
    {
        UpdatePhase::Install
    } else {
        UpdatePhase::Download
    }
}

fn render_update_steps(ui: &mut egui::Ui, phase: UpdatePhase) {
    ui.horizontal(|ui| {
        for (index, (step, label)) in [
            (UpdatePhase::Download, "Download"),
            (UpdatePhase::Install, "Install"),
            (UpdatePhase::Relaunch, "Relaunch"),
        ]
        .iter()
        .enumerate()
        {
            let active = *step <= phase;
            let color = if active { accent() } else { muted() };
            ui.label(
                egui::RichText::new(format!("{} {}", egui_phosphor::regular::CIRCLE, label))
                    .color(color)
                    .size(11.0),
            );
            if index < 2 {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 1.0), egui::Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    egui::Rounding::same(1.0),
                    if active {
                        with_alpha(accent(), 140)
                    } else {
                        with_alpha(border(), 120)
                    },
                );
            }
        }
    });
}

fn render_update_progress_strip(ui: &mut egui::Ui, time: f64) {
    let width = ui.available_width().max(220.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 8.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::Rounding::same(4.0), elevated());
    ui.painter().rect_stroke(
        rect,
        egui::Rounding::same(4.0),
        egui::Stroke::new(1.0, border()),
    );

    let segment_width = rect.width() * 0.32;
    let travel = rect.width() + segment_width;
    let x = rect.left() - segment_width + ((time * 150.0) as f32 % travel);
    let segment = egui::Rect::from_min_size(
        egui::pos2(x, rect.top()),
        egui::vec2(segment_width, rect.height()),
    )
    .intersect(rect);
    ui.painter().rect_filled(
        segment,
        egui::Rounding::same(4.0),
        with_alpha(accent(), 210),
    );
}

fn latest_update_status(tail: &str) -> Option<String> {
    tail.split(['\n', '\r'])
        .rev()
        .map(strip_terminal_noise)
        .map(|line| line.trim().to_string())
        .find(|line| !line.is_empty() && !is_noisy_progress_line(line))
}

fn strip_terminal_noise(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if matches!(chars.peek(), Some('[')) {
                chars.next();
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if ch.is_control() && ch != '\t' {
            continue;
        }
        out.push(ch);
    }
    out
}

fn is_noisy_progress_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.starts_with("binary SHA256:") {
        return true;
    }
    let progress_chars = trimmed
        .chars()
        .filter(|ch| matches!(ch, '#' | '=' | '-' | 'O' | ' ' | '.' | '%') || ch.is_ascii_digit())
        .count();
    progress_chars * 100 / trimmed.chars().count().max(1) > 80
}

fn truncate_middle(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }
    let keep_each_side = max_chars.saturating_sub(1) / 2;
    let start: String = value.chars().take(keep_each_side).collect();
    let end: String = value
        .chars()
        .rev()
        .take(keep_each_side)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{start}…{end}")
}
