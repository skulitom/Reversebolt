use super::tests::{controller, drive_change, initial_view};
use super::*;
use reversebolt_core::{memory_followup as f, memory_trajectories as m};

fn complete(command: &Command, view: &mut EngineView, eligible: bool) -> Value {
    let live = view.live.as_mut().unwrap();
    match command.action.as_str() {
        "capture" => {
            live.sample_count += 1;
            if command.args["memory_review"] == true && live.coverage.batch == 1 {
                live.memory_proposals = Some(m::store::Summary {
                    investigation_id: live.investigation_id.clone(),
                    batch: 1,
                    captures: live.sample_count,
                    memory_samples: 16,
                    policy: m::POLICY.into(),
                    report_sha256: "a".repeat(64),
                    evidence_manifest_sha256: "b".repeat(64),
                    groups: 1,
                    coverage: Default::default(),
                });
            }
        }
        "prepare_memory_followup" => {
            assert_eq!(live.sample_count, 31);
            assert_eq!(command.args["after_sample"], 31);
            assert!(command.args.get("groups").is_none());
            if eligible {
                live.coverage.batch = 2;
                live.sample_count = 0;
                live.memory_proposals = None;
                live.memory_followup = Some(f::Reference {
                    investigation_id: live.investigation_id.clone(),
                    batch: 2,
                    plan_sha256: "c".repeat(64),
                });
            }
            return json!({"ok":true,"data":{"prepared":eligible},"state":view});
        }
        _ => panic!("Unexpected command {}", command.action),
    }
    json!({"ok":true,"data":{"captured":true},"state":view})
}

#[test]
fn memory_followup_runs_once_after_initial_job_or_abstains_without_a_child() {
    for eligible in [false, true] {
        let (tx, rx) = mpsc::channel();
        let engine = EngineHandle::test_worker(initial_view(), move |command, view| {
            tx.send(command.clone()).unwrap();
            complete(&command, view, eligible)
        });
        let mut c = controller(engine);
        let parent = c.start_automatic_observation("agent".into()).unwrap()["ack"]["job"]
            .as_u64()
            .unwrap();
        for _ in 0..2000 {
            drive_change(&mut c);
            if c.memory_continuation.is_none()
                && c.manual.is_none()
                && c.jobs.snapshot().jobs.values().all(|j| j.phase.terminal())
            {
                break;
            }
        }
        let commands: Vec<_> = rx.try_iter().collect();
        assert_eq!(
            commands
                .iter()
                .filter(|c| c.action == "prepare_memory_followup")
                .count(),
            1
        );
        assert_eq!(
            commands.iter().filter(|c| c.action == "capture").count(),
            if eligible { 43 } else { 31 }
        );
        assert_eq!(c.jobs.snapshot().jobs.len(), if eligible { 2 } else { 1 });
        assert!(
            c.jobs
                .snapshot()
                .jobs
                .values()
                .all(|j| j.phase == job::Phase::Completed)
        );
        if eligible {
            let (p, child) = c.change_child.unwrap();
            assert_eq!(p, parent);
            for step in &c.jobs.snapshot().jobs[&child].steps {
                let step: CaptureStep = serde_json::from_str(&step.payload).unwrap();
                assert_eq!(step.mode, reversebolt_core::live::CaptureMode::Visual);
                assert!(!step.compare && step.local_model.is_none());
            }
        }
        for _ in 0..5 {
            drive_change(&mut c);
        }
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn human_stop_during_memory_preparation_prevents_fresh_job() {
    let (tx, rx) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let engine = EngineHandle::test_worker(initial_view(), move |command, view| {
        tx.send(command.clone()).unwrap();
        if command.action == "prepare_memory_followup" {
            held.recv().unwrap();
        }
        complete(&command, view, true)
    });
    let mut c = controller(engine);
    let parent = c.start_automatic_observation("desktop".into()).unwrap()["ack"]["job"]
        .as_u64()
        .unwrap();
    let mut preparing = false;
    for _ in 0..1500 {
        drive_change(&mut c);
        if rx.try_iter().any(|c| c.action == "prepare_memory_followup") {
            preparing = true;
            break;
        }
    }
    assert!(preparing);
    let (tx, ack) = mpsc::channel();
    c.command((
        Command {
            action: "job_control".into(),
            source: "desktop".into(),
            args: json!({"operation":"cancel","job":parent}),
        },
        Some(tx),
    ));
    assert_eq!(ack.recv().unwrap()["ok"], true);
    assert!(c.manual.as_ref().unwrap().automatic.is_none());
    release.send(()).unwrap();
    for _ in 0..100 {
        drive_change(&mut c);
        if c.manual.is_none() {
            break;
        }
    }
    assert!(c.memory_continuation.is_none());
    assert_eq!(c.jobs.snapshot().jobs.len(), 1);
    assert!(rx.try_recv().is_err());
}

#[test]
fn cancelled_initial_observation_never_prepares_memory_followup() {
    let (tx, rx) = mpsc::channel();
    let engine = EngineHandle::test_worker(initial_view(), move |command, view| {
        tx.send(command.clone()).unwrap();
        complete(&command, view, true)
    });
    let mut c = controller(engine);
    let parent = c.start_automatic_observation("desktop".into()).unwrap()["ack"]["job"]
        .as_u64()
        .unwrap();
    c.command((
        Command {
            action: "job_control".into(),
            source: "desktop".into(),
            args: json!({"operation":"cancel","job":parent}),
        },
        None,
    ));
    for _ in 0..5 {
        drive_change(&mut c);
    }
    assert!(c.memory_continuation.is_none());
    assert_eq!(c.jobs.snapshot().jobs.len(), 1);
    assert!(rx.try_recv().is_err());
}
