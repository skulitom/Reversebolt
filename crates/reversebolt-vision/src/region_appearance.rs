//! Two-image template correspondence. No persistent object or gameplay identity.
use crate::response;
use image::{
    GrayImage,
    imageops::{self, FilterType},
};
use reversebolt_core::{Result, visual_regions::Region};
use serde::Serialize;

pub const POLICY: &str = "rgb601-triangle512x288-region-ncc16-dense-reverse-v1";
pub const LIMITATION: &str = "A same-size image-template correspondence across two captures, not object identity, physical speed or gameplay meaning. Searches all fitting positions in normalized 512x288 images using at most 16x16 samples, verifies the winning rectangle densely and requires an unambiguous reverse match. No rotation, scaling, deformation or occlusion model is fitted. Matching background or visually identical objects can still mislead. Scores and thresholds are heuristics, not probabilities. No unresolved result or zero displacement proves no movement. No match is propagated to another capture.";
const WIDTH: u32 = 512;
const HEIGHT: u32 = 288;
const MIN_STD: f64 = 8.;
const MIN_NCC: f64 = 0.92;
const MIN_MARGIN: f64 = 0.06;
const BASIN: u32 = 2;
const MIN_SIDE: u32 = 8;
const MAX_AREA: u32 = 16_384;
const REVERSE_TOLERANCE: u32 = 1;

#[derive(Debug, Serialize)]
pub struct Requirements {
    pub minimum_std: f64,
    pub minimum_ncc: f64,
    pub minimum_margin: f64,
    pub excluded_basin_radius: u32,
    pub reverse_tolerance_pixels: u32,
    pub minimum_template_side: u32,
    pub maximum_template_area: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Matched,
    UnsupportedSize,
    LowTexture,
    PoorMatch,
    Ambiguous,
    ImageBoundary,
    InconsistentReverse,
}

#[derive(Debug, Serialize)]
pub struct Search {
    pub status: Status,
    pub positions: usize,
    pub textured_positions: usize,
    pub samples: usize,
    pub source_std: f64,
    /// Diagnostic candidate only; not a match when status is unresolved.
    pub best_at: Option<[u32; 2]>,
    pub best_ncc: Option<f64>,
    pub second_basin_ncc: Option<f64>,
    pub dense_ncc: Option<f64>,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub policy: &'static str,
    pub source_dimensions: [u32; 2],
    pub measurement_dimensions: [u32; 2],
    pub source_region: Region,
    pub measurement_region: Region,
    pub source_pixel_scale: [f64; 2],
    pub status: Status,
    pub requirements: Requirements,
    pub forward: Option<Search>,
    pub reverse: Option<Search>,
    /// [x,y,width,height] in original image pixels, approximate after normalization.
    pub matched_region: Option<[f64; 4]>,
    pub displacement: Option<[f64; 2]>,
    pub limitation: &'static str,
}

struct Template {
    offsets: Vec<usize>,
    pixels: Vec<i64>,
    sum: i64,
    variance: i64,
}
impl Template {
    fn new(image: &GrayImage, region: Region, sparse: bool) -> Self {
        let nx = if sparse {
            region.width.min(16)
        } else {
            region.width
        };
        let ny = if sparse {
            region.height.min(16)
        } else {
            region.height
        };
        let mut offsets = vec![];
        let mut pixels = vec![];
        for iy in 0..ny {
            for ix in 0..nx {
                let x = ix * (region.width - 1) / (nx - 1);
                let y = iy * (region.height - 1) / (ny - 1);
                offsets.push((y * image.width() + x) as usize);
                pixels.push(i64::from(image.get_pixel(region.x + x, region.y + y)[0]));
            }
        }
        let sum = pixels.iter().sum::<i64>();
        let variance = pixels.len() as i64 * pixels.iter().map(|v| v * v).sum::<i64>() - sum * sum;
        Self {
            offsets,
            pixels,
            sum,
            variance,
        }
    }
    fn std(&self) -> f64 {
        (self.variance as f64).sqrt() / self.pixels.len() as f64
    }
    fn correlate(&self, image: &GrayImage, at: [u32; 2]) -> Option<f64> {
        let base = (at[1] * image.width() + at[0]) as usize;
        let raw = image.as_raw();
        let (mut sum, mut squares, mut product) = (0i64, 0i64, 0i64);
        for (&offset, &source) in self.offsets.iter().zip(&self.pixels) {
            let value = i64::from(raw[base + offset]);
            sum += value;
            squares += value * value;
            product += value * source;
        }
        let n = self.pixels.len() as i64;
        let variance = n * squares - sum * sum;
        if self.variance <= 0 || (variance as f64) < MIN_STD * MIN_STD * (n * n) as f64 {
            return None;
        }
        Some(
            ((n * product - self.sum * sum) as f64
                / ((self.variance as f64) * (variance as f64)).sqrt())
            .clamp(-1., 1.),
        )
    }
}

fn search(source: &GrayImage, target: &GrayImage, region: Region) -> Search {
    let template = Template::new(source, region, true);
    let mut report = Search {
        status: Status::LowTexture,
        positions: 0,
        textured_positions: 0,
        samples: template.pixels.len(),
        source_std: template.std(),
        best_at: None,
        best_ncc: None,
        second_basin_ncc: None,
        dense_ncc: None,
    };
    if report.source_std < MIN_STD {
        return report;
    }
    let mut scores = vec![];
    for y in 0..=target.height() - region.height {
        for x in 0..=target.width() - region.width {
            report.positions += 1;
            if let Some(ncc) = template.correlate(target, [x, y]) {
                scores.push(([x, y], ncc));
            }
        }
    }
    report.textured_positions = scores.len();
    // Deterministic ties retain the first row-major position, never a proximity prior.
    let Some(&(at, score)) = scores.iter().max_by(|a, b| {
        a.1.total_cmp(&b.1)
            .then(b.0[1].cmp(&a.0[1]))
            .then(b.0[0].cmp(&a.0[0]))
    }) else {
        return report;
    };
    report.best_at = Some(at);
    report.best_ncc = Some(score);
    report.second_basin_ncc = scores
        .iter()
        .filter(|(p, _)| p[0].abs_diff(at[0]) > BASIN || p[1].abs_diff(at[1]) > BASIN)
        .map(|(_, v)| *v)
        .max_by(f64::total_cmp);
    report.status = if score < MIN_NCC {
        Status::PoorMatch
    } else if report
        .second_basin_ncc
        .is_none_or(|second| score - second < MIN_MARGIN)
    {
        Status::Ambiguous
    } else if at[0] == 0
        || at[1] == 0
        || at[0] + region.width == target.width()
        || at[1] + region.height == target.height()
    {
        Status::ImageBoundary
    } else {
        Status::Matched
    };
    if report.status != Status::Matched {
        return report;
    }
    // Sparse search cannot certify a rectangle: validate every normalized pixel
    // of the winner, so changes between sampled points cannot be hidden.
    let dense = Template::new(source, region, false);
    if dense.std() < MIN_STD {
        report.status = Status::LowTexture;
        return report;
    }
    report.dense_ncc = dense.correlate(target, at);
    if report.dense_ncc.is_none_or(|score| score < MIN_NCC) {
        report.status = Status::PoorMatch;
    }
    report
}

fn prepared(a: GrayImage, b: GrayImage, region: Region, dimensions: [u32; 2]) -> Report {
    let scale = [
        f64::from(dimensions[0]) / f64::from(WIDTH),
        f64::from(dimensions[1]) / f64::from(HEIGHT),
    ];
    let left = u64::from(region.x) * u64::from(WIDTH) / u64::from(dimensions[0]);
    let top = u64::from(region.y) * u64::from(HEIGHT) / u64::from(dimensions[1]);
    let right =
        (u64::from(region.x + region.width) * u64::from(WIDTH)).div_ceil(u64::from(dimensions[0]));
    let bottom = (u64::from(region.y + region.height) * u64::from(HEIGHT))
        .div_ceil(u64::from(dimensions[1]));
    let normalized = Region {
        x: left as u32,
        y: top as u32,
        width: (right - left) as u32,
        height: (bottom - top) as u32,
    };
    let mut report = Report {
        policy: POLICY,
        source_dimensions: dimensions,
        measurement_dimensions: [WIDTH, HEIGHT],
        source_region: region,
        measurement_region: normalized,
        source_pixel_scale: scale,
        status: Status::UnsupportedSize,
        requirements: Requirements {
            minimum_std: MIN_STD,
            minimum_ncc: MIN_NCC,
            minimum_margin: MIN_MARGIN,
            excluded_basin_radius: BASIN,
            reverse_tolerance_pixels: REVERSE_TOLERANCE,
            minimum_template_side: MIN_SIDE,
            maximum_template_area: MAX_AREA,
        },
        forward: None,
        reverse: None,
        matched_region: None,
        displacement: None,
        limitation: LIMITATION,
    };
    if normalized.width < MIN_SIDE
        || normalized.height < MIN_SIDE
        || normalized.width * normalized.height > MAX_AREA
    {
        return report;
    }
    let forward = search(&a, &b, normalized);
    report.status = forward.status;
    if forward.status == Status::Matched {
        let at = forward.best_at.unwrap();
        let reverse = search(
            &b,
            &a,
            Region {
                x: at[0],
                y: at[1],
                ..normalized
            },
        );
        if reverse.status != Status::Matched
            || reverse.best_at.is_none_or(|back| {
                back[0].abs_diff(normalized.x) > REVERSE_TOLERANCE
                    || back[1].abs_diff(normalized.y) > REVERSE_TOLERANCE
            })
        {
            report.status = Status::InconsistentReverse;
        } else {
            report.matched_region = Some([
                f64::from(at[0]) * scale[0],
                f64::from(at[1]) * scale[1],
                f64::from(normalized.width) * scale[0],
                f64::from(normalized.height) * scale[1],
            ]);
            report.displacement = Some([
                (f64::from(at[0]) - f64::from(normalized.x)) * scale[0],
                (f64::from(at[1]) - f64::from(normalized.y)) * scale[1],
            ]);
        }
        report.reverse = Some(reverse);
    }
    report.forward = Some(forward);
    report
}

pub fn measure(before: &[u8], after: &[u8], region: Region) -> Result<Report> {
    region.validate_shape()?;
    let a = response::grayscale(before)?;
    let b = response::grayscale(after)?;
    if a.dimensions() != b.dimensions() {
        return Err("Image geometry changed.".into());
    }
    let dimensions = [a.width(), a.height()];
    region.validate(dimensions)?;
    Ok(prepared(
        imageops::resize(&a, WIDTH, HEIGHT, FilterType::Triangle),
        imageops::resize(&b, WIDTH, HEIGHT, FilterType::Triangle),
        region,
        dimensions,
    ))
}

#[cfg(test)]
#[path = "region_appearance_tests.rs"]
mod tests;
