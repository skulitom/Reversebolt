//! Deterministic rendered-response measurement. No semantic inference.
use image::{
    GrayImage, Luma,
    imageops::{self, FilterType},
};
use reversebolt_core::{Result, visual::MAX_IMAGE_BYTES};

pub use reversebolt_core::runtime_tests::METRIC_ID;

pub(crate) fn grayscale(bytes: &[u8]) -> Result<GrayImage> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err("Response image exceeds the 10 MiB budget.".into());
    }
    let reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let (width, height) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 4_000_000 {
        return Err("Response image exceeds the four-million-pixel budget.".into());
    }
    let rgb = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?
        .to_rgb8();
    let gray = GrayImage::from_fn(width, height, |x, y| {
        let p = rgb.get_pixel(x, y);
        Luma([
            ((299 * u32::from(p[0]) + 587 * u32::from(p[1]) + 114 * u32::from(p[2]) + 500) / 1000)
                as u8,
        ])
    });
    Ok(gray)
}

fn prepare(bytes: &[u8]) -> Result<GrayImage> {
    let gray = grayscale(bytes)?;
    let scaled = imageops::resize(&gray, 640, 360, FilterType::Triangle);
    Ok(imageops::blur(
        &imageops::crop_imm(&scaled, 24, 40, 592, 296).to_image(),
        1.,
    ))
}

/// Versioned grayscale mean absolute difference. Geometry compatibility and
/// capture/input chronology must be checked by the experiment owner separately.
pub fn difference(before: &[u8], after: &[u8]) -> Result<f64> {
    let a = prepare(before)?;
    let b = prepare(after)?;
    let sum: u64 = a
        .pixels()
        .zip(b.pixels())
        .map(|(a, b)| u64::from(a[0].abs_diff(b[0])))
        .sum();
    Ok(sum as f64 / f64::from(a.width() * a.height()))
}

/// Reproduce the whole-screen metric and sampled-input evidence before reusing a
/// completed comparison. No target access, input replay or semantic promotion.
pub fn verify_recorded_response(
    test: &reversebolt_core::runtime_tests::Test,
    trial: &reversebolt_core::runtime_trials::Trial,
    mut read_image: impl FnMut(usize, usize) -> Result<Vec<u8>>,
    mut read_input: impl FnMut(usize, usize) -> Result<Option<reversebolt_core::inputs::Window>>,
) -> Result<()> {
    use reversebolt_core::visual;
    test.check_recorded_response(trial)?;
    for (c, condition) in test.conditions.iter().enumerate() {
        let input = read_input(c, 0)?;
        if visual::digest(&serde_json::to_vec(&input).map_err(|e| e.to_string())?)
            != condition.frames[0].input_sha256
        {
            return Err("Initial sampled-input checksum changed.".into());
        }
    }
    let mut failure = None;
    let mut pairs = 0;
    let checked_image = |c: usize, i: usize| {
        let bytes = read_image(c, i)?;
        let dimensions =
            image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Png)
                .into_dimensions()
                .map_err(|e| e.to_string())?;
        let frame = &test.conditions[c].frames[i];
        if dimensions != (frame.width, frame.height) {
            return Err("Retained image dimensions differ from the capture record.".into());
        }
        Ok(bytes)
    };
    crate::trial_evidence::visit(test, checked_image, read_input, |pair| {
        pairs += 1;
        let result = (|| -> Result<()> {
            let (before, after) = pair.images?;
            if !pair.input?.matches || pair.after.difference != Some(difference(before, after)?) {
                return Err("Retained response differs from its images or sampled inputs.".into());
            }
            Ok(())
        })();
        if let Err(error) = result {
            failure.get_or_insert(error);
        }
    })?;
    if let Some(error) = failure {
        return Err(error);
    }
    if pairs != 15 {
        return Err("A remembered response needs all three complete conditions.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png(value: u8) -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(vec![]);
        GrayImage::from_pixel(80, 45, Luma([value]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }
    #[test]
    fn analytic_images_have_exact_symmetric_differences() {
        let black = png(0);
        let white = png(255);
        let gray = png(40);
        assert_eq!(difference(&black, &black).unwrap(), 0.);
        assert_eq!(difference(&black, &white).unwrap(), 255.);
        assert_eq!(difference(&gray, &black).unwrap(), 40.);
        assert_eq!(difference(&black, &gray).unwrap(), 40.);
        assert!(difference(b"invalid png", &black).is_err());
    }
}
