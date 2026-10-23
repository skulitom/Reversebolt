//! Offline evaluator; no process access. Opaque mode is explicitly research-only.
use std::{io::Read, path::PathBuf, sync::atomic::AtomicBool};
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivityManifest {
    source: reversebolt_core::play_context::Source,
    frames: Vec<PathBuf>,
    #[serde(default)]
    appearances: Option<reversebolt_core::appearance_activity::Summary>,
    #[serde(default)]
    regions: Option<reversebolt_core::activity_regions::Summary>,
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
        appearances: manifest.appearances,
        regions: manifest.regions,
    };
    input.validate()?;
    Ok(input)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        return Err("usage: reversebolt-runtime-hypotheses REPORT OUTPUT MODEL [--opaque] [--select-only [--current-state]] [--activity MANIFEST] [--replay RECEIPT]".into());
    }
    let bytes = std::fs::read(&args[0])?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Report budget exceeded".into());
    }
    let report = serde_json::from_slice(&bytes)?;
    let mut inventory = reversebolt_core::runtime_inventory::build(&report)?;
    let mut opaque = false;
    let mut activity_path = None;
    let mut replay_path = None;
    let mut selection_only = false;
    let mut current_state = false;
    let mut options = args[3..].iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--opaque" if !opaque => opaque = true,
            "--select-only" if !selection_only => selection_only = true,
            "--current-state" if !current_state => current_state = true,
            "--activity" if activity_path.is_none() => {
                activity_path = Some(options.next().ok_or("Missing activity manifest")?)
            }
            "--replay" if replay_path.is_none() => {
                replay_path = Some(options.next().ok_or("Missing replay receipt")?)
            }
            _ => return Err("Unknown or repeated mode".into()),
        }
    }
    if replay_path.is_some() && activity_path.is_some() {
        return Err("Metadata replay cannot add new activity inference".into());
    }
    if selection_only && (replay_path.is_some() || activity_path.is_some()) {
        return Err("Type selection alone cannot add activity or replay naming.".into());
    }
    if current_state && !selection_only {
        return Err("Current-state selection requires --select-only.".into());
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
    let replay = if let Some(path) = replay_path {
        let mut bytes = vec![];
        std::fs::File::open(path)?
            .take(12 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 12 * 1024 * 1024 {
            return Err("Replay receipt exceeds its budget".into());
        }
        let mut receipt: reversebolt_observer::runtime_naming::Receipt =
            serde_json::from_slice(&bytes)?;
        if receipt.model != args[2] || receipt.activity.is_some() {
            return Err(
                "Replay requires the same model identity and metadata-only evidence".into(),
            );
        }
        receipt.add_object_text_questions(&inventory)?;
        Some(receipt)
    } else {
        None
    };
    std::fs::create_dir(&output)?;
    let receipt = if let Some(receipt) = replay {
        receipt
    } else {
        tokio::runtime::Runtime::new()?.block_on(async {
            let observer = reversebolt_observer::Observer::connect(&args[2]).await?;
            Ok::<_, String>(if selection_only {
                observer
                    .runtime_type_selection_for(
                        &inventory,
                        if current_state {
                            reversebolt_observer::runtime_naming::SelectionPurpose::CurrentState
                        } else {
                            reversebolt_observer::runtime_naming::SelectionPurpose::AdjustableParameters
                        },
                        &AtomicBool::new(false),
                    )
                    .await
            } else {
                observer
                    .runtime_hypotheses_with_activity(
                        &inventory,
                        activity.as_ref(),
                        &AtomicBool::new(false),
                    )
                    .await
            })
        })?
    };
    std::fs::write(
        output.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    std::fs::write(
        output.join("input.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"opaque_research_only":opaque,"source":args[0],"inventory":inventory,"activity_manifest":activity_path,"replay_receipt":replay_path}),
        )?,
    )?;
    println!(
        "{}",
        serde_json::json!({"output":output,"selection_only":selection_only,"error":receipt.error,"cancelled":receipt.cancelled,
        "elapsed_ms_is_retained":replay_path.is_some(),
        "question_source":receipt.question_source(),"question_policy":receipt.question_policy(),"metadata_model_calls":if replay_path.is_some(){0}else{receipt.stages.len()},
        "accepted":receipt.batch.as_ref().map(|b|b.accepted.len()),"rejected":receipt.batch.as_ref().map(|b|b.rejected.len()),
        "elapsed_ms":receipt.stages.iter().map(|s|s.elapsed_ms).sum::<u128>()})
    );
    Ok(())
}
