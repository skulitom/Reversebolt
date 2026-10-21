//! Explicit tests of one location, retaining alternatives rather than claiming
//! consensus across different addresses. This is not causal identification.
use crate::{
    Result,
    changes::BytePatch,
    discovery::Candidate,
    prediction::{Assessment, PredictionCheck},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Choice {
    pub indices: Vec<usize>,
    pub eligible: bool,
}

/// Connected overlapping byte ranges must be considered together, including
/// unresolved rivals. Only a supported contradiction can remove an alternative.
pub fn choices(check: &PredictionCheck) -> Result<Vec<Choice>> {
    let mut ranges = vec![];
    for (index, candidate) in check.candidates.iter().enumerate() {
        if candidate.assessment(check) == Assessment::Contradicted {
            continue;
        }
        let start = u64::from_str_radix(candidate.address.trim_start_matches("0x"), 16)
            .map_err(|_| "Invalid candidate address.")?;
        let end = start
            .checked_add(candidate.mapping.kind.width() as u64)
            .ok_or("Invalid candidate range.")?;
        ranges.push((start, end, index));
    }
    ranges.sort_unstable();
    let mut groups: Vec<(u64, Vec<usize>)> = vec![];
    for (start, end, index) in ranges {
        if let Some((last, indices)) = groups.last_mut()
            && start < *last
        {
            *last = (*last).max(end);
            indices.push(index);
        } else {
            groups.push((end, vec![index]));
        }
    }
    let mut choices: Vec<_> = groups
        .into_iter()
        .map(|(_, mut indices)| {
            indices.sort_unstable();
            let eligible = check.invalidated.is_none()
                && indices.iter().all(|i| {
                    check.candidates[*i].assessment(check) == Assessment::ConsistentInObservedScope
                });
            Choice { indices, eligible }
        })
        .collect();
    choices.sort_by_key(|c| c.indices[0]);
    Ok(choices)
}

pub fn select(check: &PredictionCheck, selected: usize) -> Result<Vec<usize>> {
    let choice = choices(check)?
        .into_iter()
        .find(|c| c.indices.contains(&selected))
        .ok_or("That possibility is absent or has been contradicted.")?;
    if !choice.eligible {
        return Err(
            "This location has overlapping interpretations that still need fresh evidence.".into(),
        );
    }
    Ok(choice.indices)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alternative {
    pub index: usize,
    pub address: u64,
    pub mapping: Candidate,
    pub assessment: Assessment,
    pub in_test_group: bool,
    pub before: Option<Vec<u8>>,
    /// Conditional on this byte mapping and the other bytes remaining unchanged.
    pub predicted_if_written: Option<f64>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Probe {
    pub selected: usize,
    pub group: Vec<usize>,
    pub alternatives: Vec<Alternative>,
    pub limitation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    Agreed,
    Differed,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comparison {
    pub index: usize,
    pub expected: Option<f64>,
    pub observed: Option<f64>,
    pub fit: Fit,
}

impl Probe {
    /// Descriptive comparison only. An intervention can propagate to other
    /// locations, and neither a response nor no response proves authority.
    pub fn compare(&self, observed: Option<f64>) -> Vec<Comparison> {
        self.alternatives
            .iter()
            .map(|a| {
                let fit = match (a.predicted_if_written, observed) {
                    (Some(expected), Some(value)) if (expected - value).abs() <= 0.55 => {
                        Fit::Agreed
                    }
                    (Some(_), Some(_)) => Fit::Differed,
                    _ => Fit::Unavailable,
                };
                Comparison {
                    index: a.index,
                    expected: a.predicted_if_written,
                    observed,
                    fit,
                }
            })
            .collect()
    }
}

pub fn prediction(
    mapping: &Candidate,
    address: u64,
    before: &[u8],
    patch: &BytePatch,
) -> Option<f64> {
    let mut after = before.to_vec();
    let patch_end = patch.address.checked_add(patch.after.len() as u64)?;
    for (offset, byte) in after.iter_mut().enumerate() {
        let at = address.checked_add(offset as u64)?;
        if patch.address <= at && at < patch_end {
            *byte = patch.after[(at - patch.address) as usize];
        }
    }
    mapping
        .kind
        .read(&after)
        .map(|v| v * mapping.slope + mapping.intercept)
        .filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        discovery::Kind,
        prediction::{FrozenCandidate, Source},
    };

    #[test]
    fn transitive_overlaps_stay_together_but_adjacent_ranges_do_not() {
        let check = PredictionCheck {
            id: "test".into(),
            investigation_id: "test".into(),
            batch: 1,
            epoch: 0,
            source: Source::Ocr,
            track: 0,
            label: "number".into(),
            after_frame: 12,
            last_frame: 20,
            tolerance: 0.55,
            basis: vec![],
            invalidated: None,
            candidates: [
                (4, Kind::I32),
                (8, Kind::I32),
                (0, Kind::I32),
                (0, Kind::F64),
            ]
            .into_iter()
            .map(|(offset, kind)| FrozenCandidate {
                address: format!("0x{:X}", 65536 + offset),
                chunk_address: 65536,
                offset_in_chunk: offset,
                mapping: Candidate {
                    offset,
                    kind,
                    slope: 1.,
                    intercept: 0.,
                    train_inliers: 6,
                    validation_inliers: 6,
                    exact: true,
                },
                invalidated: None,
                samples: vec![],
            })
            .collect(),
        };
        let groups = choices(&check).unwrap();
        assert_eq!(
            groups.iter().map(|c| c.indices.clone()).collect::<Vec<_>>(),
            vec![vec![0, 2, 3], vec![1]]
        );
        assert!(!groups[0].eligible);
        assert!(select(&check, 0).is_err());
    }
}
