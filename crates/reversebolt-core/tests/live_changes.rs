//! Live-session API contract test. The renderer is a supplied fixture, not OCR
//! accuracy or independent gameplay evidence. Native writes have a separate smoke.
use reversebolt_core::{
    Result,
    changes::{BytePatch, Phase, Store, WriteReceipt},
    live::*,
    prediction::Source,
};
use std::{
    cell::{Cell, RefCell},
    path::Path,
    rc::Rc,
};

struct State {
    bytes: RefCell<Vec<u8>>,
    calls: Cell<usize>,
    resize: Cell<bool>,
    dead: Cell<bool>,
}
impl State {
    fn set(&self, value: f32) {
        self.bytes.borrow_mut()[32..36].copy_from_slice(&value.to_le_bytes());
    }
}
struct Target {
    state: Rc<State>,
    info: ProcessInfo,
}
impl NativeTarget for Target {
    fn info(&self) -> &ProcessInfo {
        &self.info
    }
    fn alive(&self) -> bool {
        !self.state.dead.get()
    }
    fn regions(&self) -> Result<Vec<Region>> {
        Ok(vec![Region {
            base: 65536,
            length: 128,
            allocation_base: 65536,
            protection: 4,
            kind: 0x20000,
            image: false,
        }])
    }
    fn read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        let offset = (address - 65536) as usize;
        Ok(self.state.bytes.borrow()[offset..offset + length].to_vec())
    }
    fn screen(&self, path: &Path) -> Result<Screen> {
        std::fs::write(path, b"\x89PNG\r\n\x1a\ncontract fixture").map_err(|e| e.to_string())?;
        let value = f32::from_le_bytes(self.state.bytes.borrow()[32..36].try_into().unwrap());
        Ok(Screen {
            path: path.to_string_lossy().into(),
            width: 100,
            height: if self.state.resize.get() { 110 } else { 100 },
            text: String::new(),
            words: vec![Word {
                text: value.to_string(),
                bounds: [10., 10., 40., 20.],
                line: 0,
                source: TextSource::WindowsOcr,
            }],
            warning: None,
            method: "contract fixture".into(),
        })
    }
    fn guarded_write(&self, patch: &BytePatch, _: &Region) -> WriteReceipt {
        let offset = (patch.address - 65536) as usize;
        let mut bytes = self.state.bytes.borrow_mut();
        if bytes[offset..offset + patch.before.len()] != patch.before {
            return WriteReceipt::refused("conflict");
        }
        self.state.calls.set(self.state.calls.get() + 1);
        bytes[offset..offset + patch.after.len()].copy_from_slice(&patch.after);
        WriteReceipt {
            attempted: true,
            transferred: patch.after.len(),
            observed_after: Some(patch.after.clone()),
            error: None,
        }
    }
    fn recognize(&self, _: &mut Screen) -> Result<()> {
        Err("Optional recognizer unavailable; fixture words retained.".into())
    }
}

#[test]
fn session_prepares_trials_checks_source_evidence_and_revokes_passive_predictions() {
    let root =
        std::env::temp_dir().join(format!("reversebolt-live-change-{}", uuid::Uuid::new_v4()));
    let state = Rc::new(State {
        bytes: RefCell::new(vec![0; 128]),
        calls: Cell::new(0),
        resize: Cell::new(false),
        dead: Cell::new(false),
    });
    let mut session = LiveSession::attach(
        Box::new(Target {
            state: state.clone(),
            info: ProcessInfo {
                pid: 91,
                created: "created-a".into(),
                window: 100,
                ..Default::default()
            },
        }),
        1,
        root.join("captures"),
    )
    .unwrap();
    let mut store = Store::open(root.join("changes")).unwrap();
    for value in [40., 45., 51., 38., 59., 44., 57., 50., 61., 43., 55., 48.] {
        state.set(value);
        session.capture().unwrap();
    }
    session.correlate().unwrap();
    let id = session.view.tracks[0].id;
    assert!(!session.view.tracks[0].matches.is_empty());
    session
        .start_prediction_check(
            &session.view.investigation_id.clone(),
            1,
            12,
            Source::Ocr,
            id,
        )
        .unwrap();
    assert!(
        session
            .prepare_change(id, 51., "agent", &mut store)
            .is_err()
    );
    for value in [52., 52., 50., 50.] {
        state.set(value);
        session.capture().unwrap();
    }
    let change = session
        .prepare_change(id, 53., "agent", &mut store)
        .unwrap();
    assert_eq!(change.phase, Phase::Prepared);
    assert_eq!(state.calls.get(), 0);
    assert!(change.models.len() >= 2);
    let result = session
        .execute_change(&change.id, false, &mut store)
        .unwrap();
    assert_eq!(result.phase, Phase::Written);
    assert!(
        session
            .view
            .prediction_check
            .as_ref()
            .unwrap()
            .invalidated
            .is_some()
    );
    let checked = session.observe_change(&change.id, &mut store).unwrap();
    assert!(checked.observations[0].agrees);
    assert_eq!(checked.observations[0].value, Some(53.));
    assert_eq!(checked.observations[0].image_sha256.len(), 64);
    assert!(checked.observations[0].warning.is_some());
    assert!(
        session
            .prepare_change(id, 56., "agent", &mut store)
            .is_err()
    );
    session
        .execute_change(&change.id, false, &mut store)
        .unwrap();
    assert_eq!(state.calls.get(), 1);
    state.resize.set(true);
    let resized = session.observe_change(&change.id, &mut store).unwrap();
    assert!(!resized.observations[1].agrees);
    assert!(resized.observations[1].error.is_some());
    state.resize.set(false);
    let restored = session
        .execute_change(&change.id, true, &mut store)
        .unwrap();
    assert_eq!(restored.phase, Phase::Restored);
    assert_eq!(state.calls.get(), 2);
    assert!(
        session
            .observe_change(&change.id, &mut store)
            .unwrap()
            .observations
            .last()
            .unwrap()
            .agrees
    );
    state.dead.set(true);
    assert!(session.observe_change(&change.id, &mut store).is_err());
    assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
    std::fs::remove_dir_all(root).unwrap();
}
