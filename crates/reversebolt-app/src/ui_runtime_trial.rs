use crate::{
    service::Handle,
    theme::{MUTED, WARNING},
};
use eframe::egui::{self, RichText};
use reversebolt_core::{changes::Phase, engine::View, runtime::Card, runtime_trials::Trial};
use serde_json::json;

fn comparison_review<'a>(state: &'a View, card: &Card) -> Option<&'a serde_json::Value> {
    let live = state.live.as_ref()?;
    latest_review(
        &state.runtime_tests,
        &live.investigation_id,
        &live.runtime.catalog_id,
        &card.candidate.id,
    )
}

fn latest_review<'a>(
    tests: &'a [serde_json::Value],
    investigation: &str,
    catalog: &str,
    candidate: &str,
) -> Option<&'a serde_json::Value> {
    // A new comparison supersedes an older positive note, even before it finishes.
    let latest = tests
        .iter()
        .filter(|test| {
            test["investigation_id"] == investigation
                && test["catalog_id"] == catalog
                && test["candidate_id"] == candidate
        })
        .max_by_key(|test| test["created_unix_ms"].as_u64().unwrap_or(0))?;
    latest.get("last_regional_review")
}

#[test]
fn last_comparison_note_is_scoped_and_a_new_test_hides_old_support() {
    let positive = json!({"assessment":{"status":"regional_change_and_return_observed"}});
    let old = json!({"investigation_id":"game-a","catalog_id":"catalog-a","candidate_id":"path-a",
        "created_unix_ms":10,"last_regional_review":positive});
    let mut tests = vec![old.clone()];
    assert_eq!(
        latest_review(&tests, "game-a", "catalog-a", "path-a"),
        Some(&positive)
    );
    for key in ["investigation_id", "catalog_id", "candidate_id"] {
        let mut other = old.clone();
        other[key] = json!("different");
        other["created_unix_ms"] = json!(30);
        other["last_regional_review"] = json!({"assessment":{"status":"inconclusive"}});
        tests.push(other);
    }
    assert_eq!(
        latest_review(&tests, "game-a", "catalog-a", "path-a"),
        Some(&positive)
    );
    let mut newer = old;
    newer["created_unix_ms"] = json!(20);
    newer
        .as_object_mut()
        .unwrap()
        .remove("last_regional_review");
    tests.push(newer);
    assert!(latest_review(&tests, "game-a", "catalog-a", "path-a").is_none());
    for (status, label) in [
        (
            "regional_change_and_return_observed",
            "Last comparison: response changed",
        ),
        (
            "regional_change_not_observed",
            "Last comparison: no change found",
        ),
        ("inconclusive", "Last comparison: uncertain"),
        ("unsupported", "Last comparison: unavailable"),
    ] {
        let review = json!({"assessment":{"status":status}});
        assert_eq!(crate::runtime_test_review::note(&review).0, label);
    }
}

pub fn comparison_note(ui: &mut egui::Ui, state: &View, card: &Card) {
    if let Some(review) = comparison_review(state, card) {
        let (label, explanation) = crate::runtime_test_review::note(review);
        ui.label(RichText::new(label).size(12.).color(MUTED))
            .on_hover_text(explanation);
    }
}

fn trials<'a>(state: &'a View, card: &'a Card) -> impl DoubleEndedIterator<Item = &'a Trial> {
    state.runtime_trials.iter().filter(move |t| {
        state.live.as_ref().is_some_and(|live| {
            t.context.investigation == live.investigation_id
                && t.context.catalog_id == live.runtime.catalog_id
                && t.candidate_id == card.candidate.id
        })
    })
}

pub fn controls(
    ui: &mut egui::Ui,
    handle: &Handle,
    state: &View,
    card: &Card,
    editor: &mut crate::ui_value_control::Editor,
    enabled: bool,
) {
    let Some(live) = &state.live else {
        return;
    };
    let latest = trials(state, card).next_back();
    let freeze = state.freezes.iter().rev().find(|f| {
        f.investigation == live.investigation_id
            && f.catalog == live.runtime.catalog_id
            && f.candidate == card.candidate.id
    });
    let frozen = freeze.is_some_and(|f| f.active());
    let supported = crate::ui_value_control::supported_value(card.candidate.type_code, &card.value);
    let mut holding = frozen;
    let mut freeze_changed = false;
    let requested = if card.status == "read" {
        crate::ui_value_control::draw(
            ui,
            editor,
            card.candidate.type_code,
            &card.value,
            enabled && card.status == "read",
            None,
        )
    } else {
        ui.label(RichText::new("—").size(38.).color(MUTED));
        None
    };
    let mut undo = None;
    ui.horizontal(|ui| {
        freeze_changed=ui.add_enabled(frozen || (enabled && card.status=="read" && supported), egui::Checkbox::new(&mut holding,"Freeze"))
            .on_hover_text(if supported || frozen {
                "Keep this value at the chosen number or state. Uncheck to let it change again. Checks pause while discovery runs."
            } else {
                "Freeze needs a supported number or On/Off value."
            }).changed();
        if let Some(f)=freeze.filter(|f|f.phase=="uncertain" || (f.phase=="stopped" && f.reason.contains("changed"))) {
            ui.label(RichText::new("Stopped").small().color(WARNING)).on_hover_text(&f.reason);
        } else if latest.is_some_and(|t|t.phase.unresolved()) {
            ui.label(RichText::new("Check needed").small().color(WARNING));
        }
        if let Some(t) = trials(state, card)
            .rev()
            .find(|t| t.phase == Phase::Written && t.restoration.is_none())
            && ui
                .add_enabled(enabled, egui::Button::new("Undo").small())
                .clicked()
        {
            undo = Some(t.id.clone());
        }
    });
    if freeze_changed && !holding {
        if let Some(f) = freeze {
            handle.submit(
                "set_runtime_freeze",
                json!({"freeze_id":f.id,"enabled":false}),
            );
        }
    } else if (freeze_changed && holding) || (frozen && requested.is_some()) {
        let value = requested.unwrap_or_else(|| crate::ui_value_control::text(&card.value));
        handle.submit("set_runtime_freeze",json!({"freeze_id":uuid::Uuid::new_v4().to_string(),"enabled":true,"investigation_id":live.investigation_id,
            "catalog_id":live.runtime.catalog_id,"after_revision":live.runtime.revision,"candidate_id":card.candidate.id,"value":value}));
    } else if let Some(id) = undo {
        handle.submit("restore_runtime_change", json!({"change_id":id}));
    } else if let Some(value) = requested {
        handle.submit("set_runtime_value",json!({"change_id":uuid::Uuid::new_v4().to_string(),"investigation_id":live.investigation_id,
            "catalog_id":live.runtime.catalog_id,"after_revision":live.runtime.revision,"candidate_id":card.candidate.id,"value":value}));
    }
}

fn restore_button(ui: &mut egui::Ui, handle: &Handle, trial: &Trial, enabled: bool) {
    if trial.phase == Phase::Written && trial.restoration.is_none()
        && ui.add_enabled(enabled, egui::Button::new("Restore value"))
            .on_hover_text("Checks the original reference and current value before restoring. Other gameplay consequences may remain.").clicked() {
        handle.submit("restore_runtime_change", json!({"change_id":trial.id}));
    }
}

pub fn history(ui: &mut egui::Ui, handle: &Handle, state: &View, card: &Card, enabled: bool) {
    if let Some(review) = comparison_review(state, card) {
        let (label, explanation) = crate::runtime_test_review::note(review);
        ui.label(label);
        ui.label(explanation);
        if let Some(time) = review["reviewed_unix_ms"].as_u64() {
            ui.label(crate::ui_live::observation_age(
                u128::from(time),
                reversebolt_core::changes::now_ms(),
            ));
        }
    }
    for trial in trials(state, card).rev() {
        egui::CollapsingHeader::new(format!("Edit reviewed at {}", trial.patch.prepared_unix_ms))
            .id_salt(&trial.id).show(ui, |ui| {
                ui.label(&trial.reason);
                for (name, attempt) in [("Edit", &trial.write), ("Restoration", &trial.restoration)] {
                    if let Some(attempt) = attempt {
                        ui.label(format!("{name}: {}", attempt.outcome));
                        if let Some(error) = &attempt.error { ui.colored_label(WARNING, error); }
                    }
                }
                restore_button(ui, handle, trial, enabled);
                ui.label("The checks verify bytes at the observed reference. Address reuse remains possible; restored bytes do not undo all gameplay consequences.");
            });
    }
}
