//! Bind historical unverified questions to a new observation for test design.
//! This does not rebind their paths or create an executable/live editing plan.
use reversebolt_core::{runtime_hypotheses::Batch, runtime_planning, visual::FrameRef};
use reversebolt_observer::runtime_activity::Receipt;
use serde_json::{Value, json};
use std::{fs, io::Read};
fn read(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    fs::File::open(path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("Input exceeds budget".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 5 {
        return Err(
            "usage: activity-question-plan QUESTIONS ACTIVITY EVIDENCE FINAL_STATE NEW_OUTPUT"
                .into(),
        );
    }
    let questions = read(&args[0])?;
    let batch: Batch = serde_json::from_value(questions["batch"].clone())?;
    let activity: Receipt = serde_json::from_value(read(&args[1])?)?;
    let context = activity
        .context()
        .ok_or("No usable observed activity context")?;
    let evidence: reversebolt_core::play_evidence::Evidence =
        serde_json::from_value(read(&args[2])?["evidence"].clone())?;
    let frames: Vec<FrameRef> =
        serde_json::from_value(read(&args[3])?["state"]["live"]["frames"].clone())?;
    let actions = runtime_planning::actions(&context, &frames, &evidence)?;
    let choices: Vec<_> = batch.accepted.iter().map(|h|json!({
        "selected_candidate":h.candidate,"question":h.proposal.question,"explanation":h.proposal.explanation,
        "selection_role":"primary","value":"0"
    })).collect();
    if choices.is_empty() || choices.len() > 12 {
        return Err("Missing or oversized historical question batch".into());
    }
    let plan = json!({"report_sha256":batch.report_sha256,"activity_context":context,
        "observed_actions":actions,"choices":choices});
    for choice in plan["choices"].as_array().unwrap() {
        reversebolt_core::runtime_test_design::Input::from_choice(&plan, choice)?;
    }
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[4])?;
    serde_json::to_writer_pretty(
        file,
        &json!({"plan":plan,"historical_questions":true,
        "live_paths_reacquired":false,"execution_authority":false,
        "limitation":"Planning only: metadata questions belong to a prior exited process. New scene evidence does not rebind their paths. Any intervention requires fresh discovery, exact value/reference checks and a new durable trial."}),
    )?;
    println!(
        "{}",
        json!({"questions":choices.len(),"live_paths_reacquired":false,"model_calls":0})
    );
    Ok(())
}
