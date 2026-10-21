use crate::{
    service::Handle,
    theme::{MUTED, WARNING},
};
use eframe::egui::{self, RichText};
use reversebolt_core::{changes::Phase, engine::View, runtime::Card, runtime_trials::Trial};
use serde_json::json;

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
    let requested = crate::ui_value_control::draw(
        ui,
        editor,
        card.candidate.type_code,
        &card.value,
        enabled && card.status == "read",
    );
    let mut undo = None;
    ui.horizontal(|ui| {
        let label = latest.map_or(
            if card.status == "read" {
                "Learning"
            } else {
                "Unavailable"
            },
            |t| match t.phase {
                Phase::Written => "Changed",
                Phase::Restored => "Restored",
                Phase::Uncertain => "Outcome unknown",
                Phase::Writing | Phase::Restoring => "Applying…",
                Phase::Refused | Phase::Unavailable | Phase::Expired => "Couldn’t apply",
                _ => "Learning",
            },
        );
        ui.label(RichText::new(label).small().color(
            if latest.is_some_and(|t| t.phase.unresolved()) {
                WARNING
            } else {
                MUTED
            },
        ))
        .on_hover_text("Meaning and gameplay effects are still unverified.");
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
    if let Some(id) = undo {
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
