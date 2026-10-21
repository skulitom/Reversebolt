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

#[derive(Clone)]
struct CardDrag {
    scope: String,
    key: (CardSource, usize),
}

type FindingCard = (
    CardSource,
    usize,
    String,
    String,
    String,
    bool,
    Option<String>,
);

fn action_evidence(
    live: &reversebolt_core::live::LiveView,
    source: CardSource,
    id: usize,
) -> Option<&reversebolt_core::action_patterns::Evidence> {
    use reversebolt_core::action_patterns::{self, Source};
    action_patterns::find(
        live,
        match source {
            CardSource::Ocr => Source::Ocr,
            CardSource::Image => Source::External,
            CardSource::Temporal => return None,
        },
        id,
    )
}

fn brief_description(text: &str) -> String {
    let sentence = text.split(". ").next().unwrap_or(text);
    let words = sentence
        .split_whitespace()
        .take(20)
        .collect::<Vec<_>>()
        .join(" ");
    let mut brief = words.chars().take(150).collect::<String>();
    if brief.len() < sentence.len() {
        brief.push('…');
    }
    brief
}

pub(crate) fn observation_age(observed: u128, now: u128) -> String {
    let Some(age) = now.checked_sub(observed) else {
        return "Observation time unavailable".into();
    };
    let seconds = age / 1000;
    if seconds < 2 {
        "Seen just now".into()
    } else if seconds < 60 {
        format!("Seen {seconds}s ago")
    } else if seconds < 3600 {
        format!("Seen {}m ago", seconds / 60)
    } else {
        format!("Seen {}h ago", seconds / 3600)
    }
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
    requested: std::collections::BTreeMap<usize, f64>,
    probe_choice: std::collections::BTreeMap<usize, usize>,
    edit_scope: String,
    card_order: Vec<(CardSource, usize)>,
    show_inspector: bool,
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
                    "Connecting only observes. Trial edits require a separate explicit action.",
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
        let edit_scope = format!("{}:{}", live.investigation_id, live.coverage.batch);
        if self.edit_scope != edit_scope {
            self.requested.clear();
            self.probe_choice.clear();
            let carried = live.prediction_checks.iter().any(|c| {
                c.rechecked_from.as_ref().is_some_and(|origin| {
                    self.edit_scope == format!("{}:{}", live.investigation_id, origin.batch)
                })
            });
            if !carried {
                self.card_order.clear();
            }
            self.show_inspector = false;
            self.edit_scope = edit_scope;
        }
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
        ui.label(RichText::new("Your controls, as they are discovered.").color(MUTED));
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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let mut cards: Vec<FindingCard> = vec![];
        for track in live.tracks.iter().filter(|t| t.active) {
            let title = track
                .label
                .strip_prefix("Near “")
                .and_then(|s| s.strip_suffix('”'))
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Unidentified value {}", track.id + 1));
            let trial_observation = state
                .changes
                .iter()
                .filter(|c| {
                    c.scope.investigation == live.investigation_id
                        && c.scope.batch == live.coverage.batch
                })
                .flat_map(|c| c.observations.iter().map(move |o| (c, o)))
                .filter_map(|(c, o)| {
                    o.readings
                        .iter()
                        .find(|r| r.track == track.id)
                        .map(|r| r.value)
                        .or_else(|| (c.scope.track == track.id).then_some(o.value))
                        .map(|value| (o, value))
                })
                .filter(|(o, _)| {
                    live.latest
                        .as_ref()
                        .is_none_or(|f| o.time_ms >= f.started_unix_ms)
                })
                .max_by_key(|(o, _)| o.time_ms);
            let value = trial_observation.map_or_else(
                || track.samples.last().copied().flatten(),
                |(_, value)| value,
            );
            cards.push((
                CardSource::Ocr,
                track.id,
                title,
                value.map_or_else(|| "—".into(), |v| v.to_string()),
                "On-screen value; its effect is still unknown.".into(),
                value.is_some(),
                trial_observation
                    .map(|(o, _)| observation_age(o.time_ms, now))
                    .or_else(|| {
                        live.frames
                            .last()
                            .map(|f| observation_age(f.started_unix_ms, now))
                    }),
            ));
        }
        for track in live.visual_tracks.iter().filter(|t| t.active) {
            let latest = track.observations.iter().rev().flatten().next();
            let readable = track.observations.last().is_some_and(Option::is_some)
                && latest.is_some_and(|o| {
                    o.input.quality != reversebolt_core::visual::Quality::Uncertain
                });
            let value = latest
                .filter(|o| o.input.quality != reversebolt_core::visual::Quality::Uncertain)
                .and_then(|o| o.input.value.as_ref())
                .map_or_else(|| "—".into(), |v| v.display());
            cards.push((
                CardSource::Image,
                track.id,
                track.label.clone(),
                value,
                if track.submitted_by == "local-model" {
                    track
                        .observations
                        .iter()
                        .rev()
                        .flatten()
                        .next()
                        .map(|o| brief_description(&o.input.explanation))
                        .unwrap_or_else(|| "A provisional visual finding.".into())
                } else {
                    "Visual finding with an unknown effect.".into()
                },
                readable,
                latest
                    .and_then(|o| live.frames.iter().find(|f| f.id == o.input.frame_id))
                    .map(|f| observation_age(f.started_unix_ms, now)),
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
                    "A change over time with an unknown effect."
                } else {
                    "An earlier change. Waiting for a fresh observation."
                }
                .into(),
                readable,
                latest
                    .evidence
                    .frames
                    .last()
                    .map(|f| observation_age(f.started_unix_ms, now)),
            ));
        }
        self.card_order
            .retain(|key| cards.iter().any(|c| (c.0, c.1) == *key));
        for card in &cards {
            let key = (card.0, card.1);
            if !self.card_order.contains(&key) {
                self.card_order.push(key);
            }
        }
        cards.sort_by_key(|c| self.card_order.iter().position(|key| *key == (c.0, c.1)));
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
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
        let mut move_card = None;
        let mut drop_targets = Vec::new();
        for row in cards.chunks(columns) {
            ui.columns(columns, |cols| {
                for (column, (external, id, title, value, description, readable, age)) in
                    cols.iter_mut().zip(row)
                {
                    column.push_id((external, id), |ui| {
                        let key = (*external, *id);
                        let position = self.card_order.iter().position(|k| *k == key).unwrap_or(0);
                        let card = egui::Frame::new()
                            .fill(SURFACE)
                            .inner_margin(18)
                            .corner_radius(8)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_min_height(148.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(title).size(19.0).strong());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let grip = ui.add(egui::Button::new("↕").sense(egui::Sense::click_and_drag()))
                                            .on_hover_text(format!("Arrange {title}: drag onto another card, or click for options."));
                                        grip.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Arrange {title}")));
                                        grip.dnd_set_drag_payload(CardDrag { scope: self.edit_scope.clone(), key });
                                        egui::Popup::menu(&grip).show(|ui| {
                                            if ui.add_enabled(position > 0, egui::Button::new("Move earlier")).clicked() {
                                                move_card = Some((key, position - 1));
                                                ui.close();
                                            }
                                            if ui.add_enabled(position + 1 < self.card_order.len(), egui::Button::new("Move later")).clicked() {
                                                move_card = Some((key, position + 1));
                                                ui.close();
                                            }
                                        });
                                    });
                                });
                                let pattern = action_evidence(live,*external,*id).and_then(|e| {
                                    if live.rule_checks.iter().any(|c|c.source==e.source && c.track==e.track && c.invalidated.is_none()) {
                                        Some("Checking why the outcome varies.".into())
                                    } else {e.patterns.iter().find_map(|p|p.brief())}
                                });
                                ui.label(RichText::new(brief_description(pattern.as_deref().unwrap_or(description))).size(13.0).color(MUTED));
                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(value).size(32.0).color(ACCENT))
                                        .on_hover_text(age.as_deref().unwrap_or("Observation time unavailable"));
                                    ui.label(
                                        RichText::new(
                                            if *external == CardSource::Image
                                                && live
                                                    .visual_tracks
                                                    .iter()
                                                    .find(|t| t.id == *id)
                                                    .is_some_and(|t| {
                                                        t.observations
                                                            .last()
                                                            .is_some_and(Option::is_none)
                                                            && t.observations
                                                                .iter()
                                                                .any(Option::is_some)
                                                    })
                                            {
                                                "Earlier"
                                            } else if *external == CardSource::Ocr && live.prediction_checks.iter().any(|c|c.source==reversebolt_core::prediction::Source::Ocr && c.track==*id && c.invalidated.is_none() && c.automatic_selection.is_some() && c.candidates.iter().any(|m|m.samples.len()<reversebolt_core::prediction::MIN_FRESH)) {
                                                "Checking"
                                            } else if *readable {
                                                "Learning"
                                            } else {
                                                "Uncertain"
                                            },
                                        )
                                        .size(12.0)
                                        .color(MUTED),
                                    );
                                });
                                // Keep this line present as readings age so the actions never move beneath the pointer.
                                if let Some(age) = age {
                                    ui.label(RichText::new(age).size(12.).color(MUTED));
                                }
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
                                if *external==CardSource::Ocr {
                                    let change=state.changes.iter().rev().find(|c|c.scope.investigation==live.investigation_id && c.scope.batch==live.coverage.batch && c.scope.track==*id);
                                    let observed=value.parse::<f64>().ok().filter(|v|v.is_finite());
                                    let eligible=live.prediction_checks.iter().any(|check|check.source==reversebolt_core::prediction::Source::Ocr && check.track==*id && reversebolt_core::probes::choices(check).is_ok_and(|groups|groups.iter().any(|g|g.eligible)));
                                    if let Some(observed)=observed.filter(|_|eligible) {
                                        let requested=self.requested.entry(*id).or_insert_with(|| {
                                            change.filter(|c| c.phase==reversebolt_core::changes::Phase::Prepared).map_or(observed, |c| c.desired)
                                        });
                                        ui.horizontal(|ui| {
                                            let label=ui.label("Try value");
                                            ui.add_enabled(!busy && live.connected,egui::DragValue::new(requested).speed(1.).range((observed-20.)..=(observed+20.))).labelled_by(label.id).on_hover_text("Drag to adjust, or click to type. Trial changes are limited to 20 from the observed value.");
                                            if ui.add_enabled(!busy && live.connected && *requested != observed,egui::Button::new("Review edit")).clicked() {
                                                handle.submit("prepare_change",json!({"investigation_id":live.investigation_id,"batch":live.coverage.batch,"track":id,"value":*requested}));
                                            }
                                        });

                                    }
                                    if let Some(change)=change {
                                        if let Some(comparison)=change.observations.last().and_then(|o|o.comparison.as_ref()) {
                                            let count=comparison.other_values.iter().filter(|v|v.status==reversebolt_core::effects::Status::Changed).count();
                                            if count>0 {ui.label(RichText::new(format!("{count} other value{} changed · Inspect",if count==1 {""}else{"s"})).size(12.).color(WARNING));}
                                        }
                                        use reversebolt_core::changes::Phase;
                                        match change.phase {
                                            Phase::Prepared => {
                                                ui.label(RichText::new(format!("Try {} → {}",change.observed,change.desired)).color(ACCENT));
                                                ui.label(RichText::new(if change.probe.is_some() {"Tests one possibility; other explanations remain."} else {"Experimental edit. Gameplay effects are unverified."}).size(12.).color(MUTED));
                                                if let Some(probe)=&change.probe {self.probe_choice.entry(*id).or_insert(probe.selected);}
                                                let still_requested=self.requested.get(id).is_some_and(|v|*v==change.desired)
                                                    && change.probe.as_ref().is_none_or(|p|self.probe_choice.get(id)==Some(&p.selected));
                                                if ui.add_enabled(!busy && still_requested && live.connected,egui::Button::new(if change.probe.is_some() {"Try test"} else {"Try edit"})).clicked() {handle.submit("apply_change",json!({"change_id":change.id}));}
                                            }
                                            Phase::Written => {
                                                let restore_refused=change.restoration.as_ref().is_some_and(|r|!r.attempted && r.error.is_some());
                                                ui.label(RichText::new(if restore_refused {"Restore wasn't applied"} else {"Trial applied"}).color(if restore_refused {WARNING} else {ACCENT}));
                                                if let Some(observation)=change.observations.last() {
                                                    ui.label(RichText::new(observation.intervention_outcome.map(|o|o.label()).unwrap_or(if observation.agrees {"Requested number seen"} else {"Effect not confirmed"})).size(12.).color(MUTED));
                                                }
                                                ui.horizontal(|ui| {
                                                    if ui.add_enabled(!busy && live.connected,egui::Button::new("Restore bytes")).on_hover_text("Checks the original value's bytes before restoring. Downstream gameplay effects may remain.").clicked() {handle.submit("restore_change",json!({"change_id":change.id}));}
                                                    if ui.add_enabled(!busy && live.connected && change.observations.len()<8,egui::Button::new("Check effect")).clicked() {handle.submit("observe_change",json!({"change_id":change.id}));}
                                                });
                                            }
                                            Phase::Restored => {ui.label(RichText::new("Original bytes restored").size(12.).color(MUTED));}
                                            Phase::Unavailable => {ui.label(RichText::new("Still narrowing down this edit").size(12.).color(MUTED));}
                                            Phase::Uncertain|Phase::Writing|Phase::Restoring => {ui.colored_label(WARNING,"Write outcome uncertain · edits held");}
                                            Phase::Refused|Phase::Expired => {ui.label(RichText::new("Trial not applied · prepare again").size(12.).color(MUTED));}
                                        }
                                    }
                                }
                            });
                        drop_targets.push((key, position, card.response));
                    });
                }
            });
            ui.add_space(12.0);
        }
        // Resolve drops after every source has rendered, including a quick drag
        // from a later card to an earlier card within the same input frame.
        for (key, position, response) in drop_targets {
            if let Some(drag) = response.dnd_hover_payload::<CardDrag>()
                && drag.scope == self.edit_scope
                && drag.key != key
            {
                ui.painter().rect_stroke(
                    response.rect,
                    8.,
                    egui::Stroke::new(1., ACCENT),
                    egui::StrokeKind::Inside,
                );
            }
            if let Some(drag) = response.dnd_release_payload::<CardDrag>()
                && drag.scope == self.edit_scope
                && drag.key != key
            {
                move_card = Some((drag.key, position));
            }
        }
        if let Some((key, destination)) = move_card
            && let Some(source) = self.card_order.iter().position(|k| *k == key)
        {
            self.card_order.remove(source);
            self.card_order
                .insert(destination.min(self.card_order.len()), key);
        }
        if !live.prediction_checks.is_empty() {
            let unsettled = state.changes.iter().any(|c| {
                c.scope.investigation == live.investigation_id
                    && (c.phase.unresolved()
                        || c.phase == reversebolt_core::changes::Phase::Written)
            });
            if ui.add_enabled(!busy && live.connected && live.sample_count > 0 && live.coverage.batch < 24 && !unsettled,
                egui::Button::new("Recheck findings"))
                .on_hover_text(if unsettled { "Restore the applied trial before starting fresh checks." }
                    else { "Check known locations again with four fresh moments. Let values stay still and change." }).clicked() {
                handle.submit("recheck_findings",json!({"investigation_id":live.investigation_id,"batch":live.coverage.batch}));
            }
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
        if ui.button("Inspect application").clicked() {
            self.show_inspector = true;
        }
        if !live.local_observation_warnings.is_empty() {
            ui.label(
                RichText::new(
                    "Some visual items could not be read. Details in Inspect application.",
                )
                .color(WARNING),
            );
        }
        if self.show_inspector {
            let mut open = true;
            egui::Window::new("Application details")
                .open(&mut open)
                .default_width(600.)
                .vscroll(true)
                .show(ui.ctx(), |ui| {
                    self.investigation(ui, handle, state, busy);
                });
            self.show_inspector = open;
        }
        if let Some((_, external, id)) = self.detail.clone() {
            let card = cards
                .iter()
                .find(|(e, i, _, _, _, _, _)| *e == external && *i == id);
            if let Some((_, _, title, value, _, _, _)) = card {
                let response=egui::Modal::new(egui::Id::new("control_details")).show(ui.ctx(),|ui| {
                    ui.set_width(440.0);
                    ui.heading(title);
                    ui.label(RichText::new(value).size(32.0).color(ACCENT));
                    ui.label("This finding is still being investigated. A trial edit, when available, tests one requested value; it does not verify a gameplay control.");
                    if external==CardSource::Ocr && let Some(check)=live.prediction_checks.iter().find(|c|c.source==reversebolt_core::prediction::Source::Ocr && c.track==id)
                        && let Some(selection)=&check.automatic_selection {
                        egui::CollapsingHeader::new("Why this value was checked").show(ui,|ui| {
                            ui.label(&selection.rationale);
                            ui.label(format!("Chosen after moment {} with {} fresh moments scheduled; {} findings considered.",check.after_frame,selection.scheduled_fresh_captures,selection.ranked.len()));
                            ui.label(check.summary());
                        });
                    }
                    if external==CardSource::Ocr
                        && let Some(observed)=value.parse::<f64>().ok().filter(|v|v.is_finite())
                        && let Some(check)=live.prediction_checks.iter().find(|c|c.source==reversebolt_core::prediction::Source::Ocr && c.track==id && c.invalidated.is_none())
                        && let Ok(choices)=reversebolt_core::probes::choices(check)
                        && choices.len()>1 && choices.iter().any(|c|c.eligible) {
                        let change=state.changes.iter().rev().find(|c|c.scope.investigation==live.investigation_id && c.scope.batch==live.coverage.batch && c.scope.track==id);
                        let requested=self.requested.entry(id).or_insert(observed);
                        let selected=self.probe_choice.entry(id).or_insert_with(||change.and_then(|c|c.probe.as_ref()).map_or_else(||choices.iter().find(|c|c.eligible).unwrap().indices[0],|p|p.selected));
                        if !choices.iter().any(|c|c.eligible && c.indices.contains(selected)) {
                            *selected=choices.iter().find(|c|c.eligible).unwrap().indices[0];
                        }
                        egui::CollapsingHeader::new("Test a possibility").id_salt(("probe",id)).show(ui,|ui| {
                            ui.label("Several locations match this value. Test one to see what changes.");
                            let label=ui.label("Try value");
                            ui.add_enabled(!busy && live.connected,egui::DragValue::new(requested).speed(1.).range((observed-20.)..=(observed+20.))).labelled_by(label.id);
                            let ordinal=choices.iter().position(|c|c.indices.contains(selected)).unwrap()+1;
                            egui::ComboBox::from_id_salt(("probe-choice",id)).selected_text(format!("Possibility {ordinal}")).show_ui(ui,|ui| {
                                for (n,c) in choices.iter().enumerate() {
                                    ui.add_enabled_ui(c.eligible,|ui| {ui.selectable_value(selected,c.indices[0],format!("Possibility {}",n+1));});
                                }
                            });
                            if ui.add_enabled(!busy && live.connected && (*requested-observed).abs()>1.1,egui::Button::new("Review test")).clicked() {
                                handle.submit("prepare_probe",json!({"investigation_id":live.investigation_id,"batch":live.coverage.batch,"prediction_id":check.id,"track":id,"candidate":*selected,"value":*requested}));
                                self.detail=None;
                            }
                        });
                    }
                    if let Some(error)=&live.rule_error {ui.colored_label(WARNING,error);}
                    if let Some(e)=action_evidence(live,external,id) {
                        for check in live.rule_checks.iter().filter(|c|c.source==e.source && c.track==e.track) {
                            egui::CollapsingHeader::new("Predictions made before the action").id_salt(&check.id).show(ui,|ui| {
                                egui::ScrollArea::vertical().id_salt((&check.id,"rules")).max_height(210.).show(ui,|ui| {
                                    ui.label(check.summary());
                                    if let Some(reason)=&check.invalidated {ui.colored_label(WARNING,reason);}
                                    for (i,candidate) in check.candidates.iter().enumerate() {
                                        ui.label(format!("{}{}", if check.rejected(i) {"Ruled out: "} else {"Possible: "},candidate.description()));
                                        let agrees=check.challenges.iter().filter(|c|c.comparisons[i].result==reversebolt_core::rule_checks::Result::Agreed).count();
                                        let unknown=check.challenges.iter().filter(|c|c.comparisons[i].result==reversebolt_core::rule_checks::Result::Unknown).count();
                                        ui.label(format!("{agrees} later agreements · {unknown} unknown"));
                                    }
                                    ui.label("Compare a different starting value or later action history, if reachable. Unknown boundaries still need evidence; surviving predictions do not prove a gameplay rule.");
                                });
                            });
                        }
                    }
                    if let Some(evidence)=action_evidence(live,external,id) && !evidence.patterns.is_empty() {
                        egui::CollapsingHeader::new("Observed control patterns").show(ui,|ui| {
                            egui::ScrollArea::vertical().id_salt("control_pattern_details").max_height(190.).show(ui,|ui| {
                            for pattern in &evidence.patterns {
                                ui.strong(reversebolt_core::inputs::key_name(pattern.key));
                                ui.label(format!("{} initial episodes · {} later agreed · {} differed",pattern.first_episodes.len(),pattern.later_agree.len(),pattern.later_disagree.len()));
                                ui.label(format!("Without detected keys: {} unchanged · {} changed",pattern.no_keys_unchanged.len(),pattern.no_keys_changed.len()));
                                ui.label(&pattern.next_observation);
                            }
                            let excluded=evidence.intervals.iter().filter(|i|matches!(i.exposure,reversebolt_core::action_patterns::Exposure::Unknown(_))).count();
                            ui.label(format!("{excluded} intervals could not be compared. These patterns do not prove what a control does."));
                            });
                        });
                    }
                    let input_frame=match external {
                        CardSource::Ocr=>live.frames.last(),
                        CardSource::Image=>live.visual_tracks.iter().find(|t|t.id==id).and_then(|t|t.observations.iter().rev().flatten().next()).and_then(|o|live.frames.iter().find(|f|f.id==o.input.frame_id)),
                        CardSource::Temporal=>live.temporal_tracks.iter().find(|t|t.id==id).and_then(|t|t.observations.last()).and_then(|o|o.evidence.frames.last()),
                    };
                    if let Some(frame)=input_frame && let Some(context)=&frame.input_context {
                        egui::CollapsingHeader::new("Capture input context").show(ui,|ui| {
                            ui.label(context.brief());
                            ui.label(format!("Capture {} · {}",frame.number,observation_age(frame.started_unix_ms,now)));
                            ui.label("Sampled key states around this saved capture. This does not show which inputs the game accepted.");
                            ui.label(format!("{} samples · longest unsampled interval {:.0} ms",context.polls.len(),context.largest_unsampled_us as f64/1000.));
                        });
                    }
                    if external==CardSource::Ocr && let Some(change)=state.changes.iter().rev().find(|c|c.scope.investigation==live.investigation_id && c.scope.batch==live.coverage.batch && c.scope.track==id) {
                        ui.label(&change.reason);
                        if let Some(probe)=&change.probe {
                            ui.label(&probe.limitation);
                            egui::CollapsingHeader::new("What the possibilities predict").show(ui,|ui| {
                                for a in &probe.alternatives {
                                    ui.label(format!("{} {}: {}",if a.in_test_group {"Tested interpretation"} else {"Rival interpretation"},a.index+1,
                                        a.predicted_if_written.map_or_else(||"unavailable".into(),|v|format!("{v:.2}"))));
                                    if let Some(error)=&a.error {ui.label(error);}
                                }
                            });
                        }
                        if let Some(receipt)=change.restoration.as_ref().or(change.write.as_ref()) && let Some(error)=&receipt.error {ui.colored_label(ERROR,error);}
                        if let Some(observation)=change.observations.last() {
                            if let Some(outcome)=observation.intervention_outcome {
                                ui.label(outcome.explanation());
                                ui.label("These checks sample memory at capture endpoints. Changes between reads, display lag and hidden effects remain possible.");
                            }
                            ui.label(format!("Trial reading: {} · expected {}",observation.value.map_or_else(||"unavailable".into(),|v|v.to_string()),observation.expected));
                            if let Some(comparison)=&observation.comparison {
                                for value in &comparison.other_values {
                                    let shown=|n:Option<f64>|n.map_or_else(||"unreadable".into(),|n|n.to_string());
                                    ui.label(format!("{}: {} → {}",value.label,shown(value.before),shown(value.after)));
                                }
                                ui.label(&comparison.limitation);
                                ui.label(format!("{} tracks omitted; {} other numeric regions in this image were not tracked.",comparison.omitted_tracks,observation.untracked_numbers));
                            } else {ui.label("Only this screen number was checked. Hidden and downstream effects remain unknown.");}
                            if let Some(error)=&observation.error {ui.colored_label(WARNING,error);}
                        }
                    }
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
                        if external == CardSource::Image
                            && let Some(track) = live.visual_tracks.iter().find(|t|t.id==id)
                            && let Some((index,observation)) = track.observations.iter().enumerate().rev().find_map(|(i,o)|o.as_ref().map(|o|(i,o))) {
                            ui.label(&observation.input.explanation);
                            ui.label(format!("Observed in moment {} · {}",index+1,track.observer));
                        }
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
                egui::CollapsingHeader::new("Memory source").show(ui, |ui| {
                    let current = live.memory_process.as_ref().unwrap_or(&live.process);
                    ui.label(format!("Reading {} (PID {})", current.name, current.pid));
                    ui.label("Some applications keep state in a child process. Choosing one starts a fresh investigation while watching the same window.");
                    if ui.add_enabled(!busy && live.connected, egui::Button::new("Refresh memory sources")).clicked() {
                        handle.submit("list_memory_processes", json!({}));
                    }
                    if let Some(error) = &live.memory_sources_error {
                        ui.colored_label(WARNING, error);
                    }
                    egui::ScrollArea::vertical().max_height(160.).id_salt("memory_sources").show(ui, |ui| {
                        for source in &live.memory_sources {
                            let selected = source.pid == current.pid && source.created == current.created;
                            if ui.add_enabled(!busy && live.connected && !selected,
                                egui::Button::new(format!("Observe {} (PID {})", source.name, source.pid))).clicked() {
                                handle.submit("attach_process", json!({"pid":live.process.pid,"created":live.process.created,
                                    "window":live.process.window,"memory_pid":source.pid,"memory_created":source.created,
                                    "budget_mib":16,"auto_observe":true}));
                                self.image = None;
                            }
                        }
                    });
                    ui.label(RichText::new("Only accessible descendants with a live parent chain are listed. Their role in gameplay is unknown.").size(12.).color(MUTED));
                });
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
                        theme::primary("Continue discovery")).on_hover_text("Start another automatic pass over different memory. Current findings are saved.").clicked() {
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
                            !busy && live.connected && live.sample_count >= 12 && live.prediction_checks.is_empty(),
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
        if !live.local_observation_warnings.is_empty() {
            egui::CollapsingHeader::new("Visual review warnings").show(ui, |ui| {
                for warning in &live.local_observation_warnings {
                    ui.colored_label(WARNING, &warning.message);
                    ui.label(format!(
                        "Evidence: {} · observation job {}",
                        warning.evidence_id, warning.job
                    ));
                }
                ui.label("These reviews contributed no findings. Other observation continued.");
            });
        }
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
        for check in &live.prediction_checks {
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
            egui::CollapsingHeader::new("Prediction evidence").id_salt(&check.id).show(ui, |ui| {
                ui.label("Agreement is limited to these provisional observations. It cannot distinguish a controlling value from a display copy.");
                for candidate in &check.candidates {
                    egui::CollapsingHeader::new(format!("{} · {:?} · {}", candidate.address,
                        candidate.mapping.kind, candidate.assessment(check).label()))
                        .id_salt((&check.id, &candidate.address, format!("{:?}", candidate.mapping.kind)))
                        .show(ui, |ui| {
                            if let Some(reason) = &candidate.invalidated { ui.colored_label(WARNING, reason); }
                            for sample in &candidate.samples {
                                let value = |v:Option<f64>| v.map_or_else(|| "missing".into(), |n| format!("{n:.2}"));
                                ui.label(format!("Capture {}: predicted {} · observed {} · {}",
                                    sample.frame, value(sample.predicted), value(sample.observed), sample.outcome.label()));
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
                ui.label(format!("Image {} ms · memory {} ms", latest.screen_capture_ms, latest.memory_read_ms));
                if latest.mode.is_full() {
                    ui.label(format!("Text recognition {} ms", latest.recognition_ms));
                } else {
                    ui.label("Text recognition skipped for this moment.");
                }
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
    if !has_matches
        || live.prediction_checks.len() >= reversebolt_core::prediction::MAX_CHECKS
        || live.prediction_checks.iter().any(|c| {
            c.invalidated.is_some()
                || c.after_frame != live.sample_count
                || (c.track == track
                    && match c.source {
                        reversebolt_core::prediction::Source::Ocr => source == "ocr",
                        reversebolt_core::prediction::Source::External => source == "external",
                    })
        })
    {
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
