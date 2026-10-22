//! Development replay; no target access, input, field write or new screenshot.
use reversebolt_core::{runtime_priority as p, visual};
use serde_json::{Value, json};
use std::{io::Read, path::Path, sync::atomic::AtomicBool};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[0] == "check" {
        let dir = Path::new(&args[1]);
        let input: Value = serde_json::from_slice(&std::fs::read(dir.join("input.json"))?)?;
        let input: p::Input = serde_json::from_value(input["input"].clone())?;
        let receipt: reversebolt_observer::runtime_priority::Receipt =
            serde_json::from_slice(&std::fs::read(dir.join("receipt.json"))?)?;
        let preference = receipt.checked_preference(&input)?;
        let mut choices: Vec<_> = input
            .candidates
            .iter()
            .map(
                |c| json!({"selected_candidate":c.row,"value":c.proposed_value,"status":"untried"}),
            )
            .collect();
        p::apply(&input, preference, &mut choices)?;
        println!(
            "{}",
            json!({"input_sha256":input.digest()?,"checked":true,
            "ordered_paths":choices.iter().map(|c|c["selected_candidate"]["path_id"].clone()).collect::<Vec<_>>(),
            "model_calls":0,"field_writes":0,"new_images":0})
        );
        return Ok(());
    }
    if args.len() != 4 {
        return Err("usage: comparison-priority-replay CHOSEN_PLAN NEW_OUTPUT_DIR MODEL original|reversed|opaque|other_activity".into());
    }
    let mut bytes = vec![];
    std::fs::File::open(&args[0])?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Plan exceeds budget".into());
    }
    let saved: Value = serde_json::from_slice(&bytes)?;
    let plan = &saved["plan"];
    let mut input = p::Input::from_choices(
        plan["report_sha256"]
            .as_str()
            .ok_or("Missing report digest")?,
        &serde_json::from_value(plan["activity_context"].clone())?,
        &serde_json::from_value(plan["observed_actions"].clone())?,
        plan["choices"].as_array().ok_or("Missing choices")?,
    )?;
    match args[3].as_str() {
        "original" => {}
        "reversed" => input.candidates.reverse(),
        "opaque" => {
            for (i, c) in input.candidates.iter_mut().enumerate() {
                c.row.type_name = "Container".into();
                c.row.field_name = format!("F{i}");
                c.row.object_text.clear();
            }
        }
        "other_activity" => {
            input.activity.motion = None;
            input.activity.appearances = None;
            input.activity.regions = None;
            input.activity.observation.description="In both pairs the visible surfaces become brighter. Their shapes and positions appear unchanged.".into();
            input.activity.observation.uncertainty =
                "The source of the brightness change is unknown; no movement is established."
                    .into();
        }
        _ => return Err("Unknown research variation".into()),
    }
    let output = Path::new(&args[1]);
    std::fs::create_dir(output)?;
    std::fs::write(
        output.join("input.json"),
        serde_json::to_vec_pretty(
            &json!({"source_sha256":visual::digest(&bytes),"variation":args[3],"input":input,
        "limitation":"Known development case; transformed inputs are synthetic robustness checks, never game evidence."}),
        )?,
    )?;
    let receipt = tokio::runtime::Runtime::new()?.block_on(async {
        reversebolt_observer::Observer::connect(&args[2])
            .await?
            .prioritize_runtime_comparisons(&input, &AtomicBool::new(false))
            .await
    })?;
    std::fs::write(
        output.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    println!(
        "{}",
        json!({"variation":args[3],"error":receipt.error,"preference":receipt.preference,
        "elapsed_ms":receipt.stage.as_ref().map(|s|s.elapsed_ms),"new_images":0,"field_writes":0})
    );
    Ok(())
}
