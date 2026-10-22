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
    #[cfg(test)]
    value_rect: Option<egui::Rect>,
    #[cfg(test)]
    increase_rect: Option<egui::Rect>,
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

pub fn supported_value(kind: i32, value: &Value) -> bool {
    if kind == 2 {
        return value["raw"]
            .as_u64()
            .is_some_and(|raw| raw <= 1 && value["boolean"].as_bool() == Some(raw == 1));
    }
    (4..=13).contains(&kind) && encode(kind, &text(value)).is_ok()
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

fn encode_bounded(
    kind: i32,
    text: &str,
    bounds: Option<&std::ops::RangeInclusive<f64>>,
) -> Result<Vec<u8>, String> {
    let encoded = encode(kind, text)?;
    if let Some(range) = bounds {
        // Do not round large integers through f64 just to check a UI bound.
        let inside = if (4..=11).contains(&kind) {
            text.parse::<i128>().is_ok_and(|value| {
                value >= range.start().ceil() as i128 && value <= range.end().floor() as i128
            })
        } else if kind == 12 {
            text.parse::<f32>()
                .is_ok_and(|value| range.contains(&f64::from(value)))
        } else {
            text.parse::<f64>()
                .is_ok_and(|value| range.contains(&value))
        };
        if !inside {
            return Err(format!(
                "Choose a value from {} to {}.",
                range.start(),
                range.end()
            ));
        }
    }
    Ok(encoded)
}

pub fn draw(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    kind: i32,
    value: &Value,
    enabled: bool,
    bounds: Option<std::ops::RangeInclusive<f64>>,
) -> Option<String> {
    let enabled = enabled && supported_value(kind, value);
    #[cfg(test)]
    {
        editor.slider_rect = None;
    }
    if kind == 2 {
        let mut checked = value["boolean"].as_bool().unwrap_or(false);
        let label = if checked { "On" } else { "Off" };
        let changed = ui
            .scope(|ui| {
                ui.spacing_mut().icon_width = 26.;
                ui.spacing_mut().icon_width_inner = 18.;
                ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(61, 66, 75);
                ui.visuals_mut().widgets.inactive.fg_stroke = egui::Stroke::new(2., ACCENT);
                ui.add_enabled(
                    enabled,
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
    let bounds =
        bounds.filter(|r| r.start().is_finite() && r.end().is_finite() && r.start() <= r.end());
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
    let mut step = if enabled && focused {
        ui.input_mut(|input| {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                1.
            } else if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                -1.
            } else {
                0.
            }
        })
    } else {
        0.
    };
    let response = ui
        .horizontal(|ui| {
            let step_enabled = |delta| {
                enabled
                    && shifted(kind, &editor.text, delta)
                        .is_some_and(|v| encode_bounded(kind, &v, bounds.as_ref()).is_ok())
            };
            let decrease_enabled = step_enabled(-1.);
            let increase_enabled = step_enabled(1.);
            let decrease = ui.add_enabled(
                decrease_enabled,
                egui::Button::new(RichText::new("−").size(24.)).min_size(egui::vec2(36., 46.)),
            );
            decrease.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    decrease_enabled,
                    "Decrease value",
                )
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
                increase_enabled,
                egui::Button::new(RichText::new("+").size(24.)).min_size(egui::vec2(36., 46.)),
            );
            increase.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    increase_enabled,
                    "Increase value",
                )
            });
            #[cfg(test)]
            {
                editor.increase_rect = Some(increase.rect);
            }
            if increase.clicked() {
                step = 1.;
            }
            response
        })
        .inner;
    #[cfg(test)]
    {
        editor.value_rect = Some(response.rect);
    }
    // One click should let the user replace the whole number. Once focused,
    // subsequent clicks still position the caret for editing individual digits.
    if enabled
        && response.gained_focus()
        && !response.changed()
        && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), value_id)
    {
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(editor.text.chars().count()),
            )));
        state.store(ui.ctx(), value_id);
    }
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, enabled, "Value"));
    ui.ctx().accesskit_node_builder(value_id, |node| {
        node.add_action(egui::accesskit::Action::SetValue)
    });
    response.clone().on_hover_text(
        "Click to type, then press Enter or click away to apply. Use ↑ or ↓ to adjust by one.",
    );
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
    if step != 0. {
        // A step is relative to the draft the user can see, including a change
        // committed by focus loss in this frame, not an older backend reading.
        let base = if editor.dirty { &editor.text } else { &current };
        if let Some(value) = shifted(kind, base, step) {
            editor.text = value;
            editor.error = None;
            commit = true;
        } else {
            editor.error = Some("Enter a valid number before using + or −.".into());
            commit = false;
        }
    }
    // Only a known domain range can justify a slider. A small observed value
    // or a write-limit is not evidence that the setting itself has that range.
    if let Some(range) = bounds.as_ref().filter(|r| {
        r.start() < r.end()
            && r.end() - r.start() <= 20.
            // The slider uses floating-point offsets. Only exact integral
            // endpoints are suitable for an integer slider; typing stays exact.
            && (!(4..=11).contains(&kind)
                || [*r.start(), *r.end()].iter().all(|v| {
                    v.fract() == 0. && v.abs() <= 9_007_199_254_740_991.
                }))
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
        match encode_bounded(kind, &editor.text, bounds.as_ref()) {
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
    fn one_click_then_typing_replaces_the_number_and_enter_applies_once() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        let render = |editor: &mut Editor, events| {
            let mut requested = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    requested = draw(ui, editor, 8, &json!("123"), true, None);
                },
            );
            output.textures_delta.clear();
            requested
        };
        assert!(render(&mut editor, vec![]).is_none());
        let pos = editor.value_rect.unwrap().center();
        let pointer = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        assert!(
            render(
                &mut editor,
                vec![egui::Event::PointerMoved(pos), pointer(true)]
            )
            .is_none()
        );
        assert!(render(&mut editor, vec![pointer(false)]).is_none());
        assert!(render(&mut editor, vec![egui::Event::Text("7".into())]).is_none());
        assert_eq!(editor.text, "7");
        let enter = egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        assert_eq!(render(&mut editor, vec![enter]).as_deref(), Some("7"));
        assert!(render(&mut editor, vec![]).is_none());
    }

    #[test]
    fn numeric_cards_only_offer_sliders_for_known_small_ranges() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        for (bounds, has_slider) in [
            (None, false),
            (Some(1.0..=5.0), true),
            (Some(0.0..=1000.0), false),
            (Some(f64::NAN..=5.0), false),
            (Some(1.5..=5.5), false),
            (
                Some(9_007_199_254_740_992.0..=9_007_199_254_740_996.0),
                false,
            ),
        ] {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                assert!(draw(ui, &mut editor, 8, &json!("3"), true, bounds.clone()).is_none());
            });
            output.textures_delta.clear();
            assert_eq!(editor.slider_rect.is_some(), has_slider);
        }
    }

    #[test]
    fn known_bounds_apply_to_direct_entry_arrows_and_buttons() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        let render = |editor: &mut Editor, events, current: &str| {
            let mut requested = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    requested = draw(ui, editor, 8, &json!(current), true, Some(1.0..=5.0));
                },
            );
            output.textures_delta.clear();
            requested
        };
        assert!(render(&mut editor, vec![], "3").is_none());
        let id = editor.value_id.unwrap();
        let entry = |value: &str| {
            egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::SetValue,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: id.value().into(),
                data: Some(egui::accesskit::ActionData::Value(value.into())),
            })
        };
        assert!(render(&mut editor, vec![entry("6")], "3").is_none());
        assert_eq!(editor.error.as_deref(), Some("Choose a value from 1 to 5."));
        assert_eq!(
            render(&mut editor, vec![entry("5")], "3").as_deref(),
            Some("5")
        );

        // Keyboard adjustments must obey the same range as the slider.
        ctx.memory_mut(|m| m.request_focus(id));
        editor.text = "5".into();
        editor.focused = true;
        let up = egui::Event::Key {
            key: egui::Key::ArrowUp,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        assert!(render(&mut editor, vec![up], "5").is_none());
        assert!(editor.error.is_some());

        // At the endpoint the + button is disabled, with no write request.
        ctx.memory_mut(|m| m.surrender_focus(id));
        editor.focused = false;
        assert!(render(&mut editor, vec![], "5").is_none());
        let pos = editor.increase_rect.unwrap().center();
        let click = vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            },
        ];
        assert!(render(&mut editor, click, "5").is_none());
    }

    #[test]
    fn bounds_check_preserves_integer_precision_and_actual_float_storage() {
        let range = 0.0..=9_007_199_254_740_992.0;
        assert!(encode_bounded(11, "9007199254740992", Some(&range)).is_ok());
        assert!(encode_bounded(11, "9007199254740993", Some(&range)).is_err());
        assert!(encode_bounded(11, "18446744073709551615", None).is_ok());
        assert!(encode_bounded(8, "1", Some(&(1.5..=5.5))).is_err());
        assert!(encode_bounded(8, "2", Some(&(1.5..=5.5))).is_ok());
        // This decimal rounds up to 0.1f32, beyond the exact f64 endpoint.
        assert!(encode_bounded(12, "0.1", Some(&(0.0..=0.1))).is_err());
        assert!(encode_bounded(13, "0.1", Some(&(0.0..=0.1))).is_ok());
    }

    #[test]
    fn arrows_step_the_visible_draft_once_and_obey_disabled_state() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        let render = |editor: &mut Editor, events, enabled| {
            let mut requested = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    requested = draw(ui, editor, 11, &json!("3"), enabled, None);
                },
            );
            output.textures_delta.clear();
            requested
        };
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        assert!(render(&mut editor, vec![], true).is_none());
        ctx.memory_mut(|m| m.request_focus(editor.value_id.unwrap()));
        editor.text = "18446744073709551614".into();
        editor.dirty = true;
        editor.focused = true;
        assert_eq!(
            render(&mut editor, vec![key(egui::Key::ArrowUp)], true).as_deref(),
            Some("18446744073709551615")
        );
        assert!(render(&mut editor, vec![], true).is_none());
        assert!(render(&mut editor, vec![key(egui::Key::ArrowDown)], false).is_none());
    }

    #[test]
    fn unsupported_readings_cannot_offer_edit_or_freeze() {
        assert!(supported_value(11, &json!("18446744073709551615")));
        assert!(supported_value(2, &json!({"raw": 1, "boolean": true})));
        for (kind, value) in [
            (2, json!({"raw": 2, "boolean": true})),
            (2, json!({"raw": 0, "boolean": true})),
            (2, json!({"raw": 0})),
            (13, json!("NaN")),
            (13, json!("Infinity")),
            (3, json!("A")),
        ] {
            assert!(!supported_value(kind, &value));
        }
    }

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

    #[test]
    fn step_uses_the_visible_uncommitted_number_without_losing_integer_precision() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        let render = |editor: &mut Editor, events| {
            let mut requested = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    requested = draw(ui, editor, 11, &json!("3"), true, None);
                },
            );
            output.textures_delta.clear();
            requested
        };
        assert!(render(&mut editor, vec![]).is_none());
        ctx.memory_mut(|m| m.request_focus(editor.value_id.unwrap()));
        editor.text = "18446744073709551614".into();
        editor.dirty = true;
        editor.focused = true;
        let pos = editor.increase_rect.unwrap().center();
        let events = vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            },
        ];
        assert_eq!(
            render(&mut editor, events).as_deref(),
            Some("18446744073709551615")
        );
    }
}
