//! Passive image/input associations for choosing what to investigate next.
//! No key meanings, gameplay labels, bounds, causal claims or replay authority.
use crate::{
    Result,
    action_patterns::{self, Exposure},
    visual::FrameRef,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const LIMITATION: &str = "Passive associations only. Key-state samples do not prove game delivery; mouse movement, gamepads and sound are not sampled. Image differences can be caused by camera motion, animation, menus, scene transitions or other inputs. Episodes are contiguous observations, not independent experiments. No key meaning, gameplay property, executable plan or edit authority is established.";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Measurement {
    pub difference: Option<f64>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interval {
    pub before: String,
    pub after: String,
    pub before_sha256: String,
    pub after_sha256: String,
    pub exposure: Exposure,
    pub elapsed_us: Option<[u64; 2]>,
    pub measurement: Measurement,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Episode {
    pub first_interval: usize,
    pub last_interval: usize,
    pub exposure: Exposure,
    pub bounded_by_quiet: bool,
    pub median_difference: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub key: u8,
    pub key_label: String,
    pub episodes: Vec<usize>,
    pub bounded_episodes: Vec<usize>,
    pub median_difference: Option<f64>,
    pub quiet_median_difference: Option<f64>,
    pub next_observation: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub metric: String,
    pub intervals: Vec<Interval>,
    pub episodes: Vec<Episode>,
    pub actions: Vec<Action>,
    pub limitation: String,
}
fn median(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut values: Vec<_> = values.collect();
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some((values[(values.len() - 1) / 2] + values[values.len() / 2]) / 2.)
}
pub fn analyze(frames: &[FrameRef], measurements: Vec<Measurement>) -> Result<Evidence> {
    if !(2..=crate::live::MAX_SAMPLES).contains(&frames.len())
        || measurements.len() + 1 != frames.len()
        || frames.windows(2).any(|f| f[0].number >= f[1].number)
        || frames.iter().map(|f| &f.id).collect::<BTreeSet<_>>().len() != frames.len()
        || measurements.iter().any(|m| {
            m.difference
                .is_some_and(|v| !v.is_finite() || !(0. ..=255.).contains(&v))
                || (m.difference.is_some() == m.error.is_some())
        })
    {
        return Err("Supply 2–48 ordered distinct frames with one valid measurement or failure per interval.".into());
    }
    let intervals: Vec<_> = frames
        .windows(2)
        .zip(measurements)
        .map(|(f, measurement)| {
            let elapsed_us = f[0]
                .capture_interval
                .as_ref()
                .zip(f[1].capture_interval.as_ref())
                .and_then(|(a, b)| {
                    (a.clock_id == b.clock_id
                        && a.begin_us <= a.end_us
                        && a.end_us < b.begin_us
                        && b.begin_us <= b.end_us)
                        .then(|| [b.begin_us - a.end_us, b.end_us - a.begin_us])
                });
            let exposure = action_patterns::exposure(&f[0], &f[1])
                .map_err(str::to_owned)
                .unwrap_or_else(Exposure::Unknown);
            Interval {
                before: f[0].id.clone(),
                after: f[1].id.clone(),
                before_sha256: f[0].sha256.clone(),
                after_sha256: f[1].sha256.clone(),
                exposure,
                elapsed_us,
                measurement,
            }
        })
        .collect();
    let mut episodes: Vec<Episode> = vec![];
    for (index, interval) in intervals.iter().enumerate() {
        if let Some(last) = episodes.last_mut()
            && !matches!(interval.exposure, Exposure::Unknown(_))
            && last.exposure == interval.exposure
        {
            last.last_interval = index;
        } else {
            episodes.push(Episode {
                first_interval: index,
                last_interval: index,
                exposure: interval.exposure.clone(),
                bounded_by_quiet: false,
                median_difference: None,
            });
        }
    }
    for episode in &mut episodes {
        let start = episode.first_interval;
        let end = episode.last_interval;
        episode.bounded_by_quiet = matches!(episode.exposure, Exposure::Key(_))
            && start > 0
            && end + 1 < intervals.len()
            && matches!(intervals[start - 1].exposure, Exposure::NoKeysDetected)
            && matches!(intervals[end + 1].exposure, Exposure::NoKeysDetected)
            && intervals[start - 1].measurement.difference.is_some()
            && intervals[end + 1].measurement.difference.is_some();
        if intervals[start..=end]
            .iter()
            .all(|i| i.measurement.difference.is_some())
        {
            episode.median_difference = median(
                intervals[start..=end]
                    .iter()
                    .filter_map(|i| i.measurement.difference),
            );
        }
    }
    let quiet = median(
        intervals
            .iter()
            .filter(|i| matches!(i.exposure, Exposure::NoKeysDetected))
            .filter_map(|i| i.measurement.difference),
    );
    let keys: BTreeSet<_> = intervals
        .iter()
        .filter_map(|i| {
            if let Exposure::Key(k) = i.exposure {
                Some(k)
            } else {
                None
            }
        })
        .collect();
    let actions=keys.into_iter().map(|key| {
        let selected:Vec<_> = episodes.iter().enumerate().filter_map(|(i,e)| (e.exposure==Exposure::Key(key)).then_some(i)).collect();
        let bounded:Vec<_>=selected.iter().copied().filter(|i|episodes[*i].bounded_by_quiet && episodes[*i].median_difference.is_some()).collect();
        Action {key,key_label:crate::inputs::key_name(key),median_difference:median(bounded.iter().filter_map(|i|episodes[*i].median_difference)),
            next_observation: if bounded.len()<2 {"Observe another separated action and quiet interval; current coverage is insufficient for a repeatable test."}
                else {"Inspect the cited before/after images to identify what changed and test whether that consequence fits a grounded hypothesis. Do not infer the key's meaning from its name or this metric."}.into(),
            episodes:selected,bounded_episodes:bounded,quiet_median_difference:quiet}
    }).collect();
    Ok(Evidence {
        metric: crate::runtime_tests::METRIC_ID.into(),
        intervals,
        episodes,
        actions,
        limitation: LIMITATION.into(),
    })
}

#[cfg(test)]
mod tests;
