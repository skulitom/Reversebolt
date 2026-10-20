//! Offline evidence runner. Paths, game names and annotations are never sent to the model.
use reversebolt_core::Result;
use reversebolt_observer::{Frame, Observer};
use serde::Deserialize;
use std::{io::Read, path::PathBuf};

const MANIFEST_LIMIT: usize = 64 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    model: String,
    frames: Vec<InputFrame>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputFrame {
    path: PathBuf,
    timestamp_ns: u64,
}

fn parse_input(bytes: &[u8]) -> Result<Input> {
    if bytes.len() > MANIFEST_LIMIT {
        return Err("Input manifest exceeds 64 KiB.".into());
    }
    let input: Input = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if !(2..=reversebolt_observer::MAX_FRAMES).contains(&input.frames.len()) {
        return Err("Supply 2–4 frames.".into());
    }
    Ok(input)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("Usage: reversebolt-observer INPUT.json NEW_RECEIPT.json".into());
    }
    if PathBuf::from(&args[2]).exists() {
        return Err("That receipt already exists. Choose a new evidence filename.".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&args[1])
        .map_err(|e| e.to_string())?
        .take(MANIFEST_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let input = parse_input(&bytes)?;
    let frames = input
        .frames
        .iter()
        .map(|f| Frame::read(&f.path, f.timestamp_ns))
        .collect::<Result<Vec<_>>>()?;
    let observer = Observer::connect(&input.model).await?;
    let receipt = observer.observe(&frames).await?;
    let mut output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[2])
        .map_err(|e| e.to_string())?;
    use std::io::Write;
    output
        .write_all(&serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    output.sync_all().map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({"accepted":receipt.interpretation.is_some(),"rejection":receipt.rejection,"elapsed_ms":receipt.elapsed_ms})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_limits_are_checked_before_loading_images_or_connecting() {
        let frame = serde_json::json!({"path":"must-not-be-opened.png","timestamp_ns":1});
        for count in [0, 1, 5, 100] {
            let bytes = serde_json::to_vec(
                &serde_json::json!({"model":"example","frames":vec![frame.clone();count]}),
            )
            .unwrap();
            assert!(parse_input(&bytes).is_err());
        }
        assert!(parse_input(&vec![b' '; MANIFEST_LIMIT + 1]).is_err());
        let bytes = serde_json::to_vec(
            &serde_json::json!({"model":"example","frames":[frame.clone(),frame]}),
        )
        .unwrap();
        assert!(parse_input(&bytes).is_ok());
    }
}
