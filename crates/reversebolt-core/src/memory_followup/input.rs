//! Descriptive input/memory associations using memory operation bounds, not pixels.
use crate::{
    inputs::{Focus, Window},
    memory_evidence::{Manifest, Sample},
    visual::CaptureInterval,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const POLICY: &str = "memory_input_associations_v1";
pub const LIMITATION: &str = "Post-hoc descriptions of sampled key states and adjacent memory values. Input coverage includes both uncertain memory-read bounds; it is not image exposure. Repeated intervals are not independent trials. A sampled key is not confirmed game delivery, a continuous hold or an action meaning. Timing, other inputs, drift, allocation reuse and edits can explain associations. No effect, gameplay name or edit authority is established.";
const MAX_GAP_US: u64 = 100_000;
const MAX_SPAN_US: u64 = 4_000_000;
const MAX_KEYS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unknown {
    MissingValue,
    MissingContext,
    MissingRead,
    Timing,
    Generation,
    Coverage,
    MalformedPolls,
    Focus,
    Gap,
    TooFewPolls,
    OnePressedPoll,
    MixedKeys,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Exposure {
    Key(u8),
    NoKeysDetected,
    Unknown(Unknown),
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Counts {
    pub increased: usize,
    pub decreased: usize,
    pub unchanged: usize,
}
impl Counts {
    fn add(&mut self, a: f64, b: f64) {
        if b > a {
            self.increased += 1;
        } else if b < a {
            self.decreased += 1;
        } else {
            self.unchanged += 1;
        }
    }
}

impl Evidence {
    pub fn valid(&self, values: &[Option<f64>]) -> bool {
        let pairs = values.len().saturating_sub(1);
        if self.intervals.len() != pairs
            || self.keys.len() > MAX_KEYS
            || self.omitted_keys.len() > pairs
            || self.keys.windows(2).any(|p| p[0].key >= p[1].key)
            || self.omitted_keys.windows(2).any(|p| p[0] >= p[1])
            || self
                .keys
                .iter()
                .map(|k| k.key)
                .chain(self.omitted_keys.iter().copied())
                .any(|k| k == 0 || k == 255)
            || self.unknown_pairs
                != self
                    .intervals
                    .iter()
                    .filter(|e| matches!(e, Exposure::Unknown(_)))
                    .count()
        {
            return false;
        }
        let mut quiet = Counts::default();
        let mut keys: BTreeMap<u8, Counts> = BTreeMap::new();
        for (i, e) in self.intervals.iter().enumerate() {
            if let Exposure::Unknown(_) = e {
                continue;
            }
            let Some((a, b)) = values[i].zip(values[i + 1]) else {
                return false;
            };
            if !a.is_finite() || !b.is_finite() {
                return false;
            }
            match e {
                Exposure::NoKeysDetected => quiet.add(a, b),
                Exposure::Key(key) => keys.entry(*key).or_default().add(a, b),
                _ => (),
            }
        }
        quiet == self.no_keys_detected
            && self
                .keys
                .iter()
                .all(|k| keys.get(&k.key) == Some(&k.counts))
            && keys
                .keys()
                .take(MAX_KEYS)
                .copied()
                .eq(self.keys.iter().map(|k| k.key))
            && keys
                .keys()
                .skip(MAX_KEYS)
                .copied()
                .eq(self.omitted_keys.iter().copied())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyCounts {
    pub key: u8,
    pub counts: Counts,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    /// One class for each adjacent pair in the report's memory_captures list.
    pub intervals: Vec<Exposure>,
    pub no_keys_detected: Counts,
    pub keys: Vec<KeyCounts>,
    /// Keys omitted from aggregate counts only; interval classes remain complete.
    pub omitted_keys: Vec<u8>,
    pub unknown_pairs: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Description {
    pub policy: String,
    pub limitation: String,
}
impl Default for Description {
    fn default() -> Self {
        Self {
            policy: POLICY.into(),
            limitation: LIMITATION.into(),
        }
    }
}

fn read_time(sample: &Sample, address: u64) -> Result<&CaptureInterval, Unknown> {
    sample
        .reads
        .iter()
        .find(|r| crate::memory_evidence::address(&r.receipt.address).ok() == Some(address))
        .and_then(|r| r.segment.as_ref().and(r.receipt.capture_interval.as_ref()))
        .ok_or(Unknown::MissingRead)
}

fn exposure(
    previous: &Sample,
    current: &Sample,
    address: u64,
    clock: &str,
) -> Result<Exposure, Unknown> {
    let a = read_time(previous, address)?;
    let b = read_time(current, address)?;
    let old = previous
        .input_context
        .as_ref()
        .ok_or(Unknown::MissingContext)?;
    let window = current
        .input_context
        .as_ref()
        .ok_or(Unknown::MissingContext)?;
    classify(a, b, old, window, clock)
}

fn classify(
    a: &CaptureInterval,
    b: &CaptureInterval,
    old: &Window,
    w: &Window,
    clock: &str,
) -> Result<Exposure, Unknown> {
    if clock.is_empty()
        || a.clock_id != clock
        || b.clock_id != clock
        || a.begin_us > a.end_us
        || b.begin_us > b.end_us
        || a.end_us >= b.begin_us
        || b.end_us - a.begin_us > MAX_SPAN_US
    {
        return Err(Unknown::Timing);
    }
    if old.clock_id != clock
        || w.clock_id != clock
        || w.generation.is_empty()
        || old.generation != w.generation
    {
        return Err(Unknown::Generation);
    }
    if w.requested_begin_us > a.begin_us
        || w.requested_end_us < b.end_us
        || w.monitor_started_us > a.begin_us
        || w.retained_from_us > a.begin_us
        || w.omitted_polls != 0
        || w.polls.len() > crate::inputs::MAX_WINDOW_POLLS
    {
        return Err(Unknown::Coverage);
    }
    let mut previous_end = None;
    let mut cursor = a.begin_us;
    let (mut count, mut pressed) = (0, 0);
    let mut keys = BTreeSet::new();
    for p in &w.polls {
        if p.begin_us < w.requested_begin_us
            || p.end_us > w.requested_end_us
            || p.end_us < p.begin_us
            || previous_end.is_some_and(|t| p.begin_us <= t)
            || p.down.windows(2).any(|k| k[0] >= k[1])
            || p.down.iter().any(|k| *k == 0 || *k == 255)
        {
            return Err(Unknown::MalformedPolls);
        }
        previous_end = Some(p.end_us);
        if p.begin_us < a.begin_us || p.end_us > b.end_us {
            continue;
        }
        if p.focus != Focus::SelectedForeground {
            return Err(Unknown::Focus);
        }
        if p.begin_us - cursor > MAX_GAP_US || p.end_us - p.begin_us > MAX_GAP_US {
            return Err(Unknown::Gap);
        }
        cursor = p.end_us;
        count += 1;
        pressed += usize::from(!p.down.is_empty());
        keys.extend(p.down.iter().copied());
    }
    if b.end_us - cursor > MAX_GAP_US {
        return Err(Unknown::Gap);
    }
    if count < 3 {
        return Err(Unknown::TooFewPolls);
    }
    match keys.len() {
        0 => Ok(Exposure::NoKeysDetected),
        1 if pressed >= 2 => Ok(Exposure::Key(*keys.first().unwrap())),
        1 => Err(Unknown::OnePressedPoll),
        _ => Err(Unknown::MixedKeys),
    }
}

pub fn analyze(
    manifest: &Manifest,
    address: u64,
    captures: &[usize],
    values: &[Option<f64>],
) -> Option<Evidence> {
    if !manifest.samples.iter().any(|s| s.input_context.is_some()) {
        return None;
    }
    let mut evidence = Evidence {
        intervals: vec![],
        no_keys_detected: Counts::default(),
        keys: vec![],
        omitted_keys: vec![],
        unknown_pairs: 0,
    };
    let mut by_key: BTreeMap<u8, Counts> = BTreeMap::new();
    for (i, pair) in captures.windows(2).enumerate() {
        let result = match values
            .get(i)
            .copied()
            .flatten()
            .zip(values.get(i + 1).copied().flatten())
        {
            Some((a, b)) if a.is_finite() && b.is_finite() => {
                let class = pair[0]
                    .checked_sub(1)
                    .and_then(|a| manifest.samples.get(a))
                    .zip(pair[1].checked_sub(1).and_then(|b| manifest.samples.get(b)))
                    .ok_or(Unknown::MissingRead)
                    .and_then(|(a, b)| exposure(a, b, address, &manifest.state.investigation_id));
                if let Ok(class) = class {
                    match class {
                        Exposure::Key(key) => by_key.entry(key).or_default().add(a, b),
                        Exposure::NoKeysDetected => evidence.no_keys_detected.add(a, b),
                        Exposure::Unknown(_) => (),
                    }
                }
                class
            }
            _ => Err(Unknown::MissingValue),
        };
        evidence.intervals.push(match result {
            Ok(class) => class,
            Err(reason) => {
                evidence.unknown_pairs += 1;
                Exposure::Unknown(reason)
            }
        });
    }
    for (i, (key, counts)) in by_key.into_iter().enumerate() {
        if i < MAX_KEYS {
            evidence.keys.push(KeyCounts { key, counts });
        } else {
            evidence.omitted_keys.push(key);
        }
    }
    Some(evidence)
}

#[cfg(test)]
mod tests;
