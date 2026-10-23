//! Saved-plan development experiments. No game access, input or images.
use reversebolt_core::runtime_test_design::Input;
use reversebolt_observer::runtime_test_design::Receipt;
use serde_json::{Value, json};
use std::{io::Read, path::Path, sync::atomic::AtomicBool};
fn read(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    std::fs::File::open(path)?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Input exceeds budget".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[0] == "check" {
        let dir = Path::new(&args[1]);
        let input: Input = serde_json::from_value(read(&dir.join("input.json"))?)?;
        let receipt: Receipt = serde_json::from_value(read(&dir.join("receipt.json"))?)?;
        println!(
            "{}",
            json!({"checked":true,"design":receipt.checked_design(&input)?,"model_calls":0})
        );
        return Ok(());
    }
    if args.len() != 3 {
        return Err("usage: test-design-replay PLAN NEW_OUTPUT_DIR MODEL | check CASE_DIR".into());
    }
    let saved = read(Path::new(&args[0]))?;
    let plan = &saved["plan"];
    let choices = plan["choices"].as_array().ok_or("Missing choices")?;
    if choices.is_empty() || choices.len() > 60 {
        return Err("Invalid number of choices".into());
    }
    let inputs = choices
        .iter()
        .map(|c| Input::from_choice(plan, c))
        .collect::<Result<Vec<_>, _>>()?;
    // Input order has no role in any one candidate's prompt.
    let mut reverse: Vec<_> = choices
        .iter()
        .rev()
        .map(|c| Input::from_choice(plan, c))
        .collect::<Result<_, _>>()?;
    reverse.reverse();
    if reverse != inputs {
        return Err("Candidate order affected inputs".into());
    }
    let output = Path::new(&args[1]);
    std::fs::create_dir(output)?;
    tokio::runtime::Runtime::new()?.block_on(async {
        let observer=reversebolt_observer::Observer::connect(&args[2]).await?;
        for (i,input) in inputs.iter().enumerate() {
            let dir=output.join(i.to_string());std::fs::create_dir(&dir).map_err(|e|e.to_string())?;
            std::fs::write(dir.join("input.json"),serde_json::to_vec_pretty(input).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
            let receipt=observer.design_runtime_test(input,&AtomicBool::new(false)).await?;
            std::fs::write(dir.join("receipt.json"),serde_json::to_vec_pretty(&receipt).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
            println!("{}",json!({"candidate":i,"error":receipt.error,"design":receipt.design,"elapsed_ms":receipt.stage.elapsed_ms}));
        }
        Ok::<_,String>(())
    })?;
    Ok(())
}
