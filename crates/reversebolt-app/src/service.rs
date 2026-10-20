use crate::target::ChildTarget;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::post,
};
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
            let mut engine = match Engine::load(path) {
                Ok(engine) => engine,
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                    return;
                }
            };
            *published.write().unwrap() = engine.view.clone();
            let _ = ready_tx.send(Ok(()));
            while let Ok((command, reply)) = receiver.recv() {
                working.store(true, Ordering::SeqCst);
                let read_only = matches!(command.action.as_str(), "status" | "history");
                let result = if command.action == "open_practice" {
                    engine.view.error = None;
                    let seed = command.args["seed"].as_u64().unwrap_or_else(|| {
                        let bytes = uuid::Uuid::new_v4();
                        u64::from_le_bytes(bytes.as_bytes()[..8].try_into().unwrap())
                    });
                    ChildTarget::start(seed)
                        .and_then(|target| engine.open(Box::new(target), &command.source))
                        .map(|_| json!(engine.view))
                } else {
                    engine.execute(&command)
                };
                let mut error = result.err();
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
                    let _ =
                        reply.send(json!({"ok":error.is_none(),"error":error,"state":engine.view}));
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
