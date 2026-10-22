//! Fixed-window sampling. Pixel content is measured for diagnostics only and
//! never influences which completed frame is selected.
use image::RgbaImage;
use reversebolt_core::visual::{CaptureSampling, SampledFrame};

pub const SETTLE_US: u64 = 80_000;
pub const MAX_FRAMES: usize = 8;
pub const LOW_DETAIL: f64 = 0.001;

#[derive(Default)]
pub struct Burst<T> {
    pub latest: Option<T>,
    pub frames: Vec<SampledFrame>,
}
impl<T> Burst<T> {
    pub fn deadline_us(&self) -> Option<u64> {
        self.frames
            .first()
            .map(|f| f.completed_us.saturating_add(SETTLE_US))
    }
    pub fn finished(&self, now_us: u64) -> bool {
        self.frames.len() >= MAX_FRAMES || self.deadline_us().is_some_and(|d| now_us >= d)
    }
    pub fn push(&mut self, frame: SampledFrame, image: T) -> bool {
        if self.finished(frame.completed_us) {
            return false;
        }
        self.latest = Some(image);
        self.frames.push(frame);
        true
    }
    pub fn receipt(&self) -> Option<CaptureSampling> {
        Some(CaptureSampling {
            clock: None,
            clock_error: None,
            policy: "last_completed_in_fixed_window_v1".into(),
            settle_ms: SETTLE_US / 1000,
            max_frames: MAX_FRAMES,
            selected: self.frames.len().checked_sub(1)?,
            frames: self.frames.clone(),
        })
    }
}

/// Fraction of pixels clearly different from the dominant quantized color.
/// A tiny outlier cannot make an otherwise uniform capture look informative.
/// This is not semantic quality, OCR confidence, or a reason to reject a scene.
pub fn detail_fraction(image: &RgbaImage) -> f64 {
    if image.is_empty() {
        return 0.;
    }
    let mut bins = [0usize; 4096];
    for p in image.pixels() {
        bins[((p[0] as usize >> 4) << 8) | ((p[1] as usize >> 4) << 4) | (p[2] as usize >> 4)] += 1;
    }
    let dominant = bins.iter().enumerate().max_by_key(|(_, n)| *n).unwrap().0;
    let color = [
        ((dominant >> 8) * 16 + 8) as u8,
        (((dominant >> 4) & 15) * 16 + 8) as u8,
        ((dominant & 15) * 16 + 8) as u8,
    ];
    let different = image
        .pixels()
        .filter(|p| (0..3).any(|i| p[i].abs_diff(color[i]) > 16))
        .count();
    different as f64 / (image.width() as f64 * image.height() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(time: u64, detail: f64) -> SampledFrame {
        SampledFrame {
            completed_us: time,
            compositor_100ns: None,
            width: 10,
            height: 10,
            rgba_sha256: String::new(),
            detail_fraction: detail,
        }
    }
    #[test]
    fn selection_keeps_real_blanks_and_never_chooses_by_content() {
        for (first, last) in [(0., 0.2), (0.2, 0.), (0., 0.)] {
            let mut burst = Burst::default();
            assert!(burst.push(frame(1000, first), "first"));
            assert!(burst.push(frame(20000, last), "last"));
            assert!(!burst.push(frame(81000, 0.9), "late"));
            assert_eq!(burst.latest, Some("last"));
            let receipt = burst.receipt().unwrap();
            assert_eq!(receipt.selected, 1);
            assert_eq!(receipt.frames[receipt.selected].detail_fraction, last);
        }
    }
    #[test]
    fn static_windows_and_frame_limit_remain_bounded() {
        let mut burst = Burst::default();
        assert!(burst.receipt().is_none());
        burst.push(frame(0, 0.), 0);
        assert!(!burst.finished(79999));
        assert!(burst.finished(80000));
        assert_eq!(burst.latest, Some(0));
        for i in 1..MAX_FRAMES {
            assert!(burst.push(frame(i as u64, 0.1), i));
        }
        assert!(!burst.push(frame(10, 1.), 100));
        assert_eq!(burst.latest, Some(MAX_FRAMES - 1));
        assert_eq!(burst.frames.len(), MAX_FRAMES);
    }
    #[test]
    fn one_bright_pixel_does_not_hide_a_nearly_uniform_image() {
        let mut image = RgbaImage::from_pixel(600, 220, image::Rgba([24, 26, 30, 255]));
        assert_eq!(detail_fraction(&image), 0.);
        image.put_pixel(10, 10, image::Rgba([255, 255, 255, 255]));
        assert!(detail_fraction(&image) < LOW_DETAIL);
        for y in 20..40 {
            for x in 20..80 {
                image.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
        assert!(detail_fraction(&image) > LOW_DETAIL);
    }
}
