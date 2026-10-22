//! Independent passive-region correspondences supplied to an activity interpreter.
use crate::{
    Result,
    appearance_activity::{Outcome, Status},
    play_context::{Frame, Source},
    visual,
    visual_regions::Region,
};
use serde::{Deserialize, Serialize};

pub const POLICY: &str = "selected_passive_regions_v1";
pub const SELECTOR_POLICY: &str = "passive_activity_stable_texture_regions_v2";
pub const MATCHER_POLICY: &str = crate::appearance_activity::MATCHER_POLICY;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// Requested rectangle in the origin image; outcome rectangles include normalization.
    pub region: Region,
    pub root_status: Status,
    pub quiet: Outcome,
    pub pairs: [Outcome; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub policy: String,
    pub selector_policy: String,
    pub matcher_policy: String,
    pub frame_metadata_sha256: String,
    pub activity_source_sha256: String,
    pub proposal_report_sha256: String,
    pub origin: Frame,
    pub quiet_frame: Frame,
    pub quiet_input_sha256: String,
    pub regions: Vec<Evidence>,
}
fn hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
impl Summary {
    /// Existing matcher thresholds, converted from original to normalized pixels.
    /// A descriptive qualification, never object identity or gameplay authority.
    pub fn supports(&self, source: &Source) -> Result<bool> {
        self.validate(source)?;
        let size = [source.pairs[0].before.width, source.pairs[0].before.height];
        let displacement = |o: &Outcome| {
            o.displacement.map(|d| {
                (d[0] * 512. / f64::from(size[0]))
                    .abs()
                    .max((d[1] * 288. / f64::from(size[1])).abs())
            })
        };
        Ok(self.regions.iter().any(|r| {
            r.root_status == Status::Matched
                && displacement(&r.quiet).is_some_and(|d| d <= 1. + 1e-6)
                && r.pairs
                    .iter()
                    .all(|p| displacement(p).is_some_and(|d| d >= 2. - 1e-6))
        }))
    }
    /// Structural and source checks do not establish image accuracy or causality.
    pub fn validate(&self, source: &Source) -> Result<()> {
        source.validate()?;
        if self.policy != POLICY
            || self.selector_policy != SELECTOR_POLICY
            || self.matcher_policy != MATCHER_POLICY
            || self.frame_metadata_sha256 != source.frame_metadata_sha256
            || self.activity_source_sha256
                != visual::digest(&serde_json::to_vec(source).map_err(|e| e.to_string())?)
            || !hash(&self.proposal_report_sha256)
            || !hash(&self.quiet_input_sha256)
            || self.origin != source.pairs[0].before
            || self.regions.len() > 4
        {
            return Err(
                "Passive region summary differs from its activity source or policy.".into(),
            );
        }
        let quiet = &self.quiet_frame;
        let dimensions = [self.origin.width, self.origin.height];
        if quiet.id.is_empty()
            || quiet.id.len() > 256
            || quiet.id.chars().any(char::is_control)
            || source.frames().iter().any(|f| f.id == quiet.id)
            || !hash(&quiet.sha256)
            || quiet.number == 0
            || quiet.number.checked_add(1) != Some(self.origin.number)
            || [quiet.width, quiet.height] != dimensions
            || quiet.interval.clock_id != source.investigation_id
            || quiet.interval.begin_us > quiet.interval.end_us
            || quiet.interval.end_us >= self.origin.interval.begin_us
            || self
                .origin
                .interval
                .end_us
                .saturating_sub(quiet.interval.begin_us)
                > 4_000_000
        {
            return Err("Passive region quiet reference is not the preceding capture.".into());
        }
        let mut rectangles = std::collections::BTreeSet::new();
        for region in &self.regions {
            region.region.validate(dimensions)?;
            let r = region.region;
            if !rectangles.insert([r.x, r.y, r.width, r.height]) {
                return Err("Passive region summary repeats a candidate.".into());
            }
            for outcome in [&region.quiet, &region.pairs[0], &region.pairs[1]] {
                outcome.validate(dimensions)?;
            }
            if matches!(
                region.root_status,
                Status::NotMeasured | Status::SourceError
            ) || (region.root_status != Status::Matched
                && region.quiet.status != Status::NotMeasured)
                || (region.quiet.status != Status::Matched
                    && region.pairs.iter().any(|p| p.status != Status::NotMeasured))
                || (region.pairs[0].status != Status::Matched
                    && region.pairs[1].status != Status::NotMeasured)
            {
                return Err(
                    "Passive regions cannot resume after an unresolved earlier check.".into(),
                );
            }
            if let (Some(quiet_after), Some(first_before)) =
                (region.quiet.after_region, region.pairs[0].before_region)
                && quiet_after != first_before
            {
                return Err("Passive region root rectangles disagree.".into());
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 6 * 1024 {
            return Err("Passive region summary exceeds its 6 KiB budget.".into());
        }
        Ok(())
    }
}
