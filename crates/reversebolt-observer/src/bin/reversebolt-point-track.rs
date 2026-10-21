//! Offline research bridge: recorded point predictions to provisional box trajectories.
use reversebolt_core::{Result, visual::digest};
use reversebolt_observer::{
    FrameEvidence,
    point_tracks::{Group, Tracker},
    tracking::Seed,
};
use serde::Deserialize;
use serde_json::json;
use std::io::{Read, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    seed_frame: FrameEvidence,
    seeds: Vec<Seed>,
    steps: Vec<Step>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    frame: FrameEvidence,
    points: Vec<Group>,
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("Usage: reversebolt-point-track POINTS.json NEW_RECEIPT.json".into());
    }
    let mut bytes = vec![];
    std::fs::File::open(&args[1])
        .map_err(|e| e.to_string())?
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1_048_576 {
        return Err("Point receipt exceeds 1 MiB.".into());
    }
    let input: Input = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if !(1..=119).contains(&input.steps.len()) {
        return Err("Supply 1–119 updates.".into());
    }
    let mut gated = Tracker::new(&input.seed_frame, &input.seeds, true)?;
    let mut ungated = Tracker::new(&input.seed_frame, &input.seeds, false)?;
    let mut steps = vec![];
    for step in input.steps {
        steps.push(json!({"frame":step.frame,"gated":gated.update(&step.frame,&step.points)?,"ungated":ungated.update(&step.frame,&step.points)?}));
    }
    let receipt = json!({"schema_version":1,"input_sha256":digest(&bytes),
        "algorithm_source_sha256":digest(include_bytes!("../point_tracks.rs")),
        "runner_source_sha256":digest(include_bytes!("reversebolt-point-track.rs")),
        "parameters":reversebolt_observer::point_tracks::parameters(),
        "initial_frame":input.seed_frame,"seeds":input.seeds,"steps":steps,
        "scope":"Image point consensus only; not object identity, world motion, semantic discovery or edit authority."});
    let mut output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[2])
        .map_err(|e| e.to_string())?;
    output
        .write_all(&serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    output.sync_all().map_err(|e| e.to_string())?;
    Ok(())
}
