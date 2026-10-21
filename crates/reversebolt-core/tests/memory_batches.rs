//! Integration fixtures supply labels; these tests prove sampling, not semantics.
use reversebolt_core::{
    Result,
    coverage::{CHUNK_BYTES, MAX_READ_BYTES},
    live::*,
};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    rc::Rc,
};
const MIB: usize = 1024 * 1024;
const BASE: u64 = 0x10000;
const FIELD: u64 = BASE + MIB as u64 + 128;

struct Fixture {
    info: ProcessInfo,
    frame: Cell<usize>,
    drift: Rc<Cell<bool>>,
    reads: Rc<Cell<usize>>,
}
impl NativeTarget for Fixture {
    fn info(&self) -> &ProcessInfo {
        &self.info
    }
    fn alive(&self) -> bool {
        true
    }
    fn regions(&self) -> Result<Vec<Region>> {
        Ok(vec![Region {
            base: BASE,
            length: 3 * MIB,
            allocation_base: BASE,
            protection: 4,
            kind: 0x20000,
            image: false,
        }])
    }
    fn query(&self, address: u64) -> Result<Option<Region>> {
        let mut region = self.regions()?.remove(0);
        region.base = address;
        region.length = (BASE + 3 * MIB as u64 - address) as usize;
        if self.drift.get() && address <= FIELD && FIELD < address + CHUNK_BYTES as u64 {
            region.protection = 8;
        }
        Ok(Some(region))
    }
    fn read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        self.reads.set(self.reads.get() + 1);
        let mut bytes = vec![0; length];
        if address <= FIELD && FIELD + 4 <= address + length as u64 {
            let offset = (FIELD - address) as usize;
            bytes[offset..offset + 4]
                .copy_from_slice(&((self.frame.get() * 7 + 13) as i32).to_le_bytes());
        }
        Ok(bytes)
    }
    fn screen(&self, path: &Path) -> Result<Screen> {
        self.frame.set(self.frame.get() + 1);
        // File-binding fixture, not an image recognition or decoding test.
        let mut bytes = b"\x89PNG\r\n\x1a\nfixture".to_vec();
        bytes.extend(self.frame.get().to_le_bytes());
        std::fs::write(path, bytes).unwrap();
        Ok(Screen {
            sampling: None,
            path: path.to_string_lossy().into_owned(),
            width: 100,
            height: 100,
            text: String::new(),
            words: vec![Word {
                text: (self.frame.get() * 7 + 13).to_string(),
                bounds: [10., 10., 30., 20.],
                line: 0,
                source: TextSource::WindowsOcr,
            }],
            warning: None,
            method: "fixture".into(),
        })
    }
}
fn session() -> (LiveSession, PathBuf, Rc<Cell<bool>>, Rc<Cell<usize>>) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/batch-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let drift = Rc::new(Cell::new(false));
    let reads = Rc::new(Cell::new(0));
    let target = Fixture {
        info: ProcessInfo::default(),
        frame: Cell::new(0),
        drift: drift.clone(),
        reads: reads.clone(),
    };
    (
        LiveSession::attach(Box::new(target), 1, path.clone()).unwrap(),
        path,
        drift,
        reads,
    )
}
fn sample(s: &mut LiveSession) {
    for _ in 0..12 {
        s.capture().unwrap();
    }
    s.correlate().unwrap();
}

#[test]
fn later_memory_is_discovered_without_stitching_batches_and_witnesses_survive() {
    let (mut s, path, _, _) = session();
    let id = s.view.investigation_id.clone();
    sample(&mut s);
    assert!(s.view.tracks.iter().all(|t| t.matches.is_empty()));
    let old = s.view.frames[0].id.clone();
    s.advance_memory(&id, 1).unwrap();
    assert_eq!(s.view.sample_count, 0);
    assert_eq!(s.view.retained_bytes, 0);
    assert!(s.read_frame(&id, &old).is_err());
    assert!(s.advance_memory(&id, 1).is_err());
    assert!(s.correlate().is_err());
    sample(&mut s);
    let address = format!("0x{FIELD:X}");
    assert!(
        s.view
            .tracks
            .iter()
            .any(|t| t.matches.iter().any(|m| m.address == address))
    );
    assert_eq!(s.view.coverage.read_unique_bytes, 2 * MIB as u64);
    assert_eq!(s.view.coverage.complete_trajectory_bytes, 2 * MIB as u64);
    s.advance_memory(&id, 2).unwrap();
    let archive: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path.join("batch-002/memory-batch.json")).unwrap())
            .unwrap();
    let witness = archive["witnesses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["candidate"]["address"] == address)
        .unwrap();
    assert_eq!(witness["bytes"].as_array().unwrap().len(), 12);
    assert_eq!(witness["reads"].as_array().unwrap().len(), 12);
    let first: Vec<u8> = serde_json::from_value(witness["bytes"][0].clone()).unwrap();
    assert_eq!(i32::from_le_bytes(first.try_into().unwrap()), 104);
    assert!(
        witness["continuity"]
            .as_str()
            .unwrap()
            .contains("unverified")
    );
    s.capture().unwrap();
    assert_eq!(s.view.coverage.selected_unique_bytes, 3 * MIB as u64);
    assert_eq!(s.view.coverage.read_unique_bytes, 3 * MIB as u64);
    assert!(
        s.advance_memory(&id, 3)
            .unwrap_err()
            .contains("All currently inventoried")
    );
    assert_eq!(s.view.coverage.batch, 3);
    assert_eq!(s.view.retained_bytes, MIB);
}

#[test]
fn archive_failure_preserves_the_live_batch_and_stale_requests_cannot_advance() {
    let (mut s, path, _, _) = session();
    let id = s.view.investigation_id.clone();
    s.capture().unwrap();
    let before = serde_json::to_value(&s.view).unwrap();
    std::fs::create_dir(path.join("memory-batch.json")).unwrap();
    assert!(s.advance_memory(&id, 1).is_err());
    assert_eq!(before, serde_json::to_value(&s.view).unwrap());
    assert!(s.advance_memory("other", 1).is_err());
    std::fs::remove_dir(path.join("memory-batch.json")).unwrap();
    s.advance_memory(&id, 1).unwrap();
    assert_eq!(s.view.coverage.batches.len(), 1);
    assert!(path.join("frame-001.png").exists());
}

#[test]
fn observed_mapping_change_breaks_the_whole_trajectory_even_after_it_returns() {
    let (mut s, _, drift, _) = session();
    let id = s.view.investigation_id.clone();
    s.capture().unwrap();
    s.advance_memory(&id, 1).unwrap();
    for n in 0..12 {
        drift.set(n == 4);
        s.capture().unwrap();
    }
    s.correlate().unwrap();
    assert!(s.view.tracks.iter().all(|t| t.matches.is_empty()));
    assert_eq!(
        s.view.coverage.complete_trajectory_bytes,
        (MIB - CHUNK_BYTES) as u64
    );
    assert_eq!(
        s.view.latest.as_ref().unwrap().changed_mapping_bytes,
        CHUNK_BYTES
    );
    assert!(s.view.coverage.changed_or_unavailable_bytes >= 8 * CHUNK_BYTES as u64);
}

#[test]
fn read_and_storage_limits_stop_before_another_capture_or_batch() {
    let (mut s, _, _, reads) = session();
    let id = s.view.investigation_id.clone();
    s.capture().unwrap();
    let before = reads.get();
    s.view.coverage.attempted_bytes = MAX_READ_BYTES - 1;
    assert!(s.capture().is_err());
    assert_eq!(reads.get(), before);
    assert!(s.advance_memory(&id, 1).is_err());
    s.view.coverage.attempted_bytes = 0;
    s.view.coverage.archive_bytes = reversebolt_core::coverage::MAX_ARCHIVE_BYTES;
    assert!(s.advance_memory(&id, 1).is_err());
    assert_eq!(s.view.coverage.batch, 1);
    s.view.coverage.image_budget_bytes = reversebolt_core::coverage::MAX_IMAGE_DISK_BYTES;
    assert!(s.capture().is_err());
    assert_eq!(reads.get(), before);
}

#[test]
fn a_renderer_finishing_after_timeout_keeps_its_storage_reservation() {
    use std::{cell::RefCell, sync::mpsc};
    type Pending = Rc<RefCell<Option<(mpsc::Sender<()>, std::thread::JoinHandle<()>)>>>;
    struct Late {
        inner: Fixture,
        pending: Pending,
    }
    impl NativeTarget for Late {
        fn info(&self) -> &ProcessInfo {
            self.inner.info()
        }
        fn alive(&self) -> bool {
            true
        }
        fn regions(&self) -> Result<Vec<Region>> {
            self.inner.regions()
        }
        fn read(&self, a: u64, n: usize) -> Result<Vec<u8>> {
            self.inner.read(a, n)
        }
        fn screen(&self, path: &Path) -> Result<Screen> {
            let path = path.to_owned();
            let (send, receive) = mpsc::channel();
            let thread = std::thread::spawn(move || {
                receive.recv().unwrap();
                std::fs::write(path, b"\x89PNG\r\n\x1a\nlate").unwrap();
            });
            *self.pending.borrow_mut() = Some((send, thread));
            Err("Renderer timed out".into())
        }
    }
    let pending = Rc::new(RefCell::new(None));
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/batch-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let target = Late {
        pending: pending.clone(),
        inner: Fixture {
            info: ProcessInfo::default(),
            frame: Cell::new(0),
            drift: Rc::new(Cell::new(false)),
            reads: Rc::new(Cell::new(0)),
        },
    };
    let mut s = LiveSession::attach(Box::new(target), 1, path.clone()).unwrap();
    s.capture().unwrap();
    assert_eq!(s.view.coverage.image_disk_bytes, 0);
    assert_eq!(
        s.view.coverage.image_budget_bytes,
        reversebolt_core::visual::MAX_IMAGE_BYTES as u64
    );
    let (send, thread) = pending.borrow_mut().take().unwrap();
    send.send(()).unwrap();
    thread.join().unwrap();
    assert!(
        std::fs::metadata(path.join("frame-001.png")).unwrap().len()
            <= s.view.coverage.image_budget_bytes
    );
}
