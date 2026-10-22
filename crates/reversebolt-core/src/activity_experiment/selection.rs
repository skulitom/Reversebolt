//! Choose a bounded next input experiment from observed use, without key semantics.
use super::*;
use std::cmp::Reverse;

pub const POLICY: &str = "untried_observed_key_v1";
pub const LIMITATION: &str = "Sampling-based ordering, not predicted information gain or key meaning. Each key uses its latest complete source window; counts are not accumulated across overlapping windows. No new input, capture or memory access occurs during selection. Existing job authorization and experiment checks still apply.";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub key: u8,
    pub source_frame: String,
    pub source_number: usize,
    pub source_metadata_sha256: String,
    pub pressed_polls: usize,
    pub single_key_polls: usize,
}
impl Choice {
    pub(super) fn from_frame(frame: &FrameRef, key: u8) -> Result<Self> {
        sampled(frame, key)?;
        let polls = &frame.input_context.as_ref().unwrap().polls;
        Ok(Self {
            key,
            source_frame: frame.id.clone(),
            source_number: frame.number,
            source_metadata_sha256: visual::digest(
                &serde_json::to_vec(frame).map_err(|e| e.to_string())?,
            ),
            pressed_polls: polls.iter().filter(|p| p.down.contains(&key)).count(),
            single_key_polls: polls.iter().filter(|p| p.down == [key]).count(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Suggestion {
    pub policy: String,
    pub candidates: Vec<Choice>,
    pub attempted_keys: Vec<u8>,
    pub next: Option<Choice>,
    pub unavailable: Option<String>,
    pub limitation: String,
}

pub fn suggest(live: &LiveView, attempted: &[u8]) -> Result<Suggestion> {
    let mut attempted_keys = attempted.to_vec();
    attempted_keys.sort_unstable();
    attempted_keys.dedup();
    let mut candidates = eligible_keys(live)
        .into_iter()
        .map(|key| Choice::from_frame(source(live, key).unwrap(), key))
        .collect::<Result<Vec<_>>>()?;
    // Prefer input observed alone, then stronger sampling, then a recent source.
    // The final key-code tie break is deterministic, not a gameplay preference.
    candidates.sort_by_key(|c| {
        (
            Reverse(c.single_key_polls),
            Reverse(c.pressed_polls),
            Reverse(c.source_number),
            c.key,
        )
    });
    let mut next = candidates
        .iter()
        .find(|c| !attempted_keys.contains(&c.key))
        .cloned();
    let unavailable = if let Some(choice) = &next {
        Plan::new(live, choice.key, "selection-check".into()).err()
    } else if candidates.is_empty() {
        Some("Play briefly while Reversebolt observes input first.".into())
    } else {
        Some("All observed keys have already been proposed in this batch. Observe different input or begin a fresh batch.".into())
    };
    if unavailable.is_some() {
        next = None;
    }
    Ok(Suggestion {
        policy: POLICY.into(),
        candidates,
        attempted_keys,
        next,
        unavailable,
        limitation: LIMITATION.into(),
    })
}
