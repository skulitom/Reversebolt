use reversebolt_runtime_probe::win::*;
use serde_json::json;
use windows_sys::Win32::{
    Foundation::FreeLibrary,
    System::{
        LibraryLoader::{GetProcAddress, LoadLibraryW},
        Threading::{GetCurrentProcess, GetCurrentProcessId},
    },
};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 4);
    unsafe {
        let module = LoadLibraryW(wide(&args[1]).as_ptr());
        assert!(!module.is_null());
        let tick = std::mem::transmute::<
            unsafe extern "system" fn() -> isize,
            unsafe extern "C" fn(),
        >(GetProcAddress(module, c"MockTick".as_ptr().cast()).unwrap());
        let balance = std::mem::transmute::<
            unsafe extern "system" fn() -> isize,
            unsafe extern "C" fn() -> u64,
        >(GetProcAddress(module, c"MockBalance".as_ptr().cast()).unwrap());
        tick();
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&args[2])
            .unwrap();
        let pid = GetCurrentProcessId();
        serde_json::to_writer(file,&json!({"pid":pid,"created":creation(GetCurrentProcess()).unwrap(),"session":session(pid).unwrap(),"mock_abi_only":true})).unwrap();
        for _ in 0..1200 {
            if std::path::Path::new(&args[3]).exists() {
                break;
            }
            tick();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let value = balance();
        println!("{}", json!({"stops":value>>32,"starts":value as u32}));
        FreeLibrary(module);
    }
}
