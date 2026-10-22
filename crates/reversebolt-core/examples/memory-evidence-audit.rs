//! Verify a local frozen export without connecting to any process.
use reversebolt_core::{Result, memory_evidence::Archive};
use serde_json::json;
use std::path::Path;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("Supply the archive directory and expected manifest SHA-256".into());
    }
    let archive = Archive::open(Path::new(&args[0]), &args[1])?;
    archive.verify()?;
    let manifest = archive.manifest();
    println!("{}",serde_json::to_string_pretty(&json!({"verified":true,"investigation_id":manifest.state.investigation_id,
        "batch":manifest.state.coverage.batch,"captures":manifest.samples.len(),"memory_samples":manifest.state.memory_sample_count(),
        "payload_bytes":manifest.payload_bytes,"failed_reads":manifest.samples.iter().flat_map(|s|&s.reads).filter(|r|r.segment.is_none()).count(),
        "invalid_chunks":manifest.invalid_chunks.len(),"limitation":manifest.limitation})).map_err(|e|e.to_string())?);
    Ok(())
}
