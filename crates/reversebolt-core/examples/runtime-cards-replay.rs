//! File-only replay of a retained discovery review into its original card list.
use reversebolt_core::{Result, runtime, runtime_cards, visual::digest};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::{Read, Write},
    path::Path,
};

fn read(path: &str) -> Result<(Vec<u8>, Value)> {
    let mut bytes = vec![];
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Saved input exceeds 64 MiB.".into());
    }
    let value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    Ok((bytes, value))
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("Use DISCOVERY_JSON REPORT_JSON NEW_OUTPUT_JSON.".into());
    }
    let (snapshot_bytes, snapshot) = read(&args[1])?;
    let (report_bytes, report) = read(&args[2])?;
    let mut view: runtime::View =
        serde_json::from_value(snapshot["state"]["live"]["runtime"].clone())
            .map_err(|e| e.to_string())?;
    let review = view.naming.clone().ok_or("Missing retained review.")?;
    let accepted = &review
        .batch
        .as_ref()
        .ok_or("Missing retained batch.")?
        .accepted;
    let before = view.clone();
    let initially_present: Vec<_> = accepted
        .iter()
        .filter(|h| {
            before
                .cards
                .iter()
                .any(|c| c.candidate.id == h.candidate.path_id)
        })
        .map(|h| h.candidate.path_id.clone())
        .collect();
    // This replay evaluates original, read-only discovery snapshots only.
    for key in ["freezes", "runtime_trials", "runtime_tests"] {
        if snapshot["state"][key]
            .as_array()
            .is_some_and(|a| !a.is_empty())
        {
            return Err(format!("Use a discovery snapshot with no {key}."));
        }
    }
    let publication = runtime_cards::include(&mut view, &report, &review, &BTreeSet::new())?;
    let after = json!(view);
    let repeated = runtime_cards::include(&mut view, &report, &review, &BTreeSet::new())?;
    if json!(view) != after || !repeated.added.is_empty() || !repeated.removed_preview.is_empty() {
        return Err("Repeated card publication changed the saved readings.".into());
    }
    let output = json!({"schema":1,"snapshot_sha256":digest(&snapshot_bytes),"report_file_sha256":digest(&report_bytes),
        "review_id":review.evidence_id,"report_sha256":review.report_sha256,"initially_present":initially_present,
        "publication":publication,"repeat_unchanged":true,"after":after,
        "new_model_calls":0,"target_access":false,"new_captures":0,"input_sent":false,"field_writes":0,
        "limitation":"Retained metadata hypotheses and historical scalar values; no new semantic or gameplay validation."});
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(&args[3]))
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(&output).map_err(|e| e.to_string())?)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    println!(
        "{} primary paths, {} initially present, {} added, {} omitted",
        accepted.len(),
        initially_present.len(),
        publication.added.len(),
        publication.omitted.len()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
