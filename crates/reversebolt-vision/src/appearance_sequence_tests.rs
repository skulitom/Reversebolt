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
    let (frames, mut tracks) = fixture(&images);
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
    let empty = analyze("inv", &frames, &[], |_| {
        panic!("no source must mean no image reads")
    })
    .unwrap();
    assert_eq!(empty.attempted_pairs, 0);
    assert!(empty.series.is_empty());
}
