//! Durable, single-attempt runtime edits shared by human and agent callers.
use crate::{
    Result,
    changes::{Phase, now_ms},
    runtime::Scope,
    runtime_patch::Patch,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const MAX_RECORDS: usize = 128;
const MAX_RECORD_BYTES: usize = 512 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Context {
    pub investigation: String,
    pub catalog_id: String,
    pub revision: u64,
    pub process: Scope,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub path: String,
    pub report_sha256: String,
    pub report: Value,
}
impl Evidence {
    pub fn new(path: String, report: Value) -> Result<Self> {
        let bytes = serde_json::to_vec(&report).map_err(|e| e.to_string())?;
        if bytes.len() > 64 * 1024 || path.len() > 4096 {
            return Err("Runtime trial evidence exceeds its limit.".into());
        }
        Ok(Self {
            path,
            report_sha256: format!("{:x}", Sha256::digest(bytes)),
            report,
        })
    }
    pub(crate) fn valid(&self) -> bool {
        Self::new(self.path.clone(), self.report.clone())
            .is_ok_and(|e| e.report_sha256 == self.report_sha256)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attempt {
    pub patch: Patch,
    pub started_ms: u128,
    pub completed_ms: Option<u128>,
    pub evidence: Option<Evidence>,
    pub outcome: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trial {
    pub id: String,
    #[serde(default)]
    pub direct: bool,
    pub actor: String,
    pub context: Context,
    pub candidate_id: String,
    pub name: String,
    pub patch: Patch,
    pub baseline: Evidence,
    pub phase: Phase,
    pub reason: String,
    pub write: Option<Attempt>,
    pub restoration: Option<Attempt>,
}
impl Trial {
    fn validate_baseline(context: &Context, patch: &Patch, baseline: &Evidence) -> Result<()> {
        patch.validate(&context.process, patch.prepared_unix_ms)?;
        if !baseline.valid()
            || baseline.report["scope"] != json!(context.process)
            || baseline.report["operation"] != "refresh_runtime_paths"
            || baseline.report["checks"]
                .as_array()
                .is_none_or(|c| c.len() != 1)
            || baseline.report["checks"][0]["path_id"] != patch.path.id()
        {
            return Err("Review requires exact evidence for this runtime path.".into());
        }
        let text = crate::runtime_patch::value_text(patch.type_code, &patch.replacement)?;
        let checked = Patch::prepare(
            patch.path.clone(),
            &baseline.report["checks"][0],
            &text,
            patch.prepared_unix_ms,
        )?;
        if checked.id() != patch.id() {
            return Err("Runtime plan contradicts its before-reading.".into());
        }
        Ok(())
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if uuid::Uuid::parse_str(&self.id).is_err()
            || self.id.len() != 36
            || self.candidate_id != self.patch.path.id()
            || self.context.investigation.is_empty()
            || self.context.catalog_id.is_empty()
        {
            return Err("Invalid runtime trial identity.".into());
        }
        Self::validate_baseline(&self.context, &self.patch, &self.baseline)?;
        for (restore, attempt) in [(false, &self.write), (true, &self.restoration)]
            .into_iter()
            .filter_map(|(restore, attempt)| attempt.as_ref().map(|a| (restore, a)))
        {
            attempt
                .patch
                .validate(&self.context.process, attempt.patch.prepared_unix_ms)?;
            if attempt.evidence.as_ref().is_some_and(|e| !e.valid())
                || !matches!(
                    attempt.outcome.as_str(),
                    "written" | "refused" | "unknown" | "already_original"
                )
                || (attempt.outcome == "already_original" && !restore)
                || (attempt.completed_ms.is_none()
                    && (attempt.evidence.is_some()
                        || attempt.error.is_some()
                        || attempt.outcome != "unknown"))
                || attempt.outcome
                    != attempt.evidence.as_ref().map_or("unknown", |e| {
                        // Preserve old refused receipts as recorded. Only new explicit
                        // already-original outcomes require the stronger proof.
                        if attempt.outcome == "already_original" {
                            classify_restoration(&attempt.patch, &e.report)
                        } else {
                            classify(&attempt.patch, &e.report)
                        }
                    })
            {
                return Err("Invalid runtime trial result evidence.".into());
            }
        }
        if let Some(write) = &self.write
            && write.patch.id() != self.patch.id()
        {
            return Err("Runtime write does not match its reviewed plan.".into());
        }
        if let Some(restore) = &self.restoration {
            let write = self
                .write
                .as_ref()
                .filter(|w| w.outcome == "written")
                .and_then(|w| w.evidence.as_ref())
                .ok_or("Missing original runtime write receipt.")?;
            if self
                .patch
                .restoration(&write.report, restore.patch.prepared_unix_ms)?
                .id()
                != restore.patch.id()
            {
                return Err("Runtime restoration does not match the original write.".into());
            }
        }
        let phase_matches = match (&self.write, &self.restoration) {
            (None, None) => matches!(self.phase, Phase::Prepared | Phase::Expired),
            (Some(w), None) if w.completed_ms.is_none() => {
                matches!(self.phase, Phase::Writing | Phase::Uncertain)
            }
            (Some(w), None) => {
                self.phase
                    == match w.outcome.as_str() {
                        "written" => Phase::Written,
                        "refused" => Phase::Refused,
                        _ => Phase::Uncertain,
                    }
            }
            (Some(_), Some(r)) if r.completed_ms.is_none() => {
                matches!(self.phase, Phase::Restoring | Phase::Uncertain)
            }
            (Some(_), Some(r)) => {
                self.phase
                    == match r.outcome.as_str() {
                        "written" | "already_original" => Phase::Restored,
                        "refused" => Phase::Written,
                        _ => Phase::Uncertain,
                    }
            }
            _ => false,
        };
        if !phase_matches {
            return Err("Runtime trial phase contradicts its result.".into());
        }
        Ok(())
    }

    pub fn prepare(
        context: Context,
        name: String,
        actor: String,
        patch: Patch,
        baseline: Evidence,
    ) -> Result<Self> {
        patch.validate(&context.process, now_ms())?;
        Self::validate_baseline(&context, &patch, &baseline)?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            direct: false,
            actor,
            context,
            candidate_id: patch.path.id(),
            name,
            patch,
            baseline,
            phase: Phase::Prepared,
            reason: "Review ready. No value has been changed.".into(),
            write: None,
            restoration: None,
        })
    }
}

pub struct Store {
    directory: PathBuf,
    pub records: Vec<Trial>,
    failed: bool,
}
impl Store {
    pub fn open(directory: PathBuf) -> Result<Self> {
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let mut store = Self {
            directory,
            records: Vec::new(),
            failed: false,
        };
        for entry in fs::read_dir(&store.directory).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            if store.records.len() >= MAX_RECORDS
                || fs::metadata(&path).map_err(|e| e.to_string())?.len() > MAX_RECORD_BYTES as u64
            {
                return Err("Runtime trial history exceeds its budget.".into());
            }
            let mut record: Trial =
                serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("Runtime trial history is unreadable: {e}"))?;
            record.validate()?;
            if path.file_stem().and_then(|s| s.to_str()) != Some(&record.id) {
                return Err("Invalid runtime trial history identity/evidence.".into());
            }
            if matches!(record.phase, Phase::Writing | Phase::Restoring) {
                record.phase = Phase::Uncertain;
                record.reason="An operation was interrupted. Its effect is unknown and it will not be replayed.".into();
                store.persist(&record)?;
            } else if record.phase == Phase::Prepared {
                record.phase = Phase::Expired;
                record.reason =
                    "Reversebolt restarted. Review a fresh value before editing.".into();
                store.persist(&record)?;
            }
            store.records.push(record);
        }
        store.records.sort_by_key(|r| r.patch.prepared_unix_ms);
        Ok(store)
    }
    fn persist(&self, record: &Trial) -> Result<()> {
        record.validate()?;
        let bytes = serde_json::to_vec(record).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_RECORD_BYTES {
            return Err("Runtime trial record exceeds its budget.".into());
        }
        let destination = self.directory.join(format!("{}.json", record.id));
        let temporary = destination.with_extension("pending");
        let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(temporary, destination).map_err(|e| e.to_string())
    }
    pub fn insert(&mut self, record: Trial) -> Result<Trial> {
        if self.failed || self.records.len() >= MAX_RECORDS {
            return Err("Runtime trial history is unavailable or full.".into());
        }
        if record.phase != Phase::Prepared
            || record.write.is_some()
            || record.restoration.is_some()
            || self.records.iter().any(|r| r.id == record.id)
        {
            return Err("Only a new reviewed runtime trial can be inserted.".into());
        }
        self.persist(&record)?;
        self.records.push(record.clone());
        Ok(record)
    }
    fn record(&mut self, record: Trial) -> Result<Trial> {
        let index = self
            .records
            .iter()
            .position(|r| r.id == record.id)
            .ok_or("Unknown runtime trial.")?;
        self.records[index] = record.clone();
        if let Err(error) = self.persist(&record) {
            self.failed = true;
            return Err(format!(
                "Runtime trial history could not be saved. Further writes are held: {error}"
            ));
        }
        Ok(record)
    }
    pub fn get(&self, id: &str) -> Result<Trial> {
        self.records
            .iter()
            .find(|r| r.id == id)
            .cloned()
            .ok_or_else(|| "Unknown runtime trial.".into())
    }
    pub fn holds(&self, scope: &Scope) -> bool {
        self.failed
            || self
                .records
                .iter()
                .any(|r| r.context.process == *scope && r.phase.unresolved())
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn execute(
        &mut self,
        id: &str,
        context: &Context,
        restore: bool,
        backend: impl FnOnce(&Patch) -> Result<Evidence>,
    ) -> Result<Trial> {
        if self.failed {
            return Err("Runtime trial history is unavailable. Further writes are held.".into());
        }
        let mut record = self.get(id)?;
        record.validate()?;
        // Each side has one attempt, including a refused or interrupted attempt.
        // Replays return evidence; they never dispatch the helper again.
        if (restore && record.restoration.is_some()) || (!restore && record.write.is_some()) {
            return Ok(record);
        }
        if self.holds(&context.process) {
            return Err("An earlier runtime write has an uncertain outcome. Further edits to this process are held.".into());
        }
        if record.context.process != context.process
            || record.context.investigation != context.investigation
            || record.context.catalog_id != context.catalog_id
        {
            return Err(
                "The original application investigation changed. This trial cannot be used here."
                    .into(),
            );
        }
        if record.phase
            != if restore {
                Phase::Written
            } else {
                Phase::Prepared
            }
        {
            return Err("This runtime trial is not available for that operation.".into());
        }
        if !restore
            && (record.context.revision != context.revision
                || record.patch.validate(&context.process, now_ms()).is_err())
        {
            record.phase = Phase::Expired;
            record.reason = "The review or its observation changed. Review a fresh value.".into();
            return self.record(record);
        }
        let patch = if restore {
            record.patch.restoration(
                &record
                    .write
                    .as_ref()
                    .and_then(|a| a.evidence.as_ref())
                    .ok_or("Original write receipt unavailable.")?
                    .report,
                now_ms(),
            )?
        } else {
            record.patch.clone()
        };
        let attempt = Attempt {
            patch: patch.clone(),
            started_ms: now_ms(),
            completed_ms: None,
            evidence: None,
            outcome: "unknown".into(),
            error: None,
        };
        if restore {
            record.restoration = Some(attempt);
        } else {
            record.write = Some(attempt);
        }
        record.phase = if restore {
            Phase::Restoring
        } else {
            Phase::Writing
        };
        record.reason = "Operation intent saved. Waiting for its result.".into();
        self.record(record.clone())?;
        let (evidence, error) = match backend(&patch) {
            Ok(e) if e.valid() => (Some(e), None),
            Ok(_) => (
                None,
                Some("Runtime result evidence failed its integrity check.".into()),
            ),
            Err(e) => (None, Some(e)),
        };
        let outcome = evidence
            .as_ref()
            .filter(|e| e.valid())
            .map_or("unknown", |e| {
                if restore {
                    classify_restoration(&patch, &e.report)
                } else {
                    classify(&patch, &e.report)
                }
            });
        let attempt = if restore {
            record.restoration.as_mut().unwrap()
        } else {
            record.write.as_mut().unwrap()
        };
        attempt.completed_ms = Some(now_ms());
        attempt.evidence = evidence;
        attempt.error = error;
        attempt.outcome = outcome.into();
        record.phase = match outcome {
            "already_original" => Phase::Restored,
            "written" => {
                if restore {
                    Phase::Restored
                } else {
                    Phase::Written
                }
            }
            "refused" => {
                if restore {
                    Phase::Written
                } else {
                    Phase::Refused
                }
            }
            _ => Phase::Uncertain,
        };
        record.reason=match (restore,outcome) {
            (false,"written")=>"Requested bytes were observed. Gameplay effects remain unverified.",
            (true,"written")=>"Original bytes were observed again. Downstream gameplay effects may remain.",
            (true,"already_original")=>"Original bytes were already present. No restore write was attempted; downstream gameplay effects may remain.",
            (false,"refused")=>"The value or its reference changed, or the target refused it. No write was attempted.",
            (true,"refused")=>"Restoration was refused. The original change remains recorded; no restore write was attempted.",
            _=>"The operation's effect is uncertain. It will not be retried; further edits to this process are held.",
        }.into();
        self.record(record)
    }
}

/// The argument is the exact inverse of a confirmed trial, checked by Store.
/// An observed original value is not a restore write or a full gameplay rollback.
pub fn classify_restoration(inverse: &Patch, report: &Value) -> &'static str {
    let outcome = classify(inverse, report);
    if outcome != "refused" || report["patch"]["guard"] != "value_changed" {
        return outcome;
    }
    let Ok(desired) = crate::runtime_patch::value_text(inverse.type_code, &inverse.expected) else {
        return outcome;
    };
    let Ok(observed) = Patch::prepare(
        inverse.path.clone(),
        &report["checks"][0],
        &desired,
        inverse.prepared_unix_ms,
    ) else {
        return outcome;
    };
    // Reconstructing the reverse checks the value/bytes, representation, path,
    // field address and entire reference chain without authorizing another write.
    let mut reverse = inverse.clone();
    std::mem::swap(&mut reverse.expected, &mut reverse.replacement);
    if observed.id() == reverse.id() {
        "already_original"
    } else {
        outcome
    }
}

pub fn classify(patch: &Patch, report: &Value) -> &'static str {
    if patch.restoration(report, now_ms()).is_ok() {
        return "written";
    }
    if report["operation"] == "patch_runtime_path"
        && report["patch_id"] == patch.id()
        && report["scope"] == json!(patch.path.scope)
        && report["checks"].as_array().is_some_and(|c| c.len() == 1)
        && report["checks"][0]["path_id"] == patch.path.id()
        && report["patch"]["state"] == "refused"
        && report["patch"]["attempted"] == false
        && report["patch"]["transferred"] == 0
        && report["patch"]["observed_after"].is_null()
    {
        "refused"
    } else {
        "unknown"
    }
}
