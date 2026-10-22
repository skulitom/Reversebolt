//! Real dispatcher/history checks with no attached target or native operations.
use super::*;
use reversebolt_core::runtime::{FieldRef, Locator, Member, Scope, Step, TypeKey};

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn saved(investigation: &str) -> Test {
    let scope = Scope {
        pid: 1,
        created: "0000000000000001".into(),
        session: 1,
    };
    let key = TypeKey {
        image: "Fixture".into(),
        type_name: "Actor".into(),
        layout_sha256: "a".repeat(64),
    };
    let field = FieldRef {
        declaring: key.clone(),
        field_id: 0,
    };
    let path = Locator {
        scope: scope.clone(),
        root: field.clone(),
        steps: vec![Step {
            owner: key,
            member: Member::Field { field },
        }],
    };
    serde_json::from_value(json!({
        "id":id(),"change_id":id(),"actor":"agent","investigation":investigation,"catalog_id":id(),
        "scope":scope,"window":1,
        "candidate":{"id":path.id(),"name":"Speed","description":"Synthetic fixture.","locator":path,
            "value":8,"bytes":8i32.to_le_bytes(),"type_code":8,"snapshot_id":0,"node_id":0,"edit_authority":false},
        "value":"0","hypothesis":"Does the observed response change?","actions":vec![Action{keys:vec![68],hold_ms:200};4],
        "prepared_revision":1,"created_unix_ms":1,"preparation":null,"metric":runtime_tests::METRIC_ID,
        "revision":2,"phase":"stopped","pending_capture":false,"conditions":[{"frames":[]},{"frames":[]},{"frames":[]}],
        "assessment":null,"reason":"Synthetic stopped comparison."
    })).unwrap()
}

fn history_choice(test: &Test) -> Value {
    json!({"hypothesis_ref":{"selected_path_id":test.candidate.id},
        "value":test.value,"actions":test.actions,"previous_test":test.summary(),
        "status":test.assessment.as_ref().map_or("stopped", |a| a.status.as_str())})
}

fn review_snapshot(engine: &Engine, test: &Test) -> Value {
    engine
        .view
        .runtime_tests
        .iter()
        .find(|v| v["id"] == test.id)
        .map_or(Value::Null, |v| v["last_regional_review"].clone())
}

fn assert_regional_history(engine: &mut Engine, test: &Test, expected: &Value) {
    let untouched = json!({"previous_test":null,"status":"untried"});
    let mut choices = vec![history_choice(test), untouched.clone()];
    let records = json!(engine.runtime_tests.records);
    let history = comparison_history(engine, &mut choices).unwrap();
    assert_eq!(history["checked_test_ids"], json!([test.id]));
    assert_eq!(choices[0]["regional_intervention_assessment"], *expected);
    assert_eq!(choices[0]["status"], expected["status"]);
    assert_eq!(choices[0]["whole_screen_status"], "inconclusive");
    assert_eq!(choices[0]["previous_test"], test.summary());
    assert_eq!(choices[1], untouched);
    assert_eq!(json!(engine.runtime_tests.records), records);
    crate::runtime_test_review::publish(
        engine,
        Some(("plan_runtime_comparisons", &json!({"choices":choices}))),
    );
    assert_eq!(review_snapshot(engine, test)["assessment"], *expected);
    let snapshot = review_snapshot(engine, test);
    crate::runtime_test_review::publish(engine, None);
    assert_eq!(review_snapshot(engine, test), snapshot);
}

#[test]
fn regional_history_bounds_work_and_rejects_mismatched_or_unavailable_results() {
    // These deliberately incomplete records test scheduling/binding only.
    // Actual image/receipt validation is exercised by the integration test below.
    let tests: Vec<_> = (0..10)
        .map(|_| {
            let mut t = saved(&id());
            t.phase = Phase::Complete;
            t.appearance = Some(serde_json::from_value(json!({
                "selection":{"baseline_sha256":"a".repeat(64),"region":{"x":0,"y":0,"width":32,"height":32}},
                "comparison_plan_sha256":"b".repeat(64),"baseline_record_sha256":"c".repeat(64),
                "selected_revision":1,"selected_unix_ms":1
            })).unwrap());
            t.assessment = Some(runtime_tests::Assessment {
                status: "suppression_observed".into(), reason: "Synthetic".into(),
                medians: [None; 3], limitation: "Scheduling fixture only".into(),
            });
            t
        }).collect();
    let assessment = |t: &Test| {
        json!({"policy":reversebolt_vision::trial_intervention::POLICY,
        "test_id":t.id,"change_id":t.change_id,
        "test_sha256":reversebolt_core::visual::digest(&serde_json::to_vec(t).unwrap()),
        "status":"regional_change_and_return_observed"})
    };
    let mut choices: Vec<_> = tests.iter().map(history_choice).collect();
    choices.push(choices[0].clone());
    choices.push(json!({"previous_test":null,"status":"untried"}));
    let untouched = choices[11].clone();
    let mut calls = vec![];
    let history = regional_history(&tests, &mut choices, |t| {
        calls.push(t.id.clone());
        if t.id == tests[1].id {
            return Err("Missing evidence".into());
        }
        let mut a = assessment(t);
        if t.id == tests[2].id {
            a["test_sha256"] = json!("d".repeat(64));
        }
        Ok(a)
    })
    .unwrap();
    assert_eq!(
        calls,
        tests[..8].iter().map(|t| t.id.clone()).collect::<Vec<_>>()
    );
    assert_eq!(history["checked_test_ids"], json!(calls));
    assert_eq!(
        history["analysis_error_test_ids"],
        json!([tests[1].id, tests[2].id])
    );
    assert_eq!(history["omitted_test_ids"].as_array().unwrap().len(), 2);
    for i in [8, 9] {
        assert_eq!(choices[i]["status_basis"], "regional_not_checked");
        assert_eq!(choices[i]["status"], "inconclusive");
    }
    for i in [1, 2] {
        assert_eq!(choices[i]["status_basis"], "regional_unavailable");
        assert_eq!(choices[i]["status"], "inconclusive");
        assert_eq!(choices[i]["whole_screen_status"], "suppression_observed");
    }
    assert_eq!(choices[10], choices[0]); // Duplicate consumes no additional work.
    assert_eq!(choices[11], untouched);
    assert_eq!(
        choices.iter().filter(|c| c["status"] == "untried").count(),
        1
    );
    // A fresh review must remove a previous positive assessment on failure.
    regional_history(&tests, &mut choices, |_| Err("Unavailable now".into())).unwrap();
    assert!(choices[0].get("regional_intervention_assessment").is_none());
    assert_eq!(choices[0]["status"], "inconclusive");
    assert_eq!(choices[0]["whole_screen_status"], "suppression_observed");

    for field in ["previous_test", "hypothesis_ref", "value", "actions"] {
        let mut wrong = history_choice(&tests[0]);
        match field {
            "previous_test" => wrong[field]["revision"] = json!(999),
            "hypothesis_ref" => wrong[field]["selected_path_id"] = json!("other"),
            "value" => wrong[field] = json!("1"),
            _ => wrong[field] = json!([]),
        }
        assert!(
            regional_history(&tests, &mut [wrong], |_| panic!(
                "Must reject before analysis"
            ))
            .is_err()
        );
    }
    let mut stopped = tests[0].clone();
    stopped.phase = Phase::Stopped;
    let mut legacy = tests[1].clone();
    legacy.appearance = None;
    let skipped = vec![history_choice(&stopped), history_choice(&legacy)];
    let mut unchanged = skipped.clone();
    let history =
        regional_history(&[stopped, legacy], &mut unchanged, |_| panic!("Must skip")).unwrap();
    assert_eq!(unchanged, skipped);
    assert_eq!(history["checked_test_ids"], json!([]));
}

fn prepare(t: &Test) -> Command {
    Command {
        action: "prepare_runtime_test".into(),
        source: t.actor.clone(),
        args: json!({
            "test_id":t.id,"change_id":t.change_id,"investigation_id":t.investigation,"catalog_id":t.catalog_id,
            "after_revision":t.prepared_revision,"candidate_id":t.candidate.id,"value":t.value,
            "hypothesis":t.hypothesis,"actions":t.actions
        }),
    }
}

fn motion_fixture(local: bool) -> (std::path::PathBuf, std::path::PathBuf, Engine, Test) {
    use reversebolt_core::{
        inputs::{Buffer, Focus, Poll},
        visual::{CaptureInterval, digest},
    };
    let directory = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("reversebolt-trial-motion-{}", id()));
    std::fs::create_dir(&directory).unwrap();
    let state_path = directory.join("state.json");
    let engine = Engine::load(state_path.clone()).unwrap();
    let mut test = saved(&id());
    test.actions
        .iter_mut()
        .enumerate()
        .for_each(|(i, a)| a.keys = vec![if i % 2 == 0 { 68 } else { 65 }]);
    let folder = directory.join("runtime-tests").join(&test.id);
    std::fs::create_dir(&folder).unwrap();
    let texture = image::GrayImage::from_fn(256, 144, |x, y| {
        let mut n = (x + y * 256)
            .wrapping_add(9)
            .wrapping_mul(747796405)
            .wrapping_add(2891336453);
        n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
        image::Luma([32 + (((n >> 22) ^ n) % 128) as u8])
    });
    for c in 0..3 {
        for i in 0..6 {
            let shift = if c != 1 && i >= 2 && i % 2 == 0 { 8 } else { 0 };
            let image = if local {
                let mut background = image::GrayImage::from_fn(512, 288, |x, y| {
                    let mut n = (x + y * 512)
                        .wrapping_add(811)
                        .wrapping_mul(747796405)
                        .wrapping_add(2891336453);
                    n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
                    image::Luma([32 + (((n >> 22) ^ n) % 128) as u8])
                });
                let subject = image::GrayImage::from_fn(64, 64, |x, y| *texture.get_pixel(x, y));
                image::imageops::replace(&mut background, &subject, i64::from(64 + shift * 2), 80);
                background
            } else {
                image::GrayImage::from_fn(256, 144, |x, y| {
                    if x >= shift {
                        *texture.get_pixel(x - shift, y)
                    } else {
                        image::Luma([0])
                    }
                })
            };
            let dimensions = image.dimensions();
            let mut png = std::io::Cursor::new(vec![]);
            image.write_to(&mut png, image::ImageFormat::Png).unwrap();
            let bytes = png.into_inner();
            let start = ((c * 10 + i + 1) as u64) * 620000;
            let interval = CaptureInterval {
                clock_id: test.investigation.clone(),
                begin_us: start,
                end_us: start + 20000,
            };
            let begin = test.conditions[c]
                .frames
                .last()
                .map_or(start, |f| f.interval.end_us);
            let action = i.checked_sub(2).and_then(|a| test.actions.get(a));
            let mut buffer = Buffer::new(test.investigation.clone(), "synthetic-monitor".into(), 0);
            for t in (begin + 1000..interval.end_us - 100).step_by(16000) {
                buffer
                    .push(Poll {
                        begin_us: t,
                        end_us: t + 100,
                        focus: Focus::SelectedForeground,
                        down: action
                            .filter(|_| (100000..300000).contains(&(t - begin)))
                            .map_or_else(Vec::new, |a| a.keys.clone()),
                    })
                    .unwrap();
            }
            let window = buffer.window(begin, interval.end_us);
            let input = serde_json::to_vec(&window).unwrap();
            let witness = InputWitness::check(window.as_ref(), action, &interval, begin);
            if i > 0 {
                assert!(witness.matches, "{}", witness.reason);
            }
            std::fs::write(folder.join(format!("{c}-{i}.png")), &bytes).unwrap();
            std::fs::write(folder.join(format!("{c}-{i}.input.json")), &input).unwrap();
            test.conditions[c].frames.push(Frame {
                path: format!("{}/{c}-{i}.png", test.id),
                input_path: format!("{}/{c}-{i}.input.json", test.id),
                sha256: digest(&bytes),
                input_sha256: digest(&input),
                width: dimensions.0,
                height: dimensions.1,
                interval,
                started_unix_ms: u128::from(start / 1000),
                completed_unix_ms: u128::from(start / 1000 + 20),
                sampling: None,
                difference: (i > 0).then_some(0.),
                input: witness,
            });
        }
    }
    (directory, state_path, engine, test)
}

fn measured_saved_frame(engine: &Engine, complete: &Test, condition: usize, i: usize) -> Frame {
    let mut frame = complete.conditions[condition].frames[i].clone();
    frame.difference = if i == 0 {
        None
    } else {
        Some(
            reversebolt_vision::response::difference(
                &engine
                    .runtime_tests
                    .read_image(complete, condition, i - 1)
                    .unwrap(),
                &engine
                    .runtime_tests
                    .read_image(complete, condition, i)
                    .unwrap(),
            )
            .unwrap(),
        )
    };
    frame
}

/// Feed saved PNG/input bytes through the same core completion operation
/// used by native capture, including the actual whole-screen response metric.
fn collect_saved_baseline(engine: &Engine, complete: &Test) -> Test {
    let mut t = complete.clone();
    t.conditions = Default::default();
    t.phase = Phase::Baseline;
    for i in 0..6 {
        let frame = measured_saved_frame(engine, complete, 0, i);
        let input = engine.runtime_tests.read_input(complete, 0, i).unwrap();
        t.begin_capture(t.revision).unwrap();
        t.finish_capture(frame, input.as_ref()).unwrap();
    }
    t
}

#[test]
fn regional_intervention_joins_exact_edit_records_without_promoting_the_control() {
    use reversebolt_core::{
        changes::Phase as TrialPhase,
        runtime_trials::{Attempt, Context, Evidence, Trial},
    };
    let (directory, state_path, mut engine, mut images) = motion_fixture(true);
    let patch = Patch {
        path: images.candidate.locator.clone(),
        type_code: 8,
        expected: 8i32.to_le_bytes().to_vec(),
        replacement: 0i32.to_le_bytes().to_vec(),
        field_address: 0x2000,
        object_addresses: vec![0x1000],
        prepared_unix_ms: 4500,
    };
    let check = |p: &Patch, bytes: &[u8]| {
        json!({"path_id":p.path.id(),"status":"read","type_code":p.type_code,"bytes":bytes,
        "value":scalar_value(p.type_code,bytes).unwrap(),"field_address":format!("0x{:x}",p.field_address),
        "snapshot_object_addresses":p.object_addresses.iter().map(|a|format!("0x{a:x}")).collect::<Vec<_>>()})
    };
    let reading = |bytes: &[u8]| {
        Evidence::new("synthetic-read.json".into(),
        json!({"operation":"refresh_runtime_paths","scope":patch.path.scope,"checks":[check(&patch,bytes)]})).unwrap()
    };
    let attempt = |p: Patch| {
        let e = Evidence::new("synthetic-write.json".into(), json!({"operation":"patch_runtime_path","scope":p.path.scope,"patch_id":p.id(),
            "checks":[check(&p,&p.expected)],"patch":{"state":"written","attempted":true,"transferred":p.replacement.len(),"guard":"ready","observed_after":p.replacement}})).unwrap();
        Attempt {
            started_ms: p.prepared_unix_ms,
            completed_ms: Some(p.prepared_unix_ms + 20),
            patch: p,
            evidence: Some(e),
            outcome: "written".into(),
            error: None,
        }
    };
    // All receipts and wall times in this test are generated, not native edits.
    images.preparation = Some(reading(&patch.expected));
    let mut test = collect_saved_baseline(&engine, &images);
    engine.runtime_tests.record(test.clone()).unwrap();
    let proposals = reversebolt_vision::trial_appearance_suggestions::analyze(
        &test,
        |c, i| engine.runtime_tests.read_image(&test, c, i),
        |c, i| engine.runtime_tests.read_input(&test, c, i),
    )
    .unwrap();
    test.finish_baseline_review(proposals.recommended, test.revision, 4000)
        .unwrap();
    engine.runtime_tests.record(test.clone()).unwrap();
    let written = Trial {
        id: test.change_id.clone(),
        direct: true,
        actor: "agent".into(),
        context: Context {
            investigation: test.investigation.clone(),
            catalog_id: test.catalog_id.clone(),
            revision: 1,
            process: test.scope.clone(),
        },
        candidate_id: test.candidate.id.clone(),
        name: "Synthetic parameter".into(),
        patch: patch.clone(),
        baseline: reading(&patch.expected),
        phase: TrialPhase::Written,
        reason: "Synthetic receipt".into(),
        write: Some(attempt(patch.clone())),
        restoration: None,
    };
    test.sync_trial(&written).unwrap();
    for i in 0..6 {
        let input = engine.runtime_tests.read_input(&images, 1, i).unwrap();
        test.begin_capture(test.revision).unwrap();
        test.finish_capture(measured_saved_frame(&engine, &images, 1, i), input.as_ref())
            .unwrap();
    }
    test.conditions[1].runtime_evidence = Some(reading(&patch.replacement));
    test.conditions[1].runtime_checked_unix_ms = Some(10000);
    let inverse = patch
        .restoration(
            &written
                .write
                .as_ref()
                .unwrap()
                .evidence
                .as_ref()
                .unwrap()
                .report,
            11000,
        )
        .unwrap();
    let restored = Trial {
        phase: TrialPhase::Restored,
        restoration: Some(attempt(inverse)),
        ..written
    };
    test.sync_trial(&restored).unwrap();
    for i in 0..6 {
        let input = engine.runtime_tests.read_input(&images, 2, i).unwrap();
        test.begin_capture(test.revision).unwrap();
        test.finish_capture(measured_saved_frame(&engine, &images, 2, i), input.as_ref())
            .unwrap();
    }
    test.conditions[2].runtime_evidence = Some(reading(&patch.expected));
    test.conditions[2].runtime_checked_unix_ms = Some(17000);
    test.complete(&restored, test.revision).unwrap();
    test.check_recorded_intervention(&restored).unwrap();
    assert_eq!(test.assessment.as_ref().unwrap().status, "inconclusive");
    engine.runtime_tests.record(test.clone()).unwrap();
    // Persist the generated receipt to exercise strict history loading on restart.
    std::fs::write(
        engine
            .runtime_trials
            .directory()
            .join(format!("{}.json", restored.id)),
        serde_json::to_vec(&restored).unwrap(),
    )
    .unwrap();
    engine.runtime_trials.records.push(restored.clone());
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    let command = Command {
        action: "read_runtime_test".into(),
        source: "agent".into(),
        args: json!({"test_id":test.id}),
    };
    let read = |session: &mut Session, engine: &mut Engine| {
        session
            .command(engine, &command, &directory, &published)
            .unwrap()
    };
    let result = read(&mut session, &mut engine);
    assert_eq!(
        result["regional_intervention_assessment"]["status"],
        "regional_change_and_return_observed"
    );
    assert!(result["regional_intervention_assessment"]["intervention_error"].is_null());
    assert_eq!(
        result["regional_intervention_assessment"]["trial_sha256"],
        reversebolt_core::visual::digest(&serde_json::to_vec(&restored).unwrap())
    );
    let mut completion = test.summary();
    completed_regional_assessment(&engine, &test, &mut completion);
    assert_eq!(
        completion["regional_intervention_assessment"],
        result["regional_intervention_assessment"]
    );
    assert!(completion.get("trial_appearance_comparison").is_none());
    // The completion, history reader and planner feed the same published state.
    crate::runtime_test_review::publish(&mut engine, Some(("observe_runtime_test", &completion)));
    assert_eq!(
        review_snapshot(&engine, &test)["assessment"],
        result["regional_intervention_assessment"]
    );
    let mut mismatched = result.clone();
    mismatched["regional_intervention_assessment"]["test_sha256"] = json!("f".repeat(64));
    crate::runtime_test_review::publish(&mut engine, Some(("read_runtime_test", &mismatched)));
    assert!(review_snapshot(&engine, &test)["assessment"].is_null());
    assert!(review_snapshot(&engine, &test)["error"].is_string());
    crate::runtime_test_review::publish(&mut engine, Some(("read_runtime_test", &result)));
    assert_eq!(
        review_snapshot(&engine, &test)["assessment"],
        result["regional_intervention_assessment"]
    );
    let failure = json!({"test":{"id":test.id},"regional_intervention_assessment":null,"trial_appearance_error":"Unavailable"});
    crate::runtime_test_review::publish(&mut engine, Some(("read_runtime_test", &failure)));
    assert_eq!(review_snapshot(&engine, &test)["error"], "Unavailable");
    assert!(review_snapshot(&engine, &test)["assessment"].is_null());
    // No unrelated command can inject a comparison result into status.
    let old = review_snapshot(&engine, &test);
    crate::runtime_test_review::publish(&mut engine, Some(("submit_observation", &result)));
    assert_eq!(review_snapshot(&engine, &test), old);
    assert_regional_history(
        &mut engine,
        &test,
        &result["regional_intervention_assessment"],
    );
    assert_eq!(result["test"], json!(test));
    assert!(
        !engine
            .runtime_tests
            .get(&test.id)
            .unwrap()
            .candidate
            .edit_authority
    );
    engine.save().unwrap();
    drop(engine);
    let mut engine = Engine::load(state_path).unwrap();
    // Restart does not trust a persisted display snapshot as fresh evidence.
    assert!(review_snapshot(&engine, &test).is_null());
    assert_eq!(read(&mut session, &mut engine), result);
    assert_regional_history(
        &mut engine,
        &test,
        &result["regional_intervention_assessment"],
    );

    engine.runtime_trials.records.clear();
    crate::runtime_test_review::publish(&mut engine, None);
    assert!(review_snapshot(&engine, &test).is_null());
    let missing = read(&mut session, &mut engine);
    assert_eq!(
        missing["regional_intervention_assessment"]["status"],
        "inconclusive"
    );
    assert_eq!(
        missing["trial_appearance_comparison"],
        result["trial_appearance_comparison"]
    );
    assert_regional_history(
        &mut engine,
        &test,
        &missing["regional_intervention_assessment"],
    );
    engine.runtime_trials.records.push(restored.clone());
    crate::runtime_test_review::publish(&mut engine, Some(("read_runtime_test", &result)));
    assert_eq!(
        review_snapshot(&engine, &test)["assessment"],
        result["regional_intervention_assessment"]
    );
    let mut wrong = test.clone();
    wrong.conditions[1].runtime_evidence = Some(reading(&patch.expected));
    engine.runtime_tests.record(wrong).unwrap();
    crate::runtime_test_review::publish(&mut engine, None);
    assert!(review_snapshot(&engine, &test).is_null());
    let changed = read(&mut session, &mut engine);
    assert_eq!(
        changed["trial_appearance_comparison"]["status"],
        "appearance_displacement_changed_and_returned"
    );
    assert_eq!(
        changed["regional_intervention_assessment"]["status"],
        "inconclusive"
    );
    engine.runtime_tests.record(test.clone()).unwrap();

    let frame = directory
        .join("runtime-tests")
        .join(&test.id)
        .join("1-2.png");
    let original = std::fs::read(&frame).unwrap();
    std::fs::write(&frame, b"corrupt").unwrap();
    let corrupt = read(&mut session, &mut engine);
    assert_eq!(
        corrupt["regional_intervention_assessment"]["status"],
        "inconclusive"
    );
    assert!(corrupt["regional_intervention_assessment"]["intervention_error"].is_null());
    assert_regional_history(
        &mut engine,
        &test,
        &corrupt["regional_intervention_assessment"],
    );
    std::fs::write(&frame, original).unwrap();

    // An unchanged image pattern must remain negative with the same receipts.
    let mut similar = test.clone();
    for i in 0..6 {
        let bytes = engine.runtime_tests.read_image(&test, 0, i).unwrap();
        std::fs::write(
            directory
                .join("runtime-tests")
                .join(&test.id)
                .join(format!("1-{i}.png")),
            &bytes,
        )
        .unwrap();
        similar.conditions[1].frames[i].sha256 = reversebolt_core::visual::digest(&bytes);
        similar.conditions[1].frames[i].difference = test.conditions[0].frames[i].difference;
    }
    engine.runtime_tests.record(similar.clone()).unwrap();
    let negative = read(&mut session, &mut engine);
    assert_eq!(
        negative["regional_intervention_assessment"]["status"],
        "regional_change_not_observed"
    );
    assert_regional_history(
        &mut engine,
        &similar,
        &negative["regional_intervention_assessment"],
    );
    let report = reversebolt_vision::trial_appearance::analyze(
        &similar,
        similar.appearance.as_ref().unwrap().selection.clone(),
        |c, i| engine.runtime_tests.read_image(&similar, c, i),
        |c, i| engine.runtime_tests.read_input(&similar, c, i),
    )
    .unwrap();
    assert!(
        reversebolt_vision::trial_intervention::assess(&test, Some(&restored), &report).is_err()
    );
    assert!(!similar.candidate.edit_authority);
    drop(engine);
    assert!(directory.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    assert!(
        directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-trial-motion-")
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn small_local_baseline_reaches_apply_with_an_automatic_immutable_choice() {
    let (directory, state_path, mut engine, complete) = motion_fixture(true);
    let mut test = collect_saved_baseline(&engine, &complete);
    assert!(test.conditions[0].median().unwrap() < 5.);
    assert_eq!(test.phase, Phase::BaselineReview);
    assert!(test.appearance.is_none());
    let baseline_before = json!(test.conditions[0]);
    review_baseline(&mut engine, &mut test, &AtomicBool::new(false)).unwrap();
    assert_eq!(test.phase, Phase::ReadyToApply);
    assert_eq!(json!(test.conditions[0]), baseline_before);
    let plan = test.appearance.clone().unwrap();
    assert_eq!(
        engine.runtime_tests.get(&test.id).unwrap().appearance,
        Some(plan.clone())
    );
    assert_eq!(
        plan.selection.baseline_sha256,
        test.conditions[0].frames[0].sha256
    );
    engine.runtime_tests.record(test.clone()).unwrap();
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    let read = Command {
        action: "read_runtime_test".into(),
        source: "agent".into(),
        args: json!({"test_id":test.id}),
    };
    let initial = session
        .command(&mut engine, &read, &directory, &published)
        .unwrap();
    assert_eq!(initial["test"]["appearance"], json!(plan));
    assert_eq!(
        initial["trial_appearance_comparison"]["status"],
        "incomplete"
    );
    assert!(engine.runtime_trials.records.is_empty());
    assert!(!test.candidate.edit_authority);
    // Generated later evidence exercises the chosen region; no native edit/input.
    test.conditions[1..].clone_from_slice(&complete.conditions[1..]);
    test.stop("Synthetic local response history.");
    engine.runtime_tests.record(test.clone()).unwrap();
    let compared = session
        .command(&mut engine, &read, &directory, &published)
        .unwrap();
    assert_eq!(
        compared["trial_appearance_comparison"]["status"],
        "appearance_displacement_changed_and_returned"
    );
    assert!(compared["test"]["assessment"].is_null());
    drop(engine);
    let mut engine = Engine::load(state_path).unwrap();
    assert_eq!(
        session
            .command(&mut engine, &read, &directory, &published)
            .unwrap(),
        compared
    );
    let mut erased = engine.runtime_tests.get(&test.id).unwrap().clone();
    erased.appearance = None;
    assert!(engine.runtime_tests.record(erased).is_err());
    drop(engine);
    assert!(directory.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    assert!(
        directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-trial-motion-")
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn regional_baseline_review_rejects_missing_evidence_and_honors_stop_and_restart() {
    let (directory, state_path, mut engine, complete) = motion_fixture(true);
    let baseline = collect_saved_baseline(&engine, &complete);
    assert_eq!(baseline.phase, Phase::BaselineReview);
    let mut stopped = baseline.clone();
    assert!(review_baseline(&mut engine, &mut stopped, &AtomicBool::new(true)).is_err());
    assert!(stopped.appearance.is_none());
    assert!(engine.runtime_tests.records.is_empty());
    // A persisted review is interrupted on restart, never automatically resumed.
    engine.runtime_tests.record(baseline.clone()).unwrap();
    drop(engine);
    let mut engine = Engine::load(state_path).unwrap();
    assert_eq!(
        engine.runtime_tests.get(&baseline.id).unwrap().phase,
        Phase::Stopped
    );
    assert!(
        engine
            .runtime_tests
            .get(&baseline.id)
            .unwrap()
            .appearance
            .is_none()
    );
    let mut stale = baseline;
    assert!(review_baseline(&mut engine, &mut stale, &AtomicBool::new(false)).is_err());
    assert!(stale.appearance.is_none());
    drop(engine);
    assert!(directory.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    assert!(
        directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-trial-motion-")
    );
    std::fs::remove_dir_all(directory).unwrap();

    let (directory, _, mut engine, complete) = motion_fixture(true);
    let mut unavailable = collect_saved_baseline(&engine, &complete);
    std::fs::write(
        directory
            .join("runtime-tests")
            .join(&complete.id)
            .join("0-3.png"),
        b"corrupt",
    )
    .unwrap();
    review_baseline(&mut engine, &mut unavailable, &AtomicBool::new(false)).unwrap();
    assert_eq!(unavailable.phase, Phase::Stopped);
    assert!(unavailable.appearance.is_none());
    assert!(engine.runtime_trials.records.is_empty());
    drop(engine);
    assert!(directory.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    assert!(
        directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-trial-motion-")
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn regional_baseline_does_not_admit_static_scenes_or_a_prepared_edit() {
    for prepared in [false, true] {
        let (directory, _, mut engine, mut complete) = motion_fixture(true);
        if !prepared {
            let first = engine.runtime_tests.read_image(&complete, 0, 0).unwrap();
            for i in 1..6 {
                std::fs::write(
                    directory
                        .join("runtime-tests")
                        .join(&complete.id)
                        .join(format!("0-{i}.png")),
                    &first,
                )
                .unwrap();
                complete.conditions[0].frames[i].sha256 = reversebolt_core::visual::digest(&first);
            }
        }
        let mut baseline = collect_saved_baseline(&engine, &complete);
        assert_eq!(baseline.phase, Phase::BaselineReview);
        if prepared {
            use reversebolt_core::runtime_trials::{Context, Evidence, Trial};
            let patch = Patch {
                path: baseline.candidate.locator.clone(),
                type_code: 8,
                expected: 8i32.to_le_bytes().to_vec(),
                replacement: 0i32.to_le_bytes().to_vec(),
                field_address: 0x2000,
                object_addresses: vec![0x1000],
                prepared_unix_ms: now_ms(),
            };
            let evidence = Evidence::new("fixture.json".into(), json!({"operation":"refresh_runtime_paths","scope":baseline.scope,
                "checks":[{"path_id":patch.path.id(),"status":"read","type_code":8,"bytes":patch.expected,"value":8,
                "field_address":"0x2000","snapshot_object_addresses":["0x1000"]}]})).unwrap();
            let mut trial = Trial::prepare(
                Context {
                    investigation: baseline.investigation.clone(),
                    catalog_id: baseline.catalog_id.clone(),
                    revision: 1,
                    process: baseline.scope.clone(),
                },
                "Fixture".into(),
                "agent".into(),
                patch,
                evidence,
            )
            .unwrap();
            trial.id = baseline.change_id.clone();
            engine.runtime_trials.records.push(trial); // No native operation or trial file.
            assert!(review_baseline(&mut engine, &mut baseline, &AtomicBool::new(false)).is_err());
            assert_eq!(baseline.phase, Phase::BaselineReview);
        } else {
            review_baseline(&mut engine, &mut baseline, &AtomicBool::new(false)).unwrap();
            assert_eq!(baseline.phase, Phase::Stopped);
        }
        assert!(baseline.appearance.is_none());
        assert!(!baseline.candidate.edit_authority);
        drop(engine);
        assert!(directory.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            directory
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("reversebolt-trial-motion-")
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn retained_trial_motion_checks_images_inputs_and_recovery_after_restart() {
    let (directory, state_path, mut engine, test) = motion_fixture(false);
    let folder = directory.join("runtime-tests").join(&test.id);
    engine.runtime_tests.record(test.clone()).unwrap();
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    let request = Command {
        action: "read_runtime_test".into(),
        source: "agent".into(),
        args: json!({"test_id":test.id,"include_motion":true,"appearance":{"baseline_sha256":test.conditions[0].frames[0].sha256,"region":{"x":64,"y":40,"width":64,"height":64}}}),
    };
    let read = session
        .command(&mut engine, &request, &directory, &published)
        .unwrap();
    assert_eq!(read["test"], json!(test));
    assert!(read["test"]["assessment"].is_null());
    assert!(read.get("png_base64").is_none());
    assert_eq!(
        read["trial_motion_comparison"]["status"],
        "displacement_changed_and_returned"
    );
    for i in 0..4 {
        let x = if i % 2 == 0 { 8 } else { -8 };
        assert_eq!(
            read["trial_motion_comparison"]["actions"][i]["displacement"],
            json!([[x, 0], [0, 0], [x, 0]])
        );
    }
    assert_eq!(
        read["trial_appearance_comparison"]["status"],
        "appearance_displacement_changed_and_returned"
    );
    assert_eq!(read["trial_appearance_comparison"]["attempted_frames"], 18);
    checkpoint(&mut engine, &published).unwrap();
    drop(engine);
    let mut engine = Engine::load(state_path).unwrap();
    assert_eq!(
        session
            .command(&mut engine, &request, &directory, &published)
            .unwrap(),
        read
    );
    let input_path = folder.join("1-2.input.json");
    let original_input = std::fs::read(&input_path).unwrap();
    std::fs::write(&input_path, b"null").unwrap();
    let bad_input = session
        .command(&mut engine, &request, &directory, &published)
        .unwrap();
    assert_eq!(
        bad_input["trial_motion_comparison"]["status"],
        "unavailable_evidence"
    );
    assert_eq!(
        bad_input["trial_appearance_comparison"]["status"],
        "unavailable_evidence"
    );
    assert!(
        bad_input["trial_motion_comparison"]["conditions"][1]["pairs"][1]["input_error"]
            .is_string()
    );
    std::fs::write(&input_path, original_input).unwrap();
    std::fs::write(folder.join("2-2.png"), b"broken").unwrap();
    let bad_image = session
        .command(&mut engine, &request, &directory, &published)
        .unwrap();
    assert_eq!(
        bad_image["trial_motion_comparison"]["status"],
        "unavailable_evidence"
    );
    assert_eq!(
        bad_image["trial_appearance_comparison"]["status"],
        "unavailable_evidence"
    );
    assert!(bad_image["trial_motion_comparison"]["conditions"][2]["pairs"][1]["motion"].is_null());
    assert!(bad_image["trial_motion_comparison"]["conditions"][2]["pairs"][2]["motion"].is_null());
    assert!(
        bad_image["trial_motion_comparison"]["conditions"][0]["pairs"][1]["motion"].is_object()
    );
    assert_eq!(bad_image["test"], json!(test));
    let mut light = request.clone();
    light.args["include_motion"] = json!(false);
    light.args.as_object_mut().unwrap().remove("appearance");
    assert!(
        session
            .command(&mut engine, &light, &directory, &published)
            .unwrap()
            .get("trial_motion_comparison")
            .is_none()
    );
    light.args["include_motion"] = json!("true");
    assert!(
        session
            .command(&mut engine, &light, &directory, &published)
            .unwrap_err()
            .contains("true or false")
    );
    drop(engine);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn locked_appearance_survives_history_and_cannot_be_added_after_an_edit_record() {
    use reversebolt_core::{
        changes::Phase as ChangePhase,
        runtime_tests::AppearanceSelection,
        runtime_trials::{Context, Evidence, Trial},
    };
    let (directory, state_path, mut engine, complete) = motion_fixture(true);
    let mut baseline = complete.clone();
    baseline.conditions[1..].fill(Default::default());
    baseline.phase = Phase::ReadyToApply;
    engine.runtime_tests.record(baseline.clone()).unwrap();
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    let offered = session
        .command(
            &mut engine,
            &Command {
                action: "read_runtime_test".into(),
                source: "agent".into(),
                args: json!({"test_id":baseline.id}),
            },
            &directory,
            &published,
        )
        .unwrap();
    let selection: AppearanceSelection =
        serde_json::from_value(offered["appearance_suggestions"]["recommended"].clone()).unwrap();
    assert_eq!(
        offered["appearance_suggestions"]["policy"],
        reversebolt_vision::trial_appearance_suggestions::POLICY
    );
    assert_eq!(
        selection.baseline_sha256,
        baseline.conditions[0].frames[0].sha256
    );
    assert!(
        engine
            .runtime_tests
            .get(&baseline.id)
            .unwrap()
            .appearance
            .is_none()
    );
    let request = Command {
        action: "lock_runtime_test_appearance".into(),
        source: "agent".into(),
        args: json!({"test_id":baseline.id,"after_revision":baseline.revision,"appearance":selection}),
    };
    session.before(&mut engine, &request).unwrap();
    assert_eq!(
        engine.runtime_tests.get(&baseline.id).unwrap().phase,
        Phase::ReadyToApply
    );
    for (actor, revision) in [
        ("other-agent", baseline.revision),
        ("agent", baseline.revision - 1),
    ] {
        let mut bad = request.clone();
        bad.source = actor.into();
        bad.args["after_revision"] = json!(revision);
        assert!(
            session
                .command(&mut engine, &bad, &directory, &published)
                .is_err()
        );
        assert!(
            engine
                .runtime_tests
                .get(&baseline.id)
                .unwrap()
                .appearance
                .is_none()
        );
    }
    // Even a prepared record (no write attempt yet) prevents a new selection.
    let patch = Patch {
        path: baseline.candidate.locator.clone(),
        type_code: 8,
        expected: 8i32.to_le_bytes().to_vec(),
        replacement: 0i32.to_le_bytes().to_vec(),
        field_address: 0x2000,
        object_addresses: vec![0x1000],
        prepared_unix_ms: now_ms(),
    };
    let evidence=Evidence::new("fixture.json".into(),json!({"operation":"refresh_runtime_paths","scope":baseline.scope,
        "checks":[{"path_id":patch.path.id(),"status":"read","type_code":8,"bytes":patch.expected,"value":8,
        "field_address":"0x2000","snapshot_object_addresses":["0x1000"]}]})).unwrap();
    let mut trial = Trial::prepare(
        Context {
            investigation: baseline.investigation.clone(),
            catalog_id: baseline.catalog_id.clone(),
            revision: 1,
            process: baseline.scope.clone(),
        },
        "Fixture".into(),
        "agent".into(),
        patch,
        evidence,
    )
    .unwrap();
    trial.id = baseline.change_id.clone();
    assert_eq!(trial.phase, ChangePhase::Prepared);
    engine.runtime_trials.records.push(trial);
    assert!(
        session
            .command(&mut engine, &request, &directory, &published)
            .unwrap_err()
            .contains("before preparing")
    );
    assert!(
        engine
            .runtime_tests
            .get(&baseline.id)
            .unwrap()
            .appearance
            .is_none()
    );
    engine.runtime_trials.records.clear(); // Test fixture only: no trial file or operation was created.
    let receipt = session
        .command(&mut engine, &request, &directory, &published)
        .unwrap();
    assert_eq!(receipt["replayed"], false);
    let locked = engine.runtime_tests.get(&baseline.id).unwrap().clone();
    assert_eq!(locked.appearance.as_ref().unwrap().selection, selection);
    let bytes_before = std::fs::read(
        directory
            .join("runtime-tests")
            .join(format!("{}.json", baseline.id)),
    )
    .unwrap();
    let replay = session
        .command(&mut engine, &request, &directory, &published)
        .unwrap();
    assert_eq!(replay["appearance"], receipt["appearance"]);
    assert_eq!(replay["replayed"], true);
    assert_eq!(
        std::fs::read(
            directory
                .join("runtime-tests")
                .join(format!("{}.json", baseline.id))
        )
        .unwrap(),
        bytes_before
    );
    let mut changed = request.clone();
    changed.args["appearance"]["region"]["x"] = json!(selection.region.x + 1);
    assert!(
        session
            .command(&mut engine, &changed, &directory, &published)
            .is_err()
    );
    let mut erased = locked.clone();
    erased.appearance = None;
    assert!(engine.runtime_tests.record(erased).is_err());
    // Retain generated later observations without native input or a field write.
    let mut finished = locked;
    finished.conditions = complete.conditions.clone();
    finished.stop("Synthetic retained comparison.");
    engine.runtime_tests.record(finished.clone()).unwrap();
    checkpoint(&mut engine, &published).unwrap();
    let read = Command {
        action: "read_runtime_test".into(),
        source: "agent".into(),
        args: json!({"test_id":baseline.id}),
    };
    let compared = session
        .command(&mut engine, &read, &directory, &published)
        .unwrap();
    assert_eq!(
        compared["trial_appearance_comparison"]["status"],
        "appearance_displacement_changed_and_returned"
    );
    assert_eq!(
        compared["trial_appearance_comparison"]["retained_plan"],
        receipt["appearance"]
    );
    assert!(compared["test"]["assessment"].is_null());
    assert!(compared.get("png_base64").is_none());
    let mut conflict = read.clone();
    conflict.args["appearance"] = changed.args["appearance"].clone();
    assert!(
        session
            .command(&mut engine, &conflict, &directory, &published)
            .is_err()
    );
    drop(engine);
    let mut engine = Engine::load(state_path).unwrap();
    assert_eq!(
        session
            .command(&mut engine, &read, &directory, &published)
            .unwrap(),
        compared
    );
    assert_eq!(
        session
            .command(&mut engine, &request, &directory, &published)
            .unwrap()["appearance"],
        receipt["appearance"]
    );
    std::fs::write(
        directory
            .join("runtime-tests")
            .join(&baseline.id)
            .join("0-0.png"),
        b"corrupted",
    )
    .unwrap();
    let unavailable = session
        .command(&mut engine, &read, &directory, &published)
        .unwrap();
    assert_eq!(unavailable["test"], json!(finished));
    assert!(unavailable["trial_appearance_comparison"].is_null());
    assert!(unavailable["trial_appearance_error"].is_string());
    drop(engine);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn local_appearance_comparison_distinguishes_small_subject_from_static_background() {
    use reversebolt_core::{visual, visual_regions::Region};
    use reversebolt_vision::trial_appearance::{self, Selection};
    let (directory, _, mut engine, test) = motion_fixture(true);
    engine.runtime_tests.record(test.clone()).unwrap();
    let selection = Selection {
        baseline_sha256: test.conditions[0].frames[0].sha256.clone(),
        region: Region {
            x: 64,
            y: 80,
            width: 64,
            height: 64,
        },
    };
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    let request = Command {
        action: "read_runtime_test".into(),
        source: "agent".into(),
        args: json!({"test_id":test.id,"include_motion":true,"appearance":selection}),
    };
    let result = session
        .command(&mut engine, &request, &directory, &published)
        .unwrap();
    assert_eq!(
        result["trial_motion_comparison"]["status"],
        "insufficient_support"
    );
    assert_eq!(
        result["trial_appearance_comparison"]["status"],
        "appearance_displacement_changed_and_returned"
    );
    assert_eq!(result["test"], json!(test));
    assert!(result.get("png_base64").is_none());
    for i in 0..4 {
        let x = if i % 2 == 0 { 16 } else { -16 };
        assert_eq!(
            result["trial_appearance_comparison"]["actions"][i]["displacement"],
            json!([[x, 0], [0, 0], [x, 0]])
        );
        assert!(
            result["trial_motion_comparison"]["actions"][i]["displacement"]
                .as_array()
                .unwrap()
                .iter()
                .all(|d| d.is_null() || *d == json!([0, 0]))
        );
    }
    let mut bad = selection.clone();
    bad.baseline_sha256 = "a".repeat(64);
    assert!(
        trial_appearance::analyze(
            &test,
            bad,
            |_, _| panic!("stale selection must not read"),
            |_, _| panic!("stale selection must not read")
        )
        .is_err()
    );

    // One failed recovery action must not be pooled with the other three.
    let baseline = engine.runtime_tests.read_image(&test, 0, 0).unwrap();
    let mut divergent = test.clone();
    divergent.conditions[2].frames[2].sha256 = visual::digest(&baseline);
    let failed_return = trial_appearance::analyze(
        &divergent,
        selection.clone(),
        |c, i| {
            if c == 2 && i == 2 {
                Ok(baseline.clone())
            } else {
                engine.runtime_tests.read_image(&test, c, i)
            }
        },
        |c, i| engine.runtime_tests.read_input(&test, c, i),
    )
    .unwrap();
    assert_eq!(failed_return.status, "mixed_appearance_displacement");
    assert_eq!(failed_return.actions[0].returned, Some(false));

    // A moving idle interval is a confound even when later actions can be matched.
    let displaced = engine.runtime_tests.read_image(&test, 0, 2).unwrap();
    let mut idle = test.clone();
    idle.conditions[0].frames[1].sha256 = visual::digest(&displaced);
    let idle_motion = trial_appearance::analyze(
        &idle,
        selection.clone(),
        |c, i| {
            if c == 0 && i == 1 {
                Ok(displaced.clone())
            } else {
                engine.runtime_tests.read_image(&test, c, i)
            }
        },
        |c, i| engine.runtime_tests.read_input(&test, c, i),
    )
    .unwrap();
    assert_eq!(idle_motion.status, "idle_displacement");

    // Valid windows from different monitor generations must not be pooled.
    let mut restarted = test.clone();
    let mut windows = (0..3)
        .map(|c| {
            (0..6)
                .map(|i| engine.runtime_tests.read_input(&test, c, i).unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (i, window) in windows[1].iter_mut().enumerate().skip(1) {
        window.as_mut().unwrap().generation = "different-monitor".into();
        let before_end = restarted.conditions[1].frames[i - 1].interval.end_us;
        let frame = &mut restarted.conditions[1].frames[i];
        frame.input_sha256 = visual::digest(&serde_json::to_vec(&window).unwrap());
        frame.input = InputWitness::check(
            window.as_ref(),
            i.checked_sub(2).and_then(|a| test.actions.get(a)),
            &frame.interval,
            before_end,
        );
        assert!(frame.input.matches);
    }
    let mixed_monitors = trial_appearance::analyze(
        &restarted,
        selection.clone(),
        |c, i| engine.runtime_tests.read_image(&test, c, i),
        |c, i| Ok(windows[c][i].clone()),
    )
    .unwrap();
    assert_eq!(mixed_monitors.status, "input_mismatch");

    // Erasing the subject once must stop this condition, even when it reappears.
    let blank = image::GrayImage::from_pixel(512, 288, image::Luma([20]));
    let mut output = std::io::Cursor::new(vec![]);
    blank
        .write_to(&mut output, image::ImageFormat::Png)
        .unwrap();
    let missing = output.into_inner();
    let mut interrupted = test.clone();
    interrupted.conditions[1].frames[2].sha256 = visual::digest(&missing);
    let failed_match = trial_appearance::analyze(
        &interrupted,
        selection.clone(),
        |c, i| {
            if c == 1 && i == 2 {
                Ok(missing.clone())
            } else {
                engine.runtime_tests.read_image(&test, c, i)
            }
        },
        |c, i| engine.runtime_tests.read_input(&test, c, i),
    )
    .unwrap();
    assert_eq!(failed_match.status, "unresolved_appearance");
    assert_eq!(failed_match.attempted_frames, 15);
    assert!(
        failed_match.conditions[1].frames[3..]
            .iter()
            .all(|f| f.stopped && f.measurement.is_none())
    );
    assert_eq!(
        failed_match.conditions[2].pairs[1].displacement,
        Some([16, 0])
    );

    let mut incomplete = test.clone();
    incomplete.conditions[2].frames.pop();
    let partial = trial_appearance::analyze(
        &incomplete,
        selection,
        |c, i| engine.runtime_tests.read_image(&test, c, i),
        |c, i| engine.runtime_tests.read_input(&test, c, i),
    )
    .unwrap();
    assert_eq!(partial.status, "incomplete");
    drop(engine);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn appearance_suggestions_are_automatic_baseline_only_and_survive_history_reads() {
    use reversebolt_vision::trial_appearance_suggestions as suggestions;
    let (directory, state_path, mut engine, test) = motion_fixture(true);
    let report = suggestions::analyze(
        &test,
        |c, i| {
            assert_eq!(c, 0);
            engine.runtime_tests.read_image(&test, c, i)
        },
        |c, i| {
            assert_eq!(c, 0);
            engine.runtime_tests.read_input(&test, c, i)
        },
    )
    .unwrap();
    assert_eq!(report.status, "suggestions_available");
    assert!(report.suggestions.len() <= 4 && report.attempted_frames <= 24);
    let chosen = report
        .suggestions
        .iter()
        .find(|s| Some(&s.selection) == report.recommended.as_ref())
        .unwrap();
    assert_eq!(
        chosen.displacement,
        [
            Some([0, 0]),
            Some([16, 0]),
            Some([-16, 0]),
            Some([16, 0]),
            Some([-16, 0])
        ]
    );
    // Later results are deliberately invalid and must not even be consulted.
    let mut later = test.clone();
    for condition in &mut later.conditions[1..] {
        for frame in &mut condition.frames {
            frame.path = "invalid".into();
            frame.sha256.clear();
        }
    }
    let untouched = suggestions::analyze(
        &later,
        |c, i| {
            assert_eq!(c, 0);
            engine.runtime_tests.read_image(&test, c, i)
        },
        |c, i| {
            assert_eq!(c, 0);
            engine.runtime_tests.read_input(&test, c, i)
        },
    )
    .unwrap();
    assert_eq!(json!(untouched), json!(report));

    let mut baseline = test.clone();
    baseline.conditions[1..].fill_with(Default::default);
    baseline.phase = Phase::ReadyToApply;
    baseline.assessment = None;
    engine.runtime_tests.record(baseline.clone()).unwrap();
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    let mut request = Command {
        action: "read_runtime_test".into(),
        source: "agent".into(),
        args: json!({"test_id":test.id}),
    };
    let automatic = session
        .command(&mut engine, &request, &directory, &published)
        .unwrap();
    assert_eq!(automatic["appearance_suggestions"], json!(report));
    assert_eq!(automatic["test"], json!(baseline));
    assert!(automatic.get("png_base64").is_none());
    assert!(engine.runtime_trials.records.is_empty());
    assert!(
        engine
            .runtime_tests
            .get(&test.id)
            .unwrap()
            .appearance
            .is_none()
    );
    request.args["suggest_appearance"] = json!(false);
    assert!(
        session
            .command(&mut engine, &request, &directory, &published)
            .unwrap()
            .get("appearance_suggestions")
            .is_none()
    );
    request.args["suggest_appearance"] = json!("yes");
    assert!(
        session
            .command(&mut engine, &request, &directory, &published)
            .is_err()
    );
    drop(engine);
    let mut reopened = Engine::load(state_path).unwrap();
    request.args["suggest_appearance"] = json!(true);
    let historical = session
        .command(&mut reopened, &request, &directory, &published)
        .unwrap();
    assert_eq!(historical["appearance_suggestions"], json!(report));
    assert_eq!(historical["test"]["phase"], "stopped");
    drop(reopened);
    assert!(
        directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-trial-motion-")
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn baseline_suggestions_ignore_flat_flashes_and_measure_the_actual_first_template() {
    use reversebolt_vision::trial_appearance_suggestions as suggestions;
    let (directory, _, engine, mut test) = motion_fixture(true);
    let folder = directory.join("runtime-tests").join(&test.id);
    for i in 0..6 {
        let mut image =
            image::load_from_memory(&engine.runtime_tests.read_image(&test, 0, i).unwrap())
                .unwrap()
                .to_luma8();
        for y in 0..288 {
            for x in 256..512 {
                image[(x, y)] = image::Luma([if i >= 2 && i % 2 == 0 { 240 } else { 20 }]);
            }
        }
        if i == 1 {
            for p in image.pixels_mut() {
                p.0[0] = (u16::from(p.0[0]) * 99 / 100) as u8;
            }
        }
        let mut png = std::io::Cursor::new(vec![]);
        image.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let bytes = png.into_inner();
        test.conditions[0].frames[i].sha256 = reversebolt_core::visual::digest(&bytes);
        std::fs::write(folder.join(format!("0-{i}.png")), bytes).unwrap();
    }
    let run = |ranking| {
        suggestions::analyze_with_ranking(
            &test,
            |c, i| {
                assert_eq!(c, 0);
                engine.runtime_tests.read_image(&test, c, i)
            },
            |c, i| {
                assert_eq!(c, 0);
                engine.runtime_tests.read_input(&test, c, i)
            },
            ranking,
        )
        .unwrap()
    };
    let old = run(suggestions::Ranking::ChangeOnly);
    assert_eq!(old.status, "no_supported_region");
    assert!(
        old.suggestions.iter().all(
            |s| s.frames[0].status == reversebolt_vision::region_appearance::Status::LowTexture
        )
    );
    assert!(json!(old).get("selection").is_none());
    let new = run(suggestions::Ranking::StableTexture);
    assert_eq!(new.status, "suggestions_available");
    assert!(new.selection.as_ref().unwrap().rejected_low_texture > 0);
    assert!(new.suggestions.len() <= 4 && new.attempted_frames <= 24);
    let chosen = new
        .suggestions
        .iter()
        .find(|s| Some(&s.selection) == new.recommended.as_ref())
        .unwrap();
    assert_eq!(
        chosen.displacement,
        [
            Some([0, 0]),
            Some([16, 0]),
            Some([-16, 0]),
            Some([16, 0]),
            Some([-16, 0])
        ]
    );
    let quality = chosen.selection_quality.as_ref().unwrap();
    assert!(quality.quiet_mean_absolute_difference > 0.);
    assert_eq!(
        quality.source_std[0],
        chosen.frames[0].forward.as_ref().unwrap().source_std
    );
    assert_ne!(
        quality.source_std[0],
        chosen.frames[1].reverse.as_ref().unwrap().source_std
    );
    assert_eq!(
        chosen.selection.baseline_sha256,
        test.conditions[0].frames[0].sha256
    );
    assert_ne!(
        chosen.selection.baseline_sha256,
        test.conditions[0].frames[1].sha256
    );
    assert_eq!(json!(new), json!(run(suggestions::Ranking::StableTexture)));
    drop(engine);
    assert!(directory.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    assert!(
        directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-trial-motion-")
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn appearance_suggestions_abstain_on_stationary_incomplete_or_corrupt_baselines() {
    use reversebolt_vision::trial_appearance_suggestions as suggestions;
    let (directory, _, engine, test) = motion_fixture(true);
    let stationary = engine.runtime_tests.read_image(&test, 0, 0).unwrap();
    let mut quiet = test.clone();
    for frame in &mut quiet.conditions[0].frames {
        frame.sha256 = reversebolt_core::visual::digest(&stationary);
    }
    let report = suggestions::analyze(
        &quiet,
        |c, _| {
            assert_eq!(c, 0);
            Ok(stationary.clone())
        },
        |c, i| engine.runtime_tests.read_input(&test, c, i),
    )
    .unwrap();
    assert_eq!(report.status, "no_action_regions");
    assert!(report.recommended.is_none() && report.attempted_frames == 0);
    let bad_input = suggestions::analyze(
        &test,
        |c, i| engine.runtime_tests.read_image(&test, c, i),
        |_, _| Ok(None),
    )
    .unwrap();
    assert_eq!(bad_input.status, "unavailable_evidence");
    assert!(bad_input.recommended.is_none() && bad_input.attempted_frames == 0);
    let bad_image = suggestions::analyze(
        &test,
        |c, i| {
            if i == 2 {
                Ok(vec![0])
            } else {
                engine.runtime_tests.read_image(&test, c, i)
            }
        },
        |c, i| engine.runtime_tests.read_input(&test, c, i),
    )
    .unwrap();
    assert_eq!(bad_image.status, "unavailable_evidence");
    assert!(bad_image.recommended.is_none() && bad_image.attempted_frames == 0);
    let mut mixed = test.clone();
    let mut replacement = engine.runtime_tests.read_input(&test, 0, 2).unwrap();
    replacement.as_mut().unwrap().generation = "different-monitor".into();
    mixed.conditions[0].frames[2].input_sha256 =
        reversebolt_core::visual::digest(&serde_json::to_vec(&replacement).unwrap());
    mixed.conditions[0].frames[2].input = InputWitness::check(
        replacement.as_ref(),
        Some(&test.actions[0]),
        &test.conditions[0].frames[2].interval,
        test.conditions[0].frames[1].interval.end_us,
    );
    let report = suggestions::analyze(
        &mixed,
        |c, i| engine.runtime_tests.read_image(&test, c, i),
        |c, i| {
            if i == 2 {
                Ok(replacement.clone())
            } else {
                engine.runtime_tests.read_input(&test, c, i)
            }
        },
    )
    .unwrap();
    assert_eq!(report.status, "input_mismatch");
    assert!(report.recommended.is_none() && report.attempted_frames == 0);

    let mut blank = std::io::Cursor::new(vec![]);
    image::GrayImage::new(512, 288)
        .write_to(&mut blank, image::ImageFormat::Png)
        .unwrap();
    let blank = blank.into_inner();
    let mut missing = test.clone();
    missing.conditions[0].frames[3].sha256 = reversebolt_core::visual::digest(&blank);
    let report = suggestions::analyze(
        &missing,
        |c, i| {
            if i == 3 {
                Ok(blank.clone())
            } else {
                engine.runtime_tests.read_image(&test, c, i)
            }
        },
        |c, i| engine.runtime_tests.read_input(&test, c, i),
    )
    .unwrap();
    assert_eq!(report.status, "no_supported_region");
    assert!(report.recommended.is_none() && report.attempted_frames <= 16);
    assert!(report.suggestions.iter().all(|s| s.frames.len() <= 4));
    let mut partial = test.clone();
    partial.conditions[0].frames.pop();
    let report = suggestions::analyze(
        &partial,
        |_, _| panic!("incomplete must not read pixels"),
        |_, _| panic!("incomplete must not read input"),
    )
    .unwrap();
    assert_eq!(report.status, "incomplete_baseline");
    drop(engine);
    assert!(
        directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-trial-motion-")
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn comparison_capacity_and_old_request_replay_survive_engine_restart() {
    let parent = std::env::temp_dir().canonicalize().unwrap();
    let directory = parent.join(format!("reversebolt-history-command-{}", id()));
    std::fs::create_dir(&directory).unwrap();
    let state = directory.join("state.json");
    let mut engine = Engine::load(state.clone()).unwrap();
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    let investigation = id();
    let first = saved(&investigation);
    engine.runtime_tests.record(first.clone()).unwrap();
    for _ in 1..runtime_tests::MAX_TESTS_PER_INVESTIGATION {
        engine.runtime_tests.record(saved(&investigation)).unwrap();
    }
    let request = prepare(&first);
    assert_eq!(
        session
            .command(&mut engine, &request, &directory, &published)
            .unwrap(),
        first.summary()
    );
    let mut changed = request.clone();
    changed.args["value"] = json!("1");
    assert!(
        session
            .command(&mut engine, &changed, &directory, &published)
            .unwrap_err()
            .contains("different plan")
    );
    let full = prepare(&saved(&investigation));
    assert!(
        session
            .command(&mut engine, &full, &directory, &published)
            .unwrap_err()
            .contains("eight-comparison")
    );
    let next = saved(&id());
    // Capacity permits a new investigation. The next real guard requires a live
    // target; this test deliberately never supplies or launches one.
    assert_eq!(
        session
            .command(&mut engine, &prepare(&next), &directory, &published)
            .unwrap_err(),
        "Connect an application first."
    );
    engine.runtime_tests.record(next.clone()).unwrap();
    checkpoint(&mut engine, &published).unwrap();
    assert_eq!(published.read().unwrap().runtime_tests.len(), 9);
    drop(session);
    drop(engine);
    let mut engine = Engine::load(state).unwrap();
    assert_eq!(engine.view.runtime_tests.len(), 9);
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    assert_eq!(
        session
            .command(&mut engine, &request, &directory, &published)
            .unwrap(),
        first.summary()
    );
    let read = Command {
        action: "read_runtime_test".into(),
        source: "agent".into(),
        args: json!({"test_id":first.id}),
    };
    assert_eq!(
        session
            .command(&mut engine, &read, &directory, &published)
            .unwrap()["test"],
        json!(first)
    );
    assert_eq!(
        engine
            .runtime_tests
            .capacity(&next.investigation)
            .remaining_record_slots,
        7
    );
    drop(session);
    drop(engine);
    let resolved = directory.canonicalize().unwrap();
    assert_eq!(resolved.parent().unwrap(), parent);
    assert!(
        resolved
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-history-command-")
    );
    std::fs::remove_dir_all(resolved).unwrap();
}

#[test]
fn historical_frame_read_exposes_motion_and_isolates_prior_image_failure() {
    let parent = std::env::temp_dir().canonicalize().unwrap();
    let directory = parent.join(format!("reversebolt-motion-read-{}", id()));
    std::fs::create_dir(&directory).unwrap();
    let mut engine = Engine::load(directory.join("state.json")).unwrap();
    let mut test = saved(&id());
    let folder = directory.join("runtime-tests").join(&test.id);
    std::fs::create_dir(&folder).unwrap();
    let mut encoded = std::io::Cursor::new(vec![]);
    image::GrayImage::from_pixel(80, 45, image::Luma([60]))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let bytes = encoded.into_inner();
    for i in 0..2 {
        let interval = reversebolt_core::visual::CaptureInterval {
            clock_id: test.investigation.clone(),
            begin_us: i * 1000000,
            end_us: i * 1000000 + 1000,
        };
        let frame = runtime_tests::Frame {
            path: format!("{}/0-{i}.png", test.id),
            sha256: reversebolt_core::visual::digest(&bytes),
            input_path: format!("{}/0-{i}.input.json", test.id),
            input_sha256: reversebolt_core::visual::digest(b"null"),
            width: 80,
            height: 45,
            input: InputWitness::check(None, None, &interval, interval.begin_us),
            interval,
            started_unix_ms: i as u128,
            completed_unix_ms: i as u128,
            sampling: None,
            difference: if i == 0 { None } else { Some(0.) },
        };
        std::fs::write(folder.join(format!("0-{i}.png")), &bytes).unwrap();
        std::fs::write(folder.join(format!("0-{i}.input.json")), b"null").unwrap();
        test.conditions[0].frames.push(frame);
    }
    engine.runtime_tests.record(test.clone()).unwrap();
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    let request = Command {
        action: "read_runtime_test".into(),
        source: "agent".into(),
        args: json!({"test_id":test.id,"condition":0,"frame":1}),
    };
    let read = session
        .command(&mut engine, &request, &directory, &published)
        .unwrap();
    assert_eq!(read["patch_motion"]["measurement"]["matched_patches"], 0);
    assert!(read["patch_motion"]["measurement"]["consensus"].is_null());
    assert_eq!(
        read["patch_motion"]["before_sha256"],
        test.conditions[0].frames[0].sha256
    );
    assert_eq!(
        read["patch_motion"]["after_sha256"],
        test.conditions[0].frames[1].sha256
    );
    assert_eq!(read["test"], json!(test));
    std::fs::write(folder.join("0-0.png"), b"changed").unwrap();
    let partial = session
        .command(&mut engine, &request, &directory, &published)
        .unwrap();
    assert!(partial["patch_motion"]["measurement"].is_null());
    assert!(partial["patch_motion"]["error"].is_string());
    assert_eq!(partial["png_base64"], read["png_base64"]);
    assert_eq!(partial["test"], json!(test));
    std::fs::write(folder.join("0-1.png"), b"changed").unwrap();
    assert!(
        session
            .command(&mut engine, &request, &directory, &published)
            .is_err()
    );
    drop(session);
    drop(engine);
    let resolved = directory.canonicalize().unwrap();
    assert_eq!(resolved.parent().unwrap(), parent);
    assert!(
        resolved
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-motion-read-")
    );
    std::fs::remove_dir_all(resolved).unwrap();
}

#[test]
fn historical_read_compares_origin_activity_without_changing_the_trial_outcome() {
    use reversebolt_core::{activity_motion as m, play_context as pc, runtime_hypotheses as h};
    let parent = std::env::temp_dir().canonicalize().unwrap();
    let directory = parent.join(format!("reversebolt-origin-motion-{}", id()));
    std::fs::create_dir(&directory).unwrap();
    let state = directory.join("state.json");
    let mut test = saved(&id());
    let frame = |n| pc::Frame {
        id: format!("synthetic-{n}"),
        sha256: format!("{n:064x}"),
        number: n,
        width: 256,
        height: 144,
        interval: reversebolt_core::visual::CaptureInterval {
            clock_id: test.investigation.clone(),
            begin_us: n as u64 * 1_000_000,
            end_us: n as u64 * 1_000_000 + 1000,
        },
    };
    let source = pc::Source {
        investigation_id: test.investigation.clone(),
        batch: 1,
        frame_metadata_sha256: "b".repeat(64),
        key: 68,
        quiet_difference: 0.,
        pairs: [1, 4].map(|n| pc::Pair {
            episode: n,
            before: frame(n),
            after: frame(n + 1),
            input_sha256: "c".repeat(64),
            difference: 20.,
        }),
    };
    let mut context = pc::Context::from_observation(
        source,
        pc::Observation {
            consistent_visible_change: false,
            description: "The image interpretation is uncertain.".into(),
            uncertainty: "No gameplay effect has been established.".into(),
        },
    )
    .unwrap();
    context.motion = Some(m::Summary {
        metric: m::METRIC_V2.into(),
        frame_metadata_sha256: context.source.frame_metadata_sha256.clone(),
        measurement_dimensions: [256, 144],
        maximum_selected: 48,
        grid: [6, 4],
        pairs: [0, 1].map(|i| {
            let p = &context.source.pairs[i];
            let delta = [if i == 0 { 18 } else { -18 }, 0];
            m::Pair {
                before_id: p.before.id.clone(),
                before_sha256: p.before.sha256.clone(),
                after_id: p.after.id.clone(),
                after_sha256: p.after.sha256.clone(),
                error: None,
                measurement: Some(m::Measurement {
                    selected: 12,
                    matched_cells: 6,
                    unresolved: Default::default(),
                    matches: [20, 62]
                        .into_iter()
                        .flat_map(|y| {
                            [20, 34, 62, 76, 104, 118].map(|x| m::Match { at: [x, y], delta })
                        })
                        .collect(),
                    consensus: Some(m::Consensus {
                        delta,
                        supporting_patches: 12,
                        supporting_cells: 6,
                    }),
                }),
            }
        }),
    });
    let row = h::Row {
        id: 0,
        type_id: 0,
        field_id: 0,
        path_id: test.candidate.id.clone(),
        type_name: "Actor".into(),
        field_name: "Speed".into(),
        value: test.candidate.value.clone(),
        object_text: vec![],
    };
    test.origin = Some(h::TestOrigin {
        question_source: Default::default(),
        question_policy: None,
        play_context: Some(context),
        reference: h::Reference {
            review_id: id(),
            hypothesis_path_id: row.path_id.clone(),
            selected_path_id: row.path_id.clone(),
        },
        report_sha256: "a".repeat(64),
        hypothesis: h::Hypothesis {
            proposal: h::Proposal {
                candidate: 0,
                name: "Speed".into(),
                explanation: "The label suggests speed; its effect is untested.".into(),
                question: test.hypothesis.clone(),
                citations: vec![h::Citation {
                    source: "field_name".into(),
                    index: None,
                    quote: "Speed".into(),
                }],
                alternatives: vec![],
            },
            candidate: row,
            alternatives: vec![],
        },
    });
    let mut engine = Engine::load(state.clone()).unwrap();
    engine.runtime_tests.record(test.clone()).unwrap();
    drop(engine);
    let mut engine = Engine::load(state).unwrap();
    assert!(engine.live.is_none());
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    let read = session
        .command(
            &mut engine,
            &Command {
                action: "read_runtime_test".into(),
                source: "agent".into(),
                args: json!({"test_id":test.id}),
            },
            &directory,
            &published,
        )
        .unwrap();
    assert_eq!(read["test"], json!(test));
    assert!(read["test"]["assessment"].is_null());
    assert_eq!(
        read["origin_activity_motion_comparison"]["scope"],
        "discovery_activity_pairs"
    );
    assert_eq!(
        read["origin_activity_motion_comparison"]["relation"],
        "opposed_direction"
    );
    assert_eq!(
        read["test"]["origin"]["play_context"]["observation"]["consistent_visible_change"],
        false
    );
    assert_eq!(
        json!(engine.runtime_tests.get(&test.id).unwrap()),
        json!(test)
    );
    drop(session);
    drop(engine);
    let resolved = directory.canonicalize().unwrap();
    assert_eq!(resolved.parent().unwrap(), parent);
    assert!(
        resolved
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-origin-motion-")
    );
    std::fs::remove_dir_all(resolved).unwrap();
}
