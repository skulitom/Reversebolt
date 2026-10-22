use super::*;
use crate::memory_evidence::{self as m, Export, Manifest, ReadRecord, Sample, Segment};
use std::{fs, io::Write};

#[derive(Serialize, Deserialize)]
struct Reservation {
    investigation: String,
    batch: usize,
    captures: usize,
    bytes: u64,
    manifest_sha256: String,
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}
impl LiveSession {
    /// Freeze already collected bytes once per batch. No NativeTarget method is called.
    pub fn export_memory_evidence(
        &self,
        investigation: &str,
        batch: usize,
        after_sample: usize,
    ) -> Result<Export> {
        self.export_memory_evidence_with_cancel(investigation, batch, after_sample, || false)
    }
    pub fn export_memory_evidence_with_cancel(
        &self,
        investigation: &str,
        batch: usize,
        after_sample: usize,
        cancelled: impl Fn() -> bool,
    ) -> Result<Export> {
        if cancelled() {
            return Err("Memory evidence export cancelled".into());
        }
        if investigation != self.view.investigation_id
            || batch != self.view.coverage.batch
            || after_sample != self.view.sample_count
            || after_sample == 0
            || batch == 0
            || batch > coverage::MAX_BATCHES
        {
            return Err(
                "Use the exact current investigation, memory batch and sample count".into(),
            );
        }
        let root = self.root_directory.join("memory-evidence");
        let directory = root.join(format!("batch-{batch:03}"));
        let reservation = directory.join("reservation.json");
        let mut charged = 0u64;
        let mut prior = None;
        for number in 1..=coverage::MAX_BATCHES {
            let path = root.join(format!("batch-{number:03}/reservation.json"));
            if !path.try_exists().map_err(|e| e.to_string())? {
                continue;
            }
            let record: Reservation = serde_json::from_slice(&m::read_bounded(&path, 4096)?)
                .map_err(|e| e.to_string())?;
            if record.investigation != investigation
                || record.batch != number
                || record.captures == 0
                || record.captures > MAX_SAMPLES
                || record.bytes == 0
                || record.bytes > m::MAX_STORE_BYTES
                || record.manifest_sha256.len() != 64
                || !record
                    .manifest_sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit())
            {
                return Err("Memory export reservation changed scope or size".into());
            }
            charged = charged
                .checked_add(record.bytes)
                .ok_or("Memory export size overflow")?;
            if number == batch {
                prior = Some(record);
            }
        }
        if charged > m::MAX_STORE_BYTES {
            return Err("Memory export storage budget exceeded".into());
        }
        if let Some(prior) = prior {
            if prior.captures != after_sample {
                return Err(
                    "This batch already has a frozen export at another sample count".into(),
                );
            }
            let archive = m::Archive::open(&directory, &prior.manifest_sha256)?;
            if archive.manifest().state.investigation_id != investigation
                || archive.manifest().state.coverage.batch != batch
                || archive.manifest().state.sample_count != after_sample
                || archive.manifest().payload_bytes
                    + fs::metadata(directory.join("manifest.json"))
                        .map_err(|e| e.to_string())?
                        .len()
                    != prior.bytes
            {
                return Err("Frozen memory export scope changed".into());
            }
            archive.verify_with_cancel(&cancelled)?;
            return Ok(Export {
                directory: directory.to_string_lossy().into(),
                manifest_sha256: prior.manifest_sha256,
                payload_bytes: archive.manifest().payload_bytes,
                captures: after_sample,
                memory_samples: archive.manifest().state.memory_sample_count(),
                replayed: true,
                historical: true,
            });
        }
        if self.snapshots.len() != after_sample || self.receipts.len() != after_sample {
            return Err("Memory capture history is incomplete".into());
        }
        let memory = self
            .view
            .memory_sample_numbers
            .clone()
            .unwrap_or_else(|| (1..=after_sample).collect());
        let mut parts = vec![];
        let mut samples = vec![];
        let mut offset = 0u64;
        for (index, (snapshot, receipts)) in self.snapshots.iter().zip(&self.receipts).enumerate() {
            let mut reads = vec![];
            for receipt in receipts {
                if cancelled() {
                    return Err("Memory evidence export cancelled".into());
                }
                let bytes = snapshot.get(&m::address(&receipt.address)?);
                let segment = bytes.map(|bytes| {
                    let segment = Segment {
                        offset,
                        length: bytes.len(),
                        sha256: visual::digest(bytes),
                    };
                    offset += bytes.len() as u64;
                    parts.push(bytes.as_slice());
                    segment
                });
                reads.push(ReadRecord {
                    receipt: receipt.clone(),
                    segment,
                });
            }
            samples.push(Sample {
                number: index + 1,
                memory_sample: memory.contains(&(index + 1)),
                reads,
            });
        }
        let manifest = Manifest {
            schema: m::SCHEMA,
            state: self.view.clone(),
            plan: self.plan.clone(),
            invalid_chunks: self.invalid_chunks.iter().copied().collect(),
            samples,
            payload_bytes: offset,
            limitation: m::LIMITATION.into(),
        };
        manifest.validate()?;
        let bytes = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
        let total = offset
            .checked_add(bytes.len() as u64)
            .ok_or("Memory export size overflow")?;
        if bytes.len() > m::MAX_MANIFEST_BYTES || total > m::MAX_STORE_BYTES.saturating_sub(charged)
        {
            return Err("The complete export does not fit the 512 MiB local evidence budget; no bytes were exported".into());
        }
        let reference = visual::digest(&bytes);
        if cancelled() {
            return Err("Memory evidence export cancelled".into());
        }
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        write_new(
            &reservation,
            &serde_json::to_vec(&Reservation {
                investigation: investigation.into(),
                batch,
                captures: after_sample,
                bytes: total,
                manifest_sha256: reference.clone(),
            })
            .map_err(|e| e.to_string())?,
        )?;
        let mut payload = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("bytes.bin"))
            .map_err(|e| e.to_string())?;
        for part in parts {
            if cancelled() {
                return Err(
                    "Memory evidence export cancelled; partial export remains reserved".into(),
                );
            }
            payload.write_all(part).map_err(|e| e.to_string())?;
        }
        payload.sync_all().map_err(|e| e.to_string())?;
        if cancelled() {
            return Err("Memory evidence export cancelled; partial export remains reserved".into());
        }
        write_new(&directory.join("manifest.json"), &bytes)?;
        m::Archive::open(&directory, &reference)?.verify_with_cancel(cancelled)?;
        Ok(Export {
            directory: directory.to_string_lossy().into(),
            manifest_sha256: reference,
            payload_bytes: offset,
            captures: after_sample,
            memory_samples: memory.len(),
            replayed: false,
            historical: true,
        })
    }
}
