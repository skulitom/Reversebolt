//! Offline measurement of retained image pairs. Does not capture or edit a target.
use std::{fs, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = std::env::args()
        .nth(1)
        .ok_or("Supply a JSON array of before/after file paths.")?;
    let pairs: Vec<[String; 2]> = serde_json::from_slice(&fs::read(manifest)?)?;
    if pairs.len() > 256 {
        return Err("At most 256 retained image pairs.".into());
    }
    let mut results = vec![];
    for pair in pairs {
        let a = reversebolt_core::visual::image_bytes(Path::new(&pair[0]))?;
        let b = reversebolt_core::visual::image_bytes(Path::new(&pair[1]))?;
        results.push(serde_json::json!({"paths":pair,"hashes":[reversebolt_core::visual::digest(&a),reversebolt_core::visual::digest(&b)],
            "difference":reversebolt_vision::response::difference(&a,&b)?}));
    }
    println!(
        "{}",
        serde_json::json!({"metric":reversebolt_vision::response::METRIC_ID,"pairs":results})
    );
    Ok(())
}
