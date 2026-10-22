//! Read-only interpretation of retained, hash-checked images and input windows.
use reversebolt_core::{
    Result,
    live::LiveSession,
    play_evidence::{self, Measurement},
};
use serde_json::{Value, json};

pub(crate) fn measure_pair(
    before: &reversebolt_core::visual::FrameRef,
    after: &reversebolt_core::visual::FrameRef,
    a: &[u8],
    b: &[u8],
) -> (Measurement, Value) {
    let compatible =
        before.width == after.width && before.height == after.height && before.epoch == after.epoch;
    let difference = if compatible {
        reversebolt_vision::response::difference(a, b)
    } else {
        Err("Image geometry or scene epoch changed.".into())
    };
    let spatial = if compatible {
        reversebolt_vision::patch_motion::measure(a, b)
    } else {
        Err("Image geometry or scene epoch changed.".into())
    };
    let measurement = match difference {
        Ok(difference) => Measurement {
            difference: Some(difference),
            error: None,
        },
        Err(error) => Measurement {
            difference: None,
            error: Some(error),
        },
    };
    let (spatial, error) = match spatial {
        Ok(report) => (Some(report), None),
        Err(error) => (None, Some(error)),
    };
    (
        measurement,
        json!({"before":before.id,"after":after.id,"before_sha256":before.sha256,
        "after_sha256":after.sha256,"measurement":spatial,"error":error}),
    )
}

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
    let mut patch_motion = vec![];
    let mut previous: Option<Vec<u8>> = None;
    for id in ids {
        let (frame, image) = session.read_frame(investigation, &id)?;
        if let Some(prior) = &previous {
            let (measurement, spatial) =
                measure_pair(frames.last().unwrap(), &frame, prior, &image);
            measurements.push(measurement);
            patch_motion.push(spatial);
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
        "frame_metadata_sha256":frame_metadata_sha256,"evidence":evidence,
        "patch_motion":{"metric":reversebolt_vision::patch_motion::METRIC,"intervals":patch_motion,
            "limitation":reversebolt_vision::patch_motion::LIMITATION}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_epoch_is_missing_motion_not_a_zero_displacement() {
        let a: reversebolt_core::visual::FrameRef = serde_json::from_value(json!({
            "id":"a","number":1,"sha256":"a".repeat(64),"width":100,"height":100,"epoch":0,"started_unix_ms":1
        })).unwrap();
        let mut b = a.clone();
        b.id = "b".into();
        b.number = 2;
        b.sha256 = "b".repeat(64);
        b.epoch = 1;
        let (difference, motion) = measure_pair(&a, &b, b"not decoded", b"not decoded");
        assert!(difference.difference.is_none());
        assert_eq!(motion["measurement"], Value::Null);
        assert_eq!(motion["before_sha256"], a.sha256);
        assert_eq!(motion["after_sha256"], b.sha256);
        assert_eq!(motion["error"], "Image geometry or scene epoch changed.");
    }
}
