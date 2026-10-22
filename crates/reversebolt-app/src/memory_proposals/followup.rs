use super::*;
use reversebolt_core::memory_followup as f;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Prepare {
    investigation_id: String,
    batch: usize,
    after_sample: usize,
    report_sha256: String,
    groups: Vec<usize>,
    #[serde(default)]
    auto_observe: Option<bool>,
}
pub fn prepare(engine: &mut Engine, root: &Path, args: &Value) -> Result<Value> {
    let p: Prepare = serde_json::from_value(args.clone())
        .map_err(|e| format!("Invalid memory follow-up: {e}"))?;
    let _ = p.auto_observe; // Controller owns automatic job admission.
    let view = &engine
        .live
        .as_ref()
        .ok_or("Connect an application first")?
        .view;
    if p.investigation_id != view.investigation_id
        || p.batch != view.coverage.batch
        || p.after_sample != view.sample_count
    {
        return Err("Supply the exact current investigation, batch and capture count".into());
    }
    let report = store::read(root, &p.investigation_id, p.batch, &p.report_sha256)?;
    let reference = engine
        .live
        .as_mut()
        .ok_or("Connect an application first")?
        .prepare_memory_followup(root, &report, &p.report_sha256, &p.groups, &engine.changes)?;
    Ok(json!({"prepared":true,"reference":reference,"limitation":f::LIMITATION}))
}
pub fn finish(engine: &mut Engine, root: &Path, cancel: &AtomicBool) -> Result<Value> {
    let session = engine.live.as_ref().ok_or("The investigation ended")?;
    let reference = session
        .view
        .memory_followup
        .as_ref()
        .ok_or("No memory follow-up is prepared")?;
    if cancel.load(Ordering::SeqCst) {
        return Err("Memory follow-up cancelled".into());
    }
    let plan = f::read_plan(root, reference)?;
    let (hash, report, replayed) = if let Some((hash, report)) = f::read_report(root, reference)? {
        (hash, report, true)
    } else {
        if session.view.memory_sample_count() < 4 {
            return Ok(
                json!({"status":"skipped","kind":"followup","reason":"Fewer than four fresh memory samples"}),
            );
        }
        let export = session.export_memory_evidence_with_cancel(
            &reference.investigation_id,
            reference.batch,
            session.view.sample_count,
            || cancel.load(Ordering::SeqCst),
        )?;
        let archive = Archive::open(Path::new(&export.directory), &export.manifest_sha256)?;
        let report = f::assess(&plan, &archive, || cancel.load(Ordering::SeqCst))?;
        if cancel.load(Ordering::SeqCst) {
            return Err("Memory follow-up cancelled".into());
        }
        let hash = f::save_report(root, &report)?;
        (hash, report, false)
    };
    if cancel.load(Ordering::SeqCst) {
        return Err("Memory follow-up cancelled before publication".into());
    }
    let result = json!({"status":"completed","kind":"followup","historical":true,"replayed":replayed,
        "summary":{"reference":reference,"report_sha256":hash,"captures":report.captures,"members":report.outcomes.len(),"evidence_manifest_sha256":report.evidence_manifest_sha256}});
    engine.live.as_mut().unwrap().view.memory_proposal_error = None;
    Ok(result)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Page {
    investigation_id: String,
    batch: usize,
    plan_sha256: String,
    report_sha256: Option<String>,
    #[serde(default)]
    offset: usize,
    limit: Option<usize>,
}
pub fn read(root: &Path, args: &Value) -> Result<Value> {
    let p: Page = serde_json::from_value(args.clone())
        .map_err(|e| format!("Invalid memory follow-up page: {e}"))?;
    let reference = f::Reference {
        investigation_id: p.investigation_id,
        batch: p.batch,
        plan_sha256: p.plan_sha256,
    };
    let plan = f::read_plan(root, &reference)?;
    let result = f::read_report(root, &reference)?;
    let limit = p.limit.unwrap_or(8);
    if !(1..=8).contains(&limit) || p.offset > plan.candidates.len() {
        return Err("Use up to eight candidates per page within the plan".into());
    }
    if p.report_sha256
        .as_ref()
        .is_some_and(|h| result.as_ref().is_none_or(|(hash, _)| hash != h))
    {
        return Err("Memory follow-up result checksum does not match".into());
    }
    if p.report_sha256.is_none() && (p.offset != 0 || p.limit.is_some()) {
        return Err("Read the summary first, then pin its result checksum for pagination".into());
    }
    let rows: Vec<_> = if p.report_sha256.is_some() {
        plan.candidates.iter().enumerate().skip(p.offset).take(limit)
            .map(|(i,c)|json!({"index":i,"candidate":c,"outcome":result.as_ref().map(|(_,r)|&r.outcomes[i])})).collect()
    } else {
        vec![]
    };
    Ok(
        json!({"historical":true,"reference":reference,"source_report_sha256":plan.source_report_sha256,
        "source_batch":plan.source_batch,"total_members":plan.candidates.len(),
        "report_sha256":result.as_ref().map(|(h,_)|h),"captures":result.as_ref().map(|(_,r)|r.captures),
        "memory_captures":result.as_ref().map(|(_,r)|&r.memory_captures),"members":rows,
        "input_associations":result.as_ref().and_then(|(_,r)|r.input_associations.as_ref()),
        "evidence_manifest_sha256":result.as_ref().map(|(_,r)|&r.evidence_manifest_sha256),
        "source_manifest_sha256":plan.source_manifest_sha256,
        "next_offset":if p.report_sha256.is_some() && p.offset+limit<plan.candidates.len(){Some(p.offset+limit)}else{None},
        "limitation":f::LIMITATION}),
    )
}
