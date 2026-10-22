//! File-only replay of retained observations through the product's region path.
#[allow(dead_code)]
#[path = "../src/play_evidence.rs"]
mod play_evidence;

use reversebolt_core::{
    visual::{self, FrameRef, VisualTrack},
    visual_regions,
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
    investigation_id: String,
    frames: Vec<Frame>,
    visual_tracks: Vec<VisualTrack>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: observed-region-replay CASES_JSON NEW_OUTPUT_JSON".into());
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
        let references = case
            .frames
            .iter()
            .map(|f| f.reference.clone())
            .collect::<Vec<_>>();
        let selection =
            visual_regions::select(&case.investigation_id, &references, &case.visual_tracks)?;
        let mut previous: Option<Vec<u8>> = None;
        let mut measurements = vec![];
        for (index, frame) in case.frames.iter().enumerate() {
            let image = visual::image_bytes(Path::new(&frame.file))?;
            if visual::digest(&image) != frame.reference.sha256 {
                return Err("Saved image changed".into());
            }
            if let Some(prior) = &previous {
                measurements.extend(play_evidence::automatic_interval(
                    &selection,
                    &references[index - 1],
                    &frame.reference,
                    prior,
                    &image,
                ));
            }
            previous = Some(image);
        }
        let sequences = play_evidence::automatic_sequences(
            &case.investigation_id,
            &references,
            &case.visual_tracks,
            |frame| {
                let file = &case
                    .frames
                    .iter()
                    .find(|f| f.reference.id == frame.id)
                    .ok_or("Unknown replay frame")?
                    .file;
                visual::image_bytes(Path::new(file))
            },
        );
        results.push(json!({"name":case.name,"selection":selection,"measurements":measurements,"appearance_sequences":sequences}));
    }
    let output = json!({"schema":1,"manifest_sha256":visual::digest(&bytes),"cases":results,
        "target_access":false,"input_sent":false,"new_model_calls":0,"field_writes":0,
        "limitation":"Known saved-observation regression. Observer claims remain unverified; no object tracking, motion ground truth or gameplay effect is established."});
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    serde_json::to_writer_pretty(file, &output)?;
    println!("{}", json!({"cases":results.len(),"target_access":false}));
    Ok(())
}
