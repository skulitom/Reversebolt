//! Optional metadata review owned by the existing discovery job.
use crate::service::OperationOwner;
use reversebolt_core::{
    Result,
    engine::{Engine, View},
    runtime,
    runtime_hypotheses::Review,
};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::Path,
    sync::{Arc, RwLock, atomic::Ordering},
};

fn save_new(path: &Path, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("Metadata review exceeds its archive budget.".into());
    }
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(&bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| e.to_string())
}
fn read_archive(path: &Path) -> Result<Value> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(12 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("Metadata archive exceeds its budget.".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

/// Agent-supplied proposals use the same inventory and validation as local inference.
pub fn command(engine: &mut Engine, c: &reversebolt_core::Command, base: &Path) -> Result<Value> {
    if c.action == "plan_runtime_comparisons" {
        return plan_comparisons(engine, c);
    }
    use reversebolt_core::{runtime_hypotheses as h, runtime_inventory as inventory};
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Submission {
        review_id: String,
        catalog_id: String,
        after_revision: u64,
        after_review: Option<String>,
        report_sha256: String,
        selected_types: Vec<usize>,
        hypotheses: Vec<h::Proposal>,
    }
    let submission = if c.action == "submit_runtime_hypotheses" {
        let p: Submission = serde_json::from_value(c.args.clone()).map_err(|e| e.to_string())?;
        if !uuid::Uuid::parse_str(&p.review_id).is_ok_and(|id| id.to_string() == p.review_id) {
            return Err("Supply a new canonical review UUID.".into());
        }
        Some(p)
    } else {
        None
    };
    let request = json!({"actor":c.source,"args":c.args});
    // A replay returns its archived outcome and never republishes an older review.
    if let Some(p) = &submission {
        let dir = base.join("naming").join(&p.review_id);
        if dir.exists() {
            let saved = read_archive(&dir.join("agent-request.json"))?;
            if saved != request {
                return Err("This review identity belongs to a different submission.".into());
            }
            let receipt = read_archive(&dir.join("receipt.json"))?;
            return Ok(json!({"review":receipt["review"],"replayed":true,"published":false}));
        }
    }
    let view = &mut engine
        .live
        .as_mut()
        .ok_or("Connect an application first.")?
        .view
        .runtime;
    if c.args["catalog_id"] != view.catalog_id || view.catalog_id.is_empty() {
        return Err("The runtime catalogue changed.".into());
    }
    let report = crate::runtime_bridge::catalog_report(view)?;
    let inventory = inventory::build(&report)?;
    let Some(p) = submission else {
        let selection = c
            .args
            .get("selected_types")
            .filter(|v| !v.is_null())
            .map(|v| serde_json::from_value::<Vec<usize>>(v.clone()).map_err(|e| e.to_string()))
            .transpose()?
            .map(|ids| h::select(&inventory, &ids))
            .transpose()?;
        let index:Vec<_> = inventory.types.iter().map(|t|json!({"id":t.id,"name":t.key.type_name,
            "fields":t.fields.iter().map(|f|json!({"id":f.field_id,"name":f.name,"type_code":f.type_code,"paths":f.path_count})).collect::<Vec<_>>()})).collect();
        return Ok(
            json!({"catalog_id":view.catalog_id,"revision":view.revision,"report_sha256":inventory.report_sha256,
            "types":if selection.is_none(){json!(index)}else{Value::Null},"selection":selection,"limitation":h::LIMITATION}),
        );
    };
    if p.catalog_id != view.catalog_id
        || p.after_revision != view.revision
        || p.after_review.as_deref() != view.naming.as_ref().map(|n| n.evidence_id.as_str())
        || p.report_sha256 != inventory.report_sha256
    {
        return Err(
            "The catalogue, readings or hypothesis review changed. Read current status first."
                .into(),
        );
    }
    let selection = h::select(&inventory, &p.selected_types)?;
    let batch = h::validate(
        &selection,
        h::Proposals {
            hypotheses: p.hypotheses,
        },
    )?;
    let review = Review {
        play_context: None,
        evidence_id: p.review_id.clone(),
        report_sha256: inventory.report_sha256.clone(),
        model: format!("agent:{}", c.source),
        status: "complete".into(),
        batch: Some(batch),
        error: None,
    };
    let dir = base.join("naming").join(&p.review_id);
    std::fs::create_dir_all(base.join("naming")).map_err(|e| e.to_string())?;
    std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
    save_new(&dir.join("agent-request.json"), &request)?;
    save_new(&dir.join("inventory.json"), &json!(inventory))?;
    save_new(
        &dir.join("receipt.json"),
        &json!({"schema":1,"selection":selection,"review":review,"limitation":h::LIMITATION}),
    )?;
    publish(view, &p.catalog_id, review.clone(), false)?;
    Ok(json!({"review":review,"replayed":false,"published":true}))
}

fn plan_comparisons(engine: &Engine, c: &reversebolt_core::Command) -> Result<Value> {
    use reversebolt_core::runtime_planning as p;
    let session = engine
        .live
        .as_ref()
        .ok_or("Connect an application first.")?;
    let rt = &session.view.runtime;
    let review = rt
        .naming
        .as_ref()
        .ok_or("Wait for a completed activity-informed review.")?;
    if c.args["catalog_id"] != rt.catalog_id || c.args["review_id"] != review.evidence_id {
        return Err("The catalogue or hypothesis review changed.".into());
    }
    let context = review
        .play_context
        .as_ref()
        .ok_or("No consistent observed activity is available for a comparison plan.")?;
    let response = crate::play_evidence::read(
        session,
        &json!({"investigation_id":session.view.investigation_id,"batch":session.view.coverage.batch,"frame_ids":session.view.frames.iter().map(|f|&f.id).collect::<Vec<_>>()}),
    )?;
    let evidence =
        serde_json::from_value(response["evidence"].clone()).map_err(|e| e.to_string())?;
    let actions = p::actions(context, &session.view.frames, &evidence)?;
    let inventory =
        reversebolt_core::runtime_inventory::build(&crate::runtime_bridge::catalog_report(rt)?)?;
    let choices = p::choices(
        review,
        &inventory,
        &actions,
        &engine.runtime_tests.records,
        &session.view.investigation_id,
        &rt.catalog_id,
    )?;
    let busy = engine.runtime_trials.holds(&inventory.scope)
        || engine.runtime_tests.records.iter().any(|t| t.active())
        || engine.view.freezes.iter().any(|f| f.active());
    let next = if busy {
        None
    } else {
        choices.iter().find(|v| v["status"] == "untried")
    };
    Ok(
        json!({"investigation_id":session.view.investigation_id,"catalog_id":rt.catalog_id,"after_revision":rt.revision,"review_id":review.evidence_id,
        "report_sha256":review.report_sha256,"observed_actions":actions,"choices":choices,"next":next,"busy":busy,"limitation":p::LIMITATION}),
    )
}

pub(crate) fn publish(
    view: &mut runtime::View,
    catalog: &str,
    mut review: Review,
    cancelled: bool,
) -> Result<()> {
    if view.catalog_id != catalog
        || view
            .catalog_source
            .as_ref()
            .is_none_or(|s| s.document_sha256 != review.report_sha256)
    {
        return Err("Metadata hypotheses belong to a replaced catalogue.".into());
    }
    if cancelled {
        review.status = "cancelled".into();
        review.batch = None;
        review.play_context = None;
    } else if let Some(batch) = &review.batch
        && batch.report_sha256 != review.report_sha256
    {
        return Err("Hypothesis report binding changed.".into());
    }
    if let Some(context) = &review.play_context {
        context.validate()?;
    }
    view.naming = Some(review);
    Ok(())
}

pub fn review(
    engine: &mut Engine,
    model: &str,
    owner: &OperationOwner,
    base: &Path,
    published: &Arc<RwLock<View>>,
) -> Result<Value> {
    let result = review_inner(engine, model, owner, base, published);
    if let Err(error) = &result {
        if let Some(review) = engine
            .live
            .as_mut()
            .and_then(|s| s.view.runtime.naming.as_mut())
            && review.status == "pending"
        {
            review.status = "failed".into();
            review.batch = None;
            review.error = Some(error.clone());
        }
        // Keep the failed archive/checkpoint visible; never publish partial hypotheses.
        let _ = crate::observation_pipeline::checkpoint(engine, published);
    }
    result
}

fn review_inner(
    engine: &mut Engine,
    model: &str,
    owner: &OperationOwner,
    base: &Path,
    published: &Arc<RwLock<View>>,
) -> Result<Value> {
    if owner.cancel.load(Ordering::SeqCst) {
        return Ok(json!({"cancelled":true}));
    }
    let session = engine
        .live
        .as_ref()
        .ok_or("The application disconnected.")?;
    let source = session
        .view
        .runtime
        .catalog_source
        .clone()
        .ok_or("No discovery report is available.")?;
    let catalog = session.view.runtime.catalog_id.clone();
    let report = crate::runtime_bridge::catalog_report(&session.view.runtime)?;
    let id = uuid::Uuid::new_v4().to_string();
    let directory = base.join("naming").join(&id);
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    save_new(
        &directory.join("intent.json"),
        &json!({"schema":1,"catalog_id":catalog,"report_sha256":source.document_sha256,
        "source":source.path,"job":owner.job,"model":model,"scope":{"pid":report["pid"],"created":report["created"],"session":report["session"]},"status":"pending"}),
    )?;
    let mut state = Review {
        play_context: None,
        evidence_id: id,
        report_sha256: source.document_sha256,
        model: model.into(),
        status: "pending".into(),
        batch: None,
        error: None,
    };
    publish(
        &mut engine.live.as_mut().unwrap().view.runtime,
        &catalog,
        state.clone(),
        false,
    )?;
    crate::observation_pipeline::checkpoint(engine, published)?;
    let inference = (|| -> Result<Value> {
        let inventory = reversebolt_core::runtime_inventory::build(&report)?;
        save_new(&directory.join("inventory.json"), &json!(inventory))?;
        let activity = prepare_activity(engine.live.as_ref().unwrap(), &directory)?;
        if owner.cancel.load(Ordering::SeqCst) {
            return Ok(json!({"cancelled":true}));
        }
        let receipt = tokio::runtime::Runtime::new()
            .map_err(|e| e.to_string())?
            .block_on(async {
                let observer = reversebolt_observer::Observer::connect(model).await?;
                Ok::<_, String>(
                    observer
                        .runtime_hypotheses_with_activity(
                            &inventory,
                            activity.as_ref(),
                            &owner.cancel,
                        )
                        .await,
                )
            })?;
        state.play_context = receipt.activity.as_ref().and_then(|a| a.context());
        state.batch = receipt.batch.clone();
        state.error = receipt.error.clone();
        Ok(json!(receipt))
    })();
    let result = match inference {
        Ok(value) => value,
        Err(error) => {
            state.error = Some(error.clone());
            json!({"error":error})
        }
    };
    // Persist the outcome before any hypothesis becomes visible, including failures.
    save_new(&directory.join("receipt.json"), &result)?;
    let session = engine.live.as_mut().unwrap();
    session.refresh_connection();
    let cancelled = owner.cancel.load(Ordering::SeqCst) || !session.view.connected;
    if !cancelled {
        crate::runtime_bridge::catalog_report(&session.view.runtime)?;
    }
    state.status = if state.error.is_some() {
        "unavailable"
    } else {
        "complete"
    }
    .into();
    if state.error.is_some() {
        state.batch = None;
        state.play_context = None;
    }
    let count = state.batch.as_ref().map_or(0, |b| b.accepted.len());
    let evidence_id = state.evidence_id.clone();
    publish(&mut session.view.runtime, &catalog, state, cancelled)?;
    engine.event("local-model",format!("Metadata review {evidence_id} retained {} provisional questions; cancelled: {cancelled}. No card meaning or edit authority changed.",if cancelled{0}else{count}));
    crate::observation_pipeline::checkpoint(engine, published)?;
    Ok(
        json!({"evidence_id":evidence_id,"hypotheses":if cancelled{0}else{count},"cancelled":cancelled}),
    )
}

fn prepare_activity(
    session: &reversebolt_core::live::LiveSession,
    directory: &Path,
) -> Result<Option<reversebolt_observer::runtime_activity::Input>> {
    use reversebolt_core::{play_context, play_evidence};
    let ids: Vec<_> = session.view.frames.iter().map(|f| f.id.clone()).collect();
    let args = json!({"investigation_id":session.view.investigation_id,"batch":session.view.coverage.batch,"frame_ids":ids});
    let prepared = (|| -> Result<_> {
        let evidence = crate::play_evidence::read(session, &args)?;
        let parsed: play_evidence::Evidence =
            serde_json::from_value(evidence["evidence"].clone()).map_err(|e| e.to_string())?;
        let source = play_context::select(
            &session.view.frames,
            &parsed,
            &session.view.investigation_id,
            session.view.coverage.batch,
        )?;
        Ok((evidence, source))
    })();
    let (evidence, source) = match prepared {
        Ok(value) => value,
        Err(error) => {
            save_new(
                &directory.join("play-source.json"),
                &json!({"unavailable":error,"request":args}),
            )?;
            return Ok(None);
        }
    };
    save_new(
        &directory.join("play-source.json"),
        &json!({"request":args,"frames":session.view.frames,"evidence":evidence,"selected":source}),
    )?;
    let Some(source) = source else {
        return Ok(None);
    };
    let mut frames = vec![];
    let mut size = 0;
    for (index, reference) in source.frames().into_iter().enumerate() {
        let (_, bytes) = session.read_frame(&source.investigation_id, &reference.id)?;
        size += bytes.len();
        if size > 16 * 1024 * 1024 {
            return Err("Activity images exceed the archive budget.".into());
        }
        let frame = reversebolt_observer::Frame::from_png(
            bytes.clone(),
            reference
                .interval
                .begin_us
                .checked_mul(1000)
                .ok_or("Capture clock overflow")?,
        )?;
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(directory.join(format!("play-{index}.png")))
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        frames.push(frame);
    }
    let input = reversebolt_observer::runtime_activity::Input { source, frames };
    input.validate()?;
    Ok(Some(input))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view() -> runtime::View {
        runtime::View {
            catalog_id: "current".into(),
            catalog_source: Some(runtime::CatalogSource {
                path: "unused".into(),
                document_sha256: "a".repeat(64),
                observed_unix_ms: 0,
            }),
            ..Default::default()
        }
    }
    fn review() -> Review {
        Review {
            play_context: None,
            evidence_id: "review".into(),
            report_sha256: "a".repeat(64),
            model: "local".into(),
            status: "complete".into(),
            error: None,
            batch: Some(reversebolt_core::runtime_hypotheses::Batch {
                report_sha256: "a".repeat(64),
                selected_types: vec![],
                considered_samples: 0,
                omitted_samples: 0,
                accepted: vec![],
                rejected: vec![],
                limitation: "Unverified".into(),
            }),
        }
    }
    #[test]
    fn cancelled_or_rebound_reviews_never_publish_hypotheses() {
        let mut v = view();
        publish(&mut v, "current", review(), true).unwrap();
        assert_eq!(v.naming.as_ref().unwrap().status, "cancelled");
        assert!(v.naming.as_ref().unwrap().batch.is_none());
        assert!(publish(&mut v, "old", review(), false).is_err());
        let mut r = review();
        r.batch.as_mut().unwrap().report_sha256 = "b".repeat(64);
        assert!(publish(&mut v, "current", r, false).is_err());
        assert!(v.naming.as_ref().unwrap().batch.is_none());
        assert!(v.cards.is_empty());
        assert_eq!(v.revision, 0);
    }
}
