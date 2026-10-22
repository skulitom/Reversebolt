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
    values: Rc<Cell<Option<(i32, f32)>>>,
    missing: Rc<Cell<bool>>,
    changed_mapping: Rc<Cell<bool>>,
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
            protection: if self.changed_mapping.get() { 8 } else { 4 },
            kind: 0x20000,
            image: false,
        }])
    }
    fn read(&self, address: u64, n: usize) -> Result<Vec<u8>> {
        self.check();
        self.reads.set(self.reads.get() + 1);
        if self.missing.get() {
            return Err("Injected missing read".into());
        }
        let mut bytes = [0; 128];
        let (flag, number) = self.values.get().unwrap_or((
            (self.frame.get() / 2 % 2) as i32,
            self.frame.get() as f32 * 0.25,
        ));
        bytes[..4].copy_from_slice(&flag.to_le_bytes());
        bytes[8..12].copy_from_slice(&number.to_le_bytes());
        let offset = (address - 0x10000) as usize;
        Ok(bytes[offset..offset + n].to_vec())
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
    values: Rc<Cell<Option<(i32, f32)>>>,
    missing: Rc<Cell<bool>>,
    changed_mapping: Rc<Cell<bool>>,
}
impl Case {
    fn followup_request(&mut self) -> Value {
        self.command("analyze_memory_evidence", self.request())
            .unwrap();
        let summary = self
            .engine
            .live
            .as_ref()
            .unwrap()
            .view
            .memory_proposals
            .as_ref()
            .unwrap();
        let report = store::read(
            &self.root.join("captures"),
            &summary.investigation_id,
            summary.batch,
            &summary.report_sha256,
        )
        .unwrap();
        json!({"investigation_id":summary.investigation_id,"batch":summary.batch,"after_sample":summary.captures,
            "report_sha256":summary.report_sha256,"groups":(0..report.groups.len()).collect::<Vec<_>>(),"auto_observe":false})
    }
    fn followup_page(&mut self) -> Value {
        let reference = self
            .engine
            .live
            .as_ref()
            .unwrap()
            .view
            .memory_followup
            .as_ref()
            .unwrap();
        let args = serde_json::to_value(reference).unwrap();
        let summary = self.command("read_memory_followup", args.clone()).unwrap();
        let mut page = args;
        page["report_sha256"] = summary["report_sha256"].clone();
        self.command("read_memory_followup", page).unwrap()
    }
    fn new(count: usize) -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/memory-proposal-tests")
            .join(uuid::Uuid::new_v4().to_string());
        let mut engine = Engine::load(root.join("engine.json")).unwrap();
        let frames = Rc::new(Cell::new(0));
        let reads = Rc::new(Cell::new(0));
        let sealed = Rc::new(Cell::new(false));
        let values = Rc::new(Cell::new(None));
        let missing = Rc::new(Cell::new(false));
        let changed_mapping = Rc::new(Cell::new(false));
        let mut session = LiveSession::attach(
            Box::new(Target {
                info: ProcessInfo::default(),
                frame: frames.clone(),
                reads: reads.clone(),
                sealed: sealed.clone(),
                values: values.clone(),
                missing: missing.clone(),
                changed_mapping: changed_mapping.clone(),
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
            values,
            missing,
            changed_mapping,
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
fn memory_followup_pins_before_capture_and_replays_after_restart_without_target_access() {
    use reversebolt_core::memory_followup as f;
    let mut c = Case::new(12);
    let args = c.followup_request();
    let prepared = c.command("prepare_memory_followup", args.clone()).unwrap();
    let reference: f::Reference = serde_json::from_value(prepared["reference"].clone()).unwrap();
    let plan = f::read_plan(&c.root.join("captures"), &reference).unwrap();
    assert_eq!((c.frames.get(), c.reads.get()), (12, 12));
    assert_eq!(plan.candidates.len(), 3); // Float and its integer rival both survive.
    assert_eq!(plan.windows.len(), 1);
    assert_eq!(plan.windows[0].length, 64);
    assert_eq!(c.engine.live.as_ref().unwrap().view.sample_count, 0);
    c.sealed.set(true);
    assert!(c.command("prepare_memory_followup", args).is_err());
    assert_eq!(
        c.command("read_memory_followup", json!(reference)).unwrap()["report_sha256"],
        Value::Null
    );
    c.sealed.set(false);
    for i in 0..12 {
        let result = c
            .command("capture", json!({"mode":"visual","memory_review":i==11}))
            .unwrap();
        if i == 11 {
            assert_eq!(result["memory_proposals"]["status"], "completed");
        }
    }
    c.sealed.set(true);
    let page = c.followup_page();
    assert_eq!(page["captures"], 12);
    assert!(
        page["members"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["outcome"]["assessment"]["verdict"] == "consistent")
    );
    let replay = c.command("analyze_memory_evidence", c.request()).unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["summary"]["report_sha256"], page["report_sha256"]);
    assert_eq!((c.frames.get(), c.reads.get()), (24, 24));
    assert!(c.engine.live.as_ref().unwrap().view.tracks.is_empty());
    assert!(
        c.engine
            .live
            .as_ref()
            .unwrap()
            .view
            .prediction_checks
            .is_empty()
    );
    c.engine.view.live = Some(c.engine.live.as_ref().unwrap().view.clone());
    c.engine.save().unwrap();
    c.engine = Engine::load(c.root.join("engine.json")).unwrap();
    assert!(c.engine.live.is_none());
    let mut page_args = json!(reference);
    page_args["report_sha256"] = page["report_sha256"].clone();
    assert_eq!(
        c.command("read_memory_followup", page_args.clone())
            .unwrap(),
        page
    );
    page_args["limit"] = json!(1);
    assert_eq!(
        c.command("read_memory_followup", page_args.clone())
            .unwrap()["next_offset"],
        1
    );
    page_args["offset"] = json!(1);
    assert_eq!(
        c.command("read_memory_followup", page_args.clone())
            .unwrap()["members"][0]["index"],
        1
    );
    page_args["report_sha256"] = json!("0".repeat(64));
    assert!(c.command("read_memory_followup", page_args).is_err());
    let directory = c
        .root
        .join("captures/memory-followups")
        .join(&reference.investigation_id)
        .join("batch-002");
    let report_path = directory.join("report.json");
    let original = fs::read(&report_path).unwrap();
    let mut changed = original.clone();
    changed.push(b' ');
    fs::write(&report_path, changed).unwrap();
    assert!(c.command("read_memory_followup", json!(reference)).is_err());
    fs::write(&report_path, &original).unwrap();
    let plan_path = directory.join("plan.json");
    let original_plan = fs::read(&plan_path).unwrap();
    let mut changed = original_plan.clone();
    changed.push(b' ');
    fs::write(&plan_path, changed).unwrap();
    assert!(c.command("read_memory_followup", json!(reference)).is_err());
    fs::write(&plan_path, original_plan).unwrap();
    println!(
        "MEMORY_FOLLOWUP_FIXTURE={}",
        json!({"root":c.root,"reference":reference,"report_sha256":page["report_sha256"],"source_report_sha256":plan.source_report_sha256,"members":plan.candidates.len(),"selected_bytes":64,"source_captures":12,"fresh_captures":12,"target_reads":c.reads.get()})
    );
}

#[test]
fn memory_followup_keeps_contradictions_and_never_bridges_old_or_missing_observations() {
    let mut c = Case::new(12);
    let args = c.followup_request();
    c.command("prepare_memory_followup", args).unwrap();
    let numbers = [
        1.0, 2.0, 99.0, 0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.25,
    ];
    for (i, n) in numbers.iter().enumerate() {
        c.values
            .set(Some((if i == 1 { 2 } else { (i % 2) as i32 }, *n)));
        c.missing.set(i == 2);
        c.command("capture", json!({"mode":"visual","memory_review":i==11}))
            .unwrap();
    }
    c.sealed.set(true);
    let page = c.followup_page();
    let rows = page["members"].as_array().unwrap();
    let float = rows
        .iter()
        .find(|r| r["candidate"]["selection"]["kind"] == "F32")
        .unwrap();
    assert_eq!(float["outcome"]["assessment"]["verdict"], "consistent");
    assert_eq!(float["outcome"]["assessment"]["checked"], 9);
    assert_eq!(float["outcome"]["assessment"]["missing_reads"], 1);
    assert_eq!(float["outcome"]["assessment"]["values"][2], Value::Null);
    let flag = rows
        .iter()
        .find(|r| r["candidate"]["selection"]["address"] == 0x10000)
        .unwrap();
    assert_eq!(flag["outcome"]["assessment"]["verdict"], "contradicted");
    assert_eq!(flag["outcome"]["assessment"]["violations"], 1);
    assert_eq!(flag["candidate"]["source_verdict"], "consistent");
}

#[test]
fn memory_followup_missing_or_constant_observations_do_not_become_activity_evidence() {
    for changed_mapping in [false, true] {
        let mut c = Case::new(12);
        let args = c.followup_request();
        c.command("prepare_memory_followup", args).unwrap();
        c.values.set(Some((0, 0.0)));
        c.changed_mapping.set(changed_mapping);
        for i in 0..4 {
            c.command("capture", json!({"mode":"visual","memory_review":i==3}))
                .unwrap();
        }
        c.sealed.set(true);
        let page = c.followup_page();
        for row in page["members"].as_array().unwrap() {
            assert_eq!(row["outcome"]["changed_pairs"], 0);
            assert_eq!(
                row["outcome"]["mapping_invalidated_in_recording"],
                changed_mapping
            );
            assert_eq!(
                row["outcome"]["assessment"]["verdict"],
                if changed_mapping {
                    "insufficient"
                } else {
                    "consistent"
                }
            );
        }
    }
}

#[test]
fn memory_followup_waits_for_four_fresh_samples_and_preserves_invalid_numbers() {
    let mut c = Case::new(12);
    let args = c.followup_request();
    c.command("prepare_memory_followup", args).unwrap();
    for n in [1.0, f32::NAN, 2.0] {
        c.values.set(Some((0, n)));
        c.command("capture", json!({"mode":"visual"})).unwrap();
    }
    assert_eq!(
        finish(&mut c.engine, &c.root.join("captures"), &c.owner)["status"],
        "skipped"
    );
    assert!(
        !c.root
            .join("captures/source-folder/memory-evidence/batch-002")
            .exists()
    );
    c.values.set(Some((0, 1.0)));
    c.command("capture", json!({"mode":"visual","memory_review":true}))
        .unwrap();
    c.sealed.set(true);
    let page = c.followup_page();
    let float = page["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["candidate"]["selection"]["kind"] == "F32")
        .unwrap();
    let a = &float["outcome"]["assessment"];
    assert_eq!(a["invalid_numbers"], 1);
    assert_eq!(a["missing_reads"], 0);
    assert_eq!(a["checked"], 1);
    assert_eq!(a["violations"], 1);
    assert_eq!(a["verdict"], "contradicted");
}

#[test]
fn memory_followup_rejects_stale_selection_and_cancellation_preserves_unpublished_plan() {
    use reversebolt_core::memory_followup as f;
    let mut c = Case::new(12);
    let args = c.followup_request();
    c.sealed.set(true);
    for (field, value) in [
        ("after_sample", json!(11)),
        ("groups", json!([0, 0])),
        ("groups", json!([128])),
        ("groups", json!([])),
        ("report_sha256", json!("0".repeat(64))),
        ("unexpected", json!(true)),
    ] {
        let mut bad = args.clone();
        bad[field] = value;
        assert!(c.command("prepare_memory_followup", bad).is_err());
    }
    assert_eq!(c.engine.live.as_ref().unwrap().view.coverage.batch, 1);
    c.sealed.set(false);
    c.command("prepare_memory_followup", args).unwrap();
    let reference = c
        .engine
        .live
        .as_ref()
        .unwrap()
        .view
        .memory_followup
        .clone()
        .unwrap();
    let root = c.root.join("captures");
    let plan = f::read_plan(&root, &reference).unwrap();
    assert_eq!(f::save_plan(&root, &plan).unwrap(), reference);
    let mut other = plan.clone();
    other.source_captures += 1;
    assert!(f::save_plan(&root, &other).is_err());
    for _ in 0..4 {
        c.command("capture", json!({"mode":"visual"})).unwrap();
    }
    c.sealed.set(true);
    c.owner.cancel.store(true, Ordering::SeqCst);
    assert_eq!(
        finish(&mut c.engine, &root, &c.owner)["status"],
        "cancelled"
    );
    assert!(f::read_report(&root, &reference).unwrap().is_none());
    assert!(
        !c.root
            .join("captures/source-folder/memory-evidence/batch-002")
            .exists()
    );
    c.owner.cancel.store(false, Ordering::SeqCst);
    assert_eq!(
        finish(&mut c.engine, &root, &c.owner)["status"],
        "completed"
    );
    let session = c.engine.live.as_ref().unwrap();
    let exported = session
        .export_memory_evidence(&reference.investigation_id, 2, 4)
        .unwrap();
    let archive = Archive::open(Path::new(&exported.directory), &exported.manifest_sha256).unwrap();
    let mut changed = plan.clone();
    changed.window = 1;
    assert!(f::assess(&changed, &archive, || false).is_err());
    assert!(f::assess(&plan, &archive, || true).is_err());
    assert_eq!(c.reads.get(), 16);
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
