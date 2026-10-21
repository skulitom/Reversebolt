use crate::fixture::{Scenario, World};
use reversebolt_core::{Result, engine::PracticeTarget};
use serde_json::{Value, json};
use std::{
    ffi::c_void,
    io::{BufRead, BufReader, Write},
    os::windows::process::CommandExt,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
    fn ReadProcessMemory(
        process: *mut c_void,
        address: *const c_void,
        buffer: *mut c_void,
        size: usize,
        read: *mut usize,
    ) -> i32;
    fn WriteProcessMemory(
        process: *mut c_void,
        address: *mut c_void,
        buffer: *const c_void,
        size: usize,
        written: *mut usize,
    ) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
}

pub fn serve(seed: u64) -> Result<()> {
    let mut world = World::new(seed, Scenario::ScaledInteger);
    println!(
        "{}",
        json!({"base": world.bytes.as_ptr() as usize, "length": world.bytes.len()})
    );
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    for line in std::io::stdin().lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        let reply = match line.as_str() {
            "step" => json!({"observations": world.advance()}),
            "idle" => json!({"observations": world.idle()}),
            "quit" => break,
            _ => json!({"error":"unknown command"}),
        };
        println!("{reply}");
        std::io::stdout().flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub struct ChildTarget {
    child: Child,
    input: ChildStdin,
    output: mpsc::Receiver<std::result::Result<String, std::io::Error>>,
    handle: *mut c_void,
    base: usize,
    length: usize,
}

impl Drop for ChildTarget {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if !self.handle.is_null() {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

impl ChildTarget {
    /// Used only by the native backend's owned-process regression check.
    pub(crate) fn arena(&self) -> (usize, usize) {
        (self.base, self.length)
    }
    pub fn start(seed: u64) -> Result<Self> {
        let mut child = Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
            .args(["--fixture", &seed.to_string()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
        let input = child.stdin.take().ok_or("Child input unavailable.")?;
        let output = child.stdout.take().ok_or("Child output unavailable.")?;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut target = Self {
            child,
            input,
            output: rx,
            handle: std::ptr::null_mut(),
            base: 0,
            length: 0,
        };
        let hello = target.reply()?;
        target.base = hello["base"].as_u64().ok_or("Missing practice arena.")? as usize;
        target.length = hello["length"]
            .as_u64()
            .ok_or("Missing practice arena length.")? as usize;
        if target.length != 256 * 1024 {
            return Err("Unexpected practice arena size.".into());
        }
        target.handle = unsafe { OpenProcess(0x08 | 0x10 | 0x20, 0, target.child.id()) };
        if target.handle.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(target)
    }

    fn reply(&self) -> Result<Value> {
        let line = self
            .output
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "The practice process stopped responding.".to_string())?
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&line).map_err(|e| e.to_string())
    }
}

impl PracticeTarget for ChildTarget {
    fn pid(&self) -> u32 {
        self.child.id()
    }
    fn sample(&mut self, advance: bool) -> Result<Vec<f64>> {
        writeln!(self.input, "{}", if advance { "step" } else { "idle" })
            .map_err(|e| e.to_string())?;
        self.input.flush().map_err(|e| e.to_string())?;
        self.reply()?["observations"]
            .as_array()
            .ok_or("Missing observation data.")?
            .iter()
            .map(|v| {
                v.as_f64()
                    .filter(|n| n.is_finite())
                    .ok_or("Invalid observation.".into())
            })
            .collect()
    }
    fn read(&self) -> Result<Vec<u8>> {
        let mut bytes = vec![0; self.length];
        let mut transferred = 0;
        let success = unsafe {
            ReadProcessMemory(
                self.handle,
                self.base as *const _,
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                &mut transferred,
            )
        };
        if success == 0 || transferred != bytes.len() {
            return Err(format!(
                "Memory read failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(bytes)
    }
    fn write(&self, offset: usize, bytes: &[u8]) -> Result<()> {
        if offset
            .checked_add(bytes.len())
            .is_none_or(|end| end > self.length)
        {
            return Err("Write exceeds the practice arena.".into());
        }
        let mut transferred = 0;
        let success = unsafe {
            WriteProcessMemory(
                self.handle,
                (self.base + offset) as *mut _,
                bytes.as_ptr().cast(),
                bytes.len(),
                &mut transferred,
            )
        };
        if success == 0 || transferred != bytes.len() {
            return Err(format!(
                "Memory write failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
}
