//! Replay saved evidence through an agent review handoff; no model or target access.
use reversebolt_core::{
    live::LiveView,
    play_evidence::{self, Measurement},
    runtime_hypotheses as h, runtime_inventory, runtime_planning as p, visual,
};
use serde_json::{Value, json};
use std::{io::Read, path::Path};
fn read(path: &Path) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    std::fs::File::open(path)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Replay input exceeds its budget".into());
    }
    Ok((serde_json::from_slice(&bytes)?, visual::digest(&bytes)))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err(
            "usage: agent-handoff-replay STATUS_JSON PRIOR_REPLAY_JSON CAPTURES OUTPUT_JSON".into(),
        );
    }
    let (status, status_hash) = read(Path::new(&args[0]))?;
    let live: LiveView = serde_json::from_value(status["state"]["live"].clone())?;
    let (prior, prior_hash) = read(Path::new(&args[1]))?;
    if prior["input_hashes"]["status"] != status_hash {
        return Err("The prior replay belongs to another status".into());
    }
    let old: h::Review = serde_json::from_value(prior["revised_review"].clone())?;
    let mut source_review = live
        .runtime
        .naming
        .clone()
        .ok_or("Missing original review")?;
    source_review.play_context = old.play_context.clone();
    if json!(source_review) != json!(old) {
        return Err("The prior replay changed metadata hypotheses".into());
    }
    let source = live
        .runtime
        .catalog_source
        .as_ref()
        .ok_or("Missing report")?;
    let (report, report_hash) = read(Path::new(&source.path))?;
    if prior["input_hashes"]["report_file"] != report_hash {
        return Err("The report file changed".into());
    }
    let inventory = runtime_inventory::build(&report)?;
    if inventory.report_sha256 != old.report_sha256
        || !(2..=reversebolt_core::live::MAX_SAMPLES).contains(&live.frames.len())
    {
        return Err("Invalid retained scope".into());
    }
    let mut images = vec![];
    let mut previous: Option<Vec<u8>> = None;
    let mut measurements = vec![];
    for (i, frame) in live.frames.iter().enumerate() {
        let bytes = visual::image_bytes(
            &Path::new(&args[2]).join(format!("frame-{:03}.png", frame.number)),
        )?;
        if visual::digest(&bytes) != frame.sha256 {
            return Err("Image changed".into());
        }
        if let Some(a) = &previous {
            let before = &live.frames[i - 1];
            if (before.width, before.height, before.epoch)
                != (frame.width, frame.height, frame.epoch)
            {
                return Err("Scene geometry changed".into());
            }
            measurements.push(Measurement {
                difference: Some(reversebolt_vision::response::difference(a, &bytes)?),
                error: None,
            });
        }
        previous = Some(bytes);
        images.push(json!({"id":frame.id,"sha256":frame.sha256}));
    }
    let evidence = play_evidence::analyze(&live.frames, measurements)?;
    if json!(evidence) != prior["evidence"] {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args[3])?;
        serde_json::to_writer_pretty(
            file,
            &json!({"error":"Recomputed play evidence changed",
            "actual":evidence,"expected":prior["evidence"],"target_access":false,"new_model_calls":0}),
        )?;
        return Err("Recomputed play evidence changed".into());
    }
    let selection = h::select(
        &inventory,
        &old.batch
            .as_ref()
            .ok_or("Missing questions")?
            .selected_types,
    )?;
    let proposals = h::Proposals {
        hypotheses: old
            .batch
            .as_ref()
            .unwrap()
            .accepted
            .iter()
            .map(|h| h.proposal.clone())
            .collect(),
    };
    let mut agent = h::Review {
        evidence_id: uuid::Uuid::new_v4().to_string(),
        model: "agent:offline-replay".into(),
        status: "complete".into(),
        error: None,
        report_sha256: old.report_sha256.clone(),
        question_source: Default::default(),
        question_origin: None,
        question_policy: None,
        play_context: None,
        batch: Some(h::validate(&selection, proposals)?),
    };
    let before_handoff = agent.clone();
    let (context, transfer) = h::transfer_activity(
        Some(&old),
        &inventory.report_sha256,
        &live.investigation_id,
        live.coverage.batch,
        &live.frames,
    );
    agent.play_context = context;
    let context = agent
        .play_context
        .as_ref()
        .ok_or("Activity was not retained")?;
    let actions = p::actions(context, &live.frames, &evidence)?;
    let choices = p::choices(
        &agent,
        &inventory,
        &actions,
        &[],
        &live.investigation_id,
        &live.runtime.catalog_id,
    )?;
    let origins: Vec<_> = choices
        .iter()
        .map(|c| {
            let reference =
                serde_json::from_value(c["hypothesis_ref"].clone()).map_err(|e| e.to_string())?;
            h::resolve(&agent, &reference, &inventory)
        })
        .collect::<reversebolt_core::Result<_>>()?;
    let restored: h::Review = serde_json::from_slice(&serde_json::to_vec(&agent)?)?;
    let restored_actions = p::actions(
        restored.play_context.as_ref().unwrap(),
        &live.frames,
        &evidence,
    )?;
    let mut changed = live.frames.clone();
    changed[0].sha256 = "0".repeat(64);
    let stale = h::transfer_activity(
        Some(&agent),
        &inventory.report_sha256,
        &live.investigation_id,
        live.coverage.batch,
        &changed,
    );
    let output = json!({"schema":1,"prior_submission_context":before_handoff.play_context,"activity_transfer":transfer,
        "agent_review":agent,"observed_actions":actions,"choices":choices,"test_origins":origins,
        "roundtrip_actions_equal":restored_actions==actions,"changed_image_transfer":stale.1,
        "input_hashes":{"status":status_hash,"prior_replay":prior_hash,"report":report_hash,"images":images},
        "new_model_calls":0,"target_access":false,"input_sent":false,"field_writes":0,
        "limitation":"Offline regression on a known case. No new agent intelligence, native integration, gameplay effect or security clearance is established."});
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[3])?;
    serde_json::to_writer_pretty(file, &output)?;
    println!(
        "{}",
        json!({"retained":transfer.retained,"choices":choices.len(),"stale_retained":stale.1.retained,"target_access":false})
    );
    Ok(())
}
