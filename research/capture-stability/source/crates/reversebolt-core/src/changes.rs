//! Explicit, bounded trial edits. Byte observations never establish gameplay
//! authority or complete rollback. The native backend provides guarded writes,
//! not an atomic compare-and-swap with the target process.
use crate::{
    Result,
    coverage::{Chunk, Region},
    discovery::Candidate,
    live::{NativeTarget, ProcessInfo},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const MAX_RECORDS: usize = 128;
pub const PLAN_LIFETIME_MS: u128 = 60_000;
pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    #[serde(default)]
    pub reference: Option<crate::references::Reference>,
    pub index: usize,
    pub address: u64,
    pub mapping: Candidate,
    pub region: Region,
    pub before: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytePatch {
    pub address: u64,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alternative {
    pub index: usize,
    pub patch: Option<BytePatch>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scope {
    pub investigation: String,
    pub batch: usize,
    pub epoch: usize,
    pub frame: usize,
    pub prediction: String,
    pub process: ProcessInfo,
    pub track: usize,
    pub label: String,
    pub bounds: [f32; 4],
    pub image_size: [u32; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Unavailable,
    Prepared,
    Writing,
    Written,
    Uncertain,
    Restoring,
    Restored,
    Refused,
    Expired,
}

impl Phase {
    pub fn unresolved(&self) -> bool {
        matches!(self, Self::Writing | Self::Uncertain | Self::Restoring)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WriteReceipt {
    pub attempted: bool,
    pub transferred: usize,
    pub observed_after: Option<Vec<u8>>,
    pub error: Option<String>,
}

impl WriteReceipt {
    pub fn refused(error: impl Into<String>) -> Self {
        Self {
            attempted: false,
            transferred: 0,
            observed_after: None,
            error: Some(error.into()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    #[serde(default)]
    pub sampling: Option<crate::visual::CaptureSampling>,
    pub time_ms: u128,
    pub image: String,
    pub image_sha256: String,
    pub value: Option<f64>,
    pub expected: f64,
    pub agrees: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub warning: Option<String>,
    #[serde(default)]
    pub stage: Option<ObservationStage>,
    #[serde(default)]
    pub capture_interval: Option<crate::visual::CaptureInterval>,
    #[serde(default)]
    pub readings: Vec<crate::effects::Reading>,
    #[serde(default)]
    pub untracked_numbers: usize,
    #[serde(default)]
    pub comparison: Option<crate::effects::Comparison>,
    #[serde(default)]
    pub probe_comparisons: Vec<crate::probes::Comparison>,
    #[serde(default)]
    pub intervention_memory: Vec<crate::intervention::Sample>,
    #[serde(default)]
    pub intervention_outcome: Option<crate::intervention::Outcome>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStage {
    BeforeEdit,
    AfterEdit,
    AfterRestoration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change {
    pub id: String,
    pub prepared_ms: u128,
    pub actor: String,
    pub scope: Scope,
    pub observed: f64,
    pub desired: f64,
    pub models: Vec<Model>,
    pub alternatives: Vec<Alternative>,
    pub patch: Option<BytePatch>,
    pub phase: Phase,
    pub reason: String,
    pub write: Option<WriteReceipt>,
    pub restoration: Option<WriteReceipt>,
    pub observations: Vec<Observation>,
    #[serde(default)]
    pub watches: Vec<crate::effects::Watch>,
    #[serde(default)]
    pub omitted_tracks: usize,
    #[serde(default)]
    pub baseline: Option<Observation>,
    /// A separately reviewed location test. Other locations remain competing
    /// hypotheses, not consenting interpretations or verified controls.
    #[serde(default)]
    pub probe: Option<crate::probes::Probe>,
}

fn agrees_region(target: &dyn NativeTarget, model: &Model) -> Result<()> {
    if let Some(reference) = &model.reference
        && reference.resolve(target)? != model.address
    {
        return Err("The learned reference moved. Recheck findings before another trial.".into());
    }
    let region = target
        .query(model.address)?
        .ok_or("The memory allocation disappeared.")?;
    if !(Chunk {
        address: model.address,
        length: model.before.len(),
        region: model.region.clone(),
    })
    .covered_by(&region)
    {
        return Err("The memory allocation changed.".into());
    }
    Ok(())
}

impl Change {
    pub fn prepare(
        scope: Scope,
        models: Vec<Model>,
        observed: f64,
        desired: f64,
        actor: &str,
    ) -> Result<Self> {
        if models.is_empty()
            || models.len() > 200
            || !observed.is_finite()
            || !desired.is_finite()
            || models.iter().any(|m| {
                !m.mapping.slope.is_finite()
                    || !m.mapping.intercept.is_finite()
                    || m.before.len() != m.mapping.kind.width()
                    || m.address.checked_add(m.before.len() as u64).is_none()
            })
            || (desired - observed).abs() > 20.
            || (desired - observed).abs() <= 1.1
        {
            return Err("A trial needs supported interpretations and a change greater than 1.1 and no more than 20 from the current reading.".into());
        }
        let alternatives: Vec<_> = models
            .iter()
            .map(|m| {
                let proposed = (|| -> Result<Option<BytePatch>> {
                    let c = &m.mapping;
                    if m.before.len() != c.kind.width()
                        || !c.slope.is_finite()
                        || c.slope.abs() < 1e-12
                        || !c.intercept.is_finite()
                    {
                        return Err("Invalid interpretation or incomplete bytes.".into());
                    }
                    let baseline = c
                        .kind
                        .read(&m.before)
                        .map(|v| c.slope * v + c.intercept)
                        .filter(|v| v.is_finite())
                        .ok_or("Unreadable current value.")?;
                    if (baseline - observed).abs() > 0.55 {
                        return Err("The current reading and memory disagree.".into());
                    }
                    let after = c
                        .encode_observation(desired)
                        .ok_or("This interpretation cannot encode the requested value.")?;
                    let prediction = c
                        .kind
                        .read(&after)
                        .map(|v| c.slope * v + c.intercept)
                        .filter(|v| v.is_finite())
                        .ok_or("Unreadable proposed value.")?;
                    if (prediction - desired).abs() > 0.55 {
                        return Err("This encoding misses the requested value.".into());
                    }
                    let Some(first) = m.before.iter().zip(&after).position(|(a, b)| a != b) else {
                        return Ok(None);
                    };
                    let last = m
                        .before
                        .iter()
                        .zip(&after)
                        .rposition(|(a, b)| a != b)
                        .unwrap();
                    Ok(Some(BytePatch {
                        address: m
                            .address
                            .checked_add(first as u64)
                            .ok_or("Address overflow.")?,
                        before: m.before[first..=last].to_vec(),
                        after: after[first..=last].to_vec(),
                    }))
                })();
                match proposed {
                    Ok(patch) => Alternative {
                        index: m.index,
                        patch,
                        error: None,
                    },
                    Err(error) => Alternative {
                        index: m.index,
                        patch: None,
                        error: Some(error),
                    },
                }
            })
            .collect();
        let patch = alternatives
            .first()
            .and_then(|a| a.patch.clone())
            .filter(|p| {
                alternatives
                    .iter()
                    .all(|a| a.error.is_none() && a.patch.as_ref() == Some(p))
            });
        let phase = if patch.is_some() {
            Phase::Prepared
        } else {
            Phase::Unavailable
        };
        let reason=if patch.is_some() {"All remaining interpretations propose the same exact byte change. Gameplay effects are unverified."}
            else {"The remaining interpretations do not agree on one edit. More investigation is needed."}.into();
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            prepared_ms: now_ms(),
            actor: actor.chars().take(40).collect(),
            scope,
            observed,
            desired,
            models,
            alternatives,
            patch,
            phase,
            reason,
            write: None,
            restoration: None,
            observations: vec![],
            watches: vec![],
            omitted_tracks: 0,
            baseline: None,
            probe: None,
        })
    }

    pub fn check_target(&self, target: &dyn NativeTarget, scope: &Scope) -> Result<()> {
        if !target.alive()
            || target.memory_info().pid != self.scope.process.pid
            || target.memory_info().created != self.scope.process.created
            || scope.process.pid != self.scope.process.pid
            || scope.process.created != self.scope.process.created
            || scope.investigation != self.scope.investigation
            || scope.batch != self.scope.batch
            || scope.epoch != self.scope.epoch
            || scope.process.window != self.scope.process.window
        {
            return Err(
                "The original application or investigation changed. This edit cannot be reused."
                    .into(),
            );
        }
        for model in &self.models {
            agrees_region(target, model)?;
        }
        Ok(())
    }
}

/// Independent durable operation records survive losing the UI/session save.
/// Pending intents become uncertain on restart and are never replayed.
pub struct Store {
    directory: PathBuf,
    pub records: Vec<Change>,
    failed: bool,
}
impl Store {
    pub fn open(directory: PathBuf) -> Result<Self> {
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let mut store = Self {
            directory,
            records: vec![],
            failed: false,
        };
        for entry in fs::read_dir(&store.directory).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            if store.records.len() >= MAX_RECORDS
                || fs::metadata(&path).map_err(|e| e.to_string())?.len() > 1024 * 1024
            {
                return Err("Change history exceeded its retained record budget.".into());
            }
            let mut change: Change =
                serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("Could not read change history: {e}"))?;
            if uuid::Uuid::parse_str(&change.id).is_err()
                || path.file_stem().and_then(|s| s.to_str()) != Some(&change.id)
            {
                return Err("Invalid change history identity.".into());
            }
            if matches!(change.phase, Phase::Writing | Phase::Restoring) {
                change.phase = Phase::Uncertain;
                change.reason="Reversebolt stopped during an operation. Its effect is unknown; it was not replayed.".into();
                store.persist(&change)?;
            } else if change.phase == Phase::Prepared {
                change.phase = Phase::Expired;
                change.reason = "The engine restarted. Prepare a new trial.".into();
                store.persist(&change)?;
            }
            store.records.push(change);
        }
        store.records.sort_by_key(|c| c.prepared_ms);
        Ok(store)
    }
    fn persist(&self, change: &Change) -> Result<()> {
        let destination = self.directory.join(format!("{}.json", change.id));
        let temporary = destination.with_extension("pending");
        let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(change).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(temporary, destination).map_err(|e| e.to_string())
    }
    pub fn insert(&mut self, change: Change) -> Result<Change> {
        if self.failed || self.records.len() >= MAX_RECORDS {
            return Err("Change history is unavailable or full; no trial was prepared.".into());
        }
        self.persist(&change)?;
        self.records.push(change.clone());
        Ok(change)
    }
    pub fn record(&mut self, change: Change) -> Result<Change> {
        let index = self
            .records
            .iter()
            .position(|c| c.id == change.id)
            .ok_or("Unknown change.")?;
        // Keep the actual in-memory outcome even when the final disk save fails.
        self.records[index] = change.clone();
        if let Err(error) = self.persist(&change) {
            self.failed = true;
            return Err(format!(
                "Change history could not be saved; further writes are held: {error}"
            ));
        }
        Ok(change)
    }
    pub fn get(&self, id: &str) -> Result<Change> {
        self.records
            .iter()
            .find(|c| c.id == id)
            .cloned()
            .ok_or("Unknown change identity.".into())
    }
    pub fn execute(
        &mut self,
        id: &str,
        target: &dyn NativeTarget,
        scope: &Scope,
        restore: bool,
    ) -> Result<Change> {
        if self.failed {
            return Err("Change history is unavailable; further writes are held.".into());
        }
        let mut change = self.get(id)?;
        // Terminal replay is idempotent. Never retry an uncertain native call.
        if (!restore && change.phase == Phase::Written)
            || (restore && change.phase == Phase::Restored)
        {
            return Ok(change);
        }
        if change.phase
            != if restore {
                Phase::Written
            } else {
                Phase::Prepared
            }
        {
            return Err(
                "This change is not available for that operation. Inspect its recorded outcome."
                    .into(),
            );
        }
        if self.records.iter().any(|c| {
            c.phase.unresolved()
                && c.scope.process.pid == change.scope.process.pid
                && c.scope.process.created == change.scope.process.created
        }) {
            return Err("An earlier write has an uncertain outcome. Further edits to this process are held.".into());
        }
        change.check_target(target, scope)?;
        if !restore
            && (scope.frame != change.scope.frame
                || scope.prediction != change.scope.prediction
                || now_ms()
                    .checked_sub(change.prepared_ms)
                    .is_none_or(|age| age > PLAN_LIFETIME_MS))
        {
            change.phase = Phase::Expired;
            change.reason = "The observation or edit plan expired. Prepare a fresh trial.".into();
            return self.record(change);
        }
        let original = change
            .patch
            .as_ref()
            .ok_or("There is no agreed byte action.")?;
        let patch = if restore {
            BytePatch {
                address: original.address,
                before: original.after.clone(),
                after: original.before.clone(),
            }
        } else {
            original.clone()
        };
        for model in &change.models {
            let mut expected = model.before.clone();
            if restore {
                // Guard the entire interpretation, not only the changed byte.
                // Otherwise a different value with the same edited byte could
                // have its remaining bits silently combined with an old undo.
                for (i, byte) in expected.iter_mut().enumerate() {
                    let address = model.address + i as u64;
                    if address >= original.address
                        && address < original.address + original.after.len() as u64
                    {
                        *byte = original.after[(address - original.address) as usize];
                    }
                }
            }
            if target.read(model.address, expected.len())? != expected {
                if !restore {
                    change.phase = Phase::Refused;
                }
                change.reason =
                    "The value's bytes changed since this edit. Nothing was overwritten.".into();
                let receipt = WriteReceipt::refused(change.reason.clone());
                if restore {
                    change.restoration = Some(receipt);
                } else {
                    change.write = Some(receipt);
                }
                return self.record(change);
            }
        }
        if target.read(patch.address, patch.before.len())? != patch.before {
            // A restoration conflict leaves the original write record available.
            change.reason =
                "The current bytes differ from this edit. Nothing was overwritten.".into();
            if !restore {
                change.phase = Phase::Refused;
            }
            let receipt = WriteReceipt::refused(change.reason.clone());
            if restore {
                change.restoration = Some(receipt);
            } else {
                change.write = Some(receipt);
            }
            return self.record(change);
        }
        change.phase = if restore {
            Phase::Restoring
        } else {
            Phase::Writing
        };
        change.reason = "Write intent recorded. Outcome pending.".into();
        self.record(change.clone())?; // Must reach disk BEFORE entering the backend.
        let receipt = target.guarded_write(&patch, &change.models[0].region);
        let exact = receipt.attempted
            && receipt.error.is_none()
            && receipt.transferred == patch.after.len()
            && receipt.observed_after.as_ref() == Some(&patch.after);
        change.phase = if exact {
            if restore {
                Phase::Restored
            } else {
                Phase::Written
            }
        } else if receipt.attempted {
            Phase::Uncertain
        } else if restore {
            Phase::Written
        } else {
            Phase::Refused
        };
        change.reason=if exact {if restore {"Original bytes were observed again. This does not undo downstream gameplay effects."} else {"Requested bytes were observed after writing. Gameplay effects have not been verified."}}
            else if receipt.attempted {"The write outcome is uncertain. It will not be retried and further edits to this process are held."}
            else {"The backend refused the write before attempting it."}.into();
        if restore {
            change.restoration = Some(receipt)
        } else {
            change.write = Some(receipt)
        }
        self.record(change)
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    /// Historical evidence can be inspected after disconnect/restart, but cannot
    /// grant a new target capability. Only recorded, confined, hashed PNGs load.
    pub fn read_images(
        &self,
        id: &str,
        observation: usize,
    ) -> Result<(Observation, Observation, Vec<Vec<u8>>)> {
        let change = self.get(id)?;
        let baseline = change
            .baseline
            .ok_or("This older trial has no before-image.")?;
        let after = change
            .observations
            .get(observation)
            .cloned()
            .ok_or("Unknown trial observation.")?;
        let root = self.directory.canonicalize().map_err(|e| e.to_string())?;
        let mut images = vec![];
        let mut total = 0;
        for evidence in [&baseline, &after] {
            let path = Path::new(&evidence.image)
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if path.parent() != Some(root.as_path()) || path.extension().is_none_or(|e| e != "png")
            {
                return Err("The trial image is outside its evidence directory.".into());
            }
            let bytes = crate::visual::image_bytes(&path)?;
            if crate::visual::digest(&bytes) != evidence.image_sha256 {
                return Err("The trial image changed and no longer supports this evidence.".into());
            }
            total += bytes.len();
            if total > crate::temporal::MAX_SEQUENCE_BYTES {
                return Err("The trial image pair exceeds its 16 MiB budget.".into());
            }
            images.push(bytes);
        }
        Ok((baseline, after, images))
    }
    /// Failed captures keep their reservation because a native worker may finish
    /// later. Never reuse an image path, including after a process crash.
    pub fn reserve_image(&self, id: &str) -> Result<(PathBuf, PathBuf)> {
        let mut used = 0u64;
        for entry in fs::read_dir(&self.directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            used = used.saturating_add(entry.metadata().map_err(|e| e.to_string())?.len());
            if entry.path().extension().is_some_and(|e| e == "reservation") {
                used = used.saturating_add(crate::visual::MAX_IMAGE_BYTES as u64);
            }
        }
        if used + crate::visual::MAX_IMAGE_BYTES as u64 > 128 * 1024 * 1024 {
            return Err("Trial evidence reached its 128 MiB storage budget.".into());
        }
        let path = self
            .directory
            .join(format!("{id}-{}.png", uuid::Uuid::new_v4()));
        let reservation = path.with_extension("reservation");
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&reservation)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        Ok((path, reservation))
    }
}
