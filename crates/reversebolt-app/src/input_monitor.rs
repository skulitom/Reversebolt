//! Job-owned, foreground-scoped Windows key-state sampling. No hooks or text decoding.
use reversebolt_core::{
    Result,
    inputs::{Buffer, Focus, Poll, Window},
    live::ProcessInfo,
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, HWND},
    System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    UI::{
        Input::KeyboardAndMouse::GetAsyncKeyState,
        WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
};

struct OwnedHandle(usize);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0 as HANDLE);
        }
    }
}

pub struct Monitor {
    cancel: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    buffer: Arc<Mutex<Buffer>>,
    worker: Option<JoinHandle<()>>,
}

impl Monitor {
    pub fn start(
        info: &ProcessInfo,
        clock_id: &str,
        origin: Instant,
        cancel: Arc<AtomicBool>,
    ) -> Result<Self> {
        let raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, info.pid) };
        if raw.is_null() {
            return Err("Input sampling cannot verify the selected process.".into());
        }
        let handle = OwnedHandle(raw as usize);
        if format!("{:016X}", crate::native::created(raw)?) != info.created {
            return Err("The input target restarted. Reconnect before observing it.".into());
        }
        let now = || origin.elapsed().as_micros() as u64;
        let buffer = Arc::new(Mutex::new(Buffer::new(
            clock_id.into(),
            uuid::Uuid::new_v4().to_string(),
            now(),
        )));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_buffer = buffer.clone();
        let worker_stop = stop.clone();
        let worker_cancel = cancel.clone();
        let pid = info.pid;
        let window = info.window;
        let worker = std::thread::Builder::new()
            .name("reversebolt-input-context".into())
            .spawn(move || {
                let _handle = handle;
                loop {
                    if worker_stop.load(Ordering::SeqCst) || worker_cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    let begin_us = origin.elapsed().as_micros() as u64;
                    let hwnd = window as HWND;
                    let mut actual_pid = 0;
                    let mut exit = 0;
                    let alive = unsafe {
                        GetExitCodeProcess(_handle.0 as HANDLE, &mut exit) != 0 && exit == 259
                    };
                    let owned = unsafe {
                        GetWindowThreadProcessId(hwnd, &mut actual_pid) != 0 && actual_pid == pid
                    };
                    let foreground = unsafe { GetForegroundWindow() };
                    let mut focus = if !alive || !owned || foreground.is_null() {
                        Focus::Unavailable
                    } else if foreground == hwnd {
                        Focus::SelectedForeground
                    } else {
                        Focus::OutsideSelected
                    };
                    let mut down = Vec::new();
                    if focus == Focus::SelectedForeground {
                        // The high bit is the current state. The shared low "pressed since" bit is never used.
                        for key in 1u8..=254 {
                            if matches!(key, 16..=18) {
                                continue;
                            } // Use distinct left/right modifier states.
                            if unsafe { GetAsyncKeyState(i32::from(key)) } < 0 {
                                down.push(key);
                            }
                        }
                        // A switch during the scan invalidates the entire key set.
                        let after = unsafe { GetForegroundWindow() };
                        let mut after_pid = 0;
                        if after != hwnd
                            || unsafe { GetWindowThreadProcessId(hwnd, &mut after_pid) } == 0
                            || after_pid != pid
                        {
                            down.clear();
                            focus = Focus::Unavailable;
                        }
                    }
                    let end_us = origin.elapsed().as_micros() as u64;
                    if worker_stop.load(Ordering::SeqCst) || worker_cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Ok(mut buffer) = worker_buffer.lock() {
                        if buffer
                            .push(Poll {
                                begin_us,
                                end_us,
                                focus,
                                down,
                            })
                            .is_err()
                        {
                            break;
                        }
                    } else {
                        break;
                    }
                    if !alive || !owned {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(16));
                }
            })
            .map_err(|e| format!("Could not start input sampling: {e}"))?;
        Ok(Self {
            cancel,
            stop,
            buffer,
            worker: Some(worker),
        })
    }

    pub fn owned_by(&self, cancel: &Arc<AtomicBool>) -> bool {
        Arc::ptr_eq(cancel, &self.cancel)
            && !cancel.load(Ordering::SeqCst)
            && self.worker.as_ref().is_some_and(|w| !w.is_finished())
    }
    pub fn window(&self, begin_us: u64, end_us: u64) -> Option<Window> {
        self.buffer.lock().ok()?.window(begin_us, end_us)
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
