//! Project an engine-produced appearance sequence onto selected action intervals.
use crate::{appearance_sequence, region_appearance};
use reversebolt_core::{
    Result, appearance_activity as a,
    play_context::Source,
    visual::{self, FrameRef},
};

pub(crate) fn validate_source(source: &Source, frames: &[FrameRef]) -> Result<usize> {
    source.validate()?;
    let hash = visual::digest(&serde_json::to_vec(frames).map_err(|e| e.to_string())?);
    if !(2..=20).contains(&frames.len()) || source.frame_metadata_sha256 != hash {
        return Err("Activity frame metadata changed.".into());
    }
    for reference in source.frames() {
        let frame = frames
            .iter()
            .find(|f| f.id == reference.id)
            .ok_or("Selected image is missing")?;
        if frame.sha256 != reference.sha256
            || frame.number != reference.number
            || [frame.width, frame.height] != [reference.width, reference.height]
            || frame.capture_interval.as_ref() != Some(&reference.interval)
        {
            return Err("Selected image metadata changed.".into());
        }
    }
    for pair in &source.pairs {
        let before = frames.iter().position(|f| f.id == pair.before.id).unwrap();
        let after = frames.iter().position(|f| f.id == pair.after.id).unwrap();
        if before + 1 != after
            || visual::digest(
                &serde_json::to_vec(&frames[after].input_context).map_err(|e| e.to_string())?,
            ) != pair.input_sha256
            || reversebolt_core::action_patterns::exposure(&frames[before], &frames[after])
                != Ok(reversebolt_core::action_patterns::Exposure::Key(source.key))
        {
            return Err("Selected action input differs from its retained interval.".into());
        }
    }
    Ok(frames
        .iter()
        .position(|f| f.id == source.pairs[1].after.id)
        .unwrap())
}

pub fn summarize(
    report: &appearance_sequence::Report,
    source: &Source,
    frames: &[FrameRef],
) -> Result<a::Summary> {
    let last = validate_source(source, frames)?;
    let hash = source.frame_metadata_sha256.clone();
    let bound_source = visual::digest(&serde_json::to_vec(source).map_err(|e| e.to_string())?);
    let valid_policy = match (&report.target, report.policy) {
        (None, a::SEQUENCE_POLICY) => {
            report.maximum_steps_per_source == appearance_sequence::MAX_STEPS
        }
        (Some(target), a::ACTIVITY_SEQUENCE_POLICY) => {
            report.maximum_steps_per_source == frames.len() - 1
                && target.source_sha256 == bound_source
                && target.last_frame_id == frames[last].id
        }
        _ => false,
    };
    if !valid_policy
        || report.appearance_policy != a::MATCHER_POLICY
        || report.frame_metadata_sha256 != hash
        || report.maximum_attempted_pairs != appearance_sequence::MAX_ATTEMPTS
        || report.attempted_pairs > appearance_sequence::MAX_ATTEMPTS
    {
        return Err("Appearance sequence differs from the activity's retained evidence.".into());
    }
    let mut appearances = vec![];
    for series in &report.series {
        let origin = frames
            .iter()
            .find(|f| f.id == series.source.before_id)
            .ok_or("Appearance origin is missing")?;
        if series.source.sources.is_empty()
            || series.source.sources.iter().any(|s| {
                s.observation.input.investigation_id != source.investigation_id
                    || s.observation.input.frame_id != origin.id
                    || s.observation.input.image_sha256 != origin.sha256
            })
        {
            return Err("Appearance observation belongs to another investigation or image.".into());
        }
        let mut pairs = std::array::from_fn(|_| a::Outcome {
            status: a::Status::NotMeasured,
            displacement: None,
            before_region: None,
            after_region: None,
        });
        for (index, pair) in source.pairs.iter().enumerate() {
            let Some(step) = series
                .steps
                .iter()
                .find(|s| s.before_id == pair.before.id && s.after_id == pair.after.id)
            else {
                continue;
            };
            if step.before_sha256 != pair.before.sha256
                || step.after_sha256 != pair.after.sha256
                || step.input_sha256 != pair.input_sha256
                || step.exposure != reversebolt_core::action_patterns::Exposure::Key(source.key)
            {
                return Err(
                    "Appearance interval differs from the selected action evidence.".into(),
                );
            }
            let status = match (&step.measurement, &step.error) {
                (None, Some(_)) => a::Status::SourceError,
                (Some(m), None) => match m.status {
                    region_appearance::Status::Matched => a::Status::Matched,
                    region_appearance::Status::UnsupportedSize => a::Status::UnsupportedSize,
                    region_appearance::Status::LowTexture => a::Status::LowTexture,
                    region_appearance::Status::PoorMatch => a::Status::PoorMatch,
                    region_appearance::Status::Ambiguous => a::Status::Ambiguous,
                    region_appearance::Status::ImageBoundary => a::Status::ImageBoundary,
                    region_appearance::Status::InconsistentReverse => {
                        a::Status::InconsistentReverse
                    }
                },
                _ => return Err("Appearance interval lacks one measurement outcome.".into()),
            };
            let after_region = step.measurement.as_ref().and_then(|m| m.matched_region);
            let before_region = after_region
                .zip(step.displacement_from_previous)
                .map(|(r, d)| [r[0] - d[0], r[1] - d[1], r[2], r[3]]);
            pairs[index] = a::Outcome {
                status,
                displacement: step.displacement_from_previous,
                before_region,
                after_region,
            };
        }
        appearances.push(a::Appearance {
            origin: a::Origin {
                frame_id: origin.id.clone(),
                image_sha256: origin.sha256.clone(),
                number: origin.number,
                dimensions: [origin.width, origin.height],
                region: series.source.region,
            },
            source_sha256: series.source.source_sha256.clone(),
            pairs,
        });
    }
    let summary = a::Summary {
        policy: a::POLICY.into(),
        sequence_policy: report.policy.into(),
        matcher_policy: report.appearance_policy.into(),
        frame_metadata_sha256: hash,
        activity_source_sha256: visual::digest(
            &serde_json::to_vec(source).map_err(|e| e.to_string())?,
        ),
        sequence_report_sha256: visual::digest(
            &serde_json::to_vec(report).map_err(|e| e.to_string())?,
        ),
        appearances,
    };
    summary.validate(source)?;
    Ok(summary)
}
