use super::*;
use crate::visual::{ObservationInput, VisualValue};
use serde_json::json;

fn frames(count: usize) -> Vec<FrameRef> {
    (1..=count)
        .map(|n| {
            serde_json::from_value(json!({
                "id":format!("capture-{n}"),"number":n,"sha256":"a".repeat(64),
                "width":256,"height":144,"epoch":0,"started_unix_ms":n
            }))
            .unwrap()
        })
        .collect()
}
fn track(frames: &[FrameRef], id: usize, index: usize, bounds: [f32; 4]) -> VisualTrack {
    let mut tracks = vec![];
    visual::record(
        &mut tracks,
        ObservationInput {
            investigation_id: "investigation".into(),
            frame_id: frames[index].id.clone(),
            image_sha256: frames[index].sha256.clone(),
            observer: "observer".into(),
            key: format!("object-{id}"),
            label: format!("Claim {id}"),
            unit: String::new(),
            value: Some(VisualValue::Text("visible".into())),
            quality: Quality::Provisional,
            bounds: Some(bounds),
            explanation: "An unverified visual interpretation.".into(),
        },
        &frames[index],
        "local-observer",
        frames.last().unwrap().number,
    )
    .unwrap();
    tracks[0].id = id;
    tracks.remove(0)
}
fn input(t: &mut VisualTrack) -> &mut ObservationInput {
    &mut t.observations.iter_mut().flatten().next().unwrap().input
}

#[test]
fn historical_exact_observations_round_outward_and_bind_claims_and_metadata() {
    let f = frames(2);
    let mut t = track(&f, 7, 0, [16.25, 8.5, 32., 32.]);
    t.active = false; // A later view can retire a track without erasing its history.
    let selected = select("investigation", &f, &[t.clone()]).unwrap();
    let p = &selected.regions[0];
    assert_eq!(
        p.region,
        Region {
            x: 16,
            y: 8,
            width: 33,
            height: 33
        }
    );
    assert_eq!(p.before_id, f[0].id);
    assert_eq!(p.after_id, f[1].id);
    assert_eq!(p.sources[0].track_id, 7);
    for change in 0..4 {
        let mut t = t.clone();
        let mut f = f.clone();
        match change {
            0 => input(&mut t).explanation = "A different unverified interpretation.".into(),
            1 => {
                t.label = "Alternative name".into();
                input(&mut t).label = t.label.clone();
            }
            2 => f[1].sha256 = "b".repeat(64),
            _ => f[1].started_unix_ms += 1,
        }
        let other = select("investigation", &f, &[t]).unwrap();
        assert_ne!(p.source_sha256, other.regions[0].source_sha256);
    }
}

#[test]
fn source_mismatch_uncertainty_and_unsupported_boxes_are_omitted_not_invented() {
    let f = frames(2);
    let original = track(&f, 0, 0, [8., 8., 40., 40.]);
    for change in 0..12 {
        let mut t = original.clone();
        let reason = match change {
            0 => {
                input(&mut t).investigation_id = "other".into();
                "invalid_source_binding"
            }
            1 => {
                input(&mut t).frame_id = f[1].id.clone();
                "invalid_source_binding"
            }
            2 => {
                input(&mut t).image_sha256 = "b".repeat(64);
                "invalid_source_binding"
            }
            3 => {
                t.epoch = 1;
                "invalid_source_binding"
            }
            4 => {
                t.key = "other".into();
                "invalid_source_binding"
            }
            5 => {
                t.submitted_by = "different transport".into();
                "invalid_source_binding"
            }
            6 => {
                input(&mut t).quality = Quality::Uncertain;
                "uncertain_observation"
            }
            7 => {
                input(&mut t).bounds = None;
                "missing_bounds"
            }
            8 => {
                input(&mut t).bounds = Some([8., 8., 31., 32.]);
                "unsupported_region_size"
            }
            9 => {
                input(&mut t).bounds = Some([240., 8., 40., 40.]);
                "invalid_source_binding"
            }
            10 => {
                input(&mut t).bounds = Some([f32::NAN, 8., 40., 40.]);
                "invalid_source_binding"
            }
            _ => {
                input(&mut t).value = None;
                "invalid_source_binding"
            }
        };
        let selected = select("investigation", &f, &[t]).unwrap();
        assert!(selected.regions.is_empty(), "case {change}");
        assert_eq!(selected.considered_observations, 1);
        assert_eq!(selected.omitted_observations[reason], 1, "case {change}");
    }
}

#[test]
fn no_region_is_carried_across_missing_frames_scene_changes_or_the_last_capture() {
    for change in 0..4 {
        let mut f = frames(2);
        let t = track(&f, 0, usize::from(change == 3), [8., 8., 40., 40.]);
        let reason = match change {
            0 => {
                f[1].number = 3;
                "omitted_capture"
            }
            1 => {
                f[1].epoch = 1;
                "changed_geometry_or_epoch"
            }
            2 => {
                f[1].width = 255;
                "changed_geometry_or_epoch"
            }
            _ => "no_following_image",
        };
        let selected = select("investigation", &f, &[t]).unwrap();
        assert!(selected.regions.is_empty());
        assert_eq!(selected.omitted_observations[reason], 1);
    }
    let mut f = frames(2);
    f.reverse();
    assert!(select("investigation", &f, &[]).is_err());
    assert!(select("investigation", &frames(1), &[]).is_err());
    assert!(
        select("investigation", &frames(2), &[])
            .unwrap()
            .regions
            .is_empty()
    );
}

#[test]
fn budget_shares_pairs_prefers_small_regions_and_retains_duplicate_claims() {
    let f = frames(4);
    let mut tracks = vec![];
    for pair in 0..3 {
        tracks.push(track(&f, pair * 2, pair, [0., 0., 128., 72.]));
        tracks.push(track(&f, pair * 2 + 1, pair, [16., 16., 32., 32.]));
    }
    tracks.push(track(&f, 9, 0, [16., 16., 32., 32.]));
    let selected = select("investigation", &f, &tracks).unwrap();
    assert_eq!(selected.eligible_observations, 7);
    assert_eq!(selected.duplicate_observations, 1);
    assert_eq!(selected.candidate_regions, 6);
    assert_eq!(selected.omitted_regions, 2);
    assert_eq!(selected.regions.len(), 4);
    assert_eq!(
        selected
            .regions
            .iter()
            .map(|p| p.before_id.as_str())
            .collect::<Vec<_>>(),
        ["capture-1", "capture-2", "capture-3", "capture-1"]
    );
    assert_eq!(
        selected
            .regions
            .iter()
            .map(|p| p.region.width)
            .collect::<Vec<_>>(),
        [32, 32, 32, 128]
    );
    assert_eq!(
        selected.regions[0]
            .sources
            .iter()
            .map(|s| s.track_id)
            .collect::<Vec<_>>(),
        [1, 9]
    );
    tracks.reverse();
    assert_eq!(
        json!(selected),
        json!(select("investigation", &f, &tracks).unwrap())
    );
    tracks.push(tracks[0].clone());
    assert!(select("investigation", &f, &tracks).is_err());
}
