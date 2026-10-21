//! Prospective checks of frozen numeric mappings. These cannot grant write authority.
use crate::{discovery::Candidate, live::MemoryRead, visual::FrameRef};
use serde::{Deserialize, Serialize};

pub const TOLERANCE: f64 = 0.55;
pub const MIN_FRESH: usize = 4;
pub const MAX_CHECKS: usize = 8;
pub const MAX_CANDIDATES: usize = 200;

/// Historical sessions stored one optional check. New sessions publish a list.
pub fn deserialize_checks<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Vec<PredictionCheck>, D::Error> {
    // Deserialize through JSON rather than Serde's untagged content buffer:
    // check witnesses include u128 timestamps, which that buffer cannot restore.
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Null => Ok(vec![]),
        serde_json::Value::Array(checks) => checks
            .into_iter()
            .map(serde_json::from_value)
            .collect::<std::result::Result<_, _>>()
            .map_err(serde::de::Error::custom),
        check => serde_json::from_value(check)
            .map(|check| vec![check])
            .map_err(serde::de::Error::custom),
    }
}

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
    TimingUncertain,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Waiting for a reading",
            Self::Missing => "Memory reading unavailable",
            Self::Agrees => "Agrees",
            Self::Contradicts => "Contradicts",
            Self::TimingUncertain => "Capture timing uncertain",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Witness {
    pub bytes: Option<Vec<u8>>,
    pub read: MemoryRead,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub frame: usize,
    pub image: Option<FrameRef>,
    /// A bounded read of this candidate before requesting the image. Historical
    /// samples without this witness cannot establish a timed comparison.
    #[serde(default)]
    pub before_image: Option<Witness>,
    pub bytes: Option<Vec<u8>>,
    pub read: Option<MemoryRead>,
    pub predicted: Option<f64>,
    pub observed: Option<f64>,
    pub outcome: Outcome,
}

impl Sample {
    /// Equal endpoints bracket the image request, not every intervening state.
    /// They cannot exclude an ABA change or a delayed/compositor display copy.
    pub fn supports_comparison(&self) -> bool {
        let (Some(before), Some(after), Some(read), Some(image)) =
            (&self.before_image, &self.bytes, &self.read, &self.image)
        else {
            return false;
        };
        let (Some(bytes), Some(first), Some(last), Some(capture)) = (
            &before.bytes,
            &before.read.capture_interval,
            &read.capture_interval,
            &image.capture_interval,
        ) else {
            return false;
        };
        image.number == self.frame
            && !bytes.is_empty()
            && bytes.len() <= 8
            && bytes == after
            && before.read.requested == bytes.len()
            && before.read.retained == bytes.len()
            && read.retained >= after.len()
            && before.read.disposition == "retained_generation_unverified"
            && read.disposition == "retained_generation_unverified"
            && first.clock_id == capture.clock_id
            && last.clock_id == capture.clock_id
            && first.begin_us <= first.end_us
            && first.end_us <= capture.begin_us
            && capture.begin_us <= capture.end_us
            && capture.end_us <= last.begin_us
            && last.begin_us <= last.end_us
    }
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
    #[serde(default)]
    pub automatic_selection: Option<crate::investigation::Selection>,
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
            .any(|s| s.supports_comparison() && s.outcome == Outcome::Contradicts)
        {
            return Assessment::Contradicted;
        }
        let usable = self
            .samples
            .iter()
            .filter(|s| s.supports_comparison() && s.outcome == Outcome::Agrees)
            .count();
        let mut hold = false;
        let mut change = false;
        for pair in self.samples.windows(2) {
            if pair
                .iter()
                .all(|s| s.supports_comparison() && s.outcome == Outcome::Agrees)
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
                    (None, _) => Outcome::Missing,
                    _ if !sample.supports_comparison() => Outcome::TimingUncertain,
                    (Some(predicted), Some(observed)) => {
                        if (predicted - observed).abs() <= self.tolerance {
                            Outcome::Agrees
                        } else {
                            Outcome::Contradicts
                        }
                    }
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
    use crate::visual::CaptureInterval;

    fn read_at(begin_us: u64, end_us: u64) -> MemoryRead {
        MemoryRead {
            address: "0x10".into(),
            requested: 4,
            retained: 4,
            begin_ms: 0,
            end_ms: 0,
            disposition: "retained_generation_unverified".into(),
            capture_interval: Some(CaptureInterval {
                clock_id: "session".into(),
                begin_us,
                end_us,
            }),
        }
    }

    fn check(predictions: &[f64]) -> PredictionCheck {
        PredictionCheck {
            automatic_selection: None,
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
                            input_context: None,
                            id: format!("frame-{i}"),
                            number: 13 + i,
                            sha256: "fixture".into(),
                            width: 10,
                            height: 10,
                            epoch: 0,
                            started_unix_ms: i as u128,
                            capture_interval: Some(CaptureInterval {
                                clock_id: "session".into(),
                                begin_us: 3,
                                end_us: 6,
                            }),
                        }),
                        before_image: Some(Witness {
                            bytes: Some((*p as i32).to_le_bytes().to_vec()),
                            read: read_at(1, 2),
                        }),
                        bytes: Some((*p as i32).to_le_bytes().to_vec()),
                        read: Some(read_at(7, 8)),
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

    #[test]
    fn changing_bytes_or_unbracketed_images_never_confirm_or_refute_a_mapping() {
        for fault in [
            "changed",
            "missing",
            "before_overlap",
            "after_overlap",
            "clock",
            "image",
            "short_read",
        ] {
            let mut c = check(&[1., 1., 3., 3.]);
            let sample = &mut c.candidates[0].samples[1];
            match fault {
                "changed" => {
                    sample.before_image.as_mut().unwrap().bytes = Some(9_i32.to_le_bytes().to_vec())
                }
                "missing" => sample.before_image = None,
                "before_overlap" => {
                    sample
                        .before_image
                        .as_mut()
                        .unwrap()
                        .read
                        .capture_interval
                        .as_mut()
                        .unwrap()
                        .end_us = 4
                }
                "after_overlap" => {
                    sample
                        .read
                        .as_mut()
                        .unwrap()
                        .capture_interval
                        .as_mut()
                        .unwrap()
                        .begin_us = 5
                }
                "clock" => {
                    sample
                        .read
                        .as_mut()
                        .unwrap()
                        .capture_interval
                        .as_mut()
                        .unwrap()
                        .clock_id = "other".into()
                }
                "image" => sample.image.as_mut().unwrap().capture_interval = None,
                "short_read" => sample.before_image.as_mut().unwrap().read.retained = 2,
                _ => unreachable!(),
            }
            for observed in [1., 99.] {
                c.observe(&observations(&[
                    Some(1.),
                    Some(observed),
                    Some(3.),
                    Some(3.),
                ]));
                assert_eq!(c.candidates[0].samples[1].outcome, Outcome::TimingUncertain);
                assert_eq!(
                    c.candidates[0].assessment(&c),
                    Assessment::NeedsContrastingEvidence
                );
            }
        }
    }

    #[test]
    fn legacy_agreements_without_timing_witnesses_do_not_authorize_a_trial() {
        let mut c = check(&[1., 1., 3., 3.]);
        c.observe(&observations(&[Some(1.), Some(1.), Some(3.), Some(3.)]));
        let mut json = serde_json::to_value(&c).unwrap();
        for s in json["candidates"][0]["samples"].as_array_mut().unwrap() {
            s.as_object_mut().unwrap().remove("before_image");
        }
        let historical: PredictionCheck = serde_json::from_value(json).unwrap();
        assert_eq!(
            historical.candidates[0].assessment(&historical),
            Assessment::NeedsContrastingEvidence
        );
    }
}
