use reversebolt_core::runtime_recovery;
use serde_json::Value;
use std::{fs, io::Read};
fn read(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    fs::File::open(path)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Report exceeds budget".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err("usage: runtime_recovery OLD_REPORT PATH_ID NEW_REPORT NEW_OUTPUT".into());
    }
    let recovered = runtime_recovery::find(&read(&args[0])?, &args[1], &read(&args[2])?)?;
    let out = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[3])?;
    serde_json::to_writer_pretty(out, &recovered)?;
    println!(
        "{}",
        serde_json::json!({"status":recovered.status,"same_process":recovered.same_process,
        "path_id":recovered.candidate.as_ref().map(|c| &c.id),"edit_authority":false})
    );
    Ok(())
}
