//! Bounded, image-only offline tracking experiment. No model or process access.
use reversebolt_core::{Result, visual};
use reversebolt_observer::{
    Frame,
    tracking::{Seed, Tracker},
};
use serde::Deserialize;
use serde_json::json;
use std::{
    io::{Read, Write},
    path::PathBuf,
    time::Instant,
};

const MANIFEST_LIMIT: usize = 64 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    frames: Vec<InputFrame>,
    seed_image_sha256: String,
    seeds: Vec<Seed>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputFrame {
    path: PathBuf,
    timestamp_ns: u64,
}

fn parse_input(bytes: &[u8]) -> Result<Input> {
    if bytes.len() > MANIFEST_LIMIT {
        return Err("Manifest exceeds 64 KiB.".into());
    }
    let input: Input = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if !(2..=120).contains(&input.frames.len()) || input.seeds.len() > 6 {
        return Err("Supply 2–120 frames and at most six seeds.".into());
    }
    if input
        .frames
        .windows(2)
        .any(|pair| pair[0].timestamp_ns >= pair[1].timestamp_ns)
    {
        return Err("Frames must be strictly chronological.".into());
    }
    if input.seed_image_sha256.len() != 64
        || !input
            .seed_image_sha256
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
    {
        return Err("Supply the seed image SHA-256.".into());
    }
    Ok(input)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("Usage: reversebolt-track INPUT.json NEW_RECEIPT.json".into());
    }
    if PathBuf::from(&args[2]).exists() {
        return Err("Choose a new receipt filename.".into());
    }
    let mut bytes = vec![];
    std::fs::File::open(&args[1])
        .map_err(|e| e.to_string())?
        .take(MANIFEST_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let input = parse_input(&bytes)?;
    let started = Instant::now();
    let initial = Frame::read(&input.frames[0].path, input.frames[0].timestamp_ns)?;
    if !initial
        .evidence
        .sha256
        .eq_ignore_ascii_case(&input.seed_image_sha256)
    {
        return Err("Seed geometry is bound to a different image.".into());
    }
    let mut gated = Tracker::new(&initial, &input.seeds, true)?;
    let mut ungated = Tracker::new(&initial, &input.seeds, false)?;
    let mut steps = vec![];
    for source in &input.frames[1..] {
        let frame = Frame::read(&source.path, source.timestamp_ns)?;
        let t = Instant::now();
        let checked = gated.update(&frame)?;
        let gated_ms = t.elapsed().as_secs_f64() * 1000.;
        let t = Instant::now();
        let unchecked = ungated.update(&frame)?;
        steps.push(
            json!({"frame":frame.evidence,"gated":checked,"ungated":unchecked,
            "gated_ms":gated_ms,"ungated_ms":t.elapsed().as_secs_f64()*1000.}),
        );
    }
    let receipt = json!({"schema_version":1,"input_sha256":visual::digest(&bytes),
        "algorithm_source_sha256":visual::digest(include_bytes!("../tracking.rs")),
        "parameters":reversebolt_observer::tracking::parameters(),
        "initial_frame":initial.evidence,"seeds":input.seeds,"steps":steps,
        "elapsed_ms":started.elapsed().as_secs_f64()*1000.,
        "scope":"Provisional image-patch correspondence. Not semantic identity, world motion or edit authority."});
    let mut output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[2])
        .map_err(|e| e.to_string())?;
    output
        .write_all(&serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    output.sync_all().map_err(|e| e.to_string())?;
    println!(
        "{}",
        json!({"frames":input.frames.len(),"seeds":input.seeds.len(),"elapsed_ms":receipt["elapsed_ms"]})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_bounds_chronology_and_contract() {
        let mut valid = json!({"frames":[{"path":"a.png","timestamp_ns":1},{"path":"b.png","timestamp_ns":2}],
            "seed_image_sha256":"a".repeat(64),"seeds":[]});
        assert!(parse_input(&serde_json::to_vec(&valid).unwrap()).is_ok());
        valid["frames"][1]["timestamp_ns"] = json!(1);
        assert!(parse_input(&serde_json::to_vec(&valid).unwrap()).is_err());
        valid["frames"][1]["timestamp_ns"] = json!(2);
        valid["hidden_labels"] = json!([]);
        assert!(parse_input(&serde_json::to_vec(&valid).unwrap()).is_err());
        assert!(parse_input(&vec![b' '; MANIFEST_LIMIT + 1]).is_err());
        valid.as_object_mut().unwrap().remove("hidden_labels");
        valid["frames"] = json!(vec![json!({"path":"x","timestamp_ns":0}); 121]);
        assert!(parse_input(&serde_json::to_vec(&valid).unwrap()).is_err());
    }
}
