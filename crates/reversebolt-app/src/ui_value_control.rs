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
) -> Option<String> {
    if kind == 2 {
        let mut checked = value["boolean"].as_bool().unwrap_or(false);
        let canonical = value["raw"].as_u64().is_some_and(|v| v <= 1);
        let label = if checked { "On" } else { "Off" };
        let changed = ui
            .add_enabled(
                enabled && canonical,
                egui::Checkbox::new(&mut checked, RichText::new(label).size(28.).color(ACCENT)),
            )
            .changed();
        return changed.then(|| checked.to_string());
    }
    if !(4..=13).contains(&kind) {
        ui.label(RichText::new(text(value)).size(36.).color(ACCENT));
        return None;
    }
    let current = text(value);
    let value_id = ui.id().with("direct-value");
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
    let size = if editor.text.len() > 14 { 25. } else { 38. };
    let response = ui.add_enabled(
        enabled,
        egui::TextEdit::singleline(&mut editor.text)
            .id(value_id)
            .font(egui::FontId::proportional(size))
            .text_color(ACCENT)
            .desired_width(ui.available_width())
            .char_limit(64)
            .hint_text("Value"),
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, enabled, "Value"));
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
        && (enter || response.lost_focus() || (response.changed() && !response.has_focus()));
    editor.focused = response.has_focus();
    if enter {
        response.surrender_focus();
        editor.focused = false;
    }
    let mut range = -20.0..=20.0;
    if let Ok(base) = editor.anchor.parse::<i128>() {
        let limits = match kind {
            4 => Some((i8::MIN as i128, i8::MAX as i128)),
            5 => Some((0, u8::MAX as i128)),
            6 => Some((i16::MIN as i128, i16::MAX as i128)),
            7 => Some((0, u16::MAX as i128)),
            8 => Some((i32::MIN as i128, i32::MAX as i128)),
            9 => Some((0, u32::MAX as i128)),
            10 => Some((i64::MIN as i128, i64::MAX as i128)),
            11 => Some((0, u64::MAX as i128)),
            _ => None,
        };
        if let Some((min, max)) = limits {
            range = (min - base).max(-20) as f64..=(max - base).min(20) as f64;
        }
    }
    ui.scope(|ui| {
        ui.spacing_mut().slider_width = ui.available_width();
        ui.spacing_mut().interact_size.y = 34.;
        let mut slider = egui::Slider::new(&mut editor.offset, range).show_value(false);
        if (4..=11).contains(&kind) { slider=slider.step_by(1.); }
        let slider = ui.add_enabled(enabled, slider).on_hover_text("Drag to change the value. Release to apply. Small adjustments are currently limited to 20.");
        #[cfg(test)] { editor.slider_rect = Some(slider.rect); }
        slider.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Slider, enabled, "Adjust value"));
        if slider.changed() {
            editor.error=None;
            if let Some(value) = shifted(kind, &editor.anchor, editor.offset) { editor.text=value; }
            editor.drag_dirty=true;
        }
        let pointer_down=ui.input(|i|i.pointer.any_down());
        editor.dragging=slider.dragged() || slider.is_pointer_button_down_on();
        if enabled && editor.drag_dirty && !pointer_down {
            commit=true; editor.drag_dirty=false;
        }
    });
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
    );
    let trial = state.changes.iter().rev().find(|c| {
        c.scope.investigation == live.investigation_id
            && c.scope.batch == live.coverage.batch
            && c.scope.track == track
    });
    let mut undo = None;
    ui.horizontal(|ui| {
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
                    result = draw(ui, &mut editor, 8, &json!("50"), true);
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
        assert!((51..=70).contains(&value));
        assert!(frame(vec![]).0.is_none());
        assert!(frame(vec![]).0.is_none());
    }
}
