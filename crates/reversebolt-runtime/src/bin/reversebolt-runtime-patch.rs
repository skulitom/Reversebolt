//! Prepare an explicit bounded path patch or its guarded inverse from saved evidence.
use reversebolt_core::{changes::now_ms, runtime::Locator, runtime_patch::Patch};
use serde_json::{Value, json};
use std::{io::Read, path::Path};

fn read(path: &str) -> Result<Value, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("Patch evidence exceeds 64 KiB.".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let patch = match args.get(1).map(String::as_str) {
        Some("prepare") if args.len() == 6 => {
            let path: Locator = serde_json::from_value(read(&args[2])?).map_err(|e| e.to_string())?;
            let report = read(&args[3])?;
            if report["scope"] != json!(path.scope) || report["operation"] != "refresh_runtime_paths" || report["edit_authority"] != false {
                return Err("Expected a fresh read report for this process.".into());
            }
            let checks = report["checks"].as_array().ok_or("Missing path reading.")?;
            if checks.len() != 1 { return Err("Prepare one selected path at a time.".into()); }
            Patch::prepare(path, &checks[0], &args[5], now_ms())?
        }
        Some("restore") if args.len() == 5 => {
            let patch: Patch = serde_json::from_value(read(&args[2])?).map_err(|e| e.to_string())?;
            patch.restoration(&read(&args[3])?, now_ms())?
        }
        _ => return Err("usage: reversebolt-runtime-patch prepare LOCATOR_JSON REFRESH_JSON NEW_OUTPUT_JSON DESIRED | restore PATCH_JSON WRITE_REPORT_JSON NEW_OUTPUT_JSON".into()),
    };
    if !Path::new(&args[4]).is_absolute() {
        return Err("Absolute new output required.".into());
    }
    let output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[4])
        .map_err(|e| e.to_string())?;
    serde_json::to_writer(&output, &patch).map_err(|e| e.to_string())?;
    output.sync_all().map_err(|e| e.to_string())?;
    println!(
        "{}",
        json!({"patch_id":patch.id(),"gameplay_write_attempted":false})
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
