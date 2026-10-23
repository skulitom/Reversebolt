use super::*;
use crate::runtime_naming_bridge as naming;
use reversebolt_core::{runtime, runtime_hypotheses as h};

fn install_questions(engine: &mut Engine, root: &Path) -> h::Review {
    let info = &engine.live.as_ref().unwrap().view.process;
    let key = |name| json!({"image":"Fixture.dll","type_name":name,"layout_sha256":"a".repeat(64)});
    let report = json!({"schema":4,"pid":info.pid,"created":info.created.to_ascii_lowercase(),"session":info.session,"edit_authority":false,
        "classes":[{"id":0,"key":key("Roots"),"shape":"object","fields":[{"id":0,"name":"Current","reference":true,"type_code":18,"width":8,"flags":16}]},
        {"id":1,"key":key("Actor"),"shape":"object","fields":[{"id":0,"name":"Speed","reference":false,"type_code":8,"width":4,"flags":0}]}],
        "snapshots":[{"snapshot_id":0,"nodes":[{"id":0,"class":1,"address":"0x1000"}],"roots":[{"class":0,"field":0,"read_ok":true,"node":{"id":0}}],
        "reads":[{"node":0,"declaring_class":1,"field":0,"element_index":null,"type_code":8,"bytes":8i32.to_le_bytes(),"value":8,"reference":null,"read_ok":true}]}]});
    let path = root.join("metadata.json");
    naming::save_new(&path, &report).unwrap();
    let inventory = reversebolt_core::runtime_inventory::build(&report).unwrap();
    let selected = h::select(
        &inventory,
        &inventory.types.iter().map(|t| t.id).collect::<Vec<_>>(),
    )
    .unwrap();
    let batch = h::validate(
        &selected,
        h::Proposals {
            hypotheses: vec![h::Proposal {
                candidate: selected.rows[0].id,
                name: "Speed".into(),
                explanation: "The field label suggests speed; the effect is unverified.".into(),
                citations: vec![h::Citation {
                    source: "field_name".into(),
                    index: None,
                    quote: "Speed".into(),
                }],
                alternatives: vec![],
                question: "Does this field affect the observed response?".into(),
            }],
        },
    )
    .unwrap();
    let original = h::Review {
        question_origin: None,
        question_policy: None,
        question_source: Default::default(),
        play_context: None,
        evidence_id: uuid::Uuid::new_v4().to_string(),
        report_sha256: inventory.report_sha256.clone(),
        model: "agent:original-author".into(),
        status: "complete".into(),
        batch: Some(batch),
        error: None,
    };
    let catalog = uuid::Uuid::new_v4().to_string();
    let live = &mut engine.live.as_mut().unwrap().view;
    live.runtime = runtime::View {
        catalog_id: catalog.clone(),
        catalog_source: Some(runtime::CatalogSource {
            path: path.to_string_lossy().into(),
            document_sha256: inventory.report_sha256,
            observed_unix_ms: 0,
        }),
        ..Default::default()
    };
    naming::publish(
        &mut live.runtime,
        &catalog,
        original.clone(),
        false,
        Some(&report),
        &Default::default(),
    )
    .unwrap();
    let selected_id = live.runtime.cards[0].candidate.id.clone();
    live.runtime.selected_cards.push(selected_id);
    live.runtime.cards[0].value = "999".into(); // Selection must not refresh this display from the old report.
    original
}

#[test]
fn automatic_input_handoff_preserves_questions_and_reaches_comparison_planning() {
    for behavior in [
        "positive",
        "negative",
        "model_failure",
        "cancel",
        "metadata_changed",
    ] {
        let (mut engine, state, root, owner, mut plan, _) = setup();
        let original = install_questions(&mut engine, &root);
        plan.observer_model = Some("fixture:image-model".into());
        plan.metadata =
            activity::metadata::Binding::capture(&engine.live.as_ref().unwrap().view.runtime)
                .unwrap();
        assert!(plan.metadata.is_some());
        if behavior == "positive" {
            let mut changed = engine.live.as_ref().unwrap().view.runtime.clone();
            changed
                .naming
                .as_mut()
                .unwrap()
                .batch
                .as_mut()
                .unwrap()
                .accepted[0]
                .proposal
                .question
                .push('!');
            assert!(plan.metadata.as_ref().unwrap().resolve(&changed).is_err());
            changed = engine.live.as_ref().unwrap().view.runtime.clone();
            changed.catalog_source.as_mut().unwrap().document_sha256 = "0".repeat(64);
            assert!(plan.metadata.as_ref().unwrap().resolve(&changed).is_err());
        }
        let base = root.join("runtime");
        let reference = store(&base, &plan).unwrap();
        for index in 0..activity::STEPS {
            let result = run_with(
                &mut engine,
                &Step {
                    activity: reference.clone(),
                    index,
                    key: plan.key,
                },
                &owner,
                &base,
                |_, _, i, _, _| execute(&state, i),
            )
            .unwrap();
            assert_eq!(result["success"], true, "{result}");
        }
        if behavior == "metadata_changed" {
            engine.live.as_mut().unwrap().view.runtime.revision += 1;
        }
        let step = Step {
            activity: reference.clone(),
            index: activity::STEPS,
            key: plan.key,
        };
        let mut calls = 0;
        let result =
            super::super::review::run_with(&mut engine, &step, &owner, &base, |_, input| {
                calls += 1;
                if behavior == "cancel" {
                    owner.cancel.store(true, Ordering::SeqCst);
                }
                if behavior == "model_failure" {
                    return Err("Image model unavailable".into());
                }
                Ok(reversebolt_observer::runtime_activity::Receipt {
                    source: input.source.clone(),
                    regions: input.regions.clone(),
                    appearances: input.appearances.clone(),
                    motion: None,
                    stage: None,
                    error: None,
                    observation: Some(reversebolt_core::play_context::Observation {
                        consistent_visible_change: behavior != "negative",
                        description: "Generated response; no game semantics established.".into(),
                        uncertainty: "Mock interpretation for integration testing.".into(),
                    }),
                })
            })
            .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(result["metadata_handoff"]["new_model_calls"], 0);
        let live = &engine.live.as_ref().unwrap().view;
        let derived = live.runtime.naming.as_ref().unwrap().clone();
        assert_eq!(derived.batch, original.batch);
        assert_eq!(derived.model, original.model);
        assert_eq!(live.runtime.cards.len(), 1);
        assert_eq!(live.runtime.cards[0].value, "999");
        assert!(!live.runtime.cards[0].candidate.edit_authority);
        assert_eq!(live.sample_count, 11);
        assert_eq!(live.memory_sample_count(), 0);
        if matches!(behavior, "positive" | "negative" | "model_failure") {
            assert_ne!(derived.evidence_id, original.evidence_id);
            assert_eq!(
                derived.question_origin.as_ref().unwrap().review_id,
                original.evidence_id
            );
            assert_eq!(
                result["metadata_handoff"]["status"],
                if behavior == "model_failure" {
                    "activity_unavailable"
                } else {
                    "associated"
                }
            );
            if behavior == "model_failure" {
                assert!(derived.play_context.is_none());
            } else {
                let context = derived.play_context.as_ref().unwrap();
                assert_eq!(
                    context.observation.consistent_visible_change,
                    behavior == "positive"
                );
                assert_eq!(
                    context.interpretation_source.as_ref().unwrap().model,
                    "fixture:image-model"
                );
                let catalog = live.runtime.catalog_id.clone();
                let comparison = naming::command(
                    &mut engine,
                    &reversebolt_core::Command {
                        action: "plan_runtime_comparisons".into(),
                        source: "fixture-agent".into(),
                        args: json!({"catalog_id":catalog,"review_id":derived.evidence_id}),
                    },
                    &base,
                )
                .unwrap();
                assert_eq!(
                    comparison["observed_actions"]["actions"]
                        .as_array()
                        .unwrap()
                        .len(),
                    4
                );
                assert!(comparison["next"].is_object());
                let before_inspection = json!(engine.live.as_ref().unwrap().view);
                let inspection = naming::command(
                    &mut engine,
                    &reversebolt_core::Command {
                        action: "inspect_runtime_investigation".into(),
                        source: "fixture-agent".into(),
                        args: json!({"catalog_id":catalog,"review_id":derived.evidence_id}),
                    },
                    &base,
                )
                .unwrap();
                assert!(inspection["activity_error"].is_null());
                assert_eq!(
                    inspection["activity"]["replay"]["actions"],
                    comparison["observed_actions"]
                );
                assert_eq!(
                    inspection["activity"]["context"],
                    comparison["activity_context"]
                );
                assert_eq!(inspection["questions"].as_array().unwrap().len(), 1);
                assert_eq!(json!(engine.live.as_ref().unwrap().view), before_inspection);
                let report = crate::runtime_bridge::catalog_report(
                    &engine.live.as_ref().unwrap().view.runtime,
                )
                .unwrap();
                let inventory = reversebolt_core::runtime_inventory::build(&report).unwrap();
                let reference: h::Reference =
                    serde_json::from_value(comparison["next"]["hypothesis_ref"].clone()).unwrap();
                let origin = h::resolve(&derived, &reference, &inventory).unwrap();
                assert_eq!(origin.question_origin, derived.question_origin);
                let restored: h::TestOrigin = serde_json::from_value(json!(origin)).unwrap();
                assert_eq!(restored, origin);
            }
        } else {
            assert_eq!(derived.evidence_id, original.evidence_id);
            assert_eq!(
                result["metadata_handoff"]["status"],
                if behavior == "cancel" {
                    "cancelled"
                } else {
                    "unavailable"
                }
            );
        }
        owner.cancel.store(false, Ordering::SeqCst);
        let revision = engine.live.as_ref().unwrap().view.runtime.revision;
        let replay = super::super::review::run_with(&mut engine, &step, &owner, &base, |_, _| {
            panic!("A completed review must not infer again")
        })
        .unwrap();
        assert_eq!(replay["replayed"], true);
        assert_eq!(replay["metadata_handoff"], result["metadata_handoff"]);
        assert_eq!(
            engine.live.as_ref().unwrap().view.runtime.revision,
            revision
        );
        let published = Arc::new(std::sync::RwLock::new(engine.view.clone()));
        // The production wrapper takes the replay path and checkpoints both
        // halves together; it must not reach its real model callback.
        super::super::review::run(&mut engine, &step, &owner, &base, &published).unwrap();
        let persisted: reversebolt_core::engine::View =
            serde_json::from_slice(&fs::read(root.join("engine.json")).unwrap()).unwrap();
        let persisted = persisted.live.unwrap();
        assert_eq!(
            persisted.runtime.naming.unwrap().evidence_id,
            derived.evidence_id
        );
        assert!(
            persisted.input_experiments[0]
                .metadata_handoff_sha256
                .is_some()
        );
        assert!(
            naming::link_activity::finish(&mut engine, &plan, &step.activity, &owner, &base)
                .is_err()
        );
        let archived = super::super::review::read_result(&engine, &step.activity, &base).unwrap();
        assert_eq!(archived["metadata_handoff"], result["metadata_handoff"]);
        let link = location(&base, &step.activity)
            .unwrap()
            .join("review/metadata/result.json");
        fs::write(&link, b"changed").unwrap();
        let archived = super::super::review::read_result(&engine, &step.activity, &base).unwrap();
        assert!(archived["metadata_handoff"].is_null());
        assert!(archived["metadata_handoff_error"].is_string());
        if matches!(behavior, "positive" | "negative") {
            assert!(naming::activity::latest(&engine, &base).unwrap().is_some());
        }
        drop(engine);
        cleanup(root);
    }
}
