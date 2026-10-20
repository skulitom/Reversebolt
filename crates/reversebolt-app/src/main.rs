#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fixture;
mod service;
mod target;
mod ui;

use fs2::FileExt;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        let directory = reversebolt_core::data_dir();
        let _ = std::fs::create_dir_all(&directory);
        let _ = std::fs::write(directory.join("startup-error.txt"), &error);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--fixture") {
        return target::serve(args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8675309));
    }
    let directory = args
        .iter()
        .position(|s| s == "--data-dir")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(reversebolt_core::data_dir);
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("engine.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock_exclusive()
        .map_err(|_| "Reversebolt is already running for this data directory.".to_string())?;
    let handle = service::Handle::start(directory.join("session.json"))?;
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let endpoint = runtime.block_on(service::serve(handle.clone(), &directory))?;
    if args.iter().any(|s| s == "--headless") {
        runtime.block_on(std::future::pending::<()>());
    } else {
        let options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1120.0, 620.0])
                .with_position([45.0, 25.0])
                .with_min_inner_size([940.0, 580.0]),
            ..Default::default()
        };
        eframe::run_native(
            "Reversebolt",
            options,
            Box::new(move |cc| Ok(Box::new(ui::Workbench::new(cc, handle, endpoint.url)))),
        )
        .map_err(|e| e.to_string())?;
    }
    let _ = std::fs::remove_file(directory.join("endpoint.json"));
    drop(lock);
    Ok(())
}
