use reversebolt_session_jobs::{adapter::Inbox, fake::*, *};

fn human() -> Actor {
    Actor {
        client: "desktop".into(),
        role: Role::Human,
    }
}
fn agent() -> Actor {
    Actor {
        client: "mcp-agent".into(),
        role: Role::Agent,
    }
}
fn step(kind: Kind, payload: &str) -> Step {
    Step {
        kind,
        payload: payload.into(),
    }
}
fn setup(limits: Limits) -> (Jobs<MemoryJournal>, MemoryJournal, FakeExecutor) {
    let journal = MemoryJournal::default();
    let mut jobs = Jobs::open(journal.clone(), "session-test", limits).unwrap();
    let binding = jobs
        .bind(Some("pid:42/created:100/window:7"))
        .unwrap()
        .unwrap();
    let mut fake = FakeExecutor::default();
    fake.current = Some(binding);
    fake.intent_witness = Some(journal.clone());
    (jobs, journal, fake)
}
fn envelope(j: &Jobs<MemoryJournal>, actor: Actor, action: Action) -> Command {
    let request = j
        .snapshot()
        .clients
        .get(&actor.client)
        .map_or(1, |c| c.high_water + 1);
    Command {
        request,
        actor,
        boot: j.snapshot().boot,
        binding: j.snapshot().current.clone().unwrap(),
        action,
    }
}
fn send(j: &mut Jobs<MemoryJournal>, actor: Actor, action: Action) -> Reply {
    j.command(envelope(j, actor, action)).unwrap()
}
fn submit(j: &mut Jobs<MemoryJournal>, steps: Vec<Step>) -> u64 {
    send(
        j,
        human(),
        Action::Submit {
            steps,
            consent: true,
        },
    )
    .ack
    .job
    .unwrap()
}
fn run(j: &mut Jobs<MemoryJournal>, e: &mut FakeExecutor) {
    for _ in 0..200 {
        match j.tick(e).unwrap() {
            Progress::Idle | Progress::Fenced | Progress::NeedsDrain(_) => return,
            _ => {}
        }
    }
    panic!("unbounded progress")
}

#[test]
fn shared_checkpoints_and_intent_precede_every_effect() {
    let (mut j, _, mut e) = setup(Limits::default());
    let id = submit(
        &mut j,
        vec![step(Kind::Observe, "read"), step(Kind::Effect, "write")],
    );
    run(&mut j, &mut e);
    assert_eq!(j.snapshot().jobs[&id].phase, Phase::Completed);
    assert_eq!(e.audit.effects.len(), 1);
    assert_eq!(e.audit.intent_violations, 0);
    assert_eq!(j.snapshot().jobs[&id].checkpoints.len(), 2);
    assert_eq!(j.snapshot().jobs[&id].next_step, 2);
    j.snapshot().validate().unwrap();
}

#[test]
fn executor_failures_distinguish_not_applied_from_applied_without_retry() {
    for after in [false, true] {
        let (mut j, _, mut e) = setup(Limits::default());
        e.behaviors.insert(
            "fail".into(),
            Behavior {
                fail_before: !after,
                fail_after: after,
                ..Behavior::default()
            },
        );
        let id = submit(
            &mut j,
            vec![step(Kind::Effect, "fail"), step(Kind::Effect, "later")],
        );
        run(&mut j, &mut e);
        assert_eq!(j.snapshot().jobs[&id].phase, Phase::Failed);
        assert_eq!(e.audit.begins.len(), 1);
        assert_eq!(e.audit.effects.len(), usize::from(after));
        assert_eq!(
            j.snapshot().jobs[&id].checkpoints[0].outcome.effect,
            if after {
                Knowledge::Applied
            } else {
                Knowledge::NotApplied
            }
        );
    }
}

#[test]
fn failed_cancel_persistence_retains_flight_and_drain_reports_volatile_outcome() {
    let (mut j, journal, mut e) = setup(Limits::default());
    e.behaviors.insert(
        "slow".into(),
        Behavior {
            delay: 1,
            apply_on_begin: false,
            ..Behavior::default()
        },
    );
    let id = submit(
        &mut j,
        vec![step(Kind::Effect, "slow"), step(Kind::Effect, "later")],
    );
    for _ in 0..3 {
        j.tick(&mut e).unwrap();
    }
    journal.fail_next(StorageFault::BeforeWrite);
    assert!(
        j.command(envelope(&j, human(), Action::Cancel { job: id }))
            .is_err()
    );
    assert!(j.view().in_flight.is_some());
    assert_eq!(j.tick(&mut e).unwrap(), Progress::Fenced);
    assert_eq!(j.drain(&mut e).unwrap(), Progress::Fenced);
    assert!(j.view().in_flight.is_none());
    assert_eq!(
        j.view().volatile_outcome.unwrap().1.effect,
        Knowledge::Applied
    );
    assert_eq!(e.audit.begins.len(), 1);
    assert_eq!(e.audit.effects.len(), 1);
    assert_eq!(e.audit.cancellation_seen, 1);
    // The volatile receipt was not committed. Recovery must still say Unknown.
    let recovered = Jobs::open(j.into_journal(), "session-test", Limits::default()).unwrap();
    assert_eq!(
        recovered.snapshot().jobs[&id].checkpoints[0].outcome.effect,
        Knowledge::Unknown
    );
}

#[test]
fn deterministic_command_schedules_preserve_effect_and_scope_invariants() {
    use std::collections::BTreeSet;
    for seed in 1..=32u64 {
        let limits = Limits {
            queued: 4,
            jobs: 8,
            events: 16,
            receipts_per_client: 4,
            polls: 4,
            ..Limits::default()
        };
        let (mut j, journal, mut e) = setup(limits.clone());
        e.behaviors.insert(
            "slow".into(),
            Behavior {
                delay: 2,
                apply_on_begin: false,
                ignore_cancel: seed % 2 == 0,
                ..Behavior::default()
            },
        );
        let mut rng = seed;
        let mut cancelled = BTreeSet::new();
        for _ in 0..256 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let before_begins = e.audit.begins.len();
            let pick = j
                .snapshot()
                .jobs
                .keys()
                .copied()
                .nth((rng as usize / 16) % j.snapshot().jobs.len().max(1));
            match rng % 12 {
                0 | 1 => {
                    send(
                        &mut j,
                        human(),
                        Action::Submit {
                            steps: vec![
                                step(Kind::Observe, "read"),
                                step(Kind::Effect, "slow"),
                                step(Kind::Effect, "last"),
                            ],
                            consent: true,
                        },
                    );
                }
                2 => {
                    if let Some(job) = pick {
                        let reply = send(&mut j, human(), Action::Cancel { job });
                        if reply.ack.rejection.is_none() {
                            cancelled.insert(job);
                        }
                    }
                }
                3 => {
                    if let Some(job) = pick {
                        send(&mut j, human(), Action::Pause { job });
                    }
                }
                4 => {
                    if let Some(job) = pick {
                        send(
                            &mut j,
                            human(),
                            Action::Resume {
                                job,
                                renew_consent: true,
                            },
                        );
                    }
                }
                5 => {
                    e.current = j.bind(Some("pid:42/created:100/window:7")).unwrap();
                }
                6 => {
                    // Simulated controller crash. An independent executor may
                    // still finish; its receipt is external reconciliation.
                    let flight = j.view().in_flight;
                    j = Jobs::open(j.into_journal(), "session-test", limits.clone()).unwrap();
                    if let Some(f) = flight {
                        let outcome = loop {
                            if let Poll::Ready(outcome) = e.poll(f.operation.id, true) {
                                break outcome;
                            }
                        };
                        if j.snapshot().unresolved.contains(&f.operation.id) {
                            j.reconcile(f.operation.id, Some(outcome)).unwrap();
                        }
                    }
                    // A prepared intent with no executor entry is also Unknown
                    // after recovery; this test owns an independent audit.
                    for id in j.snapshot().unresolved.clone() {
                        assert!(!e.audit.begins.iter().any(|op| op.id == id));
                        j.reconcile(
                            id,
                            Some(Outcome {
                                success: false,
                                effect: Knowledge::NotApplied,
                                data: vec![],
                                note: "independent fake audit: never entered".into(),
                            }),
                        )
                        .unwrap();
                    }
                    e.current = j.bind(Some("pid:42/created:100/window:7")).unwrap();
                }
                7 => {
                    let old = j
                        .snapshot()
                        .clients
                        .get("desktop")
                        .and_then(|c| c.receipts.back())
                        .map(|r| r.command.clone());
                    if let Some(c) = old {
                        assert!(j.command(c).unwrap().duplicate);
                    }
                }
                8 => {
                    if let Some(job) = pick {
                        send(&mut j, human(), Action::Continue { job, consent: true });
                    }
                }
                _ => {
                    j.tick(&mut e).unwrap();
                }
            }
            j.snapshot().validate().unwrap();
            for op in &e.audit.begins[before_begins..] {
                assert!(
                    !cancelled.contains(&op.job),
                    "future operation after cancellation"
                );
                assert_eq!(Some(&op.binding), j.snapshot().current.as_ref());
            }
            assert_eq!(e.audit.intent_violations, 0);
            let unique: BTreeSet<_> = e
                .audit
                .effects
                .iter()
                .map(|effect| effect.operation)
                .collect();
            assert_eq!(
                unique.len(),
                e.audit.effects.len(),
                "an effect was repeated"
            );
            assert!(journal.writes() <= 4096);
        }
    }
    println!("deterministic command schedules checked: 32 x 256 = 8192 actions");
}

#[test]
fn cancel_at_every_boundary_prevents_second_effect_but_reports_first() {
    for boundary in 0..=6 {
        let (mut j, _, mut e) = setup(Limits::default());
        e.behaviors.insert(
            "first".into(),
            Behavior {
                delay: 2,
                apply_on_begin: false,
                ..Behavior::default()
            },
        );
        let id = submit(
            &mut j,
            vec![
                step(Kind::Effect, "first"),
                step(Kind::Effect, "must-not-start"),
            ],
        );
        for _ in 0..boundary {
            j.tick(&mut e).unwrap();
        }
        let begun = e.audit.begins.len();
        let ack = send(&mut j, human(), Action::Cancel { job: id }).ack;
        if ack.phase == Some(Phase::Cancelling) {
            assert!(ack.in_flight.is_some());
        }
        run(&mut j, &mut e);
        assert_eq!(
            j.snapshot().jobs[&id].phase,
            Phase::Cancelled,
            "boundary {boundary}"
        );
        assert_eq!(
            e.audit.begins.len(),
            begun,
            "dispatch after cancellation boundary {boundary}"
        );
        assert!(e.audit.effects.len() <= 1);
        assert!(
            e.audit
                .begins
                .iter()
                .all(|op| op.instruction.payload != "must-not-start")
        );
        if begun > 0 {
            assert_eq!(
                e.audit.effects.len(),
                1,
                "already started operation is reported, not undone"
            );
        }
    }
}

#[test]
fn pausing_waits_then_resume_requires_renewed_human_effect_scope() {
    let (mut j, _, mut e) = setup(Limits::default());
    e.behaviors.insert(
        "slow".into(),
        Behavior {
            delay: 2,
            ..Behavior::default()
        },
    );
    let id = submit(
        &mut j,
        vec![step(Kind::Effect, "slow"), step(Kind::Effect, "second")],
    );
    for _ in 0..3 {
        j.tick(&mut e).unwrap();
    }
    assert_eq!(
        send(&mut j, human(), Action::Pause { job: id }).ack.phase,
        Some(Phase::Pausing)
    );
    run(&mut j, &mut e);
    assert_eq!(j.snapshot().jobs[&id].phase, Phase::Paused);
    assert_eq!(e.audit.effects.len(), 1);
    assert!(
        send(
            &mut j,
            human(),
            Action::Resume {
                job: id,
                renew_consent: false
            }
        )
        .ack
        .rejection
        .is_some()
    );
    assert!(
        send(
            &mut j,
            human(),
            Action::Resume {
                job: id,
                renew_consent: true
            }
        )
        .ack
        .rejection
        .is_none()
    );
    run(&mut j, &mut e);
    assert_eq!(e.audit.effects.len(), 2);
    assert_eq!(j.snapshot().jobs[&id].phase, Phase::Completed);
}

#[test]
fn human_takeover_transfers_control_and_holds_agent_queue() {
    let (mut j, _, mut e) = setup(Limits::default());
    e.behaviors.insert(
        "slow".into(),
        Behavior {
            delay: 2,
            ..Behavior::default()
        },
    );
    let first = send(
        &mut j,
        agent(),
        Action::Submit {
            steps: vec![step(Kind::Effect, "slow"), step(Kind::Effect, "next")],
            consent: false,
        },
    )
    .ack
    .job
    .unwrap();
    let second = send(
        &mut j,
        agent(),
        Action::Submit {
            steps: vec![step(Kind::Observe, "queued")],
            consent: false,
        },
    )
    .ack
    .job
    .unwrap();
    send(&mut j, human(), Action::Authorize { job: first });
    for _ in 0..3 {
        j.tick(&mut e).unwrap();
    }
    let takeover = send(&mut j, human(), Action::TakeControl);
    assert!(takeover.ack.in_flight.is_some());
    run(&mut j, &mut e);
    assert_eq!(j.snapshot().jobs[&first].phase, Phase::Paused);
    assert_eq!(j.snapshot().jobs[&second].phase, Phase::Paused);
    assert_eq!(j.snapshot().jobs[&first].controller, human());
    assert!(
        send(
            &mut j,
            agent(),
            Action::Resume {
                job: first,
                renew_consent: false
            }
        )
        .ack
        .rejection
        .is_some()
    );
    assert!(
        send(
            &mut j,
            agent(),
            Action::Submit {
                steps: vec![step(Kind::Observe, "late")],
                consent: false
            }
        )
        .ack
        .rejection
        .is_some()
    );
    let human_job = submit(&mut j, vec![step(Kind::Observe, "human")]);
    run(&mut j, &mut e);
    assert_eq!(j.snapshot().jobs[&human_job].phase, Phase::Completed);
    assert_eq!(e.audit.effects.len(), 1);
    assert!(
        j.snapshot()
            .events
            .iter()
            .any(|event| event.actor == Some(human()) && event.request == Some(2))
    );
}

#[test]
fn duplicate_and_collision_semantics_survive_restart() {
    let (mut j, journal, mut e) = setup(Limits::default());
    let c = envelope(
        &j,
        human(),
        Action::Submit {
            steps: vec![step(Kind::Effect, "once")],
            consent: true,
        },
    );
    let first = j.command(c.clone()).unwrap();
    assert!(j.command(c.clone()).unwrap().duplicate);
    let mut different = c.clone();
    different.action = Action::TakeControl;
    assert!(j.command(different).unwrap_err().contains("collision"));
    run(&mut j, &mut e);
    drop(j);
    let mut reopened = Jobs::open(journal, "session-test", Limits::default()).unwrap();
    let duplicate = reopened.command(c).unwrap();
    assert_eq!(duplicate.ack, first.ack);
    assert!(duplicate.duplicate);
    assert_eq!(reopened.snapshot().jobs.len(), 1);
    assert_eq!(
        reopened.snapshot().jobs[&first.ack.job.unwrap()].phase,
        Phase::Completed
    );
    assert!(!reopened.snapshot().jobs[&first.ack.job.unwrap()].evidence_valid);
    run(&mut reopened, &mut e);
    assert_eq!(e.audit.effects.len(), 1);
}

#[test]
fn evicted_request_ids_are_tombstoned_and_job_ids_never_reused() {
    let limits = Limits {
        receipts_per_client: 2,
        jobs: 2,
        ..Limits::default()
    };
    let (mut j, journal, mut e) = setup(limits.clone());
    let first = envelope(
        &j,
        human(),
        Action::Submit {
            steps: vec![step(Kind::Observe, "old")],
            consent: false,
        },
    );
    j.command(first.clone()).unwrap();
    run(&mut j, &mut e);
    for _ in 0..4 {
        submit(&mut j, vec![step(Kind::Observe, "new")]);
        run(&mut j, &mut e);
    }
    assert_eq!(j.snapshot().jobs.len(), 2);
    assert_eq!(j.snapshot().next_job, 6);
    assert!(j.command(first.clone()).unwrap_err().contains("expired"));
    drop(j);
    let mut j = Jobs::open(journal, "session-test", limits).unwrap();
    assert!(j.command(first).unwrap_err().contains("expired"));
}

#[test]
fn stale_boot_and_generation_cannot_retarget_work() {
    let (mut j, _, mut e) = setup(Limits::default());
    let stale = envelope(
        &j,
        agent(),
        Action::Submit {
            steps: vec![step(Kind::Effect, "old")],
            consent: false,
        },
    );
    let id = submit(&mut j, vec![step(Kind::Observe, "old-read")]);
    let next = j
        .bind(Some("pid:42/created:200/window:7"))
        .unwrap()
        .unwrap();
    e.attach(next);
    assert_eq!(j.snapshot().jobs[&id].phase, Phase::Interrupted);
    assert!(
        j.command(stale)
            .unwrap()
            .ack
            .rejection
            .unwrap()
            .contains("stale")
    );
    run(&mut j, &mut e);
    assert!(e.audit.begins.is_empty());
    let mut c = envelope(
        &j,
        human(),
        Action::Submit {
            steps: vec![step(Kind::Observe, "new")],
            consent: false,
        },
    );
    c.boot -= 1;
    assert!(j.command(c).unwrap().ack.rejection.is_some());
}

#[test]
fn target_replacement_keeps_old_in_flight_operation_pinned() {
    let (mut j, _, mut e) = setup(Limits::default());
    e.behaviors.insert(
        "slow".into(),
        Behavior {
            delay: 2,
            apply_on_begin: false,
            ..Behavior::default()
        },
    );
    let old_binding = j.snapshot().current.clone().unwrap();
    let old = submit(
        &mut j,
        vec![step(Kind::Effect, "slow"), step(Kind::Effect, "never")],
    );
    for _ in 0..3 {
        j.tick(&mut e).unwrap();
    }
    let new_binding = j
        .bind(Some("pid:99/created:999/window:3"))
        .unwrap()
        .unwrap();
    e.attach(new_binding.clone());
    let new = submit(&mut j, vec![step(Kind::Effect, "new")]);
    run(&mut j, &mut e);
    assert_eq!(j.snapshot().jobs[&old].phase, Phase::Interrupted);
    assert!(!j.snapshot().jobs[&old].evidence_valid);
    assert_eq!(j.snapshot().jobs[&new].phase, Phase::Completed);
    assert_eq!(e.audit.effects.len(), 2);
    assert_eq!(e.audit.effects[0].binding, old_binding);
    assert_eq!(e.audit.effects[1].binding, new_binding);
}

#[test]
fn process_exit_records_known_effect_and_stops_future_steps() {
    let (mut j, _, mut e) = setup(Limits::default());
    e.behaviors.insert(
        "slow".into(),
        Behavior {
            delay: 4,
            ..Behavior::default()
        },
    );
    let id = submit(
        &mut j,
        vec![step(Kind::Effect, "slow"), step(Kind::Effect, "never")],
    );
    for _ in 0..3 {
        j.tick(&mut e).unwrap();
    }
    e.dead_targets
        .insert(j.snapshot().current.as_ref().unwrap().target.clone());
    j.bind(None).unwrap();
    run(&mut j, &mut e);
    let job = &j.snapshot().jobs[&id];
    assert_eq!(job.phase, Phase::Interrupted);
    assert_eq!(job.checkpoints[0].outcome.effect, Knowledge::Applied);
    assert_eq!(e.audit.effects.len(), 1);
}

#[test]
fn observation_restart_is_history_until_explicit_fresh_continuation() {
    let (mut j, journal, mut e) = setup(Limits::default());
    let old = submit(
        &mut j,
        vec![step(Kind::Observe, "a"), step(Kind::Observe, "b")],
    );
    for _ in 0..3 {
        j.tick(&mut e).unwrap();
    }
    assert_eq!(j.snapshot().jobs[&old].next_step, 1);
    drop(j);
    let mut j = Jobs::open(journal, "session-test", Limits::default()).unwrap();
    assert_eq!(j.snapshot().jobs[&old].phase, Phase::Interrupted);
    assert_eq!(j.tick(&mut e).unwrap(), Progress::Idle);
    let binding = j
        .bind(Some("pid:42/created:100/window:7"))
        .unwrap()
        .unwrap();
    e.attach(binding.clone());
    assert!(
        send(
            &mut j,
            human(),
            Action::Resume {
                job: old,
                renew_consent: false
            }
        )
        .ack
        .rejection
        .is_some()
    );
    let new = send(
        &mut j,
        human(),
        Action::Continue {
            job: old,
            consent: false,
        },
    )
    .ack
    .job
    .unwrap();
    assert!(j.snapshot().jobs[&new].checkpoints.is_empty());
    assert_eq!(j.snapshot().jobs[&new].binding, binding);
    assert_eq!(j.snapshot().jobs[&new].parent, Some(old));
    run(&mut j, &mut e);
    assert_eq!(j.snapshot().jobs[&new].checkpoints.len(), 2);
    assert!(!j.snapshot().jobs[&old].evidence_valid);
}

#[test]
fn crash_after_intent_is_unknown_even_if_nothing_was_executed() {
    let (mut j, journal, mut e) = setup(Limits::default());
    let id = submit(&mut j, vec![step(Kind::Effect, "uncertain")]);
    j.tick(&mut e).unwrap();
    let p = j.tick(&mut e).unwrap();
    let Progress::Intent(op) = p else { panic!() };
    assert!(e.audit.effects.is_empty());
    drop(j);
    let mut j = Jobs::open(journal, "session-test", Limits::default()).unwrap();
    assert_eq!(
        j.snapshot().jobs[&id].checkpoints[0].outcome.effect,
        Knowledge::Unknown
    );
    assert_eq!(j.tick(&mut e).unwrap(), Progress::Fenced);
    let binding = j
        .bind(Some("pid:42/created:100/window:7"))
        .unwrap()
        .unwrap();
    e.attach(binding);
    assert!(
        send(
            &mut j,
            human(),
            Action::Continue {
                job: id,
                consent: true
            }
        )
        .ack
        .rejection
        .is_some()
    );
    j.reconcile(op, None).unwrap();
    assert!(
        send(
            &mut j,
            human(),
            Action::Continue {
                job: id,
                consent: true
            }
        )
        .ack
        .rejection
        .is_some(),
        "quiescence alone is not an outcome"
    );
    let new = submit(&mut j, vec![step(Kind::Observe, "independent")]);
    run(&mut j, &mut e);
    assert_eq!(j.snapshot().jobs[&new].phase, Phase::Completed);
    assert!(e.audit.effects.is_empty());
}

#[test]
fn storage_failure_at_each_intent_and_outcome_boundary_fences_dispatch() {
    for fault in [StorageFault::BeforeWrite, StorageFault::AfterDurableWrite] {
        for boundary in ["intent", "outcome"] {
            let (mut j, journal, mut e) = setup(Limits::default());
            let id = submit(&mut j, vec![step(Kind::Effect, "once")]);
            j.tick(&mut e).unwrap();
            if boundary == "outcome" {
                j.tick(&mut e).unwrap();
            }
            journal.fail_next(fault);
            assert!(j.tick(&mut e).is_err());
            assert_eq!(j.tick(&mut e).unwrap(), Progress::Fenced);
            assert_eq!(e.audit.effects.len(), usize::from(boundary == "outcome"));
            if boundary == "outcome" {
                assert_eq!(
                    j.view().volatile_outcome.as_ref().unwrap().1.effect,
                    Knowledge::Applied
                );
            }
            drop(j);
            let recovered = Jobs::open(journal, "session-test", Limits::default()).unwrap();
            let job = &recovered.snapshot().jobs[&id];
            match (boundary, fault) {
                ("intent", StorageFault::BeforeWrite) => assert!(job.checkpoints.is_empty()),
                ("outcome", StorageFault::AfterDurableWrite) => {
                    assert_eq!(job.phase, Phase::Completed)
                }
                _ => assert_eq!(
                    job.checkpoints.last().unwrap().outcome.effect,
                    Knowledge::Unknown
                ),
            }
        }
    }
}

#[test]
fn lost_submission_ack_is_replayed_after_restart_not_resubmitted() {
    let (mut j, journal, _) = setup(Limits::default());
    let c = envelope(
        &j,
        human(),
        Action::Submit {
            steps: vec![step(Kind::Effect, "once")],
            consent: true,
        },
    );
    journal.fail_next(StorageFault::AfterDurableWrite);
    assert!(j.command(c.clone()).is_err());
    drop(j);
    let mut j = Jobs::open(journal, "session-test", Limits::default()).unwrap();
    assert!(j.command(c).unwrap().duplicate);
    assert_eq!(j.snapshot().jobs.len(), 1);
    assert_eq!(
        j.snapshot().jobs.values().next().unwrap().phase,
        Phase::Interrupted
    );
}

#[test]
fn cancelled_ack_write_failure_fences_even_though_no_ack_was_returned() {
    let (mut j, journal, mut e) = setup(Limits::default());
    let id = submit(&mut j, vec![step(Kind::Effect, "once")]);
    j.tick(&mut e).unwrap();
    j.tick(&mut e).unwrap();
    journal.fail_next(StorageFault::BeforeWrite);
    let c = envelope(&j, human(), Action::Cancel { job: id });
    assert!(j.command(c).is_err());
    run(&mut j, &mut e);
    assert!(e.audit.effects.is_empty());
}

#[test]
fn slow_executor_budget_retains_barrier_until_explicit_drain() {
    let limits = Limits {
        polls: 2,
        ..Limits::default()
    };
    let (mut j, _, mut e) = setup(limits);
    e.behaviors.insert(
        "slow".into(),
        Behavior {
            delay: 4,
            apply_on_begin: false,
            ..Behavior::default()
        },
    );
    let id = submit(
        &mut j,
        vec![step(Kind::Effect, "slow"), step(Kind::Effect, "never")],
    );
    run(&mut j, &mut e);
    assert!(matches!(j.tick(&mut e).unwrap(), Progress::NeedsDrain(_)));
    let polls = e.audit.polls;
    for _ in 0..10 {
        j.tick(&mut e).unwrap();
    }
    assert_eq!(e.audit.polls, polls);
    assert!(e.has_pending());
    j.drain(&mut e).unwrap();
    j.drain(&mut e).unwrap();
    assert_eq!(j.snapshot().jobs[&id].phase, Phase::Cancelled);
    assert_eq!(e.audit.effects.len(), 1);
    assert_eq!(e.audit.begins.len(), 1);
}

#[test]
fn oversized_result_fails_job_but_preserves_applied_effect_knowledge() {
    let (mut j, _, mut e) = setup(Limits::default());
    e.behaviors.insert(
        "big".into(),
        Behavior {
            result_bytes: 2048,
            ..Behavior::default()
        },
    );
    let id = submit(
        &mut j,
        vec![step(Kind::Effect, "big"), step(Kind::Effect, "never")],
    );
    run(&mut j, &mut e);
    let job = &j.snapshot().jobs[&id];
    assert_eq!(job.phase, Phase::Failed);
    assert!(job.checkpoints[0].outcome.data.is_empty());
    assert_eq!(job.checkpoints[0].outcome.effect, Knowledge::Applied);
    assert_eq!(e.audit.effects.len(), 1);
    let next = send(
        &mut j,
        human(),
        Action::Continue {
            job: id,
            consent: true,
        },
    )
    .ack
    .job
    .unwrap();
    assert_eq!(
        j.snapshot().jobs[&next].steps,
        vec![step(Kind::Effect, "never")]
    );
}

#[test]
fn queue_client_event_and_result_budgets_are_observable() {
    let limits = Limits {
        queued: 2,
        clients: 2,
        events: 4,
        result_per_job: 8,
        ..Limits::default()
    };
    let (mut j, _, mut e) = setup(limits);
    let id = submit(
        &mut j,
        vec![step(Kind::Observe, "a"), step(Kind::Observe, "b")],
    );
    submit(&mut j, vec![step(Kind::Observe, "queued")]);
    assert!(
        send(
            &mut j,
            human(),
            Action::Submit {
                steps: vec![step(Kind::Observe, "overflow")],
                consent: false
            }
        )
        .ack
        .rejection
        .is_some()
    );
    send(
        &mut j,
        agent(),
        Action::Submit {
            steps: vec![step(Kind::Observe, "overflow")],
            consent: false,
        },
    );
    let c = envelope(
        &j,
        Actor {
            client: "third".into(),
            role: Role::Agent,
        },
        Action::TakeControl,
    );
    assert!(j.command(c).is_err());
    run(&mut j, &mut e);
    assert_eq!(j.snapshot().jobs[&id].result_bytes(), 8);
    assert_eq!(j.snapshot().jobs[&id].phase, Phase::Failed);
    assert!(matches!(
        j.events_after(0, 100),
        Err(CursorError::Expired { .. })
    ));
    let head = j.snapshot().event_head;
    assert!(j.events_after(head, 100).unwrap().events.is_empty());
    assert_eq!(j.events_after(head + 1, 1).err(), Some(CursorError::Future));
}

#[test]
fn reserved_human_ingress_preserves_per_client_order() {
    let (j, _, _) = setup(Limits::default());
    let mut inbox = Inbox::new(2, 1).unwrap();
    let mut a = envelope(
        &j,
        agent(),
        Action::Submit {
            steps: vec![step(Kind::Observe, "a")],
            consent: false,
        },
    );
    inbox.try_send(a.clone()).unwrap();
    a.request += 1;
    inbox.try_send(a.clone()).unwrap();
    assert!(inbox.try_send(a).is_err());
    inbox
        .try_send(envelope(&j, human(), Action::TakeControl))
        .unwrap();
    assert_eq!(inbox.pop().unwrap().actor, human());
    assert_eq!(inbox.pop().unwrap().request, 1);
    assert_eq!(inbox.pop().unwrap().request, 2);
    let submit = envelope(
        &j,
        human(),
        Action::Submit {
            steps: vec![step(Kind::Observe, "x")],
            consent: false,
        },
    );
    let mut control = submit.clone();
    control.request = 2;
    control.action = Action::TakeControl;
    inbox.try_send(submit).unwrap();
    inbox.try_send(control).unwrap();
    assert_eq!(inbox.pop().unwrap().request, 1);
    assert_eq!(inbox.pop().unwrap().request, 2);
}

#[test]
fn failed_recovery_checkpoint_never_exposes_an_active_runner() {
    let (j, journal, _) = setup(Limits::default());
    drop(j);
    journal.fail_next(StorageFault::BeforeWrite);
    assert!(Jobs::open(journal, "session-test", Limits::default()).is_err());
}

#[test]
fn actual_effect_may_remain_after_crash_until_independent_reconciliation() {
    let (mut j, journal, mut e) = setup(Limits::default());
    e.behaviors.insert(
        "slow".into(),
        Behavior {
            delay: 1,
            apply_on_begin: false,
            ..Behavior::default()
        },
    );
    let id = submit(&mut j, vec![step(Kind::Effect, "slow")]);
    for _ in 0..3 {
        j.tick(&mut e).unwrap();
    }
    let op = j.view().in_flight.unwrap().operation.id;
    drop(j);
    let mut j = Jobs::open(journal, "session-test", Limits::default()).unwrap();
    assert_eq!(j.tick(&mut e).unwrap(), Progress::Fenced);
    let Poll::Ready(receipt) = e.poll(op, true) else {
        panic!()
    };
    assert_eq!(e.audit.effects.len(), 1);
    j.reconcile(op, Some(receipt)).unwrap();
    assert_eq!(j.snapshot().jobs[&id].phase, Phase::Interrupted);
    assert_eq!(
        j.snapshot().jobs[&id].checkpoints[0].outcome.effect,
        Knowledge::Applied
    );
    assert!(j.snapshot().unresolved.is_empty());
}
