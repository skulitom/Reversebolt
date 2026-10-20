//! The discovery algorithm knows bytes and observed numeric tracks only.
//! It has no game names, field offsets, fixture types, or health/ammo rules.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    I32,
    F32,
    F64,
}

impl Kind {
    pub fn width(self) -> usize {
        match self {
            Self::I32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }

    pub fn read(self, bytes: &[u8]) -> Option<f64> {
        let value = match self {
            Self::I32 => i32::from_le_bytes(bytes.get(..4)?.try_into().ok()?) as f64,
            Self::F32 => f32::from_le_bytes(bytes.get(..4)?.try_into().ok()?) as f64,
            Self::F64 => f64::from_le_bytes(bytes.get(..8)?.try_into().ok()?),
        };
        // This is a general numeric prior, not a game-specific range.
        (value.is_finite() && value.abs() <= 1.0e12 && (value == 0.0 || value.abs() >= 1.0e-12))
            .then_some(value)
    }

    pub fn encode(self, value: f64) -> Option<Vec<u8>> {
        if !value.is_finite() {
            return None;
        }
        Some(match self {
            Self::I32 if value >= i32::MIN as f64 && value <= i32::MAX as f64 => {
                (value.round() as i32).to_le_bytes().to_vec()
            }
            Self::I32 => return None,
            Self::F32 if value.abs() <= f32::MAX as f64 => (value as f32).to_le_bytes().to_vec(),
            Self::F32 => return None,
            Self::F64 => value.to_le_bytes().to_vec(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub offset: usize,
    pub kind: Kind,
    pub slope: f64,
    pub intercept: f64,
    pub train_inliers: usize,
    pub validation_inliers: usize,
    pub exact: bool,
}

impl Candidate {
    pub fn encode_observation(&self, observed_value: f64) -> Option<Vec<u8>> {
        if self.slope.abs() < 1.0e-12 {
            return None;
        }
        self.kind
            .encode((observed_value - self.intercept) / self.slope)
    }
}

/// Robustly fits y = a*x+b using pair hypotheses. The last half of the samples
/// are held out: they never participate in fitting or candidate ordering.
fn fit(pairs: &[(f64, f64)], tolerance: f64) -> Option<(f64, f64, usize)> {
    let mut best: Option<(f64, f64, usize, f64)> = None;
    for first in 0..pairs.len() {
        for second in first + 1..pairs.len() {
            let dx = pairs[second].0 - pairs[first].0;
            let dy = pairs[second].1 - pairs[first].1;
            if dx.abs() < 1.0e-9 || dy.abs() < tolerance * 2.0 {
                continue;
            }
            let slope = dy / dx;
            if !slope.is_finite() || slope.abs() < 1.0e-9 || slope.abs() > 1.0e9 {
                continue;
            }
            let intercept = pairs[first].1 - slope * pairs[first].0;
            let mut inliers = 0;
            let mut error = 0.0;
            for &(x, y) in pairs {
                let residual = (slope * x + intercept - y).abs();
                if residual <= tolerance {
                    inliers += 1;
                    error += residual;
                }
            }
            if best.is_none_or(|(_, _, count, residual)| {
                inliers > count || (inliers == count && error < residual)
            }) {
                best = Some((slope, intercept, inliers, error));
            }
        }
    }
    best.map(|(slope, intercept, inliers, _)| (slope, intercept, inliers))
}

/// Natural alignment only in this prototype. Missing observations are skipped.
pub fn discover(snapshots: &[Vec<u8>], observations: &[Option<f64>]) -> Vec<Candidate> {
    if snapshots.len() != observations.len() || snapshots.len() < 12 {
        return Vec::new();
    }
    let split = snapshots.len() / 2;
    let observed: Vec<f64> = observations.iter().flatten().copied().collect();
    if observed.len() < 10 {
        return Vec::new();
    }
    let minimum = observed.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = observed.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let tolerance = 0.55;
    if maximum - minimum < tolerance * 4.0 {
        return Vec::new();
    }
    let length = snapshots.iter().map(Vec::len).min().unwrap_or(0);
    let mut candidates = Vec::new();
    for kind in [Kind::I32, Kind::F32, Kind::F64] {
        let width = kind.width();
        for offset in (0..length.saturating_sub(width - 1)).step_by(width) {
            // Reject unchanged locations cheaply before floating-point fitting.
            let initial = &snapshots[0][offset..offset + width];
            if snapshots[1..split]
                .iter()
                .all(|snapshot| &snapshot[offset..offset + width] == initial)
            {
                continue;
            }
            let pairs: Option<Vec<(f64, f64)>> = snapshots
                .iter()
                .zip(observations)
                .take(split)
                .filter_map(|(snapshot, observation)| {
                    observation.map(|y| kind.read(&snapshot[offset..]).map(|x| (x, y)))
                })
                .collect();
            let Some(pairs) = pairs else { continue };
            if pairs.len() < 5 {
                continue;
            }
            let Some((slope, intercept, train_inliers)) = fit(&pairs, tolerance) else {
                continue;
            };
            if train_inliers * 5 < pairs.len() * 4 {
                continue;
            }
            let mut validation_inliers = 0;
            let mut validation_count = 0;
            let mut exact_matches = pairs
                .iter()
                .filter(|(x, y)| (x - y).abs() <= tolerance)
                .count();
            for (snapshot, observation) in snapshots.iter().zip(observations).skip(split) {
                if let Some(y) = observation {
                    validation_count += 1;
                    if let Some(x) = kind.read(&snapshot[offset..]) {
                        validation_inliers +=
                            usize::from((slope * x + intercept - y).abs() <= tolerance);
                        exact_matches += usize::from((x - y).abs() <= tolerance);
                    }
                }
            }
            if validation_count < 5 || validation_inliers * 5 < validation_count * 4 {
                continue;
            }
            candidates.push(Candidate {
                offset,
                kind,
                slope,
                intercept,
                train_inliers,
                validation_inliers,
                exact: exact_matches * 5 >= (pairs.len() + validation_count) * 4,
            });
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_inverted_scale_with_outlier_and_holds_out_later_frames() {
        let values = [
            84., 71., 95., 62., 83., 44., 58., 93., 35., 61., 42., 87., 30., 55., 78., 63.,
        ];
        let mut observations: Vec<_> = values.iter().copied().map(Some).collect();
        observations[3] = Some(999.);
        let snapshots: Vec<Vec<u8>> = values
            .iter()
            .map(|value| (4000 - (*value as i32) * 7).to_le_bytes().to_vec())
            .collect();
        let candidates = discover(&snapshots, &observations);
        assert_eq!(candidates.len(), 1);
        assert!((candidates[0].slope + 1. / 7.).abs() < 1e-8);
        let mut broken = snapshots;
        for snapshot in &mut broken[8..] {
            *snapshot = 11111_i32.to_le_bytes().to_vec();
        }
        assert!(discover(&broken, &observations).is_empty());
    }

    #[test]
    fn identical_observations_cannot_distinguish_authority_from_a_copy() {
        let values = [
            84_i32, 71, 95, 62, 83, 44, 58, 93, 35, 61, 42, 87, 30, 55, 78, 63,
        ];
        let snapshots = values
            .iter()
            .map(|value| value.to_le_bytes().repeat(2))
            .collect::<Vec<_>>();
        let observations = values
            .iter()
            .map(|value| Some(*value as f64))
            .collect::<Vec<_>>();
        let candidates = discover(&snapshots, &observations);
        assert_eq!(
            candidates
                .iter()
                .filter(|candidate| candidate.kind == Kind::I32)
                .count(),
            2
        );
    }
}
