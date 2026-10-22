//! Numeric region proposals from two retained activity intervals, without labels.
use crate::{
    appearance_activity, region_appearance as matcher, stable_region_selection as stable,
    trial_appearance_suggestions as tiles,
};
use reversebolt_core::{
    Result,
    action_patterns::{self, Exposure},
    play_context::Source,
    visual::{self, FrameRef},
    visual_regions::Region,
};
use serde::Serialize;
use serde_json::json;

pub const POLICY: &str = reversebolt_core::activity_regions::SELECTOR_POLICY;
pub const CHANGE_POLICY: &str = "passive_activity_region_suggestions_v1";
pub const CHANGE_LIMITATION: &str = "Numeric proposals from two sampled-key intervals and the quiet interval immediately before the first. First-action change minus quiet change ranks fixed multiscale tiles. At most four candidates and twenty original-template checks. The two action intervals are independent correspondences, not continuous tracking between them; intervening images are not read and occlusion or replacement there is unknown. Same-key samples do not prove game input delivery, equal movement or direction. Camera/background motion and identical appearances can pass. No object identity, gameplay name, physical speed, causal field, edit authority or trial baseline is established. No control or comparison selection is created.";
pub const LIMITATION: &str = "Numeric proposals from two sampled-key intervals and preceding quiet evidence. First-action-minus-quiet change supplies fixed tiles; source texture, two-direction corner strength and quiet stability filter and rank them before matching. Quality scores are heuristics, not confidence. At most four candidates and twenty original-template checks. The two action intervals are independent correspondences, not continuous tracking between them; intervening images are not read and occlusion or replacement there is unknown. Same-key samples do not prove game input delivery, equal movement or direction. Camera/background motion and identical appearances can pass. No object identity, gameplay name, physical speed, causal field, edit authority or trial baseline is established. No control or comparison selection is created.";
const MAX_CANDIDATES: usize = 4;

pub use crate::stable_region_selection::Ranking;

#[derive(Debug, Serialize)]
pub struct Suggestion {
    /// Coordinates in the first action's before image, not a later trial baseline.
    pub region: Region,
    pub action_minus_idle_change: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_quality: Option<stable::Quality>,
    /// Root self-match, quiet image, first after, second before, second after.
    pub frames: Vec<matcher::Report>,
    /// Quiet interval, then the two independent activity intervals, normalized pixels.
    pub displacement: [Option<[i32; 2]>; 3],
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub policy: &'static str,
    pub matcher_policy: &'static str,
    pub source: Source,
    pub source_sha256: String,
    pub origin_frame_id: String,
    pub origin_image_sha256: String,
    /// Fixed check order, shared by every suggestion's frames.
    pub checked_frames: Vec<FrameRef>,
    pub quiet_input_sha256: String,
    pub measurement_dimensions: [u32; 2],
    pub ranked_regions: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<stable::Selection>,
    pub attempted_frames: usize,
    pub maximum_attempted_frames: usize,
    pub suggestions: Vec<Suggestion>,
    /// Index of the first ranked candidate with quiet <= 1 and both actions >= 2 pixels.
    pub recommended: Option<usize>,
    pub status: &'static str,
    pub limitation: &'static str,
}

pub(crate) fn selected(source: &Source, frames: &[FrameRef]) -> Result<[usize; 5]> {
    let last = appearance_activity::validate_source(source, frames)?;
    let first = frames
        .iter()
        .position(|f| f.id == source.pairs[0].before.id)
        .unwrap();
    let quiet = first
        .checked_sub(1)
        .ok_or("Missing preceding quiet image.")?;
    if last <= first + 2 {
        return Err("Selected activity pairs are not ordered in the requested frames.".into());
    }
    if action_patterns::exposure(&frames[quiet], &frames[first]) != Ok(Exposure::NoKeysDetected) {
        return Err("The preceding interval does not have bounded quiet input evidence.".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for frame in frames {
        if frame.id.is_empty() || !ids.insert(&frame.id) {
            return Err("Frame identities must be distinct and nonempty.".into());
        }
    }
    // A missing capture or known scene change must not be silently bridged, even
    // though unexamined intermediate pixels cannot establish continuous identity.
    for pair in frames[quiet..=last].windows(2) {
        let a = &pair[0];
        let b = &pair[1];
        let (Some(ai), Some(bi)) = (&a.capture_interval, &b.capture_interval) else {
            return Err("Missing capture interval.".into());
        };
        if a.number.checked_add(1) != Some(b.number)
            || a.epoch != b.epoch
            || [a.width, a.height] != [b.width, b.height]
            || ai.clock_id != source.investigation_id
            || bi.clock_id != source.investigation_id
            || ai.begin_us > ai.end_us
            || bi.begin_us > bi.end_us
            || ai.end_us >= bi.begin_us
        {
            return Err(
                "Activity crosses omitted captures, changed geometry, scene or clock.".into(),
            );
        }
    }
    let indices = [first, quiet, first + 1, last - 1, last];
    let generation = &frames[first].input_context.as_ref().unwrap().generation;
    if indices.iter().any(|&i| {
        frames[i]
            .input_context
            .as_ref()
            .is_none_or(|w| &w.generation != generation)
    }) {
        return Err("Selected intervals use different input monitor generations.".into());
    }
    Ok(indices)
}

pub fn analyze(
    source: &Source,
    frames: &[FrameRef],
    read_image: impl FnMut(&FrameRef) -> Result<Vec<u8>>,
) -> Result<Report> {
    analyze_with_ranking(source, frames, read_image, Ranking::StableTexture)
}

pub fn analyze_with_ranking(
    source: &Source,
    frames: &[FrameRef],
    mut read_image: impl FnMut(&FrameRef) -> Result<Vec<u8>>,
    ranking: Ranking,
) -> Result<Report> {
    let (policy, limitation) = match ranking {
        Ranking::ChangeOnly => (CHANGE_POLICY, CHANGE_LIMITATION),
        Ranking::StableTexture => (POLICY, LIMITATION),
    };
    let indices = selected(source, frames)?;
    let root = &frames[indices[0]];
    let dimensions = [root.width, root.height];
    let checked_frames: Vec<_> = indices.map(|i| frames[i].clone()).into();
    let quiet_input_sha256 =
        visual::digest(&serde_json::to_vec(&root.input_context).map_err(|e| e.to_string())?);
    let source_sha256 = visual::digest(
        &serde_json::to_vec(&json!({"policy":policy,"matcher":matcher::POLICY,
            "source":source,"checked_frames":checked_frames,"quiet_input_sha256":quiet_input_sha256}))
        .map_err(|e| e.to_string())?,
    );
    let mut report = Report {
        policy,
        matcher_policy: matcher::POLICY,
        source: source.clone(),
        source_sha256,
        origin_frame_id: root.id.clone(),
        origin_image_sha256: root.sha256.clone(),
        checked_frames,
        quiet_input_sha256,
        measurement_dimensions: [512, 288],
        ranked_regions: 0,
        selection: None,
        attempted_frames: 0,
        maximum_attempted_frames: 20,
        suggestions: vec![],
        recommended: None,
        status: "no_action_regions",
        limitation,
    };
    // Rank in quiet/root/first-after order. Retain only five normalized images,
    // not all requested PNGs; failures never produce a partial recommendation.
    let mut images = vec![];
    for i in [indices[1], indices[0], indices[2], indices[3], indices[4]] {
        let frame = &frames[i];
        let bytes = read_image(frame)?;
        if bytes.len() > 10 * 1024 * 1024 || visual::digest(&bytes) != frame.sha256 {
            return Err("Activity image checksum or size differs from retained evidence.".into());
        }
        images.push(tiles::resized(&bytes, dimensions)?);
    }
    let ranked = tiles::regions(&images, dimensions);
    let ranked = match ranking {
        Ranking::ChangeOnly => ranked.into_iter().map(stable::Candidate::from).collect(),
        Ranking::StableTexture => {
            let (ranked, selection) = stable::select(&images[0], &images[1], dimensions, ranked);
            report.selection = Some(selection);
            ranked
        }
    };
    report.ranked_regions = ranked.len();
    let mut used = vec![];
    for stable::Candidate {
        change,
        region,
        normalized,
        quality,
    } in ranked
    {
        if used.iter().any(|&r| tiles::overlap(r, normalized)) {
            continue;
        }
        used.push(normalized);
        let mut suggestion = Suggestion {
            region,
            action_minus_idle_change: change,
            selection_quality: quality,
            frames: vec![],
            displacement: [None; 3],
            status: "unresolved_appearance",
        };
        let mut positions = vec![];
        for i in [1, 0, 2, 3, 4] {
            let measured = matcher::prepared(&images[1], &images[i], region, dimensions);
            report.attempted_frames += 1;
            let resolved = measured.status == matcher::Status::Matched;
            if resolved {
                positions.push(measured.forward.as_ref().unwrap().best_at.unwrap());
            }
            suggestion.frames.push(measured);
            if !resolved {
                break;
            }
        }
        let delta = |a: usize, b: usize| {
            positions
                .get(a)
                .zip(positions.get(b))
                .map(|(a, b)| [b[0] as i32 - a[0] as i32, b[1] as i32 - a[1] as i32])
        };
        suggestion.displacement = [delta(1, 0), delta(0, 2), delta(3, 4)];
        if positions.len() == 5 {
            let magnitude = |i: usize| {
                let d = suggestion.displacement[i].unwrap();
                d[0].unsigned_abs().max(d[1].unsigned_abs())
            };
            suggestion.status = if magnitude(0) > 1 {
                "quiet_displacement"
            } else if magnitude(1) < 2 || magnitude(2) < 2 {
                "below_threshold"
            } else {
                "motion_in_both_pairs"
            };
            if suggestion.status == "motion_in_both_pairs" && report.recommended.is_none() {
                report.recommended = Some(report.suggestions.len());
            }
        }
        report.suggestions.push(suggestion);
        if used.len() == MAX_CANDIDATES {
            break;
        }
    }
    report.status = if report.recommended.is_some() {
        "suggestions_available"
    } else if report.ranked_regions > 0 {
        "no_supported_region"
    } else if report
        .selection
        .as_ref()
        .is_some_and(|s| s.input_regions > 0)
    {
        "no_stable_textured_region"
    } else {
        "no_action_regions"
    };
    Ok(report)
}

#[cfg(test)]
#[path = "activity_region_suggestions_tests.rs"]
mod tests;
