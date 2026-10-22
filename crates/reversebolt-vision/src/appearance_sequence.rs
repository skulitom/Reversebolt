//! Bounded sequences of independent matches to one immutable source appearance.
use crate::region_appearance::{self, Report as MatchReport, Status as MatchStatus};
use reversebolt_core::{
    Result,
    action_patterns::{self, Exposure},
    temporal,
    visual::{self, FrameRef, VisualTrack},
    visual_regions::{self, Plan},
};
use serde::Serialize;
use serde_json::json;

pub const POLICY: &str = "immutable_appearance_sequence_v1";
pub const MAX_STEPS: usize = 6;
pub const MAX_ATTEMPTS: usize = 12;
pub const LIMITATION: &str = "Independent image matches to the original observed rectangle, never an updated template or persistent object identity. Stop on the first unresolved appearance, source error, missing capture or changed geometry/epoch; never resume across that gap. At most six following captures per source and twelve attempted pairs shared in rounds across selected sources. Displacements are approximate image pixels, not physical speed or causal effects. Capture bounds are not presentation times, and sampled keys do not prove delivery or absence of other input. Identical-looking replacements and background dominance can mislead. No interpolation, semantic label, memory correspondence or edit authority is created.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stop {
    EndOfRequestedFrames,
    StepLimit,
    AnalysisBudget,
    UnresolvedAppearance,
    SourceError,
    OmittedCapture,
    ChangedGeometryOrEpoch,
}
#[derive(Debug, Serialize)]
pub struct Step {
    pub before_id: String,
    pub before_sha256: String,
    pub after_id: String,
    pub after_sha256: String,
    pub source_sha256: String,
    pub elapsed_us: Option<[u64; 2]>,
    pub timing_error: Option<String>,
    pub exposure: Exposure,
    pub input_sha256: String,
    /// Only accepted matches have deltas. These are not world units or speed.
    pub displacement_from_origin: Option<[f64; 2]>,
    pub displacement_from_previous: Option<[f64; 2]>,
    pub measurement: Option<MatchReport>,
    pub error: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Series {
    pub source: Plan,
    pub accepted_steps: usize,
    pub steps: Vec<Step>,
    pub stop: Stop,
    /// First requested frame not accepted, or next eligible frame at a limit.
    pub stopped_before: Option<String>,
    pub reason: String,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub policy: &'static str,
    pub appearance_policy: &'static str,
    pub frame_metadata_sha256: String,
    pub selection_sha256: String,
    pub source_sha256: String,
    pub maximum_steps_per_source: usize,
    pub maximum_attempted_pairs: usize,
    pub attempted_pairs: usize,
    pub series: Vec<Series>,
    pub limitation: &'static str,
}

fn stop(series: &mut Series, kind: Stop, next: Option<&FrameRef>, reason: &str) {
    series.stop = kind;
    series.stopped_before = next.map(|f| f.id.clone());
    series.reason = reason.into();
}

pub fn analyze(
    investigation: &str,
    frames: &[FrameRef],
    tracks: &[VisualTrack],
    mut read_image: impl FnMut(&FrameRef) -> Result<Vec<u8>>,
) -> Result<Report> {
    let selection = visual_regions::select(investigation, frames, tracks)?;
    let selection_sha256 =
        visual::digest(&serde_json::to_vec(&selection).map_err(|e| e.to_string())?);
    let source_sha256=visual::digest(&serde_json::to_vec(&json!({"policy":POLICY,"appearance_policy":region_appearance::POLICY,
        "selection_sha256":selection_sha256,"maximum_steps":MAX_STEPS,"maximum_attempts":MAX_ATTEMPTS})).map_err(|e|e.to_string())?);
    let starts = selection
        .regions
        .iter()
        .map(|p| frames.iter().position(|f| f.id == p.before_id).unwrap())
        .collect::<Vec<_>>();
    let mut report = Report {
        policy: POLICY,
        appearance_policy: region_appearance::POLICY,
        frame_metadata_sha256: selection.frame_metadata_sha256,
        selection_sha256,
        source_sha256,
        maximum_steps_per_source: MAX_STEPS,
        maximum_attempted_pairs: MAX_ATTEMPTS,
        attempted_pairs: 0,
        series: selection
            .regions
            .into_iter()
            .map(|source| Series {
                source,
                accepted_steps: 0,
                steps: vec![],
                stop: Stop::EndOfRequestedFrames,
                stopped_before: None,
                reason: String::new(),
            })
            .collect(),
        limitation: LIMITATION,
    };
    let mut finished = vec![false; report.series.len()];
    for round in 0..MAX_STEPS {
        for (index, series) in report.series.iter_mut().enumerate() {
            if finished[index] {
                continue;
            }
            let origin = &frames[starts[index]];
            let previous = &frames[starts[index] + round];
            let Some(next) = frames.get(starts[index] + round + 1) else {
                stop(
                    series,
                    Stop::EndOfRequestedFrames,
                    None,
                    "No further requested capture.",
                );
                finished[index] = true;
                continue;
            };
            let failure = if previous.number.checked_add(1) != Some(next.number) {
                Some((Stop::OmittedCapture, "An intervening capture was omitted."))
            } else if origin.epoch != next.epoch
                || [origin.width, origin.height] != [next.width, next.height]
            {
                Some((
                    Stop::ChangedGeometryOrEpoch,
                    "Image geometry or scene epoch changed.",
                ))
            } else if report.attempted_pairs == MAX_ATTEMPTS {
                Some((
                    Stop::AnalysisBudget,
                    "The shared appearance-analysis budget was reached.",
                ))
            } else {
                None
            };
            if let Some((kind, reason)) = failure {
                stop(series, kind, Some(next), reason);
                finished[index] = true;
                continue;
            }
            report.attempted_pairs += 1;
            let timing = temporal::sequence(
                &[previous.clone(), next.clone()],
                &[previous.id.clone(), next.id.clone()],
            );
            let (elapsed_us, timing_error) = match timing {
                Ok(s) => (Some(s.elapsed_us), None),
                Err(error) => (None, Some(error)),
            };
            let exposure = action_patterns::exposure(previous, next)
                .unwrap_or_else(|e| Exposure::Unknown(e.into()));
            let bound=visual::digest(&serde_json::to_vec(&json!({"policy":POLICY,"appearance_policy":region_appearance::POLICY,
                "source_sha256":series.source.source_sha256,"frames":&frames[starts[index]..=starts[index]+round+1]})).map_err(|e|e.to_string())?);
            let measured = (|| -> Result<MatchReport> {
                let mut checked = |frame: &FrameRef| -> Result<Vec<u8>> {
                    let bytes = read_image(frame)?;
                    if bytes.len() > visual::MAX_IMAGE_BYTES
                        || visual::digest(&bytes) != frame.sha256
                    {
                        return Err("Source capture is oversized or its checksum changed.".into());
                    }
                    Ok(bytes)
                };
                // Always recheck the original pixels. A previous match never becomes a template.
                let a = checked(origin)?;
                let b = checked(next)?;
                let measured = region_appearance::measure(&a, &b, series.source.region)?;
                if measured.source_dimensions != [origin.width, origin.height] {
                    return Err("PNG dimensions differ from the retained image metadata.".into());
                }
                Ok(measured)
            })();
            let (measurement, error) = match measured {
                Ok(m) => (Some(m), None),
                Err(error) => (None, Some(error)),
            };
            let displacement = measurement
                .as_ref()
                .filter(|m| m.status == MatchStatus::Matched)
                .and_then(|m| m.displacement);
            let prior = series
                .steps
                .last()
                .and_then(|s| s.displacement_from_origin)
                .unwrap_or([0., 0.]);
            let step_delta = displacement.map(|d| [d[0] - prior[0], d[1] - prior[1]]);
            let failed = if error.is_some() {
                Some((
                    Stop::SourceError,
                    "A source image could not be verified or measured.",
                ))
            } else if displacement.is_none() {
                Some((
                    Stop::UnresolvedAppearance,
                    "The appearance could not be matched unambiguously.",
                ))
            } else {
                None
            };
            series.steps.push(Step {
                before_id: previous.id.clone(),
                before_sha256: previous.sha256.clone(),
                after_id: next.id.clone(),
                after_sha256: next.sha256.clone(),
                source_sha256: bound,
                elapsed_us,
                timing_error,
                exposure,
                input_sha256: visual::digest(
                    &serde_json::to_vec(&next.input_context).map_err(|e| e.to_string())?,
                ),
                displacement_from_origin: displacement,
                displacement_from_previous: step_delta,
                measurement,
                error,
            });
            if let Some((kind, reason)) = failed {
                stop(series, kind, Some(next), reason);
                finished[index] = true;
            } else {
                series.accepted_steps += 1;
            }
        }
    }
    for (index, series) in report.series.iter_mut().enumerate() {
        if finished[index] {
            continue;
        }
        let next = frames.get(starts[index] + MAX_STEPS + 1);
        if next.is_some() {
            stop(
                series,
                Stop::StepLimit,
                next,
                "The per-source six-capture limit was reached.",
            );
        } else {
            stop(
                series,
                Stop::EndOfRequestedFrames,
                None,
                "No further requested capture.",
            );
        }
    }
    Ok(report)
}

#[cfg(test)]
#[path = "appearance_sequence_tests.rs"]
mod tests;
