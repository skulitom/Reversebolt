//! Build address-free naming evidence from a retained report; no target access.
use reversebolt_core::{Result, runtime_inventory};
use serde_json::Value;
use std::{
    io::{Read, Write},
    path::PathBuf,
};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err(
            "Usage: reversebolt-runtime-inventory DISCOVERY.json NEW_INVENTORY.json".into(),
        );
    }
    let output = PathBuf::from(&args[2]);
    if !output.is_absolute() || output.exists() {
        return Err("Choose a new absolute output path.".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&args[1])
        .map_err(|e| e.to_string())?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Discovery report exceeds 64 MiB.".into());
    }
    let report: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let inventory = runtime_inventory::build(&report)?;
    let result = serde_json::to_vec_pretty(&inventory).map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    file.write_all(&result).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    println!(
        "{} observed types, {} fields, {} reachable paths.",
        inventory.types.len(),
        inventory
            .types
            .iter()
            .map(|g| g.fields.len())
            .sum::<usize>(),
        inventory.reachable_paths
    );
    Ok(())
}
