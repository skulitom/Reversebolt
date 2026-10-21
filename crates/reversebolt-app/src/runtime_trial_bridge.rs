//! Human/MCP runtime trial commands through the shared, exclusive engine worker.
use crate::runtime_bridge;
use reversebolt_core::{
    Command, Result,
    changes::{Phase, now_ms},
    engine::{Engine, View},
    runtime::{Task, scalar_value},
    runtime_patch::{Patch, encode},
    runtime_trials::{Context, Evidence, Trial},
};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, RwLock, atomic::AtomicBool},
};

pub(crate) fn evidence(outcome: runtime_bridge::Outcome) -> Result<Evidence> {
    let path = outcome
        .evidence
        .join("report.json")
        .to_string_lossy()
        .into_owned();
    if outcome.unresolved || outcome.cancelled || outcome.error.is_some() {
        return Err(format!(
            "{} Evidence: {path}",
            outcome
                .error
                .unwrap_or_else(|| "Runtime operation did not settle.".into())
        ));
    }
    Evidence::new(path, outcome.report.ok_or("Runtime report unavailable.")?)
}

pub fn command(
    engine: &mut Engine,
    command: &Command,
    base: &Path,
    published: &Arc<RwLock<View>>,
) -> Result<Value> {
    let direct = command.action == "set_runtime_value";
    let direct_id = if direct {
        let id = command.args["change_id"]
            .as_str()
            .ok_or("Supply a unique edit identity.")?;
        if id.len() != 36 || !uuid::Uuid::parse_str(id).is_ok_and(|uuid| uuid.to_string() == id) {
            return Err("Invalid edit identity.".into());
        }
        if let Ok(record) = engine.runtime_trials.get(id) {
            if !record.direct
                || command.args["investigation_id"] != record.context.investigation
                || command.args["catalog_id"] != record.context.catalog_id
                || command.args["candidate_id"] != record.candidate_id
                || command.args["after_revision"]
                    .as_u64()
                    .and_then(|r| r.checked_add(1))
                    != Some(record.context.revision)
                || command.args["value"]
                    .as_str()
                    .and_then(|v| encode(record.patch.type_code, v).ok())
                    .as_ref()
                    != Some(&record.patch.replacement)
            {
                return Err("This edit identity already belongs to a different request.".into());
            }
            return Ok(json!(record));
        }
        Some(id)
    } else {
        None
    };
    let prepare = command.action == "prepare_runtime_change" || direct;
    let restore = command.action == "restore_runtime_change";
    let id = if prepare {
        None
    } else {
        Some(
            command.args["change_id"]
                .as_str()
                .ok_or("Choose a runtime trial from the shared history.")?,
        )
    };
    if let Some(id) = id {
        let record = engine.runtime_trials.get(id)?;
        if (restore && record.restoration.is_some()) || (!restore && record.write.is_some()) {
            return Ok(json!(record));
        }
    }
    let session = engine
        .live
        .as_mut()
        .ok_or("Connect the original application first.")?;
    session.refresh_connection();
    if !session.view.connected || !session.view.runtime.available {
        return Err("No connected supported runtime is available.".into());
    }
    if direct && command.args["investigation_id"] != session.view.investigation_id {
        return Err("The application changed before this edit could begin.".into());
    }
    let info = session
        .view
        .memory_process
        .as_ref()
        .unwrap_or(&session.view.process)
        .clone();
    let mut context = Context {
        investigation: session.view.investigation_id.clone(),
        catalog_id: session.view.runtime.catalog_id.clone(),
        revision: session.view.runtime.revision,
        process: runtime_bridge::runtime_scope(&info),
    };
    if prepare {
        let candidate_id = command.args["candidate_id"]
            .as_str()
            .ok_or("Choose an internal-value card.")?;
        let desired = command.args["value"]
            .as_str()
            .ok_or("Supply the requested number as text.")?;
        let task: Task = serde_json::from_value(
            json!({"operation":"refresh","catalog_id":command.args["catalog_id"],
            "after_revision":command.args["after_revision"],"ids":[candidate_id]}),
        )
        .map_err(|e| e.to_string())?;
        let paths = session.view.runtime.paths(&task)?;
        let candidate = session
            .view
            .runtime
            .cards
            .iter()
            .find(|c| c.candidate.id == candidate_id)
            .ok_or("Card unavailable.")?
            .candidate
            .clone();
        encode(candidate.type_code, desired)?;
        session.view.runtime.phase = "inspecting".into();
        session.view.runtime.notice = None;
        crate::observation_pipeline::checkpoint(engine, published)?;
        let start = now_ms();
        let result =
            runtime_bridge::run(&info, &paths, base, &AtomicBool::new(false)).and_then(evidence);
        let end = now_ms();
        let session = engine.live.as_mut().unwrap();
        let baseline = match result {
            Ok(e) => e,
            Err(error) => {
                session.view.runtime.phase = "uncertain".into();
                session.view.runtime.notice = Some(error.clone());
                return Err(error);
            }
        };
        session.view.runtime.phase = "ready".into();
        session.view.runtime.apply_refresh(
            &task,
            &baseline.report,
            [start, end],
            &baseline.path,
        )?;
        context.revision = session.view.runtime.revision;
        let patch = Patch::prepare(
            paths[0].clone(),
            &baseline.report["checks"][0],
            desired,
            end,
        )?;
        crate::runtime_test_bridge::check_patch(engine, direct_id, &patch)?;
        let mut trial = Trial::prepare(
            context,
            candidate.name,
            command.source.clone(),
            patch,
            baseline,
        )?;
        if let Some(id) = direct_id {
            trial.id = id.into();
            trial.direct = true;
        }
        let trial = engine.runtime_trials.insert(trial)?;
        engine.event(
            &command.source,
            "Reviewed an internal-value trial. No value was changed.",
        );
        if direct {
            return self::command(
                engine,
                &Command {
                    action: "apply_runtime_change".into(),
                    args: json!({"change_id":trial.id}),
                    source: command.source.clone(),
                },
                base,
                published,
            );
        }
        return Ok(json!(trial));
    }
    if engine.changes.holds_process(&info) {
        return Err(
            "An earlier trial has an uncertain outcome. Further edits to this process are held."
                .into(),
        );
    }
    if context.revision == u64::MAX {
        return Err("Runtime revision limit reached. Reconnect before editing.".into());
    }
    session.view.runtime.phase = if restore { "restoring" } else { "editing" }.into();
    crate::observation_pipeline::checkpoint(engine, published)?;
    let result = engine
        .runtime_trials
        .execute(id.unwrap(), &context, restore, |patch| {
            runtime_bridge::run_patch(&info, patch, base).and_then(evidence)
        });
    let (trial, error) = match result {
        Ok(trial) => (trial, None),
        Err(error) => (engine.runtime_trials.get(id.unwrap())?, Some(error)),
    };
    let held = engine.runtime_trials.holds(&context.process);
    let session = engine.live.as_mut().unwrap();
    session.view.runtime.phase = "ready".into();
    session.view.runtime.notice = Some(error.clone().unwrap_or_else(|| trial.reason.clone()));
    if held || trial.phase == Phase::Uncertain {
        session.view.runtime.phase = "uncertain".into();
    }
    let attempt = if restore {
        trial.restoration.as_ref()
    } else {
        trial.write.as_ref()
    };
    if let Some(attempt) = attempt {
        session.view.runtime.revision += 1;
        if let Some(card) = session
            .view
            .runtime
            .cards
            .iter_mut()
            .find(|c| c.candidate.id == trial.candidate_id)
        {
            if matches!(attempt.outcome.as_str(), "written" | "already_original") {
                card.value = scalar_value(attempt.patch.type_code, &attempt.patch.replacement)?;
                card.status = "read".into();
                card.observed_unix_ms = attempt.completed_ms.unwrap_or_else(now_ms);
            } else if attempt.outcome == "refused"
                && let Some(e) = &attempt.evidence
            {
                let check = &e.report["checks"][0];
                if check["status"] == "read"
                    && check["type_code"] == card.candidate.type_code
                    && let Ok(bytes) = serde_json::from_value::<Vec<u8>>(check["bytes"].clone())
                    && let Ok(value) = scalar_value(card.candidate.type_code, &bytes)
                    && value == check["value"]
                {
                    card.value = value;
                    card.status = "read".into();
                    card.observed_unix_ms = attempt.completed_ms.unwrap_or_else(now_ms);
                } else {
                    card.status = "unavailable".into();
                    card.value = Value::Null;
                }
            } else {
                card.status = "unavailable".into();
                card.value = Value::Null;
            }
        }
    }
    engine.event(&command.source, trial.reason.clone());
    if let Some(error) = error {
        return Err(error);
    }
    Ok(json!(trial))
}
