//! Cheap source/quiet quality checks before the bounded appearance-search budget.
use crate::{patch_motion, region_appearance as matcher};
use image::GrayImage;
use reversebolt_core::visual_regions::Region;
use serde::Serialize;

const MAX_QUIET_DIFFERENCE: f64 = 4.;

#[derive(Clone, Copy)]
pub enum Ranking {
    ChangeOnly,
    StableTexture,
}

#[derive(Debug, Serialize)]
pub struct Quality {
    pub measurement_region: Region,
    pub source_std: [f64; 2],
    pub corner_score: f64,
    pub quiet_mean_absolute_difference: f64,
    pub rank_score: f64,
}

#[derive(Debug, Serialize)]
pub struct Selection {
    pub input_regions: usize,
    pub eligible_regions: usize,
    pub rejected_low_texture: usize,
    pub rejected_weak_corner: usize,
    pub rejected_quiet_change: usize,
    pub minimum_source_std: f64,
    pub minimum_corner_score: f64,
    pub maximum_quiet_mean_absolute_difference: f64,
    pub corner_patch_side: u32,
}

pub(crate) struct Candidate {
    pub change: f64,
    pub region: Region,
    pub normalized: Region,
    pub quality: Option<Quality>,
}

impl From<(f64, Region, Region)> for Candidate {
    fn from((change, region, normalized): (f64, Region, Region)) -> Self {
        Self {
            change,
            region,
            normalized,
            quality: None,
        }
    }
}

/// Only the quiet/root images are inspected. Later match outcomes cannot rerank.
pub(crate) fn select(
    quiet: &GrayImage,
    root: &GrayImage,
    dimensions: [u32; 2],
    candidates: Vec<(f64, Region, Region)>,
) -> (Vec<Candidate>, Selection) {
    let mut selection = Selection {
        input_regions: candidates.len(),
        eligible_regions: 0,
        rejected_low_texture: 0,
        rejected_weak_corner: 0,
        rejected_quiet_change: 0,
        minimum_source_std: matcher::MIN_STD,
        minimum_corner_score: patch_motion::MIN_CORNER,
        maximum_quiet_mean_absolute_difference: MAX_QUIET_DIFFERENCE,
        corner_patch_side: 9,
    };
    let mut eligible = vec![];
    for (change, region, normalized) in candidates {
        // Use the rectangle actually checked by the matcher, including outward
        // rounding through original pixels, rather than assuming the grid tile.
        let r = matcher::normalized_region(region, dimensions);
        let source_std = matcher::template_std(root, r);
        if source_std.iter().any(|&s| s < matcher::MIN_STD) {
            selection.rejected_low_texture += 1;
            continue;
        }
        let center = [(r.x + r.width / 2) as i32, (r.y + r.height / 2) as i32];
        let corner_score = patch_motion::corner_score(root, center);
        if corner_score < patch_motion::MIN_CORNER {
            selection.rejected_weak_corner += 1;
            continue;
        }
        let mut quiet_sum = 0u32;
        for y in r.y..r.y + r.height {
            for x in r.x..r.x + r.width {
                quiet_sum += u32::from(root[(x, y)][0].abs_diff(quiet[(x, y)][0]));
            }
        }
        let quiet_mean_absolute_difference = f64::from(quiet_sum) / f64::from(r.width * r.height);
        if quiet_mean_absolute_difference > MAX_QUIET_DIFFERENCE {
            selection.rejected_quiet_change += 1;
            continue;
        }
        let rank_score = corner_score / (1. + quiet_mean_absolute_difference);
        eligible.push(Candidate {
            change,
            region,
            normalized,
            quality: Some(Quality {
                measurement_region: r,
                source_std,
                corner_score,
                quiet_mean_absolute_difference,
                rank_score,
            }),
        });
    }
    eligible.sort_by(|a, b| {
        b.quality
            .as_ref()
            .unwrap()
            .rank_score
            .total_cmp(&a.quality.as_ref().unwrap().rank_score)
            .then(b.change.total_cmp(&a.change))
            .then(a.normalized.width.cmp(&b.normalized.width))
            .then(a.normalized.y.cmp(&b.normalized.y))
            .then(a.normalized.x.cmp(&b.normalized.x))
    });
    selection.eligible_regions = eligible.len();
    (eligible, selection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    #[test]
    fn source_quality_rejects_flat_single_direction_and_quiet_flicker() {
        let r = Region {
            x: 64,
            y: 64,
            width: 32,
            height: 32,
        };
        let raw = vec![(20., r, r)];
        let flat = GrayImage::from_pixel(512, 288, Luma([40]));
        let edge = GrayImage::from_fn(512, 288, |x, _| Luma([if x < 80 { 20 } else { 220 }]));
        let textured = GrayImage::from_fn(512, 288, |x, y| {
            let mut n = (x + y * 512 + 17)
                .wrapping_mul(747796405)
                .wrapping_add(2891336453);
            n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
            Luma([40 + (((n >> 22) ^ n) % 140) as u8])
        });
        let (_, summary) = select(&flat, &flat, [512, 288], raw.clone());
        assert_eq!(summary.rejected_low_texture, 1);
        let (_, summary) = select(&edge, &edge, [512, 288], raw.clone());
        assert_eq!(summary.rejected_weak_corner, 1);
        let flicker = GrayImage::from_fn(512, 288, |x, y| Luma([textured[(x, y)][0] + 12]));
        let (selected, summary) = select(&flicker, &textured, [512, 288], raw.clone());
        assert!(selected.is_empty());
        assert_eq!(summary.rejected_quiet_change, 1);
        let (selected, summary) = select(&textured, &textured, [512, 288], raw);
        assert_eq!(summary.eligible_regions, 1);
        assert_eq!(
            selected[0]
                .quality
                .as_ref()
                .unwrap()
                .quiet_mean_absolute_difference,
            0.
        );
    }

    #[test]
    fn quality_cites_the_exact_rounded_matcher_rectangle() {
        let image = GrayImage::from_fn(512, 288, |x, y| {
            Luma([((x * 7 + y * 37 + x * y) % 256) as u8])
        });
        let region = Region {
            x: 401,
            y: 157,
            width: 111,
            height: 85,
        };
        let normalized = matcher::normalized_region(region, [1100, 600]);
        let (selected, _) = select(&image, &image, [1100, 600], vec![(12., region, normalized)]);
        let quality = selected[0].quality.as_ref().unwrap();
        let actual = matcher::prepared(&image, &image, region, [1100, 600]);
        assert_eq!(
            json_region(quality.measurement_region),
            json_region(actual.measurement_region)
        );
        assert_eq!(quality.source_std[0], actual.forward.unwrap().source_std);
    }
    fn json_region(r: Region) -> serde_json::Value {
        serde_json::to_value(r).unwrap()
    }
}
