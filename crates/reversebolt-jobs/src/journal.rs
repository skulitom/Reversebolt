//! Bounded append-only, checksummed full checkpoints. No rename/compaction race.
use crate::Snapshot;
use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

pub const MAX_RECORD: usize = 2 * 1024 * 1024;
pub const MAX_LOG: u64 = 64 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"RBJOB001";
const END: &[u8; 8] = b"COMMIT01";
const HEADER: usize = 28;

pub trait Journal {
    fn load(&self) -> Option<Snapshot>;
    /// Success means write_all AND sync_all completed. Any error may be ambiguous.
    /// Ordinary writes reserve two maximum records for outcome/recovery.
    fn append(&mut self, state: &Snapshot, reserve_records: usize) -> Result<(), String>;
}
pub struct FileJournal {
    file: File,
    _lock: File,
    last: Option<Snapshot>,
    pub repaired_tail_bytes: u64,
    bytes: u64,
    ceiling: u64,
}

pub fn encode(state: &Snapshot) -> Result<Vec<u8>, String> {
    state.validate()?;
    let payload = serde_json::to_vec(state).map_err(|e| e.to_string())?;
    if payload.len() > MAX_RECORD {
        return Err("checkpoint exceeds record limit".into());
    }
    let mut record = Vec::with_capacity(HEADER + payload.len() + END.len());
    record.extend_from_slice(MAGIC);
    record.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    record.extend_from_slice(&state.revision.to_le_bytes());
    // Include length and sequence in checksum so corrupt framing is detected.
    record.extend_from_slice(&crc32fast::hash(&record).to_le_bytes());
    record.extend_from_slice(&crc32fast::hash(&payload).to_le_bytes());
    record.extend_from_slice(&payload);
    record.extend_from_slice(END);
    Ok(record)
}

impl FileJournal {
    pub fn open(directory: &Path) -> Result<Self, String> {
        Self::with_ceiling(directory, MAX_LOG)
    }
    pub fn with_ceiling(directory: &Path, ceiling: u64) -> Result<Self, String> {
        if !(4 * MAX_RECORD as u64..=MAX_LOG).contains(&ceiling) {
            return Err("invalid journal ceiling".into());
        }
        std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("owner.lock"))
            .map_err(|e| e.to_string())?;
        lock.try_lock_exclusive()
            .map_err(|_| "journal already owned".to_string())?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("jobs.wal"))
            .map_err(|e| e.to_string())?;
        let length = file.metadata().map_err(|e| e.to_string())?.len();
        if length > ceiling {
            return Err("journal exceeds disk budget".into());
        }
        let mut good = 0;
        let mut last: Option<Snapshot> = None;
        while good < length {
            if length - good < HEADER as u64 {
                break;
            }
            let mut header = [0u8; HEADER];
            file.read_exact(&mut header).map_err(|e| e.to_string())?;
            if &header[..8] != MAGIC {
                return Err(format!("invalid record magic at {good}"));
            }
            if crc32fast::hash(&header[..20])
                != u32::from_le_bytes(header[20..24].try_into().unwrap())
            {
                return Err("corrupt record header".into());
            }
            let size = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
            let revision = u64::from_le_bytes(header[12..20].try_into().unwrap());
            if size > MAX_RECORD {
                return Err("invalid record length".into());
            }
            let total = HEADER + size + END.len();
            if length - good < total as u64 {
                break;
            }
            let mut payload = vec![0; size];
            file.read_exact(&mut payload).map_err(|e| e.to_string())?;
            let mut end = [0u8; 8];
            file.read_exact(&mut end).map_err(|e| e.to_string())?;
            let expected = u32::from_le_bytes(header[24..28].try_into().unwrap());
            if &end != END || crc32fast::hash(&payload) != expected {
                return Err(format!(
                    "corrupt committed record at {good}; manual investigation required"
                ));
            }
            let state: Snapshot =
                serde_json::from_slice(&payload).map_err(|e| format!("invalid checkpoint: {e}"))?;
            state.validate()?;
            if state.revision != revision
                || revision != last.as_ref().map_or(1, |s| s.revision + 1)
                || last.as_ref().is_some_and(|s| {
                    s.session != state.session
                        || s.event_head > state.event_head
                        || s.boot > state.boot
                        || s.next_job > state.next_job
                        || s.next_operation > state.next_operation
                        || s.generation > state.generation
                        || s.limits != state.limits
                        || s.clients.iter().any(|(id, old)| {
                            state.clients.get(id).is_none_or(|new| {
                                old.role != new.role || old.high_water > new.high_water
                            })
                        })
                })
            {
                return Err("non-monotonic or incompatible checkpoint".into());
            }
            last = Some(state);
            good += total as u64;
        }
        let repaired_tail_bytes = length - good;
        if repaired_tail_bytes > 0 {
            // Only an incomplete final frame is discarded. Complete invalid
            // records are never skipped, even if later records look valid.
            file.set_len(good).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
        }
        file.seek(SeekFrom::Start(good))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            file,
            _lock: lock,
            last,
            repaired_tail_bytes,
            bytes: good,
            ceiling,
        })
    }
}
impl Journal for FileJournal {
    fn load(&self) -> Option<Snapshot> {
        self.last.clone()
    }
    fn append(&mut self, state: &Snapshot, reserve_records: usize) -> Result<(), String> {
        if state.revision != self.last.as_ref().map_or(1, |s| s.revision + 1) {
            return Err("journal revision conflict".into());
        }
        let record = encode(state)?;
        let reserve = reserve_records as u64 * (MAX_RECORD + HEADER + END.len()) as u64;
        if self.bytes + record.len() as u64 + reserve > self.ceiling {
            return Err("journal budget exhausted; archive/rotate explicitly".into());
        }
        self.file.write_all(&record).map_err(|e| e.to_string())?;
        self.file.sync_all().map_err(|e| e.to_string())?;
        self.bytes += record.len() as u64;
        self.last = Some(state.clone());
        Ok(())
    }
}
