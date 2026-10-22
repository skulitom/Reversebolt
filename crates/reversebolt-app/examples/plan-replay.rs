//! Offline planning from retained images, input metadata and unmodified model output.
use reversebolt_core::{
    live::LiveView,
    play_evidence::{self, Measurement},
    runtime_inventory, runtime_planning, visual,
};
use serde_json::{Value, json};
use std::{io::Read, path::Path};

fn document(path: &Path) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    std::fs::File::open(path)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Replay document exceeds its budget".into());
    }
    Ok((serde_json::from_slice(&bytes)?, visual::digest(&bytes)))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err(
            "usage: plan-replay STATUS_JSON CAPTURE_DIRECTORY MODEL_RECEIPT OUTPUT_JSON".into(),
        );
    }
    let (status, status_hash) = document(Path::new(&args[0]))?;
    let live: LiveView = serde_json::from_value(status["state"]["live"].clone())?;
    if !(2..=reversebolt_core::live::MAX_SAMPLES).contains(&live.frames.len()) {
        return Err("Replay requires 2-36 retained frames".into());
    }
    let (raw_receipt, receipt_hash) = document(Path::new(&args[2]))?;
    let receipt: reversebolt_observer::runtime_naming::Receipt =
        serde_json::from_value(raw_receipt)?;
    let original = live.runtime.naming.as_ref().ok_or("No metadata review")?;
    if receipt.cancelled
        || receipt.error.is_some()
        || original.status != "complete"
        || receipt.report_sha256 != original.report_sha256
        || receipt.batch != original.batch
        || receipt.model != original.model
    {
        return Err("Model receipt does not match the completed review".into());
    }
    let source = live
        .runtime
        .catalog_source
        .as_ref()
        .ok_or("No report source")?;
    let (report, report_file_hash) = document(Path::new(&source.path))?;
    let inventory = runtime_inventory::build(&report)?;
    if inventory.report_sha256 != original.report_sha256 {
        return Err("Runtime report changed".into());
    }
    let mut previous: Option<Vec<u8>> = None;
    let mut measurements = vec![];
    let mut images = vec![];
    for (i, frame) in live.frames.iter().enumerate() {
        let path = Path::new(&args[1]).join(format!("frame-{:03}.png", frame.number));
        let bytes = visual::image_bytes(&path)?;
        if visual::digest(&bytes) != frame.sha256 {
            return Err("Retained image hash changed".into());
        }
        if let Some(prior) = previous {
            let a = &live.frames[i - 1];
            let measured =
                if (a.width, a.height, a.epoch) != (frame.width, frame.height, frame.epoch) {
                    Err("Image geometry or scene epoch changed.".into())
                } else {
                    reversebolt_vision::response::difference(&prior, &bytes)
                };
            measurements.push(match measured {
                Ok(difference) => Measurement {
                    difference: Some(difference),
                    error: None,
                },
                Err(error) => Measurement {
                    difference: None,
                    error: Some(error),
                },
            });
        }
        previous = Some(bytes);
        images.push(json!({"id":frame.id,"number":frame.number,"sha256":frame.sha256}));
    }
    let evidence = play_evidence::analyze(&live.frames, measurements)?;
    let activity = receipt
        .activity
        .as_ref()
        .ok_or("No retained activity review")?;
    let selected = reversebolt_core::play_context::select(
        &live.frames,
        &evidence,
        &live.investigation_id,
        live.coverage.batch,
    )?;
    if selected.as_ref() != Some(&activity.source) {
        return Err(
            "Recomputed image/input evidence differs from the retained activity source".into(),
        );
    }
    let mut revised = original.clone();
    revised.play_context = activity.context();
    let context = revised
        .play_context
        .as_ref()
        .ok_or("No valid measured activity context")?;
    let plan = (|| -> reversebolt_core::Result<Value> {
        let actions = runtime_planning::actions(context, &live.frames, &evidence)?;
        let tests: Vec<reversebolt_core::runtime_tests::Test> =
            serde_json::from_value(status["state"]["runtime_tests"].clone())
                .map_err(|e| e.to_string())?;
        let choices = runtime_planning::choices(
            &revised,
            &inventory,
            &actions,
            &tests,
            &live.investigation_id,
            &live.runtime.catalog_id,
        )?;
        let next = choices.iter().find(|c| c["status"] == "untried");
        Ok(json!({"observed_actions":actions,"choices":choices,"next":next}))
    })();
    let (plan, error) = match plan {
        Ok(plan) => (Some(plan), None),
        Err(error) => (None, Some(error)),
    };
    let output = json!({"schema":1,"original_context":original.play_context,"revised_context":context,
        "revised_review":revised,"plan":plan,"error":error,"evidence":evidence,
        "input_hashes":{"status":status_hash,"receipt":receipt_hash,"report_file":report_file_hash,"images":images},
        "metadata_model_calls":0,"target_access":false,"input_sent":false,"field_writes":0,
        "limitation":"Offline replay of a known development case. It proposes an investigation; it does not validate a field's meaning, action delivery, gameplay effect or edit permission."});
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[3])?;
    serde_json::to_writer_pretty(file, &output)?;
    println!(
        "{}",
        json!({"basis":context.basis,"model_consistent_visible_change":context.observation.consistent_visible_change,
        "plan_available":plan.is_some(),"error":error,"choices":plan.as_ref().map(|p|p["choices"].as_array().unwrap().len()),
        "metadata_model_calls":0,"target_access":false})
    );
    Ok(())
}
