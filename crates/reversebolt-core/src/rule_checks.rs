//! Frozen conditional predictions over sampled input episodes, never causal authority.
use crate::{
    action_patterns::{self, Evidence, Exposure, Outcome, Source},
    live::LiveView,
    visual::{Quality, VisualValue},
};
use serde::{Deserialize, Serialize};

pub const MAX_CHECKS: usize = 8;
pub const MAX_FEATURES: usize = 32;
pub const LIMITATION: &str = "Frozen predictions of observed episode changes, not game-delivered input, independent randomized trials, a unique mechanism or edit authority. Threshold gaps retain all training-equivalent boundaries. State, action history, unobserved inputs and observation errors can remain confounded. No hidden state is recovered. Predictions count only when issued before the first recorded key-down sample; a missed earlier press is still possible.";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "track", rename_all = "snake_case")]
pub enum Feature {
    Ocr(usize),
    External(usize),
    PriorEpisodes,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub before_frame: String,
    pub after_frame: String,
    pub before_number: usize,
    pub after_number: usize,
    pub outcome: Outcome,
    pub prior_episodes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub feature: Feature,
    pub label: String,
    /// Boundaries within this gap are indistinguishable in the fitting data.
    pub lower: f64,
    pub upper: f64,
    pub change_above: bool,
    pub changed: Outcome,
    pub training_values: Vec<f64>,
}
impl Candidate {
    pub fn predict(&self, value: f64) -> Option<Outcome> {
        if !value.is_finite() {
            return None;
        }
        let high = if value >= self.upper {
            true
        } else if value <= self.lower {
            false
        } else {
            return None;
        };
        Some(if high == self.change_above {
            self.changed
        } else {
            Outcome::Unchanged
        })
    }
    pub fn description(&self) -> String {
        format!(
            "{}: change on the {} side; boundary somewhere between {} and {}",
            self.label,
            if self.change_above { "higher" } else { "lower" },
            self.lower,
            self.upper
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prediction {
    pub issued_us: u64,
    pub value: f64,
    pub expected: Option<Outcome>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Forecast {
    pub frame_id: String,
    pub image_sha256: String,
    pub number: usize,
    pub prior_episodes: usize,
    /// One slot per immutable candidate. A missing current-frame slot can be filled
    /// once when a declaration arrives; older forecasts never acquire late context.
    pub predictions: Vec<Option<Prediction>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Result {
    Agreed,
    Contradicted,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comparison {
    pub candidate: usize,
    pub expected: Option<Outcome>,
    pub issued_us: Option<u64>,
    pub result: Result,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Challenge {
    pub episode: Row,
    pub first_pressed_us: Option<u64>,
    pub comparisons: Vec<Comparison>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Check {
    pub id: String,
    pub investigation_id: String,
    pub batch: usize,
    pub source: Source,
    pub track: usize,
    pub key: u8,
    pub epoch: usize,
    pub generation: String,
    pub frozen_after_frame: usize,
    pub frozen_at_us: u64,
    pub training: Vec<Row>,
    pub candidates: Vec<Candidate>,
    pub omitted_features: usize,
    pub forecasts: Vec<Forecast>,
    pub challenges: Vec<Challenge>,
    pub invalidated: Option<String>,
    pub limitation: String,
}
impl Check {
    pub fn rejected(&self, candidate: usize) -> bool {
        self.challenges
            .iter()
            .any(|c| c.comparisons[candidate].result == Result::Contradicted)
    }
    pub fn summary(&self) -> String {
        if self.invalidated.is_some() {
            return "Pattern check needs a fresh investigation".into();
        }
        let rejected = (0..self.candidates.len())
            .filter(|i| self.rejected(*i))
            .count();
        format!(
            "{} possible explanations · {} ruled out · {} later episodes checked",
            self.candidates.len() - rejected,
            rejected,
            self.challenges.len()
        )
    }
}

fn rows(e: &Evidence, key: u8) -> Vec<Row> {
    e.episodes
        .iter()
        .filter(|ep| ep.bounded && ep.exposure == Exposure::Key(key))
        .enumerate()
        .map(|(prior, ep)| {
            let first = &e.intervals[ep.first_interval];
            let last = &e.intervals[ep.last_interval];
            Row {
                before_frame: first.before_frame.clone(),
                after_frame: last.after_frame.clone(),
                before_number: first.before_number,
                after_number: last.after_number,
                outcome: ep.outcome,
                prior_episodes: prior,
            }
        })
        .collect()
}

fn scalar(v: &VisualValue) -> Option<f64> {
    match v {
        VisualValue::Number(n) if n.is_finite() => Some(*n),
        VisualValue::Boolean(b) => Some(f64::from(*b)),
        _ => None,
    }
}
fn value(view: &LiveView, feature: &Feature, number: usize, prior: usize) -> Option<f64> {
    let index = number.checked_sub(1)?;
    match feature {
        Feature::PriorEpisodes => Some(prior as f64),
        Feature::Ocr(id) => view
            .tracks
            .iter()
            .find(|t| t.active && t.id == *id)?
            .samples
            .get(index)
            .copied()
            .flatten()
            .filter(|n| n.is_finite()),
        Feature::External(id) => view
            .visual_tracks
            .iter()
            .find(|t| t.active && t.id == *id)?
            .observations
            .get(index)?
            .as_ref()
            .filter(|o| o.input.quality == Quality::Provisional)
            .and_then(|o| o.input.value.as_ref())
            .and_then(scalar),
    }
}
fn features(view: &LiveView) -> Vec<(Feature, String)> {
    let mut all = vec![(Feature::PriorEpisodes, "Earlier sampled episodes".into())];
    all.extend(
        view.tracks
            .iter()
            .filter(|t| t.active)
            .map(|t| (Feature::Ocr(t.id), t.label.clone())),
    );
    all.extend(
        view.visual_tracks
            .iter()
            .filter(|t| t.active)
            .map(|t| (Feature::External(t.id), t.label.clone())),
    );
    all
}

fn fit(
    feature: Feature,
    label: String,
    values: Vec<f64>,
    training: &[Row],
    changed: Outcome,
) -> Option<Candidate> {
    let mut positive = vec![];
    let mut negative = vec![];
    for (x, row) in values.iter().zip(training) {
        if row.outcome == changed {
            positive.push(*x);
        } else if row.outcome == Outcome::Unchanged {
            negative.push(*x);
        } else {
            return None;
        }
    }
    if positive.is_empty() || negative.is_empty() {
        return None;
    }
    let bounds = |v: &[f64]| {
        (
            v.iter().copied().fold(f64::INFINITY, f64::min),
            v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let (p0, p1) = bounds(&positive);
    let (n0, n1) = bounds(&negative);
    let (lower, upper, change_above) = if n1 < p0 {
        (n1, p0, true)
    } else if p1 < n0 {
        (p1, n0, false)
    } else {
        return None;
    };
    Some(Candidate {
        feature,
        label,
        lower,
        upper,
        change_above,
        changed,
        training_values: values,
    })
}

fn freeze(view: &LiveView, e: &Evidence, key: u8, now: u64) -> Option<Check> {
    let capacity = (512 * 1024 * 1024 / view.planned_bytes.max(1)).min(20);
    if capacity.saturating_sub(view.sample_count) < 3 {
        return None;
    }
    let training = rows(e, key);
    if training.len() < 3
        || e.episodes
            .iter()
            .any(|ep| ep.exposure == Exposure::NoKeysDetected && ep.outcome != Outcome::Unchanged)
    {
        return None;
    }
    let changed = training
        .iter()
        .find(|r| r.outcome != Outcome::Unchanged)?
        .outcome;
    if changed == Outcome::Mixed {
        return None;
    }
    let all = features(view);
    let omitted_features = all.len().saturating_sub(MAX_FEATURES);
    let mut candidates = vec![];
    for (feature, label) in all.into_iter().take(MAX_FEATURES) {
        let values = training
            .iter()
            .map(|r| value(view, &feature, r.before_number, r.prior_episodes))
            .collect::<Option<Vec<_>>>();
        if let Some(values) = values
            && let Some(c) = fit(feature, label, values, &training, changed)
        {
            candidates.push(c);
        }
    }
    if candidates.is_empty() {
        return None;
    }
    let last = view.frames.last()?;
    Some(Check {
        id: uuid::Uuid::new_v4().to_string(),
        investigation_id: view.investigation_id.clone(),
        batch: view.coverage.batch,
        source: e.source,
        track: e.track,
        key,
        epoch: last.epoch,
        generation: last.input_context.as_ref()?.generation.clone(),
        frozen_after_frame: view.sample_count,
        frozen_at_us: now,
        training,
        candidates,
        omitted_features,
        forecasts: vec![],
        challenges: vec![],
        invalidated: None,
        limitation: LIMITATION.into(),
    })
}

fn first_pressed(view: &LiveView, row: &Row, key: u8) -> Option<u64> {
    view.frames
        .iter()
        .filter(|f| f.number > row.before_number && f.number <= row.after_number)
        .filter_map(|f| {
            let time = f.capture_interval.as_ref()?;
            f.input_context
                .as_ref()?
                .polls
                .iter()
                .filter(|p| p.end_us <= time.begin_us && p.down.contains(&key))
                .map(|p| p.begin_us)
                .min()
        })
        .min()
}

fn update_check(check: &mut Check, view: &LiveView, now: u64) {
    if check.invalidated.is_some() {
        return;
    }
    let last = view.frames.last();
    if !view.connected
        || check.investigation_id != view.investigation_id
        || check.batch != view.coverage.batch
        || !last.is_some_and(|f| {
            f.epoch == check.epoch
                && f.input_context
                    .as_ref()
                    .is_some_and(|w| w.generation == check.generation)
        })
    {
        check.invalidated =
            Some("Process, memory batch, image geometry or input generation changed.".into());
        return;
    }
    let Some(e) = action_patterns::find(view, check.source, check.track) else {
        check.invalidated = Some("The observation series is no longer active.".into());
        return;
    };
    let all_rows = rows(e, check.key);
    let previous: Vec<_> = all_rows
        .iter()
        .filter(|r| r.after_number <= check.frozen_after_frame)
        .cloned()
        .collect();
    if previous != check.training
        || check.candidates.iter().any(|c| {
            check.training.iter().zip(&c.training_values).any(|(r, x)| {
                value(view, &c.feature, r.before_number, r.prior_episodes) != Some(*x)
            })
        })
    {
        check.invalidated=Some("Earlier observations changed after the hypotheses were frozen; no refitting was performed.".into());
        return;
    }
    for row in all_rows
        .iter()
        .filter(|r| r.before_number >= check.frozen_after_frame)
    {
        if check
            .challenges
            .iter()
            .any(|c| c.episode.before_frame == row.before_frame)
        {
            continue;
        }
        let first_pressed_us = first_pressed(view, row, check.key);
        let forecast = check
            .forecasts
            .iter()
            .find(|f| f.frame_id == row.before_frame && f.prior_episodes == row.prior_episodes);
        let comparisons = check
            .candidates
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let prediction = forecast.and_then(|f| f.predictions[i].as_ref());
                let (result, reason) = match prediction {
                    None => (
                        Result::Unknown,
                        "No forecast with known starting context was issued for this episode",
                    ),
                    Some(p) if first_pressed_us.is_none_or(|t| t <= p.issued_us) => (
                        Result::Unknown,
                        "The prediction was not issued before the first sampled press",
                    ),
                    Some(p) if p.expected.is_none() => (
                        Result::Unknown,
                        "Starting context lies inside an unresolved threshold gap",
                    ),
                    Some(p) if p.expected == Some(row.outcome) => (
                        Result::Agreed,
                        "The frozen prediction agreed with this observed episode",
                    ),
                    Some(_) => (
                        Result::Contradicted,
                        "The observed episode contradicted this frozen prediction",
                    ),
                };
                Comparison {
                    candidate: i,
                    expected: prediction.and_then(|p| p.expected),
                    issued_us: prediction.map(|p| p.issued_us),
                    result,
                    reason: reason.into(),
                }
            })
            .collect();
        check.challenges.push(Challenge {
            episode: row.clone(),
            first_pressed_us,
            comparisons,
        });
    }
    let last = last.unwrap();
    let prior = all_rows.len();
    if !check
        .forecasts
        .iter()
        .any(|f| f.number == last.number && f.prior_episodes == prior)
    {
        check.forecasts.push(Forecast {
            frame_id: last.id.clone(),
            image_sha256: last.sha256.clone(),
            number: last.number,
            prior_episodes: prior,
            predictions: vec![None; check.candidates.len()],
        });
    }
    let forecast = check
        .forecasts
        .iter_mut()
        .find(|f| f.number == last.number && f.prior_episodes == prior)
        .unwrap();
    for (i, candidate) in check.candidates.iter().enumerate() {
        if forecast.predictions[i].is_none()
            && let Some(x) = value(view, &candidate.feature, last.number, prior)
        {
            forecast.predictions[i] = Some(Prediction {
                issued_us: now,
                value: x,
                expected: candidate.predict(x),
            });
        }
    }
}

/// Same automatic path for desktop and MCP. No action is delivered or authorized.
pub fn update(view: &LiveView, now: u64) -> Vec<Check> {
    let mut checks = view.rule_checks.clone();
    for check in &mut checks {
        update_check(check, view, now);
    }
    if !view.connected {
        return checks;
    }
    for evidence in &view.action_patterns {
        for pattern in &evidence.patterns {
            if checks.len() >= MAX_CHECKS {
                return checks;
            }
            if checks.iter().any(|c| {
                c.source == evidence.source && c.track == evidence.track && c.key == pattern.key
            }) {
                continue;
            }
            if let Some(mut check) = freeze(view, evidence, pattern.key, now) {
                update_check(&mut check, view, now);
                checks.push(check);
            }
        }
    }
    checks
}

/// Engine calls this after computation, before persisting and publishing. Do not
/// move timestamps of any already issued prediction, including unknown boundaries.
pub fn stamp_prepared(checks: &mut [Check], old: &[Check], prepared_us: u64) {
    for check in checks {
        let previous = old.iter().find(|c| c.id == check.id);
        if previous.is_none() {
            check.frozen_at_us = prepared_us;
        }
        for forecast in &mut check.forecasts {
            let old_forecast = previous.and_then(|c| {
                c.forecasts.iter().find(|f| {
                    f.frame_id == forecast.frame_id && f.prior_episodes == forecast.prior_episodes
                })
            });
            for (i, prediction) in forecast.predictions.iter_mut().enumerate() {
                if old_forecast.is_none_or(|f| f.predictions[i].is_none())
                    && let Some(p) = prediction
                {
                    p.issued_us = prepared_us;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::{NumberTrack, ProcessInfo};
    use crate::visual::FrameRef;

    fn trace(changes: &[f64]) -> (LiveView, Vec<FrameRef>, Vec<Option<VisualValue>>) {
        let (mut frames, values) = action_patterns::tests::fixture(
            &[0, 68, 68, 0, 68, 68, 0, 68, 68, 0, 0, 68, 68, 0, 68, 68, 0],
            changes,
        );
        // Synthetic key-down begins after the forecast issue time, not at the image boundary.
        for f in &mut frames {
            if let Some(w) = &mut f.input_context {
                for p in &mut w.polls {
                    if p.begin_us < w.requested_begin_us + 40_000 {
                        p.down.clear();
                    }
                }
            }
        }
        let view = LiveView {
            memory_sample_numbers: None,
            runtime: Default::default(),
            change_searches: vec![],
            value_searches: vec![],
            memory_process: None,
            memory_sources: vec![],
            memory_sources_error: None,
            local_observation_warnings: vec![],
            rule_checks: vec![],
            rule_error: None,
            action_patterns: vec![],
            prediction_checks: vec![],
            coverage: crate::coverage::Coverage {
                batch: 1,
                ..Default::default()
            },
            investigation_id: "clock".into(),
            frames: vec![],
            visual_tracks: vec![],
            temporal_tracks: vec![],
            process: ProcessInfo::default(),
            eligible_bytes: 128,
            planned_bytes: 128,
            sample_count: 0,
            retained_bytes: 0,
            latest: None,
            tracks: vec![NumberTrack {
                id: 0,
                label: "Level".into(),
                bounds: [0.; 4],
                samples: vec![],
                matches: vec![],
                readings: vec![],
                active: true,
            }],
            phase: String::new(),
            connected: true,
        };
        (view, frames, values)
    }
    fn append(view: &mut LiveView, f: &FrameRef, v: &Option<VisualValue>, late: bool) {
        view.frames.push(f.clone());
        view.sample_count = f.number;
        view.tracks[0].samples.push(v.as_ref().and_then(scalar));
        view.action_patterns =
            action_patterns::analyze(&view.frames, &view.tracks, &view.visual_tracks);
        let issued =
            f.capture_interval.as_ref().unwrap().end_us + if late { 2_000_000 } else { 10_000 };
        view.rule_checks = update(view, issued);
    }
    fn history_changes() -> [f64; 17] {
        [
            0., -1., 0., 0., -1., 0., 0., 0., 0., 0., 2., 0., 0., 0., 0., 0., 0.,
        ]
    }
    #[test]
    fn frozen_alternatives_disagree_after_refill_and_future_outcomes_eliminate_the_wrong_rule() {
        for actual_state_gate in [false, true] {
            let mut changes = history_changes();
            if actual_state_gate {
                changes[11] = -1.;
                changes[14] = -1.;
            }
            let (mut view, frames, values) = trace(&changes);
            let mut frozen = String::new();
            for (f, v) in frames.iter().zip(&values) {
                append(&mut view, f, v, false);
                if f.number == 11 {
                    assert_eq!(view.rule_checks.len(), 1);
                    let c = &view.rule_checks[0];
                    assert_eq!(c.frozen_after_frame, 11);
                    assert_eq!(c.training.len(), 3);
                    assert_eq!(c.candidates.len(), 2);
                    frozen = serde_json::to_string(&c.candidates).unwrap();
                }
            }
            let check = &view.rule_checks[0];
            assert!(check.invalidated.is_none(), "{:?}", check.invalidated);
            assert_eq!(check.challenges.len(), 2);
            assert_eq!(
                serde_json::to_string(&check.candidates).unwrap(),
                frozen,
                "Never refit to fresh outcomes"
            );
            let refill = check.forecasts.iter().find(|f| f.number == 12).unwrap();
            assert_ne!(
                refill.predictions[0].as_ref().unwrap().expected,
                refill.predictions[1].as_ref().unwrap().expected
            );
            for c in &check.challenges {
                assert!(
                    c.comparisons
                        .iter()
                        .all(|p| p.issued_us.unwrap() < c.first_pressed_us.unwrap())
                );
            }
            for (i, c) in check.candidates.iter().enumerate() {
                assert_eq!(
                    check.rejected(i),
                    matches!(c.feature, Feature::PriorEpisodes) == actual_state_gate
                );
            }
        }
    }
    #[test]
    fn boundary_gap_is_an_abstention_even_when_other_predictions_agree() {
        let mut changes = history_changes();
        changes[10] = 0.5;
        let (mut view, frames, values) = trace(&changes);
        for (f, v) in frames.iter().zip(&values) {
            append(&mut view, f, v, false);
        }
        let check = &view.rule_checks[0];
        let index = check
            .candidates
            .iter()
            .position(|c| matches!(c.feature, Feature::Ocr(0)))
            .unwrap();
        assert_eq!(check.candidates[index].lower, 98.);
        assert_eq!(check.candidates[index].upper, 99.);
        assert!(
            check
                .challenges
                .iter()
                .all(|c| c.comparisons[index].result == Result::Unknown)
        );
        assert!(
            !check.rejected(index),
            "Unknown is neither agreement nor contradiction"
        );
    }
    #[test]
    fn predictions_issued_after_a_sampled_press_never_count_as_confirmation() {
        let (mut view, frames, values) = trace(&history_changes());
        for (f, v) in frames.iter().zip(&values) {
            append(&mut view, f, v, true);
        }
        assert_eq!(view.rule_checks[0].challenges.len(), 2);
        assert!(
            view.rule_checks[0]
                .challenges
                .iter()
                .all(|c| c.comparisons.iter().all(|p| p.result == Result::Unknown))
        );
    }
    #[test]
    fn scope_changes_and_altered_training_invalidate_without_restarting_the_check() {
        for mode in 0..4 {
            let (mut view, frames, values) = trace(&history_changes());
            for (f, v) in frames.iter().zip(&values).take(11) {
                append(&mut view, f, v, false);
            }
            let id = view.rule_checks[0].id.clone();
            match mode {
                0 => view.connected = false,
                1 => view.frames.last_mut().unwrap().epoch += 1,
                2 => {
                    view.frames
                        .last_mut()
                        .unwrap()
                        .input_context
                        .as_mut()
                        .unwrap()
                        .generation = "resumed".into()
                }
                _ => view.tracks[0].samples[1] = Some(80.),
            }
            view.rule_checks = update(&view, 20_000_000);
            assert!(view.rule_checks[0].invalidated.is_some());
            assert_eq!(view.rule_checks[0].id, id);
            assert_eq!(view.rule_checks.len(), 1);
        }
    }
    #[test]
    fn no_response_or_constant_response_cannot_invent_a_conditional_rule() {
        for changes in [
            [0.; 17],
            [
                0., -1., 0., 0., -1., 0., 0., -1., 0., 0., 0., -1., 0., 0., -1., 0., 0.,
            ],
        ] {
            let (mut view, frames, values) = trace(&changes);
            for (f, v) in frames.iter().zip(&values) {
                append(&mut view, f, v, false);
            }
            assert!(view.rule_checks.is_empty());
        }
    }

    #[test]
    fn completion_timestamps_do_not_move_existing_predictions() {
        let (mut view, frames, values) = trace(&history_changes());
        for (f, v) in frames.iter().zip(&values).take(11) {
            append(&mut view, f, v, false);
        }
        let mut checks = view.rule_checks.clone();
        stamp_prepared(&mut checks, &[], 11_100_000);
        assert_eq!(checks[0].frozen_at_us, 11_100_000);
        assert!(
            checks[0]
                .forecasts
                .iter()
                .flat_map(|f| f.predictions.iter().flatten())
                .all(|p| p.issued_us == 11_100_000)
        );
        let old = checks.clone();
        stamp_prepared(&mut checks, &old, 12_100_000);
        assert_eq!(
            serde_json::to_string(&checks).unwrap(),
            serde_json::to_string(&old).unwrap()
        );
    }

    #[test]
    fn late_current_image_interpretation_gets_a_new_forecast_when_prior_episode_count_changes() {
        use crate::visual::{Observation, ObservationInput, VisualTrack};
        let (mut view, frames, values) = trace(&history_changes());
        view.visual_tracks.push(VisualTrack {
            id: 7,
            observer: "fixture".into(),
            submitted_by: "test".into(),
            key: "value".into(),
            label: "Observed state".into(),
            unit: "".into(),
            epoch: 0,
            active: true,
            observations: vec![],
            matches: vec![],
        });
        for (f, v) in frames.iter().zip(&values) {
            view.visual_tracks[0].observations.push(None);
            append(&mut view, f, v, false);
            *view.visual_tracks[0].observations.last_mut().unwrap() = Some(Observation {
                submitted_by: "test".into(),
                input: ObservationInput {
                    investigation_id: "clock".into(),
                    frame_id: f.id.clone(),
                    image_sha256: f.sha256.clone(),
                    observer: "fixture".into(),
                    key: "value".into(),
                    label: "Observed state".into(),
                    unit: "".into(),
                    value: v.clone(),
                    quality: Quality::Provisional,
                    bounds: None,
                    explanation: "Test annotation".into(),
                },
            });
            view.action_patterns =
                action_patterns::analyze(&view.frames, &view.tracks, &view.visual_tracks);
            let old = view.rule_checks.clone();
            view.rule_checks = update(&view, f.capture_interval.as_ref().unwrap().end_us + 20_000);
            stamp_prepared(
                &mut view.rule_checks,
                &old,
                f.capture_interval.as_ref().unwrap().end_us + 21_000,
            );
        }
        let check = view
            .rule_checks
            .iter()
            .find(|c| c.source == Source::External)
            .unwrap();
        assert!(check.invalidated.is_none());
        assert_eq!(check.challenges.len(), 2);
        assert!(
            check
                .challenges
                .iter()
                .all(|c| c.comparisons.iter().all(|p| p.result != Result::Unknown))
        );
        let forecasts: Vec<_> = check.forecasts.iter().filter(|f| f.number == 15).collect();
        assert_eq!(forecasts.len(), 2);
        assert_eq!(forecasts[0].prior_episodes, 3);
        assert_eq!(forecasts[1].prior_episodes, 4);
        assert!(
            forecasts[1]
                .predictions
                .iter()
                .flatten()
                .all(|p| p.issued_us == 14_022_000)
        );
    }

    #[test]
    fn conditional_fit_leaves_capture_space_under_the_actual_memory_budget() {
        let mut changes = history_changes();
        changes[10] = 0.;
        let (mut view, frames, values) = trace(&changes);
        for (f, v) in frames.iter().zip(&values).take(15) {
            append(&mut view, f, v, false);
        }
        view.rule_checks.clear();
        view.planned_bytes = 32 * 1024 * 1024;
        assert!(update(&view, 20_000_000).is_empty());
        view.planned_bytes = 1024 * 1024;
        assert!(!update(&view, 20_000_000).is_empty());
    }
}
