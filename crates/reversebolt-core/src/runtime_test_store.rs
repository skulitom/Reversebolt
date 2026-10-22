//! Bounded durable experiment records and engine-owned evidence paths.
use crate::{
    Result,
    runtime_tests::{FRAMES_PER_CONDITION, MAX_RETAINED_TESTS, MAX_TESTS_PER_INVESTIGATION, Test},
    visual,
};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const RECORD_BYTES: u64 = 512 * 1024;
const TEST_BYTES: u64 = 64 * 1024 * 1024;
const TOTAL_EVIDENCE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, serde::Serialize)]
pub struct Capacity {
    pub investigation_records: usize,
    pub investigation_limit: usize,
    pub retained_records: usize,
    pub retained_limit: usize,
    pub remaining_record_slots: usize,
    pub reason: Option<String>,
}

/// Count actual bytes before reserving another capture. Include earlier tests,
/// incomplete reservations and orphaned evidence left by interrupted work.
/// Only the store's single evidence-directory level is supported; never recurse.
fn evidence_usage(directory: &Path, current: &Path) -> Result<(u64, u64)> {
    let mut total = 0u64;
    let mut this_test = 0u64;
    let mut entries = 0usize;
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        entries += 1;
        if entries > 16_384 {
            return Err("Too many experiment evidence entries.".into());
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err("Experiment evidence contains an unsupported link.".into());
        }
        let size = if kind.is_dir() {
            let mut size = 0u64;
            for item in fs::read_dir(entry.path()).map_err(|e| e.to_string())? {
                let item = item.map_err(|e| e.to_string())?;
                entries += 1;
                if entries > 16_384 {
                    return Err("Too many experiment evidence entries.".into());
                }
                if !item.file_type().map_err(|e| e.to_string())?.is_file() {
                    return Err(
                        "Experiment evidence contains an unsupported nested directory or link."
                            .into(),
                    );
                }
                size = size
                    .checked_add(item.metadata().map_err(|e| e.to_string())?.len())
                    .ok_or("Experiment storage size overflow.")?;
            }
            size
        } else if kind.is_file() {
            entry.metadata().map_err(|e| e.to_string())?.len()
        } else {
            return Err("Unsupported experiment storage entry.".into());
        };
        if entry.path() == current {
            this_test = size;
        }
        total = total
            .checked_add(size)
            .ok_or("Experiment storage size overflow.")?;
    }
    Ok((this_test, total))
}

fn check_evidence_space(
    directory: &Path,
    current: &Path,
    charge: u64,
    per_test: u64,
    total_limit: u64,
) -> Result<()> {
    let (used, total) = evidence_usage(directory, current)?;
    if used.checked_add(charge).is_none_or(|n| n > per_test) {
        return Err("The experiment image budget is exhausted.".into());
    }
    if total.checked_add(charge).is_none_or(|n| n > total_limit) {
        return Err(
            "The retained experiment evidence budget is full. Existing history remains readable."
                .into(),
        );
    }
    Ok(())
}
pub struct Store {
    directory: PathBuf,
    pub records: Vec<Test>,
    pub fault: Option<String>,
}
fn bounded(path: &Path, max: u64) -> Result<Vec<u8>> {
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err("Experiment evidence exceeds its size limit.".into());
    }
    Ok(bytes)
}
impl Store {
    pub fn open(directory: PathBuf) -> Result<Self> {
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let mut store = Self {
            directory,
            records: vec![],
            fault: None,
        };
        for entry in fs::read_dir(&store.directory).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if store.records.len() >= MAX_RETAINED_TESTS {
                return Err("Too many retained experiments.".into());
            }
            let mut record: Test = serde_json::from_slice(&bounded(&path, RECORD_BYTES)?)
                .map_err(|e| e.to_string())?;
            Self::validate(&record)?;
            if store
                .records
                .iter()
                .filter(|r| r.investigation == record.investigation)
                .count()
                >= MAX_TESTS_PER_INVESTIGATION
            {
                return Err("Too many retained experiments for one investigation.".into());
            }
            if path.file_stem().and_then(|s| s.to_str()) != Some(record.id.as_str()) {
                return Err("Experiment file identity mismatch.".into());
            }
            let active = record.active();
            record.recover();
            if active {
                store.write(&record)?;
            }
            store.records.push(record);
        }
        store.records.sort_by_key(|r| r.created_unix_ms);
        Ok(store)
    }
    fn validate(t: &Test) -> Result<()> {
        t.validate_plan()?;
        if t.conditions
            .iter()
            .any(|c| c.frames.len() > FRAMES_PER_CONDITION)
            || t.reason.len() > 4096
        {
            return Err("Experiment record exceeds its bounds.".into());
        }
        for (c, condition) in t.conditions.iter().enumerate() {
            for (i, f) in condition.frames.iter().enumerate() {
                if f.path != format!("{}/{c}-{i}.png", t.id)
                    || f.input_path != format!("{}/{c}-{i}.input.json", t.id)
                    || f.interval.clock_id != t.investigation
                    || f.interval.begin_us > f.interval.end_us
                    || f.started_unix_ms > f.completed_unix_ms
                    || [&f.sha256, &f.input_sha256]
                        .iter()
                        .any(|h| h.len() != 64 || !h.bytes().all(|c| c.is_ascii_hexdigit()))
                {
                    return Err("Invalid retained experiment evidence reference.".into());
                }
            }
        }
        Ok(())
    }
    fn write(&self, t: &Test) -> Result<()> {
        Self::validate(t)?;
        let bytes = serde_json::to_vec_pretty(t).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > RECORD_BYTES {
            return Err("Experiment record exceeds its budget.".into());
        }
        let tmp = self.directory.join(format!("{}.pending", t.id));
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(&bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        drop(f);
        fs::rename(tmp, self.directory.join(format!("{}.json", t.id))).map_err(|e| e.to_string())
    }
    pub fn get(&self, id: &str) -> Result<&Test> {
        self.records
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| "Experiment not found in this session.".into())
    }
    pub fn capacity(&self, investigation: &str) -> Capacity {
        let count = self
            .records
            .iter()
            .filter(|r| r.investigation == investigation)
            .count();
        let remaining = MAX_TESTS_PER_INVESTIGATION
            .saturating_sub(count)
            .min(MAX_RETAINED_TESTS.saturating_sub(self.records.len()));
        let reason = self.fault.clone().or_else(|| {
            if self.records.len() >= MAX_RETAINED_TESTS {
                Some("The retained comparison history is full; existing evidence remains readable.".into())
            } else if count >= MAX_TESTS_PER_INVESTIGATION {
                Some("This investigation has used its eight-comparison budget; earlier results remain available.".into())
            } else { None }
        });
        Capacity {
            investigation_records: count,
            investigation_limit: MAX_TESTS_PER_INVESTIGATION,
            retained_records: self.records.len(),
            retained_limit: MAX_RETAINED_TESTS,
            remaining_record_slots: if reason.is_some() { 0 } else { remaining },
            reason,
        }
    }
    pub fn ensure_room(&self, investigation: &str) -> Result<()> {
        self.capacity(investigation).reason.map_or(Ok(()), Err)
    }
    pub fn record(&mut self, t: Test) -> Result<()> {
        if let Some(fault) = &self.fault {
            return Err(fault.clone());
        }
        let existing = self.records.iter().position(|r| r.id == t.id);
        if let Some(i) = existing {
            if self.records[i].investigation != t.investigation {
                return Err("A saved comparison cannot move to a different investigation.".into());
            }
        } else {
            self.ensure_room(&t.investigation)?;
        }
        if let Err(error) = self.write(&t) {
            self.fault = Some(format!(
                "Experiment storage failed. Further experiments are held: {error}"
            ));
            return Err(self.fault.clone().unwrap());
        }
        if let Some(i) = existing {
            self.records[i] = t;
        } else {
            self.records.push(t);
        }
        Ok(())
    }
    pub fn reserve(&self, t: &Test, c: usize, i: usize) -> Result<(PathBuf, PathBuf)> {
        Self::validate(t)?;
        if !t.pending_capture || c >= 3 || i >= FRAMES_PER_CONDITION {
            return Err("Save the capture intent before reserving evidence.".into());
        }
        let folder = self.directory.join(&t.id);
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        check_evidence_space(
            &self.directory,
            &folder,
            visual::MAX_IMAGE_BYTES as u64 + 1024 * 1024 + 2 * RECORD_BYTES + 65_536,
            TEST_BYTES,
            TOTAL_EVIDENCE_BYTES,
        )?;
        let reservation = folder.join(format!("{c}-{i}.reserved"));
        let f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(reservation)
            .map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        let image = folder.join(format!("{c}-{i}.png"));
        let input = folder.join(format!("{c}-{i}.input.json"));
        if image.exists() || input.exists() {
            return Err("Evidence slot already exists; never recapture it.".into());
        }
        Ok((image, input))
    }
    pub fn read_image(&self, t: &Test, c: usize, i: usize) -> Result<Vec<u8>> {
        Self::validate(t)?;
        let f = t
            .conditions
            .get(c)
            .and_then(|c| c.frames.get(i))
            .ok_or("Choose a retained experiment frame.")?;
        let bytes = visual::image_bytes(&self.directory.join(&f.path))?;
        if visual::digest(&bytes) != f.sha256 {
            return Err("Experiment image checksum changed.".into());
        }
        Ok(bytes)
    }
    /// Intent/outcome pairs remain readable even when input failed before an image.
    pub fn read_executions(&self, t: &Test) -> Result<Vec<serde_json::Value>> {
        Self::validate(t)?;
        let mut entries = vec![];
        for c in 0..3 {
            for i in 0..FRAMES_PER_CONDITION {
                let plan = self
                    .directory
                    .join(&t.id)
                    .join(format!("{c}-{i}.action.json"));
                if !plan.exists() {
                    continue;
                }
                let bytes = bounded(&plan, 16_384)?;
                let value: serde_json::Value =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                let action = i.checked_sub(2).and_then(|n| t.actions.get(n));
                if value["schema"] != 1
                    || value["test_id"] != t.id
                    || value["condition"] != c
                    || value["frame"] != i
                    || value["target"]["window"] != t.window
                    || value["target"]["session"] != t.scope.session
                    || value["action"] != serde_json::json!(action)
                {
                    return Err("Retained input intent differs from the comparison plan.".into());
                }
                let outcome = plan.with_extension("outcome.json");
                let result = if outcome.exists() {
                    let bytes = bounded(&outcome, 16_384)?;
                    let receipt: serde_json::Value =
                        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                    if receipt["plan_sha256"] != visual::digest(&bounded(&plan, 16_384)?) {
                        return Err("Input outcome belongs to a different saved intent.".into());
                    }
                    serde_json::json!({"sha256":visual::digest(&bytes),"receipt":receipt})
                } else {
                    serde_json::Value::Null
                };
                entries.push(serde_json::json!({"condition":c,"frame":i,"plan":value,"plan_sha256":visual::digest(&bytes),"outcome":result}));
            }
        }
        Ok(entries)
    }
    pub fn read_input(
        &self,
        t: &Test,
        c: usize,
        i: usize,
    ) -> Result<Option<crate::inputs::Window>> {
        Self::validate(t)?;
        let f = t
            .conditions
            .get(c)
            .and_then(|c| c.frames.get(i))
            .ok_or("Choose a retained experiment frame.")?;
        let bytes = bounded(&self.directory.join(&f.input_path), 1024 * 1024)?;
        if visual::digest(&bytes) != f.input_sha256 {
            return Err("Experiment input checksum changed.".into());
        }
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_quota_counts_prior_and_interrupted_evidence_before_capture() {
        let directory =
            std::env::temp_dir().join(format!("reversebolt-quota-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        let current = directory.join("current");
        for name in ["current", "previous", "interrupted"] {
            fs::create_dir(directory.join(name)).unwrap();
        }
        fs::write(current.join("image.png"), [0; 10]).unwrap();
        fs::write(directory.join("previous/image.png"), [0; 20]).unwrap();
        fs::write(directory.join("interrupted/pending.input.json"), [0; 15]).unwrap();
        fs::write(directory.join("record.pending"), [0; 5]).unwrap();
        assert_eq!(evidence_usage(&directory, &current).unwrap(), (10, 50));
        // Small injected limits exercise the same filesystem check as reserve,
        // without allocating a half-gigabyte test file.
        check_evidence_space(&directory, &current, 50, 64, 100).unwrap();
        assert!(
            check_evidence_space(&directory, &current, 51, 64, 100)
                .unwrap_err()
                .contains("retained")
        );
        assert!(
            check_evidence_space(&directory, &current, 55, 64, 200)
                .unwrap_err()
                .contains("image budget")
        );
        assert_eq!(evidence_usage(&directory, &current).unwrap(), (10, 50));
        // Unsupported nested content cannot silently disappear from accounting.
        fs::create_dir(directory.join("interrupted/nested")).unwrap();
        assert!(evidence_usage(&directory, &current).is_err());
        let resolved = directory.canonicalize().unwrap();
        assert_eq!(
            resolved.parent().unwrap(),
            std::env::temp_dir().canonicalize().unwrap()
        );
        assert!(
            resolved
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("reversebolt-quota-")
        );
        fs::remove_dir_all(resolved).unwrap();
    }
}
