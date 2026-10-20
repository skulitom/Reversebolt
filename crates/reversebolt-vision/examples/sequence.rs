//! Image-only evaluation. The output contains no supplied labels or game rules.
use sha2::{Digest, Sha256};
use std::{io::Write, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("Usage: sequence <models> <frames> <new-output.json>".into());
    }
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?;
    let mut reader = reversebolt_vision::Reader::load(&args[0])?;
    let mut paths = std::fs::read_dir(&args[1])?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut frames = vec![];
    for path in paths {
        let image = image::open(&path)?.to_rgb8();
        let reading = reader.read(&image)?;
        println!(
            "{}: {} ms, {:?}",
            path.file_name().unwrap().to_string_lossy(),
            reading.elapsed_ms,
            reading
                .words
                .iter()
                .filter(|w| reversebolt_core::live::parse_number(&w.text).is_some())
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
        );
        frames.push(serde_json::json!({"path":path,"sha256":format!("{:x}",Sha256::digest(std::fs::read(&path)?)),
            "elapsed_ms":reading.elapsed_ms,"words":reading.words,"lines":reading.lines}));
    }
    out.write_all(&serde_json::to_vec_pretty(&serde_json::json!({"implementation":"reversebolt-vision native prototype","frames":frames}))?)?;
    Ok(())
}
