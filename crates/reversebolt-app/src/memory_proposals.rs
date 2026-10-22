//! Shared, target-free publication and reading of memory-only research proposals.
use crate::service::OperationOwner;
use reversebolt_core::{
    Result,
    engine::Engine,
    memory_evidence::Archive,
    memory_trajectories::{self as m, store},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
pub mod followup;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub investigation_id: String,
    pub batch: usize,
    pub after_sample: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    investigation_id: String,
    batch: usize,
    report_sha256: Option<String>,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
}
pub fn read(directory: &Path, args: &Value) -> Result<Value> {
    let p: Page =
        serde_json::from_value(args.clone()).map_err(|e| format!("Invalid proposal page: {e}"))?;
    let Some(hash) = p.report_sha256 else {
        if p.offset.is_some() || p.limit.is_some() {
            return Err("Read the summary first, then pin its report hash for pagination".into());
        }
        return Ok(
            json!({"historical":true,"summary":store::load_summary(directory,&p.investigation_id,p.batch)?}),
        );
    };
    store::page(
        directory,
        &p.investigation_id,
        p.batch,
        &hash,
        p.offset.unwrap_or(0),
        p.limit.unwrap_or(4),
    )
}
pub fn analyze(
    engine: &mut Engine,
    directory: &Path,
    request: &Request,
    cancel: &AtomicBool,
) -> Result<Value> {
    let session = engine
        .live
        .as_ref()
        .ok_or("Collect memory observations first")?;
    if request.investigation_id != session.view.investigation_id
        || request.batch != session.view.coverage.batch
        || request.after_sample != session.view.sample_count
    {
        return Err("Supply the exact current investigation, batch and sample count".into());
    }
    if cancel.load(Ordering::SeqCst) {
        return Err("Memory review cancelled".into());
    }
    if session.view.memory_followup.is_some() {
        return followup::finish(engine, directory, cancel);
    }
    if session.view.memory_sample_count() < m::MIN_SAMPLES {
        return Err("Memory review needs twelve retained memory-sample attempts".into());
    }
    let (summary, replayed) = if let Some(summary) =
        store::load_summary(directory, &request.investigation_id, request.batch)?
    {
        if summary.captures != request.after_sample {
            return Err(
                "This batch already has a historical memory review at another capture count".into(),
            );
        }
        (summary, true)
    } else {
        let export = session.export_memory_evidence_with_cancel(
            &request.investigation_id,
            request.batch,
            request.after_sample,
            || cancel.load(Ordering::SeqCst),
        )?;
        let archive = Archive::open(Path::new(&export.directory), &export.manifest_sha256)?;
        let report = m::analyze_with_cancel(&archive, || cancel.load(Ordering::SeqCst))?;
        if cancel.load(Ordering::SeqCst) {
            return Err("Memory review cancelled".into());
        }
        (
            store::save(directory, &report, request.after_sample)?,
            false,
        )
    };
    // A completed file may survive cancellation, but it is not silently published.
    if cancel.load(Ordering::SeqCst) {
        return Err("Memory review cancelled before publication".into());
    }
    let view = &mut engine.live.as_mut().unwrap().view;
    view.memory_proposals = Some(summary.clone());
    view.memory_proposal_error = None;
    Ok(json!({"status":"completed","historical":true,"replayed":replayed,"summary":summary}))
}

pub fn finish(engine: &mut Engine, directory: &Path, owner: &OperationOwner) -> Value {
    let Some(session) = engine.live.as_ref() else {
        return json!({"status":"unavailable","error":"The investigation ended"});
    };
    if owner.cancel.load(Ordering::SeqCst) {
        return json!({"status":"cancelled"});
    }
    if session.view.memory_followup.is_some() {
        return match followup::finish(engine, directory, &owner.cancel) {
            Ok(result) => result,
            Err(error) => {
                let error: String = error.chars().take(512).collect();
                if !owner.cancel.load(Ordering::SeqCst) {
                    engine.live.as_mut().unwrap().view.memory_proposal_error = Some(error.clone());
                }
                json!({"status":if owner.cancel.load(Ordering::SeqCst){"cancelled"}else{"unavailable"},"kind":"followup","error":error})
            }
        };
    }
    if session.view.memory_sample_count() < m::MIN_SAMPLES {
        return json!({"status":"skipped","reason":"Fewer than twelve memory samples"});
    }
    let request = Request {
        investigation_id: session.view.investigation_id.clone(),
        batch: session.view.coverage.batch,
        after_sample: session.view.sample_count,
    };
    let result = (|| {
        if let Some(summary) =
            store::load_summary(directory, &request.investigation_id, request.batch)?
        {
            if owner.cancel.load(Ordering::SeqCst) {
                return Err("Memory review cancelled".into());
            }
            engine.live.as_mut().unwrap().view.memory_proposals = Some(summary.clone());
            engine.live.as_mut().unwrap().view.memory_proposal_error = None;
            return Ok(
                json!({"status":"completed","historical":true,"replayed":true,"summary":summary}),
            );
        }
        analyze(engine, directory, &request, &owner.cancel)
    })();
    match result {
        Ok(value) => value,
        Err(error) => {
            let error: String = error.chars().take(512).collect();
            if !owner.cancel.load(Ordering::SeqCst) {
                engine.live.as_mut().unwrap().view.memory_proposal_error = Some(error.clone());
            }
            json!({"status":if owner.cancel.load(Ordering::SeqCst) {"cancelled"} else {"unavailable"},"error":error})
        }
    }
}

#[cfg(test)]
mod tests;
