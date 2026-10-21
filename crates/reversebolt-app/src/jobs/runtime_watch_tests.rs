use super::{tests::*, *};
use reversebolt_core::runtime::{self, Task};

pub(super) fn runtime_view() -> EngineView {
    let mut view = initial_view();
    let typ = runtime::TypeKey {
        image: "Fixture.dll".into(),
        type_name: "Actor".into(),
        layout_sha256: "a".repeat(64),
    };
    let locator = runtime::Locator {
        scope: runtime::Scope {
            pid: 77,
            created: "0123456789abcdef".into(),
            session: 2,
        },
        root: runtime::FieldRef {
            declaring: typ.clone(),
            field_id: 0,
        },
        steps: vec![runtime::Step {
            owner: typ.clone(),
            member: runtime::Member::Field {
                field: runtime::FieldRef {
                    declaring: typ,
                    field_id: 1,
                },
            },
        }],
    };
    let candidate = runtime::Candidate {
        id: locator.id(),
        locator,
        name: "Rate".into(),
        description: "Observed field".into(),
        value: json!(50),
        bytes: 50_i32.to_le_bytes().to_vec(),
        type_code: 8,
        snapshot_id: 0,
        node_id: 0,
        edit_authority: false,
        context: Vec::new(),
    };
    view.live.as_mut().unwrap().runtime = runtime::View {
        available: true,
        catalog_id: "catalog".into(),
        cards: vec![runtime::Card {
            value: candidate.value.clone(),
            candidate,
            status: "read".into(),
            observed_unix_ms: 1,
            readings: Vec::new(),
            readings_dropped: 0,
        }],
        ..Default::default()
    };
    view
}

fn args(view: &EngineView, count: usize) -> Value {
    let runtime = &view.live.as_ref().unwrap().runtime;
    json!({"task":{"operation":"refresh","catalog_id":runtime.catalog_id,"after_revision":runtime.revision,
        "ids":[runtime.cards[0].candidate.id]},"count":count,"interval_ms":250})
}

fn tracking_controller() -> (Controller, mpsc::Receiver<Task>) {
    tracking_controller_with_gate(None)
}

fn tracking_controller_with_gate(
    mut gate: Option<mpsc::Receiver<()>>,
) -> (Controller, mpsc::Receiver<Task>) {
    let directory = Arc::new(Mutex::new(PathBuf::new()));
    let worker_directory = directory.clone();
    let (sent, received) = mpsc::channel();
    let engine = EngineHandle::test_worker(runtime_view(), move |command, view| {
        assert_eq!(command.action, "inspect_runtime");
        let task = crate::runtime_bridge::load_task(
            &worker_directory.lock().unwrap(),
            command.args["task_ref"].as_str().unwrap(),
        )
        .unwrap();
        sent.send(task.clone()).unwrap();
        if let Some(gate) = gate.take() {
            gate.recv().unwrap();
        }
        let runtime = &mut view.live.as_mut().unwrap().runtime;
        let candidate = &runtime.cards[0].candidate;
        let value = runtime.revision as i32 + 10;
        let result = json!({"scope":candidate.locator.scope,"edit_authority":false,"checks":[{
            "path_id":candidate.id,"status":"read","type_code":8,"bytes":value.to_le_bytes(),"value":value
        }]});
        let start = runtime.revision as u128 * 100;
        let applied =
            runtime.apply_refresh(&task, &result, [start, start + 1], "fixture/report.json");
        json!({"ok":applied.is_ok(),"error":applied.err(),"state":view,"data":{"runtime":{"cards":1}}})
    });
    let c = controller(engine);
    *directory.lock().unwrap() = c.runtime_directory.clone();
    (c, received)
}

#[test]
fn pausing_an_inflight_runtime_watch_keeps_its_read_and_can_resume_the_pinned_sequence() {
    let (release, wait) = mpsc::channel();
    let (mut c, received) = tracking_controller_with_gate(Some(wait));
    let command = request(&c, "start_runtime_job", args(&runtime_view(), 3));
    let id = c.job_command(&command).unwrap()["ack"]["job"]
        .as_u64()
        .unwrap();
    for _ in 0..3 {
        c.tick();
    }
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    let pause = request(&c, "job_control", json!({"job":id,"operation":"pause"}));
    c.job_command(&pause).unwrap();
    for _ in 0..3 {
        c.tick();
    }
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Pausing);
    assert!(
        !c.executor
            .flight
            .as_ref()
            .unwrap()
            .cancel
            .load(Ordering::SeqCst)
    );
    release.send(()).unwrap();
    for _ in 0..100 {
        drive_change(&mut c);
        if c.jobs.snapshot().jobs[&id].phase == job::Phase::Paused {
            break;
        }
    }
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Paused);
    assert_eq!(
        c.engine
            .view
            .read()
            .unwrap()
            .live
            .as_ref()
            .unwrap()
            .runtime
            .revision,
        1
    );
    let resume = request(&c, "job_control", json!({"job":id,"operation":"resume"}));
    c.job_command(&resume).unwrap();
    for _ in 0..100 {
        drive_change(&mut c);
        if c.jobs.snapshot().jobs[&id].phase.terminal() {
            break;
        }
    }
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Completed);
    assert_eq!(received.try_iter().count(), 2);
    assert_eq!(
        c.engine
            .view
            .read()
            .unwrap()
            .live
            .as_ref()
            .unwrap()
            .runtime
            .revision,
        3
    );
}

#[test]
fn cancelling_a_pausing_runtime_watch_revokes_the_read_and_stops_later_checks() {
    let (release, wait) = mpsc::channel();
    let (mut c, received) = tracking_controller_with_gate(Some(wait));
    let command = request(&c, "start_runtime_job", args(&runtime_view(), 3));
    let id = c.job_command(&command).unwrap()["ack"]["job"]
        .as_u64()
        .unwrap();
    for _ in 0..3 {
        c.tick();
    }
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    let pause = request(&c, "job_control", json!({"job":id,"operation":"pause"}));
    c.job_command(&pause).unwrap();
    c.tick();
    let signal = c.executor.flight.as_ref().unwrap().cancel.clone();
    assert!(!signal.load(Ordering::SeqCst));

    let cancel = request(&c, "job_control", json!({"job":id,"operation":"cancel"}));
    c.job_command(&cancel).unwrap();
    assert!(signal.load(Ordering::SeqCst));
    release.send(()).unwrap();
    for _ in 0..100 {
        drive_change(&mut c);
        if c.jobs.snapshot().jobs[&id].phase.terminal() {
            break;
        }
    }
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Cancelled);
    assert_eq!(received.try_iter().count(), 0);
}

#[test]
fn repeated_runtime_reads_pin_cursors_and_replay_never_repeats_completed_work() {
    let (mut c, received) = tracking_controller();
    let command = request(&c, "start_runtime_job", args(&runtime_view(), 3));
    let receipt = c.job_command(&command).unwrap();
    let id = receipt["ack"]["job"].as_u64().unwrap();
    for _ in 0..200 {
        drive_change(&mut c);
        if c.jobs.snapshot().jobs[&id].phase.terminal() {
            break;
        }
    }
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Completed);
    let actual: Vec<_> = received
        .try_iter()
        .map(|t| {
            serde_json::to_value(t).unwrap()["after_revision"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert_eq!(actual, vec![0, 1, 2]);
    let view = c.engine.view.read().unwrap();
    let runtime = &view.live.as_ref().unwrap().runtime;
    assert_eq!(runtime.cards[0].readings.len(), 3);
    assert_eq!(runtime.cards[0].observed_differences(), 2);
    drop(view);
    assert_eq!(c.job_command(&command).unwrap()["duplicate"], true);
    assert!(received.try_recv().is_err());
    for (index, step) in c.jobs.snapshot().jobs[&id].steps.iter().enumerate() {
        assert!(step.payload.len() <= 256);
        let step: CaptureStep = serde_json::from_str(&step.payload).unwrap();
        assert_eq!(step.delay_ms, if index == 0 { 0 } else { 250 });
        assert!(
            step.runtime.is_some()
                && step.local_model.is_none()
                && step.change.is_none()
                && !step.compare
        );
    }
}

#[test]
fn changed_runtime_cursor_during_pause_stops_without_publishing_a_rebased_reading() {
    let (mut c, received) = tracking_controller();
    let command = request(&c, "start_runtime_job", args(&runtime_view(), 3));
    let id = c.job_command(&command).unwrap()["ack"]["job"]
        .as_u64()
        .unwrap();
    let pause = request(&c, "job_control", json!({"job":id,"operation":"pause"}));
    c.job_command(&pause).unwrap();
    for _ in 0..10 {
        drive_change(&mut c);
    }
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Paused);
    c.engine
        .view
        .write()
        .unwrap()
        .live
        .as_mut()
        .unwrap()
        .runtime
        .revision += 1;
    let resume = request(&c, "job_control", json!({"job":id,"operation":"resume"}));
    c.job_command(&resume).unwrap();
    for _ in 0..100 {
        drive_change(&mut c);
        if c.jobs.snapshot().jobs[&id].phase.terminal() {
            break;
        }
    }
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Failed);
    assert_eq!(received.try_iter().count(), 1);
    assert!(
        c.engine
            .view
            .read()
            .unwrap()
            .live
            .as_ref()
            .unwrap()
            .runtime
            .cards[0]
            .readings
            .is_empty()
    );
}

#[test]
fn runtime_watch_refuses_unbounded_or_malformed_plans() {
    let (mut c, _) = tracking_controller();
    for (key, value) in [
        ("count", json!(0)),
        ("count", json!(17)),
        ("count", json!(1.5)),
        ("count", Value::Null),
        ("interval_ms", json!(249)),
        ("interval_ms", json!(60001)),
        ("interval_ms", json!("1000")),
    ] {
        let mut options = args(&runtime_view(), 3);
        options[key] = value;
        let command = request(&c, "start_runtime_job", options);
        assert!(c.job_command(&command).is_err(), "{key}");
    }
    let repeat_discovery = request(
        &c,
        "start_runtime_job",
        json!({"task":{"operation":"discover"},"count":2}),
    );
    assert!(c.job_command(&repeat_discovery).is_err());
    let valid = request(&c, "start_runtime_job", args(&runtime_view(), 16));
    let id = c.job_command(&valid).unwrap()["ack"]["job"]
        .as_u64()
        .unwrap();
    assert_eq!(c.jobs.snapshot().jobs[&id].steps.len(), 16);
}
