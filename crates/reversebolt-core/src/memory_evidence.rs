//! Immutable local evidence for offline memory research, never a live binding.
use crate::{
    Result,
    coverage::Chunk,
    live::{LiveView, MemoryRead},
    visual,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

pub const SCHEMA: u32 = 1;
pub const MAX_STORE_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_MANIFEST_BYTES: usize = 32 * 1024 * 1024;
pub const LIMITATION: &str = "Historical retained memory from the existing sampling plan, not whole-process coverage. Range selection may already depend on screen values; consult state.value_searches and state.change_searches. Missing reads stay missing. Mapping metadata does not prove allocation identity or atomic snapshots. Image/input clocks describe separate operations and may not cover memory reads. No current address, semantic meaning or edit authority is established.";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Segment {
    pub offset: u64,
    pub length: usize,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReadRecord {
    pub receipt: MemoryRead,
    pub segment: Option<Segment>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub number: usize,
    pub memory_sample: bool,
    pub reads: Vec<ReadRecord>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub state: LiveView,
    pub plan: Vec<Chunk>,
    pub invalid_chunks: Vec<u64>,
    pub samples: Vec<Sample>,
    pub payload_bytes: u64,
    pub limitation: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Export {
    pub directory: String,
    pub manifest_sha256: String,
    pub payload_bytes: u64,
    pub captures: usize,
    pub memory_samples: usize,
    pub replayed: bool,
    pub historical: bool,
}

pub(crate) fn address(value: &str) -> Result<u64> {
    u64::from_str_radix(
        value
            .strip_prefix("0x")
            .ok_or("Invalid memory evidence address")?,
        16,
    )
    .map_err(|_| "Invalid memory evidence address".into())
}
pub(crate) fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("Memory evidence document exceeds its limit".into());
    }
    Ok(bytes)
}
impl Manifest {
    pub fn validate(&self) -> Result<()> {
        let numbers = self
            .state
            .memory_sample_numbers
            .clone()
            .unwrap_or_else(|| (1..=self.state.sample_count).collect());
        if self.schema != SCHEMA
            || self.samples.len() != self.state.sample_count
            || self.samples.is_empty()
            || self.samples.len() > crate::live::MAX_SAMPLES
            || numbers.is_empty()
            || numbers.len() > crate::live::MAX_MEMORY_SAMPLES
            || numbers.windows(2).any(|p| p[0] >= p[1])
            || numbers.iter().any(|n| *n == 0 || *n > self.samples.len())
            || self.payload_bytes > MAX_STORE_BYTES
            || self.plan.len() > crate::value_search::MAX_WINDOWS.max(2048)
        {
            return Err("Invalid memory evidence scope or sample budget".into());
        }
        let mut plan = BTreeMap::new();
        for chunk in &self.plan {
            if chunk.length == 0
                || chunk.length > crate::coverage::CHUNK_BYTES
                || chunk.address.checked_add(chunk.length as u64).is_none()
                || !chunk.covered_by(&chunk.region)
                || plan.insert(chunk.address, chunk).is_some()
            {
                return Err("Invalid memory evidence sampling plan".into());
            }
        }
        if plan
            .values()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|p| p[0].address + p[0].length as u64 > p[1].address)
            || self.invalid_chunks.windows(2).any(|p| p[0] >= p[1])
            || self.invalid_chunks.iter().any(|a| !plan.contains_key(a))
        {
            return Err("Overlapping or inconsistent memory evidence ranges".into());
        }
        let mut offset = 0u64;
        for (index, sample) in self.samples.iter().enumerate() {
            let memory = numbers.contains(&(index + 1));
            if sample.number != index + 1
                || sample.memory_sample != memory
                || sample.reads.len() != if memory { plan.len() } else { 0 }
            {
                return Err("Memory evidence sample membership changed".into());
            }
            let mut seen = std::collections::BTreeSet::new();
            for read in &sample.reads {
                let address = address(&read.receipt.address)?;
                let chunk = plan
                    .get(&address)
                    .ok_or("Read lies outside the recorded memory plan")?;
                if !seen.insert(address)
                    || read.receipt.requested != chunk.length
                    || read.receipt.begin_ms > read.receipt.end_ms
                    || read.receipt.capture_interval.as_ref().is_none_or(|t| {
                        t.clock_id != self.state.investigation_id || t.begin_us > t.end_us
                    })
                {
                    return Err("Invalid memory read receipt".into());
                }
                match &read.segment {
                    Some(segment) => {
                        if segment.offset != offset
                            || segment.length != chunk.length
                            || read.receipt.retained != segment.length
                            || read.receipt.disposition != "retained_generation_unverified"
                            || segment.sha256.len() != 64
                            || !segment.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                        {
                            return Err("Memory byte segment differs from its receipt".into());
                        }
                        offset = offset
                            .checked_add(segment.length as u64)
                            .ok_or("Memory evidence size overflow")?;
                    }
                    None if read.receipt.retained == 0
                        && read.receipt.disposition != "retained_generation_unverified" => {}
                    None => {
                        return Err(
                            "Missing bytes were presented as a successful memory read".into()
                        );
                    }
                }
            }
        }
        if offset != self.payload_bytes || offset != self.state.retained_bytes as u64 {
            return Err("Memory evidence payload size differs from retained observations".into());
        }
        Ok(())
    }
}

pub struct Archive {
    manifest: Manifest,
    manifest_sha256: String,
    payload: PathBuf,
}

/// One planned chunk across memory captures, not including image-only moments.
/// Missing reads remain absent. No allocation continuity is implied.
pub struct ChunkHistory {
    pub chunk: Chunk,
    pub captures: Vec<usize>,
    pub bytes: Vec<Option<Vec<u8>>>,
}
impl Archive {
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn open(directory: &Path, expected: &str) -> Result<Self> {
        let bytes = read_bounded(&directory.join("manifest.json"), MAX_MANIFEST_BYTES)?;
        if visual::digest(&bytes) != expected {
            return Err("Memory manifest checksum changed".into());
        }
        let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        manifest.validate()?;
        let archive = Self {
            manifest,
            manifest_sha256: expected.into(),
            payload: directory.join("bytes.bin"),
        };
        archive.file()?;
        Ok(archive)
    }
    fn file(&self) -> Result<fs::File> {
        let file = fs::File::open(&self.payload).map_err(|e| e.to_string())?;
        if file.metadata().map_err(|e| e.to_string())?.len() != self.manifest.payload_bytes {
            return Err("Memory payload is incomplete or has trailing bytes".into());
        }
        Ok(file)
    }
    fn bytes(file: &mut fs::File, segment: &Segment) -> Result<Vec<u8>> {
        file.seek(SeekFrom::Start(segment.offset))
            .map_err(|e| e.to_string())?;
        let mut bytes = vec![0; segment.length];
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        if visual::digest(&bytes) != segment.sha256 {
            return Err("Memory byte checksum changed".into());
        }
        Ok(bytes)
    }
    pub fn verify(&self) -> Result<()> {
        let mut file = self.file()?;
        for read in self.manifest.samples.iter().flat_map(|s| &s.reads) {
            if let Some(segment) = &read.segment {
                Self::bytes(&mut file, segment)?;
            }
        }
        Ok(())
    }
    /// Stream verified histories with at most 20 x 64 KiB of payload in memory.
    /// Every retained segment is checked, including chunks the visitor excludes.
    /// A visitor must discard its partial result if this method returns an error.
    pub fn visit_memory_chunks(
        &self,
        mut visit: impl FnMut(&ChunkHistory) -> Result<()>,
    ) -> Result<()> {
        let mut file = self.file()?;
        let samples: Vec<_> = self
            .manifest
            .samples
            .iter()
            .filter(|s| s.memory_sample)
            .collect();
        let mut segments = BTreeMap::new();
        for sample in &samples {
            for read in &sample.reads {
                if let Some(segment) = &read.segment {
                    segments.insert((address(&read.receipt.address)?, sample.number), segment);
                }
            }
        }
        let captures: Vec<_> = samples.iter().map(|s| s.number).collect();
        for chunk in &self.manifest.plan {
            let bytes = captures
                .iter()
                .map(|number| {
                    segments
                        .get(&(chunk.address, *number))
                        .map(|segment| Self::bytes(&mut file, segment))
                        .transpose()
                })
                .collect::<Result<Vec<_>>>()?;
            visit(&ChunkHistory {
                chunk: chunk.clone(),
                captures: captures.clone(),
                bytes,
            })?;
        }
        // Reject a concurrently truncated/extended payload before publishing a result.
        self.file()?;
        Ok(())
    }
    /// One bounded historical sample. Missing reads are absent, never zero-filled.
    pub fn sample(&self, number: usize) -> Result<BTreeMap<u64, Vec<u8>>> {
        let sample = self
            .manifest
            .samples
            .iter()
            .find(|s| s.number == number)
            .ok_or("Unknown memory sample")?;
        let mut file = self.file()?;
        let mut bytes = BTreeMap::new();
        for read in &sample.reads {
            if let Some(segment) = &read.segment {
                bytes.insert(
                    address(&read.receipt.address)?,
                    Self::bytes(&mut file, segment)?,
                );
            }
        }
        Ok(bytes)
    }
}
