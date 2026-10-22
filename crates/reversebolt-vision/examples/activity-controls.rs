//! Generate explicitly synthetic consistency controls from one retained image.
use image::{GrayImage, Luma, imageops::FilterType};
use reversebolt_core::{
    play_context::{Frame, Pair, Source},
    visual::{self, CaptureInterval},
};
use std::{fs, path::Path};
fn png(image: &GrayImage) -> Result<Vec<u8>, image::ImageError> {
    let mut bytes = std::io::Cursor::new(vec![]);
    image.write_to(&mut bytes, image::ImageFormat::Png)?;
    Ok(bytes.into_inner())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: activity-controls IMAGE EXPECTED_SHA NEW_DIRECTORY".into());
    }
    let bytes = visual::image_bytes(Path::new(&args[0]))?;
    if visual::digest(&bytes) != args[1] {
        return Err("Input image changed".into());
    }
    let (width, height) =
        image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Png)
            .into_dimensions()?;
    if u64::from(width) * u64::from(height) > 4_000_000 {
        return Err("Image exceeds budget".into());
    }
    let rgb = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgb8();
    let gray = GrayImage::from_fn(width, height, |x, y| {
        let p = rgb.get_pixel(x, y);
        Luma([
            ((299 * u32::from(p[0]) + 587 * u32::from(p[1]) + 114 * u32::from(p[2]) + 500) / 1000)
                as u8,
        ])
    });
    let before = image::imageops::resize(&gray, 256, 144, FilterType::Triangle);
    let shift = |dx: i32| {
        GrayImage::from_fn(256, 144, |x, y| {
            let sx = x as i32 - dx;
            if (0..256).contains(&sx) {
                *before.get_pixel(sx as u32, y)
            } else {
                Luma([0])
            }
        })
    };
    let images = [png(&before)?, png(&shift(18))?, png(&shift(-18))?];
    let dir = Path::new(&args[2]);
    fs::create_dir(dir)?;
    for (i, bytes) in images.iter().enumerate() {
        fs::write(dir.join(format!("{i}.png")), bytes)?;
    }
    for (name, last, expected) in [("consistent", 1, true), ("opposed", 2, false)] {
        // Names are output bookkeeping only; neither expected answer reaches the prompt.
        let clock = "synthetic-image-sequence".to_owned();
        let indices = [0, 1, 0, last];
        let numbers = [1, 2, 4, 5];
        let refs: Vec<_> = indices
            .iter()
            .zip(numbers)
            .map(|(i, n)| Frame {
                id: format!("{clock}-{n}"),
                sha256: visual::digest(&images[*i]),
                number: n,
                width: 256,
                height: 144,
                interval: CaptureInterval {
                    clock_id: clock.clone(),
                    begin_us: n as u64 * 1000000,
                    end_us: n as u64 * 1000000 + 1000,
                },
            })
            .collect();
        let source = Source {
            investigation_id: clock.clone(),
            batch: 1,
            frame_metadata_sha256: visual::digest(&serde_json::to_vec(&refs)?),
            key: 68,
            quiet_difference: 0.,
            pairs: [0, 2].map(|i| Pair {
                episode: i,
                before: refs[i].clone(),
                after: refs[i + 1].clone(),
                input_sha256: visual::digest(b"synthetic metadata; no input sampled or sent"),
                difference: reversebolt_vision::response::difference(
                    &images[indices[i]],
                    &images[indices[i + 1]],
                )
                .unwrap(),
            }),
        };
        source.validate()?;
        let manifest = serde_json::json!({"source":source,"frames":indices.map(|i|dir.join(format!("{i}.png")))});
        fs::write(
            dir.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        println!(
            "{}",
            serde_json::json!({"case":name,"expected_consistency":expected,"synthetic":true,"input_sent":false})
        );
    }
    Ok(())
}
