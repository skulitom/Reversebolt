//! Prospective, bounded checks of memory proposals. No gameplay/edit authority.
use crate::{
    Result,
    coverage::Chunk,
    memory_evidence::{self, Archive},
    memory_trajectories::{self as m, Assessment, Behavior, Selection, Verdict},
    visual,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
pub mod input;

pub const POLICY: &str = "memory_followup_v1";
pub const MAX_GROUPS: usize = 8;
pub const MAX_MEMBERS: usize = MAX_GROUPS * m::MAX_MEMBERS;
pub const MAX_FILE_BYTES: usize = 256 * 1024;
pub const LIMITATION: &str = "Prospective descriptive checks of selected memory views at their original addresses. Predictions and locations were pinned before fresh capture. No comparison bridges the old and fresh intervals or a missing read. Equal mapping metadata cannot establish allocation identity. Constant fresh values can satisfy an order constraint without demonstrating activity. No gameplay meaning, causal effect, relocation, independent-game validation or edit authority is established.";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reference {
    pub investigation_id: String,
    pub batch: usize,
    pub plan_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub group: usize,
    pub member: usize,
    pub selection: Selection,
    pub source_verdict: Verdict,
    pub source_mapping_invalidated: bool,
    pub window: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub policy: String,
    pub investigation_id: String,
    pub source_batch: usize,
    pub source_captures: usize,
    pub source_report_sha256: String,
    pub source_manifest_sha256: String,
    pub batch: usize,
    pub process_created: String,
    pub window: u64,
    pub candidates: Vec<Candidate>,
    pub windows: Vec<Chunk>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub group: usize,
    pub member: usize,
    pub assessment: Assessment,
    pub changed_pairs: usize,
    pub mapping_invalidated_in_recording: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<input::Evidence>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub reference: Reference,
    pub evidence_manifest_sha256: String,
    pub captures: usize,
    pub memory_captures: Vec<usize>,
    pub outcomes: Vec<Outcome>,
    pub limitation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_associations: Option<input::Description>,
}

fn hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
fn folder(root: &Path, investigation: &str, batch: usize) -> Result<PathBuf> {
    if !uuid::Uuid::parse_str(investigation).is_ok_and(|u| u.to_string() == investigation)
        || !(2..=crate::coverage::MAX_BATCHES).contains(&batch)
    {
        return Err("Invalid memory follow-up scope".into());
    }
    Ok(root
        .join("memory-followups")
        .join(investigation)
        .join(format!("batch-{batch:03}")))
}
impl Plan {
    pub fn validate(&self) -> Result<()> {
        folder(Path::new(""), &self.investigation_id, self.batch)?;
        if self.policy != POLICY
            || self.source_batch != self.batch - 1
            || !(m::MIN_SAMPLES..=crate::live::MAX_SAMPLES).contains(&self.source_captures)
            || !hash(&self.source_report_sha256)
            || !hash(&self.source_manifest_sha256)
            || self.candidates.is_empty()
            || self.candidates.len() > MAX_MEMBERS
            || self.windows.is_empty()
            || self.windows.len() > MAX_MEMBERS
        {
            return Err("Invalid memory follow-up plan or budget".into());
        }
        let mut keys = BTreeSet::new();
        let mut groups = BTreeSet::new();
        for c in &self.candidates {
            groups.insert(c.group);
            let s = &c.selection;
            let w = self
                .windows
                .iter()
                .find(|w| w.address == c.window)
                .ok_or("Missing follow-up window")?;
            if c.group >= m::MAX_GROUPS
                || c.member >= m::MAX_MEMBERS
                || !keys.insert((c.group, c.member))
                || s.address % s.kind.width() as u64 != 0
                || s.values.iter().any(|v| !v.is_finite())
                || s.address < w.address
                || s.address
                    .checked_add(s.kind.width() as u64)
                    .is_none_or(|end| end > w.address.saturating_add(w.length as u64))
            {
                return Err("Invalid follow-up candidate".into());
            }
        }
        if groups.len() > MAX_GROUPS
            || self
                .windows
                .iter()
                .any(|w| w.length == 0 || w.length > 64 || !w.covered_by(&w.region))
            || self
                .windows
                .windows(2)
                .any(|p| p[0].address.saturating_add(p[0].length as u64) > p[1].address)
        {
            return Err("Invalid follow-up windows or group budget".into());
        }
        Ok(())
    }
    pub fn reference(&self) -> Result<Reference> {
        self.validate()?;
        Ok(Reference {
            investigation_id: self.investigation_id.clone(),
            batch: self.batch,
            plan_sha256: visual::digest(&serde_json::to_vec(self).map_err(|e| e.to_string())?),
        })
    }
}
pub fn save_plan(root: &Path, plan: &Plan) -> Result<Reference> {
    let reference = plan.reference()?;
    let dir = folder(root, &plan.investigation_id, plan.batch)?;
    let bytes = serde_json::to_vec(plan).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err("Follow-up plan exceeds its budget".into());
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    m::store::same_or_new(&dir.join("plan.json"), &bytes, MAX_FILE_BYTES)?;
    Ok(reference)
}
pub fn read_plan(root: &Path, reference: &Reference) -> Result<Plan> {
    let bytes = memory_evidence::read_bounded(
        &folder(root, &reference.investigation_id, reference.batch)?.join("plan.json"),
        MAX_FILE_BYTES,
    )?;
    if visual::digest(&bytes) != reference.plan_sha256 {
        return Err("Memory follow-up plan checksum changed".into());
    }
    let plan: Plan = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if plan.reference()? != *reference {
        return Err("Memory follow-up plan scope changed".into());
    }
    Ok(plan)
}

pub fn assess(plan: &Plan, archive: &Archive, cancelled: impl Fn() -> bool) -> Result<Report> {
    let reference = plan.reference()?;
    let manifest = archive.manifest();
    let view = &manifest.state;
    if view.investigation_id != plan.investigation_id
        || view.coverage.batch != plan.batch
        || view.memory_followup.as_ref() != Some(&reference)
        || view.process.created != plan.process_created
        || view.process.window != plan.window
        || serde_json::to_vec(&manifest.plan).map_err(|e| e.to_string())?
            != serde_json::to_vec(&plan.windows).map_err(|e| e.to_string())?
    {
        return Err("Fresh evidence does not match the pinned memory follow-up".into());
    }
    let captures: Vec<_> = manifest
        .samples
        .iter()
        .filter(|s| s.memory_sample)
        .map(|s| s.number)
        .collect();
    if captures.len() < 4 {
        return Err("Memory follow-up needs at least four fresh memory samples".into());
    }
    archive.verify_with_cancel(&cancelled)?;
    let mut values = vec![Vec::new(); plan.candidates.len()];
    let mut missing = vec![0; plan.candidates.len()];
    let mut invalid = missing.clone();
    for n in &captures {
        if cancelled() {
            return Err("Memory follow-up cancelled".into());
        }
        let sample = archive.sample(*n)?;
        for (i, c) in plan.candidates.iter().enumerate() {
            let value = match sample.get(&c.window) {
                None => {
                    missing[i] += 1;
                    None
                }
                Some(bytes) => {
                    let offset = (c.selection.address - c.window) as usize;
                    let value = c.selection.kind.read(&bytes[offset..]);
                    invalid[i] += usize::from(value.is_none());
                    value
                }
            };
            values[i].push(value);
        }
    }
    let outcomes = plan
        .candidates
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut prior = None;
            let (mut checked, mut violations, mut changed_pairs) = (0, 0, 0);
            for v in &values[i] {
                if let Some(value) = v {
                    changed_pairs += usize::from(prior.is_some_and(|p| p != *value));
                    match c.selection.behavior {
                        Behavior::TwoLevels => {
                            checked += 1;
                            violations += usize::from(!c.selection.values.contains(value));
                        }
                        Behavior::Nondecreasing | Behavior::Nonincreasing => {
                            if let Some(p) = prior {
                                checked += 1;
                                violations += usize::from(
                                    if c.selection.behavior == Behavior::Nondecreasing {
                                        *value < p
                                    } else {
                                        *value > p
                                    },
                                );
                            }
                        }
                        Behavior::Varying => (),
                    }
                }
                prior = *v;
            }
            let verdict = if c.selection.behavior == Behavior::Varying {
                Verdict::NotPredicted
            } else if violations > 0 {
                Verdict::Contradicted
            } else if checked < 3 {
                Verdict::Insufficient
            } else {
                Verdict::Consistent
            };
            Outcome {
                group: c.group,
                member: c.member,
                changed_pairs,
                mapping_invalidated_in_recording: manifest.invalid_chunks.contains(&c.window),
                input: input::analyze(manifest, c.window, &captures, &values[i]),
                assessment: Assessment {
                    values: values[i].clone(),
                    missing_reads: missing[i],
                    invalid_numbers: invalid[i],
                    checked,
                    violations,
                    verdict,
                },
            }
        })
        .collect();
    if cancelled() {
        return Err("Memory follow-up cancelled".into());
    }
    Ok(Report {
        reference,
        evidence_manifest_sha256: archive.manifest_sha256().into(),
        captures: view.sample_count,
        memory_captures: captures,
        outcomes,
        limitation: LIMITATION.into(),
        input_associations: manifest
            .samples
            .iter()
            .any(|s| s.input_context.is_some())
            .then(input::Description::default),
    })
}
pub fn save_report(root: &Path, report: &Report) -> Result<String> {
    let plan = read_plan(root, &report.reference)?;
    check_report(&plan, report)?;
    let bytes = serde_json::to_vec(report).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err("Follow-up report exceeds its budget".into());
    }
    let dir = folder(root, &plan.investigation_id, plan.batch)?;
    m::store::same_or_new(&dir.join("report.json"), &bytes, MAX_FILE_BYTES)?;
    let hash = visual::digest(&bytes);
    // A separate, last-written checksum commits the complete report. Partial
    // report writes cannot masquerade as a completed result after restart.
    m::store::same_or_new(&dir.join("report.sha256"), hash.as_bytes(), 64)?;
    Ok(hash)
}
pub fn read_report(root: &Path, reference: &Reference) -> Result<Option<(String, Report)>> {
    let plan = read_plan(root, reference)?;
    let dir = folder(root, &plan.investigation_id, plan.batch)?;
    let checksum = dir.join("report.sha256");
    if !checksum.try_exists().map_err(|e| e.to_string())? {
        return Ok(None);
    }
    let path = dir.join("report.json");
    let bytes = memory_evidence::read_bounded(&path, MAX_FILE_BYTES)?;
    if memory_evidence::read_bounded(&checksum, 64)? != visual::digest(&bytes).as_bytes() {
        return Err("Memory follow-up report checksum changed".into());
    }
    let report: Report = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if report.reference != *reference || report.outcomes.len() != plan.candidates.len() {
        return Err("Memory follow-up report scope changed".into());
    }
    check_report(&plan, &report)?;
    Ok(Some((visual::digest(&bytes), report)))
}

fn check_report(plan: &Plan, report: &Report) -> Result<()> {
    let n = report.memory_captures.len();
    if report.reference != plan.reference()?
        || !hash(&report.evidence_manifest_sha256)
        || !(4..=crate::live::MAX_SAMPLES).contains(&report.captures)
        || !(4..=crate::live::MAX_MEMORY_SAMPLES).contains(&n)
        || report
            .memory_captures
            .iter()
            .any(|n| *n == 0 || *n > report.captures)
        || report.memory_captures.windows(2).any(|p| p[0] >= p[1])
        || report.outcomes.len() != plan.candidates.len()
        || report
            .input_associations
            .as_ref()
            .is_some_and(|d| d.policy != input::POLICY)
        || report.input_associations.is_some() != report.outcomes.iter().any(|o| o.input.is_some())
        || report.outcomes.iter().zip(&plan.candidates).any(|(o, c)| {
            let a = &o.assessment;
            (o.group, o.member) != (c.group, c.member)
                || a.values.len() != n
                || a.missing_reads > n
                || a.invalid_numbers > n - a.missing_reads
                || a.missing_reads + a.invalid_numbers
                    != a.values.iter().filter(|v| v.is_none()).count()
                || a.values.iter().flatten().any(|v| !v.is_finite())
                || a.checked > n
                || a.violations > a.checked
                || o.changed_pairs >= n
                || o.input.as_ref().is_some_and(|e| !e.valid(&a.values))
        })
    {
        return Err("Invalid memory follow-up result scope, counts or members".into());
    }
    Ok(())
}
