//! Descriptive input/value contrasts. No causal or editing authority is granted.
use crate::{
    inputs::{self, Focus},
    live::{LiveView, NumberTrack},
    visual::{FrameRef, Quality, VisualTrack, VisualValue},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const LIMITATION: &str = "Observed associations, not learned button meanings or causal effects. Sampled key states do not prove delivery or absence of input. Images have capture-operation bounds, not presentation times. Episodes are contiguous sampled intervals, not independent trials. Scene changes, other inputs, drift, observation errors and edits can explain differences. First-two/later comparisons are chronological descriptions, not a prospective experiment; late image annotations can revise them.";
const MAX_GAP_US: u64 = 100_000;
const MAX_SPAN_US: u64 = 4_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Ocr,
    External,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Increased,
    Decreased,
    Changed,
    Unchanged,
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Exposure {
    Key(u8),
    NoKeysDetected,
    Unknown(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interval {
    pub before_frame: String,
    pub after_frame: String,
    pub before_number: usize,
    pub after_number: usize,
    pub exposure: Exposure,
    pub before: Option<VisualValue>,
    pub after: Option<VisualValue>,
    pub outcome: Option<Outcome>,
    pub elapsed_us: Option<[u64; 2]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Episode {
    /// Inclusive indices into this evidence's intervals, never frame indices.
    pub first_interval: usize,
    pub last_interval: usize,
    pub exposure: Exposure,
    pub outcome: Outcome,
    /// Key episodes need readable no-key-detected intervals on both sides.
    pub bounded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Learning,
    NoPattern,
    Tentative,
    Contradicted,
    NoContrast,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pattern {
    pub key: u8,
    pub state: State,
    pub expected: Option<Outcome>,
    pub first_episodes: Vec<usize>,
    pub later_agree: Vec<usize>,
    pub later_disagree: Vec<usize>,
    pub no_keys_unchanged: Vec<usize>,
    pub no_keys_changed: Vec<usize>,
    pub unbounded_key_episodes: Vec<usize>,
    pub next_observation: String,
}

impl Pattern {
    pub fn brief(&self) -> Option<String> {
        let key = inputs::key_name(self.key);
        match self.state {
            State::Tentative => {
                let verb = match self.expected? {
                    Outcome::Increased => "increases",
                    Outcome::Decreased => "decreases",
                    Outcome::Changed => "changes",
                    _ => return None,
                };
                Some(format!("Often {verb} with {key} sampled · unverified"))
            }
            State::Contradicted => Some(format!("Different changes seen with {key} sampled")),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub source: Source,
    pub track: usize,
    pub intervals: Vec<Interval>,
    pub episodes: Vec<Episode>,
    pub patterns: Vec<Pattern>,
    pub limitation: String,
}

fn exposure(a: &FrameRef, b: &FrameRef) -> std::result::Result<Exposure, &'static str> {
    if a.number.checked_add(1) != Some(b.number)
        || a.epoch != b.epoch
        || a.width != b.width
        || a.height != b.height
    {
        return Err("Missing capture or changed image geometry");
    }
    let ta = a
        .capture_interval
        .as_ref()
        .ok_or("Missing capture timing")?;
    let tb = b
        .capture_interval
        .as_ref()
        .ok_or("Missing capture timing")?;
    if ta.clock_id.is_empty()
        || ta.clock_id != tb.clock_id
        || ta.end_us < ta.begin_us
        || tb.end_us < tb.begin_us
        || tb.begin_us <= ta.end_us
        || tb.end_us - ta.begin_us > MAX_SPAN_US
    {
        return Err("Invalid, overlapping or widely separated capture timing");
    }
    let previous = a
        .input_context
        .as_ref()
        .ok_or("Missing previous input context")?;
    let window = b.input_context.as_ref().ok_or("Missing input context")?;
    if window.clock_id != tb.clock_id
        || previous.clock_id != ta.clock_id
        || window.generation.is_empty()
        || previous.generation != window.generation
        || window.monitor_started_us > ta.end_us
        || window.retained_from_us > ta.end_us
        || window.omitted_polls != 0
        || window.requested_begin_us != ta.end_us
        || window.requested_end_us != tb.end_us
    {
        return Err("Input collection restarted or part of the interval is unavailable");
    }
    let mut cursor = ta.end_us;
    let mut count = 0;
    let mut pressed = 0;
    let mut keys = BTreeSet::new();
    // Do not include keys first sampled during or after the ending image operation.
    for poll in &window.polls {
        if poll.begin_us < window.requested_begin_us
            || poll.end_us < poll.begin_us
            || poll.end_us > window.requested_end_us
            || poll.begin_us < cursor
            || poll.down.windows(2).any(|k| k[0] >= k[1])
            || poll.down.iter().any(|k| *k == 0 || *k == 255)
        {
            return Err("Invalid input sample order or keys");
        }
        if poll.end_us > tb.begin_us {
            break;
        }
        if poll.focus != Focus::SelectedForeground {
            return Err("Selected window was not continuously sampled in the foreground");
        }
        if poll.begin_us - cursor > MAX_GAP_US {
            return Err("Input sampling gap");
        }
        cursor = poll.end_us;
        count += 1;
        if !poll.down.is_empty() {
            pressed += 1;
        }
        keys.extend(poll.down.iter().copied());
    }
    if count < 3 || tb.begin_us - cursor > MAX_GAP_US {
        return Err("Too few input samples or an uncovered capture edge");
    }
    match keys.len() {
        0 => Ok(Exposure::NoKeysDetected),
        1 if pressed >= 2 => Ok(Exposure::Key(*keys.first().unwrap())),
        1 => Err("Only one pressed-key sample"),
        _ => Err("Several controls sampled; individual effects are ambiguous"),
    }
}

fn outcome(a: &VisualValue, b: &VisualValue) -> Option<Outcome> {
    use Outcome::*;
    match (a, b) {
        (VisualValue::Number(x), VisualValue::Number(y)) if x.is_finite() && y.is_finite() => {
            Some(if y > x {
                Increased
            } else if y < x {
                Decreased
            } else {
                Unchanged
            })
        }
        (VisualValue::Boolean(_), VisualValue::Boolean(_))
        | (VisualValue::Text(_), VisualValue::Text(_)) => {
            Some(if a == b { Unchanged } else { Changed })
        }
        (VisualValue::Vector(x), VisualValue::Vector(y))
            if !x.is_empty() && x.len() == y.len() && x.iter().chain(y).all(|v| v.is_finite()) =>
        {
            Some(if x == y { Unchanged } else { Changed })
        }
        _ => None,
    }
}

fn merged(a: Outcome, b: Outcome) -> Outcome {
    if a == Outcome::Unchanged {
        b
    } else if b == Outcome::Unchanged || a == b {
        a
    } else {
        Outcome::Mixed
    }
}

fn evidence(
    source: Source,
    track: usize,
    frames: &[FrameRef],
    values: &[Option<VisualValue>],
) -> Evidence {
    let intervals: Vec<_> = frames
        .windows(2)
        .map(|pair| {
            let [a, b] = [&pair[0], &pair[1]];
            let value = |f: &FrameRef| {
                f.number
                    .checked_sub(1)
                    .and_then(|n| values.get(n))
                    .cloned()
                    .flatten()
            };
            let before = value(a);
            let after = value(b);
            let delta = before
                .as_ref()
                .zip(after.as_ref())
                .and_then(|(a, b)| outcome(a, b));
            let current_scope = frames.last().is_some_and(|latest| {
                latest.epoch == b.epoch
                    && latest
                        .input_context
                        .as_ref()
                        .zip(b.input_context.as_ref())
                        .is_some_and(|(x, y)| x.generation == y.generation)
            });
            let sampled = if current_scope {
                exposure(a, b)
            } else {
                Err("Earlier image geometry or input generation")
            };
            let elapsed_us = a
                .capture_interval
                .as_ref()
                .zip(b.capture_interval.as_ref())
                .and_then(|(x, y)| {
                    Some([
                        y.begin_us.checked_sub(x.end_us)?,
                        y.end_us.checked_sub(x.begin_us)?,
                    ])
                });
            Interval {
                before_frame: a.id.clone(),
                after_frame: b.id.clone(),
                before_number: a.number,
                after_number: b.number,
                exposure: match sampled {
                    Ok(e) if delta.is_some() => e,
                    Ok(_) => Exposure::Unknown(
                        "Missing, uncertain or incompatible endpoint observations".into(),
                    ),
                    Err(reason) => Exposure::Unknown(reason.into()),
                },
                before,
                after,
                outcome: delta,
                elapsed_us,
            }
        })
        .collect();
    let mut episodes: Vec<Episode> = vec![];
    for (i, interval) in intervals.iter().enumerate() {
        if matches!(interval.exposure, Exposure::Unknown(_)) {
            continue;
        }
        if let Some(last) = episodes.last_mut()
            && last.last_interval + 1 == i
            && last.exposure == interval.exposure
        {
            last.last_interval = i;
            last.outcome = merged(last.outcome, interval.outcome.unwrap());
        } else {
            episodes.push(Episode {
                first_interval: i,
                last_interval: i,
                exposure: interval.exposure.clone(),
                outcome: interval.outcome.unwrap(),
                bounded: false,
            });
        }
    }
    for episode in &mut episodes {
        let quiet = |i: usize| {
            intervals
                .get(i)
                .is_some_and(|x| x.exposure == Exposure::NoKeysDetected)
        };
        episode.bounded = episode.exposure == Exposure::NoKeysDetected
            || (episode.first_interval.checked_sub(1).is_some_and(quiet)
                && quiet(episode.last_interval + 1));
    }
    let keys: BTreeSet<_> = episodes
        .iter()
        .filter_map(|e| match e.exposure {
            Exposure::Key(k) => Some(k),
            _ => None,
        })
        .collect();
    let mut patterns = vec![];
    for key in keys {
        let mut p = Pattern {
            key,
            state: State::Learning,
            expected: None,
            first_episodes: vec![],
            later_agree: vec![],
            later_disagree: vec![],
            no_keys_unchanged: vec![],
            no_keys_changed: vec![],
            unbounded_key_episodes: vec![],
            next_observation: String::new(),
        };
        let actions: Vec<_> = episodes
            .iter()
            .enumerate()
            .filter(|(_, e)| e.exposure == Exposure::Key(key) && e.bounded)
            .collect();
        p.first_episodes = actions.iter().take(2).map(|(i, _)| *i).collect();
        if actions.len() >= 2
            && actions[0].1.outcome == actions[1].1.outcome
            && matches!(
                actions[0].1.outcome,
                Outcome::Increased | Outcome::Decreased | Outcome::Changed
            )
        {
            p.expected = Some(actions[0].1.outcome);
        }
        for (i, e) in episodes.iter().enumerate() {
            if e.exposure == Exposure::NoKeysDetected {
                if e.outcome == Outcome::Unchanged {
                    p.no_keys_unchanged.push(i);
                } else {
                    p.no_keys_changed.push(i);
                }
            } else if e.exposure == Exposure::Key(key) && !e.bounded {
                p.unbounded_key_episodes.push(i);
            }
        }
        if let Some(expected) = p.expected {
            for (i, e) in actions.iter().skip(2) {
                if e.outcome == expected {
                    p.later_agree.push(*i);
                } else {
                    p.later_disagree.push(*i);
                }
            }
            p.state = if !p.later_disagree.is_empty() {
                State::Contradicted
            } else if !p.no_keys_changed.is_empty() {
                State::NoContrast
            } else if p.no_keys_unchanged.len() >= 2 && !p.later_agree.is_empty() {
                State::Tentative
            } else {
                State::Learning
            };
        } else if actions.len() >= 2 {
            p.state = State::NoPattern;
        }
        let key_name = inputs::key_name(key);
        p.next_observation=match p.state {
            State::Tentative=>format!("Repeat {key_name} in a different situation, with comparable no-key intervals. Check for other changes before interpreting its meaning."),
            State::Contradicted=>format!("The same sampled control had a different outcome. Compare the surrounding state before and after {key_name}; do not assume a fixed effect."),
            State::NoContrast=>"This value also changes when no keys are detected. Compare timing and surrounding state before attributing an effect.".into(),
            State::NoPattern=>"The first two bounded episodes do not establish a consistent change. Investigate state, delayed effects or a different observation.".into(),
            State::Learning=>format!("Observe separated {key_name} episodes with readable no-key intervals on both sides, then check a later episode."),
        };
        patterns.push(p);
    }
    Evidence {
        source,
        track,
        intervals,
        episodes,
        patterns,
        limitation: LIMITATION.into(),
    }
}

/// Recomputed from immutable capture evidence and declared observations. No game rules.
pub fn analyze(
    frames: &[FrameRef],
    numbers: &[NumberTrack],
    visual: &[VisualTrack],
) -> Vec<Evidence> {
    let mut result = vec![];
    for t in numbers.iter().filter(|t| t.active) {
        let values: Vec<_> = t
            .samples
            .iter()
            .map(|v| v.filter(|n| n.is_finite()).map(VisualValue::Number))
            .collect();
        result.push(evidence(Source::Ocr, t.id, frames, &values));
    }
    for t in visual.iter().filter(|t| t.active) {
        let values: Vec<_> = t
            .observations
            .iter()
            .map(|v| {
                v.as_ref()
                    .filter(|o| o.input.quality == Quality::Provisional)
                    .and_then(|o| o.input.value.clone())
            })
            .collect();
        result.push(evidence(Source::External, t.id, frames, &values));
    }
    result
}

pub fn find(view: &LiveView, source: Source, track: usize) -> Option<&Evidence> {
    view.action_patterns
        .iter()
        .find(|e| e.source == source && e.track == track)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        inputs::{Buffer, Poll},
        visual::CaptureInterval,
    };
    pub(crate) fn fixture(
        keys: &[u8],
        changes: &[f64],
    ) -> (Vec<FrameRef>, Vec<Option<VisualValue>>) {
        assert_eq!(keys.len(), changes.len());
        let mut frames = vec![];
        let mut values = vec![Some(VisualValue::Number(100.))];
        let mut n = 100.;
        for i in 0..=keys.len() {
            let begin = i as u64 * 1_000_000;
            let end = begin + 1000;
            let prior = if i == 0 {
                0
            } else {
                (i as u64 - 1) * 1_000_000 + 1000
            };
            let mut buffer = Buffer::new("clock".into(), "owner".into(), 0);
            for t in (prior + 10..end).step_by(16_000) {
                let key = if i == 0 { 0 } else { keys[i - 1] };
                buffer
                    .push(Poll {
                        begin_us: t,
                        end_us: t + 1,
                        focus: Focus::SelectedForeground,
                        down: if key == 0 { vec![] } else { vec![key] },
                    })
                    .unwrap();
            }
            frames.push(FrameRef {
                sampling: None,
                id: format!("f{i}"),
                number: i + 1,
                sha256: format!("hash{i}"),
                width: 100,
                height: 100,
                epoch: 0,
                started_unix_ms: 0,
                capture_interval: Some(CaptureInterval {
                    clock_id: "clock".into(),
                    begin_us: begin,
                    end_us: end,
                }),
                input_context: buffer.window(prior, end),
            });
            if i > 0 {
                n += changes[i - 1];
                values.push(Some(VisualValue::Number(n)));
            }
        }
        (frames, values)
    }
    fn result(keys: &[u8], changes: &[f64]) -> Evidence {
        let (f, v) = fixture(keys, changes);
        evidence(Source::Ocr, 0, &f, &v)
    }
    #[test]
    fn separate_episodes_require_later_agreement_and_keep_the_exact_evidence() {
        let e = result(&[0, 68, 0, 68, 0, 68, 0], &[0., -1., 0., -1., 0., -1., 0.]);
        let p = &e.patterns[0];
        assert_eq!(p.state, State::Tentative);
        assert_eq!(p.expected, Some(Outcome::Decreased));
        assert_eq!(p.first_episodes, vec![1, 3]);
        assert_eq!(p.later_agree, vec![5]);
        assert_eq!(e.intervals[1].before_frame, "f1");
        assert_eq!(p.no_keys_unchanged.len(), 4);
        assert!(p.brief().unwrap().contains("unverified"));
        let e = result(&[0, 68, 0, 68, 0], &[0., -1., 0., -1., 0.]);
        assert_eq!(e.patterns[0].state, State::Learning);
    }
    #[test]
    fn later_no_effect_contradicts_without_refitting_and_drift_removes_contrast() {
        let e = result(&[0, 68, 0, 68, 0, 68, 0], &[0., -1., 0., -1., 0., 0., 0.]);
        let p = &e.patterns[0];
        assert_eq!(p.state, State::Contradicted);
        assert_eq!(p.expected, Some(Outcome::Decreased));
        assert_eq!(p.later_disagree, vec![5]);
        let drift = result(&[0, 68, 0, 68, 0, 68, 0], &[-1.; 7]);
        assert_eq!(drift.patterns[0].state, State::NoContrast);
        assert_eq!(drift.patterns[0].no_keys_changed.len(), 4);
        let no_effect = result(&[0, 68, 0, 68, 0, 68, 0], &[0.; 7]);
        assert_eq!(no_effect.patterns[0].state, State::NoPattern);
        let mixed = result(&[0, 68, 0, 68, 0, 68, 0], &[0., -1., 0., 1., 0., -1., 0.]);
        assert_eq!(mixed.patterns[0].state, State::NoPattern);
    }
    #[test]
    fn sustained_holds_are_one_episode_and_return_to_start_is_not_unchanged() {
        let e = result(&[0, 68, 68, 68, 0], &[0., -1., 1., 0., 0.]);
        assert_eq!(e.episodes.len(), 3);
        assert_eq!(e.episodes[1].outcome, Outcome::Mixed);
        assert_eq!(e.patterns[0].first_episodes.len(), 1);
        let unbounded = result(&[68, 68, 0, 68], &[-1., -1., 0., -1.]);
        assert!(unbounded.patterns[0].first_episodes.is_empty());
        assert_eq!(unbounded.patterns[0].unbounded_key_episodes.len(), 2);
    }
    #[test]
    fn focus_gaps_chords_and_missing_values_break_episodes_instead_of_becoming_idle() {
        let (base, v) = fixture(&[0, 68, 0, 68, 0, 68, 0], &[0., -1., 0., -1., 0., -1., 0.]);
        for mode in 0..7 {
            let mut f = base.clone();
            let mut v = v.clone();
            let w = f[4].input_context.as_mut().unwrap();
            match mode {
                0 => {
                    w.polls[3].focus = Focus::OutsideSelected;
                    w.polls[3].down.clear();
                }
                1 => {
                    w.polls.drain(5..20);
                }
                2 => {
                    w.polls[3].down.push(69);
                }
                3 => {
                    w.omitted_polls = 1;
                }
                4 => {
                    w.generation = "resumed".into();
                }
                5 => {
                    v[4] = None;
                }
                _ => {
                    f[4].number += 1;
                }
            }
            let e = evidence(Source::Ocr, 0, &f, &v);
            assert!(
                matches!(e.intervals[3].exposure, Exposure::Unknown(_)),
                "mode {mode}"
            );
            assert_ne!(e.patterns[0].state, State::Tentative);
        }
    }
    #[test]
    fn keys_during_ending_capture_do_not_get_credit_and_new_scope_retires_old_patterns() {
        let (mut f, v) = fixture(&[0, 68, 0, 68, 0, 68, 0], &[0., -1., 0., -1., 0., -1., 0.]);
        let begin = f[2].capture_interval.as_ref().unwrap().begin_us;
        let w = f[2].input_context.as_mut().unwrap();
        for p in &mut w.polls {
            p.down.clear();
        }
        w.polls.push(Poll {
            begin_us: begin + 10,
            end_us: begin + 11,
            focus: Focus::SelectedForeground,
            down: vec![68],
        });
        assert_eq!(exposure(&f[1], &f[2]).unwrap(), Exposure::NoKeysDetected);
        f.last_mut()
            .unwrap()
            .input_context
            .as_mut()
            .unwrap()
            .generation = "new-owner".into();
        assert!(evidence(Source::Ocr, 0, &f, &v).patterns.is_empty());
    }
    #[test]
    fn boolean_and_vector_changes_use_the_same_contrast_without_inventing_direction() {
        let (f, _) = fixture(&[0, 68, 0, 68, 0, 68, 0], &[0.; 7]);
        let values = [false, false, true, true, false, false, true, true];
        let v = values.map(|b| Some(VisualValue::Boolean(b)));
        let e = evidence(Source::External, 7, &f, &v);
        assert_eq!(e.patterns[0].expected, Some(Outcome::Changed));
        assert_eq!(e.patterns[0].state, State::Tentative);
        assert_eq!(
            outcome(
                &VisualValue::Vector(vec![1., 2.]),
                &VisualValue::Vector(vec![2., 3.])
            ),
            Some(Outcome::Changed)
        );
        assert_eq!(
            outcome(
                &VisualValue::Vector(vec![f64::NAN]),
                &VisualValue::Vector(vec![2.])
            ),
            None
        );
        assert_eq!(
            outcome(&VisualValue::Number(1.), &VisualValue::Text("1".into())),
            None
        );
    }
}
