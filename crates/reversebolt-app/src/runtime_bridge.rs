//! Process-scoped runtime instrumentation with durable evidence and an uncertainty fence.
use reversebolt_core::{
    Result,
    live::ProcessInfo,
    runtime::{Locator, Scope},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    os::windows::{fs::OpenOptionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

const CREATE_NO_WINDOW: u32 = 0x08000000;
fn runtime_scope(info: &ProcessInfo) -> Scope {
    Scope {
        pid: info.pid,
        created: info.created.to_ascii_lowercase(),
        session: info.session,
    }
}
fn read_bounded(path: &Path, max: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err("Runtime evidence exceeds its size limit.".into());
    }
    Ok(bytes)
}
fn read_json(path: &Path, max: u64) -> Result<Value> {
    serde_json::from_slice(&read_bounded(path, max)?).map_err(|e| e.to_string())
}
fn save(path: &Path, value: &Value) -> Result<()> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer(&file, value).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}
struct Tools {
    loader: PathBuf,
    helper: PathBuf,
    hash: String,
    _pin: File,
}
impl Tools {
    fn load() -> Result<Self> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let dir = exe.parent().ok_or("Application folder unavailable.")?;
        let loader = dir.join("reversebolt-runtime-loader.exe");
        let helper = dir.join("reversebolt_runtime.dll");
        let pin = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&loader)
            .map_err(|_| "Runtime reader is not installed beside the app.")?;
        let mut bytes = Vec::new();
        File::open(&helper)
            .map_err(|e| e.to_string())?
            .take(32 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 32 * 1024 * 1024 {
            return Err("Runtime helper exceeds its size limit.".into());
        }
        Ok(Self {
            loader,
            helper,
            hash: format!("{:x}", Sha256::digest(bytes)),
            _pin: pin,
        })
    }
}
fn logs(dir: &Path) -> Result<(File, File)> {
    let make = |name| {
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(dir.join(name))
            .map_err(|e| e.to_string())
    };
    Ok((make("stdout.jsonl")?, make("stderr.jsonl")?))
}
fn new_directory(base: &Path) -> Result<PathBuf> {
    let dir = base.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}
pub fn detect(info: &ProcessInfo, base: &Path) -> Result<bool> {
    let tools = Tools::load()?;
    let scope = runtime_scope(info);
    let dir = new_directory(&base.join("detection"))?;
    let (stdout, stderr) = logs(&dir)?;
    let mut child = Command::new(&tools.loader)
        .args([
            "--detect",
            &info.pid.to_string(),
            &scope.created,
            &info.session.to_string(),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    loop {
        if let Some(exit) = child.try_wait().map_err(|e| e.to_string())? {
            if !exit.success() {
                return Err("Internal-value discovery is unavailable for this application.".into());
            }
            let value = read_json(&dir.join("stdout.jsonl"), 65536)?;
            if value["pid"] != info.pid
                || value["created"] != scope.created
                || value["session"] != info.session
            {
                return Err("Runtime detection identity changed.".into());
            }
            return Ok(value["supported"] == true);
        }
        if start.elapsed() > Duration::from_secs(10) {
            // Detection only maps exports in this child; it never starts a target thread.
            let _ = child.kill();
            let _ = child.wait();
            return Err("Runtime detection timed out.".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn completed(dir: &Path, scope: &Scope, hash: &str) -> bool {
    let Ok(bytes) = read_bounded(&dir.join("stdout.jsonl"), 65536) else {
        return false;
    };
    let Ok(receipts) = std::str::from_utf8(&bytes) else {
        return false;
    };
    let Ok(values) = receipts
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<std::result::Result<Vec<_>, _>>()
    else {
        return false;
    };
    values.first().is_some_and(|v| {
        v["stage"] == "validated"
            && v["pid"] == scope.pid
            && v["created"] == scope.created
            && v["session"] == scope.session
            && v["helper_sha256"] == hash
    }) && values
        .last()
        .is_some_and(|v| v["stage"] == "complete" && v["helper_absent"] == true)
}

fn process_directory(base: &Path, scope: &Scope) -> PathBuf {
    base.join(format!("{}-{}", scope.pid, scope.created))
}

/// A missing/settled fence proves no owned operation is outstanding. Invalid or
/// incomplete evidence must never be interpreted as permission to start another.
fn pending(base: &Path, scope: &Scope) -> bool {
    let dir = process_directory(base, scope);
    let fence = dir.join("active.json");
    if matches!(fence.try_exists(), Ok(false)) {
        return false;
    }
    let Ok(prior) = read_json(&fence, 8192) else {
        return true;
    };
    if prior["settled"] == true {
        return false;
    }
    let Some(id) = prior["id"]
        .as_str()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
    else {
        return true;
    };
    let Some(hash) = prior["helper_sha256"].as_str() else {
        return true;
    };
    !completed(&dir.join(id.to_string()), scope, hash)
}

fn preflight(output: &Path, paths: &[Locator]) -> Result<()> {
    // Match the loader's actual payload limits before creating an unsettled fence.
    let request = json!({"output":output.to_string_lossy(),"root_start":0,
        "expected_root_catalog_sha256":null,"paths":paths})
    .to_string();
    if request.encode_utf16().count() >= 32768
        || serde_json::to_vec(paths).map_err(|e| e.to_string())?.len() > 65536
    {
        return Err(
            "These value paths exceed the reader's request limit. Refresh fewer values.".into(),
        );
    }
    Ok(())
}
pub struct Outcome {
    pub report: Option<Value>,
    pub evidence: PathBuf,
    pub unresolved: bool,
    pub cancelled: bool,
    pub error: Option<String>,
}
pub fn command(
    engine: &mut reversebolt_core::engine::Engine,
    command: &reversebolt_core::Command,
    owner: Option<&crate::service::OperationOwner>,
    base: &Path,
    published: &std::sync::Arc<std::sync::RwLock<reversebolt_core::engine::View>>,
) -> Result<Value> {
    use reversebolt_core::runtime::{Card, Task};
    let owner = owner.ok_or("Runtime inspection must belong to a managed job.")?;
    let task: Task = serde_json::from_value(command.args["task"].clone())
        .map_err(|_| "Invalid runtime task.")?;
    task.validate()?;
    let session = engine
        .live
        .as_mut()
        .ok_or("Connect an application first.")?;
    if !session.view.connected || !session.view.runtime.available {
        return Err("No supported runtime is available in this application.".into());
    }
    let info = session
        .view
        .memory_process
        .as_ref()
        .unwrap_or(&session.view.process)
        .clone();
    let paths = session.view.runtime.paths(&task)?;
    if owner.cancel.load(Ordering::SeqCst) {
        return Ok(json!({"runtime":{"cancelled":true}}));
    }
    session.view.runtime.phase = "inspecting".into();
    session.view.runtime.notice = None;
    crate::observation_pipeline::checkpoint(engine, published)?;
    let observed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let outcome = run(&info, &paths, base, &owner.cancel);
    let session = engine.live.as_mut().unwrap();
    let view = &mut session.view.runtime;
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => {
            let scope = runtime_scope(&info);
            view.phase = if pending(base, &scope) {
                "uncertain"
            } else {
                "failed"
            }
            .into();
            view.notice = Some(error.clone());
            return Err(error);
        }
    };
    view.evidence = Some(outcome.evidence.to_string_lossy().into_owned());
    if let Some(error) = outcome.error {
        view.phase = if outcome.unresolved {
            "uncertain"
        } else {
            "failed"
        }
        .into();
        view.notice = Some(error.clone());
        return Err(error);
    }
    if outcome.cancelled {
        view.phase = "stopped".into();
        view.notice = Some(
            "Inspection stopped. Its evidence is retained; new readings were not published.".into(),
        );
        return Ok(json!({"runtime":{"cancelled":true}}));
    }
    let report = outcome.report.ok_or("Missing runtime report.")?;
    let publication = match &task {
        Task::Discover => reversebolt_core::runtime::candidates(&report, 64).map(|candidates| {
            view.cards = candidates
                .into_iter()
                .map(|candidate| Card {
                    value: candidate.value.clone(),
                    candidate,
                    status: "read".into(),
                    observed_unix_ms: observed,
                })
                .collect();
            view.catalog_id = uuid::Uuid::new_v4().to_string();
            view.revision = 0;
        }),
        Task::Refresh { .. } => view.apply_refresh(&task, &report, observed),
    };
    if let Err(error) = publication {
        view.phase = "failed".into();
        view.notice = Some(error.clone());
        return Err(error);
    }
    view.phase = "ready".into();
    view.notice = Some(
        "Internal values are provisional. Their gameplay effects have not been tested.".into(),
    );
    let result = json!({"runtime":{"catalog_id":view.catalog_id,"revision":view.revision,"cards":view.cards.len(),"edit_authority":false}});
    engine.event(
        &command.source,
        "Inspected internal values. Gameplay meaning and effects remain unverified.",
    );
    Ok(result)
}
pub fn run(
    info: &ProcessInfo,
    paths: &[Locator],
    base: &Path,
    cancel: &AtomicBool,
) -> Result<Outcome> {
    let tools = Tools::load()?;
    let scope = runtime_scope(info);
    if scope.created.len() != 16 || !scope.created.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid process identity.".into());
    }
    for path in paths {
        path.validate(&scope)?;
    }
    let process_dir = process_directory(base, &scope);
    std::fs::create_dir_all(&process_dir).map_err(|e| e.to_string())?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(process_dir.join("lock"))
        .map_err(|e| e.to_string())?;
    fs2::FileExt::try_lock_exclusive(&lock)
        .map_err(|_| "Another runtime inspection still owns this process.")?;
    let fence = process_dir.join("active.json");
    if pending(base, &scope) {
        return Err("An earlier runtime inspection has not confirmed completion. No new helper was started; keep its evidence or restart the target application.".into());
    }
    let dir = new_directory(&process_dir)?;
    let output = dir.join("report.json");
    preflight(&output, paths)?;
    if cancel.load(Ordering::SeqCst) {
        return Ok(Outcome {
            report: None,
            evidence: dir,
            unresolved: false,
            cancelled: true,
            error: None,
        });
    }
    let mut args = vec![
        info.pid.to_string(),
        scope.created.clone(),
        info.session.to_string(),
        tools.helper.to_string_lossy().into_owned(),
        tools.hash.clone(),
        output.to_string_lossy().into_owned(),
    ];
    if !paths.is_empty() {
        let pathfile = dir.join("paths.json");
        save(&pathfile, &json!(paths))?;
        args.extend([
            "--paths-file".into(),
            pathfile.to_string_lossy().into_owned(),
        ]);
    }
    let (stdout, stderr) = logs(&dir)?;
    save(
        &fence,
        &json!({"id":dir.file_name().unwrap().to_string_lossy(),"scope":scope,"helper_sha256":tools.hash,"settled":false}),
    )?;
    let mut child = match Command::new(&tools.loader)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            save(&fence, &json!({"settled":true,"spawn_error":e.to_string()}))?;
            return Err(e.to_string());
        }
    };
    let start = Instant::now();
    let mut wait_error = None;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(e) => {
                wait_error = Some(e.to_string());
                break;
            }
        }
        if start.elapsed() > Duration::from_secs(130) {
            wait_error = Some(
                "Runtime reader has not settled; its process and helper were left intact.".into(),
            );
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let settled = completed(&dir, &scope, &tools.hash);
    if settled {
        save(
            &fence,
            &json!({"settled":true,"id":dir.file_name().unwrap().to_string_lossy()}),
        )?;
    }
    let report = if settled {
        read_json(&output, 64 * 1024 * 1024).ok()
    } else {
        None
    };
    let error=wait_error.or_else(|| {
        if !settled {Some("Runtime inspection did not confirm helper unload. Further inspection of this process is held.".into())}
        else if report.as_ref().is_none_or(|r|r["pid"]!=scope.pid || r["created"]!=scope.created || r["session"]!=scope.session) {
            Some(report.as_ref().and_then(|r|r["error"].as_str()).unwrap_or("Runtime result is unavailable or belongs to another process.").chars().take(512).collect())
        } else {None}
    });
    Ok(Outcome {
        report,
        evidence: dir,
        unresolved: !settled,
        cancelled: cancel.load(Ordering::SeqCst),
        error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_fence_requires_completion_but_preflight_errors_do_not_create_one() {
        let base = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let scope = Scope {
            pid: 3,
            created: "0123456789abcdef".into(),
            session: 2,
        };
        assert_eq!(
            runtime_scope(&ProcessInfo {
                pid: 3,
                created: "0123456789ABCDEF".into(),
                session: 2,
                ..Default::default()
            }),
            scope
        );
        let dir = process_directory(&base, &scope);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!pending(&base, &scope));
        assert!(preflight(&PathBuf::from("x".repeat(32768)), &[]).is_err());
        assert!(!pending(&base, &scope));
        let fence = dir.join("active.json");
        std::fs::write(&fence, b"{").unwrap();
        assert!(pending(&base, &scope));
        let id = uuid::Uuid::new_v4();
        save(
            &fence,
            &json!({"id":id.to_string(),"helper_sha256":"hash","settled":false}),
        )
        .unwrap();
        assert!(pending(&base, &scope));
        let evidence = dir.join(id.to_string());
        std::fs::create_dir(&evidence).unwrap();
        let receipt = evidence.join("stdout.jsonl");
        let validated = json!({"stage":"validated","pid":3,"created":scope.created,"session":2,"helper_sha256":"hash"});
        std::fs::write(
            &receipt,
            format!("{validated}\n{{\"stage\":\"complete\",\"helper_absent\":true}}\n"),
        )
        .unwrap();
        assert!(!pending(&base, &scope));
        std::fs::write(&receipt, b"bad receipt").unwrap();
        assert!(pending(&base, &scope));
        save(&fence, &json!({"settled":true,"spawn_error":"not started"})).unwrap();
        assert!(!pending(&base, &scope));
        std::fs::remove_file(receipt).unwrap();
        std::fs::remove_file(fence).unwrap();
        std::fs::remove_dir(evidence).unwrap();
        std::fs::remove_dir(dir).unwrap();
        std::fs::remove_dir(base).unwrap();
    }
    #[test]
    fn completion_requires_exact_identity_hash_and_unload() {
        let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir(&dir).unwrap();
        let scope = Scope {
            pid: 3,
            created: "0123456789abcdef".into(),
            session: 2,
        };
        let validated = json!({"stage":"validated","pid":3,"created":scope.created,"session":2,"helper_sha256":"hash"});
        let path = dir.join("stdout.jsonl");
        std::fs::write(
            &path,
            format!("{validated}\n{{\"stage\":\"inspect\",\"exit_code\":0}}\n"),
        )
        .unwrap();
        assert!(!completed(&dir, &scope, "hash"));
        std::fs::write(
            &path,
            format!("{validated}\n{{\"stage\":\"complete\",\"helper_absent\":true}}\n"),
        )
        .unwrap();
        assert!(completed(&dir, &scope, "hash"));
        assert!(!completed(&dir, &scope, "other"));
        let mut reused = scope;
        reused.created = "0123456789abcdee".into();
        assert!(!completed(&dir, &reused, "hash"));
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
