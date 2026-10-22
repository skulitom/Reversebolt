//! Optional metadata review owned by the existing discovery job.
pub(crate) mod activity;
#[cfg(test)]
#[path = "runtime_naming_handoff_tests.rs"]
mod handoff_tests;
pub(crate) mod link_activity;
use crate::service::OperationOwner;
use reversebolt_core::{
    Result,
    engine::{Engine, View},
    runtime,
    runtime_hypotheses::Review,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::Write,
    path::Path,
    sync::{Arc, RwLock, atomic::Ordering},
};

fn protected_cards(engine: &Engine) -> BTreeSet<String> {
    let Some(live) = &engine.live else {
        return BTreeSet::new();
    };
    let catalog = &live.view.runtime.catalog_id;
    let investigation = &live.view.investigation_id;
    engine
        .runtime_trials
        .records
        .iter()
        .filter(|t| &t.context.catalog_id == catalog && &t.context.investigation == investigation)
        .map(|t| t.candidate_id.clone())
        .chain(
            engine
                .view
                .freezes
                .iter()
                .filter(|f| {
                    &f.catalog == catalog
                        && &f.investigation == investigation
                        && (f.active() || f.phase == "uncertain")
                })
                .map(|f| f.candidate.clone()),
        )
        .chain(
            engine
                .runtime_tests
                .records
                .iter()
                .filter(|t| t.active())
                .map(|t| t.candidate.id.clone()),
        )
        .collect()
}

pub(crate) fn save_new(path: &Path, value: &Value) -> Result<()> {
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
pub(crate) fn read_archive(path: &Path) -> Result<Value> {
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

fn inventory_index(
    inventory: &reversebolt_core::runtime_inventory::Inventory,
) -> (Vec<Value>, Value) {
    // Agents need object-local clues before choosing a type, just like the local
    // observer. Preserve the existing complete field descriptors for API clients.
    let context = reversebolt_core::runtime_type_index::build(inventory, 110_000);
    let types = inventory.types.iter().map(|t| {
        let samples = context.as_ref().ok().and_then(|index| index.types.iter().find(|i| i.id == t.id));
        let mut entry = json!({"id":t.id,"name":t.key.type_name,
            "fields":t.fields.iter().map(|f|json!({"id":f.field_id,"name":f.name,"type_code":f.type_code,"paths":f.path_count})).collect::<Vec<_>>()});
        if let Some(samples) = samples.filter(|s| !s.object_samples.is_empty()) {
            entry["object_samples"] = json!(samples.object_samples);
        }
        entry
    }).collect();
    let coverage = match context {
        Ok(index) => {
            json!({"available":index.available_object_samples,"omitted":index.omitted_object_samples,"error":null})
        }
        Err(error) => json!({"available":null,"omitted":null,"error":error}),
    };
    (types, coverage)
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
            return Ok(
                json!({"review":receipt["review"],"activity_transfer":receipt["activity_transfer"],"activity_reuse":receipt["activity_reuse"],"activity_motion_comparison":receipt["activity_motion_comparison"],"card_publication":receipt["card_publication"],"replayed":true,"published":false}),
            );
        }
    }
    let (input_activity, input_activity_error) = if submission.is_some() {
        match activity::latest(engine, base) {
            Ok(Some(retained)) => (Some(retained), None),
            Ok(None)
                if engine
                    .live
                    .as_ref()
                    .is_some_and(|s| !s.view.input_experiments.is_empty()) =>
            {
                (
                    None,
                    Some(
                        "The latest input experiment has no completed interpretation to reuse"
                            .to_owned(),
                    ),
                )
            }
            Ok(None) => (None, None),
            Err(error) => (None, Some(error)),
        }
    } else {
        (None, None)
    };
    let protected = protected_cards(engine);
    let activity_reuse = input_activity
        .as_ref()
        .map(|r| r.provenance.clone())
        .or_else(|| {
            input_activity_error
                .as_ref()
                .map(|error| json!({"reused":false,"error":error,"new_image_model_calls":0}))
        });
    let live = &mut engine
        .live
        .as_mut()
        .ok_or("Connect an application first.")?
        .view;
    let view = &mut live.runtime;
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
        let (index, context_coverage) = if selection.is_none() {
            let (types, coverage) = inventory_index(&inventory);
            (json!(types), coverage)
        } else {
            (Value::Null, Value::Null)
        };
        return Ok(
            json!({"catalog_id":view.catalog_id,"revision":view.revision,"report_sha256":inventory.report_sha256,
            "types":index,"context_coverage":context_coverage,"selection":selection,"limitation":h::LIMITATION}),
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
    let (play_context, activity_transfer) = if let Some(retained) = input_activity {
        let transfer = h::ActivityTransfer {
            retained: true,
            from_review_id: retained
                .context
                .interpretation_source
                .as_ref()
                .map(|s| s.review_id.clone()),
            reason: format!(
                "Retained checked input experiment {} and its original interpretation; this does not validate these hypotheses.",
                retained.provenance["experiment"].as_str().unwrap_or("")
            ),
        };
        (Some(retained.context), transfer)
    } else if let Some(error) = input_activity_error {
        (
            None,
            h::ActivityTransfer {
                retained: false,
                from_review_id: None,
                reason: error,
            },
        )
    } else {
        h::transfer_activity(
            view.naming.as_ref(),
            &inventory.report_sha256,
            &live.investigation_id,
            live.coverage.batch,
            &live.frames,
        )
    };
    let review = Review {
        question_source: Default::default(),
        question_origin: None,
        question_policy: None,
        play_context,
        evidence_id: p.review_id.clone(),
        report_sha256: inventory.report_sha256.clone(),
        model: format!("agent:{}", c.source),
        status: "complete".into(),
        batch: Some(batch),
        error: None,
    };
    let motion_comparison = review
        .play_context
        .as_ref()
        .map(|context| context.motion_comparison())
        .transpose()?
        .flatten();
    let dir = base.join("naming").join(&p.review_id);
    let mut staged = view.clone();
    let card_publication = publish(
        &mut staged,
        &p.catalog_id,
        review.clone(),
        false,
        Some(&report),
        &protected,
    )?;
    std::fs::create_dir_all(base.join("naming")).map_err(|e| e.to_string())?;
    std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
    save_new(&dir.join("agent-request.json"), &request)?;
    save_new(&dir.join("inventory.json"), &json!(inventory))?;
    save_new(
        &dir.join("receipt.json"),
        &json!({"schema":1,"selection":selection,"review":review,"activity_transfer":activity_transfer,"activity_reuse":activity_reuse,"activity_motion_comparison":motion_comparison,"card_publication":card_publication,"limitation":h::LIMITATION}),
    )?;
    *view = staged;
    Ok(
        json!({"review":review,"activity_transfer":activity_transfer,"activity_reuse":activity_reuse,"activity_motion_comparison":motion_comparison,"card_publication":card_publication,"replayed":false,"published":true}),
    )
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
        .ok_or("Wait for a completed metadata review with retained activity evidence.")?;
    if c.args["catalog_id"] != rt.catalog_id || c.args["review_id"] != review.evidence_id {
        return Err("The catalogue or hypothesis review changed.".into());
    }
    let context = review
        .play_context
        .as_ref()
        .ok_or("No bounded measured activity is available for a comparison plan.")?;
    let frames = context.resolve_frames(&session.view.frames)?;
    let response = crate::play_evidence::read(
        session,
        &json!({"investigation_id":session.view.investigation_id,"batch":session.view.coverage.batch,"frame_ids":frames.iter().map(|f|&f.id).collect::<Vec<_>>()}),
    )?;
    let evidence =
        serde_json::from_value(response["evidence"].clone()).map_err(|e| e.to_string())?;
    crate::play_evidence::check_activity_context(context, &response)?;
    let actions = p::actions(context, &frames, &evidence)?;
    let inventory =
        reversebolt_core::runtime_inventory::build(&crate::runtime_bridge::catalog_report(rt)?)?;
    let mut choices = p::choices(
        review,
        &inventory,
        &actions,
        &engine.runtime_tests.records,
        &session.view.investigation_id,
        &rt.catalog_id,
    )?;
    let regional_history = crate::runtime_test_bridge::comparison_history(engine, &mut choices)?;
    let busy = engine.runtime_trials.holds(&inventory.scope)
        || engine.runtime_tests.records.iter().any(|t| t.active())
        || engine.view.freezes.iter().any(|f| f.active());
    let record_capacity = engine
        .runtime_tests
        .capacity(&session.view.investigation_id);
    let next = if busy || record_capacity.remaining_record_slots == 0 {
        None
    } else {
        choices.iter().find(|v| v["status"] == "untried")
    };
    Ok(
        json!({"investigation_id":session.view.investigation_id,"catalog_id":rt.catalog_id,"after_revision":rt.revision,"review_id":review.evidence_id,
        "report_sha256":review.report_sha256,"activity_context":context,"activity_motion_comparison":context.motion_comparison()?,"activity_limitation":reversebolt_core::play_context::LIMITATION,
        "observed_actions":actions,"choices":choices,"next":next,"busy":busy,"record_capacity":record_capacity,"regional_history":regional_history,"limitation":p::LIMITATION}),
    )
}

pub(crate) fn publish(
    view: &mut runtime::View,
    catalog: &str,
    mut review: Review,
    cancelled: bool,
    report: Option<&Value>,
    protected: &BTreeSet<String>,
) -> Result<Option<reversebolt_core::runtime_cards::Publication>> {
    if let Some(origin) = &review.question_origin {
        origin.validate()?;
        if origin.model != review.model {
            return Err("Retained questions changed their original author".into());
        }
    }
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
    let cards = if review.status == "complete"
        && review.error.is_none()
        && review
            .batch
            .as_ref()
            .is_some_and(|b| !b.accepted.is_empty())
    {
        Some(reversebolt_core::runtime_cards::include(
            view,
            report.ok_or("Retained discovery is required to publish reviewed cards.")?,
            &review,
            protected,
        )?)
    } else {
        None
    };
    view.naming = Some(review);
    Ok(cards)
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
        question_source: Default::default(),
        question_origin: None,
        question_policy: None,
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
        None,
        &BTreeSet::new(),
    )?;
    crate::observation_pipeline::checkpoint(engine, published)?;
    let inference = (|| -> Result<Value> {
        let inventory = reversebolt_core::runtime_inventory::build(&report)?;
        save_new(&directory.join("inventory.json"), &json!(inventory))?;
        let prepared = activity::prepare(engine, base, &directory)?;
        if owner.cancel.load(Ordering::SeqCst) {
            return Ok(json!({"cancelled":true}));
        }
        let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        let outcome = prepared.infer(&state.evidence_id, model, &owner.cancel, |input| {
            runtime.block_on(async {
                let observer = reversebolt_observer::Observer::connect(model).await?;
                Ok::<_, String>(
                    observer
                        .runtime_hypotheses_with_activity(&inventory, input, &owner.cancel)
                        .await,
                )
            })
        })?;
        let receipt = &outcome.receipt;
        state.play_context = outcome.context;
        state.question_source = receipt.question_source();
        state.question_policy = receipt.question_policy().map(str::to_owned);
        state.batch = receipt.batch.clone();
        state.error = receipt.error.clone();
        let mut saved = json!(receipt);
        saved["activity_reuse"] = outcome.reuse;
        Ok(saved)
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
    let protected = protected_cards(engine);
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
    let mut staged = session.view.runtime.clone();
    let card_publication = publish(
        &mut staged,
        &catalog,
        state,
        cancelled,
        Some(&report),
        &protected,
    )?;
    save_new(
        &directory.join("card-publication.json"),
        &json!(card_publication),
    )?;
    session.view.runtime = staged;
    let shown = card_publication.as_ref().map_or(0, |p| p.shown.len());
    engine.event("local-model",format!("Metadata review {evidence_id} retained {} provisional questions and made {shown} primary paths available as tentative cards; cancelled: {cancelled}. No gameplay meaning was verified or edit authority granted.",if cancelled{0}else{count}));
    crate::observation_pipeline::checkpoint(engine, published)?;
    Ok(
        json!({"evidence_id":evidence_id,"hypotheses":if cancelled{0}else{count},"cancelled":cancelled,"card_publication":card_publication}),
    )
}

pub(crate) fn prepare_activity(
    session: &reversebolt_core::live::LiveSession,
    directory: &Path,
) -> Result<Option<reversebolt_observer::runtime_activity::Input>> {
    let ids: Vec<_> = session
        .view
        .input_experiments
        .iter()
        .rev()
        .find(|r| r.source.is_some() && r.status != "cancelled")
        .map(|r| r.frame_ids.clone())
        .unwrap_or_else(|| session.view.frames.iter().map(|f| f.id.clone()).collect());
    prepare_activity_frames(session, directory, &ids)
}

pub(crate) fn prepare_activity_frames(
    session: &reversebolt_core::live::LiveSession,
    directory: &Path,
    ids: &[String],
) -> Result<Option<reversebolt_observer::runtime_activity::Input>> {
    let args = json!({"investigation_id":session.view.investigation_id,"batch":session.view.coverage.batch,"frame_ids":ids});
    let prepared = (|| -> Result<_> {
        let evidence = crate::play_evidence::read(session, &args)?;
        let source: Option<reversebolt_core::play_context::Source> =
            serde_json::from_value(evidence["activity_source"].clone())
                .map_err(|e| e.to_string())?;
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
        &json!({"request":args,"frames":session.view.frames.iter().filter(|f| ids.contains(&f.id)).collect::<Vec<_>>(),"evidence":evidence,"selected":source}),
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
    let appearances =
        evidence["automatic_region_motion"]["appearance_sequences"]["activity"]["summary"].clone();
    let appearances = if appearances.is_null() {
        None
    } else {
        Some(
            serde_json::from_value::<reversebolt_core::appearance_activity::Summary>(appearances)
                .map_err(|e| e.to_string())?,
        )
    };
    let regions =
        evidence["automatic_region_motion"]["activity_region_suggestions"]["summary"].clone();
    let regions = if regions.is_null() {
        None
    } else {
        Some(
            serde_json::from_value::<reversebolt_core::activity_regions::Summary>(regions)
                .map_err(|e| e.to_string())?,
        )
    };
    let input = reversebolt_observer::runtime_activity::Input {
        source,
        frames,
        appearances,
        regions,
    };
    input.validate()?;
    Ok(Some(input))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_discovery_archives_region_evidence_without_an_activity_claim() {
        let (session, directory) = crate::play_evidence::tests::saved_observations();
        assert!(prepare_activity(&session, &directory).unwrap().is_none());
        let saved: Value =
            serde_json::from_slice(&std::fs::read(directory.join("play-source.json")).unwrap())
                .unwrap();
        assert!(saved["selected"].is_null());
        let automatic = &saved["evidence"]["automatic_region_motion"];
        assert_eq!(
            automatic["selection"]["regions"].as_array().unwrap().len(),
            1
        );
        assert_eq!(automatic["measurements"].as_array().unwrap().len(), 1);
        assert_eq!(
            automatic["selection"]["omitted_observations"]["no_following_image"],
            1
        );
        assert!(automatic["measurements"][0]["result"]["error"].is_null());
        assert_eq!(
            automatic["measurements"][0]["appearance_match"]["measurement"]["status"],
            "unsupported_size"
        );
        assert_eq!(
            automatic["appearance_sequences"]["report"]["series"][0]["stop"],
            "unresolved_appearance"
        );
        assert!(automatic["appearance_sequences"]["error"].is_null());
        drop(session);
        std::fs::remove_dir_all(directory).unwrap();
    }
    fn inventory() -> reversebolt_core::runtime_inventory::Inventory {
        use reversebolt_core::{
            runtime::{FieldRef, TextHint, TypeKey},
            runtime_inventory::*,
        };
        Inventory {
            schema: 1,
            report_sha256: "a".repeat(64),
            scope: runtime::Scope {
                pid: 1,
                created: "0000000000000001".into(),
                session: 1,
            },
            types: [9, 11]
                .into_iter()
                .map(|id| {
                    let key = TypeKey {
                        image: "Observed".into(),
                        type_name: format!("Container{id}"),
                        layout_sha256: "b".repeat(64),
                    };
                    ObservedType {
                        id,
                        key: key.clone(),
                        fields: [13, 27]
                            .into_iter()
                            .map(|field_id| ObservedField {
                                field_id,
                                name: format!("Coefficient{field_id}"),
                                type_code: 12,
                                values: vec![json!("3.5")],
                                path_count: 3,
                                samples: (0..3)
                                    .map(|sample| PathSample {
                                        path_id: format!("{id:02x}{field_id:02x}{sample:060x}"),
                                        value: json!("3.5"),
                                        context: vec![TextHint {
                                            field: FieldRef {
                                                declaring: key.clone(),
                                                field_id: 50,
                                            },
                                            text: if id == 9 { "Speed" } else { "Energy" }.into(),
                                            truncated: false,
                                        }],
                                    })
                                    .collect(),
                            })
                            .collect(),
                    }
                })
                .collect(),
            reachable_paths: 12,
            omitted_core_paths: 0,
            omitted_array_elements: 0,
            omitted_non_numeric_paths: 0,
            limitation: "fixture".into(),
        }
    }
    #[test]
    fn agent_index_exposes_object_clues_without_losing_field_descriptors() {
        let inventory = inventory();
        let (types, coverage) = inventory_index(&inventory);
        assert_eq!(coverage, json!({"available":12,"omitted":4,"error":null}));
        for (observed, t) in inventory.types.iter().zip(types) {
            assert_eq!(t["id"], observed.id);
            assert_eq!(t["name"], observed.key.type_name);
            assert_eq!(t["fields"].as_array().unwrap().len(), 2);
            assert_eq!(
                t["fields"][1],
                json!({"id":27,"name":"Coefficient27","type_code":12,"paths":3})
            );
            for sample in t["object_samples"].as_array().unwrap() {
                let source = observed
                    .fields
                    .iter()
                    .flat_map(|f| f.samples.iter())
                    .find(|s| sample["path_id"] == s.path_id)
                    .unwrap();
                assert_eq!(sample["object_text"], json!([source.context[0].text]));
            }
        }
    }
    #[test]
    fn an_oversized_context_index_does_not_erase_agent_field_metadata() {
        let mut inventory = inventory();
        let template = inventory.types[0].fields[0].clone();
        inventory.types[0].fields = (0..1000)
            .map(|id| {
                let mut f = template.clone();
                f.field_id = id;
                f.name = format!("field_{id}_{}", "x".repeat(128));
                f
            })
            .collect();
        let (types, coverage) = inventory_index(&inventory);
        assert_eq!(types[0]["fields"].as_array().unwrap().len(), 1000);
        assert_eq!(
            types[0]["fields"][999]["name"],
            inventory.types[0].fields[999].name
        );
        assert!(coverage["error"].as_str().is_some());
        assert!(coverage["available"].is_null() && coverage["omitted"].is_null());
        assert!(types[0].get("object_samples").is_none());
    }
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
            question_source: Default::default(),
            question_origin: None,
            question_policy: None,
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
        publish(&mut v, "current", review(), true, None, &BTreeSet::new()).unwrap();
        assert_eq!(v.naming.as_ref().unwrap().status, "cancelled");
        assert!(v.naming.as_ref().unwrap().batch.is_none());
        assert!(publish(&mut v, "old", review(), false, None, &BTreeSet::new()).is_err());
        let mut r = review();
        r.batch.as_mut().unwrap().report_sha256 = "b".repeat(64);
        assert!(publish(&mut v, "current", r, false, None, &BTreeSet::new()).is_err());
        assert!(v.naming.as_ref().unwrap().batch.is_none());
        assert!(v.cards.is_empty());
        assert_eq!(v.revision, 0);
    }
}
