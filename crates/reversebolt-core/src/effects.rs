//! Descriptive comparisons across a trial's declared screen-number regions.
//! Coinciding changes are not causal attribution or a closed effect scope.
use serde::{Deserialize, Serialize};

pub const MAX_WATCHES: usize = 32;
pub const LIMITATION: &str = "Only these sampled screen regions were compared. Changes can come from ordinary gameplay, timing, OCR or the edit. Unseen, delayed and nonnumeric effects remain unverified; restored bytes do not establish recovered gameplay state.";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Watch {
    pub track: usize,
    pub label: String,
    pub bounds: [f32; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reading {
    pub track: usize,
    pub value: Option<f64>,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Changed,
    Unchanged,
    Unreadable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Difference {
    pub track: usize,
    pub label: String,
    pub before: Option<f64>,
    pub after: Option<f64>,
    pub status: Status,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comparison {
    pub other_values: Vec<Difference>,
    pub omitted_tracks: usize,
    pub limitation: String,
}

pub fn compare(
    watches: &[Watch],
    selected: usize,
    omitted_tracks: usize,
    before: &[Reading],
    after: &[Reading],
) -> Comparison {
    fn value(readings: &[Reading], track: usize) -> Option<f64> {
        let mut matches = readings.iter().filter(|r| r.track == track);
        let value = matches.next()?.value.filter(|v| v.is_finite());
        if matches.next().is_some() {
            None
        } else {
            value
        }
    }
    Comparison {
        other_values: watches
            .iter()
            .filter(|w| w.track != selected)
            .map(|w| {
                let before = value(before, w.track);
                let after = value(after, w.track);
                Difference {
                    track: w.track,
                    label: w.label.clone(),
                    before,
                    after,
                    status: match (before, after) {
                        (Some(a), Some(b)) if (a - b).abs() > crate::prediction::TOLERANCE => {
                            Status::Changed
                        }
                        (Some(_), Some(_)) => Status::Unchanged,
                        _ => Status::Unreadable,
                    },
                }
            })
            .collect(),
        omitted_tracks,
        limitation: LIMITATION.into(),
    }
}
