use crate::target::ChildTarget;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::post,
};
use base64::Engine as _;
use reversebolt_core::{
    Command, Endpoint, Result,
    engine::{Engine, View},
};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

#[derive(Clone)]
pub(crate) struct OperationOwner {
    pub job: u64,
    pub cancel: Arc<AtomicBool>,
}

type Job = (Command, Option<mpsc::Sender<Value>>, Option<OperationOwner>);
pub use crate::jobs::Handle;

#[derive(Clone)]
pub(crate) struct EngineHandle {
    sender: mpsc::SyncSender<Job>,
    pub view: Arc<RwLock<View>>,
    pub busy: Arc<AtomicBool>,
}

impl EngineHandle {
    #[cfg(test)]
    pub(crate) fn test_worker(
        view: View,
        mut execute: impl FnMut(Command, &mut View) -> Value + Send + 'static,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Job>(16);
        let published = Arc::new(RwLock::new(view));
        let busy = Arc::new(AtomicBool::new(false));
        let worker_view = published.clone();
        let worker_busy = busy.clone();
        std::thread::spawn(move || {
            while let Ok((command, reply, _cancel)) = receiver.recv() {
                worker_busy.store(true, Ordering::SeqCst);
                let mut view = worker_view.read().unwrap().clone();
                let result = execute(command, &mut view);
                *worker_view.write().unwrap() = view;
                worker_busy.store(false, Ordering::SeqCst);
                if let Some(reply) = reply {
                    let _ = reply.send(result);
                }
            }
        });
        Self {
            sender,
            view: published,
            busy,
        }
    }
    pub fn start(path: PathBuf) -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Job>(16);
        let (ready_tx, ready_rx) = mpsc::channel();
        let view = Arc::new(RwLock::new(View::default()));
        let busy = Arc::new(AtomicBool::new(false));
        let published = view.clone();
        let working = busy.clone();
        std::thread::spawn(move || {
            let live_directory = path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("captures");
            let mut engine = match Engine::load(path) {
                Ok(engine) => engine,
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                    return;
                }
            };
            *published.write().unwrap() = engine.view.clone();
            let _ = ready_tx.send(Ok(()));
            let mut analysis = crate::observation_pipeline::Pipeline::default();
            loop {
                analysis.poll(&mut engine, &published);
                working.store(analysis.busy(), Ordering::SeqCst);
                let job =
                    receiver.recv_timeout(std::time::Duration::from_millis(if analysis.busy() {
                        25
                    } else {
                        300
                    }));
                if let Some(session) = engine.live.as_mut()
                    && session.refresh_connection()
                {
                    engine.view.live = Some(session.view.clone());
                    *published.write().unwrap() = engine.view.clone();
                }
                let (command, reply, cancel) = match job {
                    Ok(job) => job,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        analysis.stop();
                        while analysis.busy() {
                            analysis.poll(&mut engine, &published);
                            std::thread::sleep(std::time::Duration::from_millis(25));
                        }
                        break;
                    }
                };
                working.store(true, Ordering::SeqCst);
                let read_only = matches!(
                    command.action.as_str(),
                    "status"
                        | "history"
                        | "list_processes"
                        | "list_memory_processes"
                        | "read_capture"
                        | "read_sequence"
                        | "read_change_evidence"
                );
                let result = if command.action == "open_practice" {
                    engine.view.error = None;
                    let seed = command.args["seed"].as_u64().unwrap_or_else(|| {
                        let bytes = uuid::Uuid::new_v4();
                        u64::from_le_bytes(bytes.as_bytes()[..8].try_into().unwrap())
                    });
                    ChildTarget::start(seed)
                        .and_then(|target| engine.open(Box::new(target), &command.source))
                        .map(|_| json!(engine.view))
                } else if matches!(
                    command.action.as_str(),
                    "list_processes"
                        | "list_memory_processes"
                        | "set_observer_model"
                        | "attach_process"
                        | "capture"
                        | "correlate"
                        | "start_prediction_check"
                        | "advance_memory"
                        | "recheck_findings"
                        | "detach_process"
                        | "read_capture"
                        | "submit_observation"
                        | "read_sequence"
                        | "read_change_evidence"
                        | "submit_temporal_observation"
                        | "prepare_change"
                        | "prepare_probe"
                        | "apply_change"
                        | "restore_change"
                        | "observe_change"
                ) {
                    engine.view.error = None;
                    native_command(
                        &mut engine,
                        &command,
                        &live_directory,
                        cancel.as_ref(),
                        &mut analysis,
                        &published,
                    )
                } else {
                    engine.execute(&command)
                };
                engine.view.live = engine.live.as_ref().map(|s| s.view.clone());
                engine.view.changes = engine.changes.records.clone();
                let (data, mut error) = match result {
                    Ok(data) => (
                        if matches!(
                            command.action.as_str(),
                            "read_capture"
                                | "list_memory_processes"
                                | "capture"
                                | "submit_observation"
                                | "read_sequence"
                                | "read_change_evidence"
                                | "submit_temporal_observation"
                                | "prepare_change"
                                | "prepare_probe"
                                | "apply_change"
                                | "restore_change"
                                | "observe_change"
                        ) {
                            data
                        } else {
                            Value::Null
                        },
                        None,
                    ),
                    Err(error) => (Value::Null, Some(error)),
                };
                if let Some(message) = &error {
                    engine.view.error = Some(message.clone());
                    engine.event(
                        &command.source,
                        format!("Could not complete {}: {message}", command.action),
                    );
                }
                if !read_only && let Err(message) = engine.save() {
                    error = Some(format!("Session history could not be saved: {message}"));
                    engine.view.error.clone_from(&error);
                }
                *published.write().unwrap() = engine.view.clone();
                working.store(analysis.busy(), Ordering::SeqCst);
                if let Some(reply) = reply {
                    let _ = reply.send(
                        json!({"ok":error.is_none(),"error":error,"state":engine.view,"data":data}),
                    );
                }
            }
        });
        ready_rx.recv().map_err(|e| e.to_string())??;
        Ok(Self { sender, view, busy })
    }

    pub fn start_command(&self, command: Command) -> Result<mpsc::Receiver<Value>> {
        self.start_owned_command(command, None)
    }
    pub fn start_owned_command(
        &self,
        command: Command,
        cancel: Option<OperationOwner>,
    ) -> Result<mpsc::Receiver<Value>> {
        let (tx, rx) = mpsc::channel();
        self.sender
            .try_send((command, Some(tx), cancel))
            .map_err(|_| "The engine is busy; retry after the current action.".to_string())?;
        Ok(rx)
    }
}

fn native_command(
    engine: &mut Engine,
    command: &Command,
    directory: &std::path::Path,
    owner: Option<&OperationOwner>,
    analysis: &mut crate::observation_pipeline::Pipeline,
    published: &Arc<RwLock<View>>,
) -> Result<Value> {
    if let Some(scope) = command.args.get("scope") {
        let live = engine
            .live
            .as_ref()
            .ok_or("The job's application is no longer connected.")?;
        if scope["investigation_id"] != live.view.investigation_id
            || scope["batch"] != live.view.coverage.batch
            || scope["created"] != live.view.process.created
            || scope["window"] != live.view.process.window
        {
            return Err("The job's target or memory batch changed; no capture was started.".into());
        }
    }
    match command.action.as_str() {
        "read_change_evidence" => {
            let id = command.args["change_id"]
                .as_str()
                .ok_or("Supply the change identity.")?;
            let index = command.args["observation"]
                .as_u64()
                .ok_or("Choose an observation index from change history.")?
                as usize;
            let (baseline, after, bytes) = engine.changes.read_images(id, index)?;
            let images: Vec<_> = [format!("{id}:before"), format!("{id}:observation:{index}")].into_iter().zip(bytes)
                .map(|(frame_id, bytes)| json!({"frame_id":frame_id,"png_base64":base64::engine::general_purpose::STANDARD.encode(bytes)})).collect();
            return Ok(
                json!({"change_id":id,"observation":index,"baseline":baseline,"after":after,"images":images,"limitation":reversebolt_core::effects::LIMITATION}),
            );
        }
        "prepare_change" | "prepare_probe" => {
            let session = engine
                .live
                .as_mut()
                .ok_or("Connect an application first.")?;
            if command.args["investigation_id"] != session.view.investigation_id
                || command.args["batch"] != session.view.coverage.batch
            {
                return Err(
                    "The investigation changed. Read status before preparing a trial.".into(),
                );
            }
            let track = command.args["track"]
                .as_u64()
                .ok_or("Choose a screen number.")? as usize;
            let value = command.args["value"]
                .as_f64()
                .ok_or("Supply a finite requested value.")?;
            let change = if command.action == "prepare_probe" {
                let prediction = command.args["prediction_id"]
                    .as_str()
                    .ok_or("Choose the current prediction check.")?;
                let candidate = command.args["candidate"]
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or("Choose a possibility to test.")?;
                session.prepare_probe(
                    track,
                    value,
                    (prediction, candidate),
                    &command.source,
                    &mut engine.changes,
                )?
            } else {
                session.prepare_change(track, value, &command.source, &mut engine.changes)?
            };
            engine.event(
                &command.source,
                format!(
                    "Prepared trial {} for {}. Gameplay effects remain unverified.",
                    change.id, change.scope.label
                ),
            );
            return Ok(json!(change));
        }
        "apply_change" | "restore_change" | "observe_change" => {
            let id = command.args["change_id"]
                .as_str()
                .ok_or("Supply the prepared change identity.")?;
            let session = engine
                .live
                .as_mut()
                .ok_or("The original application must still be connected.")?;
            let change = if command.action == "observe_change" {
                session.observe_change(id, &mut engine.changes)?
            } else {
                let restored = command.action == "restore_change";
                let mut result = session.execute_change(id, restored, &mut engine.changes)?;
                if matches!(
                    result.phase,
                    reversebolt_core::changes::Phase::Written
                        | reversebolt_core::changes::Phase::Restored
                ) && result.observations.len() < 8
                {
                    // A separate visual check is evidence only. Failure never hides
                    // or retries the already journaled native write.
                    if let Ok(observed) = session.observe_change(id, &mut engine.changes) {
                        result = observed;
                    }
                }
                result
            };
            engine.event(
                &command.source,
                format!("Trial {}: {}", change.id, change.reason),
            );
            return Ok(json!(change));
        }
        "set_observer_model" => {
            let value = command
                .args
                .get("model")
                .ok_or("Supply a local model name, or null to disable it.")?;
            let model = if value.is_null() {
                None
            } else {
                let model = value.as_str().ok_or("Supply a local model name or null.")?;
                crate::local_observer::check_model(model)?;
                Some(model.to_owned())
            };
            engine.view.observer_model = model;
            engine.event(
                &command.source,
                "Updated the local visual observer for future automatic jobs.",
            );
        }
        "read_sequence" => {
            let session = engine
                .live
                .as_ref()
                .ok_or("Connect a program and capture moments first.")?;
            let investigation = command.args["investigation_id"]
                .as_str()
                .ok_or("Supply the investigation identity from status.")?;
            let ids: Vec<String> = serde_json::from_value(command.args["frame_ids"].clone())
                .map_err(|_| "Supply 2–8 frame identities in chronological order.")?;
            let (sequence, images) = session.read_sequence(investigation, &ids)?;
            let images: Vec<_> = sequence.frames.iter().zip(images).map(|(frame, bytes)| json!({
                "frame_id": frame.id, "png_base64": base64::engine::general_purpose::STANDARD.encode(bytes)
            })).collect();
            return Ok(json!({"sequence": sequence, "images": images}));
        }
        "submit_temporal_observation" => {
            let input: reversebolt_core::temporal::Input =
                serde_json::from_value(command.args.clone())
                    .map_err(|e| format!("Invalid temporal observation: {e}"))?;
            let session = engine
                .live
                .as_mut()
                .ok_or("Connect a program and capture moments first.")?;
            let label = input.label.clone();
            let changed = session.submit_temporal_observation(input, &command.source)?;
            if changed {
                engine.event(&command.source, format!("Recorded provisional temporal evidence for {label}. No continuous identity or control was verified."));
            }
            return Ok(json!({"recorded": changed, "duplicate": !changed}));
        }
        "read_capture" => {
            let session = engine
                .live
                .as_ref()
                .ok_or("Connect a program and capture a moment first.")?;
            let investigation = command.args["investigation_id"]
                .as_str()
                .ok_or("Supply the investigation identity from status.")?;
            let frame_id = command.args["frame_id"]
                .as_str()
                .ok_or("Choose an image from the investigation's frames.")?;
            let (frame, bytes) = session.read_frame(investigation, frame_id)?;
            return Ok(
                json!({"frame":frame,"mime_type":"image/png","png_base64":base64::engine::general_purpose::STANDARD.encode(bytes)}),
            );
        }
        "submit_observation" => {
            let input: reversebolt_core::visual::ObservationInput =
                serde_json::from_value(command.args.clone())
                    .map_err(|e| format!("Invalid image observation: {e}"))?;
            let session = engine
                .live
                .as_mut()
                .ok_or("Connect a program and capture a moment first.")?;
            let label = input.label.clone();
            let observer = input.observer.clone();
            let changed = session.submit_observation(input, &command.source)?;
            if changed {
                engine.event(&command.source, format!("Recorded provisional visual evidence for {label} from declared observer {observer}. No control was verified."));
            }
            return Ok(json!({"recorded":changed,"duplicate":!changed}));
        }
        "list_processes" => engine.view.processes = crate::native::processes()?,
        "list_memory_processes" => {
            let session = engine
                .live
                .as_mut()
                .ok_or("Connect an application first.")?;
            session.refresh_memory_sources();
            return Ok(json!({"processes":session.view.memory_sources,
                "error":session.view.memory_sources_error,
                "limitation":"Accessible descendants with a live, same-session parent chain at inventory time. Ancestry does not identify gameplay ownership; inaccessible or exited branches are omitted. Selection starts a new investigation."}));
        }
        "attach_process" => {
            let memory = match (
                command.args.get("memory_pid"),
                command.args.get("memory_created"),
            ) {
                (None, None) => None,
                (Some(pid), Some(created)) => Some((
                    pid.as_u64()
                        .and_then(|p| u32::try_from(p).ok())
                        .filter(|p| *p != 0)
                        .ok_or("memory_pid must identify a listed process.")?,
                    created
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .ok_or("memory_created must be the listed creation identity.")?,
                )),
                _ => return Err("Supply both memory_pid and memory_created, or omit both.".into()),
            };
            let pid = command.args["pid"]
                .as_u64()
                .and_then(|p| u32::try_from(p).ok())
                .ok_or("Choose a valid process.")?;
            let created = command.args["created"]
                .as_str()
                .ok_or("Refresh the process list to obtain its current identity.")?;
            let budget = command.args["budget_mib"].as_u64().unwrap_or(16) as usize;
            let local_models = directory.parent().unwrap_or(directory).join("models");
            let models = if local_models.is_dir() {
                local_models
            } else {
                std::env::current_exe()
                    .map_err(|e| e.to_string())?
                    .parent()
                    .unwrap_or(directory)
                    .join("models")
            };
            let target = crate::native::Process::open(
                pid,
                created,
                command.args["window"].as_u64(),
                models,
                memory,
            )?;
            let session = reversebolt_core::live::LiveSession::attach(
                Box::new(target),
                budget,
                directory.join(uuid::Uuid::new_v4().to_string()),
            )?;
            engine.event(
                &command.source,
                format!(
                    "Connected to {} (window PID {}, memory PID {}) for observation.",
                    session.view.process.name,
                    pid,
                    session.view.memory_process.as_ref().unwrap().pid
                ),
            );
            engine.live = Some(session);
        }
        "capture" => {
            // Reconcile any result that arrived while the command queue was waiting.
            analysis.poll(engine, published);
            let model = command
                .args
                .get("local_model")
                .filter(|v| !v.is_null())
                .map(|v| {
                    v.as_str()
                        .ok_or("local_model must name an installed local vision model.")
                })
                .transpose()?;
            if model.is_some() && owner.is_none() {
                return Err("Local inference must belong to an observation job.".into());
            }
            if let Some(owner) = owner {
                analysis.check(owner)?;
                if owner.cancel.load(Ordering::SeqCst) {
                    return Ok(json!({"local_observation":{"cancelled":true},"captured":false}));
                }
            }
            let mode: reversebolt_core::live::CaptureMode = command
                .args
                .get("mode")
                .map(|value| serde_json::from_value(value.clone()))
                .transpose()
                .map_err(|_| "Choose capture mode full or visual.")?
                .unwrap_or_default();
            if mode == reversebolt_core::live::CaptureMode::Visual
                && command.args["compare"] == true
            {
                return Err("Visual sampling skips text recognition; compare observations separately when enough evidence exists.".into());
            }
            let session = engine
                .live
                .as_mut()
                .ok_or("Select and connect a program first.")?;
            let input_warning =
                owner.and_then(|owner| session.start_inputs(owner.cancel.clone()).err());
            if command.args["compare"] == true {
                let remaining = if owner.is_some() {
                    command.args["remaining"].as_u64().unwrap_or(0).min(16) as usize
                } else {
                    0
                };
                session.capture_for_discovery(remaining)?;
            } else {
                session.capture_mode(mode)?;
            }
            let message = session.view.phase.clone();
            engine.event(&command.source, message);
            if let Some(warning) = input_warning {
                engine.event(
                    &command.source,
                    format!("Input context unavailable: {warning}"),
                );
            }
            if let Some(model) = model {
                let owner = owner.unwrap();
                crate::observation_pipeline::checkpoint(engine, published)?;
                // The final pass waits for any first pass, then uses the latest two images.
                let previous = analysis.drain(engine, owner, published)?;
                if owner.cancel.load(Ordering::SeqCst) {
                    return Ok(json!({"local_observation":previous,"cancelled":true}));
                }
                let started = analysis.start(engine, model, owner, directory, published)?;
                let result = if command.args["background_model"] == true {
                    started
                } else {
                    analysis.drain(engine, owner, published)?
                };
                return Ok(json!({"local_observation":result}));
            }
        }
        "correlate" => {
            let session = engine
                .live
                .as_mut()
                .ok_or("Select and connect a program first.")?;
            session.correlate()?;
            let message = session.view.phase.clone();
            engine.event(&command.source, message);
        }
        "start_prediction_check" => {
            let investigation = command.args["investigation_id"]
                .as_str()
                .ok_or("Read the current investigation identity first.")?;
            let number = |key: &str| {
                command.args[key]
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| format!("Supply {key} from current status."))
            };
            let source = serde_json::from_value(command.args["source"].clone())
                .map_err(|_| "Choose an OCR or external observation series.")?;
            let session = engine
                .live
                .as_mut()
                .ok_or("Connect an application first.")?;
            session.start_prediction_check(
                investigation,
                number("batch")?,
                number("after_frame")?,
                source,
                number("track")?,
            )?;
            let message = session.view.phase.clone();
            engine.event(&command.source, message);
        }
        "advance_memory" | "recheck_findings" => {
            let investigation = command.args["investigation_id"]
                .as_str()
                .ok_or("Read the current investigation identity first.")?;
            let batch = command.args["batch"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or("Supply the current memory batch number.")?;
            let session = engine
                .live
                .as_mut()
                .ok_or("Connect an application first.")?;
            if command.action == "recheck_findings" {
                session.recheck_findings(investigation, batch, &engine.changes)?;
            } else {
                session.advance_memory(investigation, batch)?;
            }
            let message = session.view.phase.clone();
            engine.event(&command.source, message);
        }
        "detach_process" => {
            engine.live = None;
            engine.event(
                &command.source,
                "Detached from the observed program. The program keeps running.",
            );
        }
        _ => return Err("Unsupported native action.".into()),
    }
    Ok(Value::Null)
}

#[derive(Clone)]
struct ApiState {
    handle: Handle,
    token: String,
}

async fn command(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(mut request): Json<Command>,
) -> (StatusCode, Json<Value>) {
    if headers.get("authorization").and_then(|h| h.to_str().ok())
        != Some(&format!("Bearer {}", state.token))
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok":false,"error":"Authentication required."})),
        );
    }
    if headers.contains_key("origin") {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok":false,"error":"Browser requests are unsupported."})),
        );
    }
    request.source = "agent".into();
    match tokio::task::spawn_blocking(move || state.handle.call(request)).await {
        Ok(Ok(value)) => (StatusCode::OK, Json(value)),
        result => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok":false,"error":format!("Engine unavailable: {result:?}")})),
        ),
    }
}

pub async fn serve(handle: Handle, directory: &std::path::Path) -> Result<Endpoint> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let address = listener.local_addr().map_err(|e| e.to_string())?;
    let token = uuid::Uuid::new_v4().to_string();
    let endpoint = Endpoint {
        url: format!("http://{address}"),
        token: token.clone(),
        pid: std::process::id(),
    };
    std::fs::write(
        directory.join("endpoint.json"),
        serde_json::to_vec(&endpoint).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    let router = Router::new()
        .route("/command", post(command))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(ApiState { handle, token });
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router).await {
            eprintln!("Local engine API stopped: {error}");
        }
    });
    Ok(endpoint)
}
