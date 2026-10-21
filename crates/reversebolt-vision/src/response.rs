//! Deterministic rendered-response measurement. No semantic inference.
use image::{
    GrayImage, Luma,
    imageops::{self, FilterType},
};
use reversebolt_core::{Result, visual::MAX_IMAGE_BYTES};

pub use reversebolt_core::runtime_tests::METRIC_ID;

fn prepare(bytes: &[u8]) -> Result<GrayImage> {
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
