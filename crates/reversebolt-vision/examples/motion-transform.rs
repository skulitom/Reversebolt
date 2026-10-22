//! Controlled transforms of saved images; not observed game movement.
use image::{GrayImage, Luma, imageops::FilterType};
use reversebolt_core::visual;
use serde::Deserialize;
use serde_json::json;
use std::{fs, io::Read, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    name: String,
    file: String,
    sha256: String,
}
fn png(image: &GrayImage) -> Result<Vec<u8>, image::ImageError> {
    let mut data = std::io::Cursor::new(vec![]);
    image.write_to(&mut data, image::ImageFormat::Png)?;
    Ok(data.into_inner())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: motion-transform SOURCES_JSON NEW_OUTPUT_JSON".into());
    }
    let mut manifest = vec![];
    fs::File::open(&args[0])?
        .take(1048577)
        .read_to_end(&mut manifest)?;
    if manifest.len() > 1048576 {
        return Err("Manifest exceeds budget".into());
    }
    let sources: Vec<Source> = serde_json::from_slice(&manifest)?;
    if sources.is_empty() || sources.len() > 8 {
        return Err("Choose 1-8 images".into());
    }
    let mut cases = vec![];
    for source in sources {
        let bytes = visual::image_bytes(Path::new(&source.file))?;
        if visual::digest(&bytes) != source.sha256 {
            return Err("Source hash changed".into());
        }
        let (width, height) =
            image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Png)
                .into_dimensions()?;
        if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 4_000_000 {
            return Err("Image exceeds pixel budget".into());
        }
        let mut reader =
            image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Png);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        let rgb = reader.decode()?.to_rgb8();
        if u64::from(rgb.width()) * u64::from(rgb.height()) > 4_000_000 {
            return Err("Image exceeds pixel budget".into());
        }
        let gray = GrayImage::from_fn(rgb.width(), rgb.height(), |x, y| {
            let p = rgb.get_pixel(x, y);
            Luma([
                ((299 * u32::from(p[0]) + 587 * u32::from(p[1]) + 114 * u32::from(p[2]) + 500)
                    / 1000) as u8,
            ])
        });
        let normalized = image::imageops::resize(&gray, 256, 144, FilterType::Triangle);
        let before = png(&normalized)?;
        for delta in [[0, 0], [3, -2], [18, 8], [-24, -10]] {
            let after = GrayImage::from_fn(256, 144, |x, y| {
                let (sx, sy) = (x as i32 - delta[0], y as i32 - delta[1]);
                if (0..256).contains(&sx) && (0..144).contains(&sy) {
                    Luma([normalized.get_pixel(sx as u32, sy as u32)[0].saturating_add(20)])
                } else {
                    Luma([0])
                }
            });
            let after = png(&after)?;
            let report = reversebolt_vision::patch_motion::measure(&before, &after)?;
            let errors: Vec<_> = report
                .patches
                .iter()
                .filter_map(|p| {
                    p.displacement.map(|d| {
                json!({"anchor":p.anchor,"displacement":d,"error":[d[0]-delta[0],d[1]-delta[1]]})
            })
                })
                .collect();
            let wrong = report
                .patches
                .iter()
                .filter_map(|p| p.displacement)
                .filter(|d| (0..2).any(|i| (d[i] - delta[i]).abs() > 1))
                .count();
            cases.push(json!({"source":source.name,"source_sha256":source.sha256,
                "before_sha256":visual::digest(&before),"after_sha256":visual::digest(&after),
                "expected_displacement":delta,"brightness_added":20,"measurement":report,
                "accepted_errors":errors,"wrong_matches_over_one_pixel":wrong}));
        }
    }
    let result = json!({"manifest_sha256":visual::digest(&manifest),"cases":cases,
        "limitation":"Controlled image transformations, not real-game flow truth or validation of gameplay effects."});
    let output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    serde_json::to_writer_pretty(output, &result)?;
    println!("{}", json!({"cases":cases.len(),"target_access":false}));
    Ok(())
}
