//! Review an existing receipt's proposal without passing its label to the model.
use reversebolt_core::{Result, visual};
use reversebolt_observer::{Frame, Observer, Receipt, validate};
use serde::Deserialize;
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    model: String,
    frames: Vec<InputFrame>,
    proposal_receipt: PathBuf,
    candidate: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputFrame {
    path: PathBuf,
    timestamp_ns: u64,
}

fn read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = vec![];
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("Input evidence exceeds its byte limit.".into());
    }
    Ok(bytes)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("Usage: reversebolt-review INPUT.json NEW_RECEIPT.json".into());
    }
    let output = PathBuf::from(&args[2]);
    if output.exists() {
        return Err("That review receipt already exists.".into());
    }
    let input: Input = serde_json::from_slice(&read(Path::new(&args[1]), 64 * 1024)?)
        .map_err(|e| e.to_string())?;
    if !(2..=4).contains(&input.frames.len()) {
        return Err("Supply 2–4 source frames.".into());
    }
    let source = read(&input.proposal_receipt, 256 * 1024)?;
    let proposal: Receipt = serde_json::from_slice(&source).map_err(|e| e.to_string())?;
    let parsed = proposal
        .interpretation
        .as_ref()
        .ok_or("The proposal receipt has no accepted interpretation.")?;
    validate(parsed, input.frames.len())?;
    let candidate = parsed
        .entities
        .get(input.candidate)
        .ok_or("The proposal index is missing.")?;
    let frames = input
        .frames
        .iter()
        .map(|f| Frame::read(&f.path, f.timestamp_ns))
        .collect::<Result<Vec<_>>>()?;
    if proposal.frames.len() != frames.len()
        || frames.iter().zip(&proposal.frames).any(|(a, b)| {
            a.evidence.sha256 != b.sha256
                || a.evidence.timestamp_ns != b.timestamp_ns
                || a.evidence.width != b.width
                || a.evidence.height != b.height
        })
    {
        return Err("Source images or timestamps differ from the proposal receipt.".into());
    }
    let observer = Observer::connect(&input.model).await?;
    let result = observer.review(&frames, &candidate.localizations).await?;
    let receipt = serde_json::json!({"proposal_receipt_sha256":visual::digest(&source),"candidate":input.candidate,"proposer_model_digest":proposal.model_digest,"review":result});
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({"candidate":input.candidate,"kind":result.judgment.map(|j|j.kind),"rejection":result.rejection,"elapsed_ms":result.elapsed_ms})
    );
    Ok(())
}
