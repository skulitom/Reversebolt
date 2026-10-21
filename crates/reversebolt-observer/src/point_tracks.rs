//! Research-only aggregation of image point correspondences into provisional boxes.
//! This does not establish object identity, world motion, semantics or edit authority.
use crate::{FrameEvidence, tracking::Seed};
use reversebolt_core::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    /// Source image pixels, not normalized coordinates.
    pub xy: [f64; 2],
    pub visibility: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: u64,
    /// Fixed row-major inner 3-by-3 seed grid; point identities must not be reordered.
    pub points: Vec<Point>,
}

#[derive(Debug, Serialize)]
pub struct Sample {
    pub id: u64,
    pub bounds: Option<[f64; 4]>,
    pub visible_points: usize,
    pub inliers: usize,
    pub scale: Option<f64>,
    pub residual_px: Option<f64>,
    pub reason: Option<&'static str>,
}

pub struct Tracker {
    frame: FrameEvidence,
    seeds: Vec<Seed>,
    lost: Vec<bool>,
    gated: bool,
}

pub fn parameters() -> serde_json::Value {
    serde_json::json!({"grid":[0.25,0.5,0.75],"visibility_min_exclusive":0.5,
        "minimum_points":5,"minimum_seed_side_pixels":1.0,"scale_from_seed":[0.5,2.0],
        "residual_radius":"max(2 pixels, 0.12 * shorter seed side * scale)",
        "maximum_gap_ns":2_000_000_000u64,"reacquisition":false,
        "fit":"median pairwise distance scale, median axis translations; no rotation"})
}

fn valid_frame(frame: &FrameEvidence) -> bool {
    (1..=16_384).contains(&frame.width)
        && (1..=16_384).contains(&frame.height)
        && frame.sha256.len() == 64
        && frame.sha256.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn seed_points(seed: &Seed, frame: &FrameEvidence) -> Vec<[f64; 2]> {
    let [l, t, r, b] = seed.bounds;
    [0.25, 0.5, 0.75]
        .into_iter()
        .flat_map(|y| {
            [0.25, 0.5, 0.75].map(|x| {
                [
                    (l + x * (r - l)) * f64::from(frame.width) / 1000.,
                    (t + y * (b - t)) * f64::from(frame.height) / 1000.,
                ]
            })
        })
        .collect()
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.
    } else {
        values[middle]
    }
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

impl Tracker {
    pub fn new(frame: &FrameEvidence, seeds: &[Seed], gated: bool) -> Result<Self> {
        if !valid_frame(frame) || seeds.len() > 6 {
            return Err("Invalid frame or more than six seeds.".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for seed in seeds {
            let [l, t, r, b] = seed.bounds;
            if !ids.insert(seed.id)
                || seed
                    .bounds
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1000.).contains(v))
                || l >= r
                || t >= b
                || (r - l) * f64::from(frame.width) / 1000. < 1.
                || (b - t) * f64::from(frame.height) / 1000. < 1.
            {
                return Err("Seeds require unique IDs and normalized boxes at least one source pixel per side.".into());
            }
        }
        Ok(Self {
            frame: frame.clone(),
            seeds: seeds.to_vec(),
            lost: vec![false; seeds.len()],
            gated,
        })
    }

    pub fn update(&mut self, frame: &FrameEvidence, groups: &[Group]) -> Result<Vec<Sample>> {
        // Validate the complete operation before changing any tracker state.
        if !valid_frame(frame)
            || frame.width != self.frame.width
            || frame.height != self.frame.height
            || frame.timestamp_ns <= self.frame.timestamp_ns
            || frame.timestamp_ns - self.frame.timestamp_ns > 2_000_000_000
            || groups.len() != self.seeds.len()
        {
            return Err("Invalid frame dimensions, chronology, gap or point groups.".into());
        }
        for (group, seed) in groups.iter().zip(&self.seeds) {
            if group.id != seed.id
                || group.points.len() != 9
                || group.points.iter().any(|p| {
                    !p.visibility.is_finite()
                        || !(0.0..=1.0).contains(&p.visibility)
                        || p.xy.iter().any(|v| !v.is_finite() || v.abs() > 1_000_000.)
                })
            {
                return Err(
                    "Point groups must preserve nine finite ordered correspondences per seed."
                        .into(),
                );
            }
        }
        let samples = self
            .seeds
            .iter()
            .zip(groups)
            .zip(&mut self.lost)
            .map(|((seed, group), lost)| {
                let sample = aggregate(seed, frame, group, self.gated, *lost);
                *lost |= sample.bounds.is_none();
                sample
            })
            .collect();
        self.frame = frame.clone();
        Ok(samples)
    }
}

fn aggregate(seed: &Seed, frame: &FrameEvidence, group: &Group, gated: bool, lost: bool) -> Sample {
    let mut sample = Sample {
        id: seed.id,
        bounds: None,
        visible_points: 0,
        inliers: 0,
        scale: None,
        residual_px: None,
        reason: None,
    };
    if lost {
        sample.reason = Some("track_lost");
        return sample;
    }
    let origin = seed_points(seed, frame);
    let visible = |p: &Point| {
        p.visibility > 0.5
            && p.xy[0] >= 0.
            && p.xy[1] >= 0.
            && p.xy[0] < f64::from(frame.width)
            && p.xy[1] < f64::from(frame.height)
    };
    sample.visible_points = group.points.iter().filter(|p| visible(p)).count();
    let pairs: Vec<_> = origin
        .into_iter()
        .zip(&group.points)
        .filter(|(_, p)| !gated || visible(p))
        .map(|(a, p)| (a, p.xy))
        .collect();
    if pairs.len() < 5 {
        sample.reason = Some("insufficient_points");
        return sample;
    }
    let ratios: Vec<_> = pairs
        .iter()
        .enumerate()
        .flat_map(|(i, (a, b))| {
            pairs[i + 1..]
                .iter()
                .map(move |(aa, bb)| distance(*b, *bb) / distance(*a, *aa))
        })
        .collect();
    let scale = median(ratios);
    let shift = [0, 1].map(|axis| {
        median(
            pairs
                .iter()
                .map(|(a, b)| b[axis] - a[axis] * scale)
                .collect(),
        )
    });
    sample.scale = Some(scale);
    let [l, t, r, b] = seed.bounds;
    let width = (r - l) * f64::from(frame.width) / 1000.;
    let height = (b - t) * f64::from(frame.height) / 1000.;
    let residuals: Vec<_> = pairs
        .iter()
        .map(|(a, b)| distance([a[0] * scale + shift[0], a[1] * scale + shift[1]], *b))
        .collect();
    let radius = (0.12 * width.min(height) * scale).max(2.);
    sample.inliers = residuals.iter().filter(|r| **r <= radius).count();
    sample.residual_px = Some(median(residuals));
    if scale <= 0. || (gated && !(0.5..=2.).contains(&scale)) {
        sample.reason = Some("scale");
    } else if gated && sample.inliers < 5 {
        sample.reason = Some("incoherent_points");
    } else {
        let dx = shift[0] * 1000. / f64::from(frame.width);
        let dy = shift[1] * 1000. / f64::from(frame.height);
        let bounds = [
            l * scale + dx,
            t * scale + dy,
            r * scale + dx,
            b * scale + dy,
        ]
        .map(|v| v.clamp(0., 1000.));
        if bounds[0] >= bounds[2] || bounds[1] >= bounds[3] {
            sample.reason = Some("outside_frame");
        } else {
            sample.bounds = Some(bounds);
        }
    }
    sample
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (FrameEvidence, Vec<Seed>) {
        (
            FrameEvidence {
                sha256: "a".repeat(64),
                width: 800,
                height: 400,
                timestamp_ns: 1,
            },
            vec![Seed {
                id: 4,
                bounds: [200., 200., 600., 600.],
            }],
        )
    }
    fn moved(frame: &FrameEvidence, seeds: &[Seed], scale: f64) -> Vec<Group> {
        seeds
            .iter()
            .map(|s| Group {
                id: s.id,
                points: seed_points(s, frame)
                    .into_iter()
                    .map(|p| Point {
                        xy: [p[0] * scale + 20., p[1] * scale + 10.],
                        visibility: 0.9,
                    })
                    .collect(),
            })
            .collect()
    }
    #[test]
    fn translation_scale_and_outlier_preserve_geometry() {
        let (mut frame, seeds) = setup();
        let mut tracker = Tracker::new(&frame, &seeds, true).unwrap();
        let mut groups = moved(&frame, &seeds, 1.2);
        groups[0].points[0].xy = [1., 1.];
        frame.timestamp_ns += 1;
        let samples = tracker.update(&frame, &groups).unwrap();
        let actual = samples[0].bounds.unwrap();
        for (a, b) in actual.into_iter().zip([265., 265., 745., 745.]) {
            assert!((a - b).abs() < 1e-6);
        }
        assert_eq!(samples[0].inliers, 8);
    }
    #[test]
    fn lost_points_do_not_reappear() {
        let (mut frame, seeds) = setup();
        let mut tracker = Tracker::new(&frame, &seeds, true).unwrap();
        let good = moved(&frame, &seeds, 1.);
        let mut bad = good.clone();
        for p in &mut bad[0].points[..5] {
            p.visibility = 0.5;
        }
        frame.timestamp_ns += 1;
        assert_eq!(
            tracker.update(&frame, &bad).unwrap()[0].reason,
            Some("insufficient_points")
        );
        frame.timestamp_ns += 1;
        assert_eq!(
            tracker.update(&frame, &good).unwrap()[0].reason,
            Some("track_lost")
        );
    }
    #[test]
    fn scattered_and_collapsed_points_are_not_boxes() {
        let (mut frame, seeds) = setup();
        let mut groups = moved(&frame, &seeds, 1.);
        for (i, p) in groups[0].points.iter_mut().enumerate() {
            p.xy = [250. + (i % 2) as f64 * 70., 100. + (i % 3) as f64 * 50.];
        }
        let mut tracker = Tracker::new(&frame, &seeds, true).unwrap();
        frame.timestamp_ns += 1;
        assert!(tracker.update(&frame, &groups).unwrap()[0].bounds.is_none());
        let mut tracker = Tracker::new(&frame, &seeds, false).unwrap();
        for p in &mut groups[0].points {
            p.xy = [200., 200.];
        }
        frame.timestamp_ns += 1;
        assert_eq!(
            tracker.update(&frame, &groups).unwrap()[0].reason,
            Some("scale")
        );
    }
    #[test]
    fn subpixel_seeds_cannot_produce_degenerate_correspondences() {
        let (frame, mut seeds) = setup();
        seeds[0].bounds = [500., 500., 500.0000000000001, 600.];
        assert!(Tracker::new(&frame, &seeds, false).is_err());
        seeds[0].bounds = [0., 0., 1.25, 2.5];
        assert!(Tracker::new(&frame, &seeds, false).is_ok());
    }

    #[test]
    fn malformed_updates_are_atomic() {
        let (mut frame, seeds) = setup();
        let mut tracker = Tracker::new(&frame, &seeds, true).unwrap();
        let groups = moved(&frame, &seeds, 1.);
        assert!(tracker.update(&frame, &groups).is_err());
        frame.timestamp_ns += 1;
        let mut bad = groups.clone();
        bad[0].points[8].xy[0] = f64::NAN;
        assert!(tracker.update(&frame, &bad).is_err());
        bad = groups.clone();
        bad[0].id = 5;
        assert!(tracker.update(&frame, &bad).is_err());
        assert!(tracker.update(&frame, &groups).unwrap()[0].bounds.is_some());
        frame.timestamp_ns += 2_000_000_001;
        assert!(tracker.update(&frame, &groups).is_err());
        let mut duplicate = seeds.clone();
        duplicate.push(seeds[0].clone());
        assert!(Tracker::new(&frame, &duplicate, true).is_err());
    }
}
