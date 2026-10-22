//! Responsive job control around the sole engine-owning worker.
mod change_watch;
#[cfg(test)]
mod freeze_tests;
#[cfg(test)]
mod runtime_watch_tests;
#[cfg(test)]
mod watch_tests;
use crate::service::{EngineHandle, OperationOwner};
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
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

pub fn panel(ui: &mut eframe::egui::Ui, handle: &Handle, state: &job::View) {
    use crate::theme::{ACCENT, ERROR, MUTED, SURFACE};
    use eframe::egui::{self, RichText};
    let error = handle.job_error.read().unwrap().clone();
    let analysis = handle.view.read().unwrap().analysis.clone();
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
        && state.fence.is_none()
        && analysis.is_none();
    if only_history {
        return;
    }
    egui::Frame::new().fill(SURFACE).inner_margin(12).corner_radius(8).show(ui,|ui| {
        for job in state.history.jobs.values().filter(|j|!j.phase.terminal()) {
            let phase=match job.phase {
                job::Phase::Queued=>"Queued",job::Phase::Running=>"Observing",job::Phase::Pausing=>"Finishing current work, then pausing",
                job::Phase::Paused=>"Paused",job::Phase::Cancelling=>"Finishing current work, then stopping",_=>"Historical",
            };
            let plan = job.steps.first().and_then(|s|serde_json::from_str::<CaptureStep>(&s.payload).ok());
            let watching = plan.as_ref().is_some_and(|s| s.change.is_some());
            let title = if plan.as_ref().is_some_and(|s|s.runtime.is_some()) { "Reading internal values" } else if watching { "Watching effect" } else if plan.as_ref().is_some_and(|s| !s.mode.is_full()) { "Watching movement" } else { "Finding controls" };
            let model_step = job.steps.get(job.next_step).and_then(|s|serde_json::from_str::<CaptureStep>(&s.payload).ok()).is_some_and(|s|s.has_model());
            ui.horizontal_wrapped(|ui| {
                if job.phase==job::Phase::Running { ui.spinner(); }
                ui.label(RichText::new(if job.phase==job::Phase::Running {format!("{title}…")} else {phase.into()}).color(ACCENT))
                    .on_hover_text(format!("{} / {} moments. Keys sampled while the selected window is active.{}",job.next_step,job.steps.len(),if model_step {" Reading images locally."} else {""}));
                if job.phase==job::Phase::Paused {
                    if ui.button("Resume").clicked() {handle.submit("job_control",json!({"operation":"resume","job":job.id}));}
                }else if matches!(job.phase,job::Phase::Running|job::Phase::Queued) && ui.button("Pause").on_hover_text("Finish any active capture or model request, then pause.").clicked() {
                    handle.submit("job_control",json!({"operation":"pause","job":job.id}));
                }
                if ui.button("Stop").on_hover_text("Finish any active request; prevent later captures and model calls.").clicked() {handle.submit("job_control",json!({"operation":"cancel","job":job.id}));}
                if state.history.human_hold.is_none() && job.controller.role==job::Role::Agent
                    && ui.button("Take control").clicked() {handle.submit("job_control",json!({"operation":"take_control"}));}
            });
            if watching { ui.label(RichText::new("Keep playing to check for a delayed response.").size(12.).color(MUTED)); }
        }
        if let Some(analysis)=&analysis {
            let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
            ui.label(if analysis.stopping {"Finishing the active image request; further model calls and findings are stopped."} else {"Reading captured images locally."});
            ui.label(RichText::new(crate::ui_live::observation_age(analysis.observed_unix_ms,now)).size(12.).color(MUTED));
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
        && (c.action == "stop_runtime_test"
            || (c.action == "job_control"
                && matches!(
                    c.args["operation"].as_str(),
                    Some("pause" | "cancel" | "take_control")
                )))
}
fn read_only(action: &str) -> bool {
    matches!(
        action,
        "status"
            | "history"
            | "job_events"
            | "read_capture"
            | "read_sequence"
            | "read_play_evidence"
            | "read_change_evidence"
            | "read_runtime_test"
            | "search_runtime_values"
            | "plan_runtime_comparisons"
            | "read_runtime_inventory"
            | "list_processes"
            | "list_memory_processes"
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
        let runtime_directory = path
            .parent()
            .ok_or("Missing session directory")?
            .join("runtime");
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
                runtime_directory,
                jobs,
                executor: CaptureExecutor::new(engine.clone()),
                manual: None,
                engine,
                published,
                errors,
                draining: false,
                next_capture: None,
                change_continuation: None,
                change_child: None,
                freeze_stops: VecDeque::new(),
            };
            loop {
                let Some(inbox) = inbox.upgrade() else { break };
                controller.refresh_binding();
                // Human controls lead other clients without reordering one source.
                for _ in 0..24 {
                    let request = pop(&mut inbox.lock().unwrap());
                    let Some(request) = request else { break };
                    controller.command(request);
                }
                drop(inbox);
                controller.finish_manual();
                controller.tick();
                controller.tick_freezes();
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime: Option<String>,
    scope: Scope,
    delay_ms: u64,
    source: String,
    #[serde(default, skip_serializing_if = "is_false")]
    compare: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    remaining: usize,
    #[serde(
        default,
        skip_serializing_if = "reversebolt_core::live::CaptureMode::is_full"
    )]
    mode: reversebolt_core::live::CaptureMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    local_model: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    background_model: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    change: Option<change_watch::Cursor>,
}
impl CaptureStep {
    fn has_model(&self) -> bool {
        self.local_model.is_some() || self.runtime.as_ref().is_some_and(|r| r.starts_with("n:"))
    }
}

fn is_false(value: &bool) -> bool {
    !value
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

struct Flight {
    id: u64,
    response: mpsc::Receiver<Value>,
    cancel: Arc<AtomicBool>,
    poll_interval: Duration,
    runtime: bool,
}
struct CaptureExecutor {
    engine: EngineHandle,
    flight: Option<Flight>,
    ready: Option<Value>,
    next_poll: Instant,
    owner: Option<OperationOwner>,
    /// Pausing must publish an already-started runtime read so later pinned cursors remain valid.
    defer_runtime_cancel: bool,
}
impl CaptureExecutor {
    fn new(engine: EngineHandle) -> Self {
        Self {
            engine,
            flight: None,
            ready: None,
            next_poll: Instant::now(),
            owner: None,
            defer_runtime_cancel: false,
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
    fn can_poll(&mut self, cancel_requested: bool) -> bool {
        match &self.flight {
            Some(Flight {
                response,
                cancel,
                runtime,
                ..
            }) => {
                if cancel_requested && !(*runtime && self.defer_runtime_cancel) {
                    cancel.store(true, Ordering::SeqCst);
                }
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
        let model_step = step.has_model();
        let command = Command {
            action: if step.runtime.is_some() {
                "inspect_runtime"
            } else if step.change.is_some() {
                "observe_change"
            } else {
                "capture"
            }
            .into(),
            args: if let Some(task) = &step.runtime {
                json!({"scope":step.scope,"task_ref":task})
            } else if let Some(cursor) = &step.change {
                let cursor = reversebolt_core::changes::ObservationCursor {
                    change_id: cursor.change_id.clone(),
                    phase: cursor.phase.clone(),
                    after_observation: cursor.after_observation,
                };
                json!({"scope":step.scope,"change_id":cursor.change_id,"cursor":cursor})
            } else {
                json!({"scope":step.scope,"compare":step.compare,"remaining":step.remaining,"mode":step.mode,"local_model":step.local_model,"background_model":step.background_model})
            },
            source: step.source,
        };
        if self
            .owner
            .as_ref()
            .is_none_or(|o| o.job != operation.job || o.cancel.load(Ordering::SeqCst))
        {
            self.owner = Some(OperationOwner {
                job: operation.job,
                cancel: Arc::new(AtomicBool::new(false)),
            });
        }
        let owner = self.owner.as_ref().unwrap().clone();
        let cancel = owner.cancel.clone();
        // The final step can drain the first review and then run its own review.
        let poll_interval = Duration::from_secs(if model_step {
            30
        } else if step.runtime.is_some() {
            5
        } else {
            1
        });
        match self.engine.start_owned_command(command, Some(owner)) {
            Ok(response) => {
                self.flight = Some(Flight {
                    id: operation.id,
                    response,
                    cancel,
                    poll_interval,
                    runtime: step.runtime.is_some() && !model_step,
                });
                self.next_poll = Instant::now() + poll_interval;
                job::Poll::Pending
            }
            Err(error) => Self::outcome(false, &error, Value::Null),
        }
    }
    fn poll(&mut self, id: u64, cancel_requested: bool) -> job::Poll {
        if let Some(flight) = &self.flight {
            if cancel_requested && !(flight.runtime && self.defer_runtime_cancel) {
                flight.cancel.store(true, Ordering::SeqCst);
            }
            self.next_poll = Instant::now() + flight.poll_interval;
        }
        // A timed-out window worker still owns its output path. Do not declare
        // quiescence or start the next operation until that worker has settled.
        if crate::native::renderer_busy() {
            return job::Poll::Pending;
        }
        match self.flight.take() {
            Some(Flight {
                id: owned,
                response,
                cancel,
                poll_interval,
                runtime,
            }) if id == owned => {
                let value = self.ready.take().or_else(|| response.try_recv().ok());
                if let Some(value) = value {
                    let live = &value["state"]["live"];
                    Self::outcome(
                        value["ok"] == true,
                        value["error"].as_str().unwrap_or(
                            "Observation finished; see the shared investigation for evidence.",
                        ),
                        if value["data"]["runtime"].is_object() {
                            value["data"].clone()
                        } else if value["data"]["observations"].is_array() {
                            json!({"kind":"change_observation","change_id":value["data"]["id"],"phase":value["data"]["phase"],
                                "observation":value["data"]["observations"].as_array().and_then(|o|o.len().checked_sub(1)),
                                "image_sha256":value["data"]["observations"].as_array().and_then(|o|o.last()).map(|o|&o["image_sha256"])})
                        } else {
                            json!({"captured":if value["data"]["captured"]==false {Some(false)} else if value["ok"]==true {Some(true)} else {None},"investigation_id":live["investigation_id"],
                            "batch":live["coverage"]["batch"],"capture":live["sample_count"],
                            "frame":live["frames"].as_array().and_then(|f| f.last()).map(|f|&f["id"]),
                            "local_observation":value["data"]["local_observation"]})
                        },
                    )
                } else {
                    self.flight = Some(Flight {
                        id,
                        response,
                        cancel,
                        poll_interval,
                        runtime,
                    });
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
struct ChangeContinuation {
    job: u64,
    scope: Scope,
    source: String,
}
struct Controller {
    runtime_directory: PathBuf,
    jobs: job::Jobs<FileJournal>,
    executor: CaptureExecutor,
    engine: EngineHandle,
    manual: Option<Manual>,
    published: Arc<RwLock<job::View>>,
    errors: Arc<RwLock<Option<String>>>,
    draining: bool,
    next_capture: Option<(u64, usize, Instant)>,
    change_continuation: Option<ChangeContinuation>,
    change_child: Option<(u64, u64)>,
    freeze_stops: VecDeque<Request>,
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
        if self.manual.is_none()
            && self.executor.flight.is_none()
            && let Some(request) = self.freeze_stops.pop_front()
        {
            self.command(request);
        }
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
                        Ok(receipt) => {
                            if let Some(parent) = self.change_continuation.as_ref()
                                && Scope::from_view(&self.engine.view.read().unwrap())
                                    .is_some_and(|s| s.key() != parent.scope.key())
                                && let Some(child) = receipt["ack"]["job"].as_u64()
                            {
                                self.change_child = Some((parent.job, child));
                            }
                            value["data"] = json!({"automatic_observation":receipt});
                        }
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
        let state = self.engine.view.read().unwrap();
        let runtime = state
            .live
            .as_ref()
            .is_some_and(|l| l.runtime.available && l.runtime.phase == "available");
        let changing = state.live.as_ref().is_some_and(|live| {
            live.change_searches.last().is_some_and(|s| {
                s.status == "collecting" && s.selection_batch == live.coverage.batch
            })
        });
        let search_scope = changing.then(|| Scope::from_view(&state)).flatten();
        let recheck = state.live.as_ref().is_some_and(|live| {
            live.sample_count == 0
                && live
                    .prediction_checks
                    .iter()
                    .any(|c| c.rechecked_from.is_some())
        });
        let model = if recheck || changing {
            None
        } else {
            state.observer_model.clone()
        };
        drop(state);
        let client = if source == "desktop" {
            "desktop"
        } else {
            "mcp-agent"
        };
        let command = Command {
            action: if runtime {
                "start_runtime_job"
            } else {
                "start_observation_job"
            }
            .into(),
            source: source.clone(),
            args: json!({"count":if runtime {1} else if recheck || changing {4} else {16},"interval_ms":1000,"compare":!recheck && !changing,
                "local_model":model,"task":{"operation":"discover"},"observe_play":runtime,
                "request_id":snapshot.clients.get(client).map_or(1,|c|c.high_water+1),
                "boot":snapshot.boot,"binding":snapshot.current}),
        };
        let receipt = self.job_command(&command)?;
        if let Some(error) = receipt["ack"]["rejection"].as_str() {
            return Err(error.into());
        }
        if let Some(scope) = search_scope {
            self.change_continuation = Some(ChangeContinuation {
                job: receipt["ack"]["job"]
                    .as_u64()
                    .ok_or("Missing change-search job receipt")?,
                scope,
                source,
            });
        }
        Ok(receipt)
    }
    fn continue_change_search(&mut self) {
        let Some(pending) = self.change_continuation.take() else {
            return;
        };
        let snapshot = self.jobs.snapshot();
        if snapshot.human_hold.is_some()
            || self.draining
            || self.jobs.view().fence.is_some()
            || snapshot
                .current
                .as_ref()
                .is_none_or(|b| b.target != pending.scope.key())
        {
            return;
        }
        let Some(job) = snapshot.jobs.get(&pending.job) else {
            return;
        };
        if !job.phase.terminal() {
            self.change_continuation = Some(pending);
            return;
        }
        if job.phase != job::Phase::Completed || job.invalidated {
            return;
        }
        if self.executor.flight.is_some() || self.engine.view.read().unwrap().analysis.is_some() {
            self.change_continuation = Some(pending);
            return;
        }
        let source = if job.controller.role == job::Role::Human {
            "desktop".into()
        } else {
            pending.source.clone()
        };
        self.command((Command{action:"finish_change_search".into(),source,
            args:json!({"investigation_id":pending.scope.investigation_id,"batch":pending.scope.batch})},None));
        if self.manual.is_some() {
            self.change_continuation = Some(pending);
        }
    }
    fn command(&mut self, (mut command, reply): Request) {
        let stopping = (command.action == "set_runtime_freeze" && command.args["enabled"] == false)
            || command.action == "stop_runtime_test";
        if command.source == "desktop"
            && command.action == "stop_runtime_test"
            && self
                .engine
                .view
                .read()
                .unwrap()
                .runtime_tests
                .iter()
                .any(|t| t["active"] == true && t["id"] == command.args["test_id"])
        {
            self.engine.test_abort.store(true, Ordering::SeqCst);
        }
        if stopping
            && (self.manual.is_some() || self.executor.flight.is_some())
            && self.freeze_stops.len() < 16
        {
            self.freeze_stops.push_back((command, reply));
            return;
        }

        // The selection worker is read-only and may already be settling after
        // its parent job completed. A human stop/pause takes control so that
        // its future capture job cannot start after the click is acknowledged.
        if command.source == "desktop"
            && command.action == "job_control"
            && matches!(command.args["operation"].as_str(), Some("cancel" | "pause"))
            && self.manual.is_some()
            && self
                .change_continuation
                .as_ref()
                .is_some_and(|p| command.args["job"].as_u64() == Some(p.job))
        {
            command.args["operation"] = json!("take_control");
        }
        // A click on the just-completed parent's Stop control stops its fresh
        // follow-up too. Never rewrite an agent's binding or authority envelope.
        if command.source == "desktop"
            && command.action == "job_control"
            && matches!(command.args["operation"].as_str(), Some("cancel" | "pause"))
            && let Some((parent, child)) = self.change_child
            && command.args["job"].as_u64() == Some(parent)
            && self
                .jobs
                .snapshot()
                .jobs
                .get(&child)
                .is_some_and(|j| !j.phase.terminal())
        {
            command.args["job"] = json!(child);
        }
        let result: Result<Value> = (|| {
            if command.action.starts_with('_') {
                return Err("Internal worker action is not a public command.".into());
            }
            if matches!(command.action.as_str(), "status" | "history") {
                return Ok(Value::Null);
            }
            if command.action == "job_events" {
                return self.jobs.events_after(command.args["cursor"].as_u64().unwrap_or(0),64)
                    .map(|p|json!(p)).map_err(|e|format!("Event cursor unavailable: {e:?}. Refresh status and use its event_head."));
            }
            if matches!(
                command.action.as_str(),
                "start_observation_job"
                    | "start_change_watch"
                    | "start_runtime_job"
                    | "job_control"
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
                && !stopping
                && !read_only(&command.action)
                && !matches!(
                    command.action.as_str(),
                    "submit_observation" | "submit_temporal_observation"
                )
            {
                return Err("An observation job owns the investigation. Pause or cancel it, or take control from the desktop.".into());
            }
            if self.engine.view.read().unwrap().analysis.is_some()
                && !stopping
                && !read_only(&command.action)
                && !matches!(
                    command.action.as_str(),
                    "submit_observation" | "submit_temporal_observation"
                )
            {
                return Err("An earlier image review is still settling. Status and job controls remain available.".into());
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
                "attach_process"
                | "recheck_findings"
                | "search_number"
                | "start_change_search"
                | "finish_change_search" => command.args["auto_observe"].as_bool().unwrap_or(true),
                "advance_memory" => command.args["auto_observe"] == true,
                _ => false,
            };
            // Clear a previous test's stop only at admission, on this controller
            // thread. A takeover during preparation must survive the helper.
            if command.action == "prepare_runtime_test"
                && !self
                    .engine
                    .view
                    .read()
                    .unwrap()
                    .runtime_tests
                    .iter()
                    .any(|t| t["active"] == true)
            {
                self.engine.test_abort.store(false, Ordering::SeqCst);
            }
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
            "status"
                | "history"
                | "job_events"
                | "start_observation_job"
                | "start_runtime_job"
                | "start_change_watch"
                | "job_control"
        );
        match result {
            Ok(data) => {
                if is_job {
                    let rejection = data["ack"]["rejection"].as_str().map(str::to_owned);
                    if rejection.is_none()
                        && command.action == "job_control"
                        && (command.args["operation"] == "take_control"
                            || (command.args["operation"] == "cancel"
                                && self
                                    .change_continuation
                                    .as_ref()
                                    .is_some_and(|p| command.args["job"].as_u64() == Some(p.job))))
                    {
                        self.change_continuation = None;
                        if let Some(manual) = &mut self.manual {
                            manual.automatic = None;
                        }
                    }

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
    fn tick_freezes(&mut self) {
        if self.manual.is_some()
            || self.executor.flight.is_some()
            || !self.freeze_stops.is_empty()
            || self.draining
            || self.jobs.view().fence.is_some()
            || self
                .jobs
                .snapshot()
                .jobs
                .values()
                .any(|j| !j.phase.terminal())
        {
            return;
        }
        let view = self.engine.view.read().unwrap();
        if view.analysis.is_some() {
            return;
        }
        let now = reversebolt_core::changes::now_ms();
        let next = view
            .freezes
            .iter()
            .filter(|f| {
                f.phase == "active"
                    && (f.actor == "desktop" || self.jobs.snapshot().human_hold.is_none())
                    && now.saturating_sub(f.checked_ms) >= 1000
            })
            .min_by_key(|f| f.checked_ms)
            .map(|f| (f.id.clone(), f.actor.clone()));
        drop(view);
        if let Some((id, source)) = next {
            match self.engine.start_command(Command {
                action: "_tick_runtime_freeze".into(),
                source,
                args: json!({"freeze_id":id}),
            }) {
                Ok(response) => {
                    self.manual = Some(Manual {
                        response,
                        reply: None,
                        automatic: None,
                    })
                }
                Err(error) => self.error(error),
            }
        }
    }
    fn stop_revoked_analysis(&self) {
        if let Some(owner) = &self.executor.owner {
            let snapshot = self.jobs.snapshot();
            let runtime_read = self.executor.flight.as_ref().is_some_and(|f| f.runtime);
            if self.draining
                || self.jobs.view().fence.is_some()
                || snapshot.jobs.get(&owner.job).is_none_or(|j| {
                    j.invalidated
                        || (j.phase == job::Phase::Pausing && !runtime_read)
                        || matches!(
                            j.phase,
                            job::Phase::Paused
                                | job::Phase::Completed
                                | job::Phase::Cancelling
                                | job::Phase::Cancelled
                                | job::Phase::Failed
                        )
                })
            {
                owner.cancel.store(true, Ordering::SeqCst);
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
        if command.action == "job_control"
            && command.args["operation"] == "resume"
            && self.engine.view.read().unwrap().analysis.is_some()
        {
            return Err(
                "Wait for the paused image review to finish before resuming capture.".into(),
            );
        }
        let action = if command.action == "start_change_watch" {
            self.plan_change_watch(command, &binding, request, &actor.client)?
        } else if command.action == "start_runtime_job" {
            let local_model = command
                .args
                .get("local_model")
                .filter(|v| !v.is_null())
                .map(|v| {
                    v.as_str()
                        .filter(|s| {
                            !s.is_empty()
                                && s.len() <= 128
                                && s.chars()
                                    .all(|c| c.is_ascii_alphanumeric() || "-_.:/".contains(c))
                        })
                        .map(str::to_owned)
                        .ok_or("Supply an installed local model name or null.")
                })
                .transpose()?;
            let task: reversebolt_core::runtime::Task =
                serde_json::from_value(command.args["task"].clone())
                    .map_err(|_| "Supply a runtime discovery or refresh task.")?;
            task.validate()?;
            if local_model.is_some() && !matches!(task, reversebolt_core::runtime::Task::Discover) {
                return Err(
                    "Metadata hypotheses require a discovery report, not repeated value refresh."
                        .into(),
                );
            }
            let count = command.args.get("count").map_or(Ok(1), |v| {
                v.as_u64()
                    .filter(|n| (1..=16).contains(n))
                    .ok_or("Use one to sixteen internal-value checks.")
            })? as usize;
            let observe_play = command
                .args
                .get("observe_play")
                .map(|v| v.as_bool().ok_or("observe_play must be true or false."))
                .transpose()?
                .unwrap_or(false);
            if observe_play
                && (!matches!(task, reversebolt_core::runtime::Task::Discover) || count != 1)
            {
                return Err("Play observation can precede one discovery pass only.".into());
            }
            let tasks = task.sequence(count)?;
            let interval = command.args.get("interval_ms").map_or(Ok(1000), |v| {
                v.as_u64()
                    .filter(|n| (250..=60000).contains(n))
                    .ok_or("Use an interval from 250 to 60,000 ms.")
            })?;
            let parts: Vec<_> = binding.target.split('/').collect();
            if parts.len() != 4 {
                return Err("Invalid runtime job binding.".into());
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
                if snapshot.jobs.values().any(|j| !j.phase.terminal()) {
                    return Err(
                        "Finish or stop the current job before inspecting internal values.".into(),
                    );
                }
                let state = self.engine.view.read().unwrap();
                let live = state.live.as_ref().ok_or("Connect an application first.")?;
                if !live.runtime.available {
                    return Err("No supported runtime is available.".into());
                }
                live.runtime.paths(&task)?;
                if observe_play && live.sample_count + 15 > 20 {
                    return Err(
                        "The initial play observation needs fifteen available capture slots."
                            .into(),
                    );
                }
            }
            let mut steps = vec![];
            if observe_play {
                for index in 0..15 {
                    steps.push(job::Step {
                        kind: job::Kind::Observe,
                        payload: serde_json::to_string(&CaptureStep {
                            runtime: None,
                            scope: scope.clone(),
                            delay_ms: if index == 0 { 0 } else { 500 },
                            source: command.source.clone(),
                            compare: false,
                            remaining: 0,
                            mode: reversebolt_core::live::CaptureMode::Play,
                            // One early appearance pass leaves later captures
                            // available for correspondence, without pausing play.
                            local_model: (index == 1).then(|| local_model.clone()).flatten(),
                            background_model: index == 1 && local_model.is_some(),
                            change: None,
                        })
                        .map_err(|e| e.to_string())?,
                    });
                }
            }
            steps.extend(
                tasks
                    .iter()
                    .enumerate()
                    .map(|(index, next_task)| {
                        Ok(job::Step {
                            kind: job::Kind::Observe,
                            payload: serde_json::to_string(&CaptureStep {
                                runtime: Some(crate::runtime_bridge::store_plan(
                                    &self.runtime_directory,
                                    next_task,
                                    local_model.as_deref(),
                                )?),
                                scope: scope.clone(),
                                delay_ms: if index == 0 { 0 } else { interval },
                                source: command.source.clone(),
                                compare: false,
                                remaining: 0,
                                mode: reversebolt_core::live::CaptureMode::Full,
                                local_model: None,
                                background_model: false,
                                change: None,
                            })
                            .map_err(|e| e.to_string())?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            );
            job::Action::Submit {
                steps,
                consent: false,
            }
        } else if command.action == "start_observation_job" {
            let mode: reversebolt_core::live::CaptureMode = command
                .args
                .get("mode")
                .map(|value| serde_json::from_value(value.clone()))
                .transpose()
                .map_err(|_| "Choose capture mode full, visual or play.")?
                .unwrap_or_default();
            let count = command.args["count"].as_u64().unwrap_or(12) as usize;
            let local_model = command
                .args
                .get("local_model")
                .filter(|v| !v.is_null())
                .map(|v| {
                    v.as_str()
                        .filter(|s| {
                            !s.is_empty()
                                && s.len() <= 128
                                && s.chars()
                                    .all(|c| c.is_ascii_alphanumeric() || "-_.:/".contains(c))
                        })
                        .map(str::to_owned)
                        .ok_or("Supply an installed local model name or null.")
                })
                .transpose()?;
            if local_model.is_some() && count < 2 {
                return Err("Local visual observation needs at least two captures.".into());
            }
            let interval = command.args["interval_ms"].as_u64().unwrap_or(1000);
            let minimum = if mode.is_full() { 250 } else { 50 };
            if !(1..=16).contains(&count) || !(minimum..=60000).contains(&interval) {
                return Err("Use 1–16 captures. Minimum interval is 250 ms for full mode or 50 ms for visual mode; maximum is 60,000 ms.".into());
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
                if snapshot.jobs.values().any(|j| {
                    !j.phase.terminal()
                        && j.steps.iter().any(|s| {
                            serde_json::from_str::<CaptureStep>(&s.payload)
                                .is_ok_and(|s| s.change.is_some())
                        })
                }) {
                    return Err(
                        "Finish or stop the trial watch before starting discovery observations."
                            .into(),
                    );
                }
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
                let reserved_memory: usize = snapshot
                    .jobs
                    .values()
                    .filter(|j| !j.phase.terminal() && j.binding == binding)
                    .flat_map(|j| j.steps.iter().skip(j.next_step))
                    .filter(|s| {
                        serde_json::from_str::<CaptureStep>(&s.payload).is_ok_and(|s| {
                            s.runtime.is_none()
                                && s.mode != reversebolt_core::live::CaptureMode::Play
                        })
                    })
                    .count();
                if live.sample_count + count + reserved > 20
                    || live.retained_bytes
                        + (reserved_memory
                            + if mode == reversebolt_core::live::CaptureMode::Play {
                                0
                            } else {
                                count
                            })
                            * live.planned_bytes
                        > 512 * 1024 * 1024
                {
                    return Err("The requested captures exceed this batch's remaining retention budget, including queued jobs.".into());
                }
            }
            let mut steps = vec![];
            let compare = command.args["compare"].as_bool().unwrap_or(false);
            if compare && !mode.is_full() {
                return Err("Visual sampling skips text recognition; compare separately when enough evidence exists.".into());
            }
            for index in 0..count {
                steps.push(job::Step {
                    kind: job::Kind::Observe,
                    payload: serde_json::to_string(&CaptureStep {
                        runtime: None,
                        scope: scope.clone(),
                        delay_ms: if (compare || !mode.is_full()) && index == 0 {
                            0
                        } else {
                            interval
                        },
                        source: command.source.clone(),
                        compare,
                        remaining: if compare { count - index - 1 } else { 0 },
                        mode,
                        background_model: local_model.is_some() && index == 1 && index != count - 1,
                        change: None,
                        local_model: if index == 1 || index == count - 1 {
                            local_model.clone()
                        } else {
                            None
                        },
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
        if self.jobs.snapshot().human_hold.is_some() {
            self.engine.test_abort.store(true, Ordering::SeqCst);
        }
        if reply.ack.rejection.is_none() && command.action != "job_control" {
            self.engine.test_abort.store(true, Ordering::SeqCst);
        }
        self.stop_revoked_analysis();
        *self.errors.write().unwrap() = None;
        Ok(json!(reply))
    }
    fn tick(&mut self) {
        if self.manual.is_some() {
            return;
        }
        self.continue_change_search();
        if self.manual.is_some() {
            return;
        }
        self.stop_revoked_analysis();
        let view = self.jobs.view();
        // A stopped capture sequence may still have one pure image request draining.
        // Never give another job or target ownership while that worker is present.
        if view.in_flight.is_none()
            && let Some(analysis) = self.engine.view.read().unwrap().analysis.as_ref()
            && view
                .history
                .jobs
                .get(&analysis.job)
                .is_none_or(|j| j.phase != job::Phase::Running || j.invalidated)
        {
            return;
        }
        if view.in_flight.is_none() && crate::native::renderer_busy() {
            return;
        }
        self.executor.defer_runtime_cancel = !self.draining
            && view.fence.is_none()
            && view
                .in_flight
                .as_ref()
                .and_then(|f| view.history.jobs.get(&f.operation.job))
                .is_some_and(|j| !j.invalidated && j.phase == job::Phase::Pausing);
        let cancelled = self.draining
            || view.fence.is_some()
            || view
                .in_flight
                .as_ref()
                .and_then(|f| view.history.jobs.get(&f.operation.job))
                .is_some_and(|j| {
                    j.invalidated || matches!(j.phase, job::Phase::Pausing | job::Phase::Cancelling)
                });
        if view.in_flight.is_some() && !self.executor.can_poll(cancelled) {
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
    pub(super) fn initial_view() -> EngineView {
        EngineView {
            live: Some(reversebolt_core::live::LiveView {
                runtime: Default::default(),
                change_searches: vec![],
                value_searches: vec![],
                memory_process: None,
                memory_sources: vec![],
                memory_sources_error: None,
                local_observation_warnings: vec![],
                rule_checks: vec![],
                rule_error: None,
                action_patterns: vec![],
                temporal_tracks: vec![],
                prediction_checks: vec![],
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
    pub(super) fn controller(engine: EngineHandle) -> Controller {
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
            runtime_directory: directory.join("runtime"),
            jobs,
            executor: CaptureExecutor::new(engine.clone()),
            engine,
            manual: None,
            published,
            errors: Arc::new(RwLock::new(None)),
            draining: false,
            next_capture: None,
            change_continuation: None,
            change_child: None,
            freeze_stops: VecDeque::new(),
        };
        c.refresh_binding();
        c
    }
    pub(super) fn request(c: &Controller, action: &str, extra: Value) -> Command {
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

    fn change_view() -> EngineView {
        let mut view = initial_view();
        let frame = reversebolt_core::visual::FrameRef {
            sampling: None,
            input_context: None,
            id: "source".into(),
            number: 1,
            sha256: "source".into(),
            width: 100,
            height: 100,
            epoch: 0,
            started_unix_ms: 0,
            capture_interval: None,
        };
        view.live.as_mut().unwrap().change_searches.push(
            reversebolt_core::change_search::Summary {
                source_batch: 0,
                source_track: 0,
                source_frame: frame,
                label: "Number".into(),
                bounds: [0., 0., 10., 10.],
                selection_batch: 1,
                planned_bytes: 1048576,
                status: "collecting".into(),
                reason: None,
                observations: vec![],
                basis: vec![],
                complete_bytes: 0,
                matched_windows: 0,
                retained_windows: 0,
                omitted_windows: 0,
                boundary_matches: 0,
                elapsed_ms: 0,
                evidence: "fixture".into(),
            },
        );
        view
    }
    fn complete_fake(command: &Command, view: &mut EngineView) -> Value {
        let live = view.live.as_mut().unwrap();
        match command.action.as_str() {
            "capture" => live.sample_count += 1,
            "finish_change_search" => {
                assert_eq!(live.sample_count, 4);
                live.coverage.batch += 1;
                live.sample_count = 0;
                live.planned_bytes = 64;
                live.change_searches[0].status = "completed".into();
            }
            _ => panic!("Unexpected command {}", command.action),
        }
        json!({"ok":true,"error":null,"state":view})
    }
    pub(super) fn drive_change(c: &mut Controller) {
        c.finish_manual();
        if let Some((job, step, _)) = c.next_capture {
            c.next_capture = Some((job, step, Instant::now()));
        }
        c.tick();
        std::thread::sleep(Duration::from_millis(3));
    }
    #[test]
    fn change_search_hands_off_once_from_four_selection_captures_to_sixteen_fresh_ones() {
        let (sent, received) = mpsc::channel();
        let engine = EngineHandle::test_worker(change_view(), move |command, view| {
            sent.send(command.clone()).unwrap();
            complete_fake(&command, view)
        });
        let mut c = controller(engine);
        let receipt = c.start_automatic_observation("agent".into()).unwrap();
        let parent = receipt["ack"]["job"].as_u64().unwrap();
        assert_eq!(c.jobs.snapshot().jobs[&parent].steps.len(), 4);
        let mut commands = vec![];
        for _ in 0..1500 {
            drive_change(&mut c);
            commands.extend(received.try_iter());
            if c.jobs.snapshot().jobs.len() == 2
                && c.jobs
                    .snapshot()
                    .jobs
                    .values()
                    .all(|j| j.phase == job::Phase::Completed)
            {
                break;
            }
        }
        assert_eq!(
            commands.iter().filter(|c| c.action == "capture").count(),
            20
        );
        assert_eq!(
            commands
                .iter()
                .filter(|c| c.action == "finish_change_search")
                .count(),
            1
        );
        assert_eq!(
            c.engine
                .view
                .read()
                .unwrap()
                .live
                .as_ref()
                .unwrap()
                .sample_count,
            16
        );
        assert_eq!(c.change_child.unwrap().0, parent);
        for _ in 0..5 {
            drive_change(&mut c);
        }
        assert!(received.try_recv().is_err());
    }
    #[test]
    fn stop_during_selection_suppresses_followup_and_rejected_takeover_does_not() {
        let (sent, received) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let engine = EngineHandle::test_worker(change_view(), move |command, view| {
            sent.send(command.clone()).unwrap();
            if command.action == "finish_change_search" {
                held.recv().unwrap();
            }
            complete_fake(&command, view)
        });
        let mut c = controller(engine);
        let receipt = c.start_automatic_observation("agent".into()).unwrap();
        let parent = receipt["ack"]["job"].as_u64().unwrap();
        let bad = request(&c, "job_control", json!({"operation":"take_control"}));
        let (tx, rx) = mpsc::channel();
        c.command((bad, Some(tx)));
        assert_eq!(rx.recv().unwrap()["ok"], false);
        assert!(c.change_continuation.is_some());
        let mut selecting = false;
        for _ in 0..600 {
            drive_change(&mut c);
            if received
                .try_iter()
                .any(|v| v.action == "finish_change_search")
            {
                selecting = true;
                break;
            }
        }
        assert!(selecting);
        assert!(c.manual.is_some());
        let (tx, rx) = mpsc::channel();
        c.command((
            Command {
                source: "desktop".into(),
                action: "job_control".into(),
                args: json!({"operation":"cancel","job":parent}),
            },
            Some(tx),
        ));
        assert_eq!(rx.recv().unwrap()["ok"], true);
        assert!(c.manual.as_ref().unwrap().automatic.is_none());
        release.send(()).unwrap();
        for _ in 0..100 {
            drive_change(&mut c);
            if c.manual.is_none() {
                break;
            }
        }
        assert_eq!(c.jobs.snapshot().jobs.len(), 1);
        assert!(c.change_continuation.is_none());
        assert!(received.try_recv().is_err());
    }
    #[test]
    fn a_stale_desktop_stop_on_the_parent_cancels_its_queued_fresh_job() {
        let engine =
            EngineHandle::test_worker(change_view(), |command, view| complete_fake(&command, view));
        let mut c = controller(engine);
        let receipt = c.start_automatic_observation("desktop".into()).unwrap();
        let parent = receipt["ack"]["job"].as_u64().unwrap();
        for _ in 0..600 {
            c.finish_manual();
            if c.change_child.is_some() {
                break;
            }
            if let Some((job, step, _)) = c.next_capture {
                c.next_capture = Some((job, step, Instant::now()));
            }
            c.tick();
            std::thread::sleep(Duration::from_millis(3));
        }
        let child = c.change_child.unwrap().1;
        let (tx, rx) = mpsc::channel();
        c.command((
            Command {
                source: "desktop".into(),
                action: "job_control".into(),
                args: json!({"operation":"cancel","job":parent}),
            },
            Some(tx),
        ));
        assert_eq!(rx.recv().unwrap()["ok"], true);
        assert_eq!(c.jobs.snapshot().jobs[&child].phase, job::Phase::Cancelled);
        assert_eq!(
            c.engine
                .view
                .read()
                .unwrap()
                .live
                .as_ref()
                .unwrap()
                .sample_count,
            0
        );
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
        assert!(c.engine.test_abort.load(Ordering::SeqCst));
        assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Paused);
        let resume = request(&c, "job_control", json!({"operation":"resume","job":id}));
        assert!(c.job_command(&resume).unwrap()["ack"]["rejection"].is_string());
        c.tick();
        assert_eq!(c.jobs.snapshot().jobs[&id].next_step, 0);
    }

    #[test]
    fn runtime_trials_obey_human_hold_and_active_observation_ownership() {
        for action in [
            "set_runtime_value",
            "set_observed_value",
            "prepare_runtime_change",
            "apply_runtime_change",
            "restore_runtime_change",
            "prepare_runtime_test",
            "observe_runtime_test",
            "lock_runtime_test_appearance",
        ] {
            for held in [false, true] {
                let engine = EngineHandle::test_worker(initial_view(), |_, _| {
                    panic!("Runtime trial escaped ownership gate")
                });
                let mut c = controller(engine);
                let start = request(
                    &c,
                    "start_observation_job",
                    json!({"count":2,"interval_ms":1000}),
                );
                c.job_command(&start).unwrap();
                if held {
                    c.job_command(&Command {
                        action: "job_control".into(),
                        args: json!({"operation":"take_control"}),
                        source: "desktop".into(),
                    })
                    .unwrap();
                }
                let command = request(&c, action, json!({"change_id":"test"}));
                let (tx, rx) = mpsc::channel();
                c.command((command, Some(tx)));
                let response = rx.recv_timeout(Duration::from_secs(2)).unwrap();
                assert_eq!(response["ok"], false);
                assert!(response["error"].as_str().unwrap().contains(if held {
                    "desktop has control"
                } else {
                    "observation job owns"
                }));
                assert!(c.manual.is_none());
            }
        }
    }

    #[test]
    fn a_stale_desktop_stop_does_not_interrupt_a_different_experiment() {
        let mut view = initial_view();
        view.runtime_tests = vec![json!({"id":"current","active":true})];
        let engine = EngineHandle::test_worker(view, |_, _| json!({"ok":true}));
        let mut c = controller(engine);
        c.command((
            Command {
                source: "desktop".into(),
                action: "stop_runtime_test".into(),
                args: json!({"test_id":"earlier"}),
            },
            None,
        ));
        assert!(!c.engine.test_abort.load(Ordering::SeqCst));
        c.command((
            Command {
                source: "desktop".into(),
                action: "stop_runtime_test".into(),
                args: json!({"test_id":"current"}),
            },
            None,
        ));
        assert!(c.engine.test_abort.load(Ordering::SeqCst));
    }

    #[test]
    fn takeover_during_test_preparation_survives_the_pending_command() {
        let (release, held) = mpsc::channel();
        let (sent, received) = mpsc::channel();
        let engine = EngineHandle::test_worker(initial_view(), move |command, view| {
            sent.send(command.action).unwrap();
            held.recv().unwrap();
            json!({"ok":true,"state":view})
        });
        let mut c = controller(engine);
        c.engine.test_abort.store(true, Ordering::SeqCst);
        let prepare = request(&c, "prepare_runtime_test", json!({"test_id":"new"}));
        c.command((prepare.clone(), None));
        assert_eq!(
            received.recv_timeout(Duration::from_secs(2)).unwrap(),
            "prepare_runtime_test"
        );
        assert!(!c.engine.test_abort.load(Ordering::SeqCst));
        let human = Command {
            source: "desktop".into(),
            action: "job_control".into(),
            args: json!({"operation":"take_control"}),
        };
        assert!(c.job_command(&human).unwrap()["ack"]["rejection"].is_null());
        c.command((prepare, None));
        assert!(c.engine.test_abort.load(Ordering::SeqCst));
        release.send(()).unwrap();
        let end = Instant::now() + Duration::from_secs(2);
        while c.manual.is_some() && Instant::now() < end {
            c.finish_manual();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(c.manual.is_none());
        assert!(c.engine.test_abort.load(Ordering::SeqCst));
        assert!(received.try_recv().is_err());
    }

    #[test]
    fn runtime_trial_cannot_overlap_a_manual_engine_operation() {
        let (release, held) = mpsc::channel();
        let (sent, received) = mpsc::channel();
        let engine = EngineHandle::test_worker(initial_view(), move |command, view| {
            sent.send(command.action.clone()).unwrap();
            held.recv().unwrap();
            json!({"ok":true,"state":view})
        });
        let mut c = controller(engine);
        let first = request(&c, "apply_runtime_change", json!({"change_id":"one"}));
        c.command((first, None));
        assert_eq!(
            received.recv_timeout(Duration::from_secs(2)).unwrap(),
            "apply_runtime_change"
        );
        for action in [
            "set_runtime_value",
            "set_observed_value",
            "prepare_runtime_change",
            "apply_runtime_change",
            "restore_runtime_change",
        ] {
            let (tx, rx) = mpsc::channel();
            let next = request(&c, action, json!({"change_id":"two"}));
            c.command((next, Some(tx)));
            let response = rx.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_eq!(response["ok"], false);
            assert!(
                response["error"]
                    .as_str()
                    .unwrap()
                    .contains("still in progress")
            );
        }
        assert!(received.try_recv().is_err());
        release.send(()).unwrap();
        for _ in 0..200 {
            c.finish_manual();
            if c.manual.is_none() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(c.manual.is_none());
    }

    #[test]
    fn completed_observation_revokes_its_passive_input_owner() {
        let engine =
            EngineHandle::test_worker(initial_view(), |_, view| json!({"ok":true,"state":view}));
        let mut c = controller(engine);
        let command = request(
            &c,
            "start_observation_job",
            json!({"count":1,"interval_ms":250}),
        );
        let reply = c.job_command(&command).unwrap();
        let id = reply["ack"]["job"].as_u64().unwrap();
        for _ in 0..200 {
            c.tick();
            if c.jobs.snapshot().jobs[&id].phase == job::Phase::Completed {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Completed);
        c.tick();
        assert!(
            c.executor
                .owner
                .as_ref()
                .unwrap()
                .cancel
                .load(Ordering::SeqCst)
        );
    }

    #[test]
    fn connection_and_value_search_start_one_bounded_observation_job() {
        for (source, action) in [
            ("desktop", "attach_process"),
            ("agent", "attach_process"),
            ("desktop", "search_number"),
            ("agent", "search_number"),
        ] {
            let (sent, received) = mpsc::channel();
            let engine = EngineHandle::test_worker(EngineView::default(), move |command, view| {
                sent.send(command.clone()).unwrap();
                if matches!(command.action.as_str(), "attach_process" | "search_number") {
                    *view = initial_view();
                }
                json!({"ok":true,"state":view})
            });
            let mut c = controller(engine);
            let (reply, response) = mpsc::channel();
            c.command((
                Command {
                    action: action.into(),
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
                action
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

    #[test]
    fn supported_runtime_automatic_discovery_watches_play_before_one_managed_step() {
        let mut view = initial_view();
        view.live.as_mut().unwrap().runtime.available = true;
        view.live.as_mut().unwrap().runtime.phase = "available".into();
        let engine = EngineHandle::test_worker(view, |_, _| {
            panic!("No operation dispatched in this plan check")
        });
        let mut c = controller(engine);
        let receipt = c.start_automatic_observation("agent".into()).unwrap();
        let id = receipt["ack"]["job"].as_u64().unwrap();
        let snapshot = c.jobs.snapshot();
        let plan = &snapshot.jobs[&id];
        assert_eq!(plan.steps.len(), 16);
        for raw in &plan.steps[..15] {
            let step: CaptureStep = serde_json::from_str(&raw.payload).unwrap();
            assert_eq!(step.mode, reversebolt_core::live::CaptureMode::Play);
            assert!(step.runtime.is_none() && step.local_model.is_none());
        }
        let step: CaptureStep = serde_json::from_str(&plan.steps[15].payload).unwrap();
        assert!(matches!(
            crate::runtime_bridge::load_task(
                &c.runtime_directory,
                step.runtime.as_deref().unwrap()
            )
            .unwrap(),
            reversebolt_core::runtime::Task::Discover
        ));
        assert!(step.local_model.is_none());
        assert_eq!(step.delay_ms, 0);
    }

    #[test]
    fn configured_model_is_bound_to_automatic_runtime_discovery() {
        let mut view = initial_view();
        view.observer_model = Some("local-model".into());
        view.live.as_mut().unwrap().runtime.available = true;
        view.live.as_mut().unwrap().runtime.phase = "available".into();
        let engine = EngineHandle::test_worker(view, |_, _| panic!("No dispatch in plan test"));
        let mut c = controller(engine);
        let response = c.start_automatic_observation("agent".into()).unwrap();
        let id = response["ack"]["job"].as_u64().unwrap();
        let snapshot = c.jobs.snapshot();
        let steps = &snapshot.jobs[&id].steps;
        assert_eq!(steps.len(), 16);
        for (index, raw) in steps[..15].iter().enumerate() {
            let step: CaptureStep = serde_json::from_str(&raw.payload).unwrap();
            assert_eq!(step.mode, reversebolt_core::live::CaptureMode::Play);
            assert_eq!(
                step.local_model.as_deref(),
                (index == 1).then_some("local-model")
            );
            assert_eq!(step.background_model, index == 1);
            assert!(step.runtime.is_none());
        }
        let step: CaptureStep =
            serde_json::from_str(&snapshot.jobs[&id].steps.last().unwrap().payload).unwrap();
        assert!(step.local_model.is_none());
        assert_eq!(
            crate::runtime_bridge::load_plan(
                &c.runtime_directory,
                step.runtime.as_deref().unwrap()
            )
            .unwrap()
            .1
            .as_deref(),
            Some("local-model")
        );
        assert!(step.runtime.is_some());
        let refresh = request(
            &c,
            "start_runtime_job",
            json!({"task":{"operation":"refresh","catalog_id":"test","after_revision":0,"ids":["a".repeat(64)]},"local_model":"local-model"}),
        );
        assert!(
            c.job_command(&refresh)
                .unwrap_err()
                .contains("discovery report")
        );
    }

    #[test]
    fn four_value_runtime_refresh_fits_real_journal_and_checks_its_document() {
        use crate::runtime_bridge::{load_task, store_task};
        let engine = EngineHandle::test_worker(initial_view(), |_, _| panic!("No worker needed"));
        let c = controller(engine);
        let task = reversebolt_core::runtime::Task::Refresh {
            catalog_id: "c".repeat(64),
            after_revision: u64::MAX,
            ids: (0..4).map(|i| format!("{i:064x}")).collect(),
        };
        let reference = store_task(&c.runtime_directory, &task).unwrap();
        assert_eq!(store_task(&c.runtime_directory, &task).unwrap(), reference);
        assert_eq!(
            serde_json::to_value(load_task(&c.runtime_directory, &reference).unwrap()).unwrap(),
            serde_json::to_value(&task).unwrap()
        );
        let step = CaptureStep {
            runtime: Some(reference.clone()),
            scope: Scope {
                investigation_id: uuid::Uuid::new_v4().to_string(),
                batch: 24,
                created: "0123456789ABCDEF".into(),
                window: u64::MAX,
            },
            delay_ms: 0,
            source: "desktop".into(),
            compare: false,
            remaining: 0,
            mode: reversebolt_core::live::CaptureMode::Full,
            local_model: None,
            background_model: false,
            change: None,
        };
        assert!(serde_json::to_vec(&step).unwrap().len() <= c.jobs.snapshot().limits.payload_bytes);
        let path = c
            .runtime_directory
            .join("tasks")
            .join(format!("{reference}.json"));
        std::fs::write(path, b"{\"operation\":\"discover\"}").unwrap();
        assert!(load_task(&c.runtime_directory, &reference).is_err());
        assert!(store_task(&c.runtime_directory, &task).is_err());
    }

    #[test]
    fn longest_naming_plan_fits_real_identity_and_detects_model_tampering() {
        let engine = EngineHandle::test_worker(initial_view(), |_, _| panic!("No dispatch"));
        let c = controller(engine);
        let model = "m".repeat(128);
        let reference = crate::runtime_bridge::store_plan(
            &c.runtime_directory,
            &reversebolt_core::runtime::Task::Discover,
            Some(&model),
        )
        .unwrap();
        let step = CaptureStep {
            runtime: Some(reference.clone()),
            scope: Scope {
                investigation_id: uuid::Uuid::new_v4().to_string(),
                batch: 24,
                created: "0123456789ABCDEF".into(),
                window: u64::MAX,
            },
            delay_ms: 0,
            source: "desktop".into(),
            compare: false,
            remaining: 0,
            mode: Default::default(),
            local_model: None,
            background_model: false,
            change: None,
        };
        assert!(step.has_model());
        assert!(serde_json::to_vec(&step).unwrap().len() <= c.jobs.snapshot().limits.payload_bytes);
        assert_eq!(
            crate::runtime_bridge::load_plan(&c.runtime_directory, &reference)
                .unwrap()
                .1,
            Some(model)
        );
        let path = c
            .runtime_directory
            .join("tasks")
            .join(format!("{}.json", reference.strip_prefix("n:").unwrap()));
        std::fs::write(path, b"{}").unwrap();
        assert!(crate::runtime_bridge::load_plan(&c.runtime_directory, &reference).is_err());
    }

    #[test]
    fn runtime_job_replay_and_cancel_wait_for_the_owned_operation() {
        let mut view = initial_view();
        view.live.as_mut().unwrap().runtime.available = true;
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let engine = EngineHandle::test_worker(view, move |command, view| {
            started_tx.send(command).unwrap();
            release_rx.recv().unwrap();
            json!({"ok":true,"state":view,"data":{"runtime":{"cards":4}}})
        });
        let mut c = controller(engine);
        let command = request(
            &c,
            "start_runtime_job",
            json!({"task":{"operation":"discover"}}),
        );
        let reply = c.job_command(&command).unwrap();
        let id = reply["ack"]["job"].as_u64().unwrap();
        assert_eq!(c.job_command(&command).unwrap()["duplicate"], true);
        for _ in 0..3 {
            c.tick();
        }
        let dispatched = started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(dispatched.action, "inspect_runtime");
        let cancel = request(&c, "job_control", json!({"job":id,"operation":"cancel"}));
        c.job_command(&cancel).unwrap();
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
        assert!(started_rx.try_recv().is_err());
    }

    #[test]
    fn visual_job_preserves_mode_scope_replay_and_cancellation() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let engine = EngineHandle::test_worker(initial_view(), move |command, view| {
            started_tx.send(command).unwrap();
            release_rx.recv().unwrap();
            view.live.as_mut().unwrap().sample_count += 1;
            json!({"ok":true,"state":view})
        });
        let mut c = controller(engine);
        for args in [
            json!({"mode":"bad"}),
            json!({"mode":"visual","interval_ms":49}),
            json!({"mode":"full","interval_ms":50}),
            json!({"mode":"visual","interval_ms":50,"compare":true}),
        ] {
            let request = request(&c, "start_observation_job", args);
            assert!(c.job_command(&request).is_err());
        }
        assert!(c.jobs.snapshot().jobs.is_empty());
        let command = request(
            &c,
            "start_observation_job",
            json!({"count":3,"interval_ms":50,"mode":"visual"}),
        );
        let first = c.job_command(&command).unwrap();
        let id = first["ack"]["job"].as_u64().unwrap();
        let step: CaptureStep =
            serde_json::from_str(&c.jobs.snapshot().jobs[&id].steps[0].payload).unwrap();
        assert_eq!(step.delay_ms, 0);
        assert!(!step.mode.is_full());
        assert_eq!(c.job_command(&command).unwrap()["duplicate"], true);
        let mut conflict = command.clone();
        conflict.args["mode"] = json!("full");
        conflict.args["interval_ms"] = json!(250);
        assert!(c.job_command(&conflict).is_err());
        for _ in 0..3 {
            c.tick();
        }
        let actual = started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(actual.args["mode"], "visual");
        assert_eq!(
            actual.args["scope"]["investigation_id"],
            "fixture-investigation"
        );
        let cancel = request(&c, "job_control", json!({"job":id,"operation":"cancel"}));
        c.job_command(&cancel).unwrap();
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
        assert!(started_rx.try_recv().is_err());
    }

    #[test]
    fn local_model_plans_are_pinned_and_stop_keeps_the_current_request_owned() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let engine = EngineHandle::test_worker(initial_view(), move |command, view| {
            let model = command.args["local_model"].is_string();
            started_tx.send(command).unwrap();
            if model {
                release_rx.recv().unwrap();
            }
            view.live.as_mut().unwrap().sample_count += 1;
            json!({"ok":true,"state":view})
        });
        let mut c = controller(engine);
        let command = request(
            &c,
            "start_observation_job",
            json!({"count":3,"interval_ms":50,"mode":"visual","local_model":"fixture:model"}),
        );
        let reply = c.job_command(&command).unwrap();
        let id = reply["ack"]["job"].as_u64().unwrap();
        let steps = &c.jobs.snapshot().jobs[&id].steps;
        let models: Vec<_> = steps
            .iter()
            .map(|s| {
                serde_json::from_str::<CaptureStep>(&s.payload)
                    .unwrap()
                    .local_model
            })
            .collect();
        assert_eq!(
            models,
            vec![
                None,
                Some("fixture:model".into()),
                Some("fixture:model".into())
            ]
        );
        c.engine.view.write().unwrap().observer_model = Some("changed:model".into());
        assert_eq!(c.job_command(&command).unwrap()["duplicate"], true);
        let mut conflict = command.clone();
        conflict.args["local_model"] = json!("changed:model");
        assert!(c.job_command(&conflict).is_err());
        for _ in 0..150 {
            c.tick();
            if started_rx
                .try_iter()
                .any(|v| v.args["local_model"] == "fixture:model")
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(c.jobs.snapshot().jobs[&id].next_step, 1);
        assert_eq!(
            c.executor.flight.as_ref().unwrap().poll_interval,
            Duration::from_secs(30)
        );
        let queued = request(
            &c,
            "start_observation_job",
            json!({"count":1,"interval_ms":50,"mode":"visual"}),
        );
        let queued_id = c.job_command(&queued).unwrap()["ack"]["job"]
            .as_u64()
            .unwrap();
        let cancel_queued = request(
            &c,
            "job_control",
            json!({"job":queued_id,"operation":"cancel"}),
        );
        c.job_command(&cancel_queued).unwrap();
        c.tick();
        assert!(
            !c.executor
                .flight
                .as_ref()
                .unwrap()
                .cancel
                .load(Ordering::SeqCst),
            "Cancelling another queued job must not stop this model request"
        );
        let cancel = request(&c, "job_control", json!({"job":id,"operation":"cancel"}));
        c.job_command(&cancel).unwrap();
        c.tick();
        assert!(
            c.executor
                .flight
                .as_ref()
                .unwrap()
                .cancel
                .load(Ordering::SeqCst)
        );
        assert!(c.jobs.view().in_flight.is_some());
        assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Cancelling);
        release_tx.send(()).unwrap();
        for _ in 0..150 {
            c.tick();
            if c.jobs.snapshot().jobs[&id].phase == job::Phase::Cancelled {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Cancelled);
        assert_eq!(c.jobs.snapshot().jobs[&id].next_step, 2);
        assert!(started_rx.try_recv().is_err());
    }

    #[test]
    fn rechecks_queue_four_full_captures_without_refitting_or_model_calls() {
        let mut view = initial_view();
        view.observer_model = Some("chosen:local".into());
        view.live.as_mut().unwrap().prediction_checks.push(
            serde_json::from_value(json!({
                "id":"new-check", "investigation_id":"fixture-investigation", "batch":2,"epoch":1,
                "source":"ocr","track":0,"label":"Quantity","after_frame":0,"last_frame":20,
                "tolerance":0.55,"basis":[],"invalidated":null,"candidates":[],
                "rechecked_from":{"check":"old-check","batch":1,"archive":"historical.json"}
            }))
            .unwrap(),
        );
        let engine = EngineHandle::test_worker(view, |_, _| panic!("No work started"));
        let mut c = controller(engine);
        let receipt = c.start_automatic_observation("desktop".into()).unwrap();
        let id = receipt["ack"]["job"].as_u64().unwrap();
        let steps = &c.jobs.snapshot().jobs[&id].steps;
        assert_eq!(steps.len(), 4);
        for step in steps {
            let plan: CaptureStep = serde_json::from_str(&step.payload).unwrap();
            assert_eq!(plan.mode, reversebolt_core::live::CaptureMode::Full);
            assert!(!plan.compare);
            assert!(plan.local_model.is_none());
        }
    }

    #[test]
    fn automatic_jobs_select_the_configured_model_and_legacy_plans_do_not() {
        let engine = EngineHandle::test_worker(initial_view(), |_, _| panic!("No work started"));
        let mut c = controller(engine);
        c.engine.view.write().unwrap().observer_model = Some("chosen:local".into());
        let receipt = c.start_automatic_observation("desktop".into()).unwrap();
        let id = receipt["ack"]["job"].as_u64().unwrap();
        let models: Vec<_> = c.jobs.snapshot().jobs[&id]
            .steps
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let step: CaptureStep = serde_json::from_str(&s.payload).unwrap();
                step.local_model.map(|m| (i, m))
            })
            .collect();
        assert_eq!(
            models,
            vec![(1, "chosen:local".into()), (15, "chosen:local".into())]
        );
        let background: Vec<_> = c.jobs.snapshot().jobs[&id]
            .steps
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                serde_json::from_str::<CaptureStep>(&s.payload)
                    .unwrap()
                    .background_model
                    .then_some(i)
            })
            .collect();
        assert_eq!(background, vec![1]);
        for args in [
            json!({"count":1,"local_model":"x"}),
            json!({"local_model":true}),
            json!({"local_model":"bad model"}),
        ] {
            let command = request(&c, "start_observation_job", args);
            assert!(c.job_command(&command).is_err());
        }
    }

    #[test]
    fn pause_between_captures_stops_background_owner_and_waits_before_resume() {
        let (sent, received) = mpsc::channel();
        let engine = EngineHandle::test_worker(initial_view(), move |command, view| {
            sent.send(command).unwrap();
            view.live.as_mut().unwrap().sample_count += 1;
            json!({"ok":true,"state":view})
        });
        let mut c = controller(engine);
        let command = request(
            &c,
            "start_observation_job",
            json!({"count":4,"interval_ms":50,"mode":"visual","local_model":"fixture:model"}),
        );
        let id = c.job_command(&command).unwrap()["ack"]["job"]
            .as_u64()
            .unwrap();
        for _ in 0..200 {
            c.tick();
            if c.jobs.snapshot().jobs[&id].next_step == 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(c.jobs.snapshot().jobs[&id].next_step, 2);
        assert!(c.executor.flight.is_none());
        let commands: Vec<_> = received.try_iter().collect();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[1].args["background_model"], true);
        c.engine.view.write().unwrap().analysis = Some(reversebolt_core::engine::AnalysisStatus {
            job: id,
            evidence_id: "pending".into(),
            model: "fixture:model".into(),
            investigation_id: "fixture-investigation".into(),
            batch: 1,
            frame_ids: vec![],
            observed_unix_ms: 0,
            started_unix_ms: 0,
            stopping: false,
        });
        let old_signal = c.executor.owner.as_ref().unwrap().cancel.clone();
        let pause = request(&c, "job_control", json!({"operation":"pause","job":id}));
        c.job_command(&pause).unwrap();
        assert!(old_signal.load(Ordering::SeqCst));
        assert_eq!(c.jobs.snapshot().jobs[&id].phase, job::Phase::Paused);
        c.tick();
        assert!(received.try_recv().is_err());
        let resume = request(&c, "job_control", json!({"operation":"resume","job":id}));
        assert!(c.job_command(&resume).is_err());
        let (reply, answer) = mpsc::channel();
        c.command((
            Command {
                action: "detach_process".into(),
                args: json!({}),
                source: "desktop".into(),
            },
            Some(reply),
        ));
        assert_eq!(answer.recv().unwrap()["ok"], false);
        c.engine.view.write().unwrap().analysis = None;
        c.job_command(&resume).unwrap();
        for _ in 0..100 {
            c.tick();
            if c.jobs.snapshot().jobs[&id].next_step >= 3 {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(c.jobs.snapshot().jobs[&id].next_step >= 3);
        assert!(
            !c.executor
                .owner
                .as_ref()
                .unwrap()
                .cancel
                .load(Ordering::SeqCst)
        );
        assert!(
            old_signal.load(Ordering::SeqCst),
            "Resume must not clear the old worker's cancellation signal"
        );
    }

    #[test]
    fn legacy_capture_plans_keep_their_original_serialized_payload() {
        let legacy = r#"{"scope":{"investigation_id":"i","batch":1,"created":"c","window":1},"delay_ms":250,"source":"agent"}"#;
        let step: CaptureStep = serde_json::from_str(legacy).unwrap();
        assert!(step.mode.is_full());
        assert_eq!(serde_json::to_string(&step).unwrap(), legacy);
    }
}
