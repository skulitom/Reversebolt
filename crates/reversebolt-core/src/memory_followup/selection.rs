//! Select and prospectively check descriptive key/quiet contrasts, never controls.
use super::input::{self, Evidence, Exposure};
use crate::{Result, memory_evidence::Manifest, memory_trajectories as m};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const POLICY: &str = "memory_input_contrast_v1";
pub const MAX_SELECTED_GROUPS: usize = 4;
pub const LIMITATION: &str = "Selection from retained memory proposals and sampled input, not a whole-process search or gameplay identification. At least two separated key episodes and unchanged quiet intervals support a descriptive expectation. These are observed intervals, not independent trials or delivered game actions. The expectation is fixed before fresh sampling; consistency still grants no causal, allocation-lifetime, naming or editing authority. Omitted and unavailable evidence can conceal other behavior.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Increased,
    Decreased,
    Changed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Expectation {
    pub policy: String,
    pub key: u8,
    pub direction: Direction,
    pub source_memory_samples: usize,
    /// Inclusive indices into the source report's adjacent memory-capture pairs.
    pub source_episodes: Vec<[usize; 2]>,
    pub source_quiet_pairs: Vec<usize>,
    pub source_unbounded_episodes: usize,
    pub source_unknown_pairs: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    pub verdict: m::Verdict,
    pub agreeing_episodes: Vec<[usize; 2]>,
    pub disagreeing_episodes: Vec<[usize; 2]>,
    pub quiet_unchanged_pairs: Vec<usize>,
    pub quiet_changed_pairs: Vec<usize>,
    pub unbounded_episodes: usize,
    pub unknown_pairs: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    pub member: usize,
    pub expectation: Expectation,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub group: usize,
    pub members: Vec<Member>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub policy: String,
    pub investigation_id: String,
    pub batch: usize,
    pub captures: usize,
    pub report_sha256: String,
    pub evidence_manifest_sha256: String,
    pub examined_members: usize,
    pub eligible_members: usize,
    pub eligible_groups: usize,
    pub omitted_groups: usize,
    pub groups: Vec<Group>,
    pub limitation: String,
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

fn delta(a: Option<f64>, b: Option<f64>) -> Option<i8> {
    let (a, b) = a.zip(b)?;
    if !a.is_finite() || !b.is_finite() {
        return None;
    }
    Some(if b > a {
        1
    } else if b < a {
        -1
    } else {
        0
    })
}
fn quiet(e: &Evidence, values: &[Option<f64>]) -> (Vec<usize>, Vec<usize>) {
    let (mut unchanged, mut changed) = (vec![], vec![]);
    for (i, class) in e.intervals.iter().enumerate() {
        if *class == Exposure::NoKeysDetected {
            match delta(values[i], values[i + 1]) {
                Some(0) => unchanged.push(i),
                Some(_) => changed.push(i),
                _ => (),
            }
        }
    }
    (unchanged, changed)
}
type Episode = ([usize; 2], Option<Direction>);
fn episodes(e: &Evidence, values: &[Option<f64>], key: u8) -> (Vec<Episode>, usize) {
    let mut spans = vec![];
    let mut unbounded = 0;
    let mut i = 0;
    while i < e.intervals.len() {
        if e.intervals[i] != Exposure::Key(key) {
            i += 1;
            continue;
        }
        let first = i;
        while i + 1 < e.intervals.len() && e.intervals[i + 1] == Exposure::Key(key) {
            i += 1;
        }
        let last = i;
        if first == 0
            || last + 1 >= e.intervals.len()
            || e.intervals[first - 1] != Exposure::NoKeysDetected
            || e.intervals[last + 1] != Exposure::NoKeysDetected
        {
            unbounded += 1;
        } else {
            let (mut up, mut down) = (false, false);
            for j in first..=last {
                match delta(values[j], values[j + 1]) {
                    Some(1) => up = true,
                    Some(-1) => down = true,
                    _ => (),
                }
            }
            let direction = match (up, down) {
                (true, false) => Some(Direction::Increased),
                (false, true) => Some(Direction::Decreased),
                (true, true) => Some(Direction::Changed),
                _ => None,
            };
            spans.push(([first, last], direction));
        }
        i += 1;
    }
    (spans, unbounded)
}
impl Expectation {
    pub fn valid(&self) -> bool {
        let n = self.source_memory_samples;
        self.policy == POLICY
            && (1..=254).contains(&self.key)
            && (m::MIN_SAMPLES..=20).contains(&n)
            && (2..=9).contains(&self.source_episodes.len())
            && (2..=19).contains(&self.source_quiet_pairs.len())
            && self.source_quiet_pairs.iter().all(|i| *i < n - 1)
            && self
                .source_quiet_pairs
                .iter()
                .all(|i| self.source_episodes.iter().all(|[a, b]| i < a || i > b))
            && self.source_quiet_pairs.windows(2).all(|p| p[0] < p[1])
            && self.source_episodes.windows(2).all(|p| p[0][1] < p[1][0])
            && self.source_episodes.iter().all(|[a, b]| {
                *a > 0
                    && a <= b
                    && *b < n - 2
                    && self.source_quiet_pairs.contains(&(a - 1))
                    && self.source_quiet_pairs.contains(&(b + 1))
            })
            && self.source_unbounded_episodes < n
            && self.source_unknown_pairs < n
    }
}

/// Fit only the complete source record. No fresh outcomes participate.
pub fn fit(e: &Evidence, values: &[Option<f64>]) -> Option<Expectation> {
    if values.len() < m::MIN_SAMPLES || !e.valid(values) {
        return None;
    }
    let (unchanged, changed) = quiet(e, values);
    if unchanged.len() < 2 || !changed.is_empty() {
        return None;
    }
    let keys: BTreeSet<_> = e
        .intervals
        .iter()
        .filter_map(|c| {
            if let Exposure::Key(k) = c {
                Some(*k)
            } else {
                None
            }
        })
        .collect();
    let mut eligible = vec![];
    for key in keys {
        let (spans, unbounded) = episodes(e, values, key);
        if spans.len() < 2 || spans.iter().any(|(_, d)| d.is_none()) {
            continue;
        }
        let first = spans[0].1.unwrap();
        let direction = if spans.iter().all(|(_, d)| *d == Some(first)) {
            first
        } else {
            Direction::Changed
        };
        let expectation = Expectation {
            policy: POLICY.into(),
            key,
            direction,
            source_memory_samples: values.len(),
            source_episodes: spans.into_iter().map(|(s, _)| s).collect(),
            source_quiet_pairs: unchanged.clone(),
            source_unbounded_episodes: unbounded,
            source_unknown_pairs: e.unknown_pairs,
        };
        if expectation.valid() {
            eligible.push(expectation);
        }
    }
    // More separated, bounded episodes first; retain one key per typed member.
    eligible.sort_by_key(|e| {
        (
            std::cmp::Reverse(e.source_episodes.len()),
            e.source_unknown_pairs,
            e.key,
        )
    });
    eligible.into_iter().next()
}

pub fn check(expected: &Expectation, e: Option<&Evidence>, values: &[Option<f64>]) -> Check {
    let mut result = Check {
        verdict: m::Verdict::Insufficient,
        agreeing_episodes: vec![],
        disagreeing_episodes: vec![],
        quiet_unchanged_pairs: vec![],
        quiet_changed_pairs: vec![],
        unbounded_episodes: 0,
        unknown_pairs: values.len().saturating_sub(1),
    };
    let Some(e) = e.filter(|e| e.valid(values)) else {
        return result;
    };
    if !expected.valid() {
        return result;
    }
    result.unknown_pairs = e.unknown_pairs;
    (result.quiet_unchanged_pairs, result.quiet_changed_pairs) = quiet(e, values);
    let (spans, unbounded) = episodes(e, values, expected.key);
    result.unbounded_episodes = unbounded;
    for (span, direction) in spans {
        if direction
            .is_some_and(|d| expected.direction == Direction::Changed || d == expected.direction)
        {
            result.agreeing_episodes.push(span);
        } else {
            result.disagreeing_episodes.push(span);
        }
    }
    result.verdict =
        if !result.disagreeing_episodes.is_empty() || !result.quiet_changed_pairs.is_empty() {
            m::Verdict::Contradicted
        } else if result.agreeing_episodes.len() >= 2 && result.quiet_unchanged_pairs.len() >= 2 {
            m::Verdict::Consistent
        } else {
            m::Verdict::Insufficient
        };
    result
}

pub fn source_values(member: &m::Member) -> Vec<Option<f64>> {
    member
        .selection
        .values
        .iter()
        .copied()
        .map(Some)
        .chain(member.assessment.values.iter().copied())
        .collect()
}
pub fn source_captures(report: &m::Report) -> Vec<usize> {
    report
        .selection_captures
        .iter()
        .chain(&report.assessment_captures)
        .copied()
        .collect()
}
/// Index read times once. Numeric rivals share timing classifications, not values.
pub struct Source {
    classes: BTreeMap<u64, Vec<Exposure>>,
}
impl Source {
    pub fn new(report: &m::Report, manifest: &Manifest) -> Self {
        let captures = source_captures(report);
        let addresses: BTreeSet<_> = report
            .groups
            .iter()
            .flat_map(|g| &g.members)
            .map(|m| m.chunk_address)
            .collect();
        let times: BTreeMap<_, _> = manifest
            .samples
            .iter()
            .flat_map(|s| {
                s.reads.iter().filter_map(|r| {
                    let address = crate::memory_evidence::address(&r.receipt.address).ok()?;
                    if !addresses.contains(&address) {
                        return None;
                    }
                    Some((
                        (s.number, address),
                        r.segment.as_ref().and(r.receipt.capture_interval.as_ref()),
                    ))
                })
            })
            .collect();
        let classes = addresses
            .into_iter()
            .map(|address| {
                let classes = captures
                    .windows(2)
                    .map(|pair| {
                        let classify = || {
                            let a = times
                                .get(&(pair[0], address))
                                .and_then(|t| *t)
                                .ok_or(input::Unknown::MissingRead)?;
                            let b = times
                                .get(&(pair[1], address))
                                .and_then(|t| *t)
                                .ok_or(input::Unknown::MissingRead)?;
                            let context = |n: usize| {
                                manifest
                                    .samples
                                    .get(n.checked_sub(1)?)
                                    .and_then(|s| s.input_context.as_ref())
                            };
                            let old = context(pair[0]).ok_or(input::Unknown::MissingContext)?;
                            let new = context(pair[1]).ok_or(input::Unknown::MissingContext)?;
                            input::classify(a, b, old, new, &manifest.state.investigation_id)
                        };
                        classify().unwrap_or_else(Exposure::Unknown)
                    })
                    .collect();
                (address, classes)
            })
            .collect();
        Self { classes }
    }
    pub fn expectation(&self, member: &m::Member) -> Option<Expectation> {
        if member.chunk_invalidated_in_recording {
            return None;
        }
        let values = source_values(member);
        fit(
            &input::from_exposures(self.classes.get(&member.chunk_address)?, &values),
            &values,
        )
    }
}
pub fn recommend(report: &m::Report, manifest: &Manifest, hash: &str) -> Result<Recommendation> {
    let captures = source_captures(report);
    if manifest.state.investigation_id != report.investigation_id
        || manifest.state.coverage.batch != report.batch
        || manifest.state.sample_count != report.capture_count
        || manifest
            .samples
            .iter()
            .filter(|s| s.memory_sample)
            .map(|s| s.number)
            .collect::<Vec<_>>()
            != captures
    {
        return Err("Memory selection source scope or sample membership changed".into());
    }
    let source = Source::new(report, manifest);
    let mut groups = vec![];
    let mut examined = 0;
    let mut eligible_members = 0;
    for (group, g) in report.groups.iter().enumerate() {
        let mut members = vec![];
        for (member, m) in g.members.iter().enumerate() {
            examined += 1;
            if let Some(expectation) = source.expectation(m) {
                members.push(Member {
                    member,
                    expectation,
                });
            }
        }
        eligible_members += members.len();
        if !members.is_empty() {
            groups.push(Group { group, members });
        }
    }
    // Numeric rivals in one group never multiply its evidence score.
    groups.sort_by_key(|g| {
        let strength = g
            .members
            .iter()
            .map(|m| {
                (
                    m.expectation.source_episodes.len(),
                    m.expectation.source_quiet_pairs.len(),
                )
            })
            .max()
            .unwrap();
        (std::cmp::Reverse(strength), g.group)
    });
    let eligible_groups = groups.len();
    groups.truncate(MAX_SELECTED_GROUPS);
    Ok(Recommendation {
        policy: POLICY.into(),
        investigation_id: report.investigation_id.clone(),
        batch: report.batch,
        captures: report.capture_count,
        report_sha256: hash.into(),
        evidence_manifest_sha256: report.manifest_sha256.clone(),
        examined_members: examined,
        eligible_members,
        eligible_groups,
        omitted_groups: eligible_groups - groups.len(),
        groups,
        limitation: LIMITATION.into(),
    })
}

#[cfg(test)]
mod tests;
