//! Read-only selection over saved session metadata. Never connects to a target.
use reversebolt_core::{Result, activity_experiment::selection, live::LiveView, visual};
use serde_json::{Value, json};
use std::path::Path;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("Supply a saved status JSON path")?;
    if args.next().is_some() {
        return Err("Supply exactly one status path".into());
    }
    let bytes = std::fs::read(Path::new(&path)).map_err(|e| e.to_string())?;
    let document: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let live: LiveView =
        serde_json::from_value(document["state"]["live"].clone()).map_err(|e| e.to_string())?;
    let first = selection::suggest(&live, &[])?;
    let mut excluded = vec![];
    let mut ordering = vec![];
    while let Some(choice) = selection::suggest(&live, &excluded)?.next {
        excluded.push(choice.key);
        ordering.push(choice);
    }
    println!("{}", serde_json::to_string_pretty(&json!({"source_sha256":visual::digest(&bytes),"initial":first,
        "hypothetical_proposal_order":ordering,"limitation":"Read-only metadata replay on previously exposed recordings. Exclusion is hypothetical; no input experiment was executed and no useful gameplay response is established."})).map_err(|e| e.to_string())?);
    Ok(())
}
