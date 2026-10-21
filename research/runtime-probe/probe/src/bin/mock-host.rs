use reversebolt_runtime_probe::win::*;
use serde_json::json;
use std::{io::Write, time::Duration};
use windows_sys::Win32::{
    Foundation::FreeLibrary,
    System::{
        LibraryLoader::{GetProcAddress, LoadLibraryW},
        Threading::{GetCurrentProcess, GetCurrentProcessId},
    },
};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 4, "mock-host DLL RECEIPT STOP_FILE");
    unsafe {
        let module = LoadLibraryW(wide(&args[1]).as_ptr());
        assert!(!module.is_null());
        let tick = GetProcAddress(module, c"MockTick".as_ptr().cast()).unwrap();
        let tick = std::mem::transmute::<
            unsafe extern "system" fn() -> isize,
            unsafe extern "C" fn(),
        >(tick);
        let pid = GetCurrentProcessId();
        let mut out = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args[2])
            .unwrap();
        write!(out,"{}",json!({"pid":pid,"created":creation(GetCurrentProcess()).unwrap(),"session":session(pid).unwrap(),"oracle":{"charge":37,"hidden_bias":1.75,"enabled":true},"mock_abi_only":true})).unwrap();
        out.flush().unwrap();
        for _ in 0..1200 {
            if std::path::Path::new(&args[3]).exists() {
                break;
            }
            tick();
            std::thread::sleep(Duration::from_millis(100));
        }
        FreeLibrary(module);
    }
}
