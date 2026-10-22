//! Read-only saved-image evaluation through the product's pair-measurement path.
#[allow(dead_code)]
#[path = "../src/play_evidence.rs"]
mod play_evidence;

use reversebolt_core::{
    play_evidence as core_evidence,
    visual::{self, FrameRef},
};
use serde::Deserialize;
use serde_json::json;
use std::{fs, io::Read, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    file: String,
    reference: FrameRef,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    frames: Vec<Frame>,
    expected_differences: Vec<f64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: motion-replay CASES_JSON NEW_OUTPUT_JSON".into());
    }
    let mut bytes = vec![];
    fs::File::open(&args[0])?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Manifest exceeds budget".into());
    }
    let cases: Vec<Case> = serde_json::from_slice(&bytes)?;
    if cases.is_empty() || cases.len() > 8 {
        return Err("Choose 1-8 cases".into());
    }
    let mut results = vec![];
    for case in cases {
        if !(2..=20).contains(&case.frames.len())
            || case.expected_differences.len() + 1 != case.frames.len()
        {
            return Err("Invalid case frame count".into());
        }
        let mut previous: Option<Vec<u8>> = None;
        let mut patches = vec![];
        let mut differences = vec![];
        for (i, frame) in case.frames.iter().enumerate() {
            let image = visual::image_bytes(Path::new(&frame.file))?;
            if visual::digest(&image) != frame.reference.sha256 {
                return Err("Saved image changed".into());
            }
            if let Some(prior) = &previous {
                let (difference, patch) = play_evidence::measure_pair(
                    &case.frames[i - 1].reference,
                    &frame.reference,
                    prior,
                    &image,
                );
                patches.push(patch);
                differences.push(difference);
            }
            previous = Some(image);
        }
        let unchanged: Vec<_> = differences
            .iter()
            .zip(&case.expected_differences)
            .map(|(a, b)| a.difference == Some(*b))
            .collect();
        let references: Vec<_> = case.frames.iter().map(|f| f.reference.clone()).collect();
        let evidence = core_evidence::analyze(&references, differences)?;
        results.push(json!({"name":case.name,"frame_metadata_sha256":visual::digest(&serde_json::to_vec(&references)?),
            "evidence":evidence,"patch_motion":patches,"old_difference_exact_matches":unchanged}));
    }
    let output = json!({"schema":1,"manifest_sha256":visual::digest(&bytes),"cases":results,
        "target_access":false,"input_sent":false,"new_model_calls":0,"field_writes":0,
        "limitation":"Known saved-image regression, not independent-game validation. Patch matches do not identify actors or establish gameplay effects."});
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    serde_json::to_writer_pretty(file, &output)?;
    println!("{}", json!({"cases":results.len(),"target_access":false}));
    Ok(())
}
