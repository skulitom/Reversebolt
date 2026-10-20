use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub session: String,
    /// Opaque, verified identity including process creation identity, not PID alone.
    pub target: String,
    pub generation: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Human,
    Agent,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub client: String,
    pub role: Role,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Observe,
    Effect,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub kind: Kind,
    pub payload: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Queued,
    Running,
    Pausing,
    Paused,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
    Interrupted,
}
impl Phase {
    pub fn terminal(self) -> bool {
        matches!(
            self,
            Self::Cancelled | Self::Completed | Self::Failed | Self::Interrupted
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Knowledge {
    NotApplicable,
    NotApplied,
    Applied,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub success: bool,
    pub effect: Knowledge,
    pub data: Vec<u8>,
    pub note: String,
}
impl Outcome {
    pub fn unknown(note: &str) -> Self {
        Self {
            success: false,
            effect: Knowledge::Unknown,
            data: vec![],
            note: note.into(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub id: u64,
    pub job: u64,
    pub step: usize,
    pub binding: Binding,
    pub instruction: Step,
    pub result_limit: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub operation: u64,
    pub step: usize,
    pub outcome: Outcome,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    pub parent: Option<u64>,
    pub creator: Actor,
    pub controller: Actor,
    pub binding: Binding,
    pub steps: Vec<Step>,
    pub phase: Phase,
    pub next_step: usize,
    pub checkpoints: Vec<Checkpoint>,
    pub pending: Option<Operation>,
    pub consent_boot: Option<u64>,
    pub invalidated: bool,
    pub evidence_valid: bool,
}
impl Job {
    pub fn has_effects(&self) -> bool {
        self.steps[self.next_step..]
            .iter()
            .any(|s| s.kind == Kind::Effect)
    }
    pub fn result_bytes(&self) -> usize {
        self.checkpoints.iter().map(|c| c.outcome.data.len()).sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Submit {
        steps: Vec<Step>,
        consent: bool,
    },
    Pause {
        job: u64,
    },
    Resume {
        job: u64,
        renew_consent: bool,
    },
    Cancel {
        job: u64,
    },
    Authorize {
        job: u64,
    },
    /// Creates a new job. Reuses the plan, never old results or target evidence.
    Continue {
        job: u64,
        consent: bool,
    },
    TakeControl,
    ReleaseControl,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    /// Allocated monotonically per authenticated client, starting at one.
    pub request: u64,
    pub actor: Actor,
    pub boot: u64,
    pub binding: Binding,
    pub action: Action,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ack {
    pub cursor: u64,
    pub job: Option<u64>,
    pub phase: Option<Phase>,
    pub in_flight: Option<u64>,
    pub rejection: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reply {
    pub duplicate: bool,
    pub ack: Ack,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub command: Command,
    pub ack: Ack,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Client {
    pub role: Role,
    pub high_water: u64,
    pub receipts: VecDeque<Receipt>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub cursor: u64,
    pub boot: u64,
    pub job: Option<u64>,
    pub actor: Option<Actor>,
    pub request: Option<u64>,
    pub kind: String,
    pub operation: Option<u64>,
    pub note: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    pub queued: usize,
    pub jobs: usize,
    pub clients: usize,
    pub receipts_per_client: usize,
    pub events: usize,
    pub steps: usize,
    pub payload_bytes: usize,
    pub result_per_step: usize,
    pub result_per_job: usize,
    pub polls: u32,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            queued: 8,
            jobs: 24,
            clients: 8,
            receipts_per_client: 8,
            events: 128,
            steps: 16,
            payload_bytes: 256,
            result_per_step: 1024,
            result_per_job: 4096,
            polls: 16,
        }
    }
}
impl Limits {
    pub fn validate(&self) -> Result<(), String> {
        let d = Self::default();
        let pairs = [
            (self.queued, d.queued),
            (self.jobs, d.jobs),
            (self.clients, d.clients),
            (self.receipts_per_client, d.receipts_per_client),
            (self.events, d.events),
            (self.steps, d.steps),
            (self.payload_bytes, d.payload_bytes),
            (self.result_per_step, d.result_per_step),
            (self.result_per_job, d.result_per_job),
            (self.polls as usize, d.polls as usize),
        ];
        if pairs.iter().any(|(n, max)| *n == 0 || n > max) {
            return Err("limits outside supported hard ceilings".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub version: u32,
    pub revision: u64,
    pub boot: u64,
    pub session: String,
    pub generation: u64,
    pub current: Option<Binding>,
    pub human_hold: Option<Actor>,
    pub limits: Limits,
    pub next_job: u64,
    pub next_operation: u64,
    pub event_head: u64,
    pub jobs: BTreeMap<u64, Job>,
    pub clients: BTreeMap<String, Client>,
    pub events: VecDeque<Event>,
    /// Durable barrier. Unknown effects require a trusted quiescence witness.
    pub unresolved: BTreeSet<u64>,
}
impl Snapshot {
    pub fn empty(session: &str, limits: Limits) -> Self {
        Self {
            version: 1,
            revision: 0,
            boot: 0,
            session: session.into(),
            generation: 0,
            current: None,
            human_hold: None,
            limits,
            next_job: 1,
            next_operation: 1,
            event_head: 0,
            jobs: BTreeMap::new(),
            clients: BTreeMap::new(),
            events: VecDeque::new(),
            unresolved: BTreeSet::new(),
        }
    }
    pub fn event(
        &mut self,
        job: Option<u64>,
        actor: Option<Actor>,
        request: Option<u64>,
        kind: &str,
        operation: Option<u64>,
        note: &str,
    ) {
        self.event_head += 1;
        self.events.push_back(Event {
            cursor: self.event_head,
            boot: self.boot,
            job,
            actor,
            request,
            kind: kind.into(),
            operation,
            note: bounded(note, 256),
        });
        while self.events.len() > self.limits.events {
            self.events.pop_front();
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        self.limits.validate()?;
        if self.version != 1
            || self.session.is_empty()
            || self.session.len() > 64
            || self.event_head >= u64::MAX - 1000
            || self.revision >= u64::MAX - 1000
            || self.boot >= u64::MAX - 1000
            || self.generation >= u64::MAX - 1000
            || self.next_job >= u64::MAX - 1000
            || self.next_operation >= u64::MAX - 1000
            || self.jobs.len() > self.limits.jobs
            || self.clients.len() > self.limits.clients
            || self.events.len() > self.limits.events
            || self.unresolved.len() > self.limits.jobs
        {
            return Err("invalid checkpoint bounds/schema".into());
        }
        if self.current.as_ref().is_some_and(|b| {
            b.session != self.session
                || b.generation != self.generation
                || b.target.is_empty()
                || b.target.len() > 128
        }) {
            return Err("invalid current binding".into());
        }
        let mut pending = 0;
        let mut running = 0;
        let mut operations = BTreeSet::new();
        let valid_actor = |a: &Actor| !a.client.is_empty() && a.client.len() <= 64;
        if self.next_job == 0
            || self.next_operation == 0
            || self
                .human_hold
                .as_ref()
                .is_some_and(|a| a.role != Role::Human || !valid_actor(a))
        {
            return Err("invalid identity counters/hold".into());
        }
        for (&id, j) in &self.jobs {
            if id == 0
                || id != j.id
                || id >= self.next_job
                || j.parent.is_some_and(|parent| parent == 0 || parent >= id)
                || !valid_actor(&j.creator)
                || !valid_actor(&j.controller)
                || j.steps.is_empty()
                || j.steps.len() > self.limits.steps
                || j.next_step > j.steps.len()
                || j.result_bytes() > self.limits.result_per_job
                || j.checkpoints.len() > self.limits.steps
                || j.steps
                    .iter()
                    .any(|s| s.payload.len() > self.limits.payload_bytes)
                || j.checkpoints.iter().any(|c| {
                    c.step >= j.steps.len()
                        || c.operation >= self.next_operation
                        || c.outcome.data.len() > self.limits.result_per_step
                        || c.outcome.note.len() > 256
                })
                || j.binding.session != self.session
                || j.binding.generation == 0
                || j.binding.generation > self.generation
                || j.binding.target.is_empty()
                || j.binding.target.len() > 128
                || j.consent_boot.is_some_and(|boot| boot != self.boot)
                || (j.phase == Phase::Completed && j.next_step != j.steps.len())
                || (j.evidence_valid
                    && (j.invalidated || self.current.as_ref() != Some(&j.binding)))
                || (!j.phase.terminal()
                    && !j.invalidated
                    && self.current.as_ref() != Some(&j.binding))
                || (!j.phase.terminal() && j.invalidated && j.phase != Phase::Cancelling)
            {
                return Err("invalid job checkpoint".into());
            }
            for (index, cp) in j.checkpoints.iter().enumerate() {
                if cp.step != index
                    || cp.operation == 0
                    || !operations.insert(cp.operation)
                    || (index > 0 && cp.operation <= j.checkpoints[index - 1].operation)
                    || (cp.outcome.effect == Knowledge::Unknown && j.phase != Phase::Interrupted)
                    || (cp.outcome.effect != Knowledge::Unknown
                        && ((j.steps[cp.step].kind == Kind::Observe
                            && cp.outcome.effect != Knowledge::NotApplicable)
                            || (j.steps[cp.step].kind == Kind::Effect
                                && cp.outcome.effect == Knowledge::NotApplicable)))
                    || (cp.outcome.success
                        && (cp.outcome.effect == Knowledge::Unknown
                            || (j.steps[cp.step].kind == Kind::Effect
                                && cp.outcome.effect != Knowledge::Applied)))
                {
                    return Err("invalid checkpoint outcome/order".into());
                }
            }
            // Reconciliation may replace the final Unknown receipt, but never
            // turns Interrupted history into an executable checkpoint.
            if j.next_step != j.checkpoints.len()
                && !(j.phase == Phase::Interrupted && j.next_step + 1 == j.checkpoints.len())
            {
                return Err("checkpoint progress mismatch".into());
            }
            if matches!(j.phase, Phase::Running | Phase::Pausing | Phase::Cancelling) {
                running += 1;
            }
            if let Some(p) = &j.pending {
                pending += 1;
                if p.id == 0
                    || !operations.insert(p.id)
                    || p.id >= self.next_operation
                    || p.job != id
                    || p.step != j.next_step
                    || p.binding != j.binding
                    || j.steps.get(p.step) != Some(&p.instruction)
                    || j.phase.terminal()
                    || j.phase == Phase::Paused
                    || j.phase == Phase::Queued
                    || p.result_limit > self.limits.result_per_step
                    || p.result_limit > self.limits.result_per_job - j.result_bytes()
                {
                    return Err("invalid pending intent".into());
                }
            }
        }
        for operation in &self.unresolved {
            if !self.jobs.values().any(|j| {
                j.phase == Phase::Interrupted
                    && j.checkpoints.iter().any(|cp| {
                        cp.operation == *operation
                            && cp.outcome.effect == Knowledge::Unknown
                            && j.steps[cp.step].kind == Kind::Effect
                    })
            }) {
                return Err("unresolved barrier lacks an unknown effect receipt".into());
            }
        }
        if pending > 1
            || running > 1
            || self
                .jobs
                .values()
                .filter(|j| j.phase == Phase::Queued)
                .count()
                > self.limits.queued
        {
            return Err("multiple executors or queue overflow".into());
        }
        let first = self
            .events
            .front()
            .map_or(self.event_head + 1, |e| e.cursor);
        if self.events.iter().enumerate().any(|(i, e)| {
            e.cursor == 0
                || e.cursor != first + i as u64
                || e.cursor > self.event_head
                || e.boot > self.boot
                || e.actor.as_ref().is_some_and(|a| !valid_actor(a))
                || e.note.len() > 256
                || e.kind.len() > 64
        }) || self
            .events
            .back()
            .is_some_and(|e| e.cursor != self.event_head)
        {
            return Err("invalid event cursor sequence".into());
        }
        for (name, c) in &self.clients {
            if name.is_empty()
                || name.len() > 64
                || c.high_water == 0
                || c.high_water < c.receipts.len() as u64
                || c.high_water >= u64::MAX - 1000
                || c.receipts.is_empty()
                || c.receipts.len() > self.limits.receipts_per_client
                || c.receipts
                    .back()
                    .is_some_and(|r| r.command.request != c.high_water)
                || c.receipts.iter().enumerate().any(|(i, r)| {
                    r.command.actor.client != *name
                        || r.command.actor.role != c.role
                        || r.command.request == 0
                        || r.command.request != c.high_water - (c.receipts.len() - 1 - i) as u64
                        || serde_json::to_vec(&r.command).map_or(true, |v| v.len() > 8192)
                        || r.ack.cursor == 0
                        || r.ack.cursor > self.event_head
                        || r.ack.rejection.as_ref().is_some_and(|s| s.len() > 256)
                })
            {
                return Err("invalid request ledger".into());
            }
        }
        Ok(())
    }
}
pub(crate) fn bounded(value: &str, bytes: usize) -> String {
    let mut end = value.len().min(bytes);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].into()
}
