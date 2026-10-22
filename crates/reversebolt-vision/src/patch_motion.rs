//! Distributed screen-space correspondence, never world motion or object identity.
use image::{
    GrayImage,
    imageops::{self, FilterType},
};
use reversebolt_core::Result;
use serde::Serialize;

pub const METRIC: &str = "rgb601-triangle256x144-corners48-pyramid2-patch9-zmssd-r16x3-fb1-v2";
pub const LIMITATION: &str = "Sparse integer displacement of source-selected image patches in a normalized 256x144 image, not physical speed, camera motion, an actor or input causality. Feature selection omits weak texture. Repeated patterns, large motion, occlusion and appearance changes may be unresolved. Zero displacement is not proof of no movement. Consensus covers only its stated patches/cells; thresholds are heuristics, not calibrated probabilities.";
const SIZE: i32 = 9;
const HALF: i32 = SIZE / 2;
const N: i64 = (SIZE * SIZE) as i64;
const DENOM: i64 = N * N;
const WIDTH: i32 = 256;
const HEIGHT: i32 = 144;
const COLS: i32 = 6;
const ROWS: i32 = 4;
const MAX_PATCHES: usize = 48;
const MIN_DISTANCE: i32 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Matched,
    LowTexture,
    Ambiguous,
    PoorMatch,
    SearchBoundary,
    InconsistentReverse,
}
#[derive(Debug, Serialize)]
pub struct Patch {
    pub anchor: [i32; 2],
    pub corner_score: f64,
    pub status: Status,
    pub displacement: Option<[i32; 2]>,
    pub brightness_change: Option<f64>,
    pub error_rms: Option<f64>,
}
#[derive(Debug, Serialize)]
pub struct Consensus {
    pub displacement: [i32; 2],
    pub tolerance_pixels: i32,
    pub supporting_patches: usize,
    pub supporting_cells: usize,
}
#[derive(Debug, Serialize)]
pub struct Selection {
    pub candidate_patches: usize,
    pub selected_patches: usize,
    pub max_patches: usize,
    pub grid: [i32; 2],
    pub min_distance_pixels: i32,
    pub occupied_cells: usize,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub metric: &'static str,
    pub source_dimensions: [u32; 2],
    pub measurement_dimensions: [u32; 2],
    pub patch_size: i32,
    pub search_radius: i32,
    pub selection: Selection,
    pub matched_patches: usize,
    pub matched_cells: usize,
    pub zero_displacement_patches: usize,
    pub nonzero_displacement_patches: usize,
    pub patches: Vec<Patch>,
    pub consensus: Option<Consensus>,
    pub limitation: &'static str,
}
fn block(image: &GrayImage, center: [i32; 2]) -> [i64; N as usize] {
    let mut pixels = [0; N as usize];
    let mut index = 0;
    for y in center[1] - HALF..=center[1] + HALF {
        for x in center[0] - HALF..=center[0] + HALF {
            pixels[index] = i64::from(image.get_pixel(x as u32, y as u32)[0]);
            index += 1;
        }
    }
    pixels
}
fn cell([x, y]: [i32; 2]) -> usize {
    (y * ROWS / HEIGHT * COLS + x * COLS / WIDTH) as usize
}
fn occupied<'a>(points: impl Iterator<Item = &'a [i32; 2]>) -> Vec<usize> {
    let mut counts = vec![0; (COLS * ROWS) as usize];
    for point in points {
        counts[cell(*point)] += 1;
    }
    counts
}
fn corner_score(image: &GrayImage, [x, y]: [i32; 2]) -> f64 {
    let (mut xx, mut yy, mut xy) = (0., 0., 0.);
    for dy in -HALF..=HALF {
        for dx in -HALF..=HALF {
            let (px, py) = ((x + dx) as u32, (y + dy) as u32);
            let gx = (f64::from(image.get_pixel(px + 1, py)[0])
                - f64::from(image.get_pixel(px - 1, py)[0]))
                / 2.;
            let gy = (f64::from(image.get_pixel(px, py + 1)[0])
                - f64::from(image.get_pixel(px, py - 1)[0]))
                / 2.;
            xx += gx * gx;
            yy += gy * gy;
            xy += gx * gy;
        }
    }
    ((xx + yy - ((xx - yy).powi(2) + 4. * xy * xy).sqrt()) / (2. * N as f64)).max(0.)
}
fn select(image: &GrayImage) -> (Vec<(f64, [i32; 2])>, Selection) {
    let mut candidates = vec![];
    for y in (12..HEIGHT - 12).step_by(2) {
        for x in (12..WIDTH - 12).step_by(2) {
            let score = corner_score(image, [x, y]);
            if score >= 16. {
                candidates.push((score, [x, y]));
            }
        }
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let candidate_patches = candidates.len();
    let mut chosen: Vec<(f64, [i32; 2])> = vec![];
    let mut cells = [0; (COLS * ROWS) as usize];
    for candidate in candidates {
        let slot = cell(candidate.1);
        if cells[slot] == 2
            || chosen.iter().any(|(_, p)| {
                (p[0] - candidate.1[0]).pow(2) + (p[1] - candidate.1[1]).pow(2)
                    < MIN_DISTANCE.pow(2)
            })
        {
            continue;
        }
        cells[slot] += 1;
        chosen.push(candidate);
    }
    let selection = Selection {
        candidate_patches,
        selected_patches: chosen.len(),
        max_patches: MAX_PATCHES,
        grid: [COLS, ROWS],
        min_distance_pixels: MIN_DISTANCE,
        occupied_cells: cells.iter().filter(|n| **n > 0).count(),
    };
    (chosen, selection)
}
#[derive(Clone, Copy)]
struct Match {
    delta: [i32; 2],
    cost: i64,
    brightness: i64,
}
fn search(
    source: &GrayImage,
    target: &GrayImage,
    center: [i32; 2],
    guess: [i32; 2],
    radius: i32,
) -> std::result::Result<Match, Status> {
    // Source bounds are checked as well because the reverse anchor is data-dependent.
    let valid = |p: [i32; 2]| {
        p[0] >= HALF
            && p[1] >= HALF
            && p[0] < source.width() as i32 - HALF
            && p[1] < source.height() as i32 - HALF
    };
    if !valid(center) {
        return Err(Status::SearchBoundary);
    }
    let reference = block(source, center);
    let sum: i64 = reference.iter().sum();
    if N * reference.iter().map(|p| p * p).sum::<i64>() - sum * sum < 64 * DENOM {
        return Err(Status::LowTexture);
    }
    let mut scores = Vec::with_capacity(((radius * 2 + 1).pow(2)) as usize);
    for dy in guess[1] - radius..=guess[1] + radius {
        for dx in guess[0] - radius..=guess[0] + radius {
            let point = [center[0] + dx, center[1] + dy];
            if !valid(point) {
                continue;
            }
            let other = block(target, point);
            let (mut sum, mut squared) = (0, 0);
            for (a, b) in reference.iter().zip(other) {
                let d = b - a;
                sum += d;
                squared += d * d;
            }
            scores.push(Match {
                delta: [dx, dy],
                cost: N * squared - sum * sum,
                brightness: sum,
            });
        }
    }
    let best = *scores
        .iter()
        .min_by_key(|m| m.cost)
        .ok_or(Status::SearchBoundary)?;
    let point = [center[0] + best.delta[0], center[1] + best.delta[1]];
    if (0..2).any(|i| (best.delta[i] - guess[i]).abs() == radius)
        || point[0] == HALF
        || point[1] == HALF
        || point[0] == source.width() as i32 - HALF - 1
        || point[1] == source.height() as i32 - HALF - 1
    {
        return Err(Status::SearchBoundary);
    }
    if best.cost > 144 * DENOM {
        return Err(Status::PoorMatch);
    }
    let second = scores
        .iter()
        .filter(|m| (0..2).any(|i| (m.delta[i] - best.delta[i]).abs() > 1))
        .map(|m| m.cost)
        .min()
        .ok_or(Status::Ambiguous)?;
    if second * 4 < best.cost * 5 + 36 * DENOM {
        return Err(Status::Ambiguous);
    }
    Ok(best)
}
fn track(
    a: &[GrayImage; 2],
    b: &[GrayImage; 2],
    anchor: [i32; 2],
) -> std::result::Result<Match, Status> {
    let coarse = search(&a[1], &b[1], [anchor[0] / 2, anchor[1] / 2], [0, 0], 16)?;
    search(
        &a[0],
        &b[0],
        anchor,
        [coarse.delta[0] * 2, coarse.delta[1] * 2],
        3,
    )
}
fn consensus(patches: &[Patch]) -> Option<Consensus> {
    let matched: Vec<_> = patches
        .iter()
        .filter(|p| p.displacement.is_some())
        .collect();
    let (delta, supporters) = matched
        .iter()
        .map(|p| {
            let d = p.displacement.unwrap();
            let supporters: Vec<_> = matched
                .iter()
                .filter(|p| (0..2).all(|i| (p.displacement.unwrap()[i] - d[i]).abs() <= 1))
                .collect();
            (d, supporters)
        })
        .max_by_key(|(_, v)| v.len())?;
    if supporters.len() < 10 || supporters.len() * 4 < matched.len() * 3 {
        return None;
    }
    let cells = occupied(supporters.iter().map(|p| &p.anchor));
    let occupied_cells = cells.iter().filter(|n| **n > 0).count();
    let rows = (0..ROWS)
        .filter(|y| (0..COLS).any(|x| cells[(y * COLS + x) as usize] > 0))
        .count();
    let cols = (0..COLS)
        .filter(|x| (0..ROWS).any(|y| cells[(y * COLS + x) as usize] > 0))
        .count();
    if occupied_cells < 6 || rows < 2 || cols < 2 {
        return None;
    }
    Some(Consensus {
        displacement: delta,
        tolerance_pixels: 1,
        supporting_patches: supporters.len(),
        supporting_cells: occupied_cells,
    })
}
fn pyramid(image: GrayImage) -> [GrayImage; 2] {
    let coarse = imageops::resize(&image, 128, 72, FilterType::Triangle);
    [image, coarse]
}
fn prepared(a: GrayImage, b: GrayImage, dimensions: [u32; 2]) -> Report {
    debug_assert_eq!(a.dimensions(), (256, 144));
    debug_assert_eq!(b.dimensions(), (256, 144));
    let (chosen, selection) = select(&a);
    let (a, b) = (pyramid(a), pyramid(b));
    let mut patches = vec![];
    for (corner_score, anchor) in chosen {
        let measured = track(&a, &b, anchor).and_then(|forward| {
            let backward = track(
                &b,
                &a,
                [anchor[0] + forward.delta[0], anchor[1] + forward.delta[1]],
            )
            .map_err(|_| Status::InconsistentReverse)?;
            if (0..2).any(|i| (forward.delta[i] + backward.delta[i]).abs() > 1) {
                return Err(Status::InconsistentReverse);
            }
            Ok(forward)
        });
        patches.push(match measured {
            Ok(m) => Patch {
                anchor,
                corner_score,
                status: Status::Matched,
                displacement: Some(m.delta),
                brightness_change: Some(m.brightness as f64 / N as f64),
                error_rms: Some((m.cost as f64 / DENOM as f64).sqrt()),
            },
            Err(status) => Patch {
                anchor,
                corner_score,
                status,
                displacement: None,
                brightness_change: None,
                error_rms: None,
            },
        });
    }
    let matched: Vec<_> = patches
        .iter()
        .filter(|p| p.displacement.is_some())
        .collect();
    let zero = matched
        .iter()
        .filter(|p| p.displacement == Some([0, 0]))
        .count();
    let consensus = consensus(&patches);
    Report {
        metric: METRIC,
        source_dimensions: dimensions,
        measurement_dimensions: [256, 144],
        patch_size: SIZE,
        search_radius: 35,
        selection,
        matched_patches: matched.len(),
        matched_cells: occupied(matched.iter().map(|p| &p.anchor))
            .iter()
            .filter(|n| **n > 0)
            .count(),
        zero_displacement_patches: zero,
        nonzero_displacement_patches: matched.len() - zero,
        patches,
        consensus,
        limitation: LIMITATION,
    }
}
/// Bounded image correspondence. Callers separately bind hashes, time/input and epoch.
pub fn measure(before: &[u8], after: &[u8]) -> Result<Report> {
    let a = crate::response::grayscale(before)?;
    let b = crate::response::grayscale(after)?;
    if a.dimensions() != b.dimensions() {
        return Err("Image geometry changed.".into());
    }
    let dimensions = [a.width(), a.height()];
    Ok(prepared(
        imageops::resize(&a, 256, 144, FilterType::Triangle),
        imageops::resize(&b, 256, 144, FilterType::Triangle),
        dimensions,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;
    fn texture(seed: u32) -> GrayImage {
        GrayImage::from_fn(256, 144, |x, y| {
            let mut n = (x + y * 256)
                .wrapping_add(seed)
                .wrapping_mul(747796405)
                .wrapping_add(2891336453);
            n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
            Luma([32 + (((n >> 22) ^ n) % 128) as u8])
        })
    }
    fn shift(source: &GrayImage, delta: [i32; 2], bright: u8) -> GrayImage {
        GrayImage::from_fn(256, 144, |x, y| {
            let (sx, sy) = (x as i32 - delta[0], y as i32 - delta[1]);
            if (0..256).contains(&sx) && (0..144).contains(&sy) {
                Luma([source.get_pixel(sx as u32, sy as u32)[0].saturating_add(bright)])
            } else {
                Luma([0])
            }
        })
    }
    fn png(image: &GrayImage) -> Vec<u8> {
        let mut file = std::io::Cursor::new(vec![]);
        image.write_to(&mut file, image::ImageFormat::Png).unwrap();
        file.into_inner()
    }
    #[test]
    fn selects_distributed_corners_deterministically_and_ignores_flat_edges() {
        let image = texture(1);
        let (chosen, selection) = select(&image);
        assert_eq!(chosen, select(&image).0);
        assert_eq!(chosen.len(), 48);
        assert_eq!(selection.occupied_cells, 24);
        for (i, a) in chosen.iter().enumerate() {
            for b in &chosen[i + 1..] {
                assert!((a.1[0] - b.1[0]).pow(2) + (a.1[1] - b.1[1]).pow(2) >= 144);
            }
        }
        for image in [
            GrayImage::from_pixel(256, 144, Luma([60])),
            GrayImage::from_fn(256, 144, |x, _| Luma([if x < 128 { 40 } else { 180 }])),
        ] {
            assert_eq!(select(&image).1.selected_patches, 0);
        }
    }
    #[test]
    fn recovers_signed_small_and_large_translation_with_brightness() {
        let before = texture(1);
        for delta in [[0, 0], [3, -2], [18, 8], [-24, -10]] {
            let after = shift(&before, delta, 30);
            let measured = measure(&png(&before), &png(&after)).unwrap();
            assert!(
                measured.matched_patches >= 20,
                "{delta:?}: {}",
                measured.matched_patches
            );
            assert_eq!(measured.consensus.unwrap().displacement, delta);
            for patch in measured
                .patches
                .iter()
                .filter(|p| p.status == Status::Matched)
            {
                assert_eq!(patch.displacement, Some(delta));
                assert_eq!(patch.brightness_change, Some(30.));
                assert_eq!(patch.error_rms, Some(0.));
            }
            let reverse = prepared(after, before.clone(), [256, 144]);
            assert_eq!(
                reverse.consensus.unwrap().displacement,
                [-delta[0], -delta[1]]
            );
        }
    }
    #[test]
    fn repetition_unrelated_and_out_of_range_are_unresolved_not_stationary() {
        let repeated = GrayImage::from_fn(256, 144, |x, y| {
            Luma([if (x / 4 + y / 4) % 2 == 0 { 40 } else { 180 }])
        });
        let before = texture(1);
        for (a, b) in [
            (repeated.clone(), repeated),
            (before.clone(), texture(9999)),
            (before.clone(), shift(&before, [48, 0], 0)),
        ] {
            let measured = prepared(a, b, [256, 144]);
            assert_eq!(measured.matched_patches, 0);
            assert!(measured.consensus.is_none());
            assert!(measured.patches.iter().all(|p| p.displacement.is_none()));
        }
    }
    #[test]
    fn opposite_motion_is_not_averaged_and_occlusion_remains_missing() {
        let before = texture(5);
        let left = shift(&before, [18, 0], 0);
        let right = shift(&before, [-18, 0], 0);
        let after = GrayImage::from_fn(256, 144, |x, y| {
            *if x < 128 {
                left.get_pixel(x, y)
            } else {
                right.get_pixel(x, y)
            }
        });
        let measured = prepared(before.clone(), after, [256, 144]);
        assert!(measured.matched_patches >= 15);
        assert!(measured.consensus.is_none());
        for p in measured.patches.iter().filter(|p| p.displacement.is_some()) {
            assert!([[18, 0], [-18, 0]].contains(&p.displacement.unwrap()));
        }
        let after = GrayImage::from_fn(256, 144, |x, y| {
            if x < 128 {
                Luma([0])
            } else {
                *before.get_pixel(x, y)
            }
        });
        let measured = prepared(before, after, [256, 144]);
        assert!(measured.matched_patches > 0 && measured.matched_patches < 48);
        assert!(
            measured
                .patches
                .iter()
                .filter(|p| p.anchor[0] < 100)
                .all(|p| p.displacement.is_none())
        );
    }
    #[test]
    fn localized_support_cannot_claim_broad_consensus_and_invalid_images_fail() {
        let patches: Vec<_> = (0..12)
            .map(|i| Patch {
                anchor: [20 + i, 20],
                corner_score: 20.,
                status: Status::Matched,
                displacement: Some([2, 0]),
                brightness_change: Some(0.),
                error_rms: Some(0.),
            })
            .collect();
        assert!(consensus(&patches).is_none());
        let before = texture(1);
        let small = GrayImage::from_pixel(80, 45, Luma([0]));
        assert!(measure(&png(&before), &png(&small)).is_err());
        assert!(measure(b"invalid", &png(&before)).is_err());
        assert!(
            measure(
                &vec![0; reversebolt_core::visual::MAX_IMAGE_BYTES + 1],
                &png(&before)
            )
            .is_err()
        );
    }
}
