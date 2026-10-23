//! Input-associated field changes, not actor identity, semantics or edit authority.
use crate::{Result, runtime, runtime_neighbors::Plan, visual::digest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const POLICY: &str = "bracketed_sibling_input_response_v3";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Poll {
    pub begin_ticks: i64,
    pub end_ticks: i64,
    pub selected_foreground: bool,
    pub down: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputWindow {
    pub scope: runtime::Scope,
    pub frequency_hz: u64,
    pub expected_keys: Vec<u8>,
    pub polls: Vec<Poll>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub report: Value,
    pub input: InputWindow,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub frequency_hz: u64,
    pub begin_ticks: i64,
    pub end_ticks: i64,
}
pub fn input_matches(sample: &Sample, input: &InputWindow) -> Result<()> {
    let f = sample.frequency_hz;
    if f == 0
        || f > 1_000_000_000
        || f != input.frequency_hz
        || sample.begin_ticks < 0
        || sample.end_ticks < sample.begin_ticks
        || input.polls.len() < 2
        || input.polls.len() > 10000
        || input.expected_keys.len() > 1
        || input
            .expected_keys
            .iter()
            .any(|k| !matches!(k,32|37..=40|48..=57|65..=90))
    {
        return Err("Invalid or incompatible input/sample clocks.".into());
    }
    let gap_ok = |begin: i64, end: i64| {
        let delta = i128::from(end) - i128::from(begin);
        delta >= 0 && delta * 20 <= i128::from(f)
    };
    for p in &input.polls {
        if p.begin_ticks < 0
            || !gap_ok(p.begin_ticks, p.end_ticks)
            || p.down.len() > 255
            || p.down.windows(2).any(|keys| keys[0] >= keys[1])
        {
            return Err("Invalid sampled key state or polling duration.".into());
        }
    }
    for pair in input.polls.windows(2) {
        if pair[1].begin_ticks <= pair[0].end_ticks {
            return Err("Input polling has out-of-order timestamps.".into());
        }
    }
    let first = input
        .polls
        .iter()
        .rposition(|p| p.end_ticks <= sample.begin_ticks)
        .ok_or("No input poll before runtime sample")?;
    let last = input
        .polls
        .iter()
        .position(|p| p.begin_ticks >= sample.end_ticks)
        .ok_or("No input poll after runtime sample")?;
    if first >= last
        || input.polls[first..=last]
            .windows(2)
            .any(|p| !gap_ok(p[0].end_ticks, p[1].begin_ticks))
        || input.polls[first..=last]
            .iter()
            .any(|p| !p.selected_foreground || p.down != input.expected_keys)
    {
        return Err(
            "Runtime sample does not have continuous matching foreground/input coverage.".into(),
        );
    }
    Ok(())
}
pub fn analyze(plan: &Plan, batch: usize, observations: &[Observation]) -> Result<Value> {
    if plan.policy != crate::runtime_neighbors::POLICY || observations.len() != 14 {
        return Err("Use one frozen sibling plan and fourteen ordered observations.".into());
    }
    let paths = plan.batches.get(batch).ok_or("Missing path batch")?;
    if paths.is_empty()
        || paths.len() > runtime::MAX_REFRESH_PATHS
        || paths
            .iter()
            .map(|p| p.id())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != paths.len()
    {
        return Err("Invalid sibling path batch.".into());
    }
    for path in paths {
        path.validate(&plan.scope)?;
    }
    let candidates = paths
        .iter()
        .map(|p| {
            plan.candidates
                .iter()
                .find(|c| c.id == p.id() && c.locator == *p)
                .filter(|c| !c.edit_authority)
                .cloned()
                .ok_or_else(|| "Batch path is absent from sibling plan".to_owned())
        })
        .collect::<Result<Vec<_>>>()?;
    if plan.locations.len() != plan.candidates.len() {
        return Err("Missing enclosing field context".into());
    }
    for (candidate, location) in plan.candidates.iter().zip(&plan.locations) {
        if location.path_id != candidate.id
            || location.role_id
                != crate::runtime_location::role_id_at(&candidate.locator, location.anchor_step)?
            || location.members.len() != candidate.locator.steps.len() - location.anchor_step
            || !plan.roles.iter().any(|r| r.id == location.role_id)
        {
            return Err("Invalid enclosing field context".into());
        }
    }
    let references: Vec<_> = candidates.iter().collect();
    let mut readings: Vec<Vec<runtime::RefreshedValue>> = vec![vec![]; candidates.len()];
    let mut previous_end = None;
    let mut keys = vec![];
    let mut receipts = vec![];
    for (i, observation) in observations.iter().enumerate() {
        let report = &observation.report;
        if report["operation"] != "refresh_runtime_paths" || observation.input.scope != plan.scope {
            return Err("Response observation belongs to a different scope or operation.".into());
        }
        let sample: Sample = serde_json::from_value(
            report["pauses"]
                .as_array()
                .and_then(|p| p.last())
                .ok_or("Missing final sampling pause")?["sampling_qpc"]
                .clone(),
        )
        .map_err(|e| e.to_string())?;
        input_matches(&sample, &observation.input)?;
        if previous_end.is_some_and(|(t, f)| sample.begin_ticks <= t || sample.frequency_hz != f) {
            return Err("Runtime samples are out of order.".into());
        }
        previous_end = Some((sample.end_ticks, sample.frequency_hz));
        if [2, 5, 8, 11].contains(&i) {
            if observation.input.expected_keys.len() != 1 {
                return Err("Missing sampled action".into());
            }
            keys.push(observation.input.expected_keys.clone());
        } else if !observation.input.expected_keys.is_empty() {
            return Err("Expected released input around each action".into());
        }
        for (history, value) in readings
            .iter_mut()
            .zip(runtime::validate_refresh(&references, report)?)
        {
            history.push(value);
        }
        receipts.push(json!({"report_sha256":digest(&serde_json::to_vec(report).map_err(|e|e.to_string())?),
            "input_sha256":digest(&serde_json::to_vec(&observation.input).map_err(|e|e.to_string())?),"sample":sample}));
    }
    if keys[0] != keys[2] || keys[1] != keys[3] {
        return Err("Actions do not repeat in the declared two-action sequence.".into());
    }
    let mut fields = vec![];
    for (path_index, candidate) in candidates.iter().enumerate() {
        let readings = &readings[path_index];
        let first = &observations[0].report["checks"][path_index];
        let locations_stable = observations.iter().all(|o| {
            let check = &o.report["checks"][path_index];
            let address = |value: &Value| {
                value
                    .as_str()
                    .and_then(|s| s.strip_prefix("0x"))
                    .and_then(|s| u64::from_str_radix(s, 16).ok())
                    .is_some_and(|n| n > 0)
            };
            address(&check["field_address"])
                && check["field_address"] == first["field_address"]
                && check["snapshot_object_addresses"]
                    .as_array()
                    .is_some_and(|a| {
                        a.len() == candidate.locator.steps.len() && a.iter().all(address)
                    })
                && check["snapshot_object_addresses"] == first["snapshot_object_addresses"]
        });
        let mut state_pulses = 0;
        let mut step_pulses = 0;
        let mut quiet_stable = 0;
        let complete = readings.iter().all(|r| r.status == "read");
        if complete && locations_stable {
            for action in [2, 5, 8, 11] {
                let before = &readings[action - 1].bytes;
                let during = &readings[action].bytes;
                let after = &readings[action + 1].bytes;
                if readings[action - 2].bytes == *before && readings[action + 2].bytes == *after {
                    quiet_stable += 1;
                    if before == after && during != before {
                        state_pulses += 1;
                    }
                    if before != after && during == after {
                        step_pulses += 1;
                    }
                }
            }
        }
        let location = plan
            .locations
            .iter()
            .find(|l| l.path_id == candidate.id)
            .ok_or("Missing field context")?;
        fields.push(json!({"path_id":candidate.id,"role_id":location.role_id,
            "field_name":candidate.name,"member_path":location.members,"all_read":complete,"locations_stable":locations_stable,
            "quiet_stable_pulses":quiet_stable,"state_associated_pulses":state_pulses,"step_associated_pulses":step_pulses,
            "repeated_association":state_pulses==4 || step_pulses==4,
            "values":readings.iter().map(|r|&r.value).collect::<Vec<_>>()}));
    }
    Ok(
        json!({"policy":POLICY,"plan_sha256":digest(&serde_json::to_vec(plan).map_err(|e|e.to_string())?),
        "report_sha256":plan.report_sha256,"scope":plan.scope,"batch":batch,
        "observations":receipts,"actions":keys,"fields":fields,"edit_authority":false,
        "limitation":"Repeated input association within this short run, not game delivery, causality or actor identity. Stable addresses do not exclude reuse. Nonresponding fields may matter under different activities or sampling times. No field is promoted to a gameplay control."}),
    )
}
