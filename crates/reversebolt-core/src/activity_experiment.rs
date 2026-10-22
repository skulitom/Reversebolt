//! Input-only investigation before a field or key meaning is known.
use crate::{
    Result,
    inputs::{Buffer, Focus},
    live::{LiveView, ProcessInfo},
    runtime_tests::{Action, InputWitness},
    visual::{self, FrameRef},
};
use serde::{Deserialize, Serialize};
pub mod selection;

pub const STEPS: usize = 10;
pub const HOLD_MS: u32 = 250;
pub const POLICY: &str = "quiet_bracketed_observed_key_v1";
pub const LIMITATION: &str = "Four repetitions of a previously sampled key, separated by quiet observations. Input can change the game. Samples and Windows delivery receipts do not establish key meaning, a gameplay property, world rollback or memory-edit authority.";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Review {
    pub experiment: String,
    pub job: u64,
    pub frame_ids: Vec<String>,
    pub model: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub source: Option<crate::play_context::Source>,
    pub context: Option<crate::play_context::Context>,
    pub receipt_sha256: String,
}

pub fn action(index: usize, key: u8) -> Option<Action> {
    (matches!(index, 2 | 4 | 6 | 8)).then_some(Action {
        keys: vec![key],
        hold_ms: HOLD_MS,
    })
}
fn allowed(key: u8) -> bool {
    matches!(key,32|37..=40|48..=57|65..=90)
}
fn sampled(frame: &FrameRef, key: u8) -> Result<()> {
    let w = frame
        .input_context
        .as_ref()
        .ok_or("No sampled input source")?;
    let c = frame.capture_interval.as_ref().ok_or("No capture clock")?;
    if w.clock_id != c.clock_id
        || w.generation.is_empty()
        || w.requested_begin_us > w.requested_end_us
        || w.requested_end_us != c.end_us
        || c.begin_us > c.end_us
        || w.requested_begin_us > c.begin_us
        || w.monitor_started_us > w.requested_begin_us
        || w.retained_from_us > w.requested_begin_us
        || w.omitted_polls != 0
        || w.polls.len() > crate::inputs::MAX_WINDOW_POLLS
    {
        return Err("Incomplete source input coverage".into());
    }
    let mut buffer = Buffer::new(
        w.clock_id.clone(),
        w.generation.clone(),
        w.monitor_started_us,
    );
    for p in &w.polls {
        buffer.push(p.clone())?;
    }
    let rebuilt = buffer
        .window(w.requested_begin_us, w.requested_end_us)
        .unwrap();
    if rebuilt.polls != w.polls
        || rebuilt.largest_unsampled_us != w.largest_unsampled_us
        || w.largest_unsampled_us > 50_000
        || w.polls
            .iter()
            .any(|p| p.focus != Focus::SelectedForeground || p.end_us - p.begin_us > 50_000)
        || w.polls.iter().filter(|p| p.down.contains(&key)).count() < 2
    {
        return Err(
            "A completely sampled use of this key in this observation batch is required".into(),
        );
    }
    Ok(())
}
pub fn eligible_keys(live: &LiveView) -> Vec<u8> {
    let Some(last) = live.frames.last() else {
        return vec![];
    };
    (1..=254)
        .filter(|k| {
            allowed(*k)
                && live.frames.iter().rev().any(|f| {
                    f.epoch == last.epoch
                        && f.width == last.width
                        && f.height == last.height
                        && sampled(f, *k).is_ok()
                })
        })
        .collect()
}

fn source(live: &LiveView, key: u8) -> Option<&FrameRef> {
    let last = live.frames.last()?;
    live.frames.iter().rev().find(|f| {
        f.epoch == last.epoch
            && f.width == last.width
            && f.height == last.height
            && sampled(f, key).is_ok()
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub policy: String,
    pub investigation: String,
    pub batch: usize,
    pub process: ProcessInfo,
    pub first_number: usize,
    pub key: u8,
    pub source: FrameRef,
    pub request: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observer_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<selection::Suggestion>,
}
impl Plan {
    pub fn new(live: &LiveView, key: u8, request: String) -> Result<Self> {
        let source = source(live, key).ok_or("That key has no complete sampled source.")?;
        let plan = Self {
            policy: POLICY.into(),
            investigation: live.investigation_id.clone(),
            batch: live.coverage.batch,
            process: live.process.clone(),
            first_number: live.sample_count + 1,
            key,
            source: source.clone(),
            request,
            observer_model: None,
            selection: None,
        };
        plan.check(live, 0)?;
        if live.sample_count + STEPS > crate::live::MAX_SAMPLES
            || live.coverage.image_budget_bytes + STEPS as u64 * visual::MAX_IMAGE_BYTES as u64
                > crate::coverage::MAX_IMAGE_DISK_BYTES
        {
            return Err("This investigation has no room for ten response images. Start a fresh observation batch.".into());
        }
        Ok(plan)
    }
    pub fn binding(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.investigation, self.batch, self.process.created, self.process.window
        )
    }
    pub fn check(&self, live: &LiveView, index: usize) -> Result<()> {
        if index >= STEPS {
            return Err("Invalid input capture step".into());
        }
        self.check_next(live, index)
    }
    pub fn check_completed(&self, live: &LiveView) -> Result<()> {
        self.check_next(live, STEPS)
    }
    fn check_next(&self, live: &LiveView, index: usize) -> Result<()> {
        if self.policy != POLICY
            || !allowed(self.key)
            || self.first_number == 0
            || self.first_number.checked_add(index) != Some(live.sample_count + 1)
            || !live.connected
            || self.investigation != live.investigation_id
            || self.batch != live.coverage.batch
            || self.process.pid != live.process.pid
            || self.process.created != live.process.created
            || self.process.session != live.process.session
            || self.process.window != live.process.window
            || self.process.window == 0
            || self.request.is_empty()
            || self.request.len() > 200
            || self.observer_model.as_ref().is_some_and(|m| {
                m.is_empty()
                    || m.len() > 128
                    || !m
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "-_.:/".contains(c))
            })
        {
            return Err("The input experiment's target, plan or next capture changed.".into());
        }
        sampled(&self.source, self.key)?;
        if let Some(selection) = &self.selection {
            let expected = selection::Choice::from_frame(&self.source, self.key)?;
            if selection.policy != selection::POLICY
                || selection.next.as_ref() != Some(&expected)
                || selection.attempted_keys.contains(&self.key)
                || selection.unavailable.is_some()
                || !selection.candidates.contains(&expected)
            {
                return Err("Automatic input selection differs from its pinned source".into());
            }
        }
        let current = live
            .frames
            .iter()
            .find(|f| f.id == self.source.id)
            .ok_or("Source observation is no longer available")?;
        if serde_json::to_vec(current).map_err(|e| e.to_string())?
            != serde_json::to_vec(&self.source).map_err(|e| e.to_string())?
        {
            return Err("Source input evidence changed".into());
        }
        let last = live.frames.last().ok_or("Missing source image")?;
        if last.epoch != self.source.epoch
            || last.width != self.source.width
            || last.height != self.source.height
        {
            return Err("Image geometry changed; start a new observation experiment.".into());
        }
        Ok(())
    }
    pub fn check_capture(
        &self,
        index: usize,
        frame: &FrameRef,
        previous: Option<&FrameRef>,
    ) -> Result<()> {
        if index >= STEPS
            || frame.number != self.first_number + index
            || frame.epoch != self.source.epoch
            || frame.width != self.source.width
            || frame.height != self.source.height
        {
            return Err("Response image changed scope or geometry".into());
        }
        let interval = frame
            .capture_interval
            .as_ref()
            .ok_or("Missing response timing")?;
        let w = frame
            .input_context
            .as_ref()
            .ok_or("Input sampling unavailable")?;
        if interval.clock_id != self.investigation
            || w.clock_id != self.investigation
            || w.generation.is_empty()
        {
            return Err("Response clock changed".into());
        }
        crate::action_patterns::capture_input(w, interval).map_err(str::to_owned)?;
        if index > 0 {
            let previous = previous.ok_or("Missing preceding response image")?;
            if previous.number + 1 != frame.number
                || previous.input_context.as_ref().map(|p| &p.generation) != Some(&w.generation)
            {
                return Err("Input sampling restarted or a response image is missing".into());
            }
            let begin = previous
                .capture_interval
                .as_ref()
                .ok_or("Missing previous timing")?
                .end_us;
            let witness =
                InputWitness::check(Some(w), action(index, self.key).as_ref(), interval, begin);
            if !witness.matches {
                return Err(witness.reason);
            }
        }
        Ok(())
    }
}
