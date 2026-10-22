//! Offline memory-only proposals. No target connection or live editing.
use reversebolt_core::{Result, memory_evidence::Archive, memory_trajectories};
use std::path::Path;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("Supply the archive directory and expected manifest SHA-256".into());
    }
    let archive = Archive::open(Path::new(&args[0]), &args[1])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&memory_trajectories::analyze(&archive)?)
            .map_err(|e| e.to_string())?
    );
    Ok(())
}
