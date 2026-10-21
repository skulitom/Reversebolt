use super::*;
use crate::{action_patterns::tests::fixture, inputs::Focus};

fn measures(values: &[f64]) -> Vec<Measurement> {
    values
        .iter()
        .map(|v| Measurement {
            difference: Some(*v),
            error: None,
        })
        .collect()
}
#[test]
fn separates_key_episodes_from_quiet_without_inventing_meanings() {
    let (frames, _) = fixture(&[0, 68, 68, 0, 68, 0], &[0.; 6]);
    let evidence = analyze(&frames, measures(&[0., 10., 14., 1., 20., 0.])).unwrap();
    let action = &evidence.actions[0];
    assert_eq!(action.key, 68);
    assert_eq!(action.bounded_episodes, vec![1, 3]);
    assert_eq!(action.median_difference, Some(16.));
    assert_eq!(action.quiet_median_difference, Some(0.));
    assert_eq!(evidence.episodes[1].first_interval, 1);
    assert_eq!(evidence.episodes[1].last_interval, 2);
    assert_eq!(evidence.intervals[1].before, "f1");
    assert_eq!(evidence.intervals[1].after_sha256, "hash2");
    assert!(evidence.limitation.contains("No key meaning"));
}
#[test]
fn focus_chords_gaps_and_restarts_remain_unknown() {
    let (frames, _) = fixture(&[0, 68, 0], &[0.; 3]);
    for mode in 0..5 {
        let mut frames = frames.clone();
        let window = frames[2].input_context.as_mut().unwrap();
        match mode {
            0 => window.polls[3].focus = Focus::OutsideSelected,
            1 => window.polls[3].down.push(69),
            2 => {
                window.polls.drain(5..20);
            }
            3 => window.generation = "another sampler".into(),
            _ => frames[2].number += 1,
        }
        if mode == 4 {
            frames[3].number += 1;
        }
        let evidence = analyze(&frames, measures(&[0., 20., 0.])).unwrap();
        assert!(matches!(
            evidence.intervals[1].exposure,
            Exposure::Unknown(_)
        ));
        assert!(evidence.actions.is_empty());
    }
}
#[test]
fn missing_measurements_never_become_zero_or_supported_episodes() {
    let (frames, _) = fixture(&[0, 68, 0], &[0.; 3]);
    let mut m = measures(&[0., 20., 0.]);
    m[1] = Measurement {
        difference: None,
        error: Some("capture unavailable".into()),
    };
    let evidence = analyze(&frames, m).unwrap();
    assert_eq!(evidence.actions[0].median_difference, None);
    assert!(evidence.actions[0].bounded_episodes.is_empty());
    assert_eq!(
        evidence.intervals[1].measurement.error.as_deref(),
        Some("capture unavailable")
    );
    assert!(analyze(&frames, measures(&[0., f64::NAN, 0.])).is_err());
    assert!(analyze(&frames, measures(&[0., 256., 0.])).is_err());
    let mut duplicate = frames.clone();
    duplicate[2].id = duplicate[1].id.clone();
    assert!(analyze(&duplicate, measures(&[0., 20., 0.])).is_err());
}
