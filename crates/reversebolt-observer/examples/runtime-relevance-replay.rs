//! Review existing candidates without seeing the generator's claims or accessing a target.
use reversebolt_core::{runtime_hypotheses as h, runtime_relevance as r};
use reversebolt_observer::runtime_relevance::Receipt;
use serde_json::{Value, json};
use std::{io::Read, path::Path, sync::atomic::AtomicBool};

fn read(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    std::fs::File::open(path)?
        .take(12 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("Research input exceeds its budget".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[0] == "check" {
        let directory = Path::new(&args[1]);
        let input: r::Input =
            serde_json::from_value(read(&directory.join("input.json"))?["input"].clone())?;
        let receipt: Receipt = serde_json::from_value(read(&directory.join("receipt.json"))?)?;
        let assessment = receipt.checked_assessment(&input)?;
        println!(
            "{}",
            json!({"checked":true,"assessment":assessment,"model_calls":0,"target_access":false})
        );
        return Ok(());
    }
    if args.len() != 4 {
        return Err("usage: runtime-relevance-replay SOURCE NEW_OUTPUT MODEL original|reversed|opaque|synthetic".into());
    }
    let source = Path::new(&args[0]);
    let mut input = if args[3] == "synthetic" {
        let saved = read(source)?;
        if saved["synthetic"] != true {
            return Err("Authored challenge input must declare its synthetic origin".into());
        }
        let input: r::Input = serde_json::from_value(saved["input"].clone())?;
        input.validate()?;
        input
    } else {
        let inputs = read(&source.join("input.json"))?;
        let inventory = serde_json::from_value(inputs["inventory"].clone())?;
        let naming: reversebolt_observer::runtime_naming::Receipt =
            serde_json::from_value(read(&source.join("receipt.json"))?)?;
        if naming.error.is_some() || naming.cancelled {
            return Err("Source naming did not complete".into());
        }
        let selection = naming.selection.as_ref().ok_or("No source selection")?;
        if h::select(&inventory, &selection.selected_types)? != *selection {
            return Err("Source inventory changed".into());
        }
        r::Input::from_batch(selection, naming.batch.as_ref().ok_or("No source batch")?)?
    };
    match args[3].as_str() {
        "original" | "synthetic" => {}
        "reversed" => input.candidates.reverse(),
        "opaque" => {
            for row in &mut input.candidates {
                row.type_name = "Container".into();
                row.field_name = format!("F{:010x}", row.id);
                row.object_text.clear();
            }
        }
        _ => return Err("Unknown research variation".into()),
    }
    let output = Path::new(&args[1]);
    std::fs::create_dir(output)?;
    std::fs::write(
        output.join("input.json"),
        serde_json::to_vec_pretty(
            &json!({"source":args[0],"variation":args[3],"synthetic":args[3] == "synthetic","input":input}),
        )?,
    )?;
    let receipt = tokio::runtime::Runtime::new()?.block_on(async {
        reversebolt_observer::Observer::connect(&args[2])
            .await?
            .review_runtime_relevance(&input, &AtomicBool::new(false))
            .await
    })?;
    std::fs::write(
        output.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    println!(
        "{}",
        json!({"error":receipt.error,"assessment":receipt.assessment,"model_calls":usize::from(receipt.stage.is_some()),"target_access":false})
    );
    Ok(())
}
