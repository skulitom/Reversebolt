//! Inspect saved activity independently of executable replay or metadata questions.
use reversebolt_core::{play_evidence::Evidence, runtime_planning, visual::FrameRef};
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
    if args.len() != 4 {
        return Err("usage: activity-assessment ACTIVITY EVIDENCE FINAL_STATE NEW_OUTPUT".into());
    }
    let activity: Receipt = serde_json::from_value(read(&args[0])?)?;
    let context = activity
        .context()
        .ok_or("No valid retained activity context")?;
    let evidence: Evidence = serde_json::from_value(read(&args[1])?["evidence"].clone())?;
    let frames: Vec<FrameRef> =
        serde_json::from_value(read(&args[2])?["state"]["live"]["frames"].clone())?;
    let assessment = runtime_planning::assess_actions(&context, &frames, &evidence)?;
    // The old execution-facing API still refuses this same unsupported sequence.
    let legacy = runtime_planning::actions(&context, &frames, &evidence);
    if legacy.as_ref().ok() != assessment.actions.as_ref()
        || legacy.as_ref().err() != assessment.reason.as_ref()
    {
        return Err("Replay authority changed during assessment".into());
    }
    let output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[3])?;
    let value = json!({"context":context,"replay":assessment,"model_calls":0,
        "target_access":false,"execution_authority":false});
    serde_json::to_writer_pretty(output, &value)?;
    println!(
        "{}",
        json!({"replay_available":value["replay"]["actions"].is_object(),
        "timing_issues":value["replay"]["timing_issues"].as_array().unwrap().len(),"model_calls":0})
    );
    Ok(())
}
