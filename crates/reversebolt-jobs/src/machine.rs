use crate::{journal::Journal, model::*};
use serde::Serialize;
use std::collections::VecDeque;

pub enum Poll {
    Pending,
    Ready(Outcome),
}
/// Calls must be bounded/nonblocking. Ready is also a quiescence acknowledgement:
/// the executor will make no later effect for that operation. Never retarget a
/// pinned operation when the worker attaches a different target.
pub trait Executor {
    fn begin(&mut self, operation: &Operation) -> Poll;
    fn poll(&mut self, operation: u64, cancel_requested: bool) -> Poll;
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Progress {
    Idle,
    Transition,
    Intent(u64),
    Started(u64),
    Waiting(u64),
    Settled(u64),
    NeedsDrain(u64),
    Fenced,
}
#[derive(Clone, Debug, Serialize)]
pub struct Flight {
    pub operation: Operation,
    pub polls: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct View {
    pub history: Snapshot,
    pub in_flight: Option<Flight>,
    pub fence: Option<String>,
    pub volatile_outcome: Option<(u64, Outcome)>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum CursorError {
    Expired { oldest: u64, head: u64 },
    Future,
}
#[derive(Serialize)]
pub struct EventPage {
    pub events: Vec<Event>,
    pub head: u64,
    pub next: u64,
}

pub struct Jobs<J: Journal> {
    journal: J,
    state: Snapshot,
    flight: Option<Flight>,
    fence: Option<String>,
    volatile_outcome: Option<(u64, Outcome)>,
    needs_drain: bool,
}

impl<J: Journal> Jobs<J> {
    pub fn open(journal: J, session: &str, limits: Limits) -> Result<Self, String> {
        let mut state = journal
            .load()
            .unwrap_or_else(|| Snapshot::empty(session, limits.clone()));
        state.validate()?;
        limits.validate()?;
        let mut expanded = state.limits.clone();
        expanded.steps = limits.steps;
        if state.session != session || expanded != limits || limits.steps < state.limits.steps {
            return Err("session/limits mismatch; do not overwrite existing history".into());
        }
        // A larger step ceiling adds capacity without rewriting old plans or
        // resuming them. All other limits must still match the saved journal.
        if state.limits.steps != limits.steps {
            state.limits = limits;
            state.event(
                None,
                None,
                None,
                "StepLimitExpanded",
                None,
                "expanded job capacity; historical plans and restart rules preserved",
            );
        }
        state.boot += 1;
        state.current = None;
        state.human_hold = None;
        let mut recovered = vec![];
        for job in state.jobs.values_mut() {
            job.evidence_valid = false;
            job.consent_boot = None;
            if !job.phase.terminal() {
                if let Some(op) = job.pending.take() {
                    job.checkpoints.push(Checkpoint {
                        operation: op.id,
                        step: op.step,
                        outcome: Outcome::unknown(
                            "restart after durable intent; execution/outcome unknown",
                        ),
                    });
                    if op.instruction.kind == Kind::Effect {
                        state.unresolved.insert(op.id);
                    }
                    recovered.push((job.id, Some(op.id)));
                } else {
                    recovered.push((job.id, None));
                }
                job.phase = Phase::Interrupted;
                job.invalidated = true;
            }
        }
        for (id, op) in recovered {
            state.event(
                Some(id),
                None,
                None,
                "Interrupted",
                op,
                "historical checkpoint; no work resumed",
            );
        }
        state.event(
            None,
            None,
            None,
            "Boot",
            None,
            "connection and consent revoked; explicit fresh scope required",
        );
        let mut runner = Self {
            journal,
            state: state.clone(),
            flight: None,
            fence: None,
            volatile_outcome: None,
            needs_drain: false,
        };
        runner.persist(state, 0)?;
        Ok(runner)
    }
    pub fn view(&self) -> View {
        View {
            history: self.state.clone(),
            in_flight: self.flight.clone(),
            fence: self.fence.clone(),
            volatile_outcome: self.volatile_outcome.clone(),
        }
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.state
    }
    pub fn into_journal(self) -> J {
        self.journal
    }
    fn persist(&mut self, mut next: Snapshot, reserve: usize) -> Result<(), String> {
        next.revision = self.state.revision + 1;
        if let Err(error) = next.validate() {
            self.fence = Some(format!(
                "checkpoint invariant failure: {error}; dispatch fenced"
            ));
            return Err(self.fence.clone().unwrap());
        }
        if let Err(error) = self.journal.append(&next, reserve) {
            self.fence = Some(format!(
                "storage outcome uncertain: {error}; reopen/reconcile before new dispatch"
            ));
            return Err(self.fence.clone().unwrap());
        }
        self.state = next;
        Ok(())
    }
    fn healthy(&self) -> Result<(), String> {
        if let Some(f) = &self.fence {
            Err(f.clone())
        } else {
            Ok(())
        }
    }
    /// Worker-only identity operation. Every attach/reconnect/exit invalidates
    /// generation and evidence. The worker authenticates process creation identity.
    pub fn bind(&mut self, target: Option<&str>) -> Result<Option<Binding>, String> {
        self.healthy()?;
        if target.is_some_and(|t| t.is_empty() || t.len() > 128) {
            return Err("invalid target identity".into());
        }
        let mut next = self.state.clone();
        next.generation += 1;
        next.current = target.map(|t| Binding {
            session: next.session.clone(),
            target: t.into(),
            generation: next.generation,
        });
        next.human_hold = None;
        let flight = self.flight.as_ref().map(|f| f.operation.id);
        let mut events = vec![];
        for j in next.jobs.values_mut() {
            j.evidence_valid = false;
            j.consent_boot = None;
            if !j.phase.terminal() {
                j.invalidated = true;
                if j.pending.as_ref().is_some_and(|p| Some(p.id) == flight) {
                    j.phase = Phase::Cancelling;
                } else {
                    let op = j.pending.take().map(|p| p.id);
                    j.phase = Phase::Interrupted;
                    events.push((j.id, op));
                }
            }
        }
        for (id, op) in events {
            next.event(
                Some(id),
                None,
                None,
                "Invalidated",
                op,
                "no dispatch; any prepared operation was not started",
            );
        }
        next.event(
            None,
            None,
            None,
            "BindingChanged",
            flight,
            "prior evidence revoked; any old in-flight operation stays pinned",
        );
        let binding = next.current.clone();
        self.persist(next, 2)?;
        Ok(binding)
    }
    pub fn events_after(&self, cursor: u64, limit: usize) -> Result<EventPage, CursorError> {
        let oldest = self
            .state
            .events
            .front()
            .map_or(self.state.event_head + 1, |e| e.cursor);
        if cursor > self.state.event_head {
            return Err(CursorError::Future);
        }
        if cursor.saturating_add(1) < oldest {
            return Err(CursorError::Expired {
                oldest,
                head: self.state.event_head,
            });
        }
        let events: Vec<_> = self
            .state
            .events
            .iter()
            .filter(|e| e.cursor > cursor)
            .take(limit.min(self.state.limits.events))
            .cloned()
            .collect();
        let next = events.last().map_or(cursor, |e| e.cursor);
        Ok(EventPage {
            events,
            head: self.state.event_head,
            next,
        })
    }
    pub fn command(&mut self, command: Command) -> Result<Reply, String> {
        self.healthy()?;
        if command.actor.client.is_empty()
            || command.actor.client.len() > 64
            || command.request == 0
            || serde_json::to_vec(&command)
                .map_err(|e| e.to_string())?
                .len()
                > 8192
        {
            return Err("invalid or oversized envelope; request ID not consumed".into());
        }
        if let Some(client) = self.state.clients.get(&command.actor.client) {
            if client.role != command.actor.role {
                return Err("principal role mismatch".into());
            }
            if let Some(receipt) = client
                .receipts
                .iter()
                .find(|r| r.command.request == command.request)
            {
                if receipt.command != command {
                    return Err("request ID collision".into());
                }
                return Ok(Reply {
                    duplicate: true,
                    ack: receipt.ack.clone(),
                });
            }
            if command.request <= client.high_water {
                return Err("request expired; will never execute again".into());
            }
            if command.request != client.high_water + 1 {
                return Err("request sequence gap".into());
            }
        } else if command.request != 1 || self.state.clients.len() >= self.state.limits.clients {
            return Err("new client must start at one and fit client budget".into());
        }
        let mut next = self.state.clone();
        let attempted = if command.boot != self.state.boot {
            Err("stale worker incarnation".into())
        } else if self.state.current.as_ref() != Some(&command.binding) {
            Err("stale session/target/generation".into())
        } else {
            self.apply(&mut next, &command)
        };
        // Rejections are durable too, but partial changes of a rejected command
        // are never committed.
        let (job, rejection) = match attempted {
            Ok(job) => (job, None),
            Err(message) => {
                next = self.state.clone();
                (None, Some(message))
            }
        };
        next.event(
            job,
            Some(command.actor.clone()),
            Some(command.request),
            if rejection.is_some() {
                "Rejected"
            } else {
                "CommandAccepted"
            },
            self.flight.as_ref().map(|f| f.operation.id),
            rejection
                .as_deref()
                .unwrap_or("accepted at this serial ordering point"),
        );
        let ack = Ack {
            cursor: next.event_head,
            job,
            phase: job.and_then(|id| next.jobs.get(&id).map(|j| j.phase)),
            in_flight: self.flight.as_ref().map(|f| f.operation.id),
            rejection,
        };
        let client = next
            .clients
            .entry(command.actor.client.clone())
            .or_insert(Client {
                role: command.actor.role,
                high_water: 0,
                receipts: VecDeque::new(),
            });
        client.high_water = command.request;
        client.receipts.push_back(Receipt {
            command,
            ack: ack.clone(),
        });
        while client.receipts.len() > next.limits.receipts_per_client {
            client.receipts.pop_front();
        }
        self.persist(next, 2)?;
        Ok(Reply {
            duplicate: false,
            ack,
        })
    }
    fn create(
        next: &mut Snapshot,
        command: &Command,
        steps: Vec<Step>,
        consent: bool,
        parent: Option<u64>,
    ) -> Result<Option<u64>, String> {
        if steps.is_empty()
            || steps.len() > next.limits.steps
            || steps
                .iter()
                .any(|s| s.payload.len() > next.limits.payload_bytes)
        {
            return Err("invalid plan size".into());
        }
        if command.actor.role == Role::Agent && (consent || next.human_hold.is_some()) {
            return Err("agent cannot grant consent or bypass human hold".into());
        }
        if next
            .jobs
            .values()
            .filter(|j| j.phase == Phase::Queued)
            .count()
            >= next.limits.queued
        {
            return Err("queue full".into());
        }
        if next.jobs.len() >= next.limits.jobs {
            let removable = next
                .jobs
                .iter()
                .find(|(_, j)| {
                    j.phase.terminal()
                        && !j
                            .checkpoints
                            .iter()
                            .any(|c| next.unresolved.contains(&c.operation))
                })
                .map(|(&id, _)| id);
            if let Some(id) = removable {
                next.jobs.remove(&id);
            } else {
                return Err("retained job budget full".into());
            }
        }
        let id = next.next_job;
        next.next_job += 1;
        next.jobs.insert(
            id,
            Job {
                id,
                parent,
                creator: command.actor.clone(),
                controller: command.actor.clone(),
                binding: command.binding.clone(),
                steps,
                phase: Phase::Queued,
                next_step: 0,
                checkpoints: vec![],
                pending: None,
                consent_boot: consent.then_some(next.boot),
                invalidated: false,
                evidence_valid: false,
            },
        );
        Ok(Some(id))
    }
    fn stop_prepared(next: &mut Snapshot, id: u64, flight: Option<u64>) {
        let job = next.jobs.get_mut(&id).unwrap();
        if job.pending.as_ref().is_some_and(|p| Some(p.id) != flight) {
            let op = job.pending.take().unwrap();
            next.event(
                Some(id),
                None,
                None,
                "NotStarted",
                Some(op.id),
                "intent retired before executor entry; no effect started",
            );
        }
    }
    fn apply(&self, next: &mut Snapshot, c: &Command) -> Result<Option<u64>, String> {
        let flight = self.flight.as_ref().map(|f| f.operation.id);
        match &c.action {
            Action::Submit { steps, consent } => {
                Self::create(next, c, steps.clone(), *consent, None)
            }
            Action::Continue { job, consent } => {
                let old = next.jobs.get(job).ok_or("history not retained")?;
                if !old.phase.terminal() {
                    return Err(
                        "continue requires historical terminal work; use resume for paused work"
                            .into(),
                    );
                }
                if c.actor.role != Role::Human && c.actor != old.controller {
                    return Err("not controller".into());
                }
                if old.checkpoints.iter().any(|p| {
                    p.outcome.effect == Knowledge::Unknown && old.steps[p.step].kind == Kind::Effect
                }) {
                    return Err("uncertain effects cannot be continued/replayed; independent reconciliation required".into());
                }
                // Replay observations under the fresh scope; NEVER repeat an
                // already-known applied effect as a side effect of "continue".
                let steps = old
                    .steps
                    .iter()
                    .enumerate()
                    .filter(|(i, s)| {
                        s.kind == Kind::Observe
                            || !old
                                .checkpoints
                                .iter()
                                .any(|p| p.step == *i && p.outcome.effect == Knowledge::Applied)
                    })
                    .map(|(_, s)| s.clone())
                    .collect();
                Self::create(next, c, steps, *consent, Some(*job))
            }
            Action::TakeControl => {
                if c.actor.role != Role::Human {
                    return Err("human authority required".into());
                }
                next.human_hold = Some(c.actor.clone());
                let ids: Vec<_> = next
                    .jobs
                    .values()
                    .filter(|j| !j.phase.terminal() && j.controller.role == Role::Agent)
                    .map(|j| j.id)
                    .collect();
                for id in ids {
                    Self::stop_prepared(next, id, flight);
                    let j = next.jobs.get_mut(&id).unwrap();
                    j.controller = c.actor.clone();
                    j.consent_boot = None;
                    if j.phase != Phase::Cancelling {
                        j.phase = if j.pending.is_some() {
                            Phase::Pausing
                        } else {
                            Phase::Paused
                        };
                    }
                }
                Ok(None)
            }
            Action::ReleaseControl => {
                if next.human_hold.as_ref() != Some(&c.actor) || c.actor.role != Role::Human {
                    return Err("human hold owner required".into());
                }
                next.human_hold = None;
                Ok(None)
            }
            action => {
                let id = match action {
                    Action::Pause { job }
                    | Action::Resume { job, .. }
                    | Action::Cancel { job }
                    | Action::Authorize { job } => *job,
                    _ => unreachable!(),
                };
                let j = next.jobs.get(&id).ok_or("job not retained")?;
                if c.actor.role != Role::Human && c.actor != j.controller {
                    return Err("not controller".into());
                }
                if j.binding != c.binding || j.invalidated {
                    return Err("job scope revoked".into());
                }
                match action {
                    Action::Pause { .. } | Action::Cancel { .. } => {
                        if j.phase.terminal() {
                            return Err("historical job is not active".into());
                        }
                        if j.phase == Phase::Cancelling && matches!(action, Action::Pause { .. }) {
                            return Err("cancellation cannot be downgraded".into());
                        }
                        Self::stop_prepared(next, id, flight);
                        let j = next.jobs.get_mut(&id).unwrap();
                        if c.actor.role == Role::Human {
                            j.controller = c.actor.clone();
                            j.consent_boot = None;
                        }
                        j.phase =
                            match (matches!(action, Action::Cancel { .. }), j.pending.is_some()) {
                                (true, true) => Phase::Cancelling,
                                (true, false) => Phase::Cancelled,
                                (false, true) => Phase::Pausing,
                                (false, false) => Phase::Paused,
                            };
                    }
                    Action::Resume { renew_consent, .. } => {
                        if j.phase != Phase::Paused {
                            return Err("only a paused current-generation job can resume".into());
                        }
                        if next.human_hold.is_some() && c.actor.role == Role::Agent {
                            return Err("human hold".into());
                        }
                        if *renew_consent && c.actor.role != Role::Human {
                            return Err("human consent required".into());
                        }
                        if j.has_effects() && j.consent_boot != Some(next.boot) && !renew_consent {
                            return Err("renew effect scope/consent".into());
                        }
                        if next
                            .jobs
                            .values()
                            .filter(|j| j.phase == Phase::Queued)
                            .count()
                            >= next.limits.queued
                        {
                            return Err("queue full".into());
                        }
                        let j = next.jobs.get_mut(&id).unwrap();
                        if *renew_consent {
                            j.consent_boot = Some(next.boot);
                        }
                        j.phase = Phase::Queued;
                    }
                    Action::Authorize { .. } => {
                        if c.actor.role != Role::Human || j.phase.terminal() {
                            return Err("human authorization of active job required".into());
                        }
                        next.jobs.get_mut(&id).unwrap().consent_boot = Some(next.boot);
                    }
                    _ => unreachable!(),
                }
                Ok(Some(id))
            }
        }
    }
    pub fn tick(&mut self, executor: &mut impl Executor) -> Result<Progress, String> {
        if self.fence.is_some() {
            return Ok(Progress::Fenced);
        }
        if let Some(f) = &self.flight {
            if self.needs_drain {
                return Ok(Progress::NeedsDrain(f.operation.id));
            }
            if f.polls >= self.state.limits.polls {
                self.needs_drain = true;
                let id = f.operation.id;
                let job = f.operation.job;
                let mut next = self.state.clone();
                next.jobs.get_mut(&job).unwrap().phase = Phase::Cancelling;
                next.event(
                    Some(job),
                    None,
                    None,
                    "PollBudgetExceeded",
                    Some(id),
                    "in-flight work unresolved; explicit drain required; no new dispatch",
                );
                self.persist(next, 1)?;
                return Ok(Progress::NeedsDrain(id));
            }
            return self.poll(executor);
        }
        if !self.state.unresolved.is_empty() {
            return Ok(Progress::Fenced);
        }
        let running = self
            .state
            .jobs
            .values()
            .find(|j| j.phase == Phase::Running)
            .map(|j| j.id);
        let Some(id) = running else {
            let candidate = self
                .state
                .jobs
                .values()
                .find(|j| {
                    j.phase == Phase::Queued
                        && self.state.current.as_ref() == Some(&j.binding)
                        && !j.invalidated
                        && (!j.has_effects() || j.consent_boot == Some(self.state.boot))
                        && (self.state.human_hold.is_none() || j.controller.role == Role::Human)
                })
                .map(|j| j.id);
            let Some(id) = candidate else {
                return Ok(Progress::Idle);
            };
            let mut next = self.state.clone();
            next.jobs.get_mut(&id).unwrap().phase = Phase::Running;
            next.event(
                Some(id),
                None,
                None,
                "Running",
                None,
                "worker owns this job; no operation started",
            );
            self.persist(next, 2)?;
            return Ok(Progress::Transition);
        };
        let job = &self.state.jobs[&id];
        if let Some(operation) = job.pending.clone() {
            // Last guard immediately before executor entry, after durable intent.
            if self.state.current.as_ref() != Some(&operation.binding)
                || job.invalidated
                || (operation.instruction.kind == Kind::Effect
                    && job.consent_boot != Some(self.state.boot))
            {
                return Err("dispatch guard refused stale scope/consent".into());
            }
            self.flight = Some(Flight {
                operation: operation.clone(),
                polls: 0,
            });
            match executor.begin(&operation) {
                Poll::Pending => Ok(Progress::Started(operation.id)),
                Poll::Ready(outcome) => self.finish(outcome),
            }
        } else if job.next_step == job.steps.len() {
            let mut next = self.state.clone();
            next.jobs.get_mut(&id).unwrap().phase = Phase::Completed;
            next.event(
                Some(id),
                None,
                None,
                "Completed",
                None,
                "all known checkpoints recorded",
            );
            self.persist(next, 2)?;
            Ok(Progress::Transition)
        } else {
            let operation = Operation {
                id: self.state.next_operation,
                job: id,
                step: job.next_step,
                binding: job.binding.clone(),
                instruction: job.steps[job.next_step].clone(),
                result_limit: self
                    .state
                    .limits
                    .result_per_step
                    .min(self.state.limits.result_per_job - job.result_bytes()),
            };
            let mut next = self.state.clone();
            next.next_operation += 1;
            next.jobs.get_mut(&id).unwrap().pending = Some(operation.clone());
            next.event(
                Some(id),
                None,
                None,
                "IntentRecorded",
                Some(operation.id),
                "persisted before executor entry; not an effect acknowledgement",
            );
            self.persist(next, 2)?;
            Ok(Progress::Intent(operation.id))
        }
    }
    fn poll(&mut self, executor: &mut impl Executor) -> Result<Progress, String> {
        let flight = self.flight.as_mut().unwrap();
        flight.polls = flight.polls.saturating_add(1);
        let id = flight.operation.id;
        let job = &self.state.jobs[&flight.operation.job];
        let cancel = self.fence.is_some()
            || matches!(job.phase, Phase::Cancelling | Phase::Pausing)
            || job.invalidated;
        match executor.poll(id, cancel) {
            Poll::Pending => Ok(Progress::Waiting(id)),
            Poll::Ready(outcome) => self.finish(outcome),
        }
    }
    /// Worker-only, explicit one-poll drain after budget/storage failure.
    /// Never starts another operation. Cannot interrupt a blocking native call.
    pub fn drain(&mut self, executor: &mut impl Executor) -> Result<Progress, String> {
        if self.flight.is_none() {
            return Ok(Progress::Idle);
        }
        self.poll(executor)
    }
    fn finish(&mut self, mut outcome: Outcome) -> Result<Progress, String> {
        let flight = self.flight.take().unwrap();
        let op = flight.operation;
        self.needs_drain = false;
        outcome.note = bounded(&outcome.note, 256);
        if outcome.data.len() > op.result_limit {
            outcome.data.clear();
            outcome.success = false;
            outcome.note =
                "executor exceeded result budget; actual effect knowledge preserved".into();
        }
        if (op.instruction.kind == Kind::Observe && outcome.effect != Knowledge::NotApplicable)
            || (op.instruction.kind == Kind::Effect && outcome.effect == Knowledge::NotApplicable)
            || (outcome.success
                && op.instruction.kind == Kind::Effect
                && outcome.effect != Knowledge::Applied)
        {
            outcome = Outcome::unknown("executor outcome violates classification contract");
        }
        if self.fence.is_some() {
            self.volatile_outcome = Some((op.id, outcome));
            return Ok(Progress::Fenced);
        }
        let mut next = self.state.clone();
        let j = next.jobs.get_mut(&op.job).unwrap();
        j.pending = None;
        j.checkpoints.push(Checkpoint {
            operation: op.id,
            step: op.step,
            outcome: outcome.clone(),
        });
        if outcome.effect != Knowledge::Unknown {
            j.next_step += 1;
        }
        j.evidence_valid =
            !j.invalidated && next.current.as_ref() == Some(&j.binding) && outcome.success;
        j.phase = if j.invalidated || outcome.effect == Knowledge::Unknown {
            Phase::Interrupted
        } else if j.phase == Phase::Cancelling {
            Phase::Cancelled
        } else if !outcome.success {
            Phase::Failed
        } else if j.phase == Phase::Pausing {
            Phase::Paused
        } else if j.next_step == j.steps.len() {
            Phase::Completed
        } else {
            Phase::Running
        };
        if outcome.effect == Knowledge::Unknown && op.instruction.kind == Kind::Effect {
            next.unresolved.insert(op.id);
        }
        let phase = j.phase;
        next.event(
            Some(op.job),
            None,
            None,
            "OutcomeRecorded",
            Some(op.id),
            &format!("{:?}; job {:?}", outcome.effect, phase),
        );
        if let Err(error) = self.persist(next, 1) {
            self.volatile_outcome = Some((op.id, outcome));
            return Err(error);
        }
        Ok(Progress::Settled(op.id))
    }
    /// Trusted backend ONLY: independent proof that an unknown operation is now
    /// quiescent. Optional outcome must come from an external receipt, not a log
    /// guess. This never resumes work or resurrects prior evidence.
    pub fn reconcile(&mut self, operation: u64, known: Option<Outcome>) -> Result<(), String> {
        self.healthy()?;
        if !self.state.unresolved.contains(&operation) {
            return Err("no such unresolved effect".into());
        }
        let mut next = self.state.clone();
        if let Some(mut outcome) = known {
            if outcome.effect == Knowledge::Unknown
                || outcome.effect == Knowledge::NotApplicable
                || (outcome.success && outcome.effect != Knowledge::Applied)
                || outcome.data.len() > next.limits.result_per_step
            {
                return Err("invalid independent receipt".into());
            }
            outcome.note = bounded(&outcome.note, 256);
            for j in next.jobs.values_mut() {
                let current_size = j.result_bytes();
                if let Some(cp) = j.checkpoints.iter_mut().find(|c| c.operation == operation) {
                    if current_size + outcome.data.len() > next.limits.result_per_job {
                        return Err("receipt exceeds result budget".into());
                    }
                    cp.outcome = outcome.clone();
                }
            }
        }
        next.unresolved.remove(&operation);
        next.event(
            None,
            None,
            None,
            "ReconciledQuiescent",
            Some(operation),
            "trusted external witness; history stays interrupted, no automatic replay",
        );
        self.persist(next, 2)
    }
}
