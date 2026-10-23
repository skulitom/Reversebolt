//! Revalidate a saved model output without target access or inference.
use reversebolt_core::runtime_hypotheses as h;
use serde_json::{Value, json};
use std::io::Read;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: hypothesis_cleanup RECEIPT.json NEW_OUTPUT.json".into());
    }
    let mut bytes = vec![];
    std::fs::File::open(&args[0])?
        .take(12 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("Receipt exceeds budget".into());
    }
    let receipt: Value = serde_json::from_slice(&bytes)?;
    if !receipt["error"].is_null()
        || receipt["cancelled"] != false
        || !receipt["alternative_cleanup"].is_null()
    {
        return Err("Use an unchanged completed original receipt".into());
    }
    let selection: h::Selection = serde_json::from_value(receipt["selection"].clone())?;
    let mut hypotheses = vec![];
    for stage in receipt["stages"].as_array().ok_or("Missing stages")? {
        if !stage["schema"]["properties"]["hypotheses"].is_object() {
            continue;
        }
        let text = stage["response"]["message"]["content"]
            .as_str()
            .ok_or("Missing model output")?;
        let proposals: h::Proposals = serde_json::from_str(text)?;
        hypotheses.extend(proposals.hypotheses);
    }
    let proposals = h::Proposals { hypotheses };
    let original = h::validate(&selection, proposals.clone())?;
    if serde_json::to_value(&original)? != receipt["batch"] {
        return Err("Original batch does not match the retained model output".into());
    }
    let (batch, cleanup) = h::validate_with_self_alternative_cleanup(&selection, proposals)?;
    let result = json!({"source_receipt_sha256":reversebolt_core::visual::digest(&bytes),
        "batch":batch,"cleanup":cleanup,"new_inference_calls":0,"edit_authority":false});
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    serde_json::to_writer_pretty(file, &result)?;
    println!(
        "{}",
        json!({"accepted":batch.accepted.len(),"rejected":batch.rejected.len(),"cleanup_count":cleanup.as_ref().map_or(0,|c|c.removed.len())})
    );
    Ok(())
}
