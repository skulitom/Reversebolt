//! Direct card controls. Pointer edits commit once at release; text stays exact.
use crate::{
    service::Handle,
    theme::{ACCENT, MUTED, WARNING},
};
use eframe::egui::{self, RichText};
use reversebolt_core::{changes::Phase, engine::View, runtime_patch::encode};
use serde_json::{Value, json};

#[derive(Default)]
pub struct Editor {
    text: String,
    anchor: String,
    offset: f64,
    focused: bool,
    dragging: bool,
    dirty: bool,
    drag_dirty: bool,
    error: Option<String>,
    #[cfg(test)]
    slider_rect: Option<egui::Rect>,
    #[cfg(test)]
    value_id: Option<egui::Id>,
}

pub fn text(value: &Value) -> String {
    value
        .get("boolean")
        .map(Value::to_string)
        .unwrap_or_else(|| {
            value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string())
        })
}

fn offset(kind: i32, anchor: &str, current: &str) -> Option<f64> {
    if (4..=11).contains(&kind) {
        Some((current.parse::<i128>().ok()? - anchor.parse::<i128>().ok()?) as f64)
    } else {
        Some(current.parse::<f64>().ok()? - anchor.parse::<f64>().ok()?)
    }
}

fn shifted(kind: i32, anchor: &str, delta: f64) -> Option<String> {
    if (4..=11).contains(&kind) {
        Some(
            anchor
                .parse::<i128>()
                .ok()?
                .checked_add(delta.round() as i128)?
                .to_string(),
        )
    } else {
        let value = anchor.parse::<f64>().ok()? + delta;
        value.is_finite().then(|| value.to_string())
    }
}

pub fn draw(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    kind: i32,
    value: &Value,
    enabled: bool,
    bounds: Option<std::ops::RangeInclusive<f64>>,
) -> Option<String> {
    if kind == 2 {
        let mut checked = value["boolean"].as_bool().unwrap_or(false);
        let canonical = value["raw"].as_u64().is_some_and(|v| v <= 1);
        let label = if checked { "On" } else { "Off" };
        let changed = ui
            .scope(|ui| {
                ui.spacing_mut().icon_width = 26.;
                ui.spacing_mut().icon_width_inner = 18.;
                ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(61, 66, 75);
                ui.visuals_mut().widgets.inactive.fg_stroke = egui::Stroke::new(2., ACCENT);
                ui.add_enabled(
                    enabled && canonical,
                    egui::Checkbox::new(&mut checked, RichText::new(label).size(28.).color(ACCENT)),
                )
                .changed()
            })
            .inner;
        return changed.then(|| checked.to_string());
    }
    if !(4..=13).contains(&kind) {
        ui.label(RichText::new(text(value)).size(36.).color(ACCENT));
        return None;
    }
    let current = text(value);
    let value_id = ui.id().with("direct-value");
    #[cfg(test)]
    {
        editor.value_id = Some(value_id);
    }
    let focused = ui.memory(|m| m.has_focus(value_id));
    if editor.anchor.is_empty() {
        editor.anchor = current.clone();
    }
    if !focused && !editor.focused && !editor.dragging {
        editor.text = current.clone();
        editor.dirty = false;
        editor.offset = offset(kind, &editor.anchor, &current).unwrap_or(0.);
        if !editor.offset.is_finite() || editor.offset.abs() > 20. {
            editor.anchor = current.clone();
            editor.offset = 0.;
        }
    }
    // TextEdit exposes a UIA value provider, but egui does not consume its
    // string SetValue action. Handle that explicit edit without a float roundtrip.
    let accessible = enabled
        .then(|| {
            ui.input_mut(|input| {
                let mut value = None;
                input.consume_accesskit_action_requests(value_id, |request| {
                    if request.action != egui::accesskit::Action::SetValue {
                        return false;
                    }
                    if let Some(egui::accesskit::ActionData::Value(text)) = &request.data
                        && text.len() <= 64
                    {
                        value = Some(text.to_string());
                    }
                    true
                });
                value
            })
        })
        .flatten();
    if let Some(value) = &accessible {
        editor.text = value.clone();
        editor.dirty = true;
        editor.error = None;
    }
    let size = if editor.text.len() > 14 { 25. } else { 38. };
    let mut step = 0.;
    let response = ui
        .horizontal(|ui| {
            let decrease = ui.add_enabled(
                enabled,
                egui::Button::new(RichText::new("−").size(24.)).min_size(egui::vec2(36., 46.)),
            );
            decrease.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, "Decrease value")
            });
            if decrease.clicked() {
                step = -1.;
            }
            let response = ui.add_enabled(
                enabled,
                egui::TextEdit::singleline(&mut editor.text)
                    .id(value_id)
                    .font(egui::FontId::proportional(size))
                    .text_color(ACCENT)
                    .desired_width((ui.available_width() - 44.).max(70.))
                    .char_limit(64)
                    .hint_text("Value"),
            );
            let increase = ui.add_enabled(
                enabled,
                egui::Button::new(RichText::new("+").size(24.)).min_size(egui::vec2(36., 46.)),
            );
            increase.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, "Increase value")
            });
            if increase.clicked() {
                step = 1.;
            }
            response
        })
        .inner;
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, enabled, "Value"));
    ui.ctx().accesskit_node_builder(value_id, |node| {
        node.add_action(egui::accesskit::Action::SetValue)
    });
    response
        .clone()
        .on_hover_text("Click to type a value, then press Enter or click away to apply.");
    editor.dirty |= response.changed();
    if response.changed() {
        editor.error = None;
    }
    let enter = response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    let mut commit = enabled
        && editor.dirty
        && (accessible.is_some()
            || enter
            || response.lost_focus()
            || (response.changed() && !response.has_focus()));
    editor.focused = response.has_focus();
    if enter {
        response.surrender_focus();
        editor.focused = false;
    }
    if step != 0.
        && let Some(value) = shifted(kind, &current, step)
    {
        editor.text = value;
        commit = true;
    }
    // Only a known domain range can justify a slider. A small observed value
    // or a write-limit is not evidence that the setting itself has that range.
    if let Some(range) = bounds.filter(|r| {
        r.start().is_finite()
            && r.end().is_finite()
            && r.start() < r.end()
            && r.end() - r.start() <= 20.
    }) {
        editor.anchor = range.start().to_string();
        if !editor.dragging {
            editor.offset = offset(kind, &editor.anchor, &current).unwrap_or(0.);
        }
        let range = 0.0..=(range.end() - range.start());
        ui.scope(|ui| {
            ui.spacing_mut().slider_width = ui.available_width();
            ui.spacing_mut().interact_size.y = 34.;
            ui.spacing_mut().slider_rail_height = 6.;
            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(85, 92, 104);
            ui.visuals_mut().widgets.inactive.fg_stroke = egui::Stroke::new(2., ACCENT);
            ui.visuals_mut().widgets.hovered.bg_fill = ACCENT;
            ui.visuals_mut().widgets.active.bg_fill = ACCENT;
            ui.visuals_mut().selection.bg_fill = ACCENT;
            let mut slider = egui::Slider::new(&mut editor.offset, range)
                .show_value(false)
                .trailing_fill(true);
            if (4..=11).contains(&kind) {
                slider = slider.step_by(1.);
            }
            let slider = ui
                .add_enabled(enabled, slider)
                .on_hover_text("Drag to change the value. Release to apply.");
            #[cfg(test)]
            {
                editor.slider_rect = Some(slider.rect);
            }
            slider.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Slider, enabled, "Adjust value")
            });
            if slider.changed() {
                editor.error = None;
                if let Some(value) = shifted(kind, &editor.anchor, editor.offset) {
                    editor.text = value;
                }
                editor.drag_dirty = true;
            }
            let pointer_down = ui.input(|i| i.pointer.any_down());
            editor.dragging = slider.dragged() || slider.is_pointer_button_down_on();
            if enabled && editor.drag_dirty && !pointer_down {
                commit = true;
                editor.drag_dirty = false;
            }
        });
    }
    if commit {
        editor.dirty = false;
        match encode(kind, &editor.text) {
            Ok(desired) if Some(&desired) != encode(kind, &current).ok().as_ref() => {
                return Some(editor.text.clone());
            }
            Err(error) => editor.error = Some(error),
            _ => {}
        }
    }
    if let Some(error) = &editor.error {
        ui.label(RichText::new(error).small().color(WARNING));
    }
    None
}

pub fn ocr(
    ui: &mut egui::Ui,
    handle: &Handle,
    state: &View,
    track: usize,
    displayed: &str,
    editor: &mut Editor,
    enabled: bool,
) {
    let Some(live) = &state.live else {
        return;
    };
    // The card already chooses the newest trial observation over older scan samples.
    let value = displayed.parse::<f64>().ok().filter(|v| v.is_finite());
    let eligible = live.prediction_checks.iter().any(|check| {
        check.source == reversebolt_core::prediction::Source::Ocr
            && check.track == track
            && check.invalidated.is_none()
            && reversebolt_core::probes::choices(check).is_ok_and(|g| g.len() == 1 && g[0].eligible)
    });
    let requested = draw(
        ui,
        editor,
        13,
        &value.map(|v| json!(v.to_string())).unwrap_or(json!("—")),
        enabled && eligible && value.is_some(),
        None,
    );
    let trial = state.changes.iter().rev().find(|c| {
        c.scope.investigation == live.investigation_id
            && c.scope.batch == live.coverage.batch
            && c.scope.track == track
    });
    let mut undo = None;
    ui.horizontal(|ui| {
        ui.add_enabled(false, egui::Checkbox::new(&mut false,"Freeze")).on_hover_text("Freeze needs a supported internal value. This reading is still being connected to its control.");
        let label = trial.map_or("Learning", |t| match t.phase {
            Phase::Written => "Changed",
            Phase::Restored => "Restored",
            Phase::Uncertain => "Outcome unknown",
            Phase::Refused | Phase::Unavailable | Phase::Expired => "Couldn’t apply",
            _ => "Learning",
        });
        ui.label(RichText::new(label).small().color(
            if trial.is_some_and(|t| t.phase.unresolved()) {
                WARNING
            } else {
                MUTED
            },
        ));
        if let Some(t) = trial.filter(|t| t.phase == Phase::Written && t.restoration.is_none())
            && ui
                .add_enabled(enabled, egui::Button::new("Undo").small())
                .clicked()
        {
            undo = Some(t.id.clone());
        }
    });
    if let Some(id) = undo {
        handle.submit("restore_change", json!({"change_id":id}));
    } else if let Some(value) = requested.and_then(|v| v.parse::<f64>().ok()) {
        handle.submit("set_observed_value",json!({"change_id":uuid::Uuid::new_v4().to_string(),"investigation_id":live.investigation_id,
            "batch":live.coverage.batch,"track":track,"value":value}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slider_adjustments_keep_large_integer_precision() {
        assert_eq!(
            shifted(11, "18446744073709551614", 1.).unwrap(),
            "18446744073709551615"
        );
        assert_eq!(
            shifted(10, "-9223372036854775807", -1.).unwrap(),
            "-9223372036854775808"
        );
        assert_eq!(
            offset(11, "18446744073709551614", "18446744073709551615"),
            Some(1.)
        );
    }

    #[test]
    fn dragging_commits_once_on_release_and_never_during_motion() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        let mut frame = |events: Vec<egui::Event>| {
            let mut result = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400., 220.),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    result = draw(ui, &mut editor, 8, &json!("3"), true, Some(1.0..=5.0));
                },
            );
            output.textures_delta.clear();
            (result, editor.slider_rect.unwrap())
        };
        let (_, rect) = frame(vec![]);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let start = rect.center();
        assert!(
            frame(vec![egui::Event::PointerMoved(start), button(start, true)])
                .0
                .is_none()
        );
        let end = egui::pos2(rect.right() - 30., rect.center().y);
        assert!(frame(vec![egui::Event::PointerMoved(end)]).0.is_none());
        assert!(frame(vec![]).0.is_none());
        let value = frame(vec![button(end, false)])
            .0
            .unwrap()
            .parse::<i32>()
            .unwrap();
        assert!((4..=5).contains(&value));
        assert!(frame(vec![]).0.is_none());
        assert!(frame(vec![]).0.is_none());
    }

    #[test]
    fn accessible_number_entry_is_exact_once_and_obeys_disabled_state() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        let mut frame = |events, enabled| {
            let mut result = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    result = draw(
                        ui,
                        &mut editor,
                        11,
                        &json!("18446744073709551614"),
                        enabled,
                        None,
                    );
                },
            );
            output.textures_delta.clear();
            (result, editor.value_id.unwrap())
        };
        let (_, id) = frame(vec![], true);
        let action = || {
            egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::SetValue,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: id.value().into(),
                data: Some(egui::accesskit::ActionData::Value(
                    "18446744073709551615".into(),
                )),
            })
        };
        assert!(frame(vec![action()], false).0.is_none());
        assert_eq!(
            frame(vec![action()], true).0.as_deref(),
            Some("18446744073709551615")
        );
        assert!(frame(vec![], true).0.is_none());
    }
}
