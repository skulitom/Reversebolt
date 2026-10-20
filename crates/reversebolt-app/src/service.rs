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

#[derive(Clone)]
pub struct Handle {
    sender: mpsc::SyncSender<Job>,
    pub view: Arc<RwLock<View>>,
    pub busy: Arc<AtomicBool>,
}

impl Handle {
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
                    "status" | "history" | "list_processes" | "read_capture"
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
                        | "detach_process"
                        | "read_capture"
                        | "submit_observation"
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
                            "read_capture" | "submit_observation"
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

    pub fn submit(&self, action: &str, args: Value) {
        let command = Command {
            action: action.into(),
            args,
            source: "desktop".into(),
        };
        if self.sender.try_send((command, None)).is_err() {
            self.view.write().unwrap().error =
                Some("The engine queue is full. Try again after the current action.".into());
        }
    }
    pub fn call(&self, command: Command) -> Result<Value> {
        let (tx, rx) = mpsc::channel();
        self.sender
            .try_send((command, Some(tx)))
            .map_err(|_| "The engine is busy; retry after the current action.".to_string())?;
        rx.recv().map_err(|e| e.to_string())
    }
}

fn native_command(
    engine: &mut Engine,
    command: &Command,
    directory: &std::path::Path,
) -> Result<Value> {
    match command.action.as_str() {
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
            session.capture()?;
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
