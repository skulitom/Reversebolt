//! Bounded choice of a numeric finding for a prospective, read-only check.
//! Ranking measures investigation cost, not gameplay importance or authority.
use crate::live::NumberTrack;
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankedTrack {
    pub track: usize,
    pub locations: usize,
    pub interpretations: usize,
    pub readable_moments: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Selection {
    pub scheduled_fresh_captures: usize,
    pub ranked: Vec<RankedTrack>,
    pub rationale: String,
}

pub fn select(tracks: &[NumberTrack], remaining: usize) -> Option<Selection> {
    if remaining < crate::prediction::MIN_FRESH {
        return None;
    }
    let mut ranked = vec![];
    for track in tracks {
        if !track.active
            || track.matches.is_empty()
            || track.matches.len() > 200
            || !track
                .samples
                .last()
                .copied()
                .flatten()
                .is_some_and(f64::is_finite)
        {
            continue;
        }
        let readable_moments = track
            .samples
            .iter()
            .flatten()
            .filter(|v| v.is_finite())
            .count();
        if readable_moments < 10 {
            continue;
        }
        let ranges = track
            .matches
            .iter()
            .map(|c| {
                let start = u64::from_str_radix(c.address.trim_start_matches("0x"), 16).ok()?;
                Some((start, start.checked_add(c.mapping.kind.width() as u64)?))
            })
            .collect::<Option<BTreeSet<_>>>();
        let Some(ranges) = ranges else { continue };
        let mut end = 0;
        let mut locations = 0;
        for (start, last) in ranges {
            if locations == 0 || start >= end {
                locations += 1;
            }
            end = end.max(last);
        }
        ranked.push(RankedTrack {
            track: track.id,
            locations,
            interpretations: track.matches.len(),
            readable_moments,
        });
    }
    ranked.sort_by_key(|r| {
        (
            r.locations,
            Reverse(r.readable_moments),
            r.interpretations,
            r.track,
        )
    });
    (!ranked.is_empty()).then(|| Selection {
        scheduled_fresh_captures: remaining,
        ranked,
        rationale: "Chosen for fewer separate candidate locations, then more readable moments and fewer interpretations; ties use the track ID. This prioritizes a bounded investigation, not gameplay importance. Predictions were frozen before the scheduled fresh observations. No edits are automatic.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        discovery::{Candidate, Kind},
        live::LocatedCandidate,
    };
    fn track(id: usize, locations: &[(u64, Kind)]) -> NumberTrack {
        NumberTrack {
            id,
            label: "untrusted display label".into(),
            bounds: [0.; 4],
            samples: vec![Some(40.); 12],
            readings: vec![],
            active: true,
            matches: locations
                .iter()
                .map(|(address, kind)| LocatedCandidate {
                    address: format!("0x{address:X}"),
                    evidence: String::new(),
                    mapping: Candidate {
                        offset: 0,
                        kind: *kind,
                        slope: 1.,
                        intercept: 0.,
                        train_inliers: 6,
                        validation_inliers: 6,
                        exact: true,
                    },
                })
                .collect(),
        }
    }
    #[test]
    fn selection_uses_observed_coverage_and_overlap_cost_not_names_or_input_order() {
        let mut copies = track(0, &[(16, Kind::I32), (32, Kind::I32)]);
        copies.label = "HEALTH important control".into();
        let alias = track(9, &[(16, Kind::I32), (20, Kind::I32), (16, Kind::F64)]);
        let mut sparse = track(2, &[(16, Kind::I32)]);
        sparse.samples[0] = None;
        let mut missing = track(1, &[(16, Kind::I32)]);
        missing.samples[11] = None;
        let mut invalid = track(3, &[(16, Kind::I32)]);
        invalid.matches[0].address = "broken".into();
        let mut inactive = track(4, &[(16, Kind::I32)]);
        inactive.active = false;
        let tracks = vec![copies, missing, sparse, invalid, alias, inactive];
        let order = |t: &[NumberTrack]| {
            select(t, 4)
                .unwrap()
                .ranked
                .iter()
                .map(|r| r.track)
                .collect::<Vec<_>>()
        };
        assert_eq!(order(&tracks), vec![9, 2, 0]);
        assert_eq!(
            order(&tracks.iter().cloned().rev().collect::<Vec<_>>()),
            vec![9, 2, 0]
        );
        assert!(select(&tracks, 3).is_none());
        assert!(select(&[], 4).is_none());
    }
}
