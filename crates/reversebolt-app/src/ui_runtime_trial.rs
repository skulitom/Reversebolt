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
    let freeze = state.freezes.iter().rev().find(|f| {
        f.investigation == live.investigation_id
            && f.catalog == live.runtime.catalog_id
            && f.candidate == card.candidate.id
    });
    let frozen = freeze.is_some_and(|f| f.active());
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
        freeze_changed=ui.add_enabled(frozen || (enabled && card.status=="read"), egui::Checkbox::new(&mut holding,"Freeze"))
            .on_hover_text("Keep this value at the chosen number or state. Uncheck to let it change again. Checks pause while discovery runs.").changed();
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
