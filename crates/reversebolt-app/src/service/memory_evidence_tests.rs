use super::*;
use reversebolt_core::{live::*, memory_evidence::Archive};
use std::{cell::Cell, path::Path, rc::Rc};

struct SavedOnly {
    info: ProcessInfo,
    sealed: Rc<Cell<bool>>,
}
impl SavedOnly {
    fn check(&self) {
        assert!(!self.sealed.get(), "Export handler contacted the process");
    }
}
impl NativeTarget for SavedOnly {
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
            length: 64,
            allocation_base: 0x10000,
            protection: 4,
            kind: 0x20000,
            image: false,
        }])
    }
    fn read(&self, _: u64, n: usize) -> Result<Vec<u8>> {
        self.check();
        Ok(vec![42; n])
    }
    fn screen(&self, _: &Path) -> Result<Screen> {
        self.check();
        Err("No image supplied by this fixture".into())
    }
}

#[test]
fn shared_command_exports_existing_bytes_and_validates_exact_scope() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/memory-evidence-service-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let mut engine = Engine::load(root.join("engine.json")).unwrap();
    let sealed = Rc::new(Cell::new(false));
    let mut session = LiveSession::attach(
        Box::new(SavedOnly {
            info: ProcessInfo::default(),
            sealed: sealed.clone(),
        }),
        1,
        root.join("captures"),
    )
    .unwrap();
    session.capture().unwrap();
    let id = session.view.investigation_id.clone();
    let before = serde_json::to_value(&session.view).unwrap();
    engine.live = Some(session);
    sealed.set(true);
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut pipeline = crate::observation_pipeline::Pipeline::default();
    let mut tests = crate::runtime_test_bridge::Session::new(Arc::new(AtomicBool::new(false)));
    let mut run = |args| {
        native_command(
            &mut engine,
            &Command {
                action: "export_memory_evidence".into(),
                args,
                source: "agent".into(),
            },
            &root.join("captures"),
            None,
            &mut pipeline,
            &published,
            &mut tests,
        )
    };
    for args in [
        json!({}),
        json!({"investigation_id":id,"batch":1,"after_sample":2}),
        json!({"investigation_id":id,"batch":1,"after_sample":1,"path":"elsewhere"}),
    ] {
        assert!(run(args).is_err());
    }
    let args = json!({"investigation_id":id,"batch":1,"after_sample":1});
    let first = run(args.clone()).unwrap();
    assert_eq!(first["historical"], true);
    assert_eq!(first["replayed"], false);
    assert_eq!(first["payload_bytes"], 64);
    let replay = run(args).unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(first["manifest_sha256"], replay["manifest_sha256"]);
    assert_eq!(
        serde_json::to_value(&engine.live.as_ref().unwrap().view).unwrap(),
        before
    );
    let archive = Archive::open(
        Path::new(first["directory"].as_str().unwrap()),
        first["manifest_sha256"].as_str().unwrap(),
    )
    .unwrap();
    archive.verify().unwrap();
    assert_eq!(archive.sample(1).unwrap()[&0x10000], vec![42; 64]);
}
