//! Shared, streaming verification for retained trial image comparisons.
use reversebolt_core::{
    Result,
    inputs::Window,
    runtime_tests::{Frame, InputWitness, Test},
    visual,
};

pub(crate) struct Pair<'a> {
    pub condition: usize,
    pub index: usize,
    pub before: &'a Frame,
    pub after: &'a Frame,
    pub images: Result<(&'a [u8], &'a [u8])>,
    pub input: Result<InputWitness>,
}

pub(crate) fn validate(test: &Test) -> Result<()> {
    test.validate_plan()?;
    if test.conditions.iter().any(|c| c.frames.len() > 6) {
        return Err("Trial motion supports at most six frames per condition.".into());
    }
    for (c, condition) in test.conditions.iter().enumerate() {
        for (i, frame) in condition.frames.iter().enumerate() {
            if frame.path != format!("{}/{c}-{i}.png", test.id)
                || frame.input_path != format!("{}/{c}-{i}.input.json", test.id)
                || [&frame.sha256, &frame.input_sha256]
                    .iter()
                    .any(|h| h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()))
            {
                return Err("Trial motion requires canonical retained evidence references.".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn check_image(bytes: Vec<u8>, frame: &Frame) -> Result<Vec<u8>> {
    if bytes.len() > visual::MAX_IMAGE_BYTES || visual::digest(&bytes) != frame.sha256 {
        return Err("Image checksum changed or image exceeds its size limit.".into());
    }
    Ok(bytes)
}

pub(crate) fn visit(
    test: &Test,
    mut read_image: impl FnMut(usize, usize) -> Result<Vec<u8>>,
    mut read_input: impl FnMut(usize, usize) -> Result<Option<Window>>,
    mut visitor: impl FnMut(Pair<'_>),
) -> Result<()> {
    validate(test)?;
    let reference_size = test.conditions[0]
        .frames
        .first()
        .map(|f| [f.width, f.height]);
    let mut preceding: Option<&Frame> = None;
    for (c, condition) in test.conditions.iter().enumerate() {
        let mut previous: Option<Result<Vec<u8>>> = None;
        for (i, frame) in condition.frames.iter().enumerate() {
            let image = read_image(c, i).and_then(|bytes| {
                let bytes = check_image(bytes, frame)?;
                if preceding.is_some_and(|p| {
                    p.interval.end_us >= frame.interval.begin_us
                        || p.completed_unix_ms > frame.started_unix_ms
                }) {
                    return Err("Condition chronology changed.".into());
                }
                Ok(bytes)
            });
            preceding = Some(frame);
            if let Some(prior) = previous.as_ref() {
                let before = &condition.frames[i - 1];
                let images = (|| -> Result<(&[u8], &[u8])> {
                    if Some([frame.width, frame.height]) != reference_size
                        || [before.width, before.height] != [frame.width, frame.height]
                        || before.interval.clock_id != test.investigation
                        || frame.interval.clock_id != test.investigation
                        || before.interval.end_us >= frame.interval.begin_us
                        || before.completed_unix_ms > frame.started_unix_ms
                    {
                        return Err("Image geometry, clock or chronology changed.".into());
                    }
                    Ok((
                        prior.as_ref().map_err(Clone::clone)?,
                        image.as_ref().map_err(Clone::clone)?,
                    ))
                })();
                let input = read_input(c, i).and_then(|window| {
                    if visual::digest(&serde_json::to_vec(&window).map_err(|e| e.to_string())?)
                        != frame.input_sha256
                    {
                        return Err("Sampled-input checksum changed.".into());
                    }
                    let witness = InputWitness::check(
                        window.as_ref(),
                        i.checked_sub(2).and_then(|a| test.actions.get(a)),
                        &frame.interval,
                        before.interval.end_us,
                    );
                    if serde_json::to_value(&witness).map_err(|e| e.to_string())?
                        != serde_json::to_value(&frame.input).map_err(|e| e.to_string())?
                    {
                        return Err(
                            "Sampled-input summary differs from the retained window.".into()
                        );
                    }
                    Ok(witness)
                });
                visitor(Pair {
                    condition: c,
                    index: i,
                    before,
                    after: frame,
                    images,
                    input,
                });
            }
            previous = Some(image);
        }
    }
    Ok(())
}
