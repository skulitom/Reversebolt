//! Responsive job control around the sole engine-owning worker.
use crate::service::EngineHandle;
use reversebolt_core::{Command, Result, engine::View as EngineView};
use reversebolt_session_jobs::{
    self as job,
    journal::{FileJournal, Journal},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock, atomic::AtomicBool, mpsc},
    time::{Duration, Instant},
};

pub fn panel(ui: &mut eframe::egui::Ui, handle: &Handle, state: &job::View) {
    use crate::theme::{ACCENT, ERROR, MUTED, SURFACE};
    use eframe::egui::{self, RichText};
    let error = handle.job_error.read().unwrap().clone();
    if state.history.jobs.is_empty()
        && error.is_none()
        && state.fence.is_none()
        && state.history.human_hold.is_none()
    {
        return;
    }
    let only_history = state.history.jobs.values().all(|j| j.phase.terminal())
        && state.history.human_hold.is_none()
        && error.is_none()
        && state.fence.is_none();
    if only_history {
        egui::CollapsingHeader::new("Observation history").show(ui, |ui| {
            for job in state.history.jobs.values().rev().take(6) {
                ui.label(format!(
                    "Observation {} · {:?} · {} recorded moments",
                    job.id, job.phase, job.next_step
                ));
            }
        });
        ui.add_space(8.);
        return;
    }
    egui::Frame::new().fill(SURFACE).inner_margin(12).corner_radius(8).show(ui,|ui| {
        for job in state.history.jobs.values().filter(|j|!j.phase.terminal()) {
            let phase=match job.phase {
                job::Phase::Queued=>"Queued",job::Phase::Running=>"Observing",job::Phase::Pausing=>"Finishing this capture, then pausing",
                job::Phase::Paused=>"Paused",job::Phase::Cancelling=>"Finishing this capture, then stopping",_=>"Historical",
            };
            let automatic = job.steps.first().and_then(|s|serde_json::from_str::<CaptureStep>(&s.payload).ok()).is_some_and(|s|s.compare);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{} · {phase} · {} / {} moments",if automatic {"Finding controls"}else{"Observation"},job.next_step,job.steps.len())).color(ACCENT));
                if job.phase==job::Phase::Paused {
                    if ui.button("Resume").clicked() {handle.submit("job_control",json!({"operation":"resume","job":job.id}));}
                }else if matches!(job.phase,job::Phase::Running|job::Phase::Queued) && ui.button("Pause").on_hover_text("Finish the current capture, then pause.").clicked() {
                    handle.submit("job_control",json!({"operation":"pause","job":job.id}));
                }
                if ui.button("Stop").on_hover_text("Finish any capture already in progress; prevent later captures.").clicked() {handle.submit("job_control",json!({"operation":"cancel","job":job.id}));}
                if state.history.human_hold.is_none() && job.controller.role==job::Role::Agent
                    && ui.button("Take control").clicked() {handle.submit("job_control",json!({"operation":"take_control"}));}
            });
        }
        if state.history.human_hold.is_some() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("You have control. Agent changes are held.").color(MUTED));
                if ui.button("Release control").clicked() {handle.submit("job_control",json!({"operation":"release_control"}));}
            });
        }
        if let Some(error)=state.fence.as_ref().or(error.as_ref()) {
            let message=match error.as_str() {
                "stale worker incarnation"=>"Reversebolt restarted. Refresh the agent's session before starting new work.",
                "agent cannot grant consent or bypass human hold"|"human hold"=>"You have control. Release control to let the agent start new work.",
                "not controller"=>"This job is now controlled from the desktop.",
                "human authority required"=>"Use Take control in this window to manage the agent's work.",
                "job scope revoked"=>"The application or memory batch changed. Start a new observation job.",
                other=>other,
            };
            ui.colored_label(ERROR,message);
        }
    });
    ui.add_space(12.);
}

type Request = (Command, Option<mpsc::Sender<Value>>);
#[derive(Clone)]
pub struct Handle {
    inbox: Arc<Mutex<VecDeque<Request>>>,
    pub view: Arc<RwLock<EngineView>>,
    pub busy: Arc<AtomicBool>,
    pub jobs: Arc<RwLock<job::View>>,
    pub job_error: Arc<RwLock<Option<String>>>,
}
fn control(c: &Command) -> bool {
    c.source == "desktop"
        && c.action == "job_control"
        && matches!(
            c.args["operation"].as_str(),
            Some("pause" | "cancel" | "take_control")
        )
}
fn read_only(action: &str) -> bool {
    matches!(
        action,
        "status" | "history" | "job_events" | "read_capture" | "read_sequence" | "list_processes"
    )
}
fn pop(inbox: &mut VecDeque<Request>) -> Option<Request> {
    let index = inbox
        .iter()
        .position(|(c, _)| control(c))
        .and_then(|i| {
            let source = &inbox[i].0.source;
            inbox.iter().position(|(c, _)| &c.source == source)
        })
        .unwrap_or(0);
    inbox.remove(index)
}
impl Handle {
    pub fn start(path: PathBuf) -> Result<Self> {
        let journal = FileJournal::open(
            &path
                .parent()
                .ok_or("Missing session directory")?
                .join("jobs"),
        )?;
        let session = journal
            .load()
            .map(|s| s.session)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let jobs = job::Jobs::open(journal, &session, job::Limits::default())?;
        let engine = EngineHandle::start(path)?;
        let handle = Self {
            inbox: Arc::new(Mutex::new(VecDeque::new())),
            view: engine.view.clone(),
            busy: engine.busy.clone(),
            jobs: Arc::new(RwLock::new(jobs.view())),
            job_error: Arc::new(RwLock::new(None)),
        };
        let inbox = Arc::downgrade(&handle.inbox);
        let published = handle.jobs.clone();
        let errors = handle.job_error.clone();
        std::thread::spawn(move || {
            let mut controller = Controller {
                jobs,
                executor: CaptureExecutor::new(engine.clone()),
                manual: None,
                engine,
                published,
                errors,
                draining: false,
                next_capture: None,
            };
            loop {
                let Some(inbox) = inbox.upgrade() else { break };
                controller.refresh_binding();
                controller.finish_manual();
                // Human controls lead other clients without reordering one source.
                for _ in 0..24 {
                    let request = pop(&mut inbox.lock().unwrap());
                    let Some(request) = request else { break };
                    controller.command(request);
                }
                drop(inbox);
                controller.tick();
                controller.publish();
                std::thread::sleep(Duration::from_millis(25));
            }
        });
        Ok(handle)
    }
    fn enqueue(&self, request: Request) -> Result<()> {
        let priority = control(&request.0);
        let mut inbox = self.inbox.lock().unwrap();
        if inbox.iter().filter(|(c, _)| control(c) == priority).count()
            >= if priority { 8 } else { 16 }
        {
            return Err("The command lane is full; this request was not accepted.".into());
        }
        inbox.push_back(request);
        Ok(())
    }
    pub fn submit(&self, action: &str, args: Value) {
        if let Err(error) = self.enqueue((
            Command {
                action: action.into(),
                args,
                source: "desktop".into(),
            },
            None,
        )) {
            *self.job_error.write().unwrap() = Some(error);
        }
    }
    pub fn call(&self, command: Command) -> Result<Value> {
        let (send, receive) = mpsc::channel();
        self.enqueue((command, Some(send)))?;
        receive.recv().map_err(|e| e.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    investigation_id: String,
    batch: usize,
    created: String,
    window: u64,
}
impl Scope {
    fn from_view(view: &EngineView) -> Option<Self> {
        view.live.as_ref().filter(|v| v.connected).map(|v| Self {
            investigation_id: v.investigation_id.clone(),
            batch: v.coverage.batch,
            created: v.process.created.clone(),
            window: v.process.window,
        })
    }
    fn key(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.investigation_id, self.batch, self.created, self.window
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureStep {
    scope: Scope,
    delay_ms: u64,
    source: String,
    #[serde(default, skip_serializing_if = "is_false")]
    compare: bool,
}

fn is_false(value: &bool) -> bool {
    !value
}

struct Flight {
    id: u64,
    response: mpsc::Receiver<Value>,
}
struct CaptureExecutor {
    engine: EngineHandle,
    flight: Option<Flight>,
    ready: Option<Value>,
    next_poll: Instant,
}
impl CaptureExecutor {
    fn new(engine: EngineHandle) -> Self {
        Self {
            engine,
            flight: None,
            ready: None,
            next_poll: Instant::now(),
        }
    }
    fn outcome(success: bool, note: &str, data: Value) -> job::Poll {
        job::Poll::Ready(job::Outcome {
            success,
            effect: job::Knowledge::NotApplicable,
            data: serde_json::to_vec(&data).unwrap_or_default(),
            note: note.into(),
        })
    }
    /// Look for actual completion without counting busy polling as a job poll.
    fn can_poll(&mut self) -> bool {
        match &self.flight {
            Some(Flight { response, .. }) => {
                if self.ready.is_none() {
                    match response.try_recv() {
                        Ok(value) => self.ready = Some(value),
                        Err(mpsc::TryRecvError::Disconnected) => {
                            self.ready = Some(
                                json!({"ok":false,"error":"Engine worker ended before returning the capture outcome."}),
                            )
                        }
                        Err(mpsc::TryRecvError::Empty) => {}
                    }
                }
                (self.ready.is_some() && !crate::native::renderer_busy())
                    || Instant::now() >= self.next_poll
            }
            None => true,
        }
    }
}
impl job::Executor for CaptureExecutor {
    fn begin(&mut self, operation: &job::Operation) -> job::Poll {
        if self.flight.is_some() || operation.instruction.kind != job::Kind::Observe {
            return Self::outcome(
                false,
                "Only one typed observation operation can run.",
                Value::Null,
            );
        }
        let step: CaptureStep = match serde_json::from_str(&operation.instruction.payload) {
            Ok(step) => step,
            Err(_) => return Self::outcome(false, "Unsupported observation plan.", Value::Null),
        };
        if step.delay_ms > 60000 || step.scope.key() != operation.binding.target {
            return Self::outcome(
                false,
                "Observation plan does not match its immutable binding.",
                Value::Null,
            );
        }
        let scope = Scope::from_view(&self.engine.view.read().unwrap());
        if scope.as_ref().map(Scope::key) != Some(step.scope.key()) {
            return Self::outcome(
                false,
                "The target or memory batch changed before capture.",
                Value::Null,
            );
        }
        let command = Command {
            action: "capture".into(),
            args: json!({"scope":step.scope,"compare":step.compare}),
            source: step.source,
        };
        match self.engine.start_command(command) {
            Ok(response) => {
                self.flight = Some(Flight {
                    id: operation.id,
                    response,
                });
                self.next_poll = Instant::now() + Duration::from_secs(1);
                job::Poll::Pending
            }
            Err(error) => Self::outcome(false, &error, Value::Null),
        }
    }
    fn poll(&mut self, id: u64, _cancel: bool) -> job::Poll {
        self.next_poll = Instant::now() + Duration::from_secs(1);
        // A timed-out window worker still owns its output path. Do not declare
        // quiescence or start the next operation until that worker has settled.
        if crate::native::renderer_busy() {
            return job::Poll::Pending;
        }
        match self.flight.take() {
            Some(Flight {
                id: owned,
                response,
            }) if id == owned => {
                let value = self.ready.take().or_else(|| response.try_recv().ok());
                if let Some(value) = value {
                    let live = &value["state"]["live"];
                    Self::outcome(
                        value["ok"] == true,
                        value["error"].as_str().unwrap_or(
                            "Capture finished; see the shared investigation for evidence.",
                        ),
                        json!({"captured":value["ok"]==true,"investigation_id":live["investigation_id"],
                            "batch":live["coverage"]["batch"],"capture":live["sample_count"],
                            "frame":live["frames"].as_array().and_then(|f| f.last()).map(|f|&f["id"])}),
                    )
                } else {
                    self.flight = Some(Flight { id, response });
                    job::Poll::Pending
                }
            }
            other => {
                self.flight = other;
                Self::outcome(
                    false,
                    "No matching engine operation is owned by this job.",
                    Value::Null,
                )
            }
        }
    }
}

struct Manual {
    response: mpsc::Receiver<Value>,
    reply: Option<mpsc::Sender<Value>>,
    automatic: Option<String>,
}
struct Controller {
    jobs: job::Jobs<FileJournal>,
    executor: CaptureExecutor,
    engine: EngineHandle,
    manual: Option<Manual>,
    published: Arc<RwLock<job::View>>,
    errors: Arc<RwLock<Option<String>>>,
    draining: bool,
    next_capture: Option<(u64, usize, Instant)>,
}

impl Controller {
    fn publish(&self) {
        *self.published.write().unwrap() = self.jobs.view();
    }
    fn error(&self, error: String) {
        *self.errors.write().unwrap() = Some(error);
    }
    fn response(&self, ok: bool, error: Option<String>, data: Value) -> Value {
        json!({"ok":ok,"error":error,"data":data,"state":*self.engine.view.read().unwrap(),"jobs":self.jobs.view()})
    }
    fn refresh_binding(&mut self) {
        let next = Scope::from_view(&self.engine.view.read().unwrap()).map(|s| s.key());
        if self
            .jobs
            .snapshot()
            .current
            .as_ref()
            .map(|b| b.target.as_str())
            != next.as_deref()
            && let Err(error) = self.jobs.bind(next.as_deref())
        {
            self.error(error);
        }
    }
    fn finish_manual(&mut self) {
        let Some(manual) = self.manual.take() else {
            return;
        };
        match manual.response.try_recv() {
            Ok(mut value) => {
                self.refresh_binding();
                if value["ok"] == true
                    && let Some(source) = manual.automatic
                {
                    match self.start_automatic_observation(source) {
                        Ok(receipt) => value["data"] = json!({"automatic_observation":receipt}),
                        Err(error) => {
                            self.error(format!(
                                "Connected, but automatic observation could not start: {error}"
                            ));
                            value["data"] = json!({"automatic_observation_error":error});
                        }
                    }
                }
                value["jobs"] = json!(self.jobs.view());
                if let Some(reply) = manual.reply {
                    let _ = reply.send(value);
                }
            }
            Err(mpsc::TryRecvError::Empty) => self.manual = Some(manual),
            Err(mpsc::TryRecvError::Disconnected) => {
                let error = "The engine worker ended while handling a command.".to_string();
                self.error(error.clone());
                if let Some(reply) = manual.reply {
                    let _ = reply.send(self.response(false, Some(error), Value::Null));
                }
            }
        }
    }
    fn start_automatic_observation(&mut self, source: String) -> Result<Value> {
        let snapshot = self.jobs.snapshot();
        let client = if source == "desktop" {
            "desktop"
        } else {
            "mcp-agent"
        };
        let command = Command {
            action: "start_observation_job".into(),
            source,
            args: json!({"count":16,"interval_ms":1000,"compare":true,
                "request_id":snapshot.clients.get(client).map_or(1,|c|c.high_water+1),
                "boot":snapshot.boot,"binding":snapshot.current}),
        };
        let receipt = self.job_command(&command)?;
        if let Some(error) = receipt["ack"]["rejection"].as_str() {
            return Err(error.into());
        }
        Ok(receipt)
    }
    fn command(&mut self, (command, reply): Request) {
        let result: Result<Value> = (|| {
            if matches!(command.action.as_str(), "status" | "history") {
                return Ok(Value::Null);
            }
            if command.action == "job_events" {
                return self.jobs.events_after(command.args["cursor"].as_u64().unwrap_or(0),64)
                    .map(|p|json!(p)).map_err(|e|format!("Event cursor unavailable: {e:?}. Refresh status and use its event_head."));
            }
            if matches!(
                command.action.as_str(),
                "start_observation_job" | "job_control"
            ) {
                return self.job_command(&command);
            }
            if self.jobs.snapshot().human_hold.is_some()
                && command.source != "desktop"
                && !read_only(&command.action)
            {
                return Err(
                    "The desktop has control. Agent changes are held until control is released."
                        .into(),
                );
            }
            let active = self
                .jobs
                .snapshot()
                .jobs
                .values()
                .any(|j| !j.phase.terminal() && j.phase != job::Phase::Paused);
            if active
                && !read_only(&command.action)
                && !matches!(
                    command.action.as_str(),
                    "submit_observation" | "submit_temporal_observation"
                )
            {
                return Err("An observation job owns the investigation. Pause or cancel it, or take control from the desktop.".into());
            }
            if self.manual.is_some() || self.executor.flight.is_some() {
                return Err("A capture or engine command is still in progress. Status and job controls remain available.".into());
            }
            if command
                .args
                .get("auto_observe")
                .is_some_and(|v| !v.is_boolean())
            {
                return Err("auto_observe must be true or false.".into());
            }
            let automatic = match command.action.as_str() {
                "attach_process" => command.args["auto_observe"].as_bool().unwrap_or(true),
                "advance_memory" => command.args["auto_observe"] == true,
                _ => false,
            };
            let response = self.engine.start_command(command.clone())?;
            self.manual = Some(Manual {
                response,
                reply: reply.clone(),
                automatic: automatic.then(|| command.source.clone()),
            });
            Ok(Value::Null)
        })();
        // Manual engine commands respond when their actual operation settles.
        let is_job = matches!(
            command.action.as_str(),
            "status" | "history" | "job_events" | "start_observation_job" | "job_control"
        );
        match result {
            Ok(data) => {
                if is_job {
                    let rejection = data["ack"]["rejection"].as_str().map(str::to_owned);
                    if let Some(error) = &rejection {
                        self.error(error.clone());
                    }
                    if let Some(reply) = reply {
                        let _ = reply.send(self.response(rejection.is_none(), rejection, data));
                    }
                }
            }
            Err(error) => {
                self.error(error.clone());
                if let Some(reply) = reply {
                    let _ = reply.send(self.response(false, Some(error), Value::Null));
                }
            }
        }
    }
    fn job_command(&mut self, command: &Command) -> Result<Value> {
        if serde_json::to_vec(&command.args)
            .map_err(|e| e.to_string())?
            .len()
            > 8192
        {
            return Err("Job envelope exceeds 8 KiB.".into());
        }
        let human = command.source == "desktop";
        let actor = job::Actor {
            client: if human { "desktop" } else { "mcp-agent" }.into(),
            role: if human {
                job::Role::Human
            } else {
                job::Role::Agent
            },
        };
        let snapshot = self.jobs.snapshot();
        let request = if human {
            snapshot
                .clients
                .get(&actor.client)
                .map_or(1, |c| c.high_water + 1)
        } else {
            command.args["request_id"]
                .as_u64()
                .ok_or("Supply the next request_id from jobs.history.clients for mcp-agent.")?
        };
        let boot = if human {
            snapshot.boot
        } else {
            command.args["boot"]
                .as_u64()
                .ok_or("Supply the current job boot from status.")?
        };
        let binding = if human {
            snapshot
                .current
                .clone()
                .ok_or("Connect an application before controlling jobs.")?
        } else {
            serde_json::from_value::<job::Binding>(command.args["binding"].clone())
                .map_err(|_| "Supply the exact job binding from status.")?
        };
        let action = if command.action == "start_observation_job" {
            let count = command.args["count"].as_u64().unwrap_or(12) as usize;
            let interval = command.args["interval_ms"].as_u64().unwrap_or(1000);
            if !(1..=16).contains(&count) || !(250..=60000).contains(&interval) {
                return Err("Use 1–16 captures and a 250–60,000 ms interval.".into());
            }
            // Reconstruct a replay from its original binding, not today's target.
            let parts: Vec<_> = binding.target.split('/').collect();
            if parts.len() != 4 {
                return Err("Invalid capture binding.".into());
            }
            let scope = Scope {
                investigation_id: parts[0].into(),
                batch: parts[1].parse().map_err(|_| "Invalid batch binding.")?,
                created: parts[2].into(),
                window: parts[3].parse().map_err(|_| "Invalid window binding.")?,
            };
            let fresh = request
                > snapshot
                    .clients
                    .get(&actor.client)
                    .map_or(0, |c| c.high_water);
            if fresh && snapshot.current.as_ref() == Some(&binding) {
                let state = self.engine.view.read().unwrap();
                let live = state
                    .live
                    .as_ref()
                    .ok_or("Connect an application before starting an observation job.")?;
                let reserved: usize = snapshot
                    .jobs
                    .values()
                    .filter(|j| !j.phase.terminal() && j.binding == binding)
                    .map(|j| j.steps.len() - j.next_step)
                    .sum();
                if live.sample_count + count + reserved > 20
                    || (live.sample_count + count + reserved) * live.planned_bytes
                        > 512 * 1024 * 1024
                {
                    return Err("The requested captures exceed this batch's remaining retention budget, including queued jobs.".into());
                }
            }
            let mut steps = vec![];
            let compare = command.args["compare"].as_bool().unwrap_or(false);
            for index in 0..count {
                steps.push(job::Step {
                    kind: job::Kind::Observe,
                    payload: serde_json::to_string(&CaptureStep {
                        scope: scope.clone(),
                        delay_ms: if compare && index == 0 { 0 } else { interval },
                        source: command.source.clone(),
                        compare,
                    })
                    .map_err(|e| e.to_string())?,
                });
            }
            job::Action::Submit {
                steps,
                consent: false,
            }
        } else {
            let id = command.args["job"].as_u64().unwrap_or(0);
            match command.args["operation"].as_str() {
                Some("pause") => job::Action::Pause { job: id },
                Some("resume") => job::Action::Resume {
                    job: id,
                    renew_consent: false,
                },
                Some("cancel") => job::Action::Cancel { job: id },
                Some("take_control") => job::Action::TakeControl,
                Some("release_control") => job::Action::ReleaseControl,
                _ => {
                    return Err(
                        "Choose pause, resume, cancel, take_control or release_control.".into(),
                    );
                }
            }
        };
        let reply = self.jobs.command(job::Command {
            request,
            actor,
            boot,
            binding,
            action,
        })?;
        *self.errors.write().unwrap() = None;
        Ok(json!(reply))
    }
    fn tick(&mut self) {
        if self.manual.is_some() {
            return;
        }
        let view = self.jobs.view();
        if view.in_flight.is_none() && crate::native::renderer_busy() {
            return;
        }
        if view.in_flight.is_some() && !self.executor.can_poll() {
            return;
        }
        if view.in_flight.is_none()
            && let Some(running) = self
                .jobs
                .snapshot()
                .jobs
                .values()
                .find(|j| j.phase == job::Phase::Running)
            && running.pending.is_none()
            && running.next_step < running.steps.len()
        {
            let marker = (running.id, running.next_step);
            if self.next_capture.as_ref().map(|(j, s, _)| (*j, *s)) != Some(marker) {
                let delay =
                    serde_json::from_str::<CaptureStep>(&running.steps[running.next_step].payload)
                        .map_or(0, |s| s.delay_ms);
                self.next_capture = Some((
                    marker.0,
                    marker.1,
                    Instant::now() + Duration::from_millis(delay),
                ));
            }
            if self
                .next_capture
                .as_ref()
                .is_some_and(|(_, _, due)| Instant::now() < *due)
            {
                return;
            }
        }
        let result = if self.draining || view.fence.is_some() {
            if self.executor.ready.is_none() {
                return;
            }
            self.jobs.drain(&mut self.executor)
        } else {
            self.jobs.tick(&mut self.executor)
        };
        match result {
            Ok(job::Progress::NeedsDrain(_)) => {
                self.draining = true;
                self.error("A capture is taking longer than expected. Its operation remains owned; no further capture will start.".into());
            }
            Ok(job::Progress::Settled(_)) => self.draining = false,
            Err(error) => self.error(error),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn initial_view() -> EngineView {
        EngineView {
            live: Some(reversebolt_core::live::LiveView {
                temporal_tracks: vec![],
                prediction_check: None,
                coverage: reversebolt_core::coverage::Coverage {
                    batch: 1,
                    ..Default::default()
                },
                investigation_id: "fixture-investigation".into(),
                frames: vec![],
                visual_tracks: vec![],
                process: reversebolt_core::live::ProcessInfo {
                    created: "creation-1".into(),
                    window: 42,
                    ..Default::default()
                },
                eligible_bytes: 1048576,
                planned_bytes: 1048576,
                sample_count: 0,
                retained_bytes: 0,
                latest: None,
                tracks: vec![],
                phase: "fixture".into(),
                connected: true,
            }),
            ..EngineView::default()
        }
    }
    fn controller(engine: EngineHandle) -> Controller {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/job-adapter-tests")
            .join(uuid::Uuid::new_v4().to_string());
        let jobs = job::Jobs::open(
            FileJournal::open(&directory).unwrap(),
            "fixture-ledger",
            job::Limits::default(),
        )
        .unwrap();
        let published = Arc::new(RwLock::new(jobs.view()));
        let mut c = Controller {
            jobs,
            executor: CaptureExecutor::new(engine.clone()),
            engine,
            manual: None,
            published,
            errors: Arc::new(RwLock::new(None)),
            draining: false,
            next_capture: None,
        };
        c.refresh_binding();
        c
    }
    fn request(c: &Controller, action: &str, extra: Value) -> Command {
        let history = c.jobs.snapshot();
        let mut args = json!({"request_id":history.clients.get("mcp-agent").map_or(1,|c|c.high_water+1),
            "boot":history.boot,"binding":history.current});
        args.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        Command {
            action: action.into(),
            args,
            source: "agent".into(),
        }
    }
    #[test]
    fn cancel_ack_is_responsive_and_waits_for_the_actual_engine_outcome() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let engine = EngineHandle::test_worker(initial_view(), move |command, view| {
            started_tx.send(command).unwrap();
            release_rx.recv().unwrap();
            view.live.as_mut().unwrap().sample_count += 1;
            json!({"ok":true,"error":null,"state":view})
        });
        let mut c = controller(engine);
        let start = request(
            &c,
            "start_observation_job",
            json!({"count":3,"interval_ms":250}),
        );
        let ack = c.job_command(&start).unwrap();
        let id = ack["ack"]["job"].as_u64().unwrap();
        c.tick();
        c.next_capture = Some((id, 0, Instant::now()));
        c.tick();
        c.tick();
        let actual = started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(actual.action, "capture");
        assert_eq!(
            actual.args["scope"]["investigation_id"],
            "fixture-investigation"
        );
        let cancel = request(&c, "job_control", json!({"job":id,"operation":"cancel"}));
        let (reply, receive) = mpsc::channel();
        let begin = Instant::now();
        c.command((cancel, Some(reply)));
        let result = receive.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(begin.elapsed() < Duration::from_secs(1));
        assert_eq!(result["data"]["ack"]["phase"], "Cancelling");
        assert!(c.jobs.view().in_flight.is_some());
        release_tx.send(()).unwrap();
        for _ in 0..100 {
            c.tick();
            if c.jobs.snapshot().jobs[&id].phase == job::Phase::Cancelled {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Cancelled);
        assert_eq!(c.jobs.snapshot().jobs[&id].next_step, 1);
        for _ in 0..5 {
            c.tick();
        }
        assert!(started_rx.try_recv().is_err());
    }
    #[test]
    fn replay_uses_original_scope_after_rebinding_and_cannot_start_new_work() {
        let engine =
            EngineHandle::test_worker(initial_view(), |_, _| panic!("No capture should start"));
        let mut c = controller(engine);
        let command = request(
            &c,
            "start_observation_job",
            json!({"count":12,"interval_ms":1000}),
        );
        let first = c.job_command(&command).unwrap();
        c.engine.view.write().unwrap().live = None;
        c.refresh_binding();
        let replay = c.job_command(&command).unwrap();
        assert_eq!(replay["duplicate"], true);
        assert_eq!(first["ack"], replay["ack"]);
        let mut conflict = command.clone();
        conflict.args["count"] = json!(11);
        assert!(c.job_command(&conflict).is_err());
        c.tick();
        assert_eq!(c.jobs.snapshot().jobs.len(), 1);
        assert!(
            c.jobs
                .snapshot()
                .jobs
                .values()
                .all(|j| j.phase == job::Phase::Interrupted)
        );
    }
    #[test]
    fn agent_cannot_take_control_and_human_takeover_holds_future_captures() {
        let engine =
            EngineHandle::test_worker(initial_view(), |_, _| panic!("No capture should start"));
        let mut c = controller(engine);
        let start = request(
            &c,
            "start_observation_job",
            json!({"count":2,"interval_ms":1000}),
        );
        let first = c.job_command(&start).unwrap();
        let id = first["ack"]["job"].as_u64().unwrap();
        let agent = request(&c, "job_control", json!({"operation":"take_control"}));
        assert!(c.job_command(&agent).unwrap()["ack"]["rejection"].is_string());
        let human = Command {
            action: "job_control".into(),
            args: json!({"operation":"take_control"}),
            source: "desktop".into(),
        };
        assert!(c.job_command(&human).unwrap()["ack"]["rejection"].is_null());
        assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Paused);
        let resume = request(&c, "job_control", json!({"operation":"resume","job":id}));
        assert!(c.job_command(&resume).unwrap()["ack"]["rejection"].is_string());
        c.tick();
        assert_eq!(c.jobs.snapshot().jobs[&id].next_step, 0);
    }

    #[test]
    fn successful_connection_starts_one_bounded_report_without_another_command() {
        for source in ["desktop", "agent"] {
            let (sent, received) = mpsc::channel();
            let engine = EngineHandle::test_worker(EngineView::default(), move |command, view| {
                sent.send(command.clone()).unwrap();
                if command.action == "attach_process" {
                    *view = initial_view();
                }
                json!({"ok":true,"state":view})
            });
            let mut c = controller(engine);
            let (reply, response) = mpsc::channel();
            c.command((
                Command {
                    action: "attach_process".into(),
                    args: json!({}),
                    source: source.into(),
                },
                Some(reply),
            ));
            assert_eq!(
                received
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap()
                    .action,
                "attach_process"
            );
            for _ in 0..100 {
                c.finish_manual();
                if c.manual.is_none() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            let result = response.recv_timeout(Duration::from_secs(1)).unwrap();
            assert_eq!(result["ok"], true);
            let id = result["data"]["automatic_observation"]["ack"]["job"]
                .as_u64()
                .unwrap();
            let job = &c.jobs.snapshot().jobs[&id];
            assert_eq!(job.steps.len(), 16);
            assert_eq!(
                job.creator.role,
                if source == "desktop" {
                    job::Role::Human
                } else {
                    job::Role::Agent
                }
            );
            let first: CaptureStep = serde_json::from_str(&job.steps[0].payload).unwrap();
            assert!(first.compare);
            assert_eq!(first.delay_ms, 0);
            c.finish_manual();
            assert_eq!(c.jobs.snapshot().jobs.len(), 1);
            for _ in 0..3 {
                c.tick();
            }
            let capture = received.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_eq!(capture.action, "capture");
            assert_eq!(capture.args["compare"], true);
            assert_eq!(
                capture.args["scope"]["investigation_id"],
                "fixture-investigation"
            );
        }
    }

    #[test]
    fn failed_or_manual_connection_and_rejected_options_do_not_schedule_captures() {
        for (succeeds, args) in [(false, json!({})), (true, json!({"auto_observe":false}))] {
            let engine = EngineHandle::test_worker(EngineView::default(), move |_, view| {
                if succeeds {
                    *view = initial_view();
                }
                json!({"ok":succeeds,"state":view})
            });
            let mut c = controller(engine);
            c.command((
                Command {
                    action: "attach_process".into(),
                    args,
                    source: "desktop".into(),
                },
                None,
            ));
            for _ in 0..100 {
                c.finish_manual();
                if c.manual.is_none() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(c.manual.is_none());
            assert!(c.jobs.snapshot().jobs.is_empty());
        }
        let engine = EngineHandle::test_worker(EngineView::default(), |_, _| {
            panic!("Rejected before attachment")
        });
        let mut c = controller(engine);
        let (reply, response) = mpsc::channel();
        c.command((
            Command {
                action: "attach_process".into(),
                args: json!({"auto_observe":"false"}),
                source: "desktop".into(),
            },
            Some(reply),
        ));
        assert_eq!(
            response.recv_timeout(Duration::from_secs(1)).unwrap()["ok"],
            false
        );
        assert!(c.jobs.snapshot().jobs.is_empty());
    }
}
