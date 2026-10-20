use crate::{
    service::Handle,
    theme::{self, ACCENT, ERROR, GREEN, MUTED, SURFACE, WARNING},
};
use eframe::egui::{self, RichText};
use reversebolt_core::{engine::View, live::ProcessInfo};
use serde_json::json;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CardSource {
    Ocr,
    Image,
    Temporal,
}

#[derive(Default)]
pub struct LivePane {
    picker: bool,
    query: String,
    selected: Option<ProcessInfo>,
    pending: bool,
    focus_search: bool,
    image: Option<(String, egui::TextureHandle)>,
    detail: Option<(String, CardSource, usize)>,
}

impl LivePane {
    pub fn open_picker(&mut self, handle: &Handle) {
        self.picker = true;
        self.pending = false;
        self.focus_search = true;
        self.selected = None;
        self.detail = None;
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
        let response = egui::Modal::new(egui::Id::new("application_picker")).show(ctx, |ui| {
            ui.set_width(590.0);
            ui.heading("Connect to application");
            ui.label(
                RichText::new("Choose a window. Observation starts when you connect.").color(MUTED),
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let id = ui.make_persistent_id("application_search");
                // egui's text entry does not otherwise consume UIA SetValue.
                ui.input_mut(|input| {
                    input.consume_accesskit_action_requests(id, |request| {
                        if request.action == egui::accesskit::Action::SetValue
                            && let Some(egui::accesskit::ActionData::Value(value)) = &request.data
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
                        handle.submit(
                            "attach_process",
                            json!({
                                "pid":p.pid,"created":p.created,"window":p.window,"budget_mib":16
                            }),
                        );
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
        let Some(live) = &state.live else {
            self.investigation(ui, handle, state, busy);
            return;
        };
        if self.detail.as_ref().is_some_and(|(scope, _, _)| {
            scope != &format!("{}:{}", live.investigation_id, live.coverage.batch)
        }) {
            self.detail = None;
        }
        ui.horizontal_wrapped(|ui| {
            ui.heading("Controls");
            ui.label(RichText::new(&live.process.title).color(MUTED));
            if ui
                .add_enabled(!busy, egui::Button::new("Change app"))
                .clicked()
            {
                self.open_picker(handle);
            }
            if ui
                .add_enabled(!busy, egui::Button::new("Disconnect"))
                .clicked()
            {
                handle.submit("detach_process", json!({}));
            }
        });
        ui.label(
            RichText::new("Each card is something Reversebolt is learning about your app.")
                .color(MUTED),
        );
        if !live.connected {
            ui.colored_label(
                WARNING,
                "Application closed. These readings are historical.",
            );
        }
        if let Some(error) = &state.error {
            ui.colored_label(ERROR, error);
        }
        ui.add_space(16.0);
        let mut cards: Vec<(CardSource, usize, String, String, String, bool)> = vec![];
        for track in live.tracks.iter().filter(|t| t.active) {
            let title = track
                .label
                .strip_prefix("Near “")
                .and_then(|s| s.strip_suffix('”'))
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Unidentified value {}", track.id + 1));
            let value = track.samples.last().copied().flatten();
            cards.push((
                CardSource::Ocr,
                track.id,
                title,
                value.map_or_else(|| "—".into(), |v| v.to_string()),
                "Visible in the app. Still identifying what it controls.".into(),
                value.is_some(),
            ));
        }
        for track in live.visual_tracks.iter().filter(|t| t.active) {
            let latest = track.observations.last().and_then(Option::as_ref);
            let readable = latest
                .is_some_and(|o| o.input.quality != reversebolt_core::visual::Quality::Uncertain);
            let value = latest
                .and_then(|o| o.input.value.as_ref())
                .filter(|_| readable)
                .map_or_else(|| "—".into(), |v| v.display());
            cards.push((
                CardSource::Image,
                track.id,
                track.label.clone(),
                value,
                "Interpreted from the app. Its effect is not verified yet.".into(),
                readable,
            ));
        }
        for track in live.temporal_tracks.iter().filter(|t| t.active) {
            let Some(latest) = track.observations.last() else {
                continue;
            };
            let current = latest
                .evidence
                .frames
                .last()
                .is_some_and(|f| f.number == live.sample_count);
            let readable =
                current && latest.input.quality == reversebolt_core::visual::Quality::Provisional;
            let value = latest
                .input
                .value
                .as_ref()
                .filter(|_| readable)
                .map_or_else(|| "—".into(), |v| v.display());
            cards.push((
                CardSource::Temporal,
                track.id,
                track.label.clone(),
                value,
                if current {
                    "Interpreted across several moments. Its effect is not verified."
                } else {
                    "An earlier change. Waiting for a fresh observation."
                }
                .into(),
                readable,
            ));
        }
        if cards.is_empty() {
            egui::Frame::new()
                .fill(SURFACE)
                .inner_margin(24)
                .corner_radius(8)
                .show(ui, |ui| {
                    ui.heading(if busy {
                        "Finding things you can change…"
                    } else {
                        "Nothing identified yet"
                    });
                    ui.label("Play normally while Reversebolt looks for useful values.");
                    if busy {
                        ui.spinner();
                    }
                });
        }
        let columns = if ui.available_width() >= 650.0 { 2 } else { 1 };
        for row in cards.chunks(columns) {
            ui.columns(columns, |cols| {
                for (column, (external, id, title, value, description, readable)) in
                    cols.iter_mut().zip(row)
                {
                    column.push_id((external, id), |ui| {
                        egui::Frame::new()
                            .fill(SURFACE)
                            .inner_margin(18)
                            .corner_radius(8)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_min_height(148.0);
                                ui.label(RichText::new(title).size(19.0).strong());
                                ui.label(RichText::new(description).size(13.0).color(MUTED));
                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(value).size(32.0).color(ACCENT));
                                    ui.label(
                                        RichText::new(if *readable {
                                            "Learning"
                                        } else {
                                            "Uncertain"
                                        })
                                        .size(12.0)
                                        .color(MUTED),
                                    );
                                });
                                if ui.button("Inspect").clicked() {
                                    self.detail = Some((
                                        format!(
                                            "{}:{}",
                                            live.investigation_id, live.coverage.batch
                                        ),
                                        *external,
                                        *id,
                                    ));
                                }
                            });
                    });
                }
            });
            ui.add_space(12.0);
        }
        if ui
            .add_enabled(
                !busy && live.connected && live.sample_count > 0 && live.coverage.batch < 24,
                egui::Button::new("Keep looking"),
            )
            .on_hover_text("Continue automatic discovery in another memory batch.")
            .clicked()
        {
            handle.submit("advance_memory",json!({"investigation_id":live.investigation_id,"batch":live.coverage.batch,"auto_observe":true}));
        }
        ui.add_space(12.0);
        egui::CollapsingHeader::new("Investigation details")
            .show(ui, |ui| self.investigation(ui, handle, state, busy));
        if let Some((_, external, id)) = self.detail.clone() {
            let card = cards
                .iter()
                .find(|(e, i, _, _, _, _)| *e == external && *i == id);
            if let Some((_, _, title, value, _, _)) = card {
                let response=egui::Modal::new(egui::Id::new("control_details")).show(ui.ctx(),|ui| {
                    ui.set_width(440.0);
                    ui.heading(title);
                    ui.label(RichText::new(value).size(32.0).color(ACCENT));
                    ui.label("Reversebolt can observe this value. It is still identifying the controlling state, so editing is not available yet.");
                    if external == CardSource::Temporal {
                        if let Some(track) = live.temporal_tracks.iter().find(|t| t.id == id)
                            && let Some(observation) = track.observations.last() {
                            ui.label(&observation.input.explanation);
                            ui.label(format!("Interpretation by {} · {}", track.observer,
                                observation.input.value.as_ref().map_or_else(|| "uncertain".into(), |v| v.display())));
                            ui.label(format!("{} sampled moments · {:.2}–{:.2} seconds apart",
                                observation.evidence.frames.len(), observation.evidence.elapsed_us[0] as f64 / 1_000_000., observation.evidence.elapsed_us[1] as f64 / 1_000_000.));
                            ui.label("Changes between these images may be missing. This does not verify continuous movement or a gameplay effect.");
                        }
                    } else {
                        let matches = if external == CardSource::Image { live.visual_tracks.iter().find(|t| t.id == id).map(|t| &t.matches) }
                            else { live.tracks.iter().find(|t| t.id == id).map(|t| &t.matches) };
                        ui.add_space(12.0);
                        prediction_button(ui, handle, live, busy, if external == CardSource::Image {"external"} else {"ocr"}, id, matches.is_some_and(|m| !m.is_empty()));
                    }
                    if ui.button("Close").clicked() {self.detail=None;}
                });
                if response.should_close() {
                    self.detail = None;
                }
            } else {
                self.detail = None;
            }
        }
    }

    fn investigation(&mut self, ui: &mut egui::Ui, handle: &Handle, state: &View, busy: bool) {
        ui.heading("Controls");
        ui.label(
            RichText::new("Connect and keep playing. Findings appear as the app changes.")
                .color(MUTED),
        );
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
                    ui.label("Reversebolt looks for useful values automatically as you play.");
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
                        "PID {}  ·  Memory batch {}  ·  {} moments captured",
                        live.process.pid, live.coverage.batch, live.sample_count
                    ))
                    .size(12.0)
                    .color(MUTED),
                );
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(!busy && live.connected && live.sample_count > 0 && live.coverage.batch < 24,
                        theme::primary("Continue discovery")).on_hover_text("Start another automatic pass over different memory. The current report is archived.").clicked() {
                        handle.submit("advance_memory",json!({"investigation_id":live.investigation_id,"batch":live.coverage.batch,"auto_observe":true}));
                        self.image=None;
                    }
                    if ui.add_enabled(!busy, egui::Button::new("Switch application")).clicked() {
                        self.open_picker(handle);
                    }
                    if ui.add_enabled(!busy, egui::Button::new("Disconnect")).clicked() {
                        handle.submit("detach_process", json!({}));
                        self.image = None;
                    }
                });
                egui::CollapsingHeader::new("Manual investigation tools").show(ui, |ui| {
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
                            !busy && live.connected && live.sample_count >= 12 && live.prediction_check.is_none(),
                            egui::Button::new("Compare changes"),
                        )
                        .clicked()
                    {
                        handle.submit("correlate", json!({}));
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    let remaining=(512*1024*1024/live.planned_bytes.max(1)).min(20).saturating_sub(live.sample_count).min(12);
                    if ui.add_enabled(!busy && live.connected && remaining>0,
                        egui::Button::new("Observe over time")).on_hover_text("Collect up to 12 captures, one second apart. Play or interact with the app while Reversebolt observes; pause or stop at any time.").clicked() {
                        handle.submit("start_observation_job",json!({"count":remaining,"interval_ms":1000}));
                    }
                    if ui.add_enabled(!busy && live.connected && live.sample_count>0 && live.coverage.batch<24,
                        egui::Button::new("Search more memory")).on_hover_text("Archive this batch's observations and findings, release its raw memory, and collect fresh evidence from other addresses.").clicked() {
                        handle.submit("advance_memory",json!({"investigation_id":live.investigation_id,"batch":live.coverage.batch}));
                        self.image=None;
                    }
                    ui.label(RichText::new(format!("{:.1} MiB read · {:.1} MiB with complete histories",
                        live.coverage.read_unique_bytes as f64/1_048_576.0,
                        live.coverage.complete_trajectory_bytes as f64/1_048_576.0)).size(12.0).color(MUTED));
                });
                });
            });
        ui.add_space(8.0);
        if busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Finding controls… You can keep playing.");
            });
        }
        ui.label(RichText::new(&live.phase).color(MUTED));
        if let Some(error) = &state.error {
            ui.colored_label(ERROR, error);
        }
        if live.sample_count == 0 && live.connected {
            ui.add_space(16.0);
            ui.label("Waiting for the first observation. Readings will appear here as they become available.");
        }
        let latest_numbers = live
            .tracks
            .iter()
            .filter(|t| t.active && t.samples.last().is_some_and(Option::is_some))
            .count();
        let possible_matches: usize = live.tracks.iter().map(|t| t.matches.len()).sum::<usize>()
            + live
                .visual_tracks
                .iter()
                .map(|t| t.matches.len())
                .sum::<usize>();
        ui.label(format!("{latest_numbers} provisional numeric readings · {possible_matches} possible memory matches"));
        ui.label(RichText::new("The first pass collects up to 16 moments. Comparisons begin after 12; pause or stop at any time. Matches remain unverified.").size(12.0).color(MUTED));
        ui.add_space(8.0);
        if !live.coverage.batches.is_empty() {
            egui::CollapsingHeader::new("Earlier memory batches").show(ui,|ui| {
                ui.label("Historical evidence. Addresses and gameplay effects remain unverified.");
                for batch in &live.coverage.batches {
                    ui.label(format!("Batch {} · {} captures · {:.1} MiB compared · {} hypotheses ({} byte witnesses saved)",
                        batch.number,batch.captures,batch.analyzed_bytes as f64/1_048_576.0,batch.hypotheses,batch.retained_witnesses));
                }
            });
        }
        if let Some(check) = &live.prediction_check {
            ui.add_space(12.0);
            ui.label(
                RichText::new(format!("Challenge: {}", check.label))
                    .size(18.0)
                    .strong(),
            );
            ui.label("Keep the value steady for two captures, then change it distinctly and capture again. Use normal gameplay. Predictions stay fixed.");
            ui.label(RichText::new(check.summary()).color(MUTED));
            ui.label(format!(
                "{} fresh moments · {} capture slots remain",
                live.sample_count.saturating_sub(check.after_frame),
                check.last_frame.saturating_sub(live.sample_count)
            ));
            if let Some(reason) = &check.invalidated {
                ui.colored_label(WARNING, reason);
            }
            egui::CollapsingHeader::new("Prediction evidence").show(ui, |ui| {
                ui.label("Agreement is limited to these provisional observations. It cannot distinguish a controlling value from a display copy.");
                for candidate in &check.candidates {
                    egui::CollapsingHeader::new(format!("{} · {:?} · {}", candidate.address,
                        candidate.mapping.kind, candidate.assessment(check).label()))
                        .id_salt((&check.id, &candidate.address, format!("{:?}", candidate.mapping.kind)))
                        .show(ui, |ui| {
                            if let Some(reason) = &candidate.invalidated { ui.colored_label(WARNING, reason); }
                            for sample in &candidate.samples {
                                let value = |v:Option<f64>| v.map_or_else(|| "missing".into(), |n| format!("{n:.2}"));
                                ui.label(format!("Capture {}: predicted {} · observed {} · {:?}",
                                    sample.frame, value(sample.predicted), value(sample.observed), sample.outcome));
                            }
                        });
                }
            });
        }
        if !live.visual_tracks.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new("Agent observations").size(18.0).strong());
            ui.label(
                RichText::new(
                    "Image interpretations with recorded sources. Gameplay effects are unverified.",
                )
                .size(12.0)
                .color(MUTED),
            );
            for track in &live.visual_tracks {
                let latest = track
                    .observations
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(i, o)| o.as_ref().map(|o| (i + 1, o)));
                let value = latest.map_or_else(
                    || "not observed".into(),
                    |(_, o)| {
                        if o.input.quality == reversebolt_core::visual::Quality::Uncertain {
                            "uncertain".into()
                        } else {
                            o.input
                                .value
                                .as_ref()
                                .map_or_else(|| "not observed".into(), |v| v.display())
                        }
                    },
                );
                egui::CollapsingHeader::new(format!(
                    "{} · {} {} · {}",
                    track.label,
                    value,
                    track.unit,
                    if track.active {
                        "unverified"
                    } else {
                        "historical"
                    }
                ))
                .id_salt(("visual", track.id))
                .show(ui, |ui| {
                    ui.label(format!(
                        "Reported by {} via {}",
                        track.observer, track.submitted_by
                    ));
                    if let Some((number, observation)) = latest {
                        ui.label(format!(
                            "Last reported in capture {number}; current capture {}.",
                            live.sample_count
                        ));
                        ui.label(&observation.input.explanation);
                        if observation.input.quality == reversebolt_core::visual::Quality::Uncertain
                        {
                            ui.colored_label(
                                WARNING,
                                "Uncertain interpretations are excluded from memory matching.",
                            );
                        }
                    }
                    ui.label(format!(
                        "{} memory correlations; none verified as controls.",
                        track.matches.len()
                    ));
                    prediction_button(
                        ui,
                        handle,
                        live,
                        busy,
                        "external",
                        track.id,
                        !track.matches.is_empty(),
                    );
                    for candidate in track.matches.iter().take(30) {
                        ui.label(format!(
                            "{} · {:?}",
                            candidate.address, candidate.mapping.kind
                        ));
                    }
                    egui::CollapsingHeader::new("Observation history").show(ui, |ui| {
                        for (i, observation) in track.observations.iter().enumerate() {
                            if let Some(o) = observation {
                                ui.label(format!(
                                    "Capture {} · {:?} · {}",
                                    i + 1,
                                    o.input.quality,
                                    o.input
                                        .value
                                        .as_ref()
                                        .map_or_else(|| "unreadable".into(), |v| v.display())
                                ));
                            }
                        }
                    });
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
                    prediction_button(ui, handle, live, busy, "ocr", track.id, !track.matches.is_empty());
                    for candidate in track.matches.iter().take(30) {
                        ui.label(format!(
                            "{}  ·  {:?}",
                            candidate.address, candidate.mapping.kind
                        ));
                    }
                });
            }
        }
        if let Some(latest) = &live.latest {
            if let Some(error) = &latest.screen_error {
                ui.colored_label(WARNING, error);
            }
            if let Some(screen) = &latest.screen {
                if let Some(warning) = &screen.warning {
                    ui.colored_label(WARNING, warning);
                }
                egui::CollapsingHeader::new("Latest captured image").show(ui, |ui| {
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
                });
            }
        }
        egui::CollapsingHeader::new("Capture details").show(ui, |ui| {
            ui.label(format!("Investigation: {:.1} MiB requested reads of 4096 MiB budget · {} of 24 batches",
                live.coverage.attempted_bytes as f64/1_048_576.0,live.coverage.batch));
            ui.label("Coverage counts addresses over time, not verified objects. Reused allocations can look unchanged. Each batch needs fresh changing observations.");
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

fn prediction_button(
    ui: &mut egui::Ui,
    handle: &crate::service::Handle,
    live: &reversebolt_core::live::LiveView,
    busy: bool,
    source: &str,
    track: usize,
    has_matches: bool,
) {
    if !has_matches || live.prediction_check.is_some() {
        return;
    }
    let remaining = (512 * 1024 * 1024 / live.planned_bytes.max(1))
        .min(20)
        .saturating_sub(live.sample_count);
    if ui.add_enabled(!busy && live.connected && remaining >= 4, egui::Button::new("Challenge these matches"))
        .on_hover_text("Freeze these predictions, then collect fresh captures with both stable values and distinct changes. At least four capture slots are needed. No memory edits are made.").clicked() {
        handle.submit("start_prediction_check", json!({"investigation_id":live.investigation_id,
            "batch":live.coverage.batch,"after_frame":live.sample_count,"source":source,"track":track}));
    }
}
