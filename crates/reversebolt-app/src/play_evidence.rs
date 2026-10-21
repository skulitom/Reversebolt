//! Read-only interpretation of retained, hash-checked images and input windows.
use reversebolt_core::{
    Result,
    live::LiveSession,
    play_evidence::{self, Measurement},
};
use serde_json::{Value, json};

pub fn read(session: &LiveSession, args: &Value) -> Result<Value> {
    let investigation = args["investigation_id"]
        .as_str()
        .ok_or("Supply the current investigation identity.")?;
    if args["batch"] != session.view.coverage.batch {
        return Err("The observation batch changed.".into());
    }
    let ids: Vec<String> = serde_json::from_value(args["frame_ids"].clone())
        .map_err(|_| "Supply 2–20 frame identities in chronological order.")?;
    if !(2..=20).contains(&ids.len()) {
        return Err("Choose 2–20 captures.".into());
    }
    let mut frames = vec![];
    let mut measurements = vec![];
    let mut previous: Option<Vec<u8>> = None;
    for id in ids {
        let (frame, image) = session.read_frame(investigation, &id)?;
        if let Some(prior) = &previous {
            let measured = if frames
                .last()
                .is_some_and(|f: &reversebolt_core::visual::FrameRef| {
                    f.width != frame.width || f.height != frame.height || f.epoch != frame.epoch
                }) {
                Err("Image geometry or scene epoch changed.".into())
            } else {
                reversebolt_vision::response::difference(prior, &image)
            };
            measurements.push(match measured {
                Ok(difference) => Measurement {
                    difference: Some(difference),
                    error: None,
                },
                Err(error) => Measurement {
                    difference: None,
                    error: Some(error),
                },
            });
        }
        frames.push(frame);
        previous = Some(image);
    }
    let evidence = play_evidence::analyze(&frames, measurements)?;
    // Bind to both the images and complete sampled-input/timing metadata.
    let frame_metadata_sha256 =
        reversebolt_core::visual::digest(&serde_json::to_vec(&frames).map_err(|e| e.to_string())?);
    Ok(
        json!({"investigation_id":investigation,"batch":session.view.coverage.batch,
        "frame_metadata_sha256":frame_metadata_sha256,"evidence":evidence}),
    )
}
