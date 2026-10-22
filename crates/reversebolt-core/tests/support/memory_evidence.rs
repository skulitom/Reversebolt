use super::*;
use reversebolt_core::{memory_evidence as m, visual};
use std::{
    fs,
    io::{Seek, SeekFrom, Write},
};

struct Guarded {
    inner: Fixture,
    sealed: Rc<Cell<bool>>,
}
impl Guarded {
    fn check(&self) {
        assert!(!self.sealed.get(), "Export contacted the target");
    }
}
impl NativeTarget for Guarded {
    fn info(&self) -> &ProcessInfo {
        self.check();
        self.inner.info()
    }
    fn alive(&self) -> bool {
        self.check();
        self.inner.alive()
    }
    fn regions(&self) -> Result<Vec<Region>> {
        self.check();
        self.inner.regions()
    }
    fn query(&self, a: u64) -> Result<Option<Region>> {
        self.check();
        self.inner.query(a)
    }
    fn read(&self, a: u64, n: usize) -> Result<Vec<u8>> {
        self.check();
        self.inner.read(a, n)
    }
    fn screen(&self, path: &Path) -> Result<Screen> {
        self.check();
        self.inner.screen(path)
    }
    fn recognize(&self, _: &mut Screen) -> Result<()> {
        self.check();
        Ok(())
    }
    fn input_window(&self, _: u64, _: u64) -> Option<reversebolt_core::inputs::Window> {
        self.check();
        None
    }
    fn start_inputs(
        &self,
        _: &str,
        _: std::time::Instant,
        _: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<()> {
        self.check();
        Ok(())
    }
    fn guarded_write(
        &self,
        _: &reversebolt_core::changes::BytePatch,
        _: &Region,
    ) -> reversebolt_core::changes::WriteReceipt {
        panic!("No test or export may write target memory")
    }
}

#[test]
fn mixed_samples_keep_broad_bytes_missing_reads_and_separate_clocks_without_target_access() {
    let (_, path, drift, reads) = session();
    let sealed = Rc::new(Cell::new(false));
    let target = Guarded {
        inner: Fixture {
            info: ProcessInfo::default(),
            frame: Cell::new(0),
            drift: drift.clone(),
            reads: reads.clone(),
        },
        sealed: sealed.clone(),
    };
    let mut s = LiveSession::attach(Box::new(target), 1, path.clone()).unwrap();
    let id = s.view.investigation_id.clone();
    s.capture().unwrap();
    s.advance_memory(&id, 1).unwrap();
    s.capture().unwrap(); // field = 27
    s.capture_mode(CaptureMode::Play).unwrap();
    s.capture_mode(CaptureMode::Visual).unwrap(); // field = 41, no OCR
    drift.set(true);
    s.capture().unwrap(); // mapping changed, bytes missing
    drift.set(false);
    s.capture().unwrap(); // returning mapping does not recover this trajectory
    assert_eq!(s.view.memory_sample_numbers, Some(vec![1, 3, 4, 5]));
    let original = serde_json::to_value(&s.view).unwrap();
    sealed.set(true);
    let count = reads.get();
    let export = s.export_memory_evidence(&id, 2, 5).unwrap();
    assert!(!export.replayed && export.historical);
    assert_eq!(export.payload_bytes, (4 * MIB - 2 * CHUNK_BYTES) as u64);
    assert_eq!(export.memory_samples, 4);
    let replay = s.export_memory_evidence(&id, 2, 5).unwrap();
    assert!(replay.replayed);
    assert_eq!(export.manifest_sha256, replay.manifest_sha256);
    assert_eq!(reads.get(), count);
    assert_eq!(serde_json::to_value(&s.view).unwrap(), original);
    assert!(s.export_memory_evidence("other", 2, 5).is_err());
    assert!(s.export_memory_evidence(&id, 1, 5).is_err());
    assert!(s.export_memory_evidence(&id, 2, 4).is_err());
    let archive = m::Archive::open(Path::new(&export.directory), &export.manifest_sha256).unwrap();
    archive.verify().unwrap();
    assert_eq!(
        serde_json::to_value(&archive.manifest().state).unwrap(),
        original
    );
    assert_eq!(archive.manifest().invalid_chunks, vec![BASE + MIB as u64]);
    assert!(archive.sample(2).unwrap().is_empty());
    assert!(archive.sample(0).is_err());
    assert!(archive.sample(6).is_err());
    let field = |n| {
        let bytes = archive.sample(n).unwrap();
        let chunk = &bytes[&(BASE + MIB as u64)];
        i32::from_le_bytes(chunk[128..132].try_into().unwrap())
    };
    assert_eq!(field(1), 27);
    assert_eq!(field(3), 41);
    assert_eq!(archive.sample(1).unwrap().len(), MIB / CHUNK_BYTES);
    for n in [4, 5] {
        assert!(
            !archive
                .sample(n)
                .unwrap()
                .contains_key(&(BASE + MIB as u64))
        );
        assert_eq!(archive.sample(n).unwrap().len(), MIB / CHUNK_BYTES - 1);
    }
    for sample in archive
        .manifest()
        .samples
        .iter()
        .filter(|s| s.memory_sample)
    {
        let frame = &archive.manifest().state.frames[sample.number - 1];
        for read in &sample.reads {
            let t = read.receipt.capture_interval.as_ref().unwrap();
            assert_eq!(t.clock_id, id);
            assert!(t.begin_us >= frame.capture_interval.as_ref().unwrap().end_us);
        }
    }
    sealed.set(false);
    s.capture().unwrap();
    sealed.set(true);
    assert!(
        s.export_memory_evidence(&id, 2, 6)
            .unwrap_err()
            .contains("frozen export")
    );
    sealed.set(false);
    s.advance_memory(&id, 2).unwrap();
    drop(s);
    archive.verify().unwrap();
    assert_eq!(field(3), 41);
    // Printed only for a local audit; no game or native target was used.
    println!(
        "MEMORY_EVIDENCE_FIXTURE={}",
        serde_json::to_string(&export).unwrap()
    );
}

fn exported() -> (LiveSession, m::Export, m::Archive) {
    let (mut s, _, _, _) = session();
    s.capture().unwrap();
    let e = s
        .export_memory_evidence(&s.view.investigation_id, 1, 1)
        .unwrap();
    let a = m::Archive::open(Path::new(&e.directory), &e.manifest_sha256).unwrap();
    (s, e, a)
}

#[test]
fn historical_reader_detects_corruption_and_changed_file_lengths_after_open() {
    let (_, e, a) = exported();
    let path = Path::new(&e.directory);
    let payload = path.join("bytes.bin");
    let mut file = fs::OpenOptions::new().write(true).open(&payload).unwrap();
    file.write_all(&[1]).unwrap();
    assert!(a.verify().unwrap_err().contains("checksum"));
    assert!(a.sample(1).unwrap_err().contains("checksum"));
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(&[0]).unwrap();
    a.verify().unwrap();
    for size in [e.payload_bytes - 1, e.payload_bytes + 1] {
        file.set_len(size).unwrap();
        assert!(a.verify().is_err());
        assert!(a.sample(1).is_err());
        assert!(m::Archive::open(path, &e.manifest_sha256).is_err());
    }
    file.set_len(e.payload_bytes).unwrap();
    a.verify().unwrap();
    let manifest = path.join("manifest.json");
    let bytes = fs::read(&manifest).unwrap();
    fs::write(manifest, [bytes, b" ".to_vec()].concat()).unwrap();
    assert!(m::Archive::open(path, &e.manifest_sha256).is_err());
}

#[test]
fn manifests_reject_missing_bytes_false_membership_and_unbound_intervals() {
    let (_, _, a) = exported();
    let original = a.manifest();
    let mutations: [fn(&mut m::Manifest); 9] = [
        |m| m.samples[0].reads[0].segment = None,
        |m| m.samples[0].reads[0].segment.as_mut().unwrap().offset = 1,
        |m| {
            m.samples[0].reads[0]
                .receipt
                .capture_interval
                .as_mut()
                .unwrap()
                .clock_id = "other".into()
        },
        |m| m.samples[0].reads[0].receipt.capture_interval = None,
        |m| m.samples[0].memory_sample = false,
        |m| m.samples[0].reads[1] = m.samples[0].reads[0].clone(),
        |m| m.plan[1] = m.plan[0].clone(),
        |m| m.invalid_chunks.push(u64::MAX),
        |m| m.state.retained_bytes -= 1,
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(changed.validate().is_err());
    }
}

#[test]
fn interrupted_exports_stay_reserved_and_are_never_overwritten() {
    let (mut s, root, _, _) = session();
    let id = s.view.investigation_id.clone();
    s.capture().unwrap();
    let directory = root.join("memory-evidence/batch-001");
    fs::create_dir_all(&directory).unwrap();
    let payload = directory.join("bytes.bin");
    fs::write(&payload, b"interrupted attempt").unwrap();
    assert!(s.export_memory_evidence(&id, 1, 1).is_err());
    let reservation = fs::read(directory.join("reservation.json")).unwrap();
    assert!(!directory.join("manifest.json").exists());
    assert!(s.export_memory_evidence(&id, 1, 1).is_err());
    assert_eq!(fs::read(&payload).unwrap(), b"interrupted attempt");
    assert_eq!(
        fs::read(directory.join("reservation.json")).unwrap(),
        reservation
    );
    s.advance_memory(&id, 1).unwrap();
    s.capture().unwrap();
    // A previous incomplete reservation exhausts the store without allocating 512 MiB.
    let mut record: serde_json::Value = serde_json::from_slice(&reservation).unwrap();
    record["bytes"] = serde_json::json!(m::MAX_STORE_BYTES);
    fs::write(
        directory.join("reservation.json"),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
    assert!(
        s.export_memory_evidence(&id, 2, 1)
            .unwrap_err()
            .contains("512 MiB")
    );
    assert!(!root.join("memory-evidence/batch-002").exists());
}

#[test]
fn replay_rejects_a_changed_reservation_charge_and_oversized_manifest() {
    let (s, e, _) = exported();
    let path = Path::new(&e.directory);
    let marker = path.join("reservation.json");
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
    let before = fs::read(path.join("manifest.json")).unwrap();
    record["bytes"] = serde_json::json!(1);
    fs::write(marker, serde_json::to_vec(&record).unwrap()).unwrap();
    assert!(
        s.export_memory_evidence(&s.view.investigation_id, 1, 1)
            .is_err()
    );
    assert_eq!(fs::read(path.join("manifest.json")).unwrap(), before);
    let file = fs::OpenOptions::new()
        .write(true)
        .open(path.join("manifest.json"))
        .unwrap();
    file.set_len(m::MAX_MANIFEST_BYTES as u64 + 1).unwrap();
    assert!(
        m::Archive::open(path, &visual::digest(&before))
            .err()
            .unwrap()
            .contains("limit")
    );
}
