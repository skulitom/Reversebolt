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
    if args == ["schema"] {
        println!(
            "{}",
            json!({"hypotheses":h::proposal_schema(),
            "referenced_hypotheses":reversebolt_core::runtime_citations::proposal_schema(),
            "references":reversebolt_core::runtime_citations::references_schema(1),
            "relevance":reversebolt_observer::runtime_relevance::schema(8),
            "hypothesis_citation":h::citation_schema(Some(256)),"relevance_citation":h::citation_schema(None)})
        );
        return Ok(());
    }
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
    if args.len() == 2 && args[0] == "check-naming" {
        let directory = Path::new(&args[1]);
        let inventory =
            serde_json::from_value(read(&directory.join("input.json"))?["inventory"].clone())?;
        let receipt: reversebolt_observer::runtime_naming::Receipt =
            serde_json::from_value(read(&directory.join("receipt.json"))?)?;
        if receipt.error.is_some() || receipt.cancelled {
            return Err("Incomplete naming receipt".into());
        }
        let selection = receipt.selection.as_ref().ok_or("No selection")?;
        if h::select(&inventory, &selection.selected_types)? != *selection {
            return Err("Selection differs from source inventory".into());
        }
        let table = receipt
            .citation_evidence
            .as_ref()
            .ok_or("No evidence references")?;
        table.check(&selection.report_sha256, &selection.rows)?;
        let mut hypotheses = vec![];
        for stage in &receipt.stages {
            if stage.schema["properties"].get("hypotheses").is_none() {
                continue;
            }
            if stage.response["done"] != true || stage.response["done_reason"] != "stop" {
                return Err("Incomplete model response".into());
            }
            let proposed: reversebolt_core::runtime_citations::Proposals = serde_json::from_str(
                stage.response["message"]["content"]
                    .as_str()
                    .ok_or("Missing model text")?,
            )?;
            hypotheses.extend(proposed.resolve(table, selection)?.hypotheses);
        }
        let (batch, cleanup) =
            h::validate_with_self_alternative_cleanup(selection, h::Proposals { hypotheses })?;
        let original = receipt
            .object_text_fallback
            .as_ref()
            .map(|f| &f.original_batch)
            .or(receipt.batch.as_ref());
        if original != Some(&batch) || cleanup != receipt.alternative_cleanup {
            return Err("Resolved proposals differ from the canonical checked batch".into());
        }
        if receipt.object_text_fallback.is_some() {
            let mut replay = receipt.clone();
            replay.batch = Some(batch);
            replay.object_text_fallback = None;
            replay.add_object_text_questions(&inventory)?;
            if replay.batch != receipt.batch {
                return Err("Fallback differs from the observed object text".into());
            }
        }
        println!(
            "{}",
            json!({"checked":true,"model_calls":0,"target_access":false})
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
