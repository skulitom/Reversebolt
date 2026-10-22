//! Compact appearance measurements for the exact two interpreted action pairs.
use crate::{Result, play_context::Source, visual_regions::Region};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const POLICY: &str = "selected_action_appearance_v1";
pub const SEQUENCE_POLICY: &str = "immutable_appearance_sequence_v1";
pub const ACTIVITY_SEQUENCE_POLICY: &str = "activity_bounded_appearance_sequence_v1";
pub const MATCHER_POLICY: &str = "rgb601-triangle512x288-region-ncc16-dense-reverse-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    NotMeasured,
    Matched,
    UnsupportedSize,
    LowTexture,
    PoorMatch,
    Ambiguous,
    ImageBoundary,
    InconsistentReverse,
    SourceError,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub status: Status,
    /// Difference between accepted positions in this pair, in source-image pixels.
    pub displacement: Option<[f64; 2]>,
    /// [x, y, width, height] in the selected pair's original image pixels.
    pub before_region: Option<[f64; 4]>,
    pub after_region: Option<[f64; 4]>,
}
impl Outcome {
    pub(crate) fn validate(&self, dimensions: [u32; 2]) -> Result<()> {
        if (self.status == Status::Matched) != self.displacement.is_some()
            || (self.status == Status::Matched) != self.before_region.is_some()
            || (self.status == Status::Matched) != self.after_region.is_some()
        {
            return Err("Unresolved or future appearances cannot supply displacement.".into());
        }
        if let Some(delta) = self.displacement
            && (0..2).any(|i| !delta[i].is_finite() || delta[i].abs() > f64::from(dimensions[i]))
        {
            return Err("Invalid appearance displacement.".into());
        }
        for region in [self.before_region, self.after_region]
            .into_iter()
            .flatten()
        {
            if region.iter().any(|v| !v.is_finite())
                || region[2] <= 0.
                || region[3] <= 0.
                || region[0] < -1e-6
                || region[1] < -1e-6
                || region[0] + region[2] > f64::from(dimensions[0]) + 1e-6
                || region[1] + region[3] > f64::from(dimensions[1]) + 1e-6
            {
                return Err("Appearance rectangle lies outside its image.".into());
            }
        }
        if let (Some(before), Some(after), Some(delta)) =
            (self.before_region, self.after_region, self.displacement)
            && ((0..2).any(|i| (after[i] - before[i] - delta[i]).abs() > 1e-6)
                || before[2..] != after[2..])
        {
            return Err("Appearance rectangles disagree with their displacement.".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub frame_id: String,
    pub image_sha256: String,
    pub number: usize,
    pub dimensions: [u32; 2],
    pub region: Region,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Appearance {
    pub origin: Origin,
    /// Full observation/region provenance, retained in the original report.
    pub source_sha256: String,
    /// Same order as Source.pairs, with explicit missing evidence.
    pub pairs: [Outcome; 2],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub policy: String,
    pub sequence_policy: String,
    pub matcher_policy: String,
    pub frame_metadata_sha256: String,
    pub activity_source_sha256: String,
    pub sequence_report_sha256: String,
    pub appearances: Vec<Appearance>,
}
fn hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
impl Summary {
    /// Checks scope and structural claims, not image-match accuracy or causality.
    pub fn validate(&self, source: &Source) -> Result<()> {
        source.validate()?;
        if self.policy != POLICY
            || ![SEQUENCE_POLICY, ACTIVITY_SEQUENCE_POLICY].contains(&self.sequence_policy.as_str())
            || self.matcher_policy != MATCHER_POLICY
            || self.frame_metadata_sha256 != source.frame_metadata_sha256
            || self.activity_source_sha256
                != crate::visual::digest(&serde_json::to_vec(source).map_err(|e| e.to_string())?)
            || !hash(&self.sequence_report_sha256)
            || self.appearances.len() > 4
        {
            return Err("Appearance summary policy or activity source changed.".into());
        }
        let mut sources = BTreeSet::new();
        for appearance in &self.appearances {
            let origin = &appearance.origin;
            if origin.frame_id.is_empty()
                || origin.frame_id.len() > 256
                || origin.frame_id.chars().any(char::is_control)
                || !hash(&origin.image_sha256)
                || origin.number == 0
                || origin.dimensions
                    != [source.pairs[0].before.width, source.pairs[0].before.height]
                || !hash(&appearance.source_sha256)
                || !sources.insert(&appearance.source_sha256)
            {
                return Err("Invalid appearance origin or duplicate source.".into());
            }
            origin.region.validate(origin.dimensions)?;
            for (outcome, pair) in appearance.pairs.iter().zip(&source.pairs) {
                outcome.validate(origin.dimensions)?;
                if outcome.status != Status::NotMeasured && origin.number > pair.before.number {
                    return Err(
                        "Unresolved or future appearances cannot supply displacement.".into(),
                    );
                }
                for frame in [&pair.before, &pair.after] {
                    if (origin.frame_id == frame.id || origin.number == frame.number)
                        && (origin.frame_id != frame.id
                            || origin.number != frame.number
                            || origin.image_sha256 != frame.sha256)
                    {
                        return Err("Appearance origin disagrees with a selected image.".into());
                    }
                }
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 4096 {
            return Err("Appearance summary exceeds its 4 KiB budget.".into());
        }
        Ok(())
    }
}
