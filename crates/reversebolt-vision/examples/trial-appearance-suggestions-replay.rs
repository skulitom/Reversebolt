//! File-only baseline suggestions. Later condition files are never opened.
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
        return Err("Saved file exceeds its size limit.".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(2..=3).contains(&args.len()) {
        return Err("Use TEST_JSON NEW_OUTPUT_JSON [change_only|stable_texture].".into());
    }
    let ranking = match args.get(2).map(String::as_str).unwrap_or("stable_texture") {
        "change_only" => reversebolt_vision::trial_appearance_suggestions::Ranking::ChangeOnly,
        "stable_texture" => {
            reversebolt_vision::trial_appearance_suggestions::Ranking::StableTexture
        }
        _ => return Err("Unknown baseline ranking policy.".into()),
    };
    let path = Path::new(&args[0]);
    let bytes = bounded(path, 512 * 1024)?;
    let test: Test = serde_json::from_slice(&bytes)?;
    let folder = path
        .parent()
        .ok_or("Missing evidence directory")?
        .join(&test.id);
    let mut images_read = vec![];
    let mut inputs_read = vec![];
    let result = reversebolt_vision::trial_appearance_suggestions::analyze_with_ranking(
        &test,
        |c, i| {
            assert_eq!(c, 0);
            images_read.push([c, i]);
            visual::image_bytes(&folder.join(format!("{c}-{i}.png")))
        },
        |c, i| {
            assert_eq!(c, 0);
            inputs_read.push([c, i]);
            serde_json::from_slice(&bounded(
                &folder.join(format!("{c}-{i}.input.json")),
                1024 * 1024,
            )?)
            .map_err(|e| e.to_string())
        },
        ranking,
    );
    let (report, error) = match result {
        Ok(report) => (Some(report), None),
        Err(error) => (None, Some(error)),
    };
    let output = json!({"record_file_sha256":visual::digest(&bytes),"report":report,"error":error,
        "images_read":images_read,"inputs_read":inputs_read,"target_access":false,"new_captures":0,"input_sent":false,"field_writes":0,"model_calls":0});
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    serde_json::to_writer_pretty(file, &output)?;
    println!(
        "{}",
        json!({"test_id":test.id,"status":output["report"]["status"],"error":output["error"]})
    );
    Ok(())
}
