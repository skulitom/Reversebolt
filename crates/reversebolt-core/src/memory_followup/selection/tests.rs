use super::*;
use input::{Unknown, from_exposures};
fn source() -> (Vec<Exposure>, Vec<Option<f64>>) {
    let classes = vec![
        Exposure::NoKeysDetected,
        Exposure::Key(65),
        Exposure::Key(65),
        Exposure::NoKeysDetected,
        Exposure::NoKeysDetected,
        Exposure::Key(65),
        Exposure::Key(65),
        Exposure::NoKeysDetected,
        Exposure::NoKeysDetected,
        Exposure::NoKeysDetected,
        Exposure::NoKeysDetected,
    ];
    let values = [10., 10., 11., 12., 12., 12., 13., 14., 14., 14., 14., 14.]
        .map(Some)
        .to_vec();
    (classes, values)
}
#[test]
fn pinned_expectation_rejects_fresh_quiet_drift_hold_and_reversal() {
    let (classes, values) = source();
    let expected = fit(&from_exposures(&classes, &values), &values).unwrap();
    assert_eq!(expected.direction, Direction::Increased);
    assert_eq!(expected.source_episodes, vec![[1, 2], [5, 6]]);
    let assess = |v: &[Option<f64>]| check(&expected, Some(&from_exposures(&classes, v)), v);
    assert_eq!(assess(&values).verdict, m::Verdict::Consistent);
    let mut drift = values.clone();
    drift[11] = Some(15.);
    let result = assess(&drift);
    assert_eq!(result.verdict, m::Verdict::Contradicted);
    assert_eq!(result.quiet_changed_pairs, vec![10]);
    assert_eq!(
        assess(&[Some(10.); 12]).disagreeing_episodes,
        vec![[1, 2], [5, 6]]
    );
    let reversed: Vec<_> = values.iter().map(|v| v.map(|x| -x)).collect();
    assert_eq!(assess(&reversed).verdict, m::Verdict::Contradicted);
    assert_eq!(
        check(&expected, None, &values).verdict,
        m::Verdict::Insufficient
    );
    // Fresh reversal cannot silently refit the saved increasing expectation.
    assert_eq!(
        fit(&from_exposures(&classes, &reversed), &reversed)
            .unwrap()
            .direction,
        Direction::Decreased
    );
    assert_eq!(expected.direction, Direction::Increased);
}
#[test]
fn selection_abstains_for_drift_single_episode_unbounded_input_and_unreadable_boundaries() {
    let (classes, values) = source();
    let drifting: Vec<_> = (0..12).map(|i| Some(i as f64)).collect();
    assert!(fit(&from_exposures(&classes, &drifting), &drifting).is_none());
    assert!(fit(&from_exposures(&[Exposure::Key(65); 11], &values), &values).is_none());
    let mut c = classes.clone();
    c[5] = Exposure::Unknown(Unknown::Focus);
    assert!(fit(&from_exposures(&c, &values), &values).is_none());
    let mut missing = values.clone();
    missing[5] = None;
    assert!(fit(&from_exposures(&classes, &missing), &missing).is_none());
    let expected = fit(&from_exposures(&classes, &values), &values).unwrap();
    let result = check(
        &expected,
        Some(&from_exposures(&classes, &missing)),
        &missing,
    );
    assert_eq!(result.verdict, m::Verdict::Insufficient);
    assert_eq!(result.unknown_pairs, 2);
    assert_eq!(result.agreeing_episodes.len(), 1);
}
#[test]
fn changed_direction_and_unknowns_are_explicit_without_promoting_a_constant_hold() {
    let (mut c, mut v) = source();
    v[6] = Some(11.);
    v[7..].fill(Some(10.));
    c[9] = Exposure::Unknown(Unknown::Gap);
    let expected = fit(&from_exposures(&c, &v), &v).unwrap();
    assert_eq!(expected.direction, Direction::Changed);
    assert_eq!(expected.source_unknown_pairs, 1);
    assert_eq!(
        check(&expected, Some(&from_exposures(&c, &v)), &v).verdict,
        m::Verdict::Consistent
    );
    let held = vec![Some(10.); 12];
    assert_eq!(
        check(&expected, Some(&from_exposures(&c, &held)), &held).verdict,
        m::Verdict::Contradicted
    );
}
