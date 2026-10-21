use crate::{
    service::Handle,
    theme::{ACCENT, MUTED, WARNING},
};
use eframe::egui::{self, RichText};
use reversebolt_core::{
    changes::{Phase, now_ms},
    engine::View,
    runtime::{Card, scalar_value},
    runtime_patch::encode,
    runtime_trials::Trial,
};
use serde_json::{Value, json};

fn number(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
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
    draft: &mut String,
    enabled: bool,
) {
    let Some(live) = &state.live else {
        return;
    };
    let latest = trials(state, card).next_back();
    if (4..=13).contains(&card.candidate.type_code) {
        let desired = encode(card.candidate.type_code, draft).ok();
        let current = encode(card.candidate.type_code, &number(&card.value)).ok();
        ui.horizontal(|ui| {
            let label = ui.label("Try value");
            ui.add_enabled(enabled, egui::TextEdit::singleline(draft).desired_width(96.).char_limit(64)).labelled_by(label.id)
                .on_hover_text("Type an exact number. Trial changes are limited to 20 from a fresh reading.");
            if ui.add_enabled(enabled && card.status == "read" && desired.is_some() && desired != current,
                egui::Button::new("Review edit")).clicked() {
                handle.submit("prepare_runtime_change", json!({"catalog_id":live.runtime.catalog_id,
                    "after_revision":live.runtime.revision,"candidate_id":card.candidate.id,"value":draft}));
            }
        });
    }
    if let Some(trial) = latest {
        match trial.phase {
            Phase::Prepared => {
                let before = scalar_value(trial.patch.type_code, &trial.patch.expected)
                    .map(|v| number(&v))
                    .unwrap_or_default();
                let after = scalar_value(trial.patch.type_code, &trial.patch.replacement)
                    .map(|v| number(&v))
                    .unwrap_or_default();
                let fresh = trial.context.revision == live.runtime.revision
                    && trial
                        .patch
                        .validate(&trial.context.process, now_ms())
                        .is_ok();
                ui.label(RichText::new(format!("Try {before} → {after}")).color(ACCENT));
                ui.label(
                    RichText::new(if fresh {
                        "Experimental edit · effect unverified"
                    } else {
                        "Reading changed or review expired · review again"
                    })
                    .size(12.)
                    .color(MUTED),
                );
                let matches = encode(trial.patch.type_code, draft)
                    .is_ok_and(|b| b == trial.patch.replacement);
                if ui
                    .add_enabled(enabled && fresh && matches, egui::Button::new("Try edit"))
                    .clicked()
                {
                    handle.submit("apply_runtime_change", json!({"change_id":trial.id}));
                }
            }
            Phase::Written => {
                ui.label(
                    RichText::new(if trial.restoration.is_some() {
                        "Restore wasn't applied · Inspect"
                    } else {
                        "Value changed · effect unverified"
                    })
                    .size(12.)
                    .color(ACCENT),
                );
                restore_button(ui, handle, trial, enabled);
            }
            Phase::Restored => {
                ui.label(
                    RichText::new("Original value restored")
                        .size(12.)
                        .color(MUTED),
                );
            }
            Phase::Writing | Phase::Restoring => {
                ui.label("Waiting for the edit result…");
            }
            Phase::Uncertain => {
                ui.colored_label(WARNING, "Edit outcome unknown · further edits held");
            }
            _ => {
                ui.label(
                    RichText::new("Edit not applied · review again")
                        .size(12.)
                        .color(MUTED),
                );
            }
        }
    }
    // A newer review must not hide the way back from an earlier applied trial.
    if let Some(older) = trials(state, card).rev().find(|t| {
        t.phase == Phase::Written
            && t.restoration.is_none()
            && latest.is_none_or(|last| last.id != t.id)
    }) {
        ui.label(
            RichText::new("An earlier edit remains applied")
                .size(12.)
                .color(MUTED),
        );
        restore_button(ui, handle, older, enabled);
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

pub fn initial_draft(card: &Card) -> String {
    number(&card.value)
}
