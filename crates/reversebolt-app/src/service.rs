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

type Job = (Command, Option<mpsc::Sender<Value>>);
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
            while let Ok((command, reply)) = receiver.recv() {
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
            loop {
                let job = receiver.recv_timeout(std::time::Duration::from_millis(300));
                if let Some(session) = engine.live.as_mut()
                    && session.refresh_connection()
                {
                    engine.view.live = Some(session.view.clone());
                    *published.write().unwrap() = engine.view.clone();
                }
                let (command, reply) = match job {
                    Ok(job) => job,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };
                working.store(true, Ordering::SeqCst);
                let read_only = matches!(
                    command.action.as_str(),
                    "status" | "history" | "list_processes" | "read_capture" | "read_sequence"
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
                        | "attach_process"
                        | "capture"
                        | "correlate"
                        | "start_prediction_check"
                        | "advance_memory"
                        | "detach_process"
                        | "read_capture"
                        | "submit_observation"
                        | "read_sequence"
                        | "submit_temporal_observation"
                ) {
                    engine.view.error = None;
                    native_command(&mut engine, &command, &live_directory)
                } else {
                    engine.execute(&command)
                };
                engine.view.live = engine.live.as_ref().map(|s| s.view.clone());
                let (data, mut error) = match result {
                    Ok(data) => (
                        if matches!(
                            command.action.as_str(),
                            "read_capture"
                                | "submit_observation"
                                | "read_sequence"
                                | "submit_temporal_observation"
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
                working.store(false, Ordering::SeqCst);
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
        let (tx, rx) = mpsc::channel();
        self.sender
            .try_send((command, Some(tx)))
            .map_err(|_| "The engine is busy; retry after the current action.".to_string())?;
        Ok(rx)
    }
}

fn native_command(
    engine: &mut Engine,
    command: &Command,
    directory: &std::path::Path,
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
        "attach_process" => {
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
            )?;
            let session = reversebolt_core::live::LiveSession::attach(
                Box::new(target),
                budget,
                directory.join(uuid::Uuid::new_v4().to_string()),
            )?;
            engine.event(
                &command.source,
                format!(
                    "Connected to {} (PID {}) for window and memory observation.",
                    session.view.process.name, pid
                ),
            );
            engine.live = Some(session);
        }
        "capture" => {
            let session = engine
                .live
                .as_mut()
                .ok_or("Select and connect a program first.")?;
            if command.args["compare"] == true {
                session.capture_for_report()?;
            } else {
                session.capture()?;
            }
            let message = session.view.phase.clone();
            engine.event(&command.source, message);
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
        "advance_memory" => {
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
            session.advance_memory(investigation, batch)?;
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
