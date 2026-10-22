//! Review exact completed experiment images; no further capture or target access.
use super::*;
use crate::runtime_naming_bridge::{read_archive, save_new};
use reversebolt_core::{activity_experiment::Review, engine::View};
use reversebolt_observer::runtime_activity::{Input, Receipt};
use std::sync::{Arc, RwLock};

pub fn run(
    engine: &mut Engine,
    step: &Step,
    owner: &OperationOwner,
    base: &Path,
    published: &Arc<RwLock<View>>,
) -> Result<Value> {
    let result = run_with(engine, step, owner, base, |model, input| {
        tokio::runtime::Runtime::new()
            .map_err(|e| e.to_string())?
            .block_on(async {
                let observer = reversebolt_observer::Observer::connect(model).await?;
                Ok(observer.runtime_activity(input).await)
            })
    });
    crate::observation_pipeline::checkpoint(engine, published)?;
    result
}

fn completed_frames(
    engine: &Engine,
    plan: &Plan,
    directory: &Path,
    job: u64,
    reference: &str,
) -> Result<Vec<String>> {
    let live = &engine.live.as_ref().ok_or("Application disconnected")?.view;
    plan.check_completed(live)?;
    let mut ids = Vec::new();
    let mut previous = None;
    for index in 0..activity::STEPS {
        let frame = live
            .frames
            .iter()
            .find(|f| f.number == plan.first_number + index)
            .ok_or("An experiment response image is missing")?;
        let intent: Value =
            serde_json::from_slice(&read(&directory.join(format!("{index:02}.intent.json")))?)
                .map_err(|e| e.to_string())?;
        let outcome: Value =
            serde_json::from_slice(&read(&directory.join(format!("{index:02}.outcome.json")))?)
                .map_err(|e| e.to_string())?;
        if intent["job"] != job
            || intent["plan"] != reference
            || outcome["success"] != true
            || outcome["index"] != index
            || outcome["frame_id"] != frame.id
            || outcome["key"] != plan.key
            || outcome["effect"]
                != if activity::action(index, plan.key).is_some() {
                    "Applied"
                } else {
                    "NotApplicable"
                }
            || outcome["frame"] != json!(frame)
            || outcome["experiment"] != intent["plan"]
            || intent["index"] != index
        {
            return Err("Input experiment receipts are incomplete or changed".into());
        }
        plan.check_capture(index, frame, previous)?;
        previous = Some(frame);
        ids.push(frame.id.clone());
    }
    Ok(ids)
}

pub(super) fn run_with(
    engine: &mut Engine,
    step: &Step,
    owner: &OperationOwner,
    base: &Path,
    infer: impl FnOnce(&str, &Input) -> Result<Receipt>,
) -> Result<Value> {
    if step.index != activity::STEPS || owner.cancel.load(Ordering::SeqCst) {
        return Err("Input review stopped or its step changed".into());
    }
    let plan = load(base, &step.activity)?;
    if step.key != plan.key {
        return Err("Input review key changed".into());
    }
    let directory = location(base, &step.activity)?;
    let ids = completed_frames(engine, &plan, &directory, owner.job, &step.activity)?;
    // A repeated call returns the already-published result, never another model call.
    if engine
        .live
        .as_ref()
        .unwrap()
        .view
        .input_experiments
        .iter()
        .any(|r| r.experiment == step.activity)
    {
        return Ok(
            json!({"success":true,"experiment":step.activity,"index":step.index,
            "effect":"NotApplicable","replayed":true,"review":read_result(engine,&step.activity,base)?["review"]}),
        );
    }
    let review_dir = directory.join("review");
    fs::create_dir_all(&review_dir).map_err(|e| e.to_string())?;
    let review_id = uuid::Uuid::new_v4().to_string();
    save(
        &review_dir.join("intent.json"),
        &json!({"experiment":step.activity,"job":owner.job,"review_id":review_id,
        "frame_ids":ids,"model":plan.observer_model}),
    )?;
    let mut summary = Review {
        experiment: step.activity.clone(),
        job: owner.job,
        frame_ids: ids.clone(),
        model: plan.observer_model.clone(),
        status: "unavailable".into(),
        error: None,
        source: None,
        context: None,
        receipt_sha256: String::new(),
    };
    let mut receipt = None;
    let measurement = crate::runtime_naming_bridge::prepare_activity_frames(
        engine.live.as_ref().unwrap(),
        &review_dir,
        &ids,
    )?;
    let evidence = read_archive(&review_dir.join("play-source.json"))?;
    match measurement {
        None => {
            if let Some(error) = evidence["unavailable"].as_str() {
                summary.error = Some(error.into());
            } else {
                summary.status = "no_separated_response".into();
            }
        }
        Some(input) => {
            summary.source = Some(input.source.clone());
            summary.status = "measured".into();
            if !owner.cancel.load(Ordering::SeqCst)
                && let Some(model) = &plan.observer_model
            {
                match infer(model, &input) {
                    Err(error) => {
                        summary.status = "model_unavailable".into();
                        summary.error = Some(error);
                    }
                    Ok(result) => {
                        if result.source != input.source
                            || result.appearances != input.appearances
                            || result.regions != input.regions
                        {
                            summary.error =
                                Some("Model receipt differs from its measured input".into());
                        } else if let Some(error) = &result.error {
                            summary.error = Some(error.clone());
                        } else {
                            summary.context = result
                                .context()
                                .map(|c| c.bind_interpretation(&review_id, model))
                                .transpose()?;
                            if summary.context.is_none() {
                                summary.error = Some(
                                    "Model interpretation has no valid grounded context".into(),
                                );
                            }
                        }
                        summary.status = if summary.error.is_some() {
                            "model_unavailable"
                        } else {
                            "interpreted"
                        }
                        .into();
                        receipt = Some(result);
                    }
                }
            }
        }
    }
    // Stop/rebind cannot publish an interpretation after the request returns.
    let cancelled = owner.cancel.load(Ordering::SeqCst);
    plan.check_completed(&engine.live.as_ref().ok_or("Application disconnected")?.view)?;
    if cancelled {
        summary.status = "cancelled".into();
        summary.source = None;
        summary.context = None;
    }
    let result = json!({"schema":1,"experiment":step.activity,"job":owner.job,"review_id":review_id,"model":plan.observer_model,
        "status":summary.status,"error":summary.error,"frame_ids":ids,"measurement":evidence,
        "interpretation":receipt,"cancelled":cancelled,"limitation":activity::LIMITATION});
    save_new(&review_dir.join("result.json"), &result)?;
    summary.receipt_sha256 =
        visual::digest(&fs::read(review_dir.join("result.json")).map_err(|e| e.to_string())?);
    let live = &mut engine.live.as_mut().unwrap().view;
    if live.input_experiments.len() >= 4 {
        live.input_experiments.remove(0);
    }
    live.input_experiments.push(summary.clone());
    engine.event(
        "input-experiment",
        format!(
            "Input response review {}: {}. No gameplay control verified.",
            step.activity, summary.status
        ),
    );
    Ok(
        json!({"success":!cancelled,"experiment":step.activity,"index":step.index,
        "effect":"NotApplicable","review":summary,"error":if cancelled {Some("Input review stopped")} else {None}}),
    )
}

pub fn read_result(engine: &Engine, reference: &str, base: &Path) -> Result<Value> {
    let live = &engine
        .live
        .as_ref()
        .ok_or("Connect the original application investigation first")?
        .view;
    let summary = live
        .input_experiments
        .iter()
        .find(|r| r.experiment == reference)
        .ok_or("No completed input review belongs to this observation batch")?;
    let path = location(base, reference)?.join("review/result.json");
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(12 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("Input review exceeds its archive budget".into());
    }
    let result: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if visual::digest(&bytes) != summary.receipt_sha256
        || result["experiment"] != reference
        || result["frame_ids"] != json!(summary.frame_ids)
        || result["job"] != summary.job
    {
        return Err("Stored input review checksum or scope changed".into());
    }
    // The result is historical evidence within this batch, never a current value.
    for id in &summary.frame_ids {
        engine
            .live
            .as_ref()
            .unwrap()
            .read_frame(&live.investigation_id, id)?;
    }
    Ok(json!({"review":summary,"result":result,"historical":true}))
}
