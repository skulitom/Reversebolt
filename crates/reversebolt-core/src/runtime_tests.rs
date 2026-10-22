//! Bounded input-response experiments. Descriptive screen effects are not a
//! semantic name, physical quantity, persistent actor identity or edit authority.
use crate::{
    Result,
    changes::Phase as TrialPhase,
    inputs::{Focus, Window},
    runtime::{Candidate, Scope},
    runtime_patch::encode,
    runtime_trials::Trial,
    visual::CaptureInterval,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_TESTS_PER_INVESTIGATION: usize = 8;
pub const MAX_RETAINED_TESTS: usize = 128;
pub const LIFETIME_MS: u128 = 15 * 60 * 1000;
pub const FRAMES_PER_CONDITION: usize = 6;
pub const MAX_GAP_US: u64 = 50_000;
pub const METRIC_ID: &str = "rgb601-triangle640x360-crop24,40,592,296-gaussian1-mad-v1";
pub const LIMITATION: &str = "A comparison of rendered response within these observations. Windows key samples are not proof of delivered game actions. Screen changes do not measure physical speed or identify a gameplay property; unobserved and downstream effects remain unknown.";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub keys: Vec<u8>,
    pub hold_ms: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Baseline,
    BaselineReview,
    ReadyToApply,
    Changed,
    ReadyToRestore,
    Restored,
    Complete,
    Stopped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputWitness {
    pub generation: String,
    pub matches: bool,
    pub held_us: Vec<u64>,
    pub polls: usize,
    pub largest_gap_us: u64,
    pub reason: String,
}

impl InputWitness {
    pub fn check(
        window: Option<&Window>,
        action: Option<&Action>,
        interval: &CaptureInterval,
        begin: u64,
    ) -> Self {
        let keys = action.map_or(&[][..], |a| a.keys.as_slice());
        let end = interval.end_us;
        let mut result = Self {
            generation: String::new(),
            matches: false,
            held_us: vec![0; keys.len()],
            polls: 0,
            largest_gap_us: end.saturating_sub(begin),
            reason: "Input sampling is unavailable.".into(),
        };
        let Some(w) = window else { return result };
        result.generation.clone_from(&w.generation);
        result.polls = w.polls.len();
        result.largest_gap_us = w.largest_unsampled_us;
        if begin > interval.begin_us
            || interval.begin_us > end
            || w.clock_id != interval.clock_id
            || w.generation.is_empty()
            || w.requested_begin_us != begin
            || w.requested_end_us != end
            || w.monitor_started_us > begin
            || w.retained_from_us > begin
            || w.omitted_polls != 0
            || w.polls.len() < 2
            || w.polls.len() > crate::inputs::MAX_WINDOW_POLLS
            || action.is_some_and(|a| !a.valid())
            || w.polls.iter().any(|p| {
                p.focus != Focus::SelectedForeground
                    || p.begin_us < begin
                    || p.end_us > end
                    || p.end_us < p.begin_us
                    || p.end_us - p.begin_us > MAX_GAP_US
                    || p.down.windows(2).any(|k| k[0] >= k[1])
                    || (!p.down.is_empty() && p.down != keys)
            })
            || w.polls.windows(2).any(|p| p[1].begin_us <= p[0].end_us)
        {
            result.reason = "Input or foreground coverage differs from the planned action.".into();
            return result;
        }
        let mut cursor = begin;
        let mut largest_gap = 0;
        for poll in &w.polls {
            largest_gap = largest_gap.max(poll.begin_us - cursor);
            cursor = poll.end_us;
        }
        largest_gap = largest_gap.max(end - cursor);
        result.largest_gap_us = largest_gap;
        if largest_gap > MAX_GAP_US || w.largest_unsampled_us != largest_gap {
            result.reason = "Input sampling has gaps or inconsistent coverage.".into();
            return result;
        }
        // One sampled chord, bracketed by released states; summed repeated taps
        // are not interchangeable with the planned hold.
        let pressed: Vec<_> = w
            .polls
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.down.is_empty())
            .collect();
        if let Some(action) = action {
            let Some((first_index, first)) = pressed.first() else {
                result.reason = "The planned key hold was not observed.".into();
                return result;
            };
            let (last_index, last) = pressed.last().unwrap();
            if *first_index == 0
                || *last_index + 1 == w.polls.len()
                || last_index - first_index + 1 != pressed.len()
                || interval.begin_us.saturating_sub(last.end_us) < 250_000
                || first.begin_us >= interval.begin_us
            {
                result.reason =
                    "The sampled action was repeated, unbracketed or unsettled before capture."
                        .into();
                return result;
            }
            let duration = last.end_us - first.begin_us;
            if duration.abs_diff(u64::from(action.hold_ms) * 1000) > MAX_GAP_US {
                result.reason = "The sampled hold differs from the planned duration.".into();
                return result;
            }
        } else if interval.begin_us - begin < 250_000 {
            result.reason = "The stationary interval is too short.".into();
            return result;
        }
        for pair in w.polls.windows(2) {
            let delta = pair[1].begin_us.saturating_sub(pair[0].end_us);
            for (i, key) in keys.iter().enumerate() {
                if pair[0].down.contains(key) && pair[1].down.contains(key) {
                    result.held_us[i] = result.held_us[i].saturating_add(delta);
                }
            }
        }
        result.matches = result.held_us.iter().all(|us| *us >= 25_000);
        result.reason = if result.matches {
            "Expected sampled key state."
        } else {
            "The planned key hold was not sufficiently observed."
        }
        .into();
        result
    }
}

impl Action {
    fn valid(&self) -> bool {
        !self.keys.is_empty()
            && self.keys.len() <= 3
            && self
                .keys
                .iter()
                .all(|k| *k != 0 && *k != 255 && !(16..=18).contains(k))
            && self.keys.windows(2).all(|w| w[0] < w[1])
            && (100..=1000).contains(&self.hold_ms)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub path: String,
    pub sha256: String,
    pub input_path: String,
    pub input_sha256: String,
    pub width: u32,
    pub height: u32,
    pub interval: CaptureInterval,
    pub started_unix_ms: u128,
    pub completed_unix_ms: u128,
    pub sampling: Option<crate::visual::CaptureSampling>,
    /// Mean absolute grayscale difference from the preceding image of this condition.
    pub difference: Option<f64>,
    pub input: InputWitness,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Condition {
    pub frames: Vec<Frame>,
    /// Fresh engine-owned read, retained after the condition's final image.
    pub runtime_evidence: Option<crate::runtime_trials::Evidence>,
    pub runtime_checked_unix_ms: Option<u128>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearanceSelection {
    pub baseline_sha256: String,
    pub region: crate::visual_regions::Region,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearancePlan {
    pub selection: AppearanceSelection,
    pub comparison_plan_sha256: String,
    /// Typed serialization of the complete baseline condition at selection.
    pub baseline_record_sha256: String,
    pub selected_revision: u64,
    pub selected_unix_ms: u128,
}

impl Condition {
    pub fn median(&self) -> Option<f64> {
        if self.frames.len() != FRAMES_PER_CONDITION {
            return None;
        }
        let mut values = self
            .frames
            .iter()
            .skip(2)
            .map(|f| f.difference)
            .collect::<Option<Vec<_>>>()?;
        values.sort_by(f64::total_cmp);
        Some((values[1] + values[2]) / 2.)
    }
    fn stationary(&self) -> Option<f64> {
        self.frames.get(1)?.difference
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Assessment {
    pub status: String,
    pub reason: String,
    pub medians: [Option<f64>; 3],
    pub limitation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Test {
    pub id: String,
    pub change_id: String,
    pub actor: String,
    pub investigation: String,
    pub catalog_id: String,
    pub scope: Scope,
    pub window: u64,
    pub candidate: Candidate,
    pub value: String,
    /// Hypothesis supplied by the caller, never a verified semantic label.
    pub hypothesis: String,
    /// Evidence for the question, not proof of its gameplay meaning.
    #[serde(default)]
    pub origin: Option<crate::runtime_hypotheses::TestOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<AppearancePlan>,
    pub actions: Vec<Action>,
    pub prepared_revision: u64,
    pub created_unix_ms: u128,
    pub preparation: Option<crate::runtime_trials::Evidence>,
    pub metric: String,
    pub revision: u64,
    pub phase: Phase,
    pub pending_capture: bool,
    pub conditions: [Condition; 3],
    pub assessment: Option<Assessment>,
    pub reason: String,
}

impl Test {
    pub fn validate_plan(&self) -> Result<()> {
        if [
            &self.id,
            &self.change_id,
            &self.investigation,
            &self.catalog_id,
        ]
        .iter()
        .any(|id| !uuid::Uuid::parse_str(id).is_ok_and(|u| u.to_string() == **id))
            || self.metric != METRIC_ID
            || self.id == self.change_id
            || self.actor.is_empty()
            || self.actor.len() > 40
            || self.actor.chars().any(char::is_control)
            || self.window == 0
            || self.hypothesis.trim().is_empty()
            || self.hypothesis.len() > 180
            || self.hypothesis.chars().any(char::is_control)
            || self.actions.len() != 4
            || self.actions.iter().any(|a| !a.valid())
        {
            return Err("Supply one bounded four-action test with distinct identities and a short hypothesis.".into());
        }
        if let Some(plan) = &self.appearance {
            let baseline = &self.conditions[0];
            if baseline.frames.len() != FRAMES_PER_CONDITION
                || plan.selected_revision == 0
                || plan.selected_revision > self.revision
                || plan.selected_unix_ms < self.created_unix_ms
                || baseline
                    .frames
                    .iter()
                    .any(|f| f.completed_unix_ms > plan.selected_unix_ms)
                || plan.baseline_record_sha256
                    != crate::visual::digest(
                        &serde_json::to_vec(baseline).map_err(|e| e.to_string())?,
                    )
                || plan.comparison_plan_sha256 != self.comparison_plan_digest()?
                || matches!(self.phase, Phase::Baseline | Phase::BaselineReview)
            {
                return Err("The retained appearance plan or baseline changed.".into());
            }
            let first = &baseline.frames[0];
            if plan.selection.baseline_sha256 != first.sha256 {
                return Err("The selected baseline image changed.".into());
            }
            plan.selection
                .region
                .validate([first.width, first.height])?;
        }
        self.candidate.locator.validate(&self.scope)?;
        if self.candidate.id != self.candidate.locator.id() {
            return Err("Test candidate identity changed.".into());
        }
        if let Some(origin) = &self.origin {
            let selected = origin.selected()?;
            if origin
                .play_context
                .as_ref()
                .is_some_and(|c| c.source.investigation_id != self.investigation)
            {
                return Err("The activity context belongs to a different investigation.".into());
            }
            if selected.path_id != self.candidate.id
                || self.hypothesis != origin.hypothesis.proposal.question
            {
                return Err("The test does not match its retained hypothesis choice.".into());
            }
        }
        if self.candidate.name.len() > 4096
            || self.candidate.description.len() > 4096
            || crate::runtime::scalar_value(self.candidate.type_code, &self.candidate.bytes)?
                != self.candidate.value
        {
            return Err("The candidate's value and byte representation disagree.".into());
        }
        if encode(self.candidate.type_code, &self.value)? == self.candidate.bytes {
            return Err("The planned edit must differ from the observed value.".into());
        }
        if let Some(e) = &self.preparation {
            if !e.valid()
                || e.report["scope"] != serde_json::json!(self.scope)
                || e.report["operation"] != "refresh_runtime_paths"
                || e.report["checks"].as_array().is_none_or(|c| c.len() != 1)
            {
                return Err("Experiment preparation evidence is inconsistent.".into());
            }
            let patch = crate::runtime_patch::Patch::prepare(
                self.candidate.locator.clone(),
                &e.report["checks"][0],
                &self.value,
                self.created_unix_ms,
            )?;
            if patch.type_code != self.candidate.type_code || patch.expected != self.candidate.bytes
            {
                return Err("Preparation contradicts the planned candidate.".into());
            }
        }
        Ok(())
    }
    /// Records a one-time choice before the linked edit. The caller must also
    /// check its edit store before accepting a new choice. Identical replay is read-only.
    pub fn lock_appearance(
        &mut self,
        selection: AppearanceSelection,
        revision: u64,
        now: u128,
    ) -> Result<bool> {
        self.validate_plan()?;
        if let Some(plan) = &self.appearance {
            if plan.selection == selection
                && Some(plan.selected_revision) == revision.checked_add(1)
            {
                return Ok(false);
            }
            return Err("The saved appearance choice is immutable. Read the current test.".into());
        }
        self.check_cursor(revision)?;
        if self.phase != Phase::ReadyToApply
            || self.conditions[0].frames.len() != FRAMES_PER_CONDITION
            || self.conditions[1..]
                .iter()
                .any(|c| !c.frames.is_empty() || c.runtime_evidence.is_some())
            || self.assessment.is_some()
        {
            return Err(
                "Choose the appearance after the complete baseline and before the linked edit."
                    .into(),
            );
        }
        let mut next = self.clone();
        next.revision += 1;
        next.appearance = Some(AppearancePlan {
            selection,
            comparison_plan_sha256: self.comparison_plan_digest()?,
            baseline_record_sha256: crate::visual::digest(
                &serde_json::to_vec(&self.conditions[0]).map_err(|e| e.to_string())?,
            ),
            selected_revision: next.revision,
            selected_unix_ms: now,
        });
        next.reason = "Appearance choice saved. The planned edit can now be applied.".into();
        next.validate_plan()?;
        *self = next;
        Ok(true)
    }

    /// Resolve the engine's image review, never a caller assertion of motion.
    /// The engine must verify the complete saved baseline and the selected region
    /// before supplying Some, and refuse any existing linked edit or unresolved write.
    /// Acceptance locks this choice atomically before the test can permit an edit.
    pub fn finish_baseline_review(
        &mut self,
        selection: Option<AppearanceSelection>,
        revision: u64,
        now: u128,
    ) -> Result<()> {
        self.validate_plan()?;
        self.check_cursor(revision)?;
        if self.phase != Phase::BaselineReview
            || self.conditions[0].frames.len() != FRAMES_PER_CONDITION
            || !self.conditions[0]
                .frames
                .iter()
                .skip(1)
                .all(|f| f.input.matches)
            || self.appearance.is_some()
            || self.conditions[1..]
                .iter()
                .any(|c| !c.frames.is_empty() || c.runtime_evidence.is_some())
            || self.assessment.is_some()
        {
            return Err(
                "Only a complete, unedited baseline can receive its regional review.".into(),
            );
        }
        if let Some(selection) = selection {
            let mut next = self.clone();
            next.phase = Phase::ReadyToApply;
            next.lock_appearance(selection, revision, now)?;
            next.reason = "A local response was measured and its comparison region saved. The planned edit can now be applied.".into();
            *self = next;
        } else {
            self.stop("Neither the whole-screen nor regional baseline supports this comparison. No planned edit was made.");
        }
        Ok(())
    }

    fn comparison_plan_digest(&self) -> Result<String> {
        let mut plan = self.clone();
        plan.appearance = None;
        plan.revision = 0;
        plan.phase = Phase::Baseline;
        plan.pending_capture = false;
        plan.conditions = Default::default();
        plan.assessment = None;
        plan.reason.clear();
        Ok(crate::visual::digest(
            &serde_json::to_vec(&plan).map_err(|e| e.to_string())?,
        ))
    }

    pub fn active(&self) -> bool {
        !matches!(self.phase, Phase::Complete | Phase::Stopped)
    }
    pub fn stop(&mut self, reason: &str) {
        if self.active() {
            self.phase = Phase::Stopped;
            self.reason = reason.chars().take(512).collect();
            self.revision = self.revision.saturating_add(1);
        }
    }
    pub fn recover(&mut self) {
        self.stop("Reversebolt restarted. This experiment will not resume. Check the linked edit history for recovery.");
    }
    pub fn condition_index(&self) -> Result<usize> {
        match self.phase {
            Phase::Baseline => Ok(0),
            Phase::Changed => Ok(1),
            Phase::Restored => Ok(2),
            _ => Err("This experiment is not collecting images now.".into()),
        }
    }
    pub fn check_cursor(&self, revision: u64) -> Result<()> {
        if self.revision != revision
            || self.revision == u64::MAX
            || self.pending_capture
            || !self.active()
        {
            return Err(
                "Experiment changed or a capture is unsettled. Inspect its current state.".into(),
            );
        }
        Ok(())
    }
    pub fn begin_capture(&mut self, revision: u64) -> Result<(usize, usize)> {
        self.validate_plan()?;
        self.check_cursor(revision)?;
        let condition = self.condition_index()?;
        let frame = self.conditions[condition].frames.len();
        if frame >= FRAMES_PER_CONDITION {
            return Err("Condition is already complete.".into());
        }
        self.pending_capture = true;
        Ok((condition, frame))
    }
    /// The engine must supply its retained sampling window, not caller assertions
    /// that an action occurred. This recomputes the summary and binds its digest.
    pub fn finish_capture(&mut self, mut frame: Frame, input: Option<&Window>) -> Result<()> {
        let index = self.condition_index()?;
        let condition = &self.conditions[index];
        let begin = condition
            .frames
            .last()
            .map_or(frame.interval.begin_us, |f| f.interval.end_us);
        let action = condition
            .frames
            .len()
            .checked_sub(2)
            .and_then(|i| self.actions.get(i));
        frame.input = InputWitness::check(input, action, &frame.interval, begin);
        let input_bytes = serde_json::to_vec(&input).map_err(|e| e.to_string())?;
        if crate::visual::digest(&input_bytes) != frame.input_sha256 {
            return Err("The input-window digest does not match the retained evidence.".into());
        }
        if !self.pending_capture
            || frame.interval.clock_id != self.investigation
            || frame.interval.begin_us > frame.interval.end_us
            || frame.width == 0
            || frame.height == 0
            || u64::from(frame.width) * u64::from(frame.height) > 4_000_000
            || frame.started_unix_ms > frame.completed_unix_ms
            || condition.frames.len() >= FRAMES_PER_CONDITION
            || frame
                .difference
                .is_some_and(|v| !v.is_finite() || !(0.0..=255.0).contains(&v))
            || (condition.frames.is_empty() != frame.difference.is_none())
            || self
                .conditions
                .iter()
                .flat_map(|c| &c.frames)
                .last()
                .is_some_and(|previous| {
                    frame.interval.begin_us <= previous.interval.end_us
                        || frame.started_unix_ms < previous.completed_unix_ms
                })
            || ![&frame.sha256, &frame.input_sha256]
                .iter()
                .all(|h| h.len() == 64 && h.bytes().all(|c| c.is_ascii_hexdigit()))
            || [&frame.path, &frame.input_path]
                .iter()
                .any(|p| p.is_empty() || p.len() > 4096)
            || self.conditions[0].frames.first().is_some_and(|first| {
                [first.width, first.height] != [frame.width, frame.height]
                    || first.input.generation != frame.input.generation
            })
        {
            return Err(
                "Experiment image has incompatible geometry, chronology or measurement.".into(),
            );
        }
        self.conditions[index].frames.push(frame);
        self.pending_capture = false;
        self.revision = self.revision.saturating_add(1);
        if self.conditions[index].frames.len() == FRAMES_PER_CONDITION {
            match index {
                0 => {
                    let c = &self.conditions[0];
                    if !c.frames.iter().skip(1).all(|f| f.input.matches) {
                        self.stop(
                            "Baseline input coverage is insufficient. No planned edit was made.",
                        );
                    } else if c
                        .median()
                        .is_none_or(|m| m < 5. || m < 5. * c.stationary().unwrap_or(f64::INFINITY))
                    {
                        self.phase = Phase::BaselineReview;
                        self.reason =
                            "Checking the saved baseline for a local response before any edit."
                                .into();
                    } else {
                        self.phase = Phase::ReadyToApply;
                        self.reason =
                            "Baseline collected. The planned edit can now be applied.".into();
                    }
                }
                1 => {
                    self.phase = Phase::ReadyToRestore;
                    self.reason = "Changed response collected. Restore the linked edit.".into();
                }
                // Completion requires the fresh value read and durable restored trial.
                _ => {
                    self.reason =
                        "Images collected. Check the original value and finish the comparison."
                            .into();
                }
            }
        }
        Ok(())
    }
    /// Synchronize only from the shared engine's durable edit record.
    pub fn sync_trial(&mut self, trial: &Trial) -> Result<()> {
        trial.validate()?;
        self.validate_plan()?;
        if let Some(e) = &self.preparation
            && crate::runtime_patch::Patch::prepare(
                self.candidate.locator.clone(),
                &e.report["checks"][0],
                &self.value,
                trial.patch.prepared_unix_ms,
            )?
            .id()
                != trial.patch.id()
        {
            return Err("The reference or value differs from the observed baseline.".into());
        }
        if trial.id != self.change_id
            || trial.candidate_id != self.candidate.id
            || trial.context.investigation != self.investigation
            || trial.context.catalog_id != self.catalog_id
            || trial.context.process != self.scope
            || trial.patch.type_code != self.candidate.type_code
            || trial.patch.expected != self.candidate.bytes
            || trial.patch.path != self.candidate.locator
            || encode(trial.patch.type_code, &self.value)? != trial.patch.replacement
        {
            return Err("Trial does not belong to this experiment plan.".into());
        }
        if !self.active() {
            return Ok(());
        }
        match (&self.phase, &trial.phase) {
            (Phase::ReadyToApply, TrialPhase::Written) => {
                if self.conditions[0].frames.len() != FRAMES_PER_CONDITION
                    || self.conditions[0].frames.last().unwrap().completed_unix_ms
                        > trial.patch.prepared_unix_ms
                {
                    return Err("Baseline was not collected before the edit review.".into());
                }
                self.phase = Phase::Changed;
                self.revision = self.revision.saturating_add(1);
                self.reason = "Collecting the response after the planned edit.".into();
            }
            (Phase::ReadyToRestore, TrialPhase::Restored) => {
                self.phase = Phase::Restored;
                self.revision = self.revision.saturating_add(1);
                self.reason = "Collecting the response after recovery.".into();
            }
            (Phase::Changed, TrialPhase::Restored) => {
                self.stop("The value was restored early. No complete comparison is available.")
            }
            (_, TrialPhase::Refused | TrialPhase::Uncertain | TrialPhase::Expired) => self
                .stop("The edit did not settle as planned. Check its history before continuing."),
            (
                Phase::Baseline | Phase::BaselineReview,
                TrialPhase::Written | TrialPhase::Restored,
            )
            | (Phase::ReadyToApply, TrialPhase::Restored) => {
                self.stop("The edit did not follow the test's baseline and recovery order.")
            }
            _ => {}
        }
        Ok(())
    }
    pub fn complete(&mut self, trial: &Trial, revision: u64) -> Result<()> {
        self.check_cursor(revision)?;
        if self.phase != Phase::Restored
            || trial.phase != TrialPhase::Restored
            || self
                .conditions
                .iter()
                .any(|c| c.frames.len() != FRAMES_PER_CONDITION)
        {
            return Err("Complete all three conditions and restore the linked edit first.".into());
        }
        self.sync_trial(trial)?;
        self.assessment = Some(self.assess(trial));
        self.phase = Phase::Complete;
        self.reason = self.assessment.as_ref().unwrap().reason.clone();
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }
    fn assess(&self, trial: &Trial) -> Assessment {
        let medians = self.conditions.each_ref().map(Condition::median);
        let mut a = Assessment {
            status: "inconclusive".into(),
            reason: "Input or value evidence is incomplete.".into(),
            medians,
            limitation: LIMITATION.into(),
        };
        if self.conditions.iter().any(|c| {
            c.frames.len() != FRAMES_PER_CONDITION
                || !c.frames.iter().skip(1).all(|f| f.input.matches)
        }) || !self.value_observed(trial, 1)
            || !self.value_observed(trial, 2)
        {
            return a;
        }
        if trial
            .write
            .as_ref()
            .and_then(|a| a.completed_ms)
            .is_none_or(|t| self.conditions[1].frames[0].started_unix_ms < t)
            || trial
                .restoration
                .as_ref()
                .and_then(|a| a.completed_ms)
                .is_none_or(|t| self.conditions[2].frames[0].started_unix_ms < t)
            || trial.restoration.as_ref().is_none_or(|r| {
                self.conditions[1]
                    .runtime_checked_unix_ms
                    .is_none_or(|t| t > r.started_ms)
            })
        {
            a.reason = "The measurements do not bracket the recorded edit and recovery.".into();
            return a;
        }
        // Reject materially different sampled holds, even when the same keys appeared.
        for pulse in 2..FRAMES_PER_CONDITION {
            for key in 0..self.actions[pulse - 2].keys.len() {
                let holds = self
                    .conditions
                    .each_ref()
                    .map(|c| c.frames[pulse].input.held_us.get(key).copied().unwrap_or(0));
                let min = *holds.iter().min().unwrap();
                let max = *holds.iter().max().unwrap();
                if max - min > 50_000.max(max / 4) {
                    a.reason = "Sampled input durations differ between conditions.".into();
                    return a;
                }
            }
        }
        let [Some(b), Some(c), Some(r)] = medians else {
            return a;
        };
        if [0, 2].iter().any(|i| {
            medians[*i].unwrap() < 5.
                || medians[*i].unwrap()
                    < 5. * self.conditions[*i].stationary().unwrap_or(f64::INFINITY)
        }) || r < 0.5 * b
        {
            a.reason = "The baseline or recovered response is too weak for this comparison.".into();
            return a;
        }
        if c <= 2f64.max(0.2 * b) {
            a.status = "suppression_observed".into();
            a.reason="Less rendered response was observed after the edit, and response returned after recovery.".into();
        } else {
            a.status = "suppression_not_observed".into();
            a.reason =
                "The planned edit did not suppress the rendered response in this comparison."
                    .into();
        }
        a
    }
    fn value_observed(&self, trial: &Trial, index: usize) -> bool {
        let c = &self.conditions[index];
        let Some(e) = &c.runtime_evidence else {
            return false;
        };
        let Some(checked) = c.runtime_checked_unix_ms else {
            return false;
        };
        if c.frames
            .last()
            .is_none_or(|f| checked < f.completed_unix_ms)
            || !e.valid()
            || e.report["scope"] != serde_json::json!(self.scope)
            || e.report["operation"] != "refresh_runtime_paths"
            || e.report["checks"].as_array().is_none_or(|c| c.len() != 1)
        {
            return false;
        }
        let mut expected = trial.patch.clone();
        if index == 1 {
            std::mem::swap(&mut expected.expected, &mut expected.replacement);
        }
        expected.prepared_unix_ms = checked;
        let Ok(value) = crate::runtime_patch::value_text(expected.type_code, &expected.replacement)
        else {
            return false;
        };
        crate::runtime_patch::Patch::prepare(
            expected.path.clone(),
            &e.report["checks"][0],
            &value,
            checked,
        )
        .is_ok_and(|actual| actual.id() == expected.id())
    }
    pub fn summary(&self) -> Value {
        let mut summary = serde_json::json!({"id":self.id,"change_id":self.change_id,"hypothesis":self.hypothesis,
            "hypothesis_ref":self.origin.as_ref().map(|o|&o.reference),
            "candidate_id":self.candidate.id,"phase":self.phase,"revision":self.revision,"pending_capture":self.pending_capture,
            "frames":self.conditions.each_ref().map(|c|c.frames.len()),"reason":self.reason,"assessment":self.assessment,
            "actor":self.actor,"investigation_id":self.investigation,"catalog_id":self.catalog_id,
            "actions":self.actions,"created_unix_ms":self.created_unix_ms,"active":self.active()});
        if let Some(plan) = &self.appearance {
            summary["appearance"] = serde_json::json!(plan);
        }
        summary
    }
}
