use super::*;
#[path = "handoff_tests.rs"]
mod handoff;
#[path = "reuse_tests.rs"]
mod reuse;
use reversebolt_core::{
    inputs::{Buffer, Focus, Poll},
    live::{LiveSession, NativeTarget, ProcessInfo, Region, Screen},
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32},
    },
    time::{Duration, Instant},
};

struct State {
    origin: Mutex<Option<Instant>>,
    pulses: Mutex<Vec<(u64, u64, u8)>>,
    first: AtomicBool,
    generation: AtomicU32,
    shift: AtomicU32,
}
struct Fixture {
    state: Arc<State>,
    info: ProcessInfo,
}
impl NativeTarget for Fixture {
    fn info(&self) -> &ProcessInfo {
        &self.info
    }
    fn regions(&self) -> Result<Vec<Region>> {
        Ok(vec![Region {
            base: 4096,
            length: 4096,
            ..Default::default()
        }])
    }
    fn read(&self, _: u64, _: usize) -> Result<Vec<u8>> {
        panic!("Input investigation must not read target memory")
    }
    fn alive(&self) -> bool {
        true
    }
    fn start_inputs(&self, _: &str, origin: Instant, _: Arc<AtomicBool>) -> Result<()> {
        self.state.origin.lock().unwrap().get_or_insert(origin);
        Ok(())
    }
    fn input_window(&self, begin: u64, end: u64) -> Option<reversebolt_core::inputs::Window> {
        let mut b = Buffer::new(
            "fixture-clock".into(),
            format!(
                "generation-{}",
                self.state.generation.load(Ordering::SeqCst)
            ),
            0,
        );
        let pulses = self.state.pulses.lock().unwrap();
        for t in (begin..end).step_by(10_000) {
            if t + 100 > end {
                break;
            }
            b.push(Poll {
                begin_us: t,
                end_us: t + 100,
                focus: Focus::SelectedForeground,
                down: pulses
                    .iter()
                    .filter(|(a, z, _)| *a <= t && t < *z)
                    .map(|(_, _, k)| *k)
                    .collect(),
            })
            .unwrap();
        }
        b.window(begin, end)
    }
    fn screen(&self, path: &Path) -> Result<Screen> {
        if self.state.first.swap(false, Ordering::SeqCst) {
            let begin = self
                .state
                .origin
                .lock()
                .unwrap()
                .unwrap()
                .elapsed()
                .as_micros() as u64;
            self.state
                .pulses
                .lock()
                .unwrap()
                .push((begin, begin + 60_000, 65));
            std::thread::sleep(Duration::from_millis(80));
        }
        let shift = self.state.shift.load(Ordering::SeqCst);
        image::GrayImage::from_fn(256, 144, |x, y| {
            let mut n = ((x + 256 - shift % 256) % 256 + y * 256 + 17)
                .wrapping_mul(747796405)
                .wrapping_add(2891336453);
            n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
            image::Luma([((n >> 22) ^ n) as u8])
        })
        .save(path)
        .map_err(|e| e.to_string())?;
        Ok(Screen {
            path: path.to_string_lossy().into(),
            width: 256,
            height: 144,
            text: String::new(),
            words: vec![],
            warning: None,
            method: "generated-input-study-fixture".into(),
            sampling: None,
        })
    }
}
fn setup() -> (Engine, Arc<State>, PathBuf, OperationOwner, Plan, String) {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("reversebolt-input-study-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&root).unwrap();
    let state = Arc::new(State {
        origin: Mutex::new(None),
        pulses: Mutex::new(vec![]),
        first: AtomicBool::new(true),
        generation: AtomicU32::new(1),
        shift: AtomicU32::new(0),
    });
    let target = Fixture {
        state: state.clone(),
        info: ProcessInfo {
            pid: 123,
            created: "0123456789ABCDEF".into(),
            session: 1,
            window: 42,
            ..Default::default()
        },
    };
    let mut session = LiveSession::attach(Box::new(target), 1, root.join("captures")).unwrap();
    session.view.investigation_id = "fixture-clock".into();
    let owner = OperationOwner {
        job: 1,
        cancel: Arc::new(AtomicBool::new(false)),
    };
    session.start_inputs(owner.cancel.clone()).unwrap();
    session.capture_mode(CaptureMode::Play).unwrap();
    let plan = Plan::new(&session.view, 65, "fixture-request".into()).unwrap();
    let reference = store(&root.join("runtime"), &plan).unwrap();
    let mut engine = Engine::load(root.join("engine.json")).unwrap();
    engine.view.live = Some(session.view.clone());
    engine.live = Some(session);
    (engine, state, root, owner, plan, reference)
}
fn cleanup(root: PathBuf) {
    assert_eq!(
        root.parent(),
        Some(std::env::temp_dir().canonicalize().unwrap().as_path())
    );
    assert!(
        root.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-input-study-")
    );
    fs::remove_dir_all(root).unwrap();
}
fn execute(state: &State, index: usize) -> Result<()> {
    std::thread::sleep(Duration::from_millis(300));
    if let Some(action) = activity::action(index, 65) {
        let begin = state.origin.lock().unwrap().unwrap().elapsed().as_micros() as u64;
        state
            .pulses
            .lock()
            .unwrap()
            .push((begin, begin + u64::from(action.hold_ms) * 1000, 65));
        std::thread::sleep(Duration::from_millis(u64::from(action.hold_ms)));
        state.shift.fetch_add(8, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(300));
    }
    Ok(())
}
#[test]
fn activity_experiment_generates_four_bounded_responses_without_any_memory_field() {
    let (mut engine, state, root, owner, mut plan, _) = setup();
    let selection = activity::selection::suggest(&engine.live.as_ref().unwrap().view, &[]).unwrap();
    assert_eq!(selection.next.as_ref().unwrap().key, plan.key);
    plan.selection = Some(selection);
    let reference = store(&root.join("runtime"), &plan).unwrap();
    assert!(
        engine
            .live
            .as_ref()
            .unwrap()
            .view
            .runtime
            .catalog_source
            .is_none()
    );
    assert_eq!(
        activity::eligible_keys(&engine.live.as_ref().unwrap().view),
        vec![65]
    );
    for index in 0..activity::STEPS {
        let step = Step {
            activity: reference.clone(),
            index,
            key: 65,
        };
        let result = run_with(
            &mut engine,
            &step,
            &owner,
            &root.join("runtime"),
            |_, _, i, _, _| {
                // Returning to the game may exceed one bounded input window.
                // Only the seed capture itself starts the new response sequence.
                if i == 0 {
                    std::thread::sleep(Duration::from_secs(3));
                }
                execute(&state, i)
            },
        )
        .unwrap();
        assert_eq!(result["success"], true, "{result}");
        assert_eq!(
            result["effect"],
            if activity::action(index, 65).is_some() {
                "Applied"
            } else {
                "NotApplicable"
            }
        );
    }
    let session = engine.live.as_ref().unwrap();
    assert_eq!(session.view.memory_sample_count(), 0);
    let frames: Vec<_> = session
        .view
        .frames
        .iter()
        .filter(|f| f.number >= plan.first_number)
        .collect();
    assert_eq!(frames.len(), 10);
    let seed = &frames[0];
    assert_eq!(
        seed.input_context.as_ref().unwrap().requested_begin_us,
        seed.capture_interval.as_ref().unwrap().begin_us
    );
    assert_eq!(seed.input_context.as_ref().unwrap().omitted_polls, 0);
    assert!(reversebolt_core::action_patterns::exposure(&plan.source, seed).is_err());
    let evidence = crate::play_evidence::read(
        session,
        &json!({"investigation_id":plan.investigation,"batch":plan.batch,
        "frame_ids":frames.iter().map(|f|&f.id).collect::<Vec<_>>(),"auto_regions":false}),
    )
    .unwrap();
    assert_eq!(
        evidence["evidence"]["episodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["bounded_by_quiet"] == true)
            .count(),
        4
    );
    assert!(evidence["activity_source"].is_object());
    let dir = location(&root.join("runtime"), &reference).unwrap();
    assert_eq!(fs::read_dir(dir).unwrap().count(), 21); // plan + ten intents/outcomes
    let review_step = Step {
        activity: reference.clone(),
        index: activity::STEPS,
        key: 65,
    };
    let last_outcome = location(&root.join("runtime"), &reference)
        .unwrap()
        .join("09.outcome.json");
    let original = fs::read(&last_outcome).unwrap();
    let mut invalid: Value = serde_json::from_slice(&original).unwrap();
    invalid["frame_id"] = json!("other");
    fs::write(&last_outcome, serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(
        super::review::run_with(
            &mut engine,
            &review_step,
            &owner,
            &root.join("runtime"),
            |_, _| panic!("Changed completion cannot start review")
        )
        .is_err()
    );
    fs::write(&last_outcome, original).unwrap();
    let published = Arc::new(std::sync::RwLock::new(engine.view.clone()));
    let review = crate::service::native_command(
        &mut engine,
        &reversebolt_core::Command {
            action: "_review_activity".into(),
            args: json!(review_step),
            source: "input-experiment".into(),
        },
        &root.join("captures"),
        Some(&owner),
        &mut crate::observation_pipeline::Pipeline::default(),
        &published,
        &mut crate::runtime_test_bridge::Session::new(Arc::new(AtomicBool::new(false))),
    )
    .unwrap();
    assert_eq!(
        published
            .read()
            .unwrap()
            .live
            .as_ref()
            .unwrap()
            .input_experiments
            .len(),
        1
    );
    let persisted: reversebolt_core::engine::View =
        serde_json::from_slice(&fs::read(root.join("engine.json")).unwrap()).unwrap();
    assert_eq!(
        persisted.live.as_ref().unwrap().input_experiments[0].experiment,
        reference
    );
    let restored = Engine::load(root.join("engine.json")).unwrap();
    assert!(restored.view.live.is_none()); // Restart never reattaches an old target.
    assert_eq!(review["review"]["status"], "measured");
    assert!(review["review"]["context"].is_null());
    let archived = super::review::read_result(&engine, &reference, &root.join("runtime")).unwrap();
    assert_eq!(archived["result"]["input_selection"], json!(plan.selection));
    assert_eq!(
        archived["result"]["measurement"]["frames"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
    assert_eq!(engine.live.as_ref().unwrap().view.sample_count, 11);
    assert_eq!(engine.live.as_ref().unwrap().view.memory_sample_count(), 0);
    assert_eq!(
        super::review::run_with(
            &mut engine,
            &review_step,
            &owner,
            &root.join("runtime"),
            |_, _| panic!("No replayed review")
        )
        .unwrap()["replayed"],
        true
    );
    let followup = root.join("followup");
    fs::create_dir(&followup).unwrap();
    crate::runtime_naming_bridge::prepare_activity(engine.live.as_ref().unwrap(), &followup)
        .unwrap();
    let prepared: Value =
        serde_json::from_slice(&fs::read(followup.join("play-source.json")).unwrap()).unwrap();
    assert_eq!(
        prepared["request"]["frame_ids"],
        review["review"]["frame_ids"]
    );
    let result_path = location(&root.join("runtime"), &reference)
        .unwrap()
        .join("review/result.json");
    fs::write(&result_path, b"{}").unwrap();
    assert!(super::review::read_result(&engine, &reference, &root.join("runtime")).is_err());
    let replay = run_with(
        &mut engine,
        &Step {
            activity: reference,
            index: 8,
            key: 65,
        },
        &owner,
        &root.join("runtime"),
        |_, _, _, _, _| panic!("No repeated input"),
    )
    .unwrap();
    assert_eq!(replay["success"], false);
    assert_eq!(replay["effect"], "NotApplied");
    drop(engine);
    cleanup(root);
}

#[test]
fn activity_review_preserves_negative_interpretation_errors_and_stop_without_controls() {
    for behavior in [
        "positive",
        "negative",
        "failure",
        "cancel",
        "wrong_source",
        "quiet",
    ] {
        let (mut engine, state, root, owner, mut plan, _) = setup();
        plan.observer_model = Some("fixture:model".into());
        let reference = store(&root.join("runtime"), &plan).unwrap();
        for index in 0..activity::STEPS {
            let step = Step {
                activity: reference.clone(),
                index,
                key: 65,
            };
            let result = run_with(
                &mut engine,
                &step,
                &owner,
                &root.join("runtime"),
                |_, _, i, _, _| {
                    execute(&state, i)?;
                    if behavior == "quiet" {
                        state.shift.store(0, Ordering::SeqCst);
                    }
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(result["success"], true, "{result}");
        }
        let step = Step {
            activity: reference.clone(),
            index: activity::STEPS,
            key: 65,
        };
        let cancel = owner.cancel.clone();
        let mut calls = 0;
        let result = super::review::run_with(
            &mut engine,
            &step,
            &owner,
            &root.join("runtime"),
            |model, input| {
                calls += 1;
                assert_eq!(model, "fixture:model");
                if behavior == "failure" {
                    return Err("fixture model unavailable".into());
                }
                if behavior == "cancel" {
                    cancel.store(true, Ordering::SeqCst);
                }
                let mut source = input.source.clone();
                if behavior == "wrong_source" {
                    source.key = 66;
                }
                Ok(reversebolt_observer::runtime_activity::Receipt {
                    source,
                    motion: None,
                    appearances: input.appearances.clone(),
                    regions: input.regions.clone(),
                    stage: None,
                    observation: Some(reversebolt_core::play_context::Observation {
                        consistent_visible_change: behavior == "positive",
                        description:
                            "The generated pairs do not establish a common semantic change.".into(),
                        uncertainty: "Fixture model output, not a real game inference.".into(),
                    }),
                    error: None,
                })
            },
        )
        .unwrap();
        assert_eq!(calls, usize::from(behavior != "quiet"));
        let summary = &result["review"];
        if matches!(behavior, "positive" | "negative") {
            assert_eq!(summary["status"], "interpreted");
            assert_eq!(
                summary["context"]["observation"]["consistent_visible_change"],
                behavior == "positive"
            );
            assert_eq!(
                summary["context"]["interpretation_source"]["model"],
                "fixture:model"
            );
        } else {
            assert!(summary["context"].is_null());
            assert_eq!(
                summary["status"],
                if behavior == "quiet" {
                    "no_separated_response"
                } else if behavior == "cancel" {
                    "cancelled"
                } else {
                    "model_unavailable"
                }
            );
        }
        let live = &engine.live.as_ref().unwrap().view;
        assert!(live.runtime.cards.is_empty());
        assert!(live.visual_tracks.is_empty());
        assert!(live.temporal_tracks.is_empty());
        assert_eq!(live.sample_count, 11);
        assert_eq!(live.memory_sample_count(), 0);
        let saved = super::review::read_result(&engine, &reference, &root.join("runtime")).unwrap();
        assert_eq!(saved["review"], *summary);
        if matches!(behavior, "positive" | "negative") {
            reuse::check(&mut engine, &root, &saved);
        }
        // Losing the publication checkpoint must not cause another inference.
        engine.live.as_mut().unwrap().view.input_experiments.clear();
        owner.cancel.store(false, Ordering::SeqCst);
        assert!(
            super::review::run_with(
                &mut engine,
                &step,
                &owner,
                &root.join("runtime"),
                |_, _| panic!("Review intent is already durable")
            )
            .is_err()
        );
        drop(engine);
        cleanup(root);
    }
}
#[test]
fn activity_experiment_rejects_lost_scope_budget_changed_plan_and_duplicate_intent_before_input() {
    let (mut engine, _state, root, owner, mut plan, reference) = setup();
    let session = engine.live.as_ref().unwrap();
    assert!(Plan::new(&session.view, 66, "other".into()).is_err());
    let mut full = session.view.clone();
    full.sample_count = reversebolt_core::live::MAX_SAMPLES - 9;
    assert!(Plan::new(&full, 65, "other".into()).is_err());
    let mut normal = session.view.clone();
    normal.sample_count = 31;
    assert!(Plan::new(&normal, 65, "fits-after-auto".into()).is_ok());
    full = session.view.clone();
    full.process.created = "other".into();
    assert!(plan.check(&full, 0).is_err());
    plan.key = 66;
    assert!(plan.check(&session.view, 0).is_err());
    let dir = location(&root.join("runtime"), &reference).unwrap();
    save(&dir.join("00.intent.json"), &json!({"existing":true})).unwrap();
    let step = Step {
        activity: reference.clone(),
        index: 0,
        key: 65,
    };
    let result = run_with(
        &mut engine,
        &step,
        &owner,
        &root.join("runtime"),
        |_, _, _, _, _| panic!("Intent already exists"),
    )
    .unwrap();
    assert_eq!(result["success"], false);
    assert!(!dir.join("00.outcome.json").exists());
    fs::write(dir.join("plan.json"), b"{}").unwrap();
    assert!(load(&root.join("runtime"), &reference).is_err());
    drop(engine);
    cleanup(root);
}
#[test]
fn activity_experiment_stops_before_input_when_sampler_restarts() {
    let (mut engine, state, root, owner, _, reference) = setup();
    let first = Step {
        activity: reference.clone(),
        index: 0,
        key: 65,
    };
    assert_eq!(
        run_with(
            &mut engine,
            &first,
            &owner,
            &root.join("runtime"),
            |_, _, i, _, _| execute(&state, i)
        )
        .unwrap()["success"],
        true
    );
    state.generation.store(2, Ordering::SeqCst);
    let next = Step {
        activity: reference,
        index: 1,
        key: 65,
    };
    let result = run_with(
        &mut engine,
        &next,
        &owner,
        &root.join("runtime"),
        |_, _, _, _, _| panic!("Sampler changed before input"),
    )
    .unwrap();
    assert_eq!(result["success"], false);
    assert!(
        result["error"]
            .as_str()
            .unwrap()
            .contains("sampling restarted")
    );
    drop(engine);
    cleanup(root);
}
