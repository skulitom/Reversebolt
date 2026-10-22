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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub base: usize,
    pub size: usize,
    pub path: String,
}
pub const MAX_MODULES: usize = 512;

// A partial inventory cannot prove that our helper is absent or that a runtime
// provider is unique. Only the explicit end-of-list result closes the snapshot.
fn collect_modules(
    mut next: impl FnMut(bool) -> std::result::Result<Module, u32>,
) -> Result<Vec<Module>> {
    let mut result = Vec::new();
    loop {
        match next(result.is_empty()) {
            Ok(module) => {
                if result.len() == MAX_MODULES {
                    return Err("module count exceeds inspection budget".into());
                }
                result.push(module);
            }
            Err(ERROR_NO_MORE_FILES) if !result.is_empty() => return Ok(result),
            Err(code) => {
                return Err(format!(
                    "module enumeration incomplete after {} entries: {}",
                    result.len(),
                    std::io::Error::from_raw_os_error(code as i32)
                ));
            }
        }
    }
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
        collect_modules(|first| {
            let found = if first {
                Module32FirstW(snap.0, &mut entry)
            } else {
                Module32NextW(snap.0, &mut entry)
            };
            if found == 0 {
                // Capture the failure immediately, before formatting or cleanup
                // can overwrite this thread's Windows error code.
                return Err(GetLastError());
            }
            Ok(Module {
                base: entry.modBaseAddr as usize,
                size: entry.modBaseSize as usize,
                path: utf16(&entry.szExePath),
            })
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    fn module(i: usize) -> Module {
        Module {
            base: 0x10000 + i * 4096,
            size: 4096,
            path: format!("module-{i}.dll"),
        }
    }
    #[test]
    fn module_inventory_requires_explicit_completion_before_asserting_absence() {
        for error in [
            ERROR_ACCESS_DENIED,
            ERROR_BAD_LENGTH,
            ERROR_PARTIAL_COPY,
            ERROR_INVALID_HANDLE,
            ERROR_SUCCESS,
        ] {
            let mut calls = 0;
            let inventory = collect_modules(|first| {
                assert_eq!(first, calls == 0);
                calls += 1;
                if first { Ok(module(0)) } else { Err(error) }
            });
            assert!(
                inventory.is_err(),
                "Partial inventory accepted for Windows error {error}"
            );
            // This is the loader's absence check: an error cannot become true.
            let helper_absent =
                inventory.map(|items| items.iter().all(|m| m.path != "reversebolt_runtime.dll"));
            assert!(helper_absent.is_err());
            assert_eq!(calls, 2);
        }
        let mut calls = 0;
        let inventory = collect_modules(|first| {
            assert_eq!(first, calls == 0);
            calls += 1;
            if calls <= 2 {
                Ok(module(calls))
            } else {
                Err(ERROR_NO_MORE_FILES)
            }
        })
        .unwrap();
        assert_eq!(inventory, vec![module(1), module(2)]);
        assert_eq!(calls, 3);
    }
    #[test]
    fn empty_or_failed_first_module_does_not_prove_a_target_has_no_helper() {
        for error in [ERROR_NO_MORE_FILES, ERROR_ACCESS_DENIED, ERROR_BAD_LENGTH] {
            let mut calls = 0;
            assert!(
                collect_modules(|first| {
                    assert!(first);
                    calls += 1;
                    Err(error)
                })
                .is_err()
            );
            assert_eq!(calls, 1);
        }
    }
    #[test]
    fn module_inventory_budget_never_returns_a_truncated_success() {
        for count in [MAX_MODULES, MAX_MODULES + 1] {
            let mut calls = 0;
            let result = collect_modules(|_| {
                calls += 1;
                if calls <= count {
                    Ok(module(calls))
                } else {
                    Err(ERROR_NO_MORE_FILES)
                }
            });
            assert_eq!(result.is_ok(), count == MAX_MODULES);
            assert_eq!(calls, MAX_MODULES + 1);
            if let Ok(modules) = result {
                assert_eq!(modules.len(), MAX_MODULES);
            }
        }
    }
}
