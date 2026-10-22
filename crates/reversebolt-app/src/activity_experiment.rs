//! Durable input-only capture steps; all input remains owned by the job controller.
use crate::service::OperationOwner;
use reversebolt_core::{
    Result,
    activity_experiment::{self as activity, Plan},
    engine::Engine,
    live::CaptureMode,
    visual,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Step {
    pub activity: String,
    pub index: usize,
    pub key: u8,
}
fn read(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(262_145)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 262_144 {
        return Err("Input experiment document exceeds 256 KiB".into());
    }
    Ok(bytes)
}
fn save(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}
fn location(base: &Path, reference: &str) -> Result<PathBuf> {
    if reference.len() != 64 || !reference.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid input experiment reference".into());
    }
    Ok(base.join("activity").join(reference))
}
pub fn store(base: &Path, plan: &Plan) -> Result<String> {
    let bytes = serde_json::to_vec(plan).map_err(|e| e.to_string())?;
    if bytes.len() > 262_144 {
        return Err("Input experiment plan exceeds its budget".into());
    }
    let reference = visual::digest(&bytes);
    let directory = location(base, &reference)?;
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let path = directory.join("plan.json");
    if path.exists() {
        if read(&path)? != bytes {
            return Err("Stored input experiment changed".into());
        }
    } else {
        save(&path, plan)?;
    }
    Ok(reference)
}
pub fn load(base: &Path, reference: &str) -> Result<Plan> {
    let bytes = read(&location(base, reference)?.join("plan.json"))?;
    if visual::digest(&bytes) != reference {
        return Err("Input experiment checksum changed".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub fn run(engine: &mut Engine, step: &Step, owner: &OperationOwner, base: &Path) -> Result<Value> {
    run_with(
        engine,
        step,
        owner,
        base,
        |info, reference, index, path, abort| {
            crate::test_input::execute_activity(info, reference, index, step.key, path, abort)
        },
    )
}
pub(crate) fn run_with(
    engine: &mut Engine,
    step: &Step,
    owner: &OperationOwner,
    base: &Path,
    execute: impl FnOnce(
        &reversebolt_core::live::ProcessInfo,
        &str,
        usize,
        &Path,
        &std::sync::atomic::AtomicBool,
    ) -> Result<()>,
) -> Result<Value> {
    let effect = activity::action(step.index, step.key).is_some();
    let mut knowledge = if effect {
        "NotApplied"
    } else {
        "NotApplicable"
    };
    let mut reserved = None;
    let mut captured = None;
    let result = (|| -> Result<()> {
        if owner.cancel.load(Ordering::SeqCst) {
            return Err("Input experiment stopped before this step".into());
        }
        let plan = load(base, &step.activity)?;
        let session = engine.live.as_mut().ok_or("Application disconnected")?;
        if plan.key != step.key {
            return Err("Input experiment key changed".into());
        }
        plan.check(&session.view, step.index)?;
        if session.collecting_change_search()
            || engine.view.analysis.is_some()
            || engine.runtime_tests.records.iter().any(|t| t.active())
            || engine
                .view
                .freezes
                .iter()
                .any(|f| f.active() || f.phase == "uncertain")
        {
            return Err(
                "Finish active discovery, comparisons and freezes before testing inputs.".into(),
            );
        }
        session.read_frame(&plan.investigation, &plan.source.id)?;
        // Check budgets before any key can be sent, including failed-image allowances.
        if session.view.sample_count >= reversebolt_core::live::MAX_SAMPLES
            || session.view.coverage.image_budget_bytes + visual::MAX_IMAGE_BYTES as u64
                > reversebolt_core::coverage::MAX_IMAGE_DISK_BYTES
        {
            return Err("No response-image capacity remains".into());
        }
        let directory = location(base, &step.activity)?;
        let intent = directory.join(format!("{:02}.intent.json", step.index));
        save(
            &intent,
            &json!({"plan":step.activity,"index":step.index,"job":owner.job}),
        )?;
        reserved = Some(directory);
        session.start_inputs(owner.cancel.clone())?;
        let prior = session.view.frames.last().cloned();
        if step.index > 0
            && session.input_generation().as_ref()
                != prior
                    .as_ref()
                    .and_then(|f| f.input_context.as_ref())
                    .map(|w| &w.generation)
        {
            return Err("Input sampling restarted; no further press was sent.".into());
        }
        if effect {
            knowledge = "Unknown";
        }
        execute(
            &plan.process,
            &step.activity,
            step.index,
            &intent,
            &owner.cancel,
        )?;
        if effect {
            knowledge = "Applied";
        }
        if owner.cancel.load(Ordering::SeqCst) {
            return Err("Input experiment stopped after input settled".into());
        }
        session.capture_mode(CaptureMode::Play)?;
        let frame = session
            .view
            .frames
            .last()
            .filter(|f| f.number == plan.first_number + step.index)
            .ok_or("No response image was retained")?
            .clone();
        captured = Some(frame.clone());
        plan.check_capture(
            step.index,
            &frame,
            if step.index == 0 {
                None
            } else {
                prior.as_ref()
            },
        )?;
        if owner.cancel.load(Ordering::SeqCst) {
            return Err("Input experiment stopped after capture settled".into());
        }
        Ok(())
    })();
    let result = json!({"success":result.is_ok(),"error":result.err(),"effect":knowledge,
        "experiment":step.activity,"index":step.index,"key":step.key,
        "frame_id":captured.as_ref().map(|f|&f.id),"frame":captured,
        "limitation":activity::LIMITATION});
    if let Some(directory) = reserved {
        save(
            &directory.join(format!("{:02}.outcome.json", step.index)),
            &result,
        )?;
    }
    Ok(result)
}
