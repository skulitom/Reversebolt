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

#[test]
fn retained_trial_motion_checks_images_inputs_and_recovery_after_restart() {
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
    let mut engine = Engine::load(state_path.clone()).unwrap();
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
            let image = image::GrayImage::from_fn(256, 144, |x, y| {
                if x >= shift {
                    *texture.get_pixel(x - shift, y)
                } else {
                    image::Luma([0])
                }
            });
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
                width: 256,
                height: 144,
                interval,
                started_unix_ms: u128::from(start / 1000),
                completed_unix_ms: u128::from(start / 1000 + 20),
                sampling: None,
                difference: (i > 0).then_some(0.),
                input: witness,
            });
        }
    }
    engine.runtime_tests.record(test.clone()).unwrap();
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    let request = Command {
        action: "read_runtime_test".into(),
        source: "agent".into(),
        args: json!({"test_id":test.id,"include_motion":true}),
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
    assert!(bad_image["trial_motion_comparison"]["conditions"][2]["pairs"][1]["motion"].is_null());
    assert!(bad_image["trial_motion_comparison"]["conditions"][2]["pairs"][2]["motion"].is_null());
    assert!(
        bad_image["trial_motion_comparison"]["conditions"][0]["pairs"][1]["motion"].is_object()
    );
    assert_eq!(bad_image["test"], json!(test));
    let mut light = request.clone();
    light.args["include_motion"] = json!(false);
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
