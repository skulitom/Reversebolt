//! Prospective checks of frozen numeric mappings. These cannot grant write authority.
use crate::{discovery::Candidate, live::MemoryRead, visual::FrameRef};
use serde::{Deserialize, Serialize};

pub const TOLERANCE: f64 = 0.55;
pub const MIN_FRESH: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Ocr,
    External,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pending,
    Missing,
    Agrees,
    Contradicts,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub frame: usize,
    pub image: Option<FrameRef>,
    pub bytes: Option<Vec<u8>>,
    pub read: Option<MemoryRead>,
    pub predicted: Option<f64>,
    pub observed: Option<f64>,
    pub outcome: Outcome,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrozenCandidate {
    pub address: String,
    pub chunk_address: u64,
    pub offset_in_chunk: usize,
    pub mapping: Candidate,
    pub invalidated: Option<String>,
    pub samples: Vec<Sample>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PredictionCheck {
    pub id: String,
    pub investigation_id: String,
    pub batch: usize,
    pub epoch: usize,
    pub source: Source,
    pub track: usize,
    pub label: String,
    pub after_frame: usize,
    pub last_frame: usize,
    pub tolerance: f64,
    pub basis: Vec<FrameRef>,
    pub invalidated: Option<String>,
    pub candidates: Vec<FrozenCandidate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Assessment {
    Invalidated,
    Contradicted,
    ConsistentInObservedScope,
    NeedsContrastingEvidence,
}

impl Assessment {
    pub fn label(self) -> &'static str {
        match self {
            Self::Invalidated => "Scope invalidated",
            Self::Contradicted => "Contradicted by observations",
            Self::ConsistentInObservedScope => "Consistent in observed scope",
            Self::NeedsContrastingEvidence => "Needs contrasting evidence",
        }
    }
}

impl FrozenCandidate {
    /// Only fresh, jointly observed frames count. A hold and a change are a
    /// minimal contrast, not a statistical confidence or semantic/causal test.
    pub fn assessment(&self, check: &PredictionCheck) -> Assessment {
        if check.invalidated.is_some() || self.invalidated.is_some() {
            return Assessment::Invalidated;
        }
        if self
            .samples
            .iter()
            .any(|s| s.outcome == Outcome::Contradicts)
        {
            return Assessment::Contradicted;
        }
        let usable = self
            .samples
            .iter()
            .filter(|s| s.outcome == Outcome::Agrees)
            .count();
        let mut hold = false;
        let mut change = false;
        for pair in self.samples.windows(2) {
            if pair.iter().all(|s| s.outcome == Outcome::Agrees)
                && pair[1].frame == pair[0].frame + 1
                && let (Some(a), Some(b)) = (pair[0].observed, pair[1].observed)
            {
                hold |= (a - b).abs() <= check.tolerance;
                change |= (a - b).abs() > check.tolerance * 2.;
            }
        }
        if usable >= MIN_FRESH && hold && change {
            Assessment::ConsistentInObservedScope
        } else {
            Assessment::NeedsContrastingEvidence
        }
    }
}

impl PredictionCheck {
    /// External observations may arrive after capture. Predictions and bytes
    /// stay fixed; only the corresponding previously missing observation joins.
    pub fn observe(&mut self, values: &[Option<f64>]) {
        for candidate in &mut self.candidates {
            for sample in &mut candidate.samples {
                sample.observed = sample.image.as_ref().and_then(|_| {
                    values
                        .get(sample.frame - 1)
                        .copied()
                        .flatten()
                        .filter(|v| v.is_finite())
                });
                sample.outcome = match (sample.predicted, sample.observed) {
                    (Some(predicted), Some(observed)) => {
                        if (predicted - observed).abs() <= self.tolerance {
                            Outcome::Agrees
                        } else {
                            Outcome::Contradicts
                        }
                    }
                    (None, _) => Outcome::Missing,
                    (_, None) => Outcome::Pending,
                };
            }
        }
    }

    pub fn summary(&self) -> String {
        let count = |assessment| {
            self.candidates
                .iter()
                .filter(|c| c.assessment(self) == assessment)
                .count()
        };
        format!(
            "Frozen predictions: {} contradicted, {} consistent in observed scope, {} need evidence, {} invalidated. No gameplay control is verified.",
            count(Assessment::Contradicted),
            count(Assessment::ConsistentInObservedScope),
            count(Assessment::NeedsContrastingEvidence),
            count(Assessment::Invalidated)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::Kind;

    fn check(predictions: &[f64]) -> PredictionCheck {
        PredictionCheck {
            id: "check".into(),
            investigation_id: "session".into(),
            batch: 1,
            epoch: 0,
            source: Source::Ocr,
            track: 0,
            label: "anonymous".into(),
            after_frame: 12,
            last_frame: 20,
            tolerance: TOLERANCE,
            basis: vec![],
            invalidated: None,
            candidates: vec![FrozenCandidate {
                address: "0x10".into(),
                chunk_address: 0,
                offset_in_chunk: 16,
                mapping: Candidate {
                    offset: 16,
                    kind: Kind::I32,
                    slope: 1.,
                    intercept: 0.,
                    train_inliers: 6,
                    validation_inliers: 6,
                    exact: true,
                },
                invalidated: None,
                samples: predictions
                    .iter()
                    .enumerate()
                    .map(|(i, p)| Sample {
                        frame: 13 + i,
                        image: Some(FrameRef {
                            id: format!("frame-{i}"),
                            number: 13 + i,
                            sha256: "fixture".into(),
                            width: 10,
                            height: 10,
                            epoch: 0,
                            started_unix_ms: i as u128,
                            capture_interval: None,
                        }),
                        bytes: None,
                        read: None,
                        predicted: Some(*p),
                        observed: None,
                        outcome: Outcome::Pending,
                    })
                    .collect(),
            }],
        }
    }
    fn observations(values: &[Option<f64>]) -> Vec<Option<f64>> {
        [vec![Some(999.); 12], values.to_vec()].concat()
    }

    #[test]
    fn hold_breaks_time_confound_without_refitting_or_promoting_a_control() {
        let mut check = check(&[13., 14., 15., 16.]);
        let coefficients = serde_json::to_string(&check.candidates[0].mapping).unwrap();
        check.observe(&observations(&[Some(13.), Some(13.), Some(15.), Some(15.)]));
        assert_eq!(
            check.candidates[0].assessment(&check),
            Assessment::Contradicted
        );
        assert_eq!(
            serde_json::to_string(&check.candidates[0].mapping).unwrap(),
            coefficients
        );
        assert_eq!(check.candidates[0].samples[1].predicted, Some(14.));
        assert!(check.summary().contains("No gameplay control is verified"));
    }

    #[test]
    fn stable_only_changing_only_missing_or_invalid_scopes_do_not_pass() {
        for values in [[1., 1., 1., 1.], [1., 2., 3., 4.]] {
            let mut c = check(&values);
            c.observe(&observations(&values.map(Some)));
            assert_eq!(
                c.candidates[0].assessment(&c),
                Assessment::NeedsContrastingEvidence
            );
        }
        let mut c = check(&[1., 1., 3., 3.]);
        c.observe(&observations(&[Some(1.), None, Some(3.), Some(3.)]));
        assert_eq!(
            c.candidates[0].assessment(&c),
            Assessment::NeedsContrastingEvidence
        );
        c.observe(&observations(&[Some(1.), Some(1.), Some(3.), Some(3.)]));
        assert_eq!(
            c.candidates[0].assessment(&c),
            Assessment::ConsistentInObservedScope
        );
        c.invalidated = Some("target exited".into());
        assert_eq!(c.candidates[0].assessment(&c), Assessment::Invalidated);
    }
}
