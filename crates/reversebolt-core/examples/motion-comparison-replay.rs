//! Read retained observer receipts and compare their numeric motion evidence only.
use reversebolt_core::{play_context, visual};
use serde::Deserialize;
use serde_json::json;
use std::{fs, io::Read};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    file: String,
    sha256: String,
}
fn bounded(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    fs::File::open(path)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Document exceeds budget".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: motion-comparison-replay MANIFEST NEW_OUTPUT".into());
    }
    let manifest = bounded(&args[0])?;
    let cases: Vec<Case> = serde_json::from_slice(&manifest)?;
    if cases.is_empty() || cases.len() > 12 {
        return Err("Choose 1-12 receipts".into());
    }
    let mut results = vec![];
    for case in cases {
        let bytes = bounded(&case.file)?;
        if visual::digest(&bytes) != case.sha256 {
            return Err("Receipt changed".into());
        }
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        if !value["error"].is_null() {
            return Err("Receipt has an activity error".into());
        }
        let mut context = play_context::Context::from_observation(
            serde_json::from_value(value["source"].clone())?,
            serde_json::from_value(value["observation"].clone())?,
        )?;
        context.motion = serde_json::from_value(value.get("motion").cloned().unwrap_or_default())?;
        context.validate()?;
        let comparison = context.motion_comparison()?;
        results.push(json!({"name":case.name,"receipt_sha256":case.sha256,"source":context.source,"motion":context.motion,
            "original_observation":context.observation,"comparison":comparison}));
    }
    let output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    serde_json::to_writer_pretty(
        output,
        &json!({"manifest_sha256":visual::digest(&manifest),"results":results,
        "model_calls":0,"target_access":false,"input_sent":false,"field_writes":0,
        "limitation":reversebolt_core::motion_comparison::LIMITATION}),
    )?;
    println!(
        "{}",
        json!({"cases":results.len(),"model_calls":0,"target_access":false})
    );
    Ok(())
}
