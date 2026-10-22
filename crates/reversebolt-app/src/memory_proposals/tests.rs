use super::*;
use reversebolt_core::{Command, live::*, memory_trajectories::store};
use std::{
    cell::Cell,
    fs,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, RwLock},
};

struct Target {
    info: ProcessInfo,
    frame: Rc<Cell<usize>>,
    reads: Rc<Cell<usize>>,
    sealed: Rc<Cell<bool>>,
}
impl Target {
    fn check(&self) {
        assert!(!self.sealed.get(), "Memory review contacted the target");
    }
}
impl NativeTarget for Target {
    fn info(&self) -> &ProcessInfo {
        self.check();
        &self.info
    }
    fn alive(&self) -> bool {
        self.check();
        true
    }
    fn regions(&self) -> Result<Vec<Region>> {
        self.check();
        Ok(vec![Region {
            base: 0x10000,
            length: 128,
            allocation_base: 0x10000,
            protection: 4,
            kind: 0x20000,
            image: false,
        }])
    }
    fn read(&self, _: u64, n: usize) -> Result<Vec<u8>> {
        self.check();
        self.reads.set(self.reads.get() + 1);
        let mut bytes = vec![0; n];
        bytes[..4].copy_from_slice(&((self.frame.get() / 2 % 2) as i32).to_le_bytes());
        bytes[8..12].copy_from_slice(&(self.frame.get() as f32 * 0.25).to_le_bytes());
        Ok(bytes)
    }
    fn screen(&self, path: &Path) -> Result<Screen> {
        self.check();
        self.frame.set(self.frame.get() + 1);
        fs::write(path, b"\x89PNG\r\n\x1a\nfixture").unwrap();
        Ok(Screen {
            sampling: None,
            path: path.to_string_lossy().into(),
            width: 100,
            height: 100,
            text: String::new(),
            words: vec![],
            warning: None,
            method: "generated fixture".into(),
        })
    }
    fn guarded_write(
        &self,
        _: &reversebolt_core::changes::BytePatch,
        _: &Region,
    ) -> reversebolt_core::changes::WriteReceipt {
        panic!("Research cannot write target memory")
    }
}
struct Case {
    engine: Engine,
    root: PathBuf,
    frames: Rc<Cell<usize>>,
    reads: Rc<Cell<usize>>,
    sealed: Rc<Cell<bool>>,
    owner: OperationOwner,
}
impl Case {
    fn new(count: usize) -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/memory-proposal-tests")
            .join(uuid::Uuid::new_v4().to_string());
        let mut engine = Engine::load(root.join("engine.json")).unwrap();
        let frames = Rc::new(Cell::new(0));
        let reads = Rc::new(Cell::new(0));
        let sealed = Rc::new(Cell::new(false));
        let mut session = LiveSession::attach(
            Box::new(Target {
                info: ProcessInfo::default(),
                frame: frames.clone(),
                reads: reads.clone(),
                sealed: sealed.clone(),
            }),
            1,
            root.join("captures/source-folder"),
        )
        .unwrap();
        for _ in 0..count {
            session.capture().unwrap();
        }
        engine.live = Some(session);
        Self {
            engine,
            root,
            frames,
            reads,
            sealed,
            owner: OperationOwner {
                job: 7,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        }
    }
    fn command(&mut self, action: &str, args: Value) -> Result<Value> {
        let published = Arc::new(RwLock::new(self.engine.view.clone()));
        crate::service::native_command(
            &mut self.engine,
            &Command {
                action: action.into(),
                args,
                source: "agent".into(),
            },
            &self.root.join("captures"),
            Some(&self.owner),
            &mut crate::observation_pipeline::Pipeline::default(),
            &published,
            &mut crate::runtime_test_bridge::Session::new(Arc::new(AtomicBool::new(false))),
        )
    }
    fn request(&self) -> Value {
        let v = &self.engine.live.as_ref().unwrap().view;
        json!({"investigation_id":v.investigation_id,"batch":v.coverage.batch,"after_sample":v.sample_count})
    }
}

#[test]
fn final_capture_publishes_paged_historical_evidence_without_extra_target_access() {
    let mut c = Case::new(11);
    let result = c.command("capture", json!({"memory_review":true})).unwrap();
    assert_eq!(result["memory_proposals"]["status"], "completed");
    assert_eq!((c.frames.get(), c.reads.get()), (12, 12));
    c.sealed.set(true);
    let summary = c
        .engine
        .live
        .as_ref()
        .unwrap()
        .view
        .memory_proposals
        .clone()
        .unwrap();
    let replay = c.command("analyze_memory_evidence", c.request()).unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["summary"]["report_sha256"], summary.report_sha256);
    assert!(c.engine.live.as_ref().unwrap().view.tracks.is_empty());
    assert!(
        c.engine
            .live
            .as_ref()
            .unwrap()
            .view
            .visual_tracks
            .is_empty()
    );
    let args = json!({"investigation_id":summary.investigation_id,"batch":summary.batch,"report_sha256":summary.report_sha256,"limit":1});
    let page = c.command("read_memory_proposals", args.clone()).unwrap();
    assert_eq!(page["historical"], true);
    assert!(page["total_groups"].as_u64().unwrap() > 1);
    assert_eq!(page["groups"].as_array().unwrap().len(), 1);
    assert_eq!(page["next_offset"], 1);
    let mut next = args.clone();
    next["offset"] = json!(1);
    assert_eq!(
        c.command("read_memory_proposals", next).unwrap()["groups"][0]["index"],
        1
    );
    let mut bad = args.clone();
    bad["report_sha256"] = json!("0".repeat(64));
    assert!(c.command("read_memory_proposals", bad).is_err());
    let mut bad = args.clone();
    bad["investigation_id"] = json!("../outside");
    assert!(c.command("read_memory_proposals", bad).is_err());
    let mut bad = args.clone();
    bad["limit"] = json!(9);
    assert!(c.command("read_memory_proposals", bad).is_err());
    let mut bad = args.clone();
    bad["path"] = json!("elsewhere");
    assert!(c.command("read_memory_proposals", bad).is_err());
    let published = Arc::new(RwLock::new(c.engine.view.clone()));
    crate::observation_pipeline::checkpoint(&mut c.engine, &published).unwrap();
    let saved: Value =
        serde_json::from_slice(&fs::read(c.root.join("engine.json")).unwrap()).unwrap();
    assert_eq!(
        saved["live"]["memory_proposals"]["report_sha256"],
        summary.report_sha256
    );
    c.engine = Engine::load(c.root.join("engine.json")).unwrap();
    assert!(c.engine.live.is_none());
    let lookup = c
        .command(
            "read_memory_proposals",
            json!({"investigation_id":summary.investigation_id,"batch":1}),
        )
        .unwrap();
    assert_eq!(lookup["summary"]["report_sha256"], summary.report_sha256);
    assert_eq!(
        c.command("read_memory_proposals", args.clone()).unwrap(),
        page
    );
    let path = c
        .root
        .join("captures/memory-proposals")
        .join(&summary.investigation_id)
        .join("batch-001/report.json");
    let original = fs::read(&path).unwrap();
    fs::write(&path, [original.clone(), b" ".to_vec()].concat()).unwrap();
    assert!(
        c.command("read_memory_proposals", args)
            .unwrap_err()
            .contains("checksum")
    );
    fs::write(&path, original).unwrap();
    println!(
        "MEMORY_PROPOSALS_FIXTURE={}",
        json!({"root":c.root,"summary":summary,"frames":c.frames.get(),"reads":c.reads.get()})
    );
}

#[test]
fn short_cancelled_and_storage_failed_reviews_preserve_observations() {
    let mut short = Case::new(2);
    let result = short
        .command("capture", json!({"memory_review":true}))
        .unwrap();
    assert_eq!(result["memory_proposals"]["status"], "skipped");
    assert!(
        !short
            .root
            .join("captures/source-folder/memory-evidence")
            .exists()
    );
    let mut cancelled = Case::new(12);
    cancelled.sealed.set(true);
    cancelled.owner.cancel.store(true, Ordering::SeqCst);
    assert!(
        cancelled
            .command("analyze_memory_evidence", cancelled.request())
            .is_err()
    );
    assert_eq!(
        finish(
            &mut cancelled.engine,
            &cancelled.root.join("captures"),
            &cancelled.owner
        )["status"],
        "cancelled"
    );
    assert!(
        cancelled
            .engine
            .live
            .as_ref()
            .unwrap()
            .view
            .memory_proposals
            .is_none()
    );
    assert!(
        !cancelled
            .root
            .join("captures/source-folder/memory-evidence")
            .exists()
    );
    let mut failed = Case::new(11);
    fs::write(
        failed.root.join("captures/memory-proposals"),
        b"blocked path",
    )
    .unwrap();
    let result = failed
        .command("capture", json!({"memory_review":true}))
        .unwrap();
    assert_eq!(result["memory_proposals"]["status"], "unavailable");
    assert_eq!(failed.engine.live.as_ref().unwrap().view.sample_count, 12);
    assert!(
        failed
            .engine
            .live
            .as_ref()
            .unwrap()
            .view
            .memory_proposal_error
            .is_some()
    );
}

#[test]
fn later_captures_do_not_extend_or_replace_the_frozen_review() {
    let mut c = Case::new(12);
    c.sealed.set(true);
    let first = c.command("analyze_memory_evidence", c.request()).unwrap();
    let summary: store::Summary = serde_json::from_value(first["summary"].clone()).unwrap();
    c.sealed.set(false);
    c.engine.live.as_mut().unwrap().capture().unwrap();
    c.sealed.set(true);
    assert!(
        c.command("analyze_memory_evidence", c.request())
            .unwrap_err()
            .contains("historical")
    );
    let again = finish(&mut c.engine, &c.root.join("captures"), &c.owner);
    assert_eq!(again["summary"]["captures"], 12);
    assert_eq!(again["summary"]["report_sha256"], summary.report_sha256);
    assert_eq!(c.engine.live.as_ref().unwrap().view.sample_count, 13);
    assert_eq!(c.reads.get(), 13);
}

#[test]
fn partial_export_cancellation_remains_reserved_without_repeating_target_reads() {
    let c = Case::new(12);
    c.sealed.set(true);
    let session = c.engine.live.as_ref().unwrap();
    let directory = c
        .root
        .join("captures/source-folder/memory-evidence/batch-001");
    let error = session
        .export_memory_evidence_with_cancel(&session.view.investigation_id, 1, 12, || {
            directory.join("reservation.json").exists()
        })
        .unwrap_err();
    assert!(error.contains("cancelled"));
    assert!(directory.join("reservation.json").exists());
    assert!(!directory.join("manifest.json").exists());
    assert_eq!(fs::metadata(directory.join("bytes.bin")).unwrap().len(), 0);
    assert!(
        session
            .export_memory_evidence(&session.view.investigation_id, 1, 12)
            .is_err()
    );
    assert_eq!(c.reads.get(), 12);
}
