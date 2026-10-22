//! Sparse screen-space patch correspondence, never world motion or object identity.
use image::{
    GrayImage,
    imageops::{self, FilterType},
};
use reversebolt_core::Result;
use serde::Serialize;

pub const METRIC: &str = "rgb601-triangle256x144-grid21-patch12-zmssd-r10-fb1-v1";
pub const LIMITATION: &str = "Sparse integer displacement of matched image patches in a normalized 256x144 image, not physical speed, camera motion, an actor or input causality. Low texture, repeated patterns, large motion, occlusion and appearance changes may be unresolved. Zero displacement is not proof of no movement. Consensus covers only its stated patches; thresholds are heuristics, not calibrated probabilities.";
const SIZE: i32 = 12;
const RADIUS: i32 = 10;
const N: i64 = SIZE as i64 * SIZE as i64;
const DENOM: i64 = N * N;

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
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub metric: &'static str,
    pub source_dimensions: [u32; 2],
    pub measurement_dimensions: [u32; 2],
    pub patch_size: i32,
    pub search_radius: i32,
    pub matched_patches: usize,
    pub zero_displacement_patches: usize,
    pub nonzero_displacement_patches: usize,
    pub patches: Vec<Patch>,
    pub consensus: Option<Consensus>,
    pub limitation: &'static str,
}

fn block(image: &GrayImage, center: [i32; 2]) -> [i64; N as usize] {
    let mut pixels = [0; N as usize];
    let mut index = 0;
    for y in center[1] - SIZE / 2..center[1] + SIZE / 2 {
        for x in center[0] - SIZE / 2..center[0] + SIZE / 2 {
            pixels[index] = i64::from(image.get_pixel(x as u32, y as u32)[0]);
            index += 1;
        }
    }
    pixels
}
fn variance(pixels: &[i64; N as usize]) -> i64 {
    let sum: i64 = pixels.iter().sum();
    N * pixels.iter().map(|p| p * p).sum::<i64>() - sum * sum
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
) -> std::result::Result<Match, Status> {
    let reference = block(source, center);
    if variance(&reference) < 64 * DENOM {
        return Err(Status::LowTexture);
    }
    let mut scores = Vec::with_capacity(((RADIUS * 2 + 1).pow(2)) as usize);
    for dy in -RADIUS..=RADIUS {
        for dx in -RADIUS..=RADIUS {
            let other = block(target, [center[0] + dx, center[1] + dy]);
            let mut sum = 0;
            let mut squared = 0;
            for (a, b) in reference.iter().zip(other) {
                let difference = b - a;
                sum += difference;
                squared += difference * difference;
            }
            scores.push(Match {
                delta: [dx, dy],
                cost: N * squared - sum * sum,
                brightness: sum,
            });
        }
    }
    let best = *scores.iter().min_by_key(|m| m.cost).unwrap();
    if best.delta.iter().any(|d| d.abs() == RADIUS) {
        return Err(Status::SearchBoundary);
    }
    if best.cost > 144 * DENOM {
        return Err(Status::PoorMatch);
    }
    let second = scores
        .iter()
        .filter(|m| {
            (m.delta[0] - best.delta[0]).abs() > 1 || (m.delta[1] - best.delta[1]).abs() > 1
        })
        .map(|m| m.cost)
        .min()
        .unwrap();
    if second * 4 < best.cost * 5 + 36 * DENOM {
        return Err(Status::Ambiguous);
    }
    Ok(best)
}

fn prepared(a: &GrayImage, b: &GrayImage, dimensions: [u32; 2]) -> Report {
    debug_assert_eq!(a.dimensions(), (256, 144));
    debug_assert_eq!(b.dimensions(), (256, 144));
    let mut patches = vec![];
    for y in [32, 64, 96] {
        for x in [32, 64, 96, 128, 160, 192, 224] {
            let anchor = [x, y];
            let measured = search(a, b, anchor).and_then(|forward| {
                // Grid anchors leave room for both complete search windows.
                let backward = search(b, a, [x + forward.delta[0], y + forward.delta[1]])
                    .map_err(|_| Status::InconsistentReverse)?;
                if (0..2).any(|i| (forward.delta[i] + backward.delta[i]).abs() > 1) {
                    return Err(Status::InconsistentReverse);
                }
                Ok(forward)
            });
            patches.push(match measured {
                Ok(m) => Patch {
                    anchor,
                    status: Status::Matched,
                    displacement: Some(m.delta),
                    brightness_change: Some(m.brightness as f64 / N as f64),
                    error_rms: Some((m.cost as f64 / DENOM as f64).sqrt()),
                },
                Err(status) => Patch {
                    anchor,
                    status,
                    displacement: None,
                    brightness_change: None,
                    error_rms: None,
                },
            });
        }
    }
    let matches: Vec<_> = patches.iter().filter_map(|p| p.displacement).collect();
    let best = matches
        .iter()
        .map(|d| {
            (
                *d,
                matches
                    .iter()
                    .filter(|other| (0..2).all(|i| (d[i] - other[i]).abs() <= 1))
                    .count(),
            )
        })
        .max_by_key(|(_, n)| *n);
    let consensus = best
        .filter(|(_, n)| *n >= 10 && *n * 4 >= matches.len() * 3)
        .map(|(d, n)| Consensus {
            displacement: d,
            tolerance_pixels: 1,
            supporting_patches: n,
        });
    let stationary = matches.iter().filter(|d| **d == [0, 0]).count();
    Report {
        metric: METRIC,
        source_dimensions: dimensions,
        measurement_dimensions: [256, 144],
        patch_size: SIZE,
        search_radius: RADIUS,
        matched_patches: matches.len(),
        zero_displacement_patches: stationary,
        nonzero_displacement_patches: matches.len() - stationary,
        patches,
        consensus,
        limitation: LIMITATION,
    }
}

/// Bounded two-image measurement. Callers must separately bind image hashes,
/// input/timing evidence and scene epoch; this function rejects unequal geometry.
pub fn measure(before: &[u8], after: &[u8]) -> Result<Report> {
    let a = crate::response::grayscale(before)?;
    let b = crate::response::grayscale(after)?;
    if a.dimensions() != b.dimensions() {
        return Err("Image geometry changed.".into());
    }
    let dimensions = [a.width(), a.height()];
    let a = imageops::resize(&a, 256, 144, FilterType::Triangle);
    let b = imageops::resize(&b, 256, 144, FilterType::Triangle);
    Ok(prepared(&a, &b, dimensions))
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
    fn recovers_signed_translation_separately_from_uniform_brightness_change() {
        let before = texture(1);
        for delta in [[0, 0], [4, -3], [-6, 2]] {
            let after = shift(&before, delta, 30);
            let measured = measure(&png(&before), &png(&after)).unwrap();
            assert_eq!(measured.matched_patches, 21);
            assert_eq!(measured.consensus.unwrap().displacement, delta);
            for patch in measured.patches {
                assert_eq!(patch.displacement, Some(delta));
                assert_eq!(patch.brightness_change, Some(30.));
                assert_eq!(patch.error_rms, Some(0.));
            }
            let reverse = prepared(&after, &before, [256, 144]);
            assert_eq!(
                reverse.consensus.unwrap().displacement,
                [-delta[0], -delta[1]]
            );
        }
    }
    #[test]
    fn unresolved_texture_repetition_large_motion_and_scene_change_are_not_stationary() {
        let flat = GrayImage::from_pixel(256, 144, Luma([60]));
        let bright = GrayImage::from_pixel(256, 144, Luma([160]));
        let repeated = GrayImage::from_fn(256, 144, |x, y| {
            Luma([if (x / 2 + y / 2) % 2 == 0 { 40 } else { 180 }])
        });
        let before = texture(1);
        for (a, b) in [
            (&flat, &bright),
            (&repeated, &repeated),
            (&before, &texture(9999)),
            (&before, &shift(&before, [10, 0], 0)),
            (&before, &shift(&before, [18, 0], 0)),
        ] {
            let measured = prepared(a, b, [256, 144]);
            assert_eq!(measured.matched_patches, 0);
            assert_eq!(measured.zero_displacement_patches, 0);
            assert!(measured.consensus.is_none());
            assert!(
                measured
                    .patches
                    .iter()
                    .all(|p| p.displacement.is_none() && p.status != Status::Matched)
            );
        }
    }
    #[test]
    fn opposing_local_displacements_do_not_become_one_average_motion() {
        let before = texture(5);
        let left = shift(&before, [4, 0], 0);
        let right = shift(&before, [-4, 0], 0);
        let after = GrayImage::from_fn(256, 144, |x, y| {
            *if x < 128 {
                left.get_pixel(x, y)
            } else {
                right.get_pixel(x, y)
            }
        });
        let measured = prepared(&before, &after, [256, 144]);
        assert_eq!(measured.matched_patches, 18);
        assert_eq!(measured.nonzero_displacement_patches, 18);
        assert!(measured.consensus.is_none());
        assert_eq!(
            measured
                .patches
                .iter()
                .filter(|p| p.displacement == Some([4, 0]))
                .count(),
            9
        );
        assert_eq!(
            measured
                .patches
                .iter()
                .filter(|p| p.displacement == Some([-4, 0]))
                .count(),
            9
        );
    }
    #[test]
    fn occlusion_keeps_missing_patch_evidence_and_geometry_mismatch_is_rejected() {
        let before = texture(7);
        let mut after = before.clone();
        for y in 20..44 {
            for x in 20..44 {
                after.put_pixel(x, y, Luma([0]));
            }
        }
        let measured = measure(&png(&before), &png(&after)).unwrap();
        assert_eq!(measured.matched_patches, 20);
        assert!(measured.patches[0].displacement.is_none());
        assert_eq!(measured.patches[0].anchor, [32, 32]);
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
