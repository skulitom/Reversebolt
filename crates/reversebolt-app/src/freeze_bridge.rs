use crate::{runtime_bridge, runtime_trial_bridge::evidence};
use reversebolt_core::{
    Command, Result,
    changes::now_ms,
    engine::{Engine, View},
    freezes::Freeze,
    runtime::{Task, scalar_value},
    runtime_patch::encode,
    runtime_trials::classify,
};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, RwLock, atomic::AtomicBool},
};

pub fn command(engine: &mut Engine, command: &Command) -> Result<Value> {
    let id = command.args["freeze_id"]
        .as_str()
        .ok_or("Supply the freeze identity.")?;
    if !uuid::Uuid::parse_str(id).is_ok_and(|u| u.to_string() == id) {
        return Err("Invalid freeze identity.".into());
    }
    let enabled = command.args["enabled"]
        .as_bool()
        .ok_or("Choose whether Freeze is on or off.")?;
    if let Some(f) = engine.view.freezes.iter_mut().find(|f| f.id == id) {
        if command.source != "desktop" && command.source != f.actor {
            return Err("This freeze belongs to another controller.".into());
        }
        if !enabled {
            f.stop("Freeze switched off. The last value is left in place.");
            return Ok(json!(f));
        }
        if command.args["investigation_id"] != f.investigation
            || command.args["catalog_id"] != f.catalog
            || command.args["candidate_id"] != f.candidate
            || command.args["value"] != f.value
            || command.args["after_revision"] != f.requested_revision
        {
            return Err("This freeze identity belongs to a different request.".into());
        }
        return Ok(json!(f)); // Replays never restart a stopped freeze.
    }
    if !enabled {
        return Err("Freeze not found.".into());
    }
    if engine.view.freezes.len() >= 128 {
        return Err("Freeze history is full.".into());
    }
    if engine.view.freezes.iter().filter(|f| f.active()).count() >= 8 {
        return Err("Hold up to eight values at once.".into());
    }
    let live = engine
        .live
        .as_mut()
        .ok_or("Connect an application first.")?;
    live.refresh_connection();
    let rt = &live.view.runtime;
    if !live.view.connected
        || !rt.available
        || command.args["investigation_id"] != live.view.investigation_id
        || command.args["catalog_id"] != rt.catalog_id
        || command.args["after_revision"] != rt.revision
    {
        return Err("The application or values changed. Try again with the current card.".into());
    }
    let candidate = rt
        .cards
        .iter()
        .find(|c| command.args["candidate_id"] == c.candidate.id && c.status == "read")
        .ok_or("Choose an available value.")?
        .candidate
        .clone();
    let value = command.args["value"]
        .as_str()
        .ok_or("Supply the value to hold as text.")?;
    encode(candidate.type_code, value)?;
    if engine
        .view
        .freezes
        .iter()
        .any(|f| f.holds(&candidate.locator.scope))
        || engine.runtime_trials.holds(&candidate.locator.scope)
        || engine.changes.holds_process(
            live.view
                .memory_process
                .as_ref()
                .unwrap_or(&live.view.process),
        )
    {
        return Err("An earlier write has an unknown outcome. Further writes are held.".into());
    }
    for f in &mut engine.view.freezes {
        if f.investigation == live.view.investigation_id && f.candidate == candidate.id {
            if command.source != "desktop" && f.actor != command.source && f.active() {
                return Err("The desktop owns this frozen value.".into());
            }
            f.stop("Replaced by a new requested value.");
        }
    }
    let f = Freeze {
        id: id.into(),
        actor: command.source.clone(),
        investigation: live.view.investigation_id.clone(),
        catalog: rt.catalog_id.clone(),
        requested_revision: rt.revision,
        candidate: candidate.id,
        value: value.into(),
        kind: candidate.type_code,
        path: candidate.locator,
        identity: None,
        phase: "active".into(),
        reason: "Checking and holding this value while connected.".into(),
        checks: 0,
        writes: 0,
        checked_ms: 0,
        intent: None,
        evidence: None,
    };
    engine.view.freezes.push(f.clone());
    Ok(json!(f))
}

pub fn tick(
    engine: &mut Engine,
    id: &str,
    base: &Path,
    published: &Arc<RwLock<View>>,
) -> Result<Value> {
    let index = engine
        .view
        .freezes
        .iter()
        .position(|f| f.id == id)
        .ok_or("Freeze not found.")?;
    if engine.view.freezes[index].phase != "active" {
        return Ok(json!(engine.view.freezes[index]));
    }
    let result = step(engine, index, base, published);
    if let Err(error) = &result {
        let f = &mut engine.view.freezes[index];
        if f.phase == "writing" {
            f.phase = "uncertain".into();
            f.reason = format!("Freeze write outcome unknown: {error}");
        } else {
            f.stop(error);
        }
    }
    engine.view.live = engine.live.as_ref().map(|s| s.view.clone());
    if let Err(error) = engine.save() {
        let f = &mut engine.view.freezes[index];
        f.phase = "uncertain".into();
        f.reason = format!("Freeze result could not be saved. Further writes are held: {error}");
        return Err(f.reason.clone());
    }
    result.map(|_| json!(engine.view.freezes[index]))
}

fn step(
    engine: &mut Engine,
    index: usize,
    base: &Path,
    published: &Arc<RwLock<View>>,
) -> Result<()> {
    let f = engine.view.freezes[index].clone();
    let live = engine
        .live
        .as_mut()
        .ok_or("Application disconnected; Freeze stopped.")?;
    live.refresh_connection();
    if !live.view.connected
        || live.view.investigation_id != f.investigation
        || live.view.runtime.catalog_id != f.catalog
    {
        return Err("Application or catalogue changed; Freeze stopped.".into());
    }
    let info = live
        .view
        .memory_process
        .as_ref()
        .unwrap_or(&live.view.process)
        .clone();
    if runtime_bridge::runtime_scope(&info) != f.path.scope {
        return Err("Application identity changed; Freeze stopped.".into());
    }
    if engine.runtime_trials.holds(&f.path.scope)
        || engine.changes.holds_process(&info)
        || engine.view.freezes.iter().any(|x| x.holds(&f.path.scope))
    {
        return Err("An earlier write is unresolved; Freeze stopped.".into());
    }
    let task = Task::Refresh {
        catalog_id: f.catalog.clone(),
        after_revision: live.view.runtime.revision,
        ids: vec![f.candidate.clone()],
    };
    let paths = live.view.runtime.paths(&task)?;
    if paths.len() != 1 || paths[0] != f.path {
        return Err("Value reference changed; Freeze stopped.".into());
    }
    let start = now_ms();
    let read =
        runtime_bridge::run(&info, &paths, base, &AtomicBool::new(false)).and_then(evidence)?;
    let end = now_ms();
    engine.live.as_mut().unwrap().view.runtime.apply_refresh(
        &task,
        &read.report,
        [start, end],
        &read.path,
    )?;
    let patch = engine.view.freezes[index].check(&read.report["checks"][0], end)?;
    if engine.view.freezes.iter().enumerate().any(|(i, other)| {
        i != index && other.active() && engine.view.freezes[index].overlaps(other)
    }) {
        return Err(
            "Another control is already holding these bytes. Freeze stopped before writing.".into(),
        );
    }
    engine.view.freezes[index].evidence = Some(read.path);
    let Some(patch) = patch else {
        return Ok(());
    };
    engine.view.freezes[index].phase = "writing".into();
    engine.view.freezes[index].intent = Some(patch.clone());
    // This checkpoint must reach disk before the helper is allowed to write.
    crate::observation_pipeline::checkpoint(engine, published)?;
    let result = runtime_bridge::run_patch(&info, &patch, base).and_then(evidence)?;
    let outcome = classify(&patch, &result.report);
    let f = &mut engine.view.freezes[index];
    f.evidence = Some(result.path);
    f.checked_ms = now_ms();
    f.phase = match outcome {
        "written" => "active",
        "refused" => "stopped",
        _ => "uncertain",
    }
    .into();
    f.reason = match outcome {
        "written" => "Holding the requested value.",
        "refused" => "The value changed during its check. Freeze stopped.",
        _ => "Write outcome unknown. Freeze stopped; further writes are held.",
    }
    .into();
    let runtime = &mut engine.live.as_mut().unwrap().view.runtime;
    runtime.revision = runtime
        .revision
        .checked_add(1)
        .ok_or("Runtime revision limit reached.")?;
    if outcome == "written" {
        f.writes = f
            .writes
            .checked_add(1)
            .ok_or("Freeze write count limit reached.")?;
        if let Some(card) = runtime
            .cards
            .iter_mut()
            .find(|c| c.candidate.id == f.candidate)
        {
            card.value = scalar_value(patch.type_code, &patch.replacement)?;
            card.status = "read".into();
            card.observed_unix_ms = now_ms();
        }
    }
    Ok(())
}
