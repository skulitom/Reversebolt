//! Read-only interpretation of retained, hash-checked images and input windows.
use reversebolt_core::visual_regions;
use reversebolt_core::{
    Result,
    live::LiveSession,
    play_evidence::{self, Measurement},
};
use reversebolt_vision::region_motion::{self, Region};
use serde_json::{Value, json};

fn check_pair(
    before: &reversebolt_core::visual::FrameRef,
    after: &reversebolt_core::visual::FrameRef,
    a: &[u8],
    b: &[u8],
) -> Result<()> {
    if before.width != after.width || before.height != after.height || before.epoch != after.epoch {
        return Err("Image geometry or scene epoch changed.".into());
    }
    if reversebolt_core::visual::digest(a) != before.sha256
        || reversebolt_core::visual::digest(b) != after.sha256
    {
        return Err("Image checksum changed.".into());
    }
    Ok(())
}

pub(crate) fn measure_appearance_pair(
    before: &reversebolt_core::visual::FrameRef,
    after: &reversebolt_core::visual::FrameRef,
    a: &[u8],
    b: &[u8],
    region: Region,
) -> Value {
    let measured = (|| -> Result<_> {
        check_pair(before, after, a, b)?;
        let report = reversebolt_vision::region_appearance::measure(a, b, region)?;
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

pub(crate) fn measure_region_pair(
    before: &reversebolt_core::visual::FrameRef,
    after: &reversebolt_core::visual::FrameRef,
    a: &[u8],
    b: &[u8],
    region: Region,
) -> Value {
    let measured = (|| -> Result<_> {
        check_pair(before, after, a, b)?;
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

pub(crate) fn automatic_interval(
    selection: &visual_regions::Selection,
    before: &reversebolt_core::visual::FrameRef,
    after: &reversebolt_core::visual::FrameRef,
    a: &[u8],
    b: &[u8],
) -> Vec<Value> {
    selection.regions.iter().filter(|p|p.before_id==before.id && p.after_id==after.id).map(|p|
        json!({"source_sha256":p.source_sha256,"result":measure_region_pair(before,after,a,b,p.region),
            "appearance_match":measure_appearance_pair(before,after,a,b,p.region)})
    ).collect()
}

pub(crate) fn automatic_sequences(
    investigation: &str,
    frames: &[reversebolt_core::visual::FrameRef],
    tracks: &[reversebolt_core::visual::VisualTrack],
    read_image: impl FnMut(&reversebolt_core::visual::FrameRef) -> Result<Vec<u8>>,
    activity: Option<&reversebolt_core::play_context::Source>,
) -> Value {
    let report = match activity {
        Some(source) => reversebolt_vision::appearance_sequence::analyze_for_activity(
            investigation,
            frames,
            tracks,
            read_image,
            source,
        ),
        None => reversebolt_vision::appearance_sequence::analyze(
            investigation,
            frames,
            tracks,
            read_image,
        ),
    };
    match report {
        Ok(report) => {
            let mut value = json!({"report":report,"error":null});
            if let Some(source) = activity {
                value["activity"] = match reversebolt_vision::appearance_activity::summarize(
                    &report, source, frames,
                ) {
                    Ok(summary) => json!({"summary":summary,"error":null}),
                    Err(error) => json!({"summary":null,"error":error}),
                };
            }
            value
        }
        Err(error) => json!({"report":null,"error":error}),
    }
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

fn regional_suggestions(
    source: &reversebolt_core::play_context::Source,
    frames: &[reversebolt_core::visual::FrameRef],
    read: &mut impl FnMut(&reversebolt_core::visual::FrameRef) -> Result<Vec<u8>>,
) -> Value {
    match reversebolt_vision::activity_region_suggestions::analyze(source, frames, read) {
        Ok(report) => {
            match reversebolt_vision::activity_region_summary::summarize(&report, source, frames) {
                Ok(summary) => json!({"report":report,"summary":summary,"error":null}),
                Err(error) => {
                    json!({"report":report,"summary":null,"summary_error":error,"error":null})
                }
            }
        }
        Err(error) => json!({"report":null,"error":error}),
    }
}

/// Existing whole-image selection wins. Otherwise inspect at most four sampled
/// keys with the unchanged twenty-check regional matcher for each candidate.
fn select_activity(
    frames: &[reversebolt_core::visual::FrameRef],
    evidence: &play_evidence::Evidence,
    investigation: &str,
    batch: usize,
    mut read: impl FnMut(&reversebolt_core::visual::FrameRef) -> Result<Vec<u8>>,
) -> Result<(
    Option<reversebolt_core::play_context::Source>,
    Value,
    Option<Value>,
)> {
    use reversebolt_core::play_context;
    if let Some(source) = play_context::select(frames, evidence, investigation, batch)? {
        let suggestions = regional_suggestions(&source, frames, &mut read);
        return Ok((Some(source), suggestions, None));
    }
    let mut attempts = vec![];
    let mut selected = None;
    let mut suggestions = json!({"report":null,"error":null,"status":"no_activity_source"});
    for source in play_context::regional_candidates(frames, evidence, investigation, batch)? {
        let result = regional_suggestions(&source, frames, &mut read);
        let supported = serde_json::from_value::<reversebolt_core::activity_regions::Summary>(
            result["summary"].clone(),
        )
        .ok()
        .is_some_and(|summary| summary.supports(&source).unwrap_or(false));
        attempts.push(json!({"source":source,"supported":supported,"result":result}));
        if supported {
            suggestions = result;
            selected = Some(source);
            break;
        }
    }
    Ok((
        selected,
        suggestions,
        Some(json!({"policy":"regional_activity_selection_v1",
        "maximum_candidates":4,"maximum_template_checks":80,"attempts":attempts})),
    ))
}

/// Require current engine-measured qualification when a plan uses regional activity.
pub(crate) fn check_activity_context(
    context: &reversebolt_core::play_context::Context,
    result: &Value,
) -> Result<()> {
    if context.source.selection_basis
        == reversebolt_core::play_context::SelectionBasis::RegionalCandidateV1
        && (result["activity_source"] != json!(context.source)
            || result["automatic_region_motion"]["activity_region_suggestions"]["summary"]
                != json!(context.regions))
    {
        return Err("The regional activity source or its supporting measurements changed.".into());
    }
    Ok(())
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
    let auto_regions = match args.get("auto_regions") {
        None => true,
        Some(value) => value
            .as_bool()
            .ok_or("auto_regions must be true or false.")?,
    };
    let automatic = if auto_regions {
        let references = ids
            .iter()
            .map(|id| {
                session
                    .view
                    .frames
                    .iter()
                    .find(|f| &f.id == id)
                    .cloned()
                    .ok_or("Choose retained image identities.".to_string())
            })
            .collect::<Result<Vec<_>>>()?;
        Some(visual_regions::select(
            investigation,
            &references,
            &session.view.visual_tracks,
        ))
    } else {
        None
    };
    let mut automatic_measurements = vec![];
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
            if let Some(Ok(selection)) = &automatic {
                automatic_measurements.extend(automatic_interval(
                    selection,
                    frames.last().unwrap(),
                    &frame,
                    prior,
                    &image,
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
    result["activity_source"] = json!(
        reversebolt_core::play_context::select(
            &frames,
            &evidence,
            investigation,
            session.view.coverage.batch
        )
        .ok()
        .flatten()
    );
    if let Some(region) = region {
        result["region_motion"] = region_evidence(&frames, region, region_intervals)?;
    }
    if let Some(automatic) = automatic {
        result["automatic_region_motion"] = match automatic {
            Ok(selection) => {
                let selected = select_activity(
                    &frames,
                    &evidence,
                    investigation,
                    session.view.coverage.batch,
                    |frame| {
                        session
                            .read_frame(investigation, &frame.id)
                            .map(|(_, bytes)| bytes)
                    },
                );
                let (activity, suggestions, search) = match selected {
                    Ok(selected) => selected,
                    Err(error) => (None, json!({"report":null,"error":error}), None),
                };
                result["activity_source"] = json!(activity);
                let sequences = automatic_sequences(
                    investigation,
                    &frames,
                    &session.view.visual_tracks,
                    |frame| {
                        session
                            .read_frame(investigation, &frame.id)
                            .map(|(_, bytes)| bytes)
                    },
                    activity.as_ref(),
                );
                let mut automatic = json!({"selection":selection,"measurements":automatic_measurements,"appearance_sequences":sequences,"activity_region_suggestions":suggestions,"error":null});
                if let Some(search) = search {
                    automatic["regional_activity_search"] = search;
                }
                automatic
            }
            Err(error) => json!({"selection":null,"measurements":[],"error":error}),
        };
    }
    Ok(result)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use reversebolt_core::{
        live::{NativeTarget, ProcessInfo, Screen},
        visual,
    };
    use std::path::{Path, PathBuf};

    struct NoTargetAccess(ProcessInfo);
    impl NativeTarget for NoTargetAccess {
        fn info(&self) -> &ProcessInfo {
            &self.0
        }
        fn regions(&self) -> Result<Vec<reversebolt_core::live::Region>> {
            Ok(vec![reversebolt_core::live::Region {
                base: 0x1000,
                length: 4096,
                ..Default::default()
            }])
        }
        fn read(&self, _: u64, _: usize) -> Result<Vec<u8>> {
            panic!("saved evidence must not read a target")
        }
        fn screen(&self, _: &Path) -> Result<Screen> {
            panic!("saved evidence must not capture")
        }
        fn alive(&self) -> bool {
            true
        }
    }
    pub(crate) fn saved_observations() -> (LiveSession, PathBuf) {
        let directory = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("reversebolt-regions-{}", uuid::Uuid::new_v4()));
        let mut session = LiveSession::attach(
            Box::new(NoTargetAccess(ProcessInfo::default())),
            1,
            directory.clone(),
        )
        .unwrap();
        let mut encoded = std::io::Cursor::new(vec![]);
        image::GrayImage::from_pixel(256, 144, image::Luma([80]))
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let bytes = encoded.into_inner();
        session.view.frames = (1..=3)
            .map(|n| {
                std::fs::write(directory.join(format!("frame-{n:03}.png")), &bytes).unwrap();
                serde_json::from_value(
                    json!({"id":format!("frame-{n}"),"number":n,"sha256":visual::digest(&bytes),
                "width":256,"height":144,"epoch":0,"started_unix_ms":n}),
                )
                .unwrap()
            })
            .collect();
        session.view.sample_count = 3;
        for index in [0, 2] {
            let frame = &session.view.frames[index];
            visual::record(
                &mut session.view.visual_tracks,
                visual::ObservationInput {
                    investigation_id: session.view.investigation_id.clone(),
                    frame_id: frame.id.clone(),
                    image_sha256: frame.sha256.clone(),
                    observer: "fixture".into(),
                    key: format!("object-{index}"),
                    label: "Unverified object".into(),
                    unit: String::new(),
                    value: Some(visual::VisualValue::Text("visible".into())),
                    quality: visual::Quality::Provisional,
                    bounds: Some([16., 16., 128., 72.]),
                    explanation: "A fixture image region with no gameplay meaning.".into(),
                },
                frame,
                "fixture-observer",
                3,
            )
            .unwrap();
        }
        (session, directory)
    }
    fn generated_activity_session(local: bool) -> (LiveSession, PathBuf) {
        use reversebolt_core::inputs::{Focus, Poll, Window};
        let (mut session, directory) = saved_observations();
        session.view.visual_tracks.clear();
        session.view.frames.clear();
        for (i, shift) in [0, 0, 8, 8, 0, 0, 8, 8].into_iter().enumerate() {
            let n = i + 1;
            let image = image::GrayImage::from_fn(256, 144, |x, y| {
                let mut v = ((x + 256 - shift) % 256 + y * 256 + 17)
                    .wrapping_mul(747796405)
                    .wrapping_add(2891336453);
                v = ((v >> ((v >> 28) + 4)) ^ v).wrapping_mul(277803737);
                image::Luma([((v >> 22) ^ v) as u8])
            });
            let image = if local {
                let mut background = image::GrayImage::from_pixel(512, 288, image::Luma([20]));
                let subject = image::GrayImage::from_fn(64, 64, |x, y| {
                    let mut v = (x + y * 64 + 17)
                        .wrapping_mul(747796405)
                        .wrapping_add(2891336453);
                    v = ((v >> ((v >> 28) + 4)) ^ v).wrapping_mul(277803737);
                    image::Luma([40 + (((v >> 22) ^ v) % 140) as u8])
                });
                image::imageops::replace(&mut background, &subject, i64::from(64 + shift), 80);
                background
            } else {
                image
            };
            let dimensions = image.dimensions();
            let mut bytes = std::io::Cursor::new(vec![]);
            image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
            let bytes = bytes.into_inner();
            std::fs::write(directory.join(format!("frame-{n:03}.png")), &bytes).unwrap();
            let begin = n as u64 * 1_000_000;
            let prior = i as u64 * 1_000_000 + 1000;
            let input = Window {
                clock_id: session.view.investigation_id.clone(),
                generation: "sampler".into(),
                requested_begin_us: prior,
                requested_end_us: begin + 1000,
                monitor_started_us: 0,
                retained_from_us: 0,
                omitted_polls: 0,
                largest_unsampled_us: if local { 19_900 } else { 20_000 },
                polls: (prior..begin)
                    .step_by(20_000)
                    .enumerate()
                    .map(|(k, t)| Poll {
                        begin_us: t,
                        end_us: t + 100,
                        focus: Focus::SelectedForeground,
                        down: if [2, 6].contains(&i)
                            && if local { (10..20).contains(&k) } else { k < 10 }
                        {
                            vec![68]
                        } else {
                            vec![]
                        },
                    })
                    .collect(),
                limitation: "Sampled input fixture.".into(),
            };
            session.view.frames.push(serde_json::from_value(json!({
                "id":format!("frame-{n}"),"number":n,"sha256":visual::digest(&bytes),
                "width":dimensions.0,"height":dimensions.1,"epoch":0,"started_unix_ms":n,"input_context":input,
                "capture_interval":{"clock_id":session.view.investigation_id,"begin_us":begin,"end_us":begin+1000}
            })).unwrap());
        }
        session.view.sample_count = 8;
        session.view.coverage.batch = 1;
        (session, directory)
    }

    #[test]
    fn small_local_activity_reaches_interpretation_and_planning_without_global_contrast() {
        use reversebolt_core::{play_context as pc, runtime_planning as planning};
        let (session, directory) = generated_activity_session(true);
        let args = json!({"investigation_id":session.view.investigation_id,"batch":1,
            "frame_ids":session.view.frames.iter().map(|f|&f.id).collect::<Vec<_>>()});
        let before = json!(session.view);
        let result = read(&session, &args).unwrap();
        let evidence: play_evidence::Evidence =
            serde_json::from_value(result["evidence"].clone()).unwrap();
        assert!(
            evidence
                .intervals
                .iter()
                .all(|i| i.measurement.difference.unwrap() < 5.)
        );
        assert!(
            pc::select(
                &session.view.frames,
                &evidence,
                &session.view.investigation_id,
                1
            )
            .unwrap()
            .is_none()
        );
        let source: pc::Source = serde_json::from_value(result["activity_source"].clone()).unwrap();
        assert_eq!(
            source.selection_basis,
            pc::SelectionBasis::RegionalCandidateV1
        );
        let search = &result["automatic_region_motion"]["regional_activity_search"];
        assert_eq!(search["attempts"].as_array().unwrap().len(), 1);
        assert_eq!(search["maximum_template_checks"], 80);
        let prepared = crate::runtime_naming_bridge::prepare_activity(&session, &directory)
            .unwrap()
            .unwrap();
        prepared.validate().unwrap();
        assert_eq!(prepared.source, source);
        let observation = pc::Observation {
            consistent_visible_change: false,
            description: "A small area may have moved.".into(),
            uncertainty: "Synthetic images; no gameplay meaning.".into(),
        };
        assert!(pc::Context::from_observation(source.clone(), observation.clone()).is_err());
        let context =
            pc::Context::from_regional_observation(source, observation, prepared.regions.clone())
                .unwrap();
        check_activity_context(&context, &result).unwrap();
        let plan = planning::actions(&context, &session.view.frames, &evidence).unwrap();
        assert_eq!(plan.episodes.len(), 2);
        assert_eq!(plan.episode_order, Some(vec![0, 1, 0, 1]));
        assert_eq!(plan.actions.len(), 4);
        assert!(
            plan.actions
                .iter()
                .all(|a| a.keys == [68] && (180..=220).contains(&a.hold_ms))
        );
        assert!(!context.observation.consistent_visible_change);
        let roundtrip: pc::Context = serde_json::from_value(json!(context)).unwrap();
        assert_eq!(
            planning::actions(&roundtrip, &session.view.frames, &evidence).unwrap(),
            plan
        );
        let mut missing = context.clone();
        missing.regions = None;
        assert!(planning::actions(&missing, &session.view.frames, &evidence).is_err());
        let mut one_pair = context.clone();
        for region in &mut one_pair.regions.as_mut().unwrap().regions {
            if region.pairs[1].displacement.is_some() {
                region.pairs[1].displacement = Some([0., 0.]);
                region.pairs[1].after_region = region.pairs[1].before_region;
            }
        }
        assert!(one_pair.validate().is_err());
        let mut altered = result.clone();
        altered["automatic_region_motion"]["activity_region_suggestions"]["summary"] = Value::Null;
        assert!(check_activity_context(&context, &altered).is_err());
        let mut frames = session.view.frames.clone();
        frames[2].input_context = None;
        assert!(planning::actions(&context, &frames, &evidence).is_err());
        // Static images and unreadable evidence cannot qualify prospective input.
        let static_bytes = session
            .read_frame(&session.view.investigation_id, &session.view.frames[0].id)
            .unwrap()
            .1;
        let mut static_frames = session.view.frames.clone();
        for f in &mut static_frames {
            f.sha256 = reversebolt_core::visual::digest(&static_bytes);
        }
        let static_evidence = play_evidence::analyze(
            &static_frames,
            (1..static_frames.len())
                .map(|_| Measurement {
                    difference: Some(0.),
                    error: None,
                })
                .collect(),
        )
        .unwrap();
        let (none, _, audit) = select_activity(
            &static_frames,
            &static_evidence,
            &session.view.investigation_id,
            1,
            |_| Ok(static_bytes.clone()),
        )
        .unwrap();
        assert!(none.is_none());
        assert_eq!(audit.unwrap()["attempts"][0]["supported"], false);
        let (none, _, audit) = select_activity(
            &session.view.frames,
            &evidence,
            &session.view.investigation_id,
            1,
            |_| Err("Unreadable".into()),
        )
        .unwrap();
        assert!(none.is_none());
        assert_eq!(
            audit.unwrap()["attempts"][0]["result"]["error"],
            "Unreadable"
        );
        assert_eq!(json!(session.view), before);
        drop(session);
        assert!(directory.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            directory
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("reversebolt-regions-")
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn passive_suggestions_need_no_observer_labels_and_leave_session_unchanged() {
        let (session, directory) = generated_activity_session(false);
        let before = json!(session.view);
        let mut args = json!({"investigation_id":session.view.investigation_id,
            "batch":session.view.coverage.batch,
            "frame_ids":session.view.frames.iter().map(|f|&f.id).collect::<Vec<_>>()});
        let enabled = read(&session, &args).unwrap();
        let suggestions = &enabled["automatic_region_motion"]["activity_region_suggestions"];
        assert!(suggestions["error"].is_null());
        assert_eq!(
            suggestions["report"]["policy"],
            reversebolt_vision::activity_region_suggestions::POLICY
        );
        assert_eq!(suggestions["report"]["origin_frame_id"], "frame-2");
        assert!(suggestions["summary_error"].is_null());
        assert_eq!(
            suggestions["summary"]["policy"],
            reversebolt_core::activity_regions::POLICY
        );
        let prepared = crate::runtime_naming_bridge::prepare_activity(&session, &directory)
            .unwrap()
            .unwrap();
        assert_eq!(json!(prepared.regions), suggestions["summary"]);
        prepared.validate().unwrap();
        assert!(suggestions["report"]["attempted_frames"].as_u64().unwrap() <= 20);
        assert_eq!(
            suggestions["report"]["checked_frames"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        assert!(
            enabled["automatic_region_motion"]["selection"]["regions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        args["auto_regions"] = json!(false);
        let disabled = read(&session, &args).unwrap();
        assert!(disabled.get("automatic_region_motion").is_none());
        for key in ["evidence", "patch_motion", "frame_metadata_sha256"] {
            assert_eq!(enabled[key], disabled[key]);
        }
        assert_eq!(json!(session.view), before);
        drop(session);
        assert!(directory.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            directory
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("reversebolt-regions-")
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn saved_reader_automatically_measures_exact_observations_without_changing_base_evidence() {
        let (mut session, directory) = saved_observations();
        let mut args = json!({"investigation_id":session.view.investigation_id,"batch":session.view.coverage.batch,
            "frame_ids":session.view.frames.iter().map(|f|&f.id).collect::<Vec<_>>()});
        let enabled = read(&session, &args).unwrap();
        let automatic = &enabled["automatic_region_motion"];
        assert!(automatic["error"].is_null());
        assert_eq!(
            automatic["selection"]["regions"].as_array().unwrap().len(),
            1
        );
        assert_eq!(
            automatic["selection"]["omitted_observations"]["no_following_image"],
            1
        );
        assert_eq!(automatic["measurements"].as_array().unwrap().len(), 1);
        let measured = &automatic["measurements"][0];
        assert_eq!(
            measured["source_sha256"],
            automatic["selection"]["regions"][0]["source_sha256"]
        );
        assert!(measured["result"]["error"].is_null());
        assert_eq!(measured["result"]["before"], "frame-1");
        assert_eq!(measured["result"]["after"], "frame-2");
        assert_eq!(
            measured["appearance_match"]["measurement"]["status"],
            "unsupported_size"
        );
        assert!(measured["appearance_match"]["measurement"]["displacement"].is_null());
        assert_eq!(
            measured["appearance_match"]["before_sha256"],
            measured["result"]["before_sha256"]
        );
        let sequences = &automatic["appearance_sequences"];
        assert!(sequences["error"].is_null());
        assert_eq!(sequences["report"]["attempted_pairs"], 1);
        assert_eq!(
            sequences["report"]["series"][0]["stop"],
            "unresolved_appearance"
        );
        assert_eq!(sequences["report"]["series"][0]["accepted_steps"], 0);
        assert_eq!(
            automatic["activity_region_suggestions"]["status"],
            "no_activity_source"
        );
        args["auto_regions"] = json!(false);
        let disabled = read(&session, &args).unwrap();
        assert!(disabled.get("automatic_region_motion").is_none());
        for key in ["evidence", "patch_motion", "frame_metadata_sha256"] {
            assert_eq!(enabled[key], disabled[key]);
        }
        args["auto_regions"] = json!("true");
        assert!(read(&session, &args).unwrap_err().contains("auto_regions"));
        args["auto_regions"] = json!(true);
        session.view.visual_tracks[0].observations[0]
            .as_mut()
            .unwrap()
            .input
            .image_sha256 = "0".repeat(64);
        let invalid = read(&session, &args).unwrap();
        assert_eq!(
            invalid["automatic_region_motion"]["selection"]["omitted_observations"]["invalid_source_binding"],
            1
        );
        assert!(
            invalid["automatic_region_motion"]["measurements"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(enabled["evidence"], invalid["evidence"]);
        // The existing whole-request saved-pixel check still fails closed.
        let corrupt = image::GrayImage::from_pixel(256, 144, image::Luma([90]));
        corrupt.save(directory.join("frame-002.png")).unwrap();
        assert!(
            read(&session, &args)
                .unwrap_err()
                .contains("saved image has changed")
        );
        drop(session);
        std::fs::remove_dir_all(directory).unwrap();
    }
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
            let appearance = measure_appearance_pair(&a, &b, &bytes, &bytes, region);
            assert!(appearance["measurement"].is_null());
            assert_eq!(appearance["error"], refused["error"]);
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
