//! Bounded durable experiment records and engine-owned evidence paths.
use crate::{
    Result,
    runtime_tests::{FRAMES_PER_CONDITION, MAX_TESTS, Test},
    visual,
};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const RECORD_BYTES: u64 = 512 * 1024;
const TEST_BYTES: u64 = 64 * 1024 * 1024;
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
            if store.records.len() >= MAX_TESTS {
                return Err("Too many retained experiments.".into());
            }
            let mut record: Test = serde_json::from_slice(&bounded(&path, RECORD_BYTES)?)
                .map_err(|e| e.to_string())?;
            Self::validate(&record)?;
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
    pub fn record(&mut self, t: Test) -> Result<()> {
        if let Some(fault) = &self.fault {
            return Err(fault.clone());
        }
        let existing = self.records.iter().position(|r| r.id == t.id);
        if existing.is_none() && self.records.len() >= MAX_TESTS {
            return Err("The eight-experiment evidence budget is full.".into());
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
        let used = fs::read_dir(&folder)
            .map_err(|e| e.to_string())?
            .try_fold(0u64, |sum, e| {
                let n = e
                    .map_err(|e| e.to_string())?
                    .metadata()
                    .map_err(|e| e.to_string())?
                    .len();
                sum.checked_add(n)
                    .ok_or_else(|| "Evidence size overflow.".to_string())
            })?;
        if used + visual::MAX_IMAGE_BYTES as u64 + 1024 * 1024 > TEST_BYTES {
            return Err("The experiment image budget is exhausted.".into());
        }
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
