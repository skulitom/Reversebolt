use super::*;
use reversebolt_core::activity_experiment::selection;

fn multiple_keys() -> EngineView {
    let mut state = view();
    let input = state.live.as_mut().unwrap().frames[0]
        .input_context
        .as_mut()
        .unwrap();
    for poll in &mut input.polls {
        if (100_000..200_000).contains(&poll.begin_us) {
            poll.down = vec![66];
        }
    }
    state
}

#[test]
fn automatic_choice_uses_sampling_without_key_meanings_or_overlapping_counts() {
    let mut state = multiple_keys();
    let live = state.live.as_mut().unwrap();
    let first = selection::suggest(live, &[]).unwrap();
    assert_eq!(first.next.as_ref().unwrap().key, 65);
    assert_eq!(first.candidates.len(), 2);
    let counts = first.candidates[0].pressed_polls;
    // Overlapping observations must not inflate evidence by repeated counting.
    let mut duplicate = live.frames[0].clone();
    duplicate.id = "overlapping-later".into();
    duplicate.number += 1;
    live.sample_count += 1;
    live.frames.push(duplicate);
    let duplicate = selection::suggest(live, &[]).unwrap();
    assert_eq!(duplicate.next.as_ref().unwrap().pressed_polls, counts);
    assert_eq!(
        duplicate.next.as_ref().unwrap().source_frame,
        "overlapping-later"
    );
    // Renaming the observed keys changes the output in the same way.
    for frame in &mut live.frames {
        for poll in &mut frame.input_context.as_mut().unwrap().polls {
            for key in &mut poll.down {
                *key = if *key == 65 { 90 } else { 88 };
            }
        }
    }
    let renamed = selection::suggest(live, &[]).unwrap();
    assert_eq!(renamed.next.as_ref().unwrap().key, 90);
    assert_eq!(renamed.next.as_ref().unwrap().pressed_polls, counts);
    assert_eq!(
        selection::suggest(live, &[90]).unwrap().next.unwrap().key,
        88
    );
    assert!(selection::suggest(live, &[90, 88]).unwrap().next.is_none());
    // A more frequently sampled chord is still ranked behind a key seen alone.
    for frame in &mut live.frames {
        for poll in &mut frame.input_context.as_mut().unwrap().polls {
            if poll.down == [90] {
                poll.down = vec![89, 90];
            }
        }
    }
    assert_eq!(selection::suggest(live, &[]).unwrap().next.unwrap().key, 88);
}

#[test]
fn automatic_choice_abstains_for_unusable_coverage_scope_and_capacity() {
    for mutation in 0..5 {
        let mut state = multiple_keys();
        let live = state.live.as_mut().unwrap();
        match mutation {
            0 => live.frames[0].input_context.as_mut().unwrap().omitted_polls = 1,
            1 => live.frames[0].input_context.as_mut().unwrap().polls[0].focus = Focus::Unavailable,
            2 => live.sample_count = reversebolt_core::live::MAX_SAMPLES - 9,
            3 => {
                live.coverage.image_budget_bytes = reversebolt_core::coverage::MAX_IMAGE_DISK_BYTES
            }
            _ => {
                let mut last = live.frames[0].clone();
                last.id = "new-geometry".into();
                last.epoch += 1;
                last.input_context = None;
                live.frames.push(last);
            }
        }
        let suggestion = selection::suggest(live, &[]).unwrap();
        assert!(suggestion.next.is_none(), "mutation {mutation}");
        assert!(suggestion.unavailable.is_some());
    }
    let state = multiple_keys();
    let live = state.live.as_ref().unwrap();
    let selected = selection::suggest(live, &[]).unwrap();
    let mut plan = Plan::new(live, selected.next.as_ref().unwrap().key, "fixture".into()).unwrap();
    assert!(json!(plan).get("selection").is_none());
    plan.selection = Some(selected);
    assert!(plan.check(live, 0).is_ok());
    plan.selection
        .as_mut()
        .unwrap()
        .next
        .as_mut()
        .unwrap()
        .source_metadata_sha256 = "0".repeat(64);
    assert!(plan.check(live, 0).is_err());
}

#[test]
fn automatic_submission_is_pinned_replayable_and_never_retries_a_proposed_key() {
    let engine = EngineHandle::test_worker(multiple_keys(), |_, _| {
        panic!("Planning must not execute input")
    });
    let mut c = controller(engine);
    let before = c.jobs.snapshot().event_head;
    // Exercise the same public read-only route used by MCP, not only the helper.
    let (send, receive) = mpsc::channel();
    c.command((
        Command {
            source: "agent".into(),
            action: "plan_activity_experiment".into(),
            args: json!({}),
        },
        Some(send),
    ));
    let suggested = receive.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(suggested["ok"], true);
    assert_eq!(suggested["data"]["next"]["key"], 65);
    assert_eq!(c.jobs.snapshot().event_head, before);
    let command = request(&c, "start_activity_experiment", json!({}));
    let result = c.job_command(&command).unwrap();
    let id = result["ack"]["job"].as_u64().unwrap();
    let first: Step = serde_json::from_str(&c.jobs.snapshot().jobs[&id].steps[0].payload).unwrap();
    let plan = crate::activity_experiment::load(&c.runtime_directory, &first.activity).unwrap();
    assert_eq!(plan.key, 65);
    assert_eq!(plan.selection.unwrap().next.unwrap().key, 65);
    for _ in 0..3 {
        c.tick();
    }
    assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Queued);
    // A later ranking change cannot silently change an already submitted request.
    for poll in &mut c.engine.view.write().unwrap().live.as_mut().unwrap().frames[0]
        .input_context
        .as_mut()
        .unwrap()
        .polls
    {
        if poll.down == [65] {
            poll.down = vec![67];
        }
    }
    assert_eq!(c.job_command(&command).unwrap()["duplicate"], true);
    let mut changed = command.clone();
    changed.args["key"] = json!(65);
    assert!(c.job_command(&changed).is_err());
    // Restore the source only to test selection after cancellation. No worker ran.
    *c.engine.view.write().unwrap() = multiple_keys();
    human(&mut c, "cancel", id);
    finish(&mut c, id);
    assert_eq!(c.activity_suggestion().unwrap().next.unwrap().key, 66);
    let second = request(&c, "start_activity_experiment", json!({}));
    let second_id = c.job_command(&second).unwrap()["ack"]["job"]
        .as_u64()
        .unwrap();
    human(&mut c, "cancel", second_id);
    finish(&mut c, second_id);
    assert!(c.activity_suggestion().unwrap().next.is_none());
    let next = request(&c, "start_activity_experiment", json!({}));
    assert!(c.job_command(&next).is_err());
    // Actual journal eviction must not make cancelled input proposals eligible.
    for _ in 0..26 {
        let observation = request(
            &c,
            "start_observation_job",
            json!({"count":1,"mode":"play","interval_ms":50}),
        );
        let job_id = c.job_command(&observation).unwrap()["ack"]["job"]
            .as_u64()
            .unwrap();
        human(&mut c, "cancel", job_id);
        finish(&mut c, job_id);
    }
    assert!(!c.jobs.snapshot().jobs.contains_key(&id));
    assert!(!c.jobs.snapshot().jobs.contains_key(&second_id));
    assert!(c.activity_suggestion().unwrap().next.is_none());
    let original_live = c.engine.view.read().unwrap().live.clone().unwrap();
    let original_binding = c.jobs.snapshot().current.as_ref().unwrap().target.clone();
    c.engine
        .view
        .write()
        .unwrap()
        .live
        .as_mut()
        .unwrap()
        .coverage
        .batch += 1;
    c.refresh_binding();
    assert_eq!(c.activity_suggestion().unwrap().next.unwrap().key, 65);
    let directory = c.runtime_directory.clone();
    let session = c.jobs.snapshot().session.clone();
    drop(c);
    let reopened = job::Jobs::open(
        FileJournal::open(directory.parent().unwrap()).unwrap(),
        &session,
        job::Limits::default(),
    )
    .unwrap();
    assert_eq!(reopened.snapshot().jobs.len(), 24);
    assert!(
        reopened
            .snapshot()
            .jobs
            .values()
            .all(|j| j.phase.terminal())
    );
    let reserved =
        crate::activity_experiment::selection_history::keys(&directory, &original_binding).unwrap();
    assert_eq!(reserved, vec![65, 66]);
    assert!(
        selection::suggest(&original_live, &reserved)
            .unwrap()
            .next
            .is_none()
    );
    let marker = directory
        .join("activity-selection")
        .join(reversebolt_core::visual::digest(
            original_binding.as_bytes(),
        ))
        .join("065.json");
    std::fs::write(marker, b"incomplete reservation").unwrap();
    assert!(
        crate::activity_experiment::selection_history::keys(&directory, &original_binding).is_err()
    );
}
