//! Recheck pinned memory evidence offline; no target or model is involved.
use reversebolt_core::{Result, memory_evidence::Archive, memory_followup as f};
use std::path::Path;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 6 {
        return Err("Supply captures root, investigation, batch, plan SHA-256, archive directory and manifest SHA-256".into());
    }
    let reference = f::Reference {
        investigation_id: args[1].clone(),
        batch: args[2].parse().map_err(|_| "Invalid batch")?,
        plan_sha256: args[3].clone(),
    };
    let plan = f::read_plan(Path::new(&args[0]), &reference)?;
    let archive = Archive::open(Path::new(&args[4]), &args[5])?;
    println!(
        "{}",
        serde_json::to_string(&f::assess(&plan, &archive, || false)?).map_err(|e| e.to_string())?
    );
    Ok(())
}
