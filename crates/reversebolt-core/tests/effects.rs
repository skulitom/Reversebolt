use reversebolt_core::effects::*;

#[test]
fn comparison_retains_changes_missing_and_untracked_scope_without_causal_promotion() {
    let watches: Vec<_> = (0..5)
        .map(|track| Watch {
            track,
            label: format!("Region {track}"),
            bounds: [0., 0., 20., 10.],
        })
        .collect();
    let reading = |track, value| Reading {
        track,
        value,
        reason: "test".into(),
    };
    let before = vec![
        reading(0, Some(50.)),
        reading(1, Some(25.)),
        reading(2, Some(8.)),
        reading(3, None),
    ];
    let after = vec![
        reading(0, Some(53.)),
        reading(1, Some(11.)),
        reading(2, Some(8.4)),
        reading(3, Some(2.)),
        reading(4, Some(0.)),
    ];
    let result = compare(&watches, 0, 7, &before, &after);
    assert_eq!(result.omitted_tracks, 7);
    assert_eq!(
        result
            .other_values
            .iter()
            .map(|v| v.status)
            .collect::<Vec<_>>(),
        vec![
            Status::Changed,
            Status::Unchanged,
            Status::Unreadable,
            Status::Unreadable
        ]
    );
    assert!(result.limitation.contains("ordinary gameplay"));
    assert!(result.limitation.contains("nonnumeric"));
    // A missing or nonfinite reading never becomes a stable zero. Duplicate
    // identities cannot get counted as a valid independently observed value.
    let duplicate = vec![
        reading(1, Some(11.)),
        reading(1, Some(11.)),
        reading(2, Some(f64::NAN)),
    ];
    let result = compare(&watches, 0, 0, &before, &duplicate);
    assert!(
        result
            .other_values
            .iter()
            .all(|v| v.status == Status::Unreadable)
    );
}
