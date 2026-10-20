use crate::{
    service::Handle,
    theme::{self, ACCENT, ERROR, GREEN, MUTED, SURFACE, WARNING},
};
use eframe::egui::{self, RichText};
use reversebolt_core::{engine::View, live::ProcessInfo};
use serde_json::json;

#[derive(Default)]
pub struct LivePane {
    picker: bool,
    query: String,
    selected: Option<ProcessInfo>,
    pending: bool,
    focus_search: bool,
    image: Option<(String, egui::TextureHandle)>,
}

impl LivePane {
    pub fn open_picker(&mut self, handle: &Handle) {
        self.picker = true;
        self.pending = false;
        self.focus_search = true;
        self.selected = None;
        handle.submit("list_processes", json!({}));
    }

    pub fn picker(&mut self, ctx: &egui::Context, handle: &Handle, state: &View, busy: bool) {
        if !self.picker {
            return;
        }
        if self.pending
            && state.live.as_ref().is_some_and(|live| {
                live.connected
                    && self
                        .selected
                        .as_ref()
                        .is_some_and(|p| same_window(p, &live.process))
            })
        {
            self.picker = false;
            return;
        }
        let response =
            egui::Modal::new(egui::Id::new("application_picker")).show(ctx, |ui| {
                ui.set_width(590.0);
                ui.heading("Connect to application");
                ui.label(RichText::new("Choose a window to start investigating.").color(MUTED));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let id = ui.make_persistent_id("application_search");
                    // egui's text entry does not otherwise consume UIA SetValue.
                    ui.input_mut(|input| {
                        input.consume_accesskit_action_requests(id, |request| {
                            if request.action == egui::accesskit::Action::SetValue
                                && let Some(egui::accesskit::ActionData::Value(value)) =
                                    &request.data
                            {
                                self.query = value.chars().take(256).collect();
                                return true;
                            }
                            false
                        })
                    });
                    let search = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .id(id)
                            .hint_text("Search applications, titles or PID")
                            .desired_width(472.0),
                    );
                    if self.focus_search {
                        search.request_focus();
                        self.focus_search = false;
                    }
                    if ui
                        .add_enabled(!busy, egui::Button::new("Refresh"))
                        .clicked()
                    {
                        handle.submit("list_processes", json!({}));
                    }
                });
                ui.add_space(8.0);
                let query = self.query.trim().to_lowercase();
                let matches: Vec<_> = state
                    .processes
                    .iter()
                    .filter(|p| {
                        query.is_empty()
                            || format!("{} {} {}", p.name, p.title, p.pid)
                                .to_lowercase()
                                .contains(&query)
                    })
                    .collect();
                ui.label(
                    RichText::new(format!(
                        "{} available {}",
                        matches.len(),
                        if matches.len() == 1 {
                            "window"
                        } else {
                            "windows"
                        }
                    ))
                    .size(12.0)
                    .color(MUTED),
                );
                egui::ScrollArea::vertical()
                    .id_salt("application_list")
                    .auto_shrink([false, false])
                    .min_scrolled_height(230.0)
                    .max_height(280.0)
                    .show(ui, |ui| {
                        if matches.is_empty() {
                            ui.add_space(36.0);
                            ui.label(if busy {
                                "Looking for applications…"
                            } else if query.is_empty() {
                                "Open the application you want to explore, then refresh."
                            } else {
                                "No matching application. Try its name or window title."
                            });
                        }
                        ui.add_enabled_ui(!busy, |ui| {
                            for p in matches {
                                let selected =
                                    self.selected.as_ref().is_some_and(|s| same_window(s, p));
                                let label = format!("{}\n{}  ·  PID {}", p.title, p.name, p.pid);
                                let row = ui.add_sized(
                                    [ui.available_width(), 56.0],
                                    egui::Button::new(label).selected(selected),
                                );
                                if row.clicked() {
                                    self.selected = Some(p.clone());
                                    self.pending = false;
                                }
                            }
                        });
                    });
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "This preview observes the selected app. Edits are available in Practice.",
                    )
                    .size(12.0)
                    .color(MUTED),
                );
                if let Some(error) = &state.error {
                    ui.colored_label(ERROR, error);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if busy {
                        ui.spinner();
                        ui.label(if self.pending {
                            "Connecting…"
                        } else {
                            "Refreshing…"
                        });
                    } else if let Some(p) = &self.selected {
                        ui.label(RichText::new(&p.name).color(ACCENT));
                    } else {
                        ui.label(RichText::new("Select an application").color(MUTED));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let available = self
                            .selected
                            .as_ref()
                            .is_some_and(|s| state.processes.iter().any(|p| same_window(s, p)));
                        if ui
                            .add_enabled(!busy && available, theme::primary("Connect"))
                            .clicked()
                            && let Some(p) = &self.selected
                        {
                            handle.submit("attach_process", json!({
                            "pid":p.pid,"created":p.created,"window":p.window,"budget_mib":16
                        }));
                            self.pending = true;
                        }
                        if ui.button("Cancel").clicked() {
                            self.picker = false;
                        }
                    });
                });
            });
        if response.should_close() {
            self.picker = false;
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, handle: &Handle, state: &View, busy: bool) {
        ui.heading("Application workspace");
        ui.label(RichText::new("Connect. Observe a change. Follow the evidence.").color(MUTED));
        ui.add_space(16.0);
        let Some(live) = &state.live else {
            egui::Frame::new()
                .fill(SURFACE)
                .inner_margin(24)
                .corner_radius(8)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("What would you like to explore?")
                            .size(24.0)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    ui.label("Open a game or application, then connect its window here.");
                    ui.label("Capture moments as it changes to begin collecting evidence.");
                    ui.add_space(18.0);
                    if ui
                        .add_enabled(!busy, theme::primary("Connect to application"))
                        .clicked()
                    {
                        self.open_picker(handle);
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(
                            "Automatic understanding of gameplay is still in development.",
                        )
                        .size(13.0)
                        .color(MUTED),
                    );
                });
            if let Some(error) = &state.error {
                ui.colored_label(ERROR, error);
            }
            return;
        };
        egui::Frame::new()
            .fill(SURFACE)
            .inner_margin(16)
            .corner_radius(8)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&live.process.name).size(22.0).strong());
                    ui.label(
                        RichText::new(if live.connected {
                            "Connected · observation"
                        } else {
                            "Application closed"
                        })
                        .color(if live.connected {
                            GREEN
                        } else {
                            WARNING
                        }),
                    );
                });
                ui.label(&live.process.title);
                ui.label(
                    RichText::new(format!(
                        "PID {}  ·  {} moments captured",
                        live.process.pid, live.sample_count
                    ))
                    .size(12.0)
                    .color(MUTED),
                );
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            !busy && live.connected && live.sample_count < 20,
                            theme::primary("Capture moment"),
                        )
                        .clicked()
                    {
                        handle.submit("capture", json!({}));
                    }
                    if ui
                        .add_enabled(
                            !busy && live.connected && live.sample_count >= 12,
                            egui::Button::new("Compare changes"),
                        )
                        .clicked()
                    {
                        handle.submit("correlate", json!({}));
                    }
                    if ui
                        .add_enabled(!busy, egui::Button::new("Switch application"))
                        .clicked()
                    {
                        self.open_picker(handle);
                    }
                    if ui
                        .add_enabled(!busy, egui::Button::new("Disconnect"))
                        .clicked()
                    {
                        handle.submit("detach_process", json!({}));
                        self.image = None;
                    }
                });
            });
        ui.add_space(8.0);
        if busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Collecting evidence…");
            });
        }
        ui.label(RichText::new(&live.phase).color(MUTED));
        if let Some(error) = &state.error {
            ui.colored_label(ERROR, error);
        }
        if live.sample_count == 0 && live.connected {
            ui.add_space(16.0);
            ui.label("Play or interact with the app, then capture a moment here.");
            ui.label("Collect at least 12 moments with a changing on-screen number to compare it with memory.");
        }
        ui.add_space(8.0);
        if let Some(latest) = &live.latest {
            if let Some(error) = &latest.screen_error {
                ui.colored_label(WARNING, error);
            }
            if let Some(screen) = &latest.screen {
                if let Some(warning) = &screen.warning {
                    ui.colored_label(WARNING, warning);
                }
                if self
                    .image
                    .as_ref()
                    .is_none_or(|(path, _)| path != &screen.path)
                {
                    self.image = image::open(&screen.path).ok().map(|image| {
                        let image = image.thumbnail(960, 540).to_rgba8();
                        let pixels = egui::ColorImage::from_rgba_unmultiplied(
                            [image.width() as usize, image.height() as usize],
                            image.as_raw(),
                        );
                        (
                            screen.path.clone(),
                            ui.ctx().load_texture(
                                "application_capture",
                                pixels,
                                egui::TextureOptions::LINEAR,
                            ),
                        )
                    });
                }
                if let Some((_, image)) = &self.image {
                    ui.add(
                        egui::Image::new(image)
                            .max_width(ui.available_width().min(700.0))
                            .max_height(270.0),
                    );
                }
                egui::CollapsingHeader::new("Recognized text").show(ui, |ui| {
                    ui.label(if screen.text.is_empty() {
                        "No text recognized in this image."
                    } else {
                        &screen.text
                    });
                    for word in screen
                        .words
                        .iter()
                        .filter(|w| w.source != reversebolt_core::live::TextSource::WindowsOcr)
                        .take(100)
                    {
                        ui.label(format!("{:?}: {}", word.source, word.text));
                    }
                });
            }
        }
        if !live.tracks.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new("Observed numbers").size(18.0).strong());
            ui.label(
                RichText::new("Readings are provisional. Conflicting digits stay uncertain.")
                    .size(12.0)
                    .color(MUTED),
            );
            for track in live.tracks.iter().filter(|t| t.active) {
                egui::CollapsingHeader::new(format!(
                    "{}  ·  {}  ·  {} matches",
                    track.label,
                    track
                        .samples
                        .last()
                        .copied()
                        .flatten()
                        .map_or_else(|| if track.readings.last().is_some_and(|r|r.is_some()) {
                            "uncertain".into()
                        } else { "not visible".into() }, |v| v.to_string()),
                    track.matches.len()
                ))
                .id_salt(track.id)
                .show(ui, |ui| {
                    if let Some(Some(reading)) = track.readings.last() {
                        if reading.value.is_none() {
                            ui.colored_label(WARNING, "The readings disagree or cannot be assigned to one number. This moment is excluded from matching.");
                        }
                        for word in &reading.alternatives {
                            ui.label(format!("{:?}: “{}”", word.source, word.text.trim()));
                        }
                    }
                    ui.label(
                        RichText::new(
                            "Matches are correlations. Their gameplay effect has not been tested.",
                        )
                        .color(WARNING),
                    );
                    for candidate in track.matches.iter().take(30) {
                        ui.label(format!(
                            "{}  ·  {:?}",
                            candidate.address, candidate.mapping.kind
                        ));
                    }
                });
            }
        }
        egui::CollapsingHeader::new("Capture details").show(ui, |ui| {
            ui.label(format!("Memory coverage: {:.1} of {:.1} MiB eligible writable memory per moment.",
                live.planned_bytes as f64 / 1_048_576.0, live.eligible_bytes as f64 / 1_048_576.0));
            if let Some(latest) = &live.latest {
                ui.label(format!("{:.1} MiB read · {:.1} MiB unavailable · {} ms total",
                    latest.captured_bytes as f64 / 1_048_576.0, latest.unreadable_bytes as f64 / 1_048_576.0, latest.elapsed_ms));
                ui.label(format!("Image {} ms · memory {} ms · text recognition {} ms",
                    latest.screen_capture_ms, latest.memory_read_ms, latest.recognition_ms));
                if let Some(screen) = &latest.screen {
                    ui.label(format!("Capture method: {}",screen.method));
                }
            }
            ui.label("Screen and memory are captured in sequence while the app runs. Coverage is bounded; evidence may be incomplete.");
        });
    }
}

fn same_window(a: &ProcessInfo, b: &ProcessInfo) -> bool {
    a.pid == b.pid && a.created == b.created && a.window == b.window
}
