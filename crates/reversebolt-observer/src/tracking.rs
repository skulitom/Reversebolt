//! Image-patch correspondence from explicit seed regions, not semantic identity.
//! Correlation, appearance agreement and round trips are heuristic rejection gates.
use crate::{Frame, FrameEvidence};
use image::{RgbImage, imageops::FilterType};
use reversebolt_core::Result;
use serde::{Deserialize, Serialize};

const GRID: usize = 12;
const SIDE: u32 = 640;
const SCORE: f32 = 0.72;
const ANCHOR: f32 = 0.45;
const MARGIN: f32 = 0.06;
const ROUND_TRIP: f64 = 2.5;

pub fn parameters() -> serde_json::Value {
    serde_json::json!({"maximum_side":SIDE,"patch_grid":GRID,"minimum_correlation":SCORE,
        "minimum_anchor_correlation":ANCHOR,"minimum_distinct_peak_margin":MARGIN,
        "maximum_round_trip_processing_pixels":ROUND_TRIP,"maximum_gap_ns":2_000_000_000u64,
        "scale_factors":[0.9,1.0,1.1],"coarse_stride":4,"refined_distinct_peaks":4,
        "minimum_seed_side":3,"minimum_channel_centered_variance":0.0004,
        "search_radius":"max(box width,height)*1.5 clamped to 16..48 processing pixels"})
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seed {
    pub id: u64,
    /// [left, top, right, bottom], normalized to 0..1000 in the source image.
    pub bounds: [f64; 4],
}

#[derive(Clone, Debug, Serialize)]
pub struct Sample {
    pub id: u64,
    pub bounds: Option<[f64; 4]>,
    pub status: &'static str,
    pub reason: Option<&'static str>,
    pub correlation: Option<f32>,
    pub anchor_correlation: Option<f32>,
    pub distinct_peak_margin: Option<f32>,
    pub round_trip_error_px: Option<f64>,
    /// Translation between two accepted patch centers, in source-image pixels.
    /// Not camera compensation, world movement or a verified object trajectory.
    pub delta_px: Option<[f64; 2]>,
}

struct Image {
    rgb: RgbImage,
    source: FrameEvidence,
}
impl Image {
    fn from_frame(frame: &Frame) -> Result<Self> {
        let decoded = image::load_from_memory_with_format(&frame.bytes, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        if decoded.width() != frame.evidence.width || decoded.height() != frame.evidence.height {
            return Err("Image dimensions differ from their evidence.".into());
        }
        let ratio = (SIDE as f64 / frame.evidence.width.max(frame.evidence.height) as f64).min(1.);
        let width = (frame.evidence.width as f64 * ratio).round().max(1.) as u32;
        let height = (frame.evidence.height as f64 * ratio).round().max(1.) as u32;
        Ok(Self {
            rgb: decoded
                .resize_exact(width, height, FilterType::Triangle)
                .to_rgb8(),
            source: frame.evidence.clone(),
        })
    }
    fn box_pixels(&self, b: [f64; 4]) -> [f64; 4] {
        [
            b[0] * self.rgb.width() as f64 / 1000.,
            b[1] * self.rgb.height() as f64 / 1000.,
            (b[2] - b[0]) * self.rgb.width() as f64 / 1000.,
            (b[3] - b[1]) * self.rgb.height() as f64 / 1000.,
        ]
    }
    fn normalized(&self, b: [f64; 4]) -> [f64; 4] {
        [
            b[0] * 1000. / self.rgb.width() as f64,
            b[1] * 1000. / self.rgb.height() as f64,
            (b[0] + b[2]) * 1000. / self.rgb.width() as f64,
            (b[1] + b[3]) * 1000. / self.rgb.height() as f64,
        ]
    }
}

#[derive(Clone)]
struct Patch {
    values: [f32; GRID * GRID * 3],
}
impl Patch {
    fn sample(image: &RgbImage, b: [f64; 4]) -> Option<Self> {
        if b[0] < 0.
            || b[1] < 0.
            || b[2] < 3.
            || b[3] < 3.
            || b[0] + b[2] > image.width() as f64
            || b[1] + b[3] > image.height() as f64
        {
            return None;
        }
        let mut values = [0.; GRID * GRID * 3];
        let mut means = [0.; 3];
        for row in 0..GRID {
            for column in 0..GRID {
                let x = (b[0] + (column as f64 + 0.5) * b[2] / GRID as f64 - 0.5)
                    .clamp(0., image.width() as f64 - 1.);
                let y = (b[1] + (row as f64 + 0.5) * b[3] / GRID as f64 - 0.5)
                    .clamp(0., image.height() as f64 - 1.);
                let (xx, yy) = (x.floor() as u32, y.floor() as u32);
                let (dx, dy) = ((x - xx as f64) as f32, (y - yy as f64) as f32);
                let p = [
                    image.get_pixel(xx, yy),
                    image.get_pixel((xx + 1).min(image.width() - 1), yy),
                    image.get_pixel(xx, (yy + 1).min(image.height() - 1)),
                    image.get_pixel(
                        (xx + 1).min(image.width() - 1),
                        (yy + 1).min(image.height() - 1),
                    ),
                ];
                for channel in 0..3 {
                    let value = ((1. - dy)
                        * ((1. - dx) * p[0][channel] as f32 + dx * p[1][channel] as f32)
                        + dy * ((1. - dx) * p[2][channel] as f32 + dx * p[3][channel] as f32))
                        / 255.;
                    values[(row * GRID + column) * 3 + channel] = value;
                    means[channel] += value / (GRID * GRID) as f32;
                }
            }
        }
        let mut energy = 0.;
        for (i, v) in values.iter_mut().enumerate() {
            *v -= means[i % 3];
            energy += *v * *v;
        }
        // Reject near-uniform support rather than normalizing noise into a perfect match.
        if energy / ((GRID * GRID * 3) as f32) < 0.0004 {
            return None;
        }
        let scale = energy.sqrt();
        for v in &mut values {
            *v /= scale;
        }
        Some(Self { values })
    }
    fn similarity(&self, other: &Self) -> f32 {
        self.values
            .iter()
            .zip(&other.values)
            .map(|(a, b)| a * b)
            .sum::<f32>()
            .clamp(-1., 1.)
    }
}

#[derive(Clone, Copy)]
struct Match {
    bounds: [f64; 4],
    score: f32,
}
struct Search {
    best: Match,
    margin: f32,
    patch: Patch,
}
fn center(b: [f64; 4]) -> [f64; 2] {
    [b[0] + b[2] / 2., b[1] + b[3] / 2.]
}
fn overlap(a: [f64; 4], b: [f64; 4]) -> f64 {
    let intersection = ((a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0])).max(0.)
        * ((a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1])).max(0.);
    intersection / (a[2] * a[3] + b[2] * b[3] - intersection).max(1e-9)
}

fn search(image: &RgbImage, template: &Patch, from: [f64; 4]) -> Option<Search> {
    let c = center(from);
    let radius = (from[2].max(from[3]) * 1.5).clamp(16., 48.) as i32;
    let mut choices = Vec::new();
    let mut inspect = |x: f64, y: f64, w: f64, h: f64| {
        let b = [x - w / 2., y - h / 2., w, h];
        if let Some(p) = Patch::sample(image, b) {
            choices.push(Match {
                bounds: b,
                score: template.similarity(&p),
            });
        }
    };
    for scale in [0.9, 1., 1.1] {
        for y in (-radius..=radius).step_by(4) {
            for x in (-radius..=radius).step_by(4) {
                inspect(
                    c[0] + x as f64,
                    c[1] + y as f64,
                    from[2] * scale,
                    from[3] * scale,
                );
            }
        }
        inspect(c[0], c[1], from[2] * scale, from[3] * scale);
    }
    // Refine competing peaks as well as the winner: a coarse grid can otherwise
    // miss a duplicate's exact alignment and give a misleadingly large margin.
    choices.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut peaks: Vec<Match> = Vec::new();
    for choice in &choices {
        if peaks.iter().all(|p| overlap(p.bounds, choice.bounds) < 0.5) {
            peaks.push(*choice);
            if peaks.len() == 4 {
                break;
            }
        }
    }
    for coarse in peaks {
        let cc = center(coarse.bounds);
        for scale in [0.9, 1., 1.1] {
            for y in -4..=4 {
                for x in -4..=4 {
                    let b = [
                        cc[0] + x as f64 - from[2] * scale / 2.,
                        cc[1] + y as f64 - from[3] * scale / 2.,
                        from[2] * scale,
                        from[3] * scale,
                    ];
                    if let Some(p) = Patch::sample(image, b) {
                        choices.push(Match {
                            bounds: b,
                            score: template.similarity(&p),
                        });
                    }
                }
            }
        }
    }
    let best = *choices.iter().max_by(|a, b| a.score.total_cmp(&b.score))?;
    let second = choices
        .iter()
        .filter(|m| overlap(m.bounds, best.bounds) < 0.5)
        .map(|m| m.score)
        .fold(-1., f32::max);
    Some(Search {
        best,
        margin: best.score - second,
        patch: Patch::sample(image, best.bounds)?,
    })
}

struct Track {
    id: u64,
    bounds: [f64; 4],
    anchor: Option<Patch>,
    previous: Option<Patch>,
    lost: bool,
}
pub struct Tracker {
    previous: Image,
    tracks: Vec<Track>,
    gated: bool,
}
impl Tracker {
    /// `gated=false` is an intentionally naive comparator for offline evaluation.
    pub fn new(frame: &Frame, seeds: &[Seed], gated: bool) -> Result<Self> {
        if seeds.len() > 6 {
            return Err("At most six seed regions are supported.".into());
        }
        let previous = Image::from_frame(frame)?;
        let mut tracks = vec![];
        for seed in seeds {
            if tracks.iter().any(|t: &Track| t.id == seed.id)
                || seed
                    .bounds
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1000.).contains(v))
                || seed.bounds[0] >= seed.bounds[2]
                || seed.bounds[1] >= seed.bounds[3]
            {
                return Err("Invalid or repeated seed geometry.".into());
            }
            let bounds = previous.box_pixels(seed.bounds);
            let patch = Patch::sample(&previous.rgb, bounds);
            tracks.push(Track {
                id: seed.id,
                bounds,
                anchor: patch.clone(),
                previous: patch.clone(),
                lost: patch.is_none(),
            });
        }
        Ok(Self {
            previous,
            tracks,
            gated,
        })
    }
    pub fn update(&mut self, frame: &Frame) -> Result<Vec<Sample>> {
        if frame.evidence.timestamp_ns <= self.previous.source.timestamp_ns {
            return Err("Tracking frames must be strictly chronological.".into());
        }
        let current = Image::from_frame(frame)?;
        let incompatible = if (current.source.width, current.source.height)
            != (self.previous.source.width, self.previous.source.height)
        {
            Some("geometry_changed")
        } else if current.source.timestamp_ns - self.previous.source.timestamp_ns > 2_000_000_000 {
            Some("time_gap")
        } else {
            None
        };
        let mut output = vec![];
        for track in &mut self.tracks {
            let mut sample = Sample {
                id: track.id,
                bounds: None,
                status: "missing",
                reason: None,
                correlation: None,
                anchor_correlation: None,
                distinct_peak_margin: None,
                round_trip_error_px: None,
                delta_px: None,
            };
            let reason = if track.lost {
                Some("track_lost")
            } else {
                incompatible
            };
            if reason.is_some() {
                track.lost = true;
                sample.reason = reason;
                output.push(sample);
                continue;
            }
            let Some(found) = search(&current.rgb, track.previous.as_ref().unwrap(), track.bounds)
            else {
                track.lost = true;
                sample.reason = Some("no_textured_support");
                output.push(sample);
                continue;
            };
            let anchor = track.anchor.as_ref().unwrap().similarity(&found.patch);
            sample.correlation = Some(found.best.score);
            sample.anchor_correlation = Some(anchor);
            sample.distinct_peak_margin = Some(found.margin);
            let mut rejection = if found.best.score < SCORE {
                Some("weak_correlation")
            } else if found.margin < MARGIN {
                Some("ambiguous_match")
            } else if anchor < ANCHOR {
                Some("appearance_changed")
            } else {
                None
            };
            if self.gated && rejection.is_none() {
                if let Some(back) = search(&self.previous.rgb, &found.patch, found.best.bounds) {
                    let a = center(track.bounds);
                    let b = center(back.best.bounds);
                    let error = (a[0] - b[0]).hypot(a[1] - b[1]);
                    sample.round_trip_error_px = Some(error);
                    if error > ROUND_TRIP || back.best.score < SCORE || back.margin < MARGIN {
                        rejection = Some("round_trip_failed");
                    }
                } else {
                    rejection = Some("round_trip_failed");
                }
            }
            if self.gated && rejection.is_some() {
                track.lost = true;
                sample.reason = rejection;
            } else {
                let a = center(track.bounds);
                let b = center(found.best.bounds);
                sample.bounds = Some(current.normalized(found.best.bounds));
                sample.status = "provisional";
                sample.delta_px = Some([
                    (b[0] - a[0]) * current.source.width as f64 / current.rgb.width() as f64,
                    (b[1] - a[1]) * current.source.height as f64 / current.rgb.height() as f64,
                ]);
                track.bounds = found.best.bounds;
                track.previous = Some(found.patch);
            }
            output.push(sample);
        }
        self.previous = current;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    fn scene(locations: &[(u32, u32)], size: (u32, u32), time: u64) -> Frame {
        let mut image = RgbImage::from_pixel(size.0, size.1, Rgb([10, 10, 10]));
        for &(left, top) in locations {
            for y in 0..20 {
                for x in 0..24 {
                    // Distinct broad blocks, independent of search or score logic.
                    let n = (x / 4 * 13 + y / 4 * 7 + (x / 4) * (y / 4) * 11) % 29;
                    image.put_pixel(
                        left + x,
                        top + y,
                        Rgb([(45 + n * 7) as u8, (230 - n * 6) as u8, (30 + n * 5) as u8]),
                    );
                }
            }
        }
        let mut png = std::io::Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png).unwrap();
        Frame::from_png(png.into_inner(), time).unwrap()
    }
    fn seed(x: f64, y: f64) -> Seed {
        Seed {
            id: 42,
            bounds: [
                x / 128. * 1000.,
                y / 96. * 1000.,
                (x + 24.) / 128. * 1000.,
                (y + 20.) / 96. * 1000.,
            ],
        }
    }

    #[test]
    fn follows_translation_with_source_pixel_deltas() {
        let start = scene(&[(32, 32)], (128, 96), 1);
        let mut tracker = Tracker::new(&start, &[seed(32., 32.)], true).unwrap();
        for (time, x, y, expected) in [(2, 36, 32, [4., 0.]), (3, 38, 35, [2., 3.])] {
            let frame = scene(&[(x, y)], (128, 96), time);
            let samples = tracker.update(&frame).unwrap();
            let actual = &samples[0];
            assert_eq!(actual.status, "provisional", "{actual:?}");
            let delta = actual.delta_px.unwrap();
            assert!(
                (delta[0] - expected[0]).abs() < 1.1 && (delta[1] - expected[1]).abs() < 1.1,
                "{actual:?}"
            );
            let bounds = actual.bounds.unwrap();
            assert!((bounds[0] / 1000. * 128. - x as f64).abs() < 1.1);
            assert!((bounds[1] / 1000. * 96. - y as f64).abs() < 1.1);
        }
    }

    #[test]
    fn duplicate_match_abstains_and_never_reacquires() {
        let start = scene(&[(48, 32)], (128, 96), 1);
        let mut tracker = Tracker::new(&start, &[seed(48., 32.)], true).unwrap();
        let ambiguous = tracker
            .update(&scene(&[(24, 32), (72, 32)], (128, 96), 2))
            .unwrap();
        assert_eq!(
            ambiguous[0].reason,
            Some("ambiguous_match"),
            "{ambiguous:?}"
        );
        assert!(ambiguous[0].bounds.is_none());
        let returned = tracker.update(&scene(&[(48, 32)], (128, 96), 3)).unwrap();
        assert!(returned[0].bounds.is_none());
        assert_eq!(returned[0].reason, Some("track_lost"));
    }

    #[test]
    fn disappearance_and_flat_seeds_are_missing() {
        let start = scene(&[(32, 32)], (128, 96), 1);
        let blank = scene(&[], (128, 96), 2);
        let mut tracker = Tracker::new(&start, &[seed(32., 32.)], true).unwrap();
        assert!(tracker.update(&blank).unwrap()[0].bounds.is_none());
        assert!(
            tracker.update(&scene(&[(32, 32)], (128, 96), 3)).unwrap()[0]
                .bounds
                .is_none()
        );
        let mut flat = Tracker::new(&blank, &[seed(32., 32.)], true).unwrap();
        assert!(
            flat.update(&scene(&[(32, 32)], (128, 96), 3)).unwrap()[0]
                .bounds
                .is_none()
        );
    }

    #[test]
    fn chronology_errors_do_not_advance_but_gaps_and_resize_revoke() {
        let start = scene(&[(32, 32)], (128, 96), 100);
        let mut tracker = Tracker::new(&start, &[seed(32., 32.)], true).unwrap();
        assert!(tracker.update(&start).is_err());
        assert_eq!(
            tracker.update(&scene(&[(36, 32)], (128, 96), 101)).unwrap()[0].status,
            "provisional"
        );
        assert_eq!(
            tracker
                .update(&scene(&[(36, 32)], (128, 96), 3_000_000_000))
                .unwrap()[0]
                .reason,
            Some("time_gap")
        );
        let mut resized = Tracker::new(&start, &[seed(32., 32.)], true).unwrap();
        assert_eq!(
            resized.update(&scene(&[(32, 32)], (127, 96), 101)).unwrap()[0].reason,
            Some("geometry_changed")
        );
    }

    #[test]
    fn invalid_seeds_are_rejected() {
        let start = scene(&[(32, 32)], (128, 96), 1);
        let good = seed(32., 32.);
        assert!(Tracker::new(&start, &[good.clone(), good.clone()], true).is_err());
        assert!(Tracker::new(&start, &vec![good; 7], true).is_err());
        for bounds in [
            [0., 0., f64::NAN, 10.],
            [-1., 0., 10., 10.],
            [10., 0., 0., 10.],
            [0., 0., 1001., 10.],
        ] {
            assert!(Tracker::new(&start, &[Seed { id: 1, bounds }], true).is_err());
        }
    }
}
