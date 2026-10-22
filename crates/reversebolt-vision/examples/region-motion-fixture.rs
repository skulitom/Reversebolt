//! Analytic image transforms for regression only; never gameplay evidence.
use image::{
    GrayImage, Luma,
    imageops::{self, FilterType},
};
use reversebolt_core::visual;
use reversebolt_vision::{region_motion::Region, response};
use serde_json::json;
use std::{fs, path::Path};

fn texture(width: u32, height: u32, seed: u32) -> GrayImage {
    GrayImage::from_fn(width, height, |x, y| {
        let mut n = (x + y * width)
            .wrapping_add(seed)
            .wrapping_mul(747796405)
            .wrapping_add(2891336453);
        n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
        Luma([32 + (((n >> 22) ^ n) % 128) as u8])
    })
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 1 {
        return Err("usage: region-motion-fixture NEW_DIRECTORY".into());
    }
    fs::create_dir(&args[0])?;
    let directory = Path::new(&args[0]).canonicalize()?;
    let background = imageops::resize(&texture(256, 144, 4), 1024, 576, FilterType::Nearest);
    let mut cases = vec![];
    let mut truth = vec![];
    // All cases are declared in advance; no matcher result chooses a transform.
    for (name, width, height, shift, plain, outside, expected) in [
        ("local-right", 256, 144, 8, false, false, Some([8, 0])),
        ("local-left", 256, 144, -8, false, false, Some([-8, 0])),
        ("smaller-region", 128, 72, 4, false, false, Some([8, 0])),
        ("plain-region", 128, 72, 4, true, false, None),
        ("beyond-search-radius", 256, 144, 48, false, false, None),
        ("outside-image", 256, 144, 8, false, true, None),
    ] {
        let region = Region {
            x: 384,
            y: 216,
            width,
            height,
        };
        let local = if plain {
            GrayImage::from_pixel(width, height, Luma([80]))
        } else {
            texture(width, height, 9)
        };
        let mut frames = vec![];
        let mut images = vec![];
        for (i, delta) in [0, shift].into_iter().enumerate() {
            let mut scene = background.clone();
            for y in 0..height {
                for x in 0..width {
                    let sx = x as i32 - delta;
                    let value = if plain {
                        Luma([80])
                    } else if (0..width as i32).contains(&sx) {
                        *local.get_pixel(sx as u32, y)
                    } else {
                        Luma([0])
                    };
                    scene.put_pixel(region.x + x, region.y + y, value);
                }
            }
            let file = directory.join(format!("{name}-{i}.png"));
            scene.save(&file)?;
            let bytes = visual::image_bytes(&file)?;
            frames.push(json!({"file":file,"reference":{"id":format!("{name}-{i}"),"number":i+1,"sha256":visual::digest(&bytes),
                "width":1024,"height":576,"epoch":0,"started_unix_ms":(i+1)*1000}}));
            images.push(bytes);
        }
        cases.push(json!({"name":name,"frames":frames,"region":if outside {Region{x:1000,..region}} else {region},
            "expected_differences":[response::difference(&images[0],&images[1])?]}));
        truth.push(json!({"name":name,"source_pixel_shift":[shift,0],"region_normalized_consensus":expected,"measurement_error":outside,
            "plain_texture":plain,"actual_source_region":region,"background_unchanged":true}));
    }
    serde_json::to_writer_pretty(fs::File::create(directory.join("cases.json"))?, &cases)?;
    serde_json::to_writer_pretty(fs::File::create(directory.join("truth.json"))?, &truth)?;
    println!(
        "{}",
        json!({"synthetic_cases":cases.len(),"gameplay_evidence":false})
    );
    Ok(())
}
