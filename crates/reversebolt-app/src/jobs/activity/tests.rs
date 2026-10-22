use super::*;
use crate::jobs::tests::{controller, initial_view, request};
use reversebolt_core::inputs::{Buffer, Focus, Poll};

fn view() -> EngineView {
    let mut view = initial_view();
    let live = view.live.as_mut().unwrap();
    live.sample_count = 31;
    live.memory_sample_numbers = Some((16..=31).collect());
    let mut buffer = Buffer::new(live.investigation_id.clone(), "generation".into(), 0);
    for t in (0..1_000_000).step_by(10_000) {
        buffer
            .push(Poll {
                begin_us: t,
                end_us: t + 100,
                focus: Focus::SelectedForeground,
                down: if (300_000..550_000).contains(&t) {
                    vec![65]
                } else {
                    vec![]
                },
            })
            .unwrap();
    }
    live.frames = vec![
        serde_json::from_value(json!({"id":"source","number":31,"width":256,"height":144,
        "sha256":"a".repeat(64),"epoch":0,"started_unix_ms":1,
        "capture_interval":{"clock_id":live.investigation_id,"begin_us":900000,"end_us":1000000},
        "input_context":buffer.window(0,1_000_000)}))
        .unwrap(),
    ];
    view
}
fn human(c: &mut Controller, operation: &str, id: u64) -> Value {
    c.job_command(&Command {
        source: "desktop".into(),
        action: "job_control".into(),
        args: json!({"operation":operation,"job":id}),
    })
    .unwrap()
}
fn reply(command: &Command, view: &mut EngineView, unknown: bool) -> Value {
    let step: Step = serde_json::from_value(command.args.clone()).unwrap();
    assert_eq!(
        command.action,
        if step.index == activity::STEPS {
            "_review_activity"
        } else {
            "_activity_step"
        }
    );
    let effect = activity::action(step.index, step.key).is_some();
    if !unknown && step.index < activity::STEPS {
        view.live.as_mut().unwrap().sample_count += 1;
    }
    json!({"ok":true,"error":null,"state":view,"data":{"success":!unknown,
        "effect":if unknown {"Unknown"} else if effect {"Applied"} else {"NotApplicable"},
        "experiment":step.activity,"index":step.index,"frame_id":format!("response-{}",step.index),
        "error":if unknown {Some("Unsettled input worker")} else {None}}})
}
fn finish(c: &mut Controller, id: u64) {
    for _ in 0..2000 {
        c.tick();
        if c.jobs.snapshot().jobs[&id].phase.terminal() {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("Job did not settle");
}

#[test]
fn activity_jobs_share_authorization_replay_and_capture_budgets() {
    let (sent, received) = mpsc::channel();
    let engine = EngineHandle::test_worker(view(), move |command, view| {
        sent.send(command.clone()).unwrap();
        reply(&command, view, false)
    });
    let mut c = controller(engine);
    c.engine.view.write().unwrap().observer_model = Some("first:model".into());
    let command = request(&c, "start_activity_experiment", json!({"key":65}));
    let receipt = c.job_command(&command).unwrap();
    let id = receipt["ack"]["job"].as_u64().unwrap();
    let job = &c.jobs.snapshot().jobs[&id];
    assert_eq!(job.steps.len(), 11);
    let first: Step = serde_json::from_str(&job.steps[0].payload).unwrap();
    assert_eq!(
        crate::activity_experiment::load(&c.runtime_directory, &first.activity)
            .unwrap()
            .observer_model
            .as_deref(),
        Some("first:model")
    );
    c.engine.view.write().unwrap().observer_model = Some("later:model".into());
    assert_eq!(
        job.steps
            .iter()
            .filter(|s| s.kind == job::Kind::Effect)
            .count(),
        4
    );
    assert!(job.steps.iter().all(|s| s.payload.len() <= 256));
    for _ in 0..5 {
        c.tick();
    }
    assert!(received.try_recv().is_err());
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Queued);
    let approve = request(&c, "job_control", json!({"operation":"authorize","job":id}));
    assert!(c.job_command(&approve).is_err());
    let conflicting = request(
        &c,
        "start_observation_job",
        json!({"count":1,"mode":"play","interval_ms":50}),
    );
    assert!(c.job_command(&conflicting).is_err());
    assert!(human(&mut c, "authorize", id)["ack"]["rejection"].is_null());
    finish(&mut c, id);
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Completed);
    let commands: Vec<_> = received.try_iter().collect();
    assert_eq!(commands.len(), 11);
    assert_eq!(commands.last().unwrap().action, "_review_activity");
    assert_eq!(
        c.engine
            .view
            .read()
            .unwrap()
            .live
            .as_ref()
            .unwrap()
            .sample_count,
        41
    );
    assert_eq!(c.job_command(&command).unwrap()["duplicate"], true);
    let mut changed = command;
    changed.args["key"] = json!(66);
    assert!(c.job_command(&changed).is_err());
    c.tick();
    assert!(received.try_recv().is_err());
}

#[test]
fn activity_unknown_input_is_retained_and_never_replayed_on_restart() {
    let (sent, received) = mpsc::channel();
    let engine = EngineHandle::test_worker(view(), move |command, view| {
        sent.send(command.clone()).unwrap();
        let unknown = command.args["index"] == 2;
        reply(&command, view, unknown)
    });
    let mut c = controller(engine);
    let mut command = request(&c, "start_activity_experiment", json!({"key":65}));
    command.source = "desktop".into();
    command.args = json!({"key":65});
    let id = c.job_command(&command).unwrap()["ack"]["job"]
        .as_u64()
        .unwrap();
    finish(&mut c, id);
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Interrupted);
    assert_eq!(received.try_iter().count(), 3);
    assert_eq!(
        c.jobs.snapshot().jobs[&id]
            .checkpoints
            .last()
            .unwrap()
            .outcome
            .effect,
        job::Knowledge::Unknown
    );
    let path = c.runtime_directory.parent().unwrap().to_path_buf();
    let session = c.jobs.snapshot().session.clone();
    drop(c);
    let reopened = job::Jobs::open(
        FileJournal::open(&path).unwrap(),
        &session,
        job::Limits::default(),
    )
    .unwrap();
    assert!(!reopened.snapshot().unresolved.is_empty());
    assert!(reopened.snapshot().current.is_none());
    assert!(reopened.snapshot().jobs[&id].consent_boot.is_none());
}

#[test]
fn activity_stop_owns_the_inflight_step_until_its_outcome_arrives() {
    let (sent, received) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let engine = EngineHandle::test_worker(view(), move |command, view| {
        sent.send(command.clone()).unwrap();
        wait.recv().unwrap();
        reply(&command, view, false)
    });
    let mut c = controller(engine);
    let command = Command {
        action: "start_activity_experiment".into(),
        args: json!({"key":65}),
        source: "desktop".into(),
    };
    let id = c.job_command(&command).unwrap()["ack"]["job"]
        .as_u64()
        .unwrap();
    for _ in 0..10 {
        c.tick();
        if c.executor.flight.is_some() {
            break;
        }
    }
    received.recv_timeout(Duration::from_secs(1)).unwrap();
    let stopped = human(&mut c, "cancel", id);
    assert!(stopped["ack"]["rejection"].is_null());
    assert!(
        c.executor
            .owner
            .as_ref()
            .unwrap()
            .cancel
            .load(Ordering::SeqCst)
    );
    assert!(c.executor.flight.is_some());
    release.send(()).unwrap();
    finish(&mut c, id);
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Cancelled);
    c.tick();
    assert!(received.try_recv().is_err());
}
