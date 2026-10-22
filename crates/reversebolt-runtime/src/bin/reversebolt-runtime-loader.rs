//! Explicit research instrumentation, restricted to a named identity in the loader's session.
use reversebolt_runtime::win::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{ffi::c_void, path::Path, ptr::null_mut, time::Instant};
use windows_sys::Win32::{
    Foundation::*,
    System::{Diagnostics::Debug::WriteProcessMemory, LibraryLoader::*, Memory::*, Threading::*},
};

fn normalized(path: &str) -> String {
    path.trim_start_matches(r"\\?\").to_lowercase()
}
fn system_export(pid: u32, name: &[u8]) -> Result<usize> {
    unsafe {
        let kernel = GetModuleHandleW(wide("kernel32.dll").as_ptr());
        let function =
            GetProcAddress(kernel, name.as_ptr()).ok_or_else(|| error("system export"))? as usize;
        let mut owner = null_mut();
        if GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            function as *const u16,
            &mut owner,
        ) == 0
        {
            return Err(error("export owner"));
        }
        let local = modules(GetCurrentProcessId())?
            .into_iter()
            .find(|m| m.base == owner as usize)
            .ok_or("export owner missing")?;
        let remote = modules(pid)?
            .into_iter()
            .find(|m| normalized(&m.path) == normalized(&local.path) && m.size == local.size)
            .ok_or("matching remote system module missing")?;
        let offset = function
            .checked_sub(local.base)
            .ok_or("export RVA underflow")?;
        if offset >= remote.size {
            return Err("system export outside module".into());
        }
        Ok(remote.base + offset)
    }
}
fn allocate_string(process: HANDLE, value: &str) -> Result<usize> {
    let data: Vec<u8> = wide(value).into_iter().flat_map(u16::to_le_bytes).collect();
    unsafe {
        let address = VirtualAllocEx(
            process,
            null_mut(),
            data.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        if address.is_null() {
            return Err(error("allocate helper argument"));
        }
        let mut written = 0;
        if WriteProcessMemory(
            process,
            address,
            data.as_ptr().cast(),
            data.len(),
            &mut written,
        ) == 0
            || written != data.len()
        {
            VirtualFreeEx(process, address, 0, MEM_RELEASE);
            return Err(error("write helper argument"));
        }
        Ok(address as usize)
    }
}
fn call(process: HANDLE, address: usize, argument: usize, label: &str) -> Result<u32> {
    unsafe {
        let mut thread_id = 0;
        let routine =
            std::mem::transmute::<usize, unsafe extern "system" fn(*mut c_void) -> u32>(address);
        let thread = CreateRemoteThread(
            process,
            std::ptr::null(),
            0,
            Some(routine),
            argument as *const c_void,
            0,
            &mut thread_id,
        );
        if thread.is_null() {
            return Err(error("create helper thread"));
        }
        let thread = Handle(thread);
        println!(
            "{}",
            json!({"stage":label,"thread_id":thread_id,"address":format!("0x{address:x}"),"argument":format!("0x{argument:x}")})
        );
        if WaitForSingleObject(thread.0, 40_000) != WAIT_OBJECT_0 {
            // Caller deliberately leaves the module/arguments in place on this path.
            return Err(format!(
                "{label} did not finish within 40s; thread {thread_id} may still run; arguments/helper retained; do not retry or unload until termination verified"
            ));
        }
        let mut exit = 0;
        if GetExitCodeThread(thread.0, &mut exit) == 0 {
            return Err(error("thread result"));
        }
        println!(
            "{}",
            json!({"stage":label,"thread_id":thread_id,"exit_code":exit})
        );
        Ok(exit)
    }
}
fn run() -> Result<()> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() == 5 && a[1] == "--detect" {
        let pid: u32 = a[2].parse().map_err(|_| "invalid PID")?;
        let expected_session: u32 = a[4].parse().map_err(|_| "invalid session")?;
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SYNCHRONIZE, 0, pid);
            if process.is_null() {
                return Err(error("open detection target"));
            }
            let process = Handle(process);
            if creation(process.0)? != a[3]
                || session(pid)? != expected_session
                || session(GetCurrentProcessId())? != expected_session
            {
                return Err("detection process identity mismatch".into());
            }
            let initial = modules(pid)?;
            if initial.len() > 512 {
                return Err("module count exceeds detection budget".into());
            }
            let mut providers = 0;
            let mut unavailable = 0;
            let mut read_bytes = 0;
            let mut issues = Vec::new();
            for module in &initial {
                let inspection = reversebolt_runtime::pe_exports::from_file(
                    Path::new(&module.path),
                    reversebolt_runtime::REQUIRED_EXPORTS,
                )
                .and_then(|exports| {
                    if exports.image_size as usize != module.size {
                        return Err("Module image size differs from its file".into());
                    }
                    if !exports.forwarded.is_empty() {
                        return Err("Forwarded runtime exports require separate validation".into());
                    }
                    Ok(exports)
                });
                match inspection {
                    Ok(exports) => {
                        read_bytes += exports.read_bytes;
                        providers += usize::from(
                            exports.contains_all(reversebolt_runtime::REQUIRED_EXPORTS),
                        );
                    }
                    Err(error) => {
                        unavailable += 1;
                        if issues.len() < 16 {
                            issues.push(json!({"module":module.path,"error":error}));
                        }
                    }
                }
            }
            if creation(process.0)? != a[3]
                || WaitForSingleObject(process.0, 0) != WAIT_TIMEOUT
                || modules(pid)? != initial
            {
                return Err("target or loaded modules changed during detection".into());
            }
            println!(
                "{}",
                json!({"pid":pid,"created":a[3],"session":expected_session,"supported":providers==1 && unavailable==0,
                    "providers":providers,"module_count":initial.len(),"unavailable":unavailable,"issues":issues,
                    "successful_read_bytes":read_bytes,"basis":"bounded_pe_exports_v1"})
            );
        }
        return Ok(());
    }
    if a.len() != 7 && a.len() != 9 {
        return Err("usage: reversebolt-runtime-loader PID CREATION_HEX SESSION HELPER_ABSOLUTE_PATH SHA256 OUTPUT_ABSOLUTE_PATH [ROOT_START EXPECTED_ROOT_CATALOG_SHA256 | --paths-file LOCATORS_JSON | --patch-file PATCH_JSON]".into());
    }
    let pid: u32 = a[1].parse().map_err(|_| "invalid PID")?;
    let expected_session: u32 = a[3].parse().map_err(|_| "invalid session")?;
    let path_mode = a.len() == 9 && a[7] == "--paths-file";
    let patch_mode = a.len() == 9 && a[7] == "--patch-file";
    let root_start: usize = if a.len() == 9 && !path_mode && !patch_mode {
        a[7].parse().map_err(|_| "invalid root start")?
    } else {
        0
    };
    let expected = if a.len() == 9 && !path_mode && !patch_mode {
        if a[8].len() != 64 || !a[8].bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid root catalog fingerprint".into());
        }
        Some(a[8].to_lowercase())
    } else {
        None
    };
    if root_start > 10_000 {
        return Err("root start exceeds discovery budget".into());
    }
    let paths: Vec<reversebolt_core::runtime::Locator> = if path_mode {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(&a[8])
            .map_err(|e| e.to_string())?
            .take(65537)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 65536 {
            return Err("paths file exceeds 64 KiB".into());
        }
        let paths: Vec<reversebolt_core::runtime::Locator> =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if paths.is_empty() || paths.len() > reversebolt_core::runtime::MAX_REFRESH_PATHS {
            return Err("path count outside refresh budget".into());
        }
        let scope = reversebolt_core::runtime::Scope {
            pid,
            created: a[2].clone(),
            session: expected_session,
        };
        for path in &paths {
            path.validate(&scope)?;
        }
        paths
    } else {
        Vec::new()
    };
    let patch = if patch_mode {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(&a[8])
            .map_err(|e| e.to_string())?
            .take(65537)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 65536 {
            return Err("patch file exceeds 64 KiB".into());
        }
        let patch: reversebolt_core::runtime_patch::Patch =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        patch.validate(
            &reversebolt_core::runtime::Scope {
                pid,
                created: a[2].clone(),
                session: expected_session,
            },
            reversebolt_core::changes::now_ms(),
        )?;
        Some(patch)
    } else {
        None
    };
    let request =
        json!({"output":a[6],"root_start":root_start,"expected_root_catalog_sha256":expected,"paths":paths,"patch":patch})
            .to_string();
    if request.encode_utf16().count() >= 32768 {
        return Err("request too long".into());
    }
    if !Path::new(&a[4]).is_absolute()
        || !Path::new(&a[6]).is_absolute()
        || Path::new(&a[6]).exists()
    {
        return Err("absolute helper/output paths and new output required".into());
    }
    let helper = std::fs::canonicalize(&a[4])
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    // Keep a handle denying replacement/writes until the helper has unloaded.
    use std::io::Read;
    use std::os::windows::fs::OpenOptionsExt;
    let mut helper_file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&helper)
        .map_err(|e| e.to_string())?;
    let mut helper_bytes = Vec::new();
    helper_file
        .read_to_end(&mut helper_bytes)
        .map_err(|e| e.to_string())?;
    let hash = format!("{:x}", Sha256::digest(&helper_bytes));
    if hash != a[5].to_lowercase() {
        return Err("helper SHA256 mismatch".into());
    }
    let helper_exports =
        reversebolt_runtime::pe_exports::from_bytes(&helper_bytes, &["ReverseboltInspect"])?;
    let rva = *helper_exports
        .functions
        .get("ReverseboltInspect")
        .ok_or("helper inspection export missing or forwarded")? as usize;
    unsafe {
        let self_pid = GetCurrentProcessId();
        if pid == self_pid
            || session(self_pid)? != expected_session
            || session(pid)? != expected_session
        {
            return Err("target must have the explicit identity in this loader's session".into());
        }
        let process = OpenProcess(
            PROCESS_QUERY_INFORMATION
                | PROCESS_VM_READ
                | PROCESS_VM_WRITE
                | PROCESS_VM_OPERATION
                | PROCESS_CREATE_THREAD
                | PROCESS_SYNCHRONIZE,
            0,
            pid,
        );
        if process.is_null() {
            return Err(error("open target"));
        }
        let process = Handle(process);
        if creation(process.0)? != a[2] {
            return Err("process creation identity mismatch".into());
        }
        if WaitForSingleObject(process.0, 0) != WAIT_TIMEOUT {
            return Err("target is no longer live".into());
        }
        let mut machine = 0;
        let mut native = 0;
        if IsWow64Process2(process.0, &mut machine, &mut native) == 0
            || machine != 0
            || native != 0x8664
        {
            return Err("requires native Windows x64 target".into());
        }
        let initial = modules(pid)?;
        if initial
            .iter()
            .any(|m| normalized(&m.path) == normalized(&helper))
        {
            return Err("helper already present; uncertain earlier run must be resolved".into());
        }
        let load = system_export(pid, b"LoadLibraryW\0")?;
        let unload = system_export(pid, b"FreeLibrary\0")?;
        let start = Instant::now();
        println!(
            "{}",
            json!({"stage":"validated","pid":pid,"created":a[2],"session":expected_session,"helper_sha256":hash,"output":a[6]})
        );
        let dll_arg = allocate_string(process.0, &helper)?;
        // Exit code is only 32 bits: never treat it as a 64-bit HMODULE.
        let _ = call(process.0, load, dll_arg, "load")?;
        VirtualFreeEx(process.0, dll_arg as *mut c_void, 0, MEM_RELEASE);
        let remote = modules(pid)?
            .into_iter()
            .find(|m| normalized(&m.path) == normalized(&helper))
            .ok_or("helper failed to load")?;
        if rva >= remote.size || helper_exports.image_size as usize != remote.size {
            return Err("inspection export outside helper; helper retained".into());
        }
        let output_arg = allocate_string(process.0, &request)?;
        let code = call(process.0, remote.base + rva, output_arg, "inspect")?;
        VirtualFreeEx(process.0, output_arg as *mut c_void, 0, MEM_RELEASE);
        let unloaded = call(process.0, unload, remote.base, "unload")?;
        let still_present = modules(pid)?
            .iter()
            .any(|m| normalized(&m.path) == normalized(&helper));
        println!(
            "{}",
            json!({"stage":"complete","inspection_exit":code,"unload_exit":unloaded,"helper_absent":!still_present,"elapsed_ms":start.elapsed().as_millis()})
        );
        if code != 0 || unloaded == 0 || still_present {
            return Err("probe or unload failed; inspect retained receipt".into());
        }
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&a[6]).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if !report["error"].is_null() {
            return Err(format!("inspection reported {}", report["error"]));
        }
        if report["pid"] != pid
            || report["created"] != a[2]
            || report["session"] != expected_session
        {
            return Err("report identity mismatch".into());
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{}", json!({"error":e}));
        std::process::exit(1);
    }
}
