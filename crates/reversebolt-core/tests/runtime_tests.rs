//! Synthetic protocol checks, not evidence of gameplay understanding.
use reversebolt_core::{
    changes::Phase as TrialPhase,
    inputs::{Buffer, Focus, Poll, Window},
    runtime::{Candidate, FieldRef, Locator, Member, Scope, Step, TypeKey},
    runtime_patch::Patch,
    runtime_tests::{Action, Frame, InputWitness, METRIC_ID, Phase, Test},
    runtime_trials::{Attempt, Context, Evidence, Trial},
    visual::{CaptureInterval, digest},
};
use serde_json::{Value, json};

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn check(p: &Patch, bytes: &[u8]) -> Value {
    json!({"path_id":p.path.id(),"status":"read","type_code":p.type_code,"bytes":bytes,
        "value":reversebolt_core::runtime::scalar_value(p.type_code,bytes).unwrap(),
        "field_address":format!("0x{:x}",p.field_address),
        "snapshot_object_addresses":p.object_addresses.iter().map(|a|format!("0x{a:x}")).collect::<Vec<_>>()})
}
fn reading(p: &Patch, bytes: &[u8]) -> Evidence {
    Evidence::new(
        "read.json".into(),
        json!({"operation":"refresh_runtime_paths","scope":p.path.scope,"checks":[check(p,bytes)]}),
    )
    .unwrap()
}
fn attempt(p: Patch) -> Attempt {
    let report = json!({"operation":"patch_runtime_path","scope":p.path.scope,"patch_id":p.id(),"checks":[check(&p,&p.expected)],
        "patch":{"state":"written","attempted":true,"transferred":p.replacement.len(),"guard":"ready","observed_after":p.replacement}});
    Attempt {
        started_ms: p.prepared_unix_ms,
        completed_ms: Some(p.prepared_unix_ms + 20),
        patch: p,
        evidence: Some(Evidence::new("write.json".into(), report).unwrap()),
        outcome: "written".into(),
        error: None,
    }
}
fn fixture() -> (Test, Trial, Trial) {
    let scope = Scope {
        pid: 7,
        created: "0123456789abcdef".into(),
        session: 2,
    };
    let ty = TypeKey {
        image: "Fixture.dll".into(),
        type_name: "Parameter".into(),
        layout_sha256: "a".repeat(64),
    };
    let field = FieldRef {
        declaring: ty.clone(),
        field_id: 1,
    };
    let path = Locator {
        scope: scope.clone(),
        root: field.clone(),
        steps: vec![Step {
            owner: ty,
            member: Member::Field { field },
        }],
    };
    let patch = Patch {
        path: path.clone(),
        type_code: 12,
        expected: 3.5f32.to_le_bytes().to_vec(),
        replacement: 0f32.to_le_bytes().to_vec(),
        field_address: 0x1010,
        object_addresses: vec![0x1000],
        prepared_unix_ms: 16000,
    };
    let test = Test {
        id: uuid(),
        change_id: uuid(),
        actor: "agent".into(),
        investigation: uuid(),
        catalog_id: uuid(),
        scope: scope.clone(),
        window: 42,
        candidate: Candidate {
            id: path.id(),
            name: "Coefficient".into(),
            description: "Observed field".into(),
            locator: path,
            value: json!("3.5"),
            bytes: patch.expected.clone(),
            type_code: 12,
            snapshot_id: 0,
            node_id: 0,
            edit_authority: false,
            context: vec![],
        },
        value: "0".into(),
        hypothesis: "Changing this field may suppress the rendered response".into(),
        origin: None,
        actions: vec![
            Action {
                keys: vec![68],
                hold_ms: 200
            };
            4
        ],
        prepared_revision: 0,
        created_unix_ms: 1000,
        preparation: None,
        metric: METRIC_ID.into(),
        revision: 0,
        phase: Phase::Baseline,
        pending_capture: false,
        conditions: Default::default(),
        assessment: None,
        reason: String::new(),
    };
    let write = attempt(patch.clone());
    let trial = Trial {
        id: test.change_id.clone(),
        direct: true,
        actor: "agent".into(),
        context: Context {
            investigation: test.investigation.clone(),
            catalog_id: test.catalog_id.clone(),
            revision: 2,
            process: scope,
        },
        candidate_id: test.candidate.id.clone(),
        name: "Coefficient".into(),
        baseline: reading(&patch, &patch.expected),
        patch: patch.clone(),
        phase: TrialPhase::Written,
        reason: String::new(),
        write: Some(write.clone()),
        restoration: None,
    };
    let inverse = patch
        .restoration(&write.evidence.unwrap().report, 24000)
        .unwrap();
    let restored = Trial {
        restoration: Some(attempt(inverse)),
        phase: TrialPhase::Restored,
        ..trial.clone()
    };
    (test, trial, restored)
}

fn window(clock: &str, begin: u64, end: u64, press: bool) -> Window {
    let mut buffer = Buffer::new(clock.into(), "monitor".into(), 0);
    for t in (begin + 1000..end - 100).step_by(16000) {
        buffer
            .push(Poll {
                begin_us: t,
                end_us: t + 100,
                focus: Focus::SelectedForeground,
                down: if press && (100_000..300_000).contains(&(t - begin)) {
                    vec![68]
                } else {
                    vec![]
                },
            })
            .unwrap();
    }
    buffer.window(begin, end).unwrap()
}
fn next(test: &Test, difference: f64) -> (Frame, Window) {
    let condition = test.condition_index().unwrap();
    let n = test.conditions[condition].frames.len();
    let begin = (condition as u64 * 10 + n as u64 + 1) * 620_000;
    let previous = test.conditions[condition]
        .frames
        .last()
        .map_or(begin, |f| f.interval.end_us);
    let interval = CaptureInterval {
        clock_id: test.investigation.clone(),
        begin_us: begin,
        end_us: begin + 20_000,
    };
    let input = window(&test.investigation, previous, interval.end_us, n >= 2);
    let wall = [10_000, 17_000, 25_000][condition] + n as u128 * 1000;
    let f = Frame {
        path: format!("{condition}-{n}.png"),
        sha256: "a".repeat(64),
        input_path: format!("{condition}-{n}.json"),
        input_sha256: digest(&serde_json::to_vec(&Some(&input)).unwrap()),
        width: 640,
        height: 360,
        interval: interval.clone(),
        started_unix_ms: wall,
        completed_unix_ms: wall + 20,
        sampling: None,
        difference: (n > 0).then_some(if n == 1 { 0.25 } else { difference }),
        input: InputWitness::check(None, None, &interval, previous),
    };
    (f, input)
}
fn condition(test: &mut Test, difference: f64) {
    for _ in 0..6 {
        let (f, input) = next(test, difference);
        test.begin_capture(test.revision).unwrap();
        test.finish_capture(f, Some(&input)).unwrap();
    }
}
fn ready(changed: f64) -> (Test, Trial) {
    let (mut t, write, restored) = fixture();
    condition(&mut t, 6.);
    assert_eq!(t.phase, Phase::ReadyToApply);
    t.sync_trial(&write).unwrap();
    condition(&mut t, changed);
    assert_eq!(t.phase, Phase::ReadyToRestore);
    t.conditions[1].runtime_evidence = Some(reading(&write.patch, &write.patch.replacement));
    t.conditions[1].runtime_checked_unix_ms = Some(23000);
    t.sync_trial(&restored).unwrap();
    condition(&mut t, 6.);
    t.conditions[2].runtime_evidence = Some(reading(&write.patch, &write.patch.expected));
    t.conditions[2].runtime_checked_unix_ms = Some(31000);
    (t, restored)
}

#[test]
fn comparison_needs_ordered_action_images_value_checks_and_recovery() {
    for (difference, status) in [
        (0.3, "suppression_observed"),
        (6., "suppression_not_observed"),
    ] {
        let (mut t, restored) = ready(difference);
        assert!(t.assessment.is_none());
        t.complete(&restored, t.revision).unwrap();
        assert_eq!(t.phase, Phase::Complete);
        assert_eq!(t.assessment.as_ref().unwrap().status, status);
        assert!(!t.candidate.edit_authority);
        assert!(t.complete(&restored, t.revision).is_err());
    }
}

#[test]
fn weak_baseline_and_missing_sampling_never_reach_apply() {
    let (mut t, _, _) = fixture();
    condition(&mut t, 1.);
    assert_eq!(t.phase, Phase::Stopped);
    let (mut t, _, _) = fixture();
    for _ in 0..6 {
        let (mut f, _) = next(&t, 6.);
        f.input_sha256 = digest(b"null");
        t.begin_capture(t.revision).unwrap();
        t.finish_capture(f, None).unwrap();
    }
    assert_eq!(t.phase, Phase::Stopped);
}

#[test]
fn input_witness_rejects_wrong_keys_focus_taps_duration_gaps_and_late_input() {
    let interval = CaptureInterval {
        clock_id: "clock".into(),
        begin_us: 600_000,
        end_us: 620_000,
    };
    let action = Action {
        keys: vec![68],
        hold_ms: 200,
    };
    let good = window("clock", 0, 620_000, true);
    let check = |w: &Window| InputWitness::check(Some(w), Some(&action), &interval, 0).matches;
    assert!(check(&good));
    for variant in 0..10 {
        let mut w = good.clone();
        match variant {
            0 => w.polls[0].focus = Focus::OutsideSelected,
            1 => w.polls[12].down = vec![65],
            2 => w.polls[12].down.clear(),
            3 => w.polls[0].begin_us = 40000,
            4 => w.largest_unsampled_us = 0,
            5 => w.omitted_polls = 1,
            6 => w.clock_id = "different".into(),
            7 => w.polls.iter_mut().for_each(|p| {
                if !p.down.is_empty() {
                    p.down.clear()
                }
            }),
            8 => w.polls.iter_mut().for_each(|p| {
                if p.begin_us > 300_000 {
                    p.down = vec![68]
                }
            }),
            _ => w.polls[12].end_us = w.polls[13].begin_us + 1,
        }
        assert!(!check(&w), "variant {variant}");
    }
    let wrong = Action {
        hold_ms: 500,
        ..action
    };
    assert!(!InputWitness::check(Some(&good), Some(&wrong), &interval, 0).matches);
    let idle = window("clock", 0, 620000, false);
    assert!(InputWitness::check(Some(&idle), None, &interval, 0).matches);
}

#[test]
fn overwritten_values_moved_references_and_bad_read_chronology_are_inconclusive() {
    for variant in 0..7 {
        let (mut t, trial) = ready(0.3);
        let c = &mut t.conditions[1];
        match variant {
            0 => c.runtime_evidence = Some(reading(&trial.patch, &trial.patch.expected)),
            1 => {
                let mut patch = trial.patch.clone();
                patch.object_addresses[0] += 16;
                c.runtime_evidence = Some(reading(&patch, &patch.replacement));
            }
            2 => c.runtime_evidence.as_mut().unwrap().report_sha256 = "b".repeat(64),
            3 => c.runtime_checked_unix_ms = Some(17000),
            4 => c.runtime_checked_unix_ms = Some(26000),
            5 => t.conditions[2].frames[0].started_unix_ms = 23000,
            _ => t.conditions[2].runtime_evidence = None,
        }
        t.complete(&trial, t.revision).unwrap();
        assert_eq!(
            t.assessment.unwrap().status,
            "inconclusive",
            "variant {variant}"
        );
    }
}

#[test]
fn stale_cursor_changed_geometry_generation_or_digest_cannot_append() {
    let (mut t, _, _) = fixture();
    let (f, w) = next(&t, 6.);
    assert!(t.begin_capture(1).is_err());
    t.begin_capture(0).unwrap();
    assert!(t.begin_capture(0).is_err());
    t.finish_capture(f, Some(&w)).unwrap();
    let original = t.clone();
    for variant in 0..5 {
        let mut t = original.clone();
        let (mut f, mut w) = next(&t, 6.);
        match variant {
            0 => f.width += 1,
            1 => f.interval.clock_id = "wrong".into(),
            2 => w.generation = "different".into(),
            3 => f.input_sha256 = "b".repeat(64),
            _ => f.interval.begin_us = 0,
        }
        if variant == 2 {
            f.input_sha256 = digest(&serde_json::to_vec(&Some(&w)).unwrap());
        }
        t.begin_capture(t.revision).unwrap();
        assert!(t.finish_capture(f, Some(&w)).is_err());
        assert_eq!(t.conditions[0].frames.len(), 1);
        assert!(t.pending_capture);
        t.recover();
        assert_eq!(t.phase, Phase::Stopped);
        assert!(t.begin_capture(t.revision).is_err());
    }
}

#[test]
fn premature_edit_early_undo_wrong_trial_and_restart_stop_comparison() {
    let (mut t, write, _) = fixture();
    t.sync_trial(&write).unwrap();
    assert_eq!(t.phase, Phase::Stopped);
    let (mut t, _, _) = fixture();
    condition(&mut t, 6.);
    assert!(t.sync_trial(&write).is_err()); // different planned identity
    let (mut t, write, restored) = fixture();
    condition(&mut t, 6.);
    t.sync_trial(&write).unwrap();
    t.sync_trial(&restored).unwrap();
    assert_eq!(t.phase, Phase::Stopped);
    let (mut t, _, _) = fixture();
    condition(&mut t, 6.);
    t.recover();
    assert_eq!(t.phase, Phase::Stopped);
    assert!(t.complete(&restored, t.revision).is_err());
}

#[test]
fn invalid_plans_are_rejected_before_capture_and_without_mutation() {
    let (good, _, _) = fixture();
    for variant in 0..8 {
        let mut t = good.clone();
        match variant {
            0 => t.id = "../escape".into(),
            1 => t.change_id = t.id.clone(),
            2 => t.actions[0].keys = vec![68, 68],
            3 => t.actions[0].hold_ms = 1001,
            4 => t.metric = "other".into(),
            5 => t.value = "NaN".into(),
            6 => t.candidate.value = json!("4.5"),
            _ => t.scope.pid += 1,
        }
        assert!(t.begin_capture(0).is_err());
        assert!(!t.pending_capture);
    }
}

#[test]
fn saved_capture_intent_is_not_replayed_after_restart_or_storage_failure() {
    use reversebolt_core::runtime_test_store::Store;
    let dir = std::env::temp_dir().join(format!("reversebolt-test-{}", uuid()));
    let mut store = Store::open(dir.clone()).unwrap();
    let (mut t, _, _) = fixture();
    store.record(t.clone()).unwrap();
    t.begin_capture(0).unwrap();
    store.record(t.clone()).unwrap();
    store.reserve(&t, 0, 0).unwrap();
    assert!(store.reserve(&t, 0, 0).is_err());
    drop(store);
    let mut store = Store::open(dir.clone()).unwrap();
    let recovered = store.get(&t.id).unwrap();
    assert_eq!(recovered.phase, Phase::Stopped);
    assert!(recovered.pending_capture);
    let (new, _, _) = fixture();
    std::fs::create_dir(dir.join(format!("{}.pending", new.id))).unwrap();
    assert!(store.record(new.clone()).is_err());
    assert!(store.fault.is_some());
    assert!(store.get(&new.id).is_err());
    assert!(store.record(fixture().0).is_err());
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn retained_evidence_paths_and_checksums_are_enforced() {
    use reversebolt_core::runtime_test_store::Store;
    let dir = std::env::temp_dir().join(format!("reversebolt-evidence-{}", uuid()));
    let mut store = Store::open(dir.clone()).unwrap();
    let (mut t, _, _) = fixture();
    let (mut f, w) = next(&t, 6.);
    t.begin_capture(0).unwrap();
    store.record(t.clone()).unwrap();
    let (image_path, input_path) = store.reserve(&t, 0, 0).unwrap();
    let png = b"\x89PNG\r\n\x1a\nfixture"; // storage checks bytes; decoding is the vision adapter's task
    std::fs::write(image_path, png).unwrap();
    std::fs::write(&input_path, serde_json::to_vec(&Some(&w)).unwrap()).unwrap();
    f.path = format!("{}/0-0.png", t.id);
    f.input_path = format!("{}/0-0.input.json", t.id);
    f.sha256 = digest(png);
    t.finish_capture(f, Some(&w)).unwrap();
    store.record(t.clone()).unwrap();
    assert_eq!(store.read_image(&t, 0, 0).unwrap(), png);
    assert!(store.read_input(&t, 0, 0).unwrap().is_some());
    let plan_path = dir.join(&t.id).join("0-0.action.json");
    let plan = serde_json::json!({"schema":1,"test_id":t.id,"condition":0,"frame":0,
        "target":{"window":t.window,"session":t.scope.session},"action":null});
    let plan_bytes = serde_json::to_vec(&plan).unwrap();
    std::fs::write(&plan_path, &plan_bytes).unwrap();
    assert!(store.read_executions(&t).unwrap()[0]["outcome"].is_null());
    let outcome = plan_path.with_extension("outcome.json");
    std::fs::write(
        &outcome,
        serde_json::to_vec(
            &serde_json::json!({"plan_sha256":digest(&plan_bytes),"error":"Focus lost"}),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        store.read_executions(&t).unwrap()[0]["outcome"]["receipt"]["error"],
        "Focus lost"
    );
    std::fs::write(&outcome, b"{\"plan_sha256\":\"other\"}").unwrap();
    assert!(store.read_executions(&t).is_err());
    std::fs::write(input_path, b"null").unwrap();
    assert!(store.read_input(&t, 0, 0).is_err());
    t.conditions[0].frames[0].path = "../../other.png".into();
    assert!(store.read_image(&t, 0, 0).is_err());
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn hypothesis_provenance_survives_restart_and_rejects_changed_question_or_path() {
    use reversebolt_core::{runtime_hypotheses as h, runtime_test_store::Store};
    let (mut test, _, _) = fixture();
    let row = h::Row {
        id: 17,
        type_id: 2,
        field_id: 1,
        path_id: test.candidate.id.clone(),
        type_name: "Parameter".into(),
        field_name: test.candidate.name.clone(),
        value: test.candidate.value.clone(),
        object_text: vec!["Speed".into()],
    };
    let proposal = h::Proposal {
        candidate: 17,
        name: "Speed".into(),
        explanation: "May influence speed; this remains untested.".into(),
        question: "Does this field affect the rendered response?".into(),
        citations: vec![h::Citation {
            source: "object_text".into(),
            index: Some(0),
            quote: "Speed".into(),
        }],
        alternatives: vec![],
    };
    test.hypothesis = proposal.question.clone();
    test.origin = Some(h::TestOrigin {
        question_source: Default::default(),
        question_policy: None,
        play_context: None,
        reference: h::Reference {
            review_id: uuid(),
            hypothesis_path_id: row.path_id.clone(),
            selected_path_id: row.path_id.clone(),
        },
        report_sha256: "a".repeat(64),
        hypothesis: h::Hypothesis {
            proposal,
            candidate: row,
            alternatives: vec![],
        },
    });
    test.validate_plan().unwrap();
    let directory = std::env::temp_dir().join(format!("reversebolt-origin-{}", uuid()));
    let frame = |n: usize| reversebolt_core::play_context::Frame {
        id: format!("f{n}"),
        sha256: "a".repeat(64),
        number: n,
        width: 100,
        height: 100,
        interval: CaptureInterval {
            clock_id: test.investigation.clone(),
            begin_us: n as u64 * 1_000_000,
            end_us: n as u64 * 1_000_000 + 1000,
        },
    };
    test.origin.as_mut().unwrap().play_context = Some(reversebolt_core::play_context::Context {
        basis: Default::default(),
        interpretation_source: None,
        source: reversebolt_core::play_context::Source {
            investigation_id: test.investigation.clone(),
            batch: 1,
            frame_metadata_sha256: "b".repeat(64),
            key: 68,
            quiet_difference: 0.,
            pairs: [1, 4].map(|n| reversebolt_core::play_context::Pair {
                episode: n,
                before: frame(n),
                after: frame(n + 1),
                input_sha256: "c".repeat(64),
                difference: 10.,
            }),
        },
        observation: reversebolt_core::play_context::Observation {
            consistent_visible_change: true,
            description: "The visible pattern shifts.".into(),
            uncertainty: "No physical quantity or field relationship is known.".into(),
        },
    });
    test.validate_plan().unwrap();
    let origin = test.origin.as_mut().unwrap();
    origin.play_context = Some(
        origin
            .play_context
            .take()
            .unwrap()
            .bind_interpretation(&origin.reference.review_id, "original-image-model")
            .unwrap(),
    );
    test.validate_plan().unwrap();
    let mut legacy = serde_json::to_value(&test.origin).unwrap();
    legacy.as_object_mut().unwrap().remove("play_context");
    let legacy: h::TestOrigin = serde_json::from_value(legacy).unwrap();
    assert!(legacy.play_context.is_none());
    legacy.selected().unwrap();
    let mut wrong_scope = test.clone();
    wrong_scope.investigation = uuid();
    assert!(wrong_scope.validate_plan().is_err());
    let mut uncertain = test.clone();
    uncertain
        .origin
        .as_mut()
        .unwrap()
        .play_context
        .as_mut()
        .unwrap()
        .observation
        .consistent_visible_change = false;
    assert!(uncertain.validate_plan().is_err());
    let context = uncertain
        .origin
        .as_mut()
        .unwrap()
        .play_context
        .as_mut()
        .unwrap();
    context.basis = reversebolt_core::play_context::Basis::MeasuredContrastV1;
    uncertain.validate_plan().unwrap();
    assert!(
        !uncertain
            .origin
            .as_ref()
            .unwrap()
            .play_context
            .as_ref()
            .unwrap()
            .observation
            .consistent_visible_change
    );
    let mut store = Store::open(directory.clone()).unwrap();
    store.record(test.clone()).unwrap();
    drop(store);
    let store = Store::open(directory.clone()).unwrap();
    assert_eq!(store.get(&test.id).unwrap().origin, test.origin);
    assert_eq!(store.get(&test.id).unwrap().phase, Phase::Stopped);
    for variant in 0..5 {
        let mut changed = test.clone();
        match variant {
            0 => changed.hypothesis = "A different explanation?".into(),
            1 => changed.origin.as_mut().unwrap().reference.selected_path_id = "b".repeat(64),
            2 => {
                changed
                    .origin
                    .as_mut()
                    .unwrap()
                    .hypothesis
                    .proposal
                    .citations[0]
                    .quote = "Health".into()
            }
            3 => {
                changed
                    .origin
                    .as_mut()
                    .unwrap()
                    .hypothesis
                    .proposal
                    .candidate = 18
            }
            _ => changed.origin.as_mut().unwrap().reference.review_id = "../other".into(),
        }
        assert!(changed.validate_plan().is_err());
    }
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}
