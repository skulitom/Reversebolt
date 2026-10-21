//! Offline evaluator; no process access. Opaque mode is explicitly research-only.
use std::{io::Read, path::PathBuf, sync::atomic::AtomicBool};
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivityManifest {
    source: reversebolt_core::play_context::Source,
    frames: Vec<PathBuf>,
}
fn activity(
    path: &str,
) -> Result<reversebolt_observer::runtime_activity::Input, Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    std::fs::File::open(path)?
        .take(64 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 {
        return Err("Activity manifest exceeds 64 KiB".into());
    }
    let manifest: ActivityManifest = serde_json::from_slice(&bytes)?;
    manifest.source.validate()?;
    if manifest.frames.len() != 4 {
        return Err("Supply exactly four activity images".into());
    }
    let frames = manifest
        .frames
        .iter()
        .zip(manifest.source.frames())
        .map(|(path, reference)| {
            reversebolt_observer::Frame::read(
                path,
                reference
                    .interval
                    .begin_us
                    .checked_mul(1000)
                    .ok_or("Activity clock overflow")?,
            )
        })
        .collect::<Result<Vec<_>, String>>()?;
    let input = reversebolt_observer::runtime_activity::Input {
        source: manifest.source,
        frames,
    };
    input.validate()?;
    Ok(input)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        return Err("usage: reversebolt-runtime-hypotheses REPORT OUTPUT MODEL [--opaque] [--activity MANIFEST]".into());
    }
    let bytes = std::fs::read(&args[0])?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Report budget exceeded".into());
    }
    let report = serde_json::from_slice(&bytes)?;
    let mut inventory = reversebolt_core::runtime_inventory::build(&report)?;
    let mut opaque = false;
    let mut activity_path = None;
    let mut options = args[3..].iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--opaque" if !opaque => opaque = true,
            "--activity" if activity_path.is_none() => {
                activity_path = Some(options.next().ok_or("Missing activity manifest")?)
            }
            _ => return Err("Unknown or repeated mode".into()),
        }
    }
    let activity = activity_path.map(|p| activity(p)).transpose()?;
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
                .runtime_hypotheses_with_activity(
                    &inventory,
                    activity.as_ref(),
                    &AtomicBool::new(false),
                )
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
            &serde_json::json!({"opaque_research_only":opaque,"source":args[0],"inventory":inventory,"activity_manifest":activity_path}),
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
