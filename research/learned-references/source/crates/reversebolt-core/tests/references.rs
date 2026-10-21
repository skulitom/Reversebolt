//! Controlled reference/relocation contract, not independent-game evidence.
use reversebolt_core::{
    Result,
    changes::{BytePatch, Phase, Store, WriteReceipt},
    live::*,
    prediction::Assessment,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::{Path, PathBuf},
    rc::Rc,
};

const ROOT: u64 = 0x10000;
const FIRST: u64 = 0x20000;
const SECOND: u64 = 0x30000;
const THIRD: u64 = 0x40000;
const OFFSET: usize = 64;
struct State {
    bytes: RefCell<BTreeMap<u64, Vec<u8>>>,
    active: Cell<u64>,
    writes: Cell<usize>,
    retired: Cell<bool>,
    move_in_image: Cell<bool>,
}
impl State {
    fn value(&self) -> f32 {
        f32::from_le_bytes(
            self.bytes.borrow()[&self.active.get()][OFFSET..OFFSET + 4]
                .try_into()
                .unwrap(),
        )
    }
    fn set(&self, value: f32) {
        self.bytes.borrow_mut().get_mut(&self.active.get()).unwrap()[OFFSET..OFFSET + 4]
            .copy_from_slice(&value.to_le_bytes());
    }
    fn relocate(&self, destination: u64) {
        let value = self.value();
        self.active.set(destination);
        self.set(value);
        self.bytes.borrow_mut().get_mut(&ROOT).unwrap()[..8]
            .copy_from_slice(&destination.to_le_bytes());
    }
}
struct Target(Rc<State>, ProcessInfo);
impl NativeTarget for Target {
    fn info(&self) -> &ProcessInfo {
        &self.1
    }
    fn alive(&self) -> bool {
        true
    }
    fn regions(&self) -> Result<Vec<Region>> {
        Ok([ROOT, FIRST, SECOND, THIRD]
            .into_iter()
            .filter(|base| *base != FIRST || !self.0.retired.get())
            .map(|base| Region {
                base,
                length: 128,
                allocation_base: base,
                protection: 4,
                kind: 0x20000,
                image: false,
            })
            .collect())
    }
    fn read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        let base = address & !0xFFFF;
        if base == FIRST && self.0.retired.get() {
            return Err("retired allocation".into());
        }
        self.0
            .bytes
            .borrow()
            .get(&base)
            .and_then(|b| b.get((address - base) as usize..(address - base) as usize + length))
            .map(|b| b.to_vec())
            .ok_or_else(|| "unavailable".into())
    }
    fn screen(&self, path: &Path) -> Result<Screen> {
        std::fs::write(path, b"\x89PNG\r\n\x1a\nreference fixture").unwrap();
        let value = self.0.value();
        if self.0.move_in_image.replace(false) {
            self.0.relocate(THIRD);
        }
        Ok(Screen {
            path: path.to_string_lossy().into(),
            width: 300,
            height: 100,
            text: String::new(),
            words: vec![Word {
                text: value.to_string(),
                bounds: [10., 10., 40., 20.],
                line: 0,
                source: TextSource::WindowsOcr,
            }],
            warning: None,
            method: "fixture".into(),
        })
    }
    fn guarded_write(&self, patch: &BytePatch, _: &Region) -> WriteReceipt {
        if self.read(patch.address, patch.before.len()).ok().as_ref() != Some(&patch.before) {
            return WriteReceipt::refused("changed bytes");
        }
        let base = patch.address & !0xFFFF;
        let offset = (patch.address - base) as usize;
        self.0.bytes.borrow_mut().get_mut(&base).unwrap()[offset..offset + patch.after.len()]
            .copy_from_slice(&patch.after);
        self.0.writes.set(self.0.writes.get() + 1);
        WriteReceipt {
            attempted: true,
            transferred: patch.after.len(),
            observed_after: Some(patch.after.clone()),
            error: None,
        }
    }
}
fn setup() -> (LiveSession, Rc<State>, Store, PathBuf) {
    let root =
        std::env::temp_dir().join(format!("reversebolt-references-{}", uuid::Uuid::new_v4()));
    let state = Rc::new(State {
        bytes: RefCell::new(
            [ROOT, FIRST, SECOND, THIRD]
                .into_iter()
                .map(|a| (a, vec![0; 128]))
                .collect(),
        ),
        active: Cell::new(FIRST),
        writes: Cell::new(0),
        retired: Cell::new(false),
        move_in_image: Cell::new(false),
    });
    // One live root plus a retained copy of its old value. Both are indistinguishable before a move.
    for offset in [0, 8] {
        state.bytes.borrow_mut().get_mut(&ROOT).unwrap()[offset..offset + 8]
            .copy_from_slice(&FIRST.to_le_bytes());
    }
    let mut session = LiveSession::attach(
        Box::new(Target(
            state.clone(),
            ProcessInfo {
                pid: 900,
                created: "ref-fixture".into(),
                window: 90,
                ..Default::default()
            },
        )),
        1,
        root.join("captures"),
    )
    .unwrap();
    for (i, value) in [
        40., 56., 31., 85., 64., 27., 75., 92., 44., 67., 38., 50., 54., 54., 50., 50.,
    ]
    .into_iter()
    .enumerate()
    {
        state.set(value);
        session.capture_for_discovery(15 - i).unwrap();
    }
    let store = Store::open(root.join("changes")).unwrap();
    (session, state, store, root)
}
fn fresh(session: &mut LiveSession, state: &State) {
    for value in [54., 54., 50., 50.] {
        state.set(value);
        session.capture().unwrap();
    }
}
fn selected(session: &LiveSession) -> usize {
    session.view.prediction_checks[0]
        .candidates
        .iter()
        .position(|c| c.chunk_address == SECOND + OFFSET as u64 && c.via.is_some())
        .unwrap()
}

#[test]
fn learned_offsets_find_moved_fields_and_keep_old_locations_as_rivals() {
    for retired in [false, true] {
        let (mut session, state, mut store, root) = setup();
        let original = session.view.prediction_checks[0].clone();
        assert!(original.candidates.iter().all(|c| {
            c.references.routes.len() == 2
                && c.references
                    .routes
                    .iter()
                    .all(|r| r.offset == OFFSET as u64)
        }));
        state.relocate(SECOND);
        state.retired.set(retired);
        session
            .recheck_findings(&session.view.investigation_id.clone(), 1, &store)
            .unwrap();
        let check = &session.view.prediction_checks[0];
        assert!(check.candidates.len() > original.candidates.len());
        assert!(
            check
                .rechecked_from
                .as_ref()
                .unwrap()
                .resolutions
                .iter()
                .any(|r| r.root == ROOT && r.address == Some(SECOND + OFFSET as u64))
        );
        assert!(check.candidates.iter().all(|c| c.samples.is_empty()));
        assert!(
            session
                .prepare_change(original.track, 53., "test", &mut store)
                .is_err()
        );
        fresh(&mut session, &state);
        let index = selected(&session);
        let check = &session.view.prediction_checks[0];
        let current = &check.candidates[index];
        assert_eq!(
            current.assessment(check),
            Assessment::ConsistentInObservedScope
        );
        assert!(
            current.samples.iter().all(
                |s| s.supports_reference(current.via.as_ref().unwrap(), SECOND + OFFSET as u64)
            )
        );
        assert!(
            check
                .candidates
                .iter()
                .filter(|c| c.chunk_address == FIRST + OFFSET as u64)
                .all(|c| c.assessment(check)
                    == if retired {
                        Assessment::Invalidated
                    } else {
                        Assessment::Contradicted
                    })
        );
        let id = check.id.clone();
        let trial = session
            .prepare_probe(original.track, 53., (&id, index), "test", &mut store)
            .unwrap();
        assert_eq!(trial.phase, Phase::Prepared);
        assert!(trial.models.iter().all(|m| m.reference.is_some()));
        session
            .execute_change(&trial.id, false, &mut store)
            .unwrap();
        assert_eq!(state.value(), 53.);
        assert!(
            session
                .observe_change(&trial.id, &mut store)
                .unwrap()
                .observations
                .last()
                .unwrap()
                .agrees
        );
        session.execute_change(&trial.id, true, &mut store).unwrap();
        assert_eq!(state.value(), 50.);
        assert_eq!(state.writes.get(), 2);
        assert!(root.starts_with(std::env::temp_dir()));
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn another_move_before_write_or_restore_never_writes_through_the_old_reference() {
    for restore in [false, true] {
        let (mut session, state, mut store, root) = setup();
        state.relocate(SECOND);
        session
            .recheck_findings(&session.view.investigation_id.clone(), 1, &store)
            .unwrap();
        fresh(&mut session, &state);
        let index = selected(&session);
        let check = &session.view.prediction_checks[0];
        let trial = session
            .prepare_probe(
                check.track,
                53.,
                (&check.id.clone(), index),
                "test",
                &mut store,
            )
            .unwrap();
        if restore {
            session
                .execute_change(&trial.id, false, &mut store)
                .unwrap();
        }
        let calls = state.writes.get();
        state.relocate(THIRD);
        let error = session
            .execute_change(&trial.id, restore, &mut store)
            .unwrap_err();
        assert!(error.contains("learned reference moved"));
        assert_eq!(state.writes.get(), calls);
        assert_eq!(
            store.get(&trial.id).unwrap().phase,
            if restore {
                Phase::Written
            } else {
                Phase::Prepared
            }
        );
        assert!(root.starts_with(std::env::temp_dir()));
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn a_reference_change_during_capture_invalidates_the_new_location_even_when_bytes_match() {
    let (mut session, state, store, root) = setup();
    state.relocate(SECOND);
    session
        .recheck_findings(&session.view.investigation_id.clone(), 1, &store)
        .unwrap();
    state.move_in_image.set(true);
    session.capture().unwrap();
    let check = &session.view.prediction_checks[0];
    let current = &check.candidates[selected(&session)];
    assert_eq!(current.assessment(check), Assessment::Invalidated);
    assert!(
        current.samples[0].reference_before.is_some()
            && current.samples[0].reference_after.is_some()
    );
    assert!(
        !current.samples[0]
            .supports_reference(current.via.as_ref().unwrap(), SECOND + OFFSET as u64)
    );
    assert_eq!(state.writes.get(), 0);
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn following_references_keeps_the_existing_shared_read_budget() {
    let (mut session, state, store, root) = setup();
    state.relocate(SECOND);
    let old = session.view.prediction_checks[0].id.clone();
    session.view.coverage.attempted_bytes = reversebolt_core::coverage::MAX_READ_BYTES;
    assert!(
        session
            .recheck_findings(&session.view.investigation_id.clone(), 1, &store)
            .is_err()
    );
    assert_eq!(session.view.coverage.batch, 1);
    assert_eq!(session.view.prediction_checks[0].id, old);
    assert_eq!(state.writes.get(), 0);
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::remove_dir_all(root).unwrap();
}
