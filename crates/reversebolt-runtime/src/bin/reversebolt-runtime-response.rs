//! Analyze retained sibling reads and QPC-bracketed input; no target access.
use reversebolt_core::{
    Result,
    runtime_neighbors::Plan,
    runtime_response::{Observation, analyze},
};
use std::io::{Read, Write};
fn read<T: serde::de::DeserializeOwned>(path: &str) -> Result<T> {
    let mut bytes = vec![];
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Input exceeds budget".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err(
            "usage: reversebolt-runtime-response PLAN.json BATCH OBSERVATIONS.json NEW_OUTPUT.json"
                .into(),
        );
    }
    let plan: Plan = read(&args[0])?;
    let observations: Vec<Observation> = read(&args[2])?;
    let result = analyze(
        &plan,
        args[1].parse().map_err(|_| "Invalid batch")?,
        &observations,
    )?;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[3])
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({"fields":result["fields"].as_array().unwrap().len(),
        "associations":result["fields"].as_array().unwrap().iter().filter(|f|f["repeated_association"]==true).count()})
    );
    Ok(())
}
