use super::{tests::*, *};
use reversebolt_core::changes::Phase;

fn watch_view() -> EngineView {
    let mut view = initial_view();
    view.changes.push(
        serde_json::from_value(json!({
            "id":"trial","prepared_ms":0,"actor":"fixture",
            "scope":{"investigation":"fixture-investigation","batch":1,"epoch":0,"frame":0,
                "prediction":"prediction","process":view.live.as_ref().unwrap().process,
                "track":0,"label":"Number","bounds":[0.,0.,40.,20.],"image_size":[100,100]},
            "observed":50.,"desired":60.,"models":[],"alternatives":[],"patch":null,
            "phase":"written","reason":"fixture","write":null,"restoration":null,"observations":[]
        }))
        .unwrap(),
    );
    view
}

fn args(count: usize) -> Value {
    json!({"change_id":"trial","phase":"written","after_observation":0,"count":count,"interval_ms":250})
}

fn complete(command: &Command, view: &mut EngineView) -> Value {
    assert_eq!(command.action, "observe_change");
    let change = &mut view.changes[0];
    let cursor: reversebolt_core::changes::ObservationCursor =
        serde_json::from_value(command.args["cursor"].clone()).unwrap();
    if cursor.phase != change.phase || cursor.after_observation != change.observations.len() {
        return json!({"ok":false,"error":"stale trial cursor","state":view});
    }
    change.observations.push(serde_json::from_value(json!({
        "time_ms":1,"image":"fixture","image_sha256":format!("image-{}",cursor.after_observation),
        "value":50.,"expected":60.,"agrees":false,"error":null
    })).unwrap());
    json!({"ok":true,"error":null,"data":change.clone(),"state":view})
}

#[test]
fn watch_uses_read_only_steps_and_replay_does_not_repeat_completed_observations() {
    let (sent, received) = mpsc::channel();
    let mut c = controller(EngineHandle::test_worker(
        watch_view(),
        move |command, view| {
            sent.send(command.clone()).unwrap();
            complete(&command, view)
        },
    ));
    let start = request(&c, "start_change_watch", args(3));
    let ack = c.job_command(&start).unwrap();
    let id = ack["ack"]["job"].as_u64().unwrap();
    for _ in 0..200 {
        drive_change(&mut c);
        if c.jobs.snapshot().jobs[&id].phase.terminal() {
            break;
        }
    }
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Completed);
    let actual: Vec<_> = received.try_iter().collect();
    assert_eq!(actual.len(), 3);
    assert_eq!(
        actual
            .iter()
            .map(|c| c.args["cursor"]["after_observation"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert!(actual.iter().all(|c| c.action == "observe_change"));
    assert_eq!(
        c.engine
            .view
            .read()
            .unwrap()
            .live
            .as_ref()
            .unwrap()
            .sample_count,
        0
    );
    assert_eq!(
        c.engine.view.read().unwrap().changes[0].observations.len(),
        3
    );
    let replay = c.job_command(&start).unwrap();
    assert_eq!(replay["ack"]["job"], ack["ack"]["job"]);
    assert_eq!(c.jobs.snapshot().jobs.len(), 1);
    for _ in 0..5 {
        drive_change(&mut c);
    }
    assert!(received.try_recv().is_err());
    let job = &c.jobs.snapshot().jobs[&id];
    assert_eq!(job.next_step, 3);
    for step in &job.steps {
        let plan: CaptureStep = serde_json::from_str(&step.payload).unwrap();
        assert!(!plan.compare && plan.local_model.is_none() && plan.change.is_some());
    }
}

#[test]
fn watch_rejects_stale_or_unbounded_plans_and_reserves_restoration_slots() {
    let mut c = controller(EngineHandle::test_worker(watch_view(), |_, _| {
        panic!("No operation expected")
    }));
    for (key, value) in [
        ("phase", json!("prepared")),
        ("phase", json!("restored")),
        ("after_observation", json!(1)),
        ("after_observation", json!(null)),
        ("count", json!(7)),
        ("count", json!(0)),
        ("count", json!(1.5)),
        ("interval_ms", json!(249)),
        ("interval_ms", json!(60001)),
        ("change_id", json!("other")),
    ] {
        let mut bad = args(3);
        bad[key] = value;
        let command = request(&c, "start_change_watch", bad);
        assert!(c.job_command(&command).is_err(), "{key}");
    }
    let first = request(&c, "start_change_watch", args(6));
    c.job_command(&first).unwrap();
    let overlap = request(&c, "start_change_watch", args(1));
    assert!(c.job_command(&overlap).is_err());
    let discovery = request(
        &c,
        "start_observation_job",
        json!({"count":1,"interval_ms":250}),
    );
    assert!(c.job_command(&discovery).is_err());
}

#[test]
fn restoration_during_pause_stops_the_old_watch_cursor() {
    let (sent, received) = mpsc::channel();
    let mut c = controller(EngineHandle::test_worker(
        watch_view(),
        move |command, view| {
            sent.send(command.clone()).unwrap();
            complete(&command, view)
        },
    ));
    let start = request(&c, "start_change_watch", args(3));
    let id = c.job_command(&start).unwrap()["ack"]["job"]
        .as_u64()
        .unwrap();
    let pause = request(&c, "job_control", json!({"job":id,"operation":"pause"}));
    c.job_command(&pause).unwrap();
    for _ in 0..10 {
        drive_change(&mut c);
    }
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Paused);
    c.engine.view.write().unwrap().changes[0].phase = Phase::Restored;
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
        c.engine.view.read().unwrap().changes[0]
            .observations
            .is_empty()
    );
}

#[test]
fn cancelling_an_inflight_watch_drains_once_without_writing_or_restoring() {
    let (sent, received) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let mut c = controller(EngineHandle::test_worker(
        watch_view(),
        move |command, view| {
            sent.send(command.clone()).unwrap();
            wait.recv().unwrap();
            complete(&command, view)
        },
    ));
    let start = request(&c, "start_change_watch", args(3));
    let id = c.job_command(&start).unwrap()["ack"]["job"]
        .as_u64()
        .unwrap();
    for _ in 0..4 {
        drive_change(&mut c);
    }
    assert_eq!(
        received
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .action,
        "observe_change"
    );
    let cancel = request(&c, "job_control", json!({"job":id,"operation":"cancel"}));
    let ack = c.job_command(&cancel).unwrap();
    assert_eq!(ack["ack"]["phase"], "Cancelling");
    assert!(c.jobs.view().in_flight.is_some());
    release.send(()).unwrap();
    for _ in 0..100 {
        drive_change(&mut c);
        if c.jobs.snapshot().jobs[&id].phase.terminal() {
            break;
        }
    }
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Cancelled);
    assert_eq!(
        c.engine.view.read().unwrap().changes[0].phase,
        Phase::Written
    );
    assert_eq!(
        c.engine.view.read().unwrap().changes[0].observations.len(),
        1
    );
    assert!(received.try_recv().is_err());
}
