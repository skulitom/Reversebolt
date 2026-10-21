use crate::service::Handle;
use crate::theme::{
    self, ACCENT, BACKGROUND, BORDER, CHROME, ERROR, GREEN, MUTED, SELECTED, SURFACE, TEXT, WARNING,
};
use eframe::egui::{self, Color32, RichText, Vec2};
use serde_json::json;
use std::sync::atomic::Ordering;

pub struct Workbench {
    handle: Handle,
    edits: [String; 2],
    endpoint: String,
    show_connection: bool,
    show_live: bool,
    show_observer: bool,
    show_activity: bool,
    observer_model: String,
    live: crate::ui_live::LivePane,
}

impl Workbench {
    pub fn new(cc: &eframe::CreationContext<'_>, handle: Handle, endpoint: String) -> Self {
        theme::apply(&cc.egui_ctx);
        let observer_model = handle
            .view
            .read()
            .unwrap()
            .observer_model
            .clone()
            .unwrap_or_default();
        Self {
            handle,
            edits: ["100".into(), "30".into()],
            endpoint,
            show_connection: false,
            show_live: true,
            show_observer: false,
            show_activity: false,
            observer_model,
            live: crate::ui_live::LivePane::default(),
        }
    }
}

impl eframe::App for Workbench {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(150));
        let state = self.handle.view.read().unwrap().clone();
        let job_state = self.handle.jobs.read().unwrap().clone();
        let busy =
            self.handle.busy.load(Ordering::SeqCst)
                || job_state.history.jobs.values().any(|j| {
                    !j.phase.terminal() && j.phase != reversebolt_session_jobs::Phase::Paused
                });
        egui::Panel::left("navigation")
            .exact_size(236.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(CHROME).inner_margin(18))
            .show(ui, |ui| {
                ui.visuals_mut().override_text_color = Some(TEXT);
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Reversebolt")
                        .size(25.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.label(RichText::new("Make it your game").size(13.0).color(MUTED));
                ui.add_space(24.0);
                if ui
                    .add_enabled(
                        !busy,
                        theme::primary("Connect to application")
                            .wrap_mode(egui::TextWrapMode::Extend)
                            .min_size(Vec2::new(200.0, 36.0)),
                    )
                    .clicked()
                {
                    self.show_live = true;
                    self.live.open_picker(&self.handle);
                }
                ui.add_space(12.0);
                if ui.selectable_label(self.show_live, "Application").clicked() {
                    self.show_live = true;
                }
                if ui.selectable_label(!self.show_live, "Practice").clicked() {
                    self.show_live = false;
                }
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(14.0);
                if ui.button("Activity").clicked() {
                    self.show_activity = true;
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new("Agent connection").color(ACCENT))
                            .fill(SELECTED),
                    )
                    .clicked()
                {
                    self.show_connection = !self.show_connection;
                }
                if self.show_connection {
                    ui.add_space(8.0);
                    ui.label(RichText::new(&self.endpoint).size(12.0));
                    ui.label("Run reversebolt-mcp.exe as an MCP stdio server. See docs/MCP.md.");
                }
                if ui.button("Local vision").clicked() {
                    self.show_observer = true;
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(
                        RichText::new("Research preview · v0.1.0")
                            .size(12.0)
                            .color(MUTED),
                    );
                });
            });
        if self.show_observer {
            egui::Window::new("Local vision").open(&mut self.show_observer).resizable(false).show(ui.ctx(), |ui| {
                ui.set_width(380.0);
                ui.label("Use an installed local vision model to find characters and held objects automatically.");
                ui.label("Its findings are provisional. Images stay on this computer.");
                ui.label(format!("Selected: {}", state.observer_model.as_deref().unwrap_or("None")));
                let label = ui.label("Installed model name");
                let id = ui.make_persistent_id("local_model_name");
                ui.input_mut(|input| input.consume_accesskit_action_requests(id, |request| {
                    if request.action == egui::accesskit::Action::SetValue
                        && let Some(egui::accesskit::ActionData::Value(value)) = &request.data {
                        self.observer_model = value.chars().take(128).collect(); return true;
                    }
                    false
                }));
                ui.add(egui::TextEdit::singleline(&mut self.observer_model).id(id).char_limit(128)).labelled_by(label.id);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!busy && !self.observer_model.is_empty(), egui::Button::new("Use model")).clicked() {
                        self.handle.submit("set_observer_model", json!({"model":self.observer_model}));
                    }
                    if ui.add_enabled(!busy && state.observer_model.is_some(), egui::Button::new("Disable")).clicked() {
                        self.handle.submit("set_observer_model", json!({"model":null}));
                    }
                });
                ui.label("Applies when you connect or choose Keep looking. No models are downloaded.");
                if let Some(error) = &state.error { ui.colored_label(ERROR,error); }
            });
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BACKGROUND).inner_margin(22))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    crate::jobs::panel(ui,&self.handle,&job_state);
                    for test in state.runtime_tests.iter().filter(|t|t["active"]==true) {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Checking an effect").color(ACCENT));
                            ui.label(test["hypothesis"].as_str().unwrap_or("Comparing responses"));
                            if ui.button("Stop test").clicked() {
                                self.handle.submit("stop_runtime_test",json!({"test_id":test["id"]}));
                            }
                            if job_state.history.human_hold.is_none() && test["actor"]!="desktop"
                                && ui.button("Take control").clicked() {
                                self.handle.submit("job_control",json!({"operation":"take_control"}));
                            }
                        });
                    }
                    if self.show_live {
                        self.live.show(ui, &self.handle, &state, busy);
                    } else {
                    ui.horizontal(|ui| {
                        ui.heading("Discovery workbench");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(if state.connected { "Practice connected" } else { "No active target" })
                                .color(if state.connected { GREEN } else { MUTED }));
                        });
                    });
                    ui.label(RichText::new("From an observation to a control you can test.").color(MUTED));
                    ui.add_space(6.0);
                    egui::Frame::new().fill(SURFACE).inner_margin(10).corner_radius(6).show(ui,|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("Practice environment").color(ACCENT).strong());
                        ui.label("A separate process with randomized memory, changing values, and misleading copies.");
                        ui.label(RichText::new("Labels and numeric observations come from the practice adapter. This is not screen recognition.").size(13.0).color(MUTED));
                    });
                    ui.add_space(8.0);
                    ui.add_enabled_ui(!busy,|ui| {
                        ui.horizontal_wrapped(|ui| {
                            if ui.add(if state.connected { egui::Button::new("New practice") } else { theme::primary("Open practice") }).clicked() {
                                self.handle.submit("open_practice",json!({}));
                            }
                            let observed = state.tracks.first().is_some_and(|t| !t.samples.is_empty());
                            let found = state.tracks.iter().any(|t| !t.matches.is_empty());
                            if ui.add_enabled(state.connected,egui::Button::new("1  Observe changes")).clicked() { self.handle.submit("observe",json!({})); }
                            if ui.add_enabled(observed,egui::Button::new("2  Find matches")).clicked() { self.handle.submit("discover",json!({})); }
                            if ui.add_enabled(found,egui::Button::new("3  Test effects")).clicked() { self.handle.submit("test_candidates",json!({})); }
                            let can_undo = state.connected && state.patches.iter().any(|p| !p.undone && p.session==state.session);
                            if ui.add_enabled(can_undo,egui::Button::new("Restore last edit")).clicked() { self.handle.submit("undo",json!({})); }
                        });
                    });
                    ui.horizontal(|ui| {
                        if busy { ui.spinner(); ui.label("Investigating…"); }
                        else { ui.label(RichText::new(&state.phase).size(13.0).color(MUTED)); }
                    });
                    if let Some(error) = &state.error { ui.colored_label(ERROR,error); }
                    ui.add_space(8.0);
                    ui.separator();
                    if state.tracks.is_empty() {
                        ui.add_space(30.0);
                        ui.label(RichText::new("What does this value actually do?").size(24.0));
                        ui.label("Start a practice session. Reversebolt will compare observations with memory,");
                        ui.label("then change each match briefly to see which one controls the result.");
                        ui.add_space(24.0);
                        ui.label(RichText::new("No fixed addresses. No trainer table. Evidence before an edit.").color(ACCENT));
                        ui.add_space(35.0);
                    }
                    for (index,track) in state.tracks.iter().enumerate() {
                        ui.push_id(index,|ui| {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&track.label).size(20.0).strong());
                                ui.label(RichText::new(format!("{:.0}",track.value)).size(30.0).monospace().color(ACCENT));
                                let count = track.matches.iter().filter(|m| m.passed).count();
                                let label = if track.verified().is_some() { "One tested control".into() }
                                    else if count>1 { format!("{count} plausible controls · ambiguous") }
                                    else if track.matches.is_empty() { "Waiting for evidence".into() }
                                    else { format!("{} matches · investigate",track.matches.len()) };
                                ui.label(RichText::new(label).size(13.0).color(if track.verified().is_some() { GREEN } else { WARNING }));
                            });
                            ui.horizontal(|ui| {
                                trace(ui,&track.samples);
                                ui.add_enabled_ui(track.verified().is_some() && !busy,|ui| {
                                    let label = ui.label("Set to");
                                    let mut value=self.edits[index].parse::<f64>().unwrap_or(track.value);
                                    if ui.add(egui::DragValue::new(&mut value).speed(1.0).range(-1_000_000.0..=1_000_000.0))
                                        .labelled_by(label.id)
                                        .on_hover_text("Drag to adjust, or click to type a value.").changed() {
                                        self.edits[index]=value.to_string();
                                    }
                                    if ui.button("Apply").clicked() {
                                        if let Ok(value) = self.edits[index].parse::<f64>() {
                                            self.handle.submit("set_value",json!({"track":index,"value":value}));
                                        } else {
                                            self.handle.view.write().unwrap().error = Some("Enter a numeric value before applying.".into());
                                        }
                                    }
                                });
                            });
                            egui::CollapsingHeader::new(format!("Evidence · {} memory matches",track.matches.len())).show(ui,|ui| {
                                for item in &track.matches {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(RichText::new(format!("+0x{:X}  {:?}",item.candidate.offset,item.candidate.kind)).monospace().size(12.0));
                                        ui.label(RichText::new(&item.evidence).size(12.0).color(if item.passed { GREEN } else { MUTED }));
                                    });
                                }
                                ui.label(RichText::new("Practice evidence only. Overlapping representations and hidden side effects require broader validation.").size(12.0).color(WARNING));
                            });
                            ui.separator();
                        });
                    }
                    }
                });
            });
        if self.show_activity {
            egui::Window::new("Activity")
                .open(&mut self.show_activity)
                .default_width(480.)
                .vscroll(true)
                .show(ui.ctx(), |ui| {
                    if state.history.is_empty() {
                        ui.label("Your actions and agent actions will appear here.");
                    }
                    for event in state.history.iter().rev().take(40) {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(if event.source == "agent" {
                                    "Agent"
                                } else {
                                    "You"
                                })
                                .size(12.)
                                .color(ACCENT),
                            );
                            ui.label(&event.text);
                        });
                    }
                    egui::CollapsingHeader::new("Observation history").show(ui, |ui| {
                        for job in job_state.history.jobs.values().rev().take(6) {
                            ui.label(format!(
                                "{:?} · {} recorded moments",
                                job.phase, job.next_step
                            ));
                        }
                    });
                });
        }
        self.live.picker(ui.ctx(), &self.handle, &state, busy);
    }
}

fn trace(ui: &mut egui::Ui, samples: &[Option<f64>]) {
    let width = (ui.available_width() - 280.0).max(120.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 30.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        egui::Stroke::new(1.0, BORDER),
    );
    if samples.len() < 2 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Observation trace",
            egui::FontId::proportional(12.0),
            MUTED,
        );
        return;
    }
    let lo = samples
        .iter()
        .flatten()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let hi = samples
        .iter()
        .flatten()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let points: Vec<_> = samples
        .iter()
        .enumerate()
        .filter_map(|(i, v)| {
            v.map(|v| {
                egui::pos2(
                    rect.left() + i as f32 / (samples.len() - 1) as f32 * rect.width(),
                    rect.bottom()
                        - 4.0
                        - ((v - lo) / (hi - lo).max(1.0)) as f32 * (rect.height() - 8.0),
                )
            })
        })
        .collect();
    painter.add(egui::Shape::line(points, egui::Stroke::new(1.8, ACCENT)));
}
