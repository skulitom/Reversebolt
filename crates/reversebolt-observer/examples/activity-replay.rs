//! One offline activity interpretation, without metadata inference or target access.
use reversebolt_observer::{Frame, Observer, runtime_activity::Input};
use serde::Deserialize;
use std::{fs, io::Read, path::PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    source: reversebolt_core::play_context::Source,
    frames: Vec<PathBuf>,
    #[serde(default)]
    appearances: Option<reversebolt_core::appearance_activity::Summary>,
    #[serde(default)]
    regions: Option<reversebolt_core::activity_regions::Summary>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: activity-replay MANIFEST NEW_OUTPUT MODEL".into());
    }
    let mut bytes = vec![];
    fs::File::open(&args[0])?
        .take(65537)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err("Manifest exceeds budget".into());
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    manifest.source.validate()?;
    if manifest.frames.len() != 4 {
        return Err("Supply exactly four images".into());
    }
    let frames = manifest
        .frames
        .iter()
        .zip(manifest.source.frames())
        .map(|(path, reference)| {
            Frame::read(
                path,
                reference
                    .interval
                    .begin_us
                    .checked_mul(1000)
                    .ok_or("Clock overflow")?,
            )
        })
        .collect::<Result<Vec<_>, String>>()?;
    let input = Input {
        source: manifest.source,
        frames,
        appearances: manifest.appearances,
        regions: manifest.regions,
    };
    input.validate()?;
    let output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    let receipt = tokio::runtime::Runtime::new()?.block_on(async {
        let observer = Observer::connect(&args[2]).await?;
        Ok::<_, String>(observer.runtime_activity(&input).await)
    })?;
    let context = receipt.context();
    let summary = serde_json::json!({"error":receipt.error,"observation":receipt.observation,
        "image_model_calls":usize::from(receipt.stage.is_some()),"target_access":false,
        "context_retained":context.is_some(),"regions_retained":context.as_ref().and_then(|c|c.regions.as_ref())});
    serde_json::to_writer_pretty(output, &receipt)?;
    println!("{summary}");
    Ok(())
}
