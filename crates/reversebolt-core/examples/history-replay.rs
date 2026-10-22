//! File-only migration regression. Mutates only a newly created disposable directory.
use reversebolt_core::{
    runtime_test_store::Store,
    runtime_tests::{MAX_TESTS_PER_INVESTIGATION, Phase, Test},
    visual,
};
use serde_json::json;
use std::{fs, io::Read, path::Path};

fn read(path: &Path, max: u64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    fs::File::open(path)?
        .take(max + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err("Input exceeds its budget".into());
    }
    Ok(bytes)
}

fn synthetic(original: &Test, investigation: &str) -> Test {
    let mut test = original.clone();
    test.id = uuid::Uuid::new_v4().to_string();
    test.change_id = uuid::Uuid::new_v4().to_string();
    test.investigation = investigation.into();
    test.actor = "offline-history-fixture".into();
    test.hypothesis = "Synthetic history capacity check; no comparison was performed.".into();
    test.origin = None;
    test.preparation = None;
    test.conditions = Default::default();
    test.assessment = None;
    test.phase = Phase::Stopped;
    test.pending_capture = false;
    test.reason = "Synthetic closed record, not gameplay evidence.".into();
    test
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: history-replay CLOSED_TEST_JSON NEW_OUTPUT_DIRECTORY".into());
    }
    let source = Path::new(&args[0]);
    let bytes = read(source, 512 * 1024)?;
    let original: Test = serde_json::from_slice(&bytes)?;
    original.validate_plan()?;
    if original.active() || source.file_stem().and_then(|s| s.to_str()) != Some(&original.id) {
        return Err("Choose a correctly named closed comparison record".into());
    }
    let output = Path::new(&args[1]);
    fs::create_dir(output)?; // Never operate on an existing store or overwrite a run.
    let directory = output.join("runtime-tests");
    fs::create_dir(&directory)?;
    let record = directory.join(format!("{}.json", original.id));
    fs::write(&record, &bytes)?;
    // Validate all saved relative references before copying any evidence.
    let store = Store::open(directory.clone())?;
    assert_eq!(json!(store.get(&original.id)?), json!(original));
    drop(store);
    let evidence = source
        .parent()
        .ok_or("Missing evidence parent")?
        .join(&original.id);
    let copied = directory.join(&original.id);
    fs::create_dir(&copied)?;
    let mut files = vec![];
    let mut total = 0u64;
    if evidence.exists() {
        for entry in fs::read_dir(evidence)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() || files.len() >= 128 {
                return Err("Unsupported saved evidence layout".into());
            }
            let data = read(&entry.path(), 20 * 1024 * 1024)?;
            total += data.len() as u64;
            if total > 64 * 1024 * 1024 {
                return Err("Saved evidence exceeds its budget".into());
            }
            fs::write(copied.join(entry.file_name()), &data)?;
            files.push(json!({"file":entry.file_name().to_string_lossy(),"sha256":visual::digest(&data),"bytes":data.len()}));
        }
    }
    let mut store = Store::open(directory.clone())?;
    let mut synthetic_ids = vec![];
    for _ in 1..MAX_TESTS_PER_INVESTIGATION {
        let test = synthetic(&original, &original.investigation);
        synthetic_ids.push(test.id.clone());
        store.record(test)?;
    }
    let full = store.capacity(&original.investigation);
    assert_eq!(full.remaining_record_slots, 0);
    let refused = store
        .record(synthetic(&original, &original.investigation))
        .unwrap_err();
    let next_investigation = uuid::Uuid::new_v4().to_string();
    let test = synthetic(&original, &next_investigation);
    synthetic_ids.push(test.id.clone());
    store.record(test)?;
    drop(store);
    let store = Store::open(directory)?;
    assert_eq!(store.records.len(), 9);
    assert_eq!(json!(store.get(&original.id)?), json!(original));
    assert_eq!(read(&record, 512 * 1024)?, bytes);
    let mut checked_images = 0;
    for (c, condition) in original.conditions.iter().enumerate() {
        for (i, _) in condition.frames.iter().enumerate() {
            store.read_image(&original, c, i)?;
            store.read_input(&original, c, i)?;
            checked_images += 1;
        }
    }
    let executions = store.read_executions(&original)?;
    let result = json!({"schema":1,"original_id":original.id,"original_record_sha256":visual::digest(&bytes),
        "original_record_unchanged":true,"original_assessment":original.assessment,"copied_files":files,
        "synthetic_record_ids":synthetic_ids,"retained_records":store.records.len(),"full_investigation":full,
        "same_investigation_refusal":refused,"new_investigation":store.capacity(&next_investigation),
        "images_and_input_receipts_verified":checked_images,"execution_receipts_verified":executions.len(),
        "target_access":false,"input_sent":false,"new_model_calls":0,"field_writes":0,
        "limitation":"Only the original record describes an earlier native experiment. Eight new records are synthetic capacity fixtures with no observations or assessment. This replay establishes persistence and evidence readability, not new gameplay effects."});
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("result.json"))?;
    serde_json::to_writer_pretty(file, &result)?;
    println!(
        "{}",
        json!({"records":store.records.len(),"images_checked":checked_images,"execution_receipts":executions.len(),"target_access":false})
    );
    Ok(())
}
