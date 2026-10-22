//! Short-lived input worker. It outlives an editor crash long enough to release
//! its keys, never focuses a window, and never repeats a submitted action.
use reversebolt_core::{Result, changes::now_ms, live::ProcessInfo, runtime_tests::Action};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::os::windows::process::CommandExt;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, HWND},
    System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    UI::{
        Input::KeyboardAndMouse::*,
        WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ProcessIdToSessionId(pid: u32, session: *mut u32) -> i32;
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema: u32,
    test_id: String,
    revision: u64,
    condition: usize,
    frame: usize,
    target: ProcessInfo,
    parent: u32,
    parent_created: String,
    action: Option<Action>,
}
struct Process(HANDLE);
impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
impl Process {
    fn open(pid: u32, created: &str) -> Result<Self> {
        let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if h.is_null() {
            return Err("Input target or editor is unavailable.".into());
        }
        let process = Self(h);
        if !format!("{:016X}", crate::native::created(h)?).eq_ignore_ascii_case(created) {
            return Err("Input process identity changed.".into());
        }
        Ok(process)
    }
    fn alive(&self) -> bool {
        let mut code = 0;
        unsafe { GetExitCodeProcess(self.0, &mut code) != 0 && code == 259 }
    }
}
fn session(pid: u32) -> Result<u32> {
    let mut s = 0;
    if unsafe { ProcessIdToSessionId(pid, &mut s) } == 0 {
        return Err("Input session is unavailable.".into());
    }
    Ok(s)
}
fn allowed(action: &Action) -> bool {
    (100..=1000).contains(&action.hold_ms)
        && !action.keys.is_empty()
        && action.keys.len() <= 3
        && action.keys.windows(2).all(|p| p[0] < p[1])
        && action
            .keys
            .iter()
            .all(|k| matches!(k,32|37..=40|48..=57|65..=90))
}
fn save(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(&bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| e.to_string())
}
fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 16_384 {
        return Err("Input receipt exceeds its budget.".into());
    }
    Ok(bytes)
}
fn read(path: &Path) -> Result<Value> {
    serde_json::from_slice(&read_bytes(path)?).map_err(|e| e.to_string())
}
fn packet(key: u8, up: bool) -> Result<INPUT> {
    let scan = unsafe { MapVirtualKeyW(u32::from(key), MAPVK_VK_TO_VSC_EX) };
    if scan == 0 {
        return Err("The planned key has no scan code.".into());
    }
    Ok(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: (scan & 0xff) as u16,
                dwFlags: KEYEVENTF_SCANCODE
                    | if scan & 0xff00 != 0 {
                        KEYEVENTF_EXTENDEDKEY
                    } else {
                        0
                    }
                    | if up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    })
}
struct Held {
    up: Vec<INPUT>,
    released: bool,
}
impl Held {
    fn release(&mut self) -> u32 {
        let n = unsafe {
            SendInput(
                self.up.len() as u32,
                self.up.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            )
        };
        self.released = n as usize == self.up.len();
        n
    }
}
impl Drop for Held {
    fn drop(&mut self) {
        if !self.released {
            self.release();
        }
    }
}

/// Invoked only by this binary's isolated worker entry point.
pub fn worker(plan_path: &Path) -> Result<()> {
    let receipt_path = plan_path.with_extension("outcome.json");
    let cancel_path = plan_path.with_extension("cancel");
    let plan_bytes = read_bytes(plan_path)?;
    let plan: Plan = serde_json::from_slice(&plan_bytes).map_err(|e| e.to_string())?;
    // A crash after this durable claim leaves an unknown outcome. Starting the
    // same worker plan again must never repeat a possibly delivered key press.
    claim_worker(plan_path)?;
    let mut receipt = json!({"started_unix_ms":now_ms(),"keydown_accepted":0,"keyup_accepted":0,"held_elapsed_us":0,"error":null,
        "limitation":"Windows accepted input records, not proof of delivered game actions. Sampling and image evidence must still pass the comparison checks."});
    receipt["plan_sha256"] = json!(reversebolt_core::visual::digest(&plan_bytes));
    receipt["worker_pid"] = json!(std::process::id());
    let result = (|| -> Result<()> {
        let shape = match plan.schema {
            1 => plan.condition < 3 && plan.frame < 6 && (plan.frame < 2) == plan.action.is_none(),
            2 => {
                plan.condition == 0
                    && plan.frame < reversebolt_core::activity_experiment::STEPS
                    && plan.action
                        == reversebolt_core::activity_experiment::action(
                            plan.frame,
                            plan.action
                                .as_ref()
                                .and_then(|a| a.keys.first())
                                .copied()
                                .unwrap_or(32),
                        )
            }
            _ => false,
        };
        if !shape
            || plan.target.window == 0
            || uuid::Uuid::parse_str(&plan.test_id).is_err()
            || plan.action.as_ref().is_some_and(|a| !allowed(a))
        {
            return Err("Unsupported input plan. Automatic steps support letters, digits, arrows and Space only.".into());
        }
        let parent = Process::open(plan.parent, &plan.parent_created)?;
        let target = Process::open(plan.target.pid, &plan.target.created)?;
        if session(std::process::id())? != plan.target.session
            || session(plan.parent)? != plan.target.session
            || session(plan.target.pid)? != plan.target.session
        {
            return Err("Input must run in the selected application's session.".into());
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        let guard = |permitted: &[u8]| -> Result<()> {
            if cancel_path.exists()
                || !parent.alive()
                || !target.alive()
                || Instant::now() >= deadline
            {
                return Err("Input stopped, expired or its editor exited.".into());
            }
            let hwnd = plan.target.window as HWND;
            let mut pid = 0;
            if unsafe { GetForegroundWindow() } != hwnd
                || unsafe { GetWindowThreadProcessId(hwnd, &mut pid) } == 0
                || pid != plan.target.pid
            {
                return Err("The selected application lost focus; input stopped.".into());
            }
            for key in 1u8..=254 {
                if !permitted.contains(&key) && unsafe { GetAsyncKeyState(i32::from(key)) } < 0 {
                    return Err("Another key or mouse button is pressed; input stopped.".into());
                }
            }
            if unsafe { GetForegroundWindow() } != hwnd {
                return Err("Focus changed during input validation.".into());
            }
            Ok(())
        };
        let wait = |duration: Duration, permitted: &[u8]| -> Result<()> {
            let end = Instant::now() + duration;
            loop {
                guard(permitted)?;
                let now = Instant::now();
                if now >= end {
                    return Ok(());
                }
                std::thread::sleep((end - now).min(Duration::from_millis(5)));
            }
        };
        // Bracket the hold with released polls, even when callers issue steps immediately.
        wait(Duration::from_millis(300), &[])?;
        if let Some(action) = &plan.action {
            let down = action
                .keys
                .iter()
                .map(|k| packet(*k, false))
                .collect::<Result<Vec<_>>>()?;
            let up = action
                .keys
                .iter()
                .rev()
                .map(|k| packet(*k, true))
                .collect::<Result<Vec<_>>>()?;
            guard(&[])?;
            let mut held = Held {
                up,
                released: false,
            };
            let begin = Instant::now();
            let sent = unsafe {
                SendInput(
                    down.len() as u32,
                    down.as_ptr(),
                    std::mem::size_of::<INPUT>() as i32,
                )
            };
            receipt["keydown_accepted"] = json!(sent);
            let holding = if sent as usize == down.len() {
                wait(
                    Duration::from_millis(u64::from(action.hold_ms)),
                    &action.keys,
                )
            } else {
                Err("Windows did not accept the entire planned hold.".into())
            };
            let released = held.release();
            receipt["keyup_accepted"] = json!(released);
            receipt["held_elapsed_us"] = json!(begin.elapsed().as_micros());
            holding?;
            if released as usize != action.keys.len() {
                return Err("Key release was incomplete; the release guard retried.".into());
            }
            drop(held);
            wait(Duration::from_millis(300), &[])?;
        }
        Ok(())
    })();
    receipt["completed_unix_ms"] = json!(now_ms());
    if let Err(error) = &result {
        receipt["error"] = json!(error);
    }
    save(&receipt_path, &receipt)?;
    result
}

fn claim_worker(plan_path: &Path) -> Result<()> {
    save(
        &plan_path.with_extension("claimed.json"),
        &json!({"worker_pid":std::process::id(),"started_unix_ms":now_ms()}),
    )
}

pub fn execute(
    info: &ProcessInfo,
    test: &reversebolt_core::runtime_tests::Test,
    condition: usize,
    index: usize,
    image: &Path,
    abort: &AtomicBool,
) -> Result<()> {
    let action = index
        .checked_sub(2)
        .and_then(|i| test.actions.get(i))
        .cloned();
    if action.as_ref().is_some_and(|a| !allowed(a)) {
        return Err("Automatic steps support letters, digits, arrows and Space only.".into());
    }
    if abort.load(Ordering::SeqCst) {
        return Err("The desktop stopped this comparison.".into());
    }
    execute_plan(
        info,
        &test.id,
        test.revision,
        1,
        condition,
        index,
        action,
        image,
        abort,
    )
}

pub fn execute_activity(
    info: &ProcessInfo,
    reference: &str,
    index: usize,
    key: u8,
    image: &Path,
    abort: &AtomicBool,
) -> Result<()> {
    if reference.len() != 64
        || !reference.bytes().all(|b| b.is_ascii_hexdigit())
        || index >= reversebolt_core::activity_experiment::STEPS
    {
        return Err("Invalid input experiment step".into());
    }
    // Explicit desktop actions leave time to return to the selected game.
    // Nothing changes focus; a cancellation remains responsive during this delay.
    if index == 0 {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if abort.load(Ordering::SeqCst) {
                return Err("Input experiment stopped".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let id = uuid::Uuid::parse_str(&reference[..32])
        .map_err(|e| e.to_string())?
        .to_string();
    execute_plan(
        info,
        &id,
        0,
        2,
        0,
        index,
        reversebolt_core::activity_experiment::action(index, key),
        image,
        abort,
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_plan(
    info: &ProcessInfo,
    id: &str,
    revision: u64,
    schema: u32,
    condition: usize,
    index: usize,
    action: Option<Action>,
    image: &Path,
    abort: &AtomicBool,
) -> Result<()> {
    if abort.load(Ordering::SeqCst) {
        return Err("Input stopped before starting its worker".into());
    }
    let parent = std::process::id();
    let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, parent) };
    if h.is_null() {
        return Err("Cannot identify the input controller.".into());
    }
    let handle = Process(h);
    let parent_created = format!("{:016X}", crate::native::created(handle.0)?);
    let plan = Plan {
        schema,
        test_id: id.into(),
        revision,
        condition,
        frame: index,
        target: info.clone(),
        parent,
        parent_created,
        action,
    };
    let path = image.with_extension("action.json");
    save(&path, &plan)?;
    let plan_sha = reversebolt_core::visual::digest(&fs::read(&path).map_err(|e| e.to_string())?);
    let mut child = Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
        .arg("--runtime-test-input")
        .arg(&path)
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut cancelled = false;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if !cancelled && (abort.load(Ordering::SeqCst) || Instant::now() >= deadline) {
            save(&path.with_extension("cancel"), &json!({"stop":true}))?;
            cancelled = true;
        }
        if Instant::now() >= deadline + Duration::from_secs(2) {
            return Err("Input worker outcome is unsettled. Inspect its saved receipt; never replay this step.".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let receipt = read(&path.with_extension("outcome.json"))?;
    if let Some(error) = receipt["error"].as_str() {
        return Err(error.into());
    }
    if cancelled || abort.load(Ordering::SeqCst) {
        return Err("The desktop stopped this comparison after input settled.".into());
    }
    if !status.success() {
        return Err("Input worker exited unsuccessfully".into());
    }
    validate_receipt(&receipt, &plan_sha, child.id(), plan.action.as_ref())?;
    Ok(())
}

fn validate_receipt(
    receipt: &Value,
    plan_sha: &str,
    worker: u32,
    action: Option<&Action>,
) -> Result<()> {
    let expected = action.map_or(0, |a| a.keys.len()) as u64;
    if receipt["plan_sha256"] != plan_sha
        || receipt["worker_pid"].as_u64() != Some(u64::from(worker))
        || receipt.get("error") != Some(&Value::Null)
        || receipt["keydown_accepted"].as_u64() != Some(expected)
        || receipt["keyup_accepted"].as_u64() != Some(expected)
        || receipt["started_unix_ms"]
            .as_u64()
            .zip(receipt["completed_unix_ms"].as_u64())
            .is_none_or(|(a, b)| b < a)
        || receipt["held_elapsed_us"].as_u64().is_none_or(|t| {
            if let Some(action) = action {
                t < u64::from(action.hold_ms) * 1000 || t > 3_000_000
            } else {
                t != 0
            }
        })
    {
        return Err("Input worker receipt is incomplete or does not match its plan".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_receipts_bind_the_plan_worker_and_complete_key_release() {
        let action = Action {
            keys: vec![65],
            hold_ms: 250,
        };
        let receipt = json!({"plan_sha256":"hash","worker_pid":42,"error":null,"keydown_accepted":1,
            "keyup_accepted":1,"started_unix_ms":10,"completed_unix_ms":1000,"held_elapsed_us":250_000});
        validate_receipt(&receipt, "hash", 42, Some(&action)).unwrap();
        for (key, value) in [
            ("plan_sha256", json!("other")),
            ("worker_pid", json!(43)),
            ("keyup_accepted", json!(0)),
            ("error", json!("cancelled")),
            ("held_elapsed_us", json!(100)),
            ("completed_unix_ms", json!(1)),
        ] {
            let mut changed = receipt.clone();
            changed[key] = value;
            assert!(
                validate_receipt(&changed, "hash", 42, Some(&action)).is_err(),
                "{key}"
            );
        }
        assert!(validate_receipt(&json!({}), "hash", 42, None).is_err());
    }
    #[test]
    fn input_worker_plan_cannot_be_claimed_twice_even_without_an_outcome() {
        let directory = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("reversebolt-input-claim-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        let plan = directory.join("input.action.json");
        claim_worker(&plan).unwrap();
        assert!(claim_worker(&plan).is_err());
        assert!(!plan.with_extension("outcome.json").exists());
        assert_eq!(
            directory.parent(),
            Some(std::env::temp_dir().canonicalize().unwrap().as_path())
        );
        assert!(
            directory
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("reversebolt-input-claim-")
        );
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn rejects_system_chords_mouse_and_unbounded_holds() {
        for keys in [
            vec![65, 65],
            vec![68, 65],
            vec![1],
            vec![18, 115],
            vec![91],
            vec![],
        ] {
            assert!(!allowed(&Action { keys, hold_ms: 200 }));
        }
        assert!(!allowed(&Action {
            keys: vec![65],
            hold_ms: 1001
        }));
        assert!(allowed(&Action {
            keys: vec![32, 65],
            hold_ms: 200
        }));
    }
}
