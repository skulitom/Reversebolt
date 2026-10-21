use reversebolt_core::{
    Result,
    changes::*,
    coverage::Region,
    discovery::{Candidate, Kind},
    live::{NativeTarget, ProcessInfo, Screen},
};
use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
};

struct Fixture {
    bytes: RefCell<Vec<u8>>,
    calls: Cell<usize>,
    mode: Cell<u8>,
    info: ProcessInfo,
    region: RefCell<Region>,
    intent: RefCell<Option<PathBuf>>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            bytes: RefCell::new(50f32.to_le_bytes().to_vec()),
            calls: Cell::new(0),
            mode: Cell::new(0),
            info: ProcessInfo {
                pid: 123,
                created: "generation-a".into(),
                window: 10,
                ..Default::default()
            },
            region: RefCell::new(Region {
                base: 4096,
                length: 4096,
                allocation_base: 4096,
                kind: 0x20000,
                protection: 4,
                image: false,
            }),
            intent: RefCell::new(None),
        }
    }
    fn scope(&self) -> Scope {
        Scope {
            investigation: "investigation-a".into(),
            batch: 1,
            epoch: 0,
            frame: 16,
            prediction: "prediction-a".into(),
            process: self.info.clone(),
            track: 0,
            label: "Number".into(),
            bounds: [0., 0., 40., 20.],
            image_size: [800, 600],
        }
    }
    fn models(&self) -> Vec<Model> {
        [
            (Kind::F32, 1., 0.),
            (Kind::I32, 1. / 262144., 32. - 1107296256. / 262144.),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (kind, slope, intercept))| Model {
            index,
            address: 4096,
            mapping: Candidate {
                offset: 0,
                kind,
                slope,
                intercept,
                train_inliers: 10,
                validation_inliers: 10,
                exact: true,
            },
            region: self.region.borrow().clone(),
            before: self.bytes.borrow().clone(),
        })
        .collect()
    }
    fn change(&self) -> Change {
        Change::prepare(self.scope(), self.models(), 50., 55., "test").unwrap()
    }
}
impl NativeTarget for Fixture {
    fn info(&self) -> &ProcessInfo {
        &self.info
    }
    fn alive(&self) -> bool {
        true
    }
    fn regions(&self) -> Result<Vec<Region>> {
        Ok(vec![self.region.borrow().clone()])
    }
    fn read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        self.bytes
            .borrow()
            .get((address - 4096) as usize..(address - 4096) as usize + length)
            .map(|v| v.to_vec())
            .ok_or("bad range".into())
    }
    fn screen(&self, _: &Path) -> Result<Screen> {
        Err("not used".into())
    }
    fn guarded_write(&self, p: &BytePatch, _: &Region) -> WriteReceipt {
        let durable: Change =
            serde_json::from_slice(&std::fs::read(self.intent.borrow().as_ref().unwrap()).unwrap())
                .unwrap();
        assert!(
            matches!(durable.phase, Phase::Writing | Phase::Restoring),
            "intent must precede native entry"
        );
        self.calls.set(self.calls.get() + 1);
        if self.mode.get() == 1 {
            return WriteReceipt::refused("denied before write");
        }
        let start = (p.address - 4096) as usize;
        let mut bytes = self.bytes.borrow_mut();
        bytes[start..start + p.after.len()].copy_from_slice(&p.after);
        if self.mode.get() == 2 {
            return WriteReceipt {
                attempted: true,
                transferred: 1,
                observed_after: None,
                error: Some("partial or unknown".into()),
            };
        }
        WriteReceipt {
            attempted: true,
            transferred: p.after.len(),
            observed_after: Some(p.after.clone()),
            error: None,
        }
    }
}
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("reversebolt-change-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        assert_eq!(self.0.parent(), Some(std::env::temp_dir().as_path()));
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn setup() -> (Directory, Store, Fixture, String) {
    let dir = Directory::new();
    let mut store = Store::open(dir.0.join("changes")).unwrap();
    let target = Fixture::new();
    let c = store.insert(target.change()).unwrap();
    *target.intent.borrow_mut() = Some(store.directory().join(format!("{}.json", c.id)));
    (dir, store, target, c.id)
}

#[test]
fn agreement_is_per_value_and_no_missing_rival_gets_a_vote() {
    let f = Fixture::new();
    let ready = f.change();
    assert_eq!(ready.phase, Phase::Prepared);
    assert_eq!(ready.patch.unwrap().address, 4098);
    assert_eq!(ready.alternatives.len(), 2);
    let different = Change::prepare(f.scope(), f.models(), 50., 70., "test").unwrap();
    assert_eq!(different.phase, Phase::Unavailable);
    let mut models = f.models();
    models[1].mapping.intercept += 10.;
    let blocked = Change::prepare(f.scope(), models, 50., 55., "test").unwrap();
    assert_eq!(blocked.phase, Phase::Unavailable);
    assert!(blocked.alternatives[1].error.is_some());
}

#[test]
fn explicit_write_and_restore_are_durable_and_replays_do_not_write() {
    let (_dir, mut store, f, id) = setup();
    let changed = store.execute(&id, &f, &f.scope(), false).unwrap();
    assert_eq!(changed.phase, Phase::Written);
    assert_eq!(f.calls.get(), 1);
    assert_eq!(
        f32::from_le_bytes(f.bytes.borrow().clone().try_into().unwrap()),
        55.
    );
    store.execute(&id, &f, &f.scope(), false).unwrap();
    assert_eq!(f.calls.get(), 1);
    let restored = store.execute(&id, &f, &f.scope(), true).unwrap();
    assert_eq!(restored.phase, Phase::Restored);
    store.execute(&id, &f, &f.scope(), true).unwrap();
    assert_eq!(f.calls.get(), 2);
    assert_eq!(*f.bytes.borrow(), 50f32.to_le_bytes());
    assert_eq!(
        Store::open(store.directory().to_owned())
            .unwrap()
            .get(&id)
            .unwrap()
            .phase,
        Phase::Restored
    );
}

#[test]
fn restoration_guards_the_whole_interpretation_including_unchanged_bytes() {
    let (_dir, mut store, f, id) = setup();
    store.execute(&id, &f, &f.scope(), false).unwrap();
    f.bytes.borrow_mut()[0] ^= 1; // Outside the one-byte normalized patch.
    let result = store.execute(&id, &f, &f.scope(), true).unwrap();
    assert_eq!(result.phase, Phase::Written);
    assert!(result.reason.contains("Nothing was overwritten"));
    assert_eq!(f.calls.get(), 1);
}

#[test]
fn changed_snapshot_scope_region_and_expiration_refuse_before_native_entry() {
    for mode in 0..5 {
        let (_dir, mut store, f, id) = setup();
        let mut scope = f.scope();
        match mode {
            0 => f.bytes.borrow_mut()[0] ^= 1,
            1 => scope.investigation = "new".into(),
            2 => f.region.borrow_mut().allocation_base = 0,
            3 => scope.frame += 1,
            _ => {
                let mut c = store.get(&id).unwrap();
                c.prepared_ms = 0;
                store.record(c).unwrap();
            }
        }
        let result = store.execute(&id, &f, &scope, false);
        assert!(
            result.is_err() || matches!(result.unwrap().phase, Phase::Refused | Phase::Expired)
        );
        assert_eq!(f.calls.get(), 0);
    }
}

#[test]
fn incomplete_write_is_retained_and_blocks_further_effects_on_same_process() {
    let (_dir, mut store, f, id) = setup();
    f.mode.set(2);
    assert_eq!(
        store.execute(&id, &f, &f.scope(), false).unwrap().phase,
        Phase::Uncertain
    );
    assert!(store.execute(&id, &f, &f.scope(), false).is_err());
    assert!(store.execute(&id, &f, &f.scope(), true).is_err());
    let restarted = Store::open(store.directory().to_owned()).unwrap();
    assert_eq!(restarted.get(&id).unwrap().phase, Phase::Uncertain);
    *f.bytes.borrow_mut() = 50f32.to_le_bytes().to_vec();
    f.mode.set(0);
    let next = store.insert(f.change()).unwrap();
    assert!(store.execute(&next.id, &f, &f.scope(), false).is_err());
    assert_eq!(f.calls.get(), 1);
}

#[test]
fn process_crash_never_replays_pending_intent_and_prepared_plans_expire() {
    let (_dir, mut store, f, id) = setup();
    let other = store.insert(f.change()).unwrap();
    let mut pending = store.get(&id).unwrap();
    pending.phase = Phase::Writing;
    store.record(pending).unwrap();
    let mut restarted = Store::open(store.directory().to_owned()).unwrap();
    assert_eq!(restarted.get(&id).unwrap().phase, Phase::Uncertain);
    assert_eq!(restarted.get(&other.id).unwrap().phase, Phase::Expired);
    assert!(restarted.execute(&id, &f, &f.scope(), false).is_err());
    assert_eq!(f.calls.get(), 0);
}

#[test]
fn journal_failure_prevents_native_entry() {
    let (dir, mut store, f, id) = setup();
    std::fs::rename(store.directory(), dir.0.join("saved")).unwrap();
    std::fs::write(store.directory(), b"not a directory").unwrap();
    assert!(store.execute(&id, &f, &f.scope(), false).is_err());
    assert_eq!(f.calls.get(), 0);
    assert!(store.execute(&id, &f, &f.scope(), false).is_err());
    assert_eq!(f.calls.get(), 0);
}

#[test]
fn backend_refusal_is_distinct_from_an_unknown_attempt() {
    let (_dir, mut store, f, id) = setup();
    f.mode.set(1);
    let c = store.execute(&id, &f, &f.scope(), false).unwrap();
    assert_eq!(c.phase, Phase::Refused);
    assert!(!c.write.unwrap().attempted);
    assert_eq!(*f.bytes.borrow(), 50f32.to_le_bytes());
}

#[test]
fn capture_reservations_bound_late_writers_and_never_reuse_source_paths() {
    let (_dir, store, _f, id) = setup();
    let (first, _) = store.reserve_image(&id).unwrap();
    let (second, _) = store.reserve_image(&id).unwrap();
    assert_ne!(first, second);
    for _ in 0..10 {
        store.reserve_image(&id).unwrap();
    }
    assert!(store.reserve_image(&id).is_err());
}
