//! Convert retained schema-v4 discovery into shared-engine candidate descriptors.
use std::{io::Read, path::Path};
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: reversebolt-runtime-candidates DISCOVERY_JSON NEW_OUTPUT_JSON LIMIT".into(),
        );
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&args[1])
        .map_err(|e| e.to_string())?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("discovery file exceeds 64 MiB".into());
    }
    if !Path::new(&args[2]).is_absolute() {
        return Err("absolute new output required".into());
    }
    let report = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let candidates = reversebolt_core::runtime::candidates(
        &report,
        args[3].parse().map_err(|_| "invalid limit")?,
    )?;
    let output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])
        .map_err(|e| e.to_string())?;
    serde_json::to_writer(output, &candidates).map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({"candidates":candidates.len(),"edit_authority":false})
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
