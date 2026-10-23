use super::*;
use crate::runtime_naming_bridge::{self as naming, activity as handoff};
use reversebolt_core::{runtime, runtime_hypotheses as h};

fn metadata_receipt(error: Option<String>) -> reversebolt_observer::runtime_naming::Receipt {
    reversebolt_observer::runtime_naming::Receipt {
        selection_purpose: Default::default(),
        incomplete_stage: None,
        paged_selection: None,
        object_text_fallback: None,
        activity: None,
        schema: 1,
        report_sha256: "a".repeat(64),
        model: "new-metadata-model".into(),
        model_digest: "b".repeat(64),
        server_version: "fixture".into(),
        stages: vec![],
        selection: None,
        batch: None,
        error,
        cancelled: false,
        limitation: h::LIMITATION.into(),
    }
}
pub(super) fn check(engine: &mut Engine, root: &Path, saved: &Value) {
    let base = root.join("runtime");
    let directory = root.join("fresh-invalid-context");
    fs::create_dir(&directory).unwrap();
    let mut fresh = handoff::prepare(engine, &base, &directory).unwrap();
    let previous = fresh.retained.take().unwrap().receipt;
    fresh.frames.pop();
    let outcome = fresh
        .infer(
            &uuid::Uuid::new_v4().to_string(),
            "new-image-model",
            &AtomicBool::new(false),
            |images| {
                assert!(images.is_some());
                let mut receipt = metadata_receipt(None);
                receipt.activity = Some(previous);
                Ok(receipt)
            },
        )
        .unwrap();
    assert!(outcome.context.is_none());
    assert!(outcome.receipt.error.is_none());
    assert!(outcome.reuse["error"].is_string());
    for behavior in ["success", "cancel", "metadata_failure"] {
        let directory = root.join(format!("reuse-{behavior}"));
        fs::create_dir(&directory).unwrap();
        let prepared = handoff::prepare(engine, &base, &directory).unwrap();
        assert!(prepared.retained.is_some());
        assert_eq!(prepared.frames.len(), 10);
        let cancel = AtomicBool::new(false);
        let mut metadata_calls = 0;
        let outcome = prepared
            .infer(
                &uuid::Uuid::new_v4().to_string(),
                "new-metadata-model",
                &cancel,
                |images| {
                    metadata_calls += 1;
                    assert!(
                        images.is_none(),
                        "A retained interpretation must not trigger another image-model call"
                    );
                    if behavior == "cancel" {
                        cancel.store(true, Ordering::SeqCst);
                    }
                    Ok(metadata_receipt(
                        (behavior == "metadata_failure").then(|| "metadata unavailable".into()),
                    ))
                },
            )
            .unwrap();
        assert_eq!(metadata_calls, 1);
        if behavior == "success" {
            assert_eq!(json!(outcome.context), saved["review"]["context"]);
            assert_eq!(outcome.reuse["new_image_model_calls"], 0);
            assert_eq!(
                outcome.reuse["receipt_sha256"],
                saved["review"]["receipt_sha256"]
            );
            assert_eq!(outcome.receipt.model, "new-metadata-model");
            assert_eq!(
                outcome
                    .receipt
                    .activity
                    .unwrap()
                    .observation
                    .unwrap()
                    .consistent_visible_change,
                saved["review"]["context"]["observation"]["consistent_visible_change"]
                    .as_bool()
                    .unwrap()
            );
        } else {
            assert!(outcome.context.is_none());
            assert!(outcome.receipt.activity.is_none());
        }
    }
    let original = engine.live.as_ref().unwrap().view.input_experiments[0].clone();
    engine.live.as_mut().unwrap().view.input_experiments[0]
        .context
        .as_mut()
        .unwrap()
        .observation
        .description = "Changed after publication".into();
    assert!(handoff::latest(engine, &base).is_err());
    let directory = root.join("reuse-invalid");
    fs::create_dir(&directory).unwrap();
    let prepared = handoff::prepare(engine, &base, &directory).unwrap();
    assert!(prepared.reuse_error.is_some());
    let outcome = prepared
        .infer(
            &uuid::Uuid::new_v4().to_string(),
            "metadata-model",
            &AtomicBool::new(false),
            |images| {
                assert!(images.is_none());
                Ok(metadata_receipt(None))
            },
        )
        .unwrap();
    assert!(outcome.receipt.error.is_none());
    assert!(outcome.context.is_none());
    assert!(outcome.reuse["error"].is_string());
    engine.live.as_mut().unwrap().view.input_experiments[0] = original.clone();
    let original_frame = engine.live.as_ref().unwrap().view.frames[2].clone();
    engine.live.as_mut().unwrap().view.frames[2].input_context = None;
    assert!(handoff::latest(engine, &base).is_err());
    engine.live.as_mut().unwrap().view.frames[2] = original_frame;
    engine.live.as_mut().unwrap().view.input_experiments[0].status = "cancelled".into();
    assert!(handoff::latest(engine, &base).unwrap().is_none());
    engine.live.as_mut().unwrap().view.input_experiments[0] = original;

    // The real agent submission and comparison-planning commands must retain
    // the ten-frame context despite the preceding passive capture in the session.
    let info = &engine.live.as_ref().unwrap().view.process;
    let key = |name| json!({"image":"Fixture.dll","type_name":name,"layout_sha256":"a".repeat(64)});
    let report = json!({"schema":4,"pid":info.pid,"created":info.created.to_ascii_lowercase(),"session":info.session,"edit_authority":false,
        "classes":[{"id":0,"key":key("Roots"),"shape":"object","fields":[{"id":0,"name":"Current","reference":true,"type_code":18,"width":8,"flags":16}]},
        {"id":1,"key":key("Actor"),"shape":"object","fields":[{"id":0,"name":"Speed","reference":false,"type_code":8,"width":4,"flags":0}]}],
        "snapshots":[{"snapshot_id":0,"nodes":[{"id":0,"class":1,"address":"0x1000"}],"roots":[{"class":0,"field":0,"read_ok":true,"node":{"id":0}}],
        "reads":[{"node":0,"declaring_class":1,"field":0,"element_index":null,"type_code":8,"bytes":8i32.to_le_bytes(),"value":8,"reference":null,"read_ok":true}]}]});
    let report_path = root.join("report.json");
    naming::save_new(&report_path, &report).unwrap();
    let inventory = reversebolt_core::runtime_inventory::build(&report).unwrap();
    let selected_types: Vec<_> = inventory.types.iter().map(|t| t.id).collect();
    let selection = h::select(&inventory, &selected_types).unwrap();
    let proposal = h::Proposal {
        candidate: selection.rows[0].id,
        name: "Speed".into(),
        explanation: "The label suggests speed; the gameplay effect is unverified.".into(),
        citations: vec![h::Citation {
            source: "field_name".into(),
            index: None,
            quote: "Speed".into(),
        }],
        alternatives: vec![],
        question: "Does this value affect the observed response?".into(),
    };
    let catalog = uuid::Uuid::new_v4().to_string();
    engine.live.as_mut().unwrap().view.runtime = runtime::View {
        catalog_id: catalog.clone(),
        revision: 4,
        catalog_source: Some(runtime::CatalogSource {
            path: report_path.to_string_lossy().into(),
            document_sha256: inventory.report_sha256.clone(),
            observed_unix_ms: 0,
        }),
        ..Default::default()
    };
    let review_id = uuid::Uuid::new_v4().to_string();
    let command = reversebolt_core::Command {
        action: "submit_runtime_hypotheses".into(),
        source: "fixture-agent".into(),
        args: json!({"review_id":review_id,"catalog_id":catalog,"after_revision":4,"after_review":null,
            "report_sha256":inventory.report_sha256,"selected_types":selected_types,"hypotheses":[proposal]}),
    };
    let result = naming::command(engine, &command, &base).unwrap();
    assert_eq!(result["review"]["play_context"], saved["review"]["context"]);
    assert_eq!(result["activity_reuse"]["new_image_model_calls"], 0);
    assert_eq!(result["activity_transfer"]["retained"], true);
    let context: reversebolt_core::play_context::Context =
        serde_json::from_value(saved["review"]["context"].clone()).unwrap();
    let evidence: reversebolt_core::play_evidence::Evidence =
        serde_json::from_value(saved["result"]["measurement"]["evidence"]["evidence"].clone())
            .unwrap();
    assert!(
        reversebolt_core::runtime_planning::actions(
            &context,
            &engine.live.as_ref().unwrap().view.frames,
            &evidence
        )
        .is_err(),
        "The previous all-session-frames path must reproduce the scope mismatch"
    );
    let plan = naming::command(
        engine,
        &reversebolt_core::Command {
            action: "plan_runtime_comparisons".into(),
            source: "fixture-agent".into(),
            args: json!({"catalog_id":catalog,"review_id":review_id}),
        },
        &base,
    )
    .unwrap();
    assert_eq!(
        plan["observed_actions"]["actions"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(plan["choices"].as_array().unwrap().len(), 1);
    assert!(plan["next"].is_object());
    assert_eq!(plan["activity_context"], saved["review"]["context"]);
    assert!(
        !engine.live.as_ref().unwrap().view.runtime.cards[0]
            .candidate
            .edit_authority
    );
    assert_eq!(engine.live.as_ref().unwrap().view.sample_count, 11);
    assert_eq!(engine.live.as_ref().unwrap().view.memory_sample_count(), 0);
    assert_eq!(
        naming::command(engine, &command, &base).unwrap()["activity_reuse"],
        result["activity_reuse"]
    );
    let original = engine.live.as_ref().unwrap().view.input_experiments[0].clone();
    engine.live.as_mut().unwrap().view.input_experiments[0]
        .context
        .as_mut()
        .unwrap()
        .observation
        .description = "Changed after publication".into();
    let mut next = command.clone();
    next.args["review_id"] = json!(uuid::Uuid::new_v4().to_string());
    next.args["after_review"] = json!(review_id);
    next.args["after_revision"] = json!(engine.live.as_ref().unwrap().view.runtime.revision);
    let submitted = naming::command(engine, &next, &base).unwrap();
    assert_eq!(submitted["review"]["status"], "complete");
    assert_eq!(submitted["activity_transfer"]["retained"], false);
    assert!(submitted["activity_reuse"]["error"].is_string());
    assert!(submitted["review"]["play_context"].is_null());
    engine.live.as_mut().unwrap().view.input_experiments[0] = original;
}
