//! Associate a completed input response with previously checked questions.
//! No model invocation, fresh discovery, target input or memory operation.
use super::*;
use reversebolt_core::{activity_experiment as a, runtime_hypotheses as h, visual};
use std::{fs, io::Read};

fn prepare(
    engine: &Engine,
    plan: &a::Plan,
    reference: &str,
    base: &Path,
) -> Result<(Value, Option<runtime::View>)> {
    let Some(binding) = &plan.metadata else {
        return Ok((
            json!({"status":"skipped","reason":"No completed metadata questions were available at submission."}),
            None,
        ));
    };
    let live = &engine.live.as_ref().ok_or("Application disconnected")?.view;
    plan.check_completed(live)?;
    if live
        .input_experiments
        .last()
        .is_none_or(|r| r.experiment != reference)
    {
        return Err("The latest input review changed before metadata handoff".into());
    }
    let original = binding.resolve(&live.runtime)?;
    let report = crate::runtime_bridge::catalog_report(&live.runtime)?;
    let (context, reuse) = match activity::latest(engine, base) {
        Ok(Some(retained)) => (Some(retained.context), retained.provenance),
        Ok(None) => (
            None,
            json!({"reused":false,"new_image_model_calls":0,
            "error":"The latest input experiment has no completed image interpretation."}),
        ),
        Err(error) => (
            None,
            json!({"reused":false,"new_image_model_calls":0,"error":error}),
        ),
    };
    let mut review = original.clone();
    review.question_origin = Some(
        original
            .question_origin
            .clone()
            .unwrap_or(h::QuestionOrigin {
                review_id: original.evidence_id.clone(),
                model: original.model.clone(),
            }),
    );
    review.evidence_id = uuid::Uuid::new_v4().to_string();
    review.play_context = context;
    let mut staged = live.runtime.clone();
    // Revalidates the same questions against the unchanged report and retains
    // selected/refreshed/edited controls through the normal publication path.
    let publication = publish(
        &mut staged,
        &binding.catalog_id,
        review.clone(),
        false,
        Some(&report),
        &protected_cards(engine),
    )?;
    Ok((
        json!({"status":if review.play_context.is_some(){"associated"}else{"activity_unavailable"},
        "previous_review":original,"review":review,"activity_reuse":reuse,
        "card_publication":publication}),
        Some(staged),
    ))
}

pub(crate) fn finish(
    engine: &mut Engine,
    plan: &a::Plan,
    reference: &str,
    owner: &OperationOwner,
    base: &Path,
) -> Result<Value> {
    let live = &engine.live.as_ref().ok_or("Application disconnected")?.view;
    let summary = live
        .input_experiments
        .iter()
        .find(|r| r.experiment == reference)
        .ok_or("Input review was not retained")?
        .clone();
    let directory = crate::activity_experiment::location(base, reference)?
        .join("review")
        .join("metadata");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    // A lost checkpoint must not create a second derived review on retry.
    save_new(
        &directory.join("intent.json"),
        &json!({"experiment":reference,"job":owner.job,
        "input_receipt_sha256":summary.receipt_sha256,"metadata":plan.metadata}),
    )?;
    let (mut result, mut staged) = if owner.cancel.load(Ordering::SeqCst) {
        (json!({"status":"cancelled"}), None)
    } else {
        prepare(engine, plan, reference, base)
            .unwrap_or_else(|error| (json!({"status":"unavailable","error":error}), None))
    };
    if owner.cancel.load(Ordering::SeqCst) {
        result = json!({"status":"cancelled"});
        staged = None;
    }
    result["experiment"] = json!(reference);
    result["input_receipt_sha256"] = json!(summary.receipt_sha256);
    result["new_model_calls"] = json!(0);
    result["limitation"] = json!(
        "Existing questions and newly observed response remain independent evidence. Association enables investigation, not semantic confirmation or edit authority. No values were refreshed."
    );
    let path = directory.join("result.json");
    save_new(&path, &result)?;
    let digest = visual::digest(&fs::read(&path).map_err(|e| e.to_string())?);
    let live = &mut engine.live.as_mut().ok_or("Application disconnected")?.view;
    if let Some(staged) = staged {
        live.runtime = staged;
    }
    live.input_experiments
        .iter_mut()
        .find(|r| r.experiment == reference)
        .ok_or("Input review changed during handoff")?
        .metadata_handoff_sha256 = Some(digest);
    Ok(result)
}

pub(crate) fn read(base: &Path, summary: &a::Review) -> Result<Value> {
    let Some(expected) = &summary.metadata_handoff_sha256 else {
        return Ok(Value::Null);
    };
    let path = crate::activity_experiment::location(base, &summary.experiment)?
        .join("review/metadata/result.json");
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(12 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 12 * 1024 * 1024 || visual::digest(&bytes) != *expected {
        return Err("Stored metadata handoff checksum changed".into());
    }
    let result: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if result["experiment"] != summary.experiment
        || result["input_receipt_sha256"] != summary.receipt_sha256
    {
        return Err("Stored metadata handoff belongs to another input review".into());
    }
    Ok(result)
}
