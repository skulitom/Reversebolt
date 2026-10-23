//! Plan read-only sibling observations from saved discovery and metadata review.
use reversebolt_core::{Result, runtime_neighbors};
use serde_json::Value;
use std::{io::Read, path::Path};
fn read(path: &str) -> Result<Value> {
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
    if args.len() != 3 {
        return Err(
            "usage: reversebolt-runtime-neighbors DISCOVERY.json REVIEW.json NEW_OUTPUT_DIR".into(),
        );
    }
    let report = read(&args[0])?;
    let receipt = read(&args[1])?;
    let rows = receipt["batch"]["accepted"]
        .as_array()
        .ok_or("Missing completed hypotheses")?;
    let mut seeds = vec![];
    for h in rows {
        for row in std::iter::once(&h["candidate"])
            .chain(h["alternatives"].as_array().ok_or("Missing alternatives")?)
        {
            let id = row["path_id"]
                .as_str()
                .ok_or("Missing candidate path")?
                .to_owned();
            if !seeds.contains(&id) {
                seeds.push(id);
            }
        }
    }
    let plan = runtime_neighbors::plan(&report, &seeds, 128)?;
    if receipt["report_sha256"] != plan.report_sha256 {
        return Err("Hypotheses belong to another report".into());
    }
    let output = Path::new(&args[2]);
    std::fs::create_dir(output).map_err(|e| e.to_string())?;
    std::fs::write(
        output.join("plan.json"),
        serde_json::to_vec_pretty(&plan).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    for (i, batch) in plan.batches.iter().enumerate() {
        std::fs::write(
            output.join(format!("paths-{i}.json")),
            serde_json::to_vec(batch).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    println!(
        "{}",
        serde_json::json!({"roles":plan.roles.len(),"fields":plan.candidates.len(),"omitted":plan.omitted_fields,"batches":plan.batches.len(),"edit_authority":false})
    );
    Ok(())
}
