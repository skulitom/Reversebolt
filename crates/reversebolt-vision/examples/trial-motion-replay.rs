//! Saved-file replay. Does not open a mutable store, target or model.
use reversebolt_core::{runtime_tests::Test, visual};
use serde_json::json;
use std::{fs, io::Read, path::Path};

fn bounded(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err("Saved input exceeds its size limit.".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: trial-motion-replay TEST_JSON NEW_OUTPUT_JSON".into());
    }
    let path = Path::new(&args[0]);
    let bytes = bounded(path, 512 * 1024)?;
    let test: Test = serde_json::from_slice(&bytes)?;
    let folder = path
        .parent()
        .ok_or("Missing evidence directory")?
        .join(&test.id);
    let motion = reversebolt_vision::trial_motion::analyze(
        &test,
        |c, i| visual::image_bytes(&folder.join(format!("{c}-{i}.png"))),
        |c, i| {
            serde_json::from_slice(&bounded(
                &folder.join(format!("{c}-{i}.input.json")),
                1024 * 1024,
            )?)
            .map_err(|e| e.to_string())
        },
    )?;
    let output = json!({"record_file_sha256":visual::digest(&bytes),"original_assessment":test.assessment,
        "trial_motion_comparison":motion,"target_access":false,"new_captures":0,"input_sent":false,"field_writes":0,"model_calls":0});
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    serde_json::to_writer_pretty(file, &output)?;
    println!("{}", json!({"test_id":test.id,"status":motion.status}));
    Ok(())
}
