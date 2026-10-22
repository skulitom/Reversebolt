use super::*;
use image::{GrayImage, Luma, imageops};
use reversebolt_core::{
    inputs::{Focus, Poll, Window},
    play_context::{Frame, Pair},
    visual::CaptureInterval,
};

fn fixture(xs: &[u32]) -> (Vec<Vec<u8>>, Vec<FrameRef>, Source) {
    let patch = GrayImage::from_fn(64, 64, |x, y| {
        let mut n = (x + y * 64 + 17)
            .wrapping_mul(747796405)
            .wrapping_add(2891336453);
        n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
        Luma([40 + (((n >> 22) ^ n) % 140) as u8])
    });
    let images: Vec<_> = xs
        .iter()
        .map(|&x| {
            let mut image = GrayImage::from_pixel(512, 288, Luma([20]));
            imageops::replace(&mut image, &patch, i64::from(x), 80);
            let mut bytes = std::io::Cursor::new(vec![]);
            image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
            bytes.into_inner()
        })
        .collect();
    let frames: Vec<_> = images
        .iter()
        .enumerate()
        .map(|(i, bytes)| {
            let begin = (i + 1) as u64 * 1_000_000;
            let prior = i as u64 * 1_000_000 + 1000;
            FrameRef {
                sampling: None,
                input_context: Some(Window {
                    clock_id: "clock".into(),
                    generation: "sampler".into(),
                    requested_begin_us: prior,
                    requested_end_us: begin + 1000,
                    monitor_started_us: 0,
                    retained_from_us: 0,
                    omitted_polls: 0,
                    largest_unsampled_us: 20_000,
                    polls: (prior..begin)
                        .step_by(20_000)
                        .enumerate()
                        .map(|(n, t)| Poll {
                            begin_us: t,
                            end_us: t + 100,
                            focus: Focus::SelectedForeground,
                            down: if [2, 6].contains(&i) && n < 10 {
                                vec![68]
                            } else {
                                vec![]
                            },
                        })
                        .collect(),
                    limitation: "Sampled input fixture.".into(),
                }),
                id: format!("f{i}"),
                number: i + 1,
                sha256: visual::digest(bytes),
                width: 512,
                height: 288,
                epoch: 0,
                started_unix_ms: i as u128,
                capture_interval: Some(CaptureInterval {
                    clock_id: "clock".into(),
                    begin_us: begin,
                    end_us: begin + 1000,
                }),
            }
        })
        .collect();
    let source = source(&frames);
    (images, frames, source)
}
fn source(frames: &[FrameRef]) -> Source {
    let cite = |f: &FrameRef| Frame {
        id: f.id.clone(),
        sha256: f.sha256.clone(),
        number: f.number,
        width: f.width,
        height: f.height,
        interval: f.capture_interval.clone().unwrap(),
    };
    Source {
        investigation_id: "clock".into(),
        batch: 1,
        key: 68,
        quiet_difference: 0.,
        frame_metadata_sha256: visual::digest(&serde_json::to_vec(frames).unwrap()),
        pairs: [1, 5].map(|i| Pair {
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
fn run(images: &[Vec<u8>], frames: &[FrameRef], source: &Source) -> Report {
    analyze(source, frames, |f| Ok(images[f.number - 1].clone())).unwrap()
}

#[test]
fn passive_summary_uses_original_pixel_pairs_and_rejects_rebound_or_resumed_checks() {
    use crate::activity_region_summary::summarize;
    use reversebolt_core::appearance_activity::Status;
    let (mut images, mut frames, _) = fixture(&[64, 64, 80, 80, 80, 80, 64, 64]);
    for (image, frame) in images.iter_mut().zip(&mut frames) {
        let gray = image::load_from_memory(image).unwrap().to_luma8();
        let resized = imageops::resize(&gray, 1024, 576, imageops::FilterType::Triangle);
        let mut png = std::io::Cursor::new(vec![]);
        resized.write_to(&mut png, image::ImageFormat::Png).unwrap();
        *image = png.into_inner();
        frame.sha256 = visual::digest(image);
        frame.width = 1024;
        frame.height = 576;
    }
    let source = source(&frames);
    let mut report = run(&images, &frames, &source);
    let summary = summarize(&report, &source, &frames).unwrap();
    summary.validate(&source).unwrap();
    assert_eq!(
        summary.proposal_report_sha256,
        visual::digest(&serde_json::to_vec(&report).unwrap())
    );
    let best = &summary.regions[report.recommended.unwrap()];
    assert_eq!(best.root_status, Status::Matched);
    assert_eq!(best.quiet.displacement, Some([0., 0.]));
    assert_eq!(best.pairs[0].displacement, Some([32., 0.]));
    assert_eq!(best.pairs[1].displacement, Some([-32., 0.]));
    for pair in &best.pairs {
        let a = pair.before_region.unwrap();
        let b = pair.after_region.unwrap();
        assert_eq!([b[0] - a[0], b[1] - a[1]], pair.displacement.unwrap());
    }
    let restored: reversebolt_core::activity_regions::Summary =
        serde_json::from_value(json!(summary)).unwrap();
    assert_eq!(restored, summary);
    let mut wrong = source.clone();
    wrong.batch += 1;
    assert!(summarize(&report, &wrong, &frames).is_err());
    assert!(summary.validate(&wrong).is_err());
    for case in 0..8 {
        let mut invalid = summary.clone();
        match case {
            0 => invalid.origin.sha256 = "f".repeat(64),
            1 => invalid.quiet_frame.number += 1,
            2 => invalid.regions[0].pairs[0].displacement = Some([9999., 0.]),
            3 => invalid.regions[0].pairs[0].before_region.as_mut().unwrap()[0] += 1.,
            4 => invalid.regions[0].quiet.status = Status::Ambiguous,
            5 => invalid.regions[0].root_status = Status::NotMeasured,
            6 => invalid.regions.push(invalid.regions[0].clone()),
            _ => invalid.selector_policy = "other".into(),
        }
        assert!(invalid.validate(&source).is_err(), "case {case}");
    }
    report.suggestions[0].frames[1].status = matcher::Status::PoorMatch;
    assert!(summarize(&report, &source, &frames).is_err());
    report.suggestions[0].frames.truncate(2);
    report.attempted_frames = report.suggestions.iter().map(|s| s.frames.len()).sum();
    let stopped = summarize(&report, &source, &frames).unwrap();
    assert_eq!(stopped.regions[0].quiet.status, Status::PoorMatch);
    assert!(
        stopped.regions[0]
            .pairs
            .iter()
            .all(|p| p.status == Status::NotMeasured && p.displacement.is_none())
    );
}

#[test]
fn stable_texture_selection_survives_large_untextured_changes_without_using_second_pair() {
    let (mut images, mut frames, _) = fixture(&[64, 64, 80, 80, 64, 64, 80, 80]);
    for i in [2, 6] {
        let mut image = image::load_from_memory(&images[i]).unwrap().to_luma8();
        for y in 0..288 {
            for x in 256..512 {
                image[(x, y)] = Luma([240]);
            }
        }
        let mut bytes = std::io::Cursor::new(vec![]);
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        images[i] = bytes.into_inner();
        frames[i].sha256 = visual::digest(&images[i]);
    }
    let source = source(&frames);
    let old = analyze_with_ranking(
        &source,
        &frames,
        |f| Ok(images[f.number - 1].clone()),
        Ranking::ChangeOnly,
    )
    .unwrap();
    assert_eq!(old.status, "no_supported_region");
    assert!(
        old.suggestions
            .iter()
            .all(|s| s.frames[0].status == matcher::Status::LowTexture)
    );
    assert!(json!(old).get("selection").is_none());
    let new = run(&images, &frames, &source);
    assert_eq!(new.status, "suggestions_available");
    assert!(new.selection.as_ref().unwrap().rejected_low_texture > 0);
    let best = &new.suggestions[new.recommended.unwrap()];
    assert_eq!(
        best.displacement,
        [Some([0, 0]), Some([16, 0]), Some([16, 0])]
    );
    assert_eq!(new.suggestions.len(), 4);
    assert!(new.attempted_frames <= 20);
    // Destroy both later-episode images while preserving valid hashes. Only
    // correspondence outcomes may change; selection and candidate order may not.
    images[5] = images[2].clone();
    images[6] = images[0].clone();
    for i in [5, 6] {
        frames[i].sha256 = visual::digest(&images[i]);
    }
    let mut second_source = source.clone();
    second_source.frame_metadata_sha256 = visual::digest(&serde_json::to_vec(&frames).unwrap());
    second_source.pairs[1].before.sha256 = frames[5].sha256.clone();
    second_source.pairs[1].after.sha256 = frames[6].sha256.clone();
    let changed = run(&images, &frames, &second_source);
    assert_eq!(json!(new.selection), json!(changed.selection));
    let selection = |r: &Report| {
        r.suggestions.iter().map(|s|
        json!({"region":s.region,"change":s.action_minus_idle_change,"quality":s.selection_quality})
    ).collect::<Vec<_>>()
    };
    assert_eq!(selection(&new), selection(&changed));
    assert_ne!(new.source_sha256, changed.source_sha256);
}

#[test]
fn proposals_bind_five_images_and_do_not_assume_equal_key_effects() {
    let (images, frames, source) = fixture(&[64, 64, 80, 80, 80, 80, 64, 64]);
    let mut reads = vec![];
    let report = analyze(&source, &frames, |f| {
        reads.push(f.number);
        Ok(images[f.number - 1].clone())
    })
    .unwrap();
    assert_eq!(reads, [1, 2, 3, 6, 7]);
    assert_eq!(
        report
            .checked_frames
            .iter()
            .map(|f| f.number)
            .collect::<Vec<_>>(),
        [2, 1, 3, 6, 7]
    );
    assert_eq!(report.origin_frame_id, frames[1].id);
    assert_eq!(report.origin_image_sha256, frames[1].sha256);
    assert_eq!(report.status, "suggestions_available");
    assert!(report.suggestions.len() <= 4 && report.attempted_frames <= 20);
    let best = &report.suggestions[report.recommended.unwrap()];
    assert_eq!(
        best.displacement,
        [Some([0, 0]), Some([16, 0]), Some([-16, 0])]
    );
    assert_eq!(best.status, "motion_in_both_pairs");
    assert_eq!(json!(report), json!(run(&images, &frames, &source)));
    // Unread intermediate pixels do not establish tracking or affect these independent pairs.
    let mut other = images.clone();
    other[3] = vec![];
    other[4] = vec![];
    other[7] = vec![];
    assert_eq!(json!(report), json!(run(&other, &frames, &source)));
}

#[test]
fn unavailable_or_mixed_activity_is_rejected_before_pixels() {
    let (_, frames, valid) = fixture(&[64, 64, 80, 80, 64, 64, 80, 80]);
    for case in 0..9 {
        let mut changed = frames.clone();
        match case {
            0 => changed[0].input_context = None,
            1 => changed[1].input_context.as_mut().unwrap().polls[0].down = vec![68],
            2 => changed[4].epoch = 1,
            3 => changed[4].width = 511,
            4 => changed[4].number += 1,
            5 => changed[4].capture_interval.as_mut().unwrap().clock_id = "other".into(),
            6 => {
                for i in [5, 6] {
                    changed[i].input_context.as_mut().unwrap().generation = "restarted".into();
                }
            }
            7 => changed[7].id = changed[0].id.clone(),
            _ => changed[4].capture_interval = None,
        }
        let source = source(&changed);
        assert!(
            analyze(&source, &changed, |_| panic!(
                "Invalid metadata must fail before images"
            ))
            .is_err(),
            "case {case}"
        );
    }
    let mut changed = frames.clone();
    changed[0].sha256 = "f".repeat(64);
    assert!(analyze(&valid, &changed, |_| panic!("Stale source")).is_err());
    let without_quiet = &frames[1..];
    let mut source = valid.clone();
    source.frame_metadata_sha256 = visual::digest(&serde_json::to_vec(without_quiet).unwrap());
    assert!(analyze(&source, without_quiet, |_| panic!("Missing quiet")).is_err());
    let reordered: Vec<_> = frames[4..].iter().chain(&frames[..4]).cloned().collect();
    let mut source = valid;
    source.frame_metadata_sha256 = visual::digest(&serde_json::to_vec(&reordered).unwrap());
    assert!(analyze(&source, &reordered, |_| panic!("Reversed episode blocks")).is_err());
}

#[test]
fn suggestions_abstain_on_stationarity_corruption_and_unresolved_appearance() {
    let (images, frames, stationary) = fixture(&[64, 64, 64, 64, 64, 64, 64, 64]);
    let report = run(&images, &frames, &stationary);
    assert_eq!(report.status, "no_action_regions");
    assert_eq!(report.attempted_frames, 0);
    assert!(report.recommended.is_none());
    assert!(analyze(&stationary, &frames, |_| Err("missing".into())).is_err());
    assert!(analyze(&stationary, &frames, |_| Ok(vec![0])).is_err());
    let mut wrong_dimensions = frames.clone();
    for f in &mut wrong_dimensions {
        f.width = 511;
    }
    assert!(
        analyze(&source(&wrong_dimensions), &wrong_dimensions, |f| Ok(
            images[f.number - 1].clone()
        ))
        .is_err()
    );
    let mut corrupt = frames.clone();
    corrupt[0].sha256 = visual::digest(&[0]);
    assert!(analyze(&source(&corrupt), &corrupt, |_| Ok(vec![0])).is_err());
    let (images, frames, selected) = fixture(&[64, 64, 80, 80, 64, 64, 64, 64]);
    let report = run(&images, &frames, &selected);
    assert!(report.recommended.is_none());
    assert!(
        report
            .suggestions
            .iter()
            .any(|s| s.status == "below_threshold" && s.displacement[2] == Some([0, 0]))
    );
    let (mut images, mut frames, _) = fixture(&[64, 64, 80, 80, 64, 64, 80, 80]);
    let mut blank = std::io::Cursor::new(vec![]);
    GrayImage::from_pixel(512, 288, Luma([20]))
        .write_to(&mut blank, image::ImageFormat::Png)
        .unwrap();
    images[5] = blank.into_inner();
    frames[5].sha256 = visual::digest(&images[5]);
    let source = source(&frames);
    let report = run(&images, &frames, &source);
    assert_eq!(report.status, "no_supported_region");
    assert!(report.recommended.is_none());
    assert!(
        report
            .suggestions
            .iter()
            .all(|s| s.frames.len() <= 4 && s.displacement[2].is_none())
    );
    // A later matching image cannot rescue an unresolved second-before image.
    assert!(report.attempted_frames <= 16);
}
