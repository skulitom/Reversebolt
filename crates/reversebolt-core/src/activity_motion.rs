//! Bounded, source-bound correspondence retained with an image interpretation.
use crate::{Result, play_context::Source};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const METRIC_V2: &str = "rgb601-triangle256x144-corners48-pyramid2-patch9-zmssd-r16x3-fb1-v2";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Match {
    pub at: [i32; 2],
    pub delta: [i32; 2],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consensus {
    pub delta: [i32; 2],
    pub supporting_patches: usize,
    pub supporting_cells: usize,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub selected: usize,
    pub matched_cells: usize,
    pub unresolved: BTreeMap<String, usize>,
    pub matches: Vec<Match>,
    pub consensus: Option<Consensus>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pair {
    pub before_id: String,
    pub before_sha256: String,
    pub after_id: String,
    pub after_sha256: String,
    pub measurement: Option<Measurement>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub metric: String,
    pub frame_metadata_sha256: String,
    pub measurement_dimensions: [u32; 2],
    pub maximum_selected: usize,
    pub grid: [usize; 2],
    pub pairs: [Pair; 2],
}
fn cell(p: &Match) -> [i32; 2] {
    [p.at[0] * 6 / 256, p.at[1] * 4 / 144]
}
impl Summary {
    /// Structural/provenance validation only; it cannot establish match accuracy.
    pub fn validate(&self, source: &Source) -> Result<()> {
        source.validate()?;
        if self.metric != METRIC_V2
            || self.frame_metadata_sha256 != source.frame_metadata_sha256
            || self.measurement_dimensions != [256, 144]
            || self.maximum_selected != 48
            || self.grid != [6, 4]
        {
            return Err("Motion summary version, dimensions or source changed.".into());
        }
        for (pair, reference) in self.pairs.iter().zip(&source.pairs) {
            if pair.before_id != reference.before.id
                || pair.before_sha256 != reference.before.sha256
                || pair.after_id != reference.after.id
                || pair.after_sha256 != reference.after.sha256
            {
                return Err("Motion summary images differ from the activity source.".into());
            }
            let Some(m) = &pair.measurement else {
                if !pair.error.as_ref().is_some_and(|e| {
                    !e.trim().is_empty() && e.len() <= 400 && !e.chars().any(char::is_control)
                }) {
                    return Err("Missing motion measurement needs a bounded error.".into());
                }
                continue;
            };
            if pair.error.is_some()
                || m.selected > 48
                || m.matches.len() > m.selected
                || m.unresolved.len() > 5
            {
                return Err("Invalid motion coverage.".into());
            }
            let mut missing = 0usize;
            for (status, count) in &m.unresolved {
                if ![
                    "low_texture",
                    "ambiguous",
                    "poor_match",
                    "search_boundary",
                    "inconsistent_reverse",
                ]
                .contains(&status.as_str())
                    || *count == 0
                    || *count > 48
                {
                    return Err("Invalid unresolved motion count.".into());
                }
                missing += count;
            }
            if missing + m.matches.len() != m.selected {
                return Err("Motion coverage does not add up.".into());
            }
            let mut cells = BTreeMap::new();
            for (i, p) in m.matches.iter().enumerate() {
                if !(12..244).contains(&p.at[0])
                    || !(12..132).contains(&p.at[1])
                    || p.at.iter().any(|n| n % 2 != 0)
                    || p.delta.iter().any(|n| !(-35..=35).contains(n))
                    || m.matches[..i]
                        .iter()
                        .any(|q| (p.at[0] - q.at[0]).pow(2) + (p.at[1] - q.at[1]).pow(2) < 144)
                {
                    return Err("Invalid motion anchor, displacement or spacing.".into());
                }
                *cells.entry(cell(p)).or_insert(0) += 1;
            }
            if cells.len() != m.matched_cells || cells.values().any(|n| *n > 2) {
                return Err("Invalid motion spatial coverage.".into());
            }
            if let Some(c) = &m.consensus {
                if c.delta.iter().any(|n| !(-35..=35).contains(n)) {
                    return Err("Invalid motion consensus displacement.".into());
                }
                let supporters: Vec<_> = m
                    .matches
                    .iter()
                    .filter(|p| (0..2).all(|i| (p.delta[i] - c.delta[i]).abs() <= 1))
                    .collect();
                let cells: BTreeSet<_> = supporters.iter().map(|p| cell(p)).collect();
                if supporters.len() != c.supporting_patches
                    || supporters.len() < 10
                    || supporters.len() * 4 < m.matches.len() * 3
                    || cells.len() != c.supporting_cells
                    || cells.len() < 6
                    || cells.iter().map(|p| p[0]).collect::<BTreeSet<_>>().len() < 2
                    || cells.iter().map(|p| p[1]).collect::<BTreeSet<_>>().len() < 2
                {
                    return Err("Motion consensus lacks distributed support.".into());
                }
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 10 * 1024 {
            return Err("Motion summary exceeds its budget.".into());
        }
        Ok(())
    }
}
