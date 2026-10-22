//! File-only passive activity proposals; no model or live target access.
use reversebolt_core::{
    play_context::Source,
    visual::{self, FrameRef},
};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    io::Read,
    path::{Component, Path},
};

#[derive(Deserialize)]
struct SavedFrame {
    file: String,
    reference: FrameRef,
}
#[derive(Deserialize)]
struct Case {
    source: Source,
    frames: Vec<SavedFrame>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("Use CASE_JSON NEW_OUTPUT_JSON.".into());
    }
    let path = Path::new(&args[0]);
    let mut bytes = vec![];
    fs::File::open(path)?
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Case exceeds metadata limit.".into());
    }
    let case: Case = serde_json::from_slice(&bytes)?;
    let folder = path
        .parent()
        .ok_or("Missing evidence directory")?
        .canonicalize()?;
    let frames: Vec<_> = case.frames.iter().map(|f| f.reference.clone()).collect();
    let mut images_read = vec![];
    let result =
        reversebolt_vision::activity_region_suggestions::analyze(&case.source, &frames, |frame| {
            let saved = case
                .frames
                .iter()
                .find(|f| f.reference.id == frame.id)
                .ok_or("Missing image")?;
            let relative = Path::new(&saved.file);
            if relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            {
                return Err("Only relative evidence paths are accepted.".into());
            }
            let file = folder
                .join(relative)
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if !file.starts_with(&folder) {
                return Err("Image escaped the evidence directory.".into());
            }
            images_read.push(frame.id.clone());
            visual::image_bytes(&file)
        });
    let (report, error) = match result {
        Ok(report) => (Some(report), None),
        Err(error) => (None, Some(error)),
    };
    let output = json!({"case_sha256":visual::digest(&bytes),"report":report,"error":error,
        "images_read":images_read,"target_access":false,"new_captures":0,"input_sent":false,
        "field_writes":0,"model_calls":0});
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    serde_json::to_writer_pretty(file, &output)?;
    println!(
        "{}",
        json!({"status":output["report"]["status"],"error":output["error"]})
    );
    Ok(())
}
