//! Bounded follow-up areas from exact, provisional image observations.
use crate::{
    Result,
    visual::{self, FrameRef, Observation, Quality, VisualTrack},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const POLICY: &str = "observed_region_followups_v1";
pub const LIMITATION: &str = "Up to four fixed image regions selected from provisional visual observations on the exact earlier image. Small regions are considered first within each pair; pairs share the budget in chronological rounds. Labels and explanations remain observer claims. A region does not establish object identity or follow an object. No geometry is carried across an omitted capture, changed epoch or different image. No gameplay meaning or edit authority is granted.";
pub const MAX_REGIONS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
impl Region {
    pub fn validate_shape(self) -> Result<()> {
        if self.width < 32
            || self.height < 32
            || u64::from(self.width) * u64::from(self.height) > 4_000_000
            || self.x.checked_add(self.width).is_none()
            || self.y.checked_add(self.height).is_none()
        {
            return Err("Choose an image region at least 32 pixels wide and high, within the four-million-pixel budget.".into());
        }
        Ok(())
    }
    pub fn validate(self, dimensions: [u32; 2]) -> Result<()> {
        self.validate_shape()?;
        if self.x + self.width > dimensions[0] || self.y + self.height > dimensions[1] {
            return Err("The fixed region lies outside the captured image.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Source {
    pub track_id: usize,
    pub observation_sha256: String,
    pub observation: Observation,
}
#[derive(Clone, Debug, Serialize)]
pub struct Plan {
    pub before_id: String,
    pub after_id: String,
    pub region: Region,
    pub sources: Vec<Source>,
    pub source_sha256: String,
}
#[derive(Debug, Serialize)]
pub struct Selection {
    pub policy: &'static str,
    pub frame_metadata_sha256: String,
    pub considered_observations: usize,
    pub eligible_observations: usize,
    pub omitted_observations: BTreeMap<String, usize>,
    pub duplicate_observations: usize,
    pub candidate_regions: usize,
    pub omitted_regions: usize,
    pub limit: usize,
    pub regions: Vec<Plan>,
    pub limitation: &'static str,
}

pub fn select(
    investigation: &str,
    frames: &[FrameRef],
    tracks: &[VisualTrack],
) -> Result<Selection> {
    if investigation.is_empty()
        || investigation.len() > 64
        || !(2..=20).contains(&frames.len())
        || tracks.len() > visual::MAX_TRACKS
        || tracks.iter().map(|t| t.id).collect::<BTreeSet<_>>().len() != tracks.len()
        || frames.iter().map(|f| &f.id).collect::<BTreeSet<_>>().len() != frames.len()
        || frames.iter().any(|f| {
            f.number == 0
                || f.id.is_empty()
                || f.id.len() > 256
                || f.width == 0
                || f.height == 0
                || u64::from(f.width) * u64::from(f.height) > 4_000_000
                || f.sha256.len() != 64
                || !f.sha256.bytes().all(|c| c.is_ascii_hexdigit())
        })
        || frames
            .windows(2)
            .any(|p| p[0].number >= p[1].number || p[0].started_unix_ms > p[1].started_unix_ms)
    {
        return Err("Region selection requires bounded, distinct observations and ordered image references.".into());
    }
    let mut selection = Selection {
        policy: POLICY,
        frame_metadata_sha256: visual::digest(
            &serde_json::to_vec(frames).map_err(|e| e.to_string())?,
        ),
        considered_observations: 0,
        eligible_observations: 0,
        omitted_observations: BTreeMap::new(),
        duplicate_observations: 0,
        candidate_regions: 0,
        omitted_regions: 0,
        limit: MAX_REGIONS,
        regions: vec![],
        limitation: LIMITATION,
    };
    let mut pairs = vec![Vec::<Plan>::new(); frames.len() - 1];
    let mut ordered_tracks = tracks.iter().collect::<Vec<_>>();
    ordered_tracks.sort_by_key(|t| t.id);
    for (index, before) in frames.iter().enumerate() {
        for track in &ordered_tracks {
            let Some(observation) = track
                .observations
                .get(before.number - 1)
                .and_then(Option::as_ref)
            else {
                continue;
            };
            selection.considered_observations += 1;
            let input = &observation.input;
            let candidate = (|| -> std::result::Result<Region, &str> {
                if input.investigation_id != investigation
                    || track.epoch != before.epoch
                    || track.observer != input.observer
                    || track.key != input.key
                    || track.label != input.label
                    || track.unit != input.unit
                    || track.submitted_by != observation.submitted_by
                    || observation.submitted_by.is_empty()
                    || observation.submitted_by.len() > 64
                    || observation.submitted_by.chars().any(char::is_control)
                    || visual::validate(input, before, frames.last().unwrap().number).is_err()
                {
                    return Err("invalid_source_binding");
                }
                if input.quality != Quality::Provisional {
                    return Err("uncertain_observation");
                }
                let [x, y, w, h] = input.bounds.ok_or("missing_bounds")?;
                let Some(after) = frames.get(index + 1) else {
                    return Err("no_following_image");
                };
                if before.number.checked_add(1) != Some(after.number) {
                    return Err("omitted_capture");
                }
                if before.epoch != after.epoch
                    || [before.width, before.height] != [after.width, after.height]
                {
                    return Err("changed_geometry_or_epoch");
                }
                // Round outward under the same f32 coordinate convention used by
                // visual::validate; do not pad tiny boxes or invent off-image area.
                let (left, top, right, bottom) = (
                    x.floor() as u32,
                    y.floor() as u32,
                    (x + w).ceil() as u32,
                    (y + h).ceil() as u32,
                );
                let region = Region {
                    x: left,
                    y: top,
                    width: right - left,
                    height: bottom - top,
                };
                region
                    .validate([before.width, before.height])
                    .map_err(|_| "unsupported_region_size")?;
                Ok(region)
            })();
            let region = match candidate {
                Ok(region) => region,
                Err(reason) => {
                    *selection
                        .omitted_observations
                        .entry(reason.into())
                        .or_default() += 1;
                    continue;
                }
            };
            selection.eligible_observations += 1;
            let source = Source {
                track_id: track.id,
                observation_sha256: visual::digest(
                    &serde_json::to_vec(observation).map_err(|e| e.to_string())?,
                ),
                observation: observation.clone(),
            };
            if let Some(plan) = pairs[index].iter_mut().find(|p| p.region == region) {
                plan.sources.push(source);
                selection.duplicate_observations += 1;
            } else {
                pairs[index].push(Plan {
                    before_id: before.id.clone(),
                    after_id: frames[index + 1].id.clone(),
                    region,
                    sources: vec![source],
                    source_sha256: String::new(),
                });
            }
        }
    }
    for (index, pair) in pairs.iter_mut().enumerate() {
        for plan in pair.iter_mut() {
            plan.source_sha256=visual::digest(&serde_json::to_vec(&serde_json::json!({"policy":POLICY,"investigation_id":investigation,
                "before":frames[index],"after":frames[index+1],"region":plan.region,"sources":plan.sources})).map_err(|e|e.to_string())?);
        }
        pair.sort_by_key(|p| {
            (
                u64::from(p.region.width) * u64::from(p.region.height),
                p.region,
            )
        });
        selection.candidate_regions += pair.len();
    }
    for round in 0..visual::MAX_TRACKS {
        for pair in &pairs {
            if let Some(plan) = pair.get(round)
                && selection.regions.len() < MAX_REGIONS
            {
                selection.regions.push(plan.clone());
            }
        }
    }
    selection.omitted_regions = selection.candidate_regions - selection.regions.len();
    Ok(selection)
}

#[cfg(test)]
#[path = "visual_regions_tests.rs"]
mod tests;
