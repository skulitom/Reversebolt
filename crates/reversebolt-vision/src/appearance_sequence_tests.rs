use super::*;
use image::{GrayImage, Luma, imageops};
use reversebolt_core::{
    inputs::{Focus, Poll, Window},
    visual::{CaptureInterval, ObservationInput, Quality, VisualValue},
};

fn texture(seed: u32) -> GrayImage {
    GrayImage::from_fn(64, 64, |x, y| {
        let mut n = (x + y * 64)
            .wrapping_add(seed)
            .wrapping_mul(747796405)
            .wrapping_add(2891336453);
        n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
        Luma([40 + (((n >> 22) ^ n) % 140) as u8])
    })
}
fn png(patch: &GrayImage, at: [u32; 2]) -> Vec<u8> {
    let mut frame = GrayImage::from_pixel(512, 288, Luma([20]));
    imageops::replace(&mut frame, patch, i64::from(at[0]), i64::from(at[1]));
    let mut bytes = std::io::Cursor::new(vec![]);
    frame.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}
fn fixture(images: &[Vec<u8>]) -> (Vec<FrameRef>, Vec<VisualTrack>) {
    let frames = images
        .iter()
        .enumerate()
        .map(|(i, bytes)| {
            let n = i + 1;
            let begin = n as u64 * 1_000_000;
            let end = begin + 1000;
            let prior = (n - 1) as u64 * 1_000_000 + 1000;
            FrameRef {
                sampling: None,
                input_context: Some(Window {
                    clock_id: "clock".into(),
                    generation: "sampler".into(),
                    requested_begin_us: prior,
                    requested_end_us: end,
                    monitor_started_us: 0,
                    retained_from_us: 0,
                    omitted_polls: 0,
                    largest_unsampled_us: 20_000,
                    polls: (prior..begin)
                        .step_by(20_000)
                        .enumerate()
                        .map(|(i, t)| Poll {
                            begin_us: t,
                            end_us: t + 100,
                            focus: Focus::SelectedForeground,
                            down: if n % 2 == 0 && i < 10 {
                                vec![68]
                            } else {
                                vec![]
                            },
                        })
                        .collect(),
                    limitation: "Sampled input fixture.".into(),
                }),
                id: format!("f{n}"),
                number: n,
                sha256: visual::digest(bytes),
                width: 512,
                height: 288,
                epoch: 0,
                started_unix_ms: n as u128,
                capture_interval: Some(CaptureInterval {
                    clock_id: "clock".into(),
                    begin_us: begin,
                    end_us: end,
                }),
            }
        })
        .collect::<Vec<_>>();
    let mut tracks = vec![];
    visual::record(
        &mut tracks,
        ObservationInput {
            investigation_id: "inv".into(),
            frame_id: frames[0].id.clone(),
            image_sha256: frames[0].sha256.clone(),
            observer: "fixture".into(),
            key: "appearance".into(),
            label: "Observed appearance".into(),
            unit: String::new(),
            value: Some(VisualValue::Text("image region".into())),
            quality: Quality::Provisional,
            bounds: Some([64., 80., 64., 64.]),
            explanation: "A fixture appearance with no verified gameplay meaning.".into(),
        },
        &frames[0],
        "fixture",
        frames.len(),
    )
    .unwrap();
    (frames, tracks)
}
fn run(frames: &[FrameRef], tracks: &[VisualTrack], images: &[Vec<u8>]) -> Report {
    analyze("inv", frames, tracks, |f| Ok(images[f.number - 1].clone())).unwrap()
}

fn activity_source(
    frames: &[FrameRef],
    indices: [usize; 2],
) -> reversebolt_core::play_context::Source {
    use reversebolt_core::play_context::{Frame, Pair, Source};
    let cite = |f: &FrameRef| Frame {
        id: f.id.clone(),
        sha256: f.sha256.clone(),
        number: f.number,
        width: f.width,
        height: f.height,
        interval: f.capture_interval.clone().unwrap(),
    };
    Source {
        selection_basis: Default::default(),
        investigation_id: frames[0]
            .capture_interval
            .as_ref()
            .unwrap()
            .clock_id
            .clone(),
        batch: 1,
        key: 68,
        quiet_difference: 0.,
        frame_metadata_sha256: visual::digest(&serde_json::to_vec(frames).unwrap()),
        pairs: indices.map(|i| Pair {
            episode: i,
            before: cite(&frames[i]),
            after: cite(&frames[i + 1]),
            difference: 20.,
            input_sha256: visual::digest(
                &serde_json::to_vec(&frames[i + 1].input_context).unwrap(),
            ),
        }),
    }
}

fn activity_fixture(images: &[Vec<u8>]) -> (Vec<FrameRef>, Vec<VisualTrack>) {
    let (mut frames, tracks) = fixture(images);
    for frame in &mut frames {
        frame.capture_interval.as_mut().unwrap().clock_id = "inv".into();
        frame.input_context.as_mut().unwrap().clock_id = "inv".into();
    }
    (frames, tracks)
}

#[test]
fn activity_horizon_reaches_both_selected_intervals_without_reading_beyond_them() {
    let patch = texture(17);
    let images: Vec<_> = (0..12).map(|i| png(&patch, [64 + i * 8, 80])).collect();
    let (frames, tracks) = activity_fixture(&images);
    let source = activity_source(&frames, [0, 8]);
    let old = run(&frames, &tracks, &images);
    assert_eq!(old.series[0].accepted_steps, 6);
    assert!(json!(old).get("target").is_none());
    let mut reads = vec![];
    let report = analyze_for_activity(
        "inv",
        &frames,
        &tracks,
        |f| {
            reads.push(f.number);
            Ok(images[f.number - 1].clone())
        },
        &source,
    )
    .unwrap();
    assert_eq!(report.attempted_pairs, 9);
    assert_eq!(report.series[0].accepted_steps, 9);
    assert_eq!(report.series[0].stop, Stop::ActivityHorizonReached);
    assert_eq!(report.series[0].stopped_before.as_deref(), Some("f11"));
    assert!(reads.iter().all(|n| *n <= 10));
    assert_eq!(report.target.as_ref().unwrap().last_frame_id, "f10");
    // The new scheduling policy changes provenance, never the shared prefix's pixels/matches.
    for (a, b) in old.series[0].steps.iter().zip(&report.series[0].steps) {
        let (mut a, mut b) = (json!(a), json!(b));
        a.as_object_mut().unwrap().remove("source_sha256");
        b.as_object_mut().unwrap().remove("source_sha256");
        assert_eq!(a, b);
    }
    let summary = crate::appearance_activity::summarize(&report, &source, &frames).unwrap();
    assert_eq!(summary.sequence_policy, ACTIVITY_POLICY);
    assert!(
        summary.appearances[0]
            .pairs
            .iter()
            .all(|p| p.displacement == Some([8., 0.]))
    );
    let mut changed = source.clone();
    changed.batch += 1;
    assert!(crate::appearance_activity::summarize(&report, &changed, &frames).is_err());
    let mut changed = source.clone();
    changed.key = 65;
    assert!(
        analyze_for_activity(
            "inv",
            &frames,
            &tracks,
            |_| panic!("reject input rebinding before reading images"),
            &changed
        )
        .is_err()
    );
    let mut changed = source.clone();
    changed.frame_metadata_sha256 = "e".repeat(64);
    assert!(
        analyze_for_activity(
            "inv",
            &frames,
            &tracks,
            |_| panic!("reject metadata rebinding before reading images"),
            &changed
        )
        .is_err()
    );
    for changed in [
        {
            let mut s = source.clone();
            s.pairs[0].before.id = "missing".into();
            s
        },
        {
            let mut s = source.clone();
            s.pairs[0].before.sha256 = "f".repeat(64);
            s
        },
    ] {
        assert!(
            analyze_for_activity(
                "inv",
                &frames,
                &tracks,
                |_| panic!("reject changed selected images before reading pixels"),
                &changed
            )
            .is_err()
        );
    }
}

#[test]
fn activity_horizon_preserves_budget_and_reports_uncovered_second_interval() {
    let images = vec![png(&texture(17), [64, 80]); 16];
    let (frames, tracks) = activity_fixture(&images);
    let source = activity_source(&frames, [0, 14]);
    let report = analyze_for_activity(
        "inv",
        &frames,
        &tracks,
        |f| Ok(images[f.number - 1].clone()),
        &source,
    )
    .unwrap();
    assert_eq!(report.maximum_attempted_pairs, 12);
    assert_eq!(report.attempted_pairs, 12);
    assert_eq!(report.series[0].accepted_steps, 12);
    assert_eq!(report.series[0].stop, Stop::AnalysisBudget);
    assert_eq!(report.series[0].stopped_before.as_deref(), Some("f14"));
    let summary = crate::appearance_activity::summarize(&report, &source, &frames).unwrap();
    assert_eq!(
        summary.appearances[0].pairs[1].status,
        reversebolt_core::appearance_activity::Status::NotMeasured
    );
    assert_eq!(summary.appearances[0].pairs[1].displacement, None);
}

#[test]
fn activity_horizon_never_skips_a_failed_or_missing_intermediate_capture() {
    for change in 0..4 {
        let mut images = vec![png(&texture(17), [64, 80]); 12];
        if change == 0 {
            images[7] = png(&texture(999), [64, 80]);
        }
        let (mut frames, tracks) = activity_fixture(&images);
        let expected = match change {
            0 => Stop::UnresolvedAppearance,
            1 => {
                frames.remove(7);
                Stop::OmittedCapture
            }
            2 => {
                frames[7].epoch = 1;
                Stop::ChangedGeometryOrEpoch
            }
            _ => Stop::SourceError,
        };
        let source = activity_source(&frames, [0, if change == 1 { 7 } else { 8 }]);
        let mut reads = vec![];
        let report = analyze_for_activity(
            "inv",
            &frames,
            &tracks,
            |f| {
                reads.push(f.number);
                if change == 3 && f.number == 8 {
                    return Err("Missing saved image".into());
                }
                Ok(images[f.number - 1].clone())
            },
            &source,
        )
        .unwrap();
        assert_eq!(report.series[0].accepted_steps, 6);
        assert_eq!(report.series[0].stop, expected);
        assert!(reads.iter().all(|n| *n <= 8));
        let summary = crate::appearance_activity::summarize(&report, &source, &frames).unwrap();
        assert_eq!(
            summary.appearances[0].pairs[1].status,
            reversebolt_core::appearance_activity::Status::NotMeasured
        );
    }
}

#[test]
fn activity_projection_uses_selected_intervals_not_origin_delta_and_preserves_missing() {
    use reversebolt_core::appearance_activity::Status;
    let patch = texture(17);
    let images: Vec<_> = [
        [64, 80],
        [224, 144],
        [100, 80],
        [132, 80],
        [96, 80],
        [96, 80],
        [96, 80],
        [96, 80],
    ]
    .iter()
    .map(|at| png(&patch, *at))
    .collect();
    let (frames, tracks) = activity_fixture(&images);
    let report = run(&frames, &tracks, &images);
    let source = activity_source(&frames, [0, 2]);
    let summary = crate::appearance_activity::summarize(&report, &source, &frames).unwrap();
    assert_eq!(
        summary.appearances[0].pairs[0].displacement,
        Some([160., 64.])
    );
    assert_eq!(
        summary.appearances[0].pairs[1].displacement,
        Some([32., 0.])
    );
    assert_eq!(
        summary.appearances[0].pairs[1].before_region,
        Some([100., 80., 64., 64.])
    );
    assert_eq!(
        summary.appearances[0].pairs[1].after_region,
        Some([132., 80., 64., 64.])
    );
    summary.validate(&source).unwrap();
    let source = activity_source(&frames, [0, 6]);
    let summary = crate::appearance_activity::summarize(&report, &source, &frames).unwrap();
    assert_eq!(summary.appearances[0].pairs[1].status, Status::NotMeasured);
    assert_eq!(summary.appearances[0].pairs[1].displacement, None);
    let mut failures = vec![];
    let mut bad = summary.clone();
    bad.appearances[0].pairs[1].displacement = Some([0., 0.]);
    failures.push(bad);
    let mut bad = summary.clone();
    bad.appearances[0].pairs[0].displacement = Some([f64::NAN, 0.]);
    failures.push(bad);
    let mut bad = summary.clone();
    bad.appearances[0].origin.image_sha256 = "f".repeat(64);
    failures.push(bad);
    let mut bad = summary.clone();
    bad.appearances.push(bad.appearances[0].clone());
    failures.push(bad);
    let mut bad = summary.clone();
    bad.matcher_policy.push_str("changed");
    failures.push(bad);
    for bad in failures {
        assert!(bad.validate(&source).is_err());
    }
    let mut changed = source.clone();
    changed.key = 65;
    assert!(summary.validate(&changed).is_err());
    assert!(crate::appearance_activity::summarize(&report, &changed, &frames).is_err());
    let mut changed = frames.clone();
    changed[0].started_unix_ms += 1;
    assert!(crate::appearance_activity::summarize(&report, &source, &changed).is_err());
}

#[test]
fn unresolved_appearance_never_becomes_zero_or_fills_a_later_selected_pair() {
    use reversebolt_core::appearance_activity::Status;
    let patch = texture(17);
    let images = vec![
        png(&patch, [64, 80]),
        png(&texture(999), [224, 144]),
        png(&patch, [64, 80]),
        png(&patch, [64, 80]),
    ];
    let (frames, tracks) = activity_fixture(&images);
    let report = run(&frames, &tracks, &images);
    let source = activity_source(&frames, [0, 2]);
    let summary = crate::appearance_activity::summarize(&report, &source, &frames).unwrap();
    assert_eq!(summary.appearances[0].pairs[0].status, Status::PoorMatch);
    assert_eq!(summary.appearances[0].pairs[1].status, Status::NotMeasured);
    assert!(
        summary.appearances[0]
            .pairs
            .iter()
            .all(|p| p.displacement.is_none())
    );
}

#[test]
fn sequence_keeps_origin_signed_changes_timing_inputs_and_full_source_bindings() {
    let patch = texture(17);
    let images = vec![
        png(&patch, [64, 80]),
        png(&patch, [224, 144]),
        png(&patch, [64, 80]),
        png(&patch, [96, 80]),
    ];
    let (mut frames, tracks) = fixture(&images);
    let report = run(&frames, &tracks, &images);
    let series = &report.series[0];
    assert_eq!(series.accepted_steps, 3);
    assert_eq!(series.stop, Stop::EndOfRequestedFrames);
    assert_eq!(
        series
            .steps
            .iter()
            .map(|s| s.displacement_from_previous.unwrap())
            .collect::<Vec<_>>(),
        [[160., 64.], [-160., -64.], [32., 0.]]
    );
    assert_eq!(
        series
            .steps
            .iter()
            .map(|s| s.displacement_from_origin.unwrap())
            .collect::<Vec<_>>(),
        [[160., 64.], [0., 0.], [32., 0.]]
    );
    assert_eq!(series.steps[0].elapsed_us, Some([999_000, 1_001_000]));
    assert_eq!(series.steps[0].exposure, Exposure::Key(68));
    assert_eq!(series.steps[1].exposure, Exposure::NoKeysDetected);
    frames[2].input_context.as_mut().unwrap().generation = "restarted".into();
    let other = run(&frames, &tracks, &images);
    assert_eq!(
        series.steps[0].source_sha256,
        other.series[0].steps[0].source_sha256
    );
    assert_ne!(
        series.steps[1].source_sha256,
        other.series[0].steps[1].source_sha256
    );
    assert_ne!(report.source_sha256, other.source_sha256);
    assert!(matches!(
        other.series[0].steps[1].exposure,
        Exposure::Unknown(_)
    ));
    assert_eq!(
        series.steps[1].displacement_from_origin,
        other.series[0].steps[1].displacement_from_origin
    );
}

#[test]
fn missing_appearance_stops_without_reacquiring_a_later_identical_copy() {
    let patch = texture(17);
    let images = vec![
        png(&patch, [64, 80]),
        png(&patch, [224, 144]),
        png(&texture(811), [224, 144]),
        png(&patch, [64, 80]),
    ];
    let (frames, tracks) = fixture(&images);
    let mut read_numbers = vec![];
    let report = analyze("inv", &frames, &tracks, |f| {
        read_numbers.push(f.number);
        Ok(images[f.number - 1].clone())
    })
    .unwrap();
    let series = &report.series[0];
    assert_eq!(series.accepted_steps, 1);
    assert_eq!(series.stop, Stop::UnresolvedAppearance);
    assert_eq!(series.stopped_before.as_deref(), Some("f3"));
    assert_eq!(series.steps.len(), 2);
    assert!(series.steps[1].displacement_from_previous.is_none());
    assert_eq!(read_numbers, [1, 2, 1, 3]);
}

#[test]
fn changing_appearance_never_updates_the_original_template() {
    let (a, b) = (texture(17), texture(811));
    let mix = |weight: f64| {
        GrayImage::from_fn(64, 64, |x, y| {
            Luma([(f64::from(a.get_pixel(x, y)[0]) * (1. - weight)
                + f64::from(b.get_pixel(x, y)[0]) * weight)
                .round() as u8])
        })
    };
    let images = [0., 0.2, 0.4, 0.6].map(|w| png(&mix(w), [64, 80]));
    let (frames, tracks) = fixture(&images);
    let region = visual_regions::Region {
        x: 64,
        y: 80,
        width: 64,
        height: 64,
    };
    assert_eq!(
        region_appearance::measure(&images[1], &images[2], region)
            .unwrap()
            .status,
        MatchStatus::Matched
    );
    let report = run(&frames, &tracks, &images);
    assert_eq!(report.series[0].accepted_steps, 1);
    assert_eq!(report.series[0].stop, Stop::UnresolvedAppearance);
    assert_eq!(report.series[0].stopped_before.as_deref(), Some("f3"));
}

#[test]
fn corrupted_missing_or_rebound_sources_stop_only_their_sequence() {
    let image = png(&texture(17), [64, 80]);
    let images = vec![image; 4];
    let (frames, tracks) = fixture(&images);
    for change in 0..5 {
        let mut f = frames.clone();
        let expected = match change {
            0 => {
                f.remove(2);
                Stop::OmittedCapture
            }
            1 => {
                f[2].epoch = 1;
                Stop::ChangedGeometryOrEpoch
            }
            2 => {
                f[2].width = 511;
                Stop::ChangedGeometryOrEpoch
            }
            3 => {
                f[2].sha256 = "a".repeat(64);
                Stop::SourceError
            }
            _ => Stop::SourceError,
        };
        let report = analyze("inv", &f, &tracks, |frame| {
            if change == 4 && frame.number == 3 {
                Err("File unavailable".into())
            } else {
                Ok(images[frame.number - 1].clone())
            }
        })
        .unwrap();
        assert_eq!(report.series[0].accepted_steps, 1);
        assert_eq!(report.series[0].stop, expected);
        assert!(report.series[0].steps.last().unwrap().after_id != "f4");
    }
    let mut untimed = frames;
    untimed[1].capture_interval = None;
    let report = run(&untimed, &tracks, &images);
    assert_eq!(report.series[0].accepted_steps, 3);
    assert!(report.series[0].steps[0].elapsed_us.is_none());
    assert!(report.series[0].steps[0].timing_error.is_some());
    assert!(matches!(
        report.series[0].steps[0].exposure,
        Exposure::Unknown(_)
    ));
    // A later independent observation can still be measured when an earlier
    // sequence stops. It does not resume or inherit the earlier source.
    let images = vec![images[0].clone(); 6];
    let (frames, mut tracks) = fixture(&images);
    let mut input = tracks[0].observations[0].as_ref().unwrap().input.clone();
    input.key = "later source".into();
    input.frame_id = frames[3].id.clone();
    input.image_sha256 = frames[3].sha256.clone();
    visual::record(&mut tracks, input, &frames[3], "fixture", frames.len()).unwrap();
    let report = analyze("inv", &frames, &tracks, |f| {
        if f.number == 3 {
            Err("Missing image".into())
        } else {
            Ok(images[f.number - 1].clone())
        }
    })
    .unwrap();
    assert_eq!(report.series[0].stop, Stop::SourceError);
    assert_eq!(report.series[0].accepted_steps, 1);
    assert_eq!(report.series[1].source.before_id, "f4");
    assert_eq!(report.series[1].accepted_steps, 2);
    assert_eq!(report.series[1].stop, Stop::EndOfRequestedFrames);
}

#[test]
fn shared_budget_is_round_robin_and_one_source_cannot_consume_all_work() {
    let image = png(&texture(17), [64, 80]);
    let images = vec![image; 10];
    let (frames, mut tracks) = activity_fixture(&images);
    let single = run(&frames, &tracks, &images);
    assert_eq!(single.attempted_pairs, 6);
    assert_eq!(single.series[0].stop, Stop::StepLimit);
    for id in 1..4 {
        let mut track = tracks[0].clone();
        track.id = id;
        track.key = format!("appearance-{id}");
        let observation = track.observations[0].as_mut().unwrap();
        observation.input.key = track.key.clone();
        observation.input.bounds = Some([
            64. - (id % 2) as f32,
            80. - (id / 2) as f32,
            64. + (id % 2) as f32,
            64. + (id / 2) as f32,
        ]);
        tracks.push(track);
    }
    let report = run(&frames, &tracks, &images);
    assert_eq!(report.attempted_pairs, 12);
    assert_eq!(report.series.len(), 4);
    for s in &report.series {
        assert_eq!(s.accepted_steps, 3);
        assert_eq!(s.stop, Stop::AnalysisBudget);
    }
    let activity = activity_source(&frames, [0, 8]);
    let targeted = analyze_for_activity(
        "inv",
        &frames,
        &tracks,
        |f| Ok(images[f.number - 1].clone()),
        &activity,
    )
    .unwrap();
    assert_eq!(targeted.attempted_pairs, 12);
    for s in &targeted.series {
        assert_eq!(s.accepted_steps, 3);
        assert_eq!(s.stop, Stop::AnalysisBudget);
    }
    let empty = analyze("inv", &frames, &[], |_| {
        panic!("no source must mean no image reads")
    })
    .unwrap();
    assert_eq!(empty.attempted_pairs, 0);
    assert!(empty.series.is_empty());
}
