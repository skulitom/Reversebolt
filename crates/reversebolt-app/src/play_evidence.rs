//! Read-only interpretation of retained, hash-checked images and input windows.
use reversebolt_core::{
    Result,
    live::LiveSession,
    play_evidence::{self, Measurement},
};
use reversebolt_vision::region_motion::{self, Region};
use serde_json::{Value, json};

pub(crate) fn measure_region_pair(
    before: &reversebolt_core::visual::FrameRef,
    after: &reversebolt_core::visual::FrameRef,
    a: &[u8],
    b: &[u8],
    region: Region,
) -> Value {
    let measured = (|| -> Result<_> {
        if before.width != after.width
            || before.height != after.height
            || before.epoch != after.epoch
        {
            return Err("Image geometry or scene epoch changed.".into());
        }
        if reversebolt_core::visual::digest(a) != before.sha256
            || reversebolt_core::visual::digest(b) != after.sha256
        {
            return Err("Image checksum changed.".into());
        }
        let report = region_motion::measure(a, b, region)?;
        if report.source_dimensions != [before.width, before.height] {
            return Err("PNG dimensions differ from the retained image metadata.".into());
        }
        Ok(report)
    })();
    let (measurement, error) = match measured {
        Ok(report) => (Some(report), None),
        Err(error) => (None, Some(error)),
    };
    json!({"before":before.id,"after":after.id,"before_sha256":before.sha256,"after_sha256":after.sha256,
        "measurement":measurement,"error":error})
}

pub(crate) fn region_evidence(
    frames: &[reversebolt_core::visual::FrameRef],
    region: Region,
    intervals: Vec<Value>,
) -> Result<Value> {
    let source = json!({"frames":frames,"region":region,"policy":region_motion::POLICY,"metric":reversebolt_vision::patch_motion::METRIC});
    Ok(
        json!({"policy":region_motion::POLICY,"metric":reversebolt_vision::patch_motion::METRIC,"region":region,
        "source_sha256":reversebolt_core::visual::digest(&serde_json::to_vec(&source).map_err(|e|e.to_string())?),
        "intervals":intervals,"limitation":region_motion::LIMITATION}),
    )
}

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
    let region: Option<Region> = args
        .get("region")
        .map(|r| {
            serde_json::from_value(r.clone()).map_err(|_| {
                "Supply integer x, y, width and height for one fixed image region.".to_string()
            })
        })
        .transpose()?;
    if let Some(region) = region {
        region.validate_shape()?;
    }
    let mut frames = vec![];
    let mut measurements = vec![];
    let mut patch_motion = vec![];
    let mut previous: Option<Vec<u8>> = None;
    let mut region_intervals = vec![];
    for id in ids {
        let (frame, image) = session.read_frame(investigation, &id)?;
        if let Some(prior) = &previous {
            let (measurement, spatial) =
                measure_pair(frames.last().unwrap(), &frame, prior, &image);
            measurements.push(measurement);
            patch_motion.push(spatial);
            if let Some(region) = region {
                region_intervals.push(measure_region_pair(
                    frames.last().unwrap(),
                    &frame,
                    prior,
                    &image,
                    region,
                ));
            }
        }
        frames.push(frame);
        previous = Some(image);
    }
    let evidence = play_evidence::analyze(&frames, measurements)?;
    // Bind to both the images and complete sampled-input/timing metadata.
    let frame_metadata_sha256 =
        reversebolt_core::visual::digest(&serde_json::to_vec(&frames).map_err(|e| e.to_string())?);
    let mut result = json!({"investigation_id":investigation,"batch":session.view.coverage.batch,
        "frame_metadata_sha256":frame_metadata_sha256,"evidence":evidence,
        "patch_motion":{"metric":reversebolt_vision::patch_motion::METRIC,"intervals":patch_motion,
            "limitation":reversebolt_vision::patch_motion::LIMITATION}});
    if let Some(region) = region {
        result["region_motion"] = region_evidence(&frames, region, region_intervals)?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_measurements_bind_pixels_geometry_epoch_and_region_coordinates() {
        let mut encoded = std::io::Cursor::new(vec![]);
        image::GrayImage::from_pixel(256, 144, image::Luma([80]))
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let bytes = encoded.into_inner();
        let before: reversebolt_core::visual::FrameRef=serde_json::from_value(json!({
            "id":"a","number":1,"sha256":reversebolt_core::visual::digest(&bytes),"width":256,"height":144,"epoch":0,"started_unix_ms":1
        })).unwrap();
        let mut after = before.clone();
        after.id = "b".into();
        after.number = 2;
        after.started_unix_ms = 2;
        let region = Region {
            x: 16,
            y: 16,
            width: 128,
            height: 72,
        };
        let measured = measure_region_pair(&before, &after, &bytes, &bytes, region);
        assert!(measured["error"].is_null());
        assert_eq!(measured["measurement"]["region"], json!(region));
        assert_eq!(
            measured["measurement"]["source_dimensions"],
            json!([256, 144])
        );
        assert_eq!(
            measured["measurement"]["displacement_to_source_scale"],
            json!([0.5, 0.5])
        );
        assert!(measured["measurement"]["measurement"]["consensus"].is_null());
        let frames = [before.clone(), after.clone()];
        let scope = region_evidence(&frames, region, vec![measured]).unwrap();
        let other = region_evidence(&frames, Region { x: 18, ..region }, vec![]).unwrap();
        assert_ne!(scope["source_sha256"], other["source_sha256"]);
        for kind in 0..4 {
            let (mut a, mut b) = (before.clone(), after.clone());
            match kind {
                0 => b.sha256 = "0".repeat(64),
                1 => b.epoch = 1,
                2 => b.width = 255,
                _ => {
                    a.width = 255;
                    b.width = 255;
                }
            }
            let refused = measure_region_pair(&a, &b, &bytes, &bytes, region);
            assert!(refused["measurement"].is_null());
            assert!(refused["error"].is_string());
        }
        let invalid =
            measure_region_pair(&before, &after, &bytes, &bytes, Region { x: 250, ..region });
        assert!(invalid["measurement"].is_null());
        assert!(invalid["error"].as_str().unwrap().contains("outside"));
    }
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
