//! File-only diagnostic: never launches a loader, editor or target application.
use std::path::Path;
fn main() {
    let mut names = reversebolt_runtime::REQUIRED_EXPORTS.to_vec();
    names.push("ReverseboltInspect");
    let mut failed = false;
    for path in std::env::args().skip(1) {
        let start = std::time::Instant::now();
        let value = match reversebolt_runtime::pe_exports::from_file(Path::new(&path), &names) {
            Ok(exports) => {
                serde_json::json!({"path":path,"runtime_provider":exports.contains_all(reversebolt_runtime::REQUIRED_EXPORTS),"exports":exports})
            }
            Err(error) => {
                failed = true;
                serde_json::json!({"path":path,"error":error})
            }
        };
        println!(
            "{}",
            serde_json::json!({"inspection":value,"elapsed_ms":start.elapsed().as_millis(),"target_access":false,"dll_mapping":false})
        );
    }
    if failed {
        std::process::exit(1);
    }
}
