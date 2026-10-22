//! Convert engine-produced passive proposals into compact interpretation evidence.
use crate::{activity_region_suggestions as proposals, region_appearance as matcher};
use reversebolt_core::{
    Result, activity_regions as r,
    appearance_activity::{Outcome, Status},
    play_context::{Frame, Source},
    visual::{self, FrameRef},
};
use serde_json::json;

fn status(value: matcher::Status) -> Status {
    match value {
        matcher::Status::Matched => Status::Matched,
        matcher::Status::UnsupportedSize => Status::UnsupportedSize,
        matcher::Status::LowTexture => Status::LowTexture,
        matcher::Status::PoorMatch => Status::PoorMatch,
        matcher::Status::Ambiguous => Status::Ambiguous,
        matcher::Status::ImageBoundary => Status::ImageBoundary,
        matcher::Status::InconsistentReverse => Status::InconsistentReverse,
    }
}
fn outcome(checks: &[matcher::Report], before: usize, after: usize) -> Result<Outcome> {
    let mut result = Outcome {
        status: Status::NotMeasured,
        displacement: None,
        before_region: None,
        after_region: None,
    };
    // Check order differs from quiet interval order: root is checked before quiet.
    for index in before.min(after)..=before.max(after) {
        let Some(check) = checks.get(index) else {
            return Ok(result);
        };
        if check.status != matcher::Status::Matched {
            result.status = status(check.status);
            return Ok(result);
        }
    }
    let a = checks[before]
        .matched_region
        .ok_or("Matched passive region lacks a rectangle.")?;
    let b = checks[after]
        .matched_region
        .ok_or("Matched passive region lacks a rectangle.")?;
    result.status = Status::Matched;
    result.before_region = Some(a);
    result.after_region = Some(b);
    result.displacement = Some([b[0] - a[0], b[1] - a[1]]);
    Ok(result)
}

pub fn summarize(
    report: &proposals::Report,
    source: &Source,
    frames: &[FrameRef],
) -> Result<r::Summary> {
    let indices = proposals::selected(source, frames)?;
    let checked: Vec<_> = indices.map(|i| frames[i].clone()).into();
    let quiet_input_sha256 = visual::digest(
        &serde_json::to_vec(&frames[indices[0]].input_context).map_err(|e| e.to_string())?,
    );
    let bound = visual::digest(
        &serde_json::to_vec(&json!({"policy":report.policy,"matcher":matcher::POLICY,
        "source":source,"checked_frames":checked,"quiet_input_sha256":quiet_input_sha256}))
        .map_err(|e| e.to_string())?,
    );
    if report.policy != r::SELECTOR_POLICY
        || report.matcher_policy != r::MATCHER_POLICY
        || &report.source != source
        || report.source_sha256 != bound
        || json!(report.checked_frames) != json!(checked)
        || report.origin_frame_id != source.pairs[0].before.id
        || report.origin_image_sha256 != source.pairs[0].before.sha256
        || report.quiet_input_sha256 != quiet_input_sha256
        || report.measurement_dimensions != [512, 288]
        || report.maximum_attempted_frames != 20
        || report.attempted_frames > 20
        || report.suggestions.len() > 4
        || report
            .suggestions
            .iter()
            .map(|s| s.frames.len())
            .sum::<usize>()
            != report.attempted_frames
    {
        return Err(
            "Passive proposals differ from their selected source or measurement policy.".into(),
        );
    }
    let dimensions = [source.pairs[0].before.width, source.pairs[0].before.height];
    let mut regions = vec![];
    for candidate in &report.suggestions {
        candidate.region.validate(dimensions)?;
        if !(1..=5).contains(&candidate.frames.len()) {
            return Err("Passive candidate has an invalid check count.".into());
        }
        for (index, check) in candidate.frames.iter().enumerate() {
            if check.policy != r::MATCHER_POLICY
                || check.source_dimensions != dimensions
                || check.measurement_dimensions != [512, 288]
                || json!(check.source_region) != json!(candidate.region)
                || json!(check.measurement_region)
                    != json!(matcher::normalized_region(candidate.region, dimensions))
                || (check.status != matcher::Status::Matched && index + 1 != candidate.frames.len())
                || (check.status == matcher::Status::Matched
                    && (check.matched_region.is_none() || check.displacement.is_none()))
            {
                return Err(
                    "Passive candidate checks are inconsistent or resume after failure.".into(),
                );
            }
        }
        let last = candidate.frames.last().unwrap();
        if (last.status == matcher::Status::Matched && candidate.frames.len() != 5)
            || (candidate.frames[0].status == matcher::Status::Matched
                && candidate.frames[0].displacement != Some([0., 0.]))
        {
            return Err(
                "Passive candidate lacks a complete stopping point or root correspondence.".into(),
            );
        }
        let missing = || Outcome {
            status: Status::NotMeasured,
            displacement: None,
            before_region: None,
            after_region: None,
        };
        let root_status = status(candidate.frames[0].status);
        let quiet = if root_status == Status::Matched {
            outcome(&candidate.frames, 1, 0)?
        } else {
            missing()
        };
        let first = if quiet.status == Status::Matched {
            outcome(&candidate.frames, 0, 2)?
        } else {
            missing()
        };
        let second = if first.status == Status::Matched {
            outcome(&candidate.frames, 3, 4)?
        } else {
            missing()
        };
        regions.push(r::Evidence {
            region: candidate.region,
            root_status,
            quiet,
            pairs: [first, second],
        });
    }
    let quiet = &checked[1];
    let summary = r::Summary {
        policy: r::POLICY.into(),
        selector_policy: report.policy.into(),
        matcher_policy: report.matcher_policy.into(),
        frame_metadata_sha256: source.frame_metadata_sha256.clone(),
        activity_source_sha256: visual::digest(
            &serde_json::to_vec(source).map_err(|e| e.to_string())?,
        ),
        proposal_report_sha256: visual::digest(
            &serde_json::to_vec(report).map_err(|e| e.to_string())?,
        ),
        origin: source.pairs[0].before.clone(),
        quiet_frame: Frame {
            id: quiet.id.clone(),
            sha256: quiet.sha256.clone(),
            number: quiet.number,
            width: quiet.width,
            height: quiet.height,
            interval: quiet.capture_interval.clone().unwrap(),
        },
        quiet_input_sha256,
        regions,
    };
    summary.validate(source)?;
    Ok(summary)
}
