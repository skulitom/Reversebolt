//! Bounded choice of numeric findings for prospective, read-only checks.
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
    #[serde(default)]
    pub planned_tracks: Vec<usize>,
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
    let mut remaining_candidates = crate::prediction::MAX_CANDIDATES;
    let planned_tracks = ranked
        .iter()
        .filter_map(|r| {
            if r.interpretations > remaining_candidates {
                return None;
            }
            remaining_candidates -= r.interpretations;
            Some(r.track)
        })
        .take(crate::prediction::MAX_CHECKS)
        .collect::<Vec<_>>();
    (!planned_tracks.is_empty()).then(|| Selection {
        scheduled_fresh_captures: remaining,
        ranked,
        planned_tracks,
        rationale: "Ranked by fewer separate candidate locations, then more readable moments and fewer interpretations; ties use the track ID. Up to eight series share a 200-candidate budget; a series that does not fit is deferred whole, never stripped of rivals. This prioritizes investigation cost, not gameplay importance. Planned tracks may be fewer than ranked tracks; status lists the checks actually started. Predictions freeze before scheduled fresh observations. No edits are automatic.".into(),
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

    #[test]
    fn whole_series_fit_the_shared_candidate_and_check_limits() {
        let tracks: Vec<_> = (0..12).map(|id| track(id, &[(16, Kind::I32)])).collect();
        assert_eq!(
            select(&tracks, 4).unwrap().planned_tracks,
            (0..8).collect::<Vec<_>>()
        );
        let mut large = track(0, &[(16, Kind::I32)]);
        large.matches = vec![large.matches[0].clone(); 199];
        let mut other = track(1, &[(32, Kind::I32), (48, Kind::I32)]);
        other.matches = vec![other.matches[0].clone(); 2];
        // Both have one location: the smaller series ranks first and the whole
        // 199-candidate group is deferred rather than losing competing models.
        let selection = select(&[large, other], 4).unwrap();
        assert_eq!(selection.planned_tracks, vec![1]);
        assert_eq!(selection.ranked.len(), 2);
        assert_eq!(selection.ranked[1].interpretations, 199);
    }
}
