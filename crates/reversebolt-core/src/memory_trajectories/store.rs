//! Bounded historical research results; reading never attaches or grants authority.
use super::{Coverage, MAX_GROUPS, MAX_MEMBERS, POLICY, PREFIX, Report};
use crate::{Result, memory_evidence, visual};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const MAX_REPORT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Summary {
    pub investigation_id: String,
    pub batch: usize,
    pub captures: usize,
    pub memory_samples: usize,
    pub policy: String,
    pub report_sha256: String,
    pub evidence_manifest_sha256: String,
    pub groups: usize,
    pub coverage: Coverage,
}

fn folder(root: &Path, investigation: &str, batch: usize) -> Result<PathBuf> {
    if !uuid::Uuid::parse_str(investigation).is_ok_and(|u| u.to_string() == investigation)
        || !(1..=crate::coverage::MAX_BATCHES).contains(&batch)
    {
        return Err("Invalid memory proposal scope".into());
    }
    Ok(root
        .join("memory-proposals")
        .join(investigation)
        .join(format!("batch-{batch:03}")))
}
fn check(report: &Report, investigation: &str, batch: usize) -> Result<()> {
    let numbers: Vec<_> = report
        .selection_captures
        .iter()
        .chain(&report.assessment_captures)
        .copied()
        .collect();
    if report.investigation_id != investigation
        || report.batch != batch
        || report.policy != POLICY
        || report.capture_count == 0
        || report.capture_count > crate::live::MAX_SAMPLES
        || numbers.iter().any(|n| *n > report.capture_count)
        || report.selection_captures.len() != PREFIX
        || !(6..=14).contains(&report.assessment_captures.len())
        || numbers
            .iter()
            .any(|n| *n == 0 || *n > crate::live::MAX_SAMPLES)
        || numbers.windows(2).any(|p| p[0] >= p[1])
        || report.groups.len() > MAX_GROUPS
        || report
            .groups
            .iter()
            .any(|g| g.members.len() > MAX_MEMBERS || g.candidate_count < g.members.len())
        || report.groups.iter().any(|g| {
            g.omitted_members != g.candidate_count - g.members.len()
                || g.order_signature.iter().any(|r| *r as usize >= PREFIX)
                || g.members
                    .iter()
                    .any(|m| m.assessment.values.len() != report.assessment_captures.len())
        })
        || report.coverage.retained_members
            != report.groups.iter().map(|g| g.members.len()).sum::<usize>()
    {
        return Err("Memory proposals have a changed scope, policy or budget".into());
    }
    Ok(())
}
pub(crate) fn same_or_new(path: &Path, bytes: &[u8], limit: usize) -> Result<()> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => file
            .write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if memory_evidence::read_bounded(path, limit)? == bytes {
                Ok(())
            } else {
                Err("An earlier memory proposal file differs; it was not replaced".into())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}
pub fn save(root: &Path, report: &Report, captures: usize) -> Result<Summary> {
    check(report, &report.investigation_id, report.batch)?;
    if captures != report.capture_count
        || captures == 0
        || captures > crate::live::MAX_SAMPLES
        || report
            .assessment_captures
            .last()
            .is_none_or(|n| *n > captures)
    {
        return Err("Invalid proposal capture count".into());
    }
    let bytes = serde_json::to_vec(report).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_REPORT_BYTES {
        return Err("Memory proposals exceed their 4 MiB per-batch limit".into());
    }
    let summary = Summary {
        investigation_id: report.investigation_id.clone(),
        batch: report.batch,
        captures,
        memory_samples: report.selection_captures.len() + report.assessment_captures.len(),
        policy: report.policy.clone(),
        report_sha256: visual::digest(&bytes),
        evidence_manifest_sha256: report.manifest_sha256.clone(),
        groups: report.groups.len(),
        coverage: report.coverage.clone(),
    };
    let directory = folder(root, &report.investigation_id, report.batch)?;
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    same_or_new(&directory.join("report.json"), &bytes, MAX_REPORT_BYTES)?;
    same_or_new(
        &directory.join("summary.json"),
        &serde_json::to_vec(&summary).map_err(|e| e.to_string())?,
        4096,
    )?;
    Ok(summary)
}
pub fn load_summary(root: &Path, investigation: &str, batch: usize) -> Result<Option<Summary>> {
    let directory = folder(root, investigation, batch)?;
    let path = directory.join("summary.json");
    if !path.try_exists().map_err(|e| e.to_string())? {
        return Ok(None);
    }
    let summary: Summary = serde_json::from_slice(&memory_evidence::read_bounded(&path, 4096)?)
        .map_err(|e| e.to_string())?;
    let report = read(root, investigation, batch, &summary.report_sha256)?;
    if summary.investigation_id != investigation
        || summary.batch != batch
        || summary.policy != report.policy
        || summary.evidence_manifest_sha256 != report.manifest_sha256
        || summary.groups != report.groups.len()
        || summary.captures == 0
        || summary.captures != report.capture_count
        || summary.captures > crate::live::MAX_SAMPLES
        || report
            .assessment_captures
            .last()
            .is_none_or(|n| *n > summary.captures)
        || summary.memory_samples
            != report.selection_captures.len() + report.assessment_captures.len()
        || serde_json::to_value(&summary.coverage).map_err(|e| e.to_string())?
            != serde_json::to_value(&report.coverage).map_err(|e| e.to_string())?
    {
        return Err("Memory proposal summary differs from its report".into());
    }
    Ok(Some(summary))
}
pub fn read(root: &Path, investigation: &str, batch: usize, expected: &str) -> Result<Report> {
    if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Supply the exact proposal report SHA-256".into());
    }
    let bytes = memory_evidence::read_bounded(
        &folder(root, investigation, batch)?.join("report.json"),
        MAX_REPORT_BYTES,
    )?;
    if visual::digest(&bytes) != expected {
        return Err("Memory proposal checksum changed".into());
    }
    let report: Report = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    check(&report, investigation, batch)?;
    Ok(report)
}
pub fn page(
    root: &Path,
    investigation: &str,
    batch: usize,
    hash: &str,
    offset: usize,
    limit: usize,
) -> Result<Value> {
    if !(1..=8).contains(&limit) {
        return Err("Read one to eight memory proposal groups per page".into());
    }
    let report = read(root, investigation, batch, hash)?;
    if offset > report.groups.len() {
        return Err("Proposal page is outside this report".into());
    }
    let end = (offset + limit).min(report.groups.len());
    Ok(
        json!({"historical":true,"policy":report.policy,"investigation_id":investigation,"batch":batch,"report_sha256":hash,
        "evidence_manifest_sha256":report.manifest_sha256,"selection_captures":report.selection_captures,"assessment_captures":report.assessment_captures,
        "coverage":report.coverage,"total_groups":report.groups.len(),"next_offset":(end<report.groups.len()).then_some(end),
        "groups":report.groups[offset..end].iter().enumerate().map(|(i,g)|json!({"index":offset+i,"group":g})).collect::<Vec<_>>(),"limitation":report.limitation}),
    )
}
