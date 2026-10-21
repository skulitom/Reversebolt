//! Offline evaluator; no process access. Opaque mode is explicitly research-only.
use std::{path::PathBuf, sync::atomic::AtomicBool};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(3..=4).contains(&args.len()) {
        return Err("usage: reversebolt-runtime-hypotheses REPORT OUTPUT MODEL [--opaque]".into());
    }
    let bytes = std::fs::read(&args[0])?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Report budget exceeded".into());
    }
    let report = serde_json::from_slice(&bytes)?;
    let mut inventory = reversebolt_core::runtime_inventory::build(&report)?;
    let opaque = args.get(3).is_some_and(|s| s == "--opaque");
    if args.len() == 4 && !opaque {
        return Err("Unknown mode".into());
    }
    if opaque {
        for t in &mut inventory.types {
            t.key.type_name = format!("T{:010x}", t.id);
            t.key.image = "ObservedAssembly".into();
            for f in &mut t.fields {
                f.name = format!("F{:010x}", f.field_id);
                for s in &mut f.samples {
                    s.context.clear();
                }
            }
        }
    }
    let output = PathBuf::from(&args[1]);
    std::fs::create_dir(&output)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let receipt = runtime.block_on(async {
        let observer = reversebolt_observer::Observer::connect(&args[2]).await?;
        Ok::<_, String>(
            observer
                .runtime_hypotheses(&inventory, &AtomicBool::new(false))
                .await,
        )
    })?;
    std::fs::write(
        output.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    std::fs::write(
        output.join("input.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"opaque_research_only":opaque,"source":args[0],"inventory":inventory}),
        )?,
    )?;
    println!(
        "{}",
        serde_json::json!({"output":output,"error":receipt.error,"cancelled":receipt.cancelled,
        "accepted":receipt.batch.as_ref().map(|b|b.accepted.len()),"rejected":receipt.batch.as_ref().map(|b|b.rejected.len()),
        "elapsed_ms":receipt.stages.iter().map(|s|s.elapsed_ms).sum::<u128>()})
    );
    Ok(())
}
