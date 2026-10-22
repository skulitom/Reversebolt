use super::*;
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
    let (mut engine, state, root, owner, plan, reference) = setup();
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
            |_, _, i, _, _| execute(&state, i),
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
