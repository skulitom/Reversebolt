//! Small Windows helpers for this explicit instrumentation experiment only.
use std::{ffi::c_void, mem::size_of, ptr::null_mut};
use windows_sys::Win32::{
    Foundation::*,
    System::{
        Diagnostics::{Debug::ReadProcessMemory, ToolHelp::*},
        RemoteDesktop::ProcessIdToSessionId,
        Threading::*,
    },
};
pub type Result<T> = std::result::Result<T, String>;
pub fn error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}
pub struct Handle(pub HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
#[derive(Clone, Debug)]
pub struct Module {
    pub base: usize,
    pub size: usize,
    pub path: String,
}
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}
pub fn utf16(s: &[u16]) -> String {
    String::from_utf16_lossy(&s[..s.iter().position(|v| *v == 0).unwrap_or(s.len())])
}
pub fn modules(pid: u32) -> Result<Vec<Module>> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);
        if snap == INVALID_HANDLE_VALUE {
            return Err(error("module snapshot"));
        }
        let snap = Handle(snap);
        let mut entry: MODULEENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<MODULEENTRY32W>() as u32;
        if Module32FirstW(snap.0, &mut entry) == 0 {
            return Err(error("first module"));
        }
        let mut result = Vec::new();
        loop {
            result.push(Module {
                base: entry.modBaseAddr as usize,
                size: entry.modBaseSize as usize,
                path: utf16(&entry.szExePath),
            });
            if Module32NextW(snap.0, &mut entry) == 0 {
                break;
            }
        }
        Ok(result)
    }
}
// HANDLE is an opaque kernel token, validated by Windows rather than dereferenced here.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn read(process: HANDLE, address: usize, size: usize) -> Result<Vec<u8>> {
    if address == 0 || size > 64 * 1024 * 1024 {
        return Err("invalid read range".into());
    }
    let mut bytes = vec![0; size];
    let mut count = 0;
    unsafe {
        if ReadProcessMemory(
            process,
            address as *const c_void,
            bytes.as_mut_ptr().cast(),
            size,
            &mut count,
        ) == 0
            || count != size
        {
            return Err(error("read memory"));
        }
    }
    Ok(bytes)
}
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn creation(process: HANDLE) -> Result<String> {
    unsafe {
        let mut times = [FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        }; 4];
        if GetProcessTimes(
            process,
            &mut times[0],
            &mut times[1],
            &mut times[2],
            &mut times[3],
        ) == 0
        {
            return Err(error("creation identity"));
        }
        Ok(format!(
            "{:016x}",
            ((times[0].dwHighDateTime as u64) << 32) | times[0].dwLowDateTime as u64
        ))
    }
}
pub fn session(pid: u32) -> Result<u32> {
    let mut session = 0;
    if unsafe { ProcessIdToSessionId(pid, &mut session) } == 0 {
        return Err(error("session"));
    }
    Ok(session)
}
pub fn self_read(address: usize, size: usize) -> Result<Vec<u8>> {
    read(unsafe { GetCurrentProcess() }, address, size)
}
pub fn text_pointer(address: usize) -> Result<String> {
    let mut result = Vec::new();
    // Bounded reads avoid trusting a target-owned pointer as a Rust CStr.
    for i in 0..4096 {
        let byte = self_read(address.checked_add(i).ok_or("pointer overflow")?, 1)?[0];
        if byte == 0 {
            return String::from_utf8(result).map_err(|_| "invalid UTF-8 metadata".into());
        }
        result.push(byte);
    }
    Err("metadata text exceeds 4096 bytes".into())
}
pub fn null() -> *mut c_void {
    null_mut()
}
