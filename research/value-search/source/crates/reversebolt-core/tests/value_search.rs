//! Synthetic contracts, not native-game or semantic-identification evidence.
use reversebolt_core::{
    Result,
    changes::Store,
    coverage::{Ranges, Region},
    live::*,
    value_search::{self, Summary},
};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    rc::Rc,
};
const MIB: usize = 1024 * 1024;
const BASE: u64 = 0x10000;
const FIELD: u64 = BASE + 3 * MIB as u64 + 72;

#[derive(Clone, Copy)]
enum Mode {
    Normal,
    Float,
    Double,
    Encoded,
    Partial,
    Changed,
    Dead,
    Zero,
}
struct Fixture {
    info: ProcessInfo,
    frame: Cell<usize>,
    calls: Rc<Cell<usize>>,
    mode: Mode,
}
impl Fixture {
    fn new(mode: Mode) -> Self {
        Self {
            info: ProcessInfo::default(),
            frame: Cell::new(0),
            calls: Rc::new(Cell::new(0)),
            mode,
        }
    }
    fn value(&self) -> i32 {
        [
            40, 56, 31, 85, 64, 27, 75, 92, 44, 67, 38, 50, 54, 54, 50, 50,
        ][self.frame.get().saturating_sub(1) % 16]
    }
}
impl NativeTarget for Fixture {
    fn info(&self) -> &ProcessInfo {
        &self.info
    }
    fn alive(&self) -> bool {
        !matches!(self.mode, Mode::Dead)
    }
    fn regions(&self) -> Result<Vec<Region>> {
        Ok(vec![Region {
            base: BASE,
            length: 4 * MIB,
            allocation_base: BASE,
            protection: 4,
            kind: 0x20000,
            image: false,
        }])
    }
    fn query(&self, _address: u64) -> Result<Option<Region>> {
        let mut region = self.regions()?.remove(0);
        if matches!(self.mode, Mode::Changed) && self.calls.get() > 0 {
            region.protection = 8;
        }
        Ok(Some(region))
    }
    fn read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        self.calls.set(self.calls.get() + 1);
        let mut bytes = vec![0; length];
        if !matches!(self.mode, Mode::Zero)
            && address <= FIELD
            && FIELD + 4 <= address + length as u64
        {
            let offset = (FIELD - address) as usize;
            let encoded = match self.mode {
                Mode::Float => (self.value() as f32).to_le_bytes().to_vec(),
                Mode::Double => (self.value() as f64).to_le_bytes().to_vec(),
                Mode::Encoded => (self.value() / 2).to_le_bytes().to_vec(),
                _ => self.value().to_le_bytes().to_vec(),
            };
            bytes[offset..offset + encoded.len()].copy_from_slice(&encoded);
        }
        if matches!(self.mode, Mode::Partial) {
            bytes.pop();
        }
        Ok(bytes)
    }
    fn screen(&self, path: &Path) -> Result<Screen> {
        self.frame.set(self.frame.get() + 1);
        let mut png = b"\x89PNG\r\n\x1a\nfixture".to_vec();
        png.extend(self.frame.get().to_le_bytes());
        std::fs::write(path, png).unwrap();
        Ok(Screen {
            sampling: None,
            path: path.to_string_lossy().into_owned(),
            width: 100,
            height: 100,
            text: String::new(),
            words: vec![Word {
                text: self.value().to_string(),
                bounds: [10., 10., 30., 20.],
                line: 0,
                source: TextSource::WindowsOcr,
            }],
            warning: None,
            method: "fixture".into(),
        })
    }
}
fn scan(target: &Fixture, value: f64, max: usize) -> value_search::Outcome {
    value_search::scan(
        target,
        &Ranges::default(),
        Summary {
            value,
            ..Summary::default()
        },
        4 * MIB,
        max,
    )
    .unwrap()
}
fn setup() -> (LiveSession, Store, PathBuf, Rc<Cell<usize>>) {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/value-search-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let target = Fixture::new(Mode::Normal);
    let calls = target.calls.clone();
    let session = LiveSession::attach(Box::new(target), 1, folder.join("captures")).unwrap();
    let store = Store::open(folder.join("changes")).unwrap();
    (session, store, folder, calls)
}

#[test]
fn scan_reaches_late_memory_and_never_uses_partial_changed_or_dead_reads() {
    let good = scan(&Fixture::new(Mode::Normal), 40., 16);
    assert_eq!(good.windows.len(), 1);
    assert!(
        good.windows[0].address <= FIELD
            && FIELD < good.windows[0].address + good.windows[0].length as u64
    );
    assert_eq!(good.summary.read_bytes, 4 * MIB as u64);
    for mode in [Mode::Partial, Mode::Changed, Mode::Dead] {
        let bad = scan(&Fixture::new(mode), 40., 16);
        assert!(bad.windows.is_empty());
        assert_eq!(bad.summary.read_bytes, 0);
    }
}

#[test]
fn common_values_are_bounded_spread_out_and_omissions_are_explicit() {
    let first = scan(&Fixture::new(Mode::Zero), 0., 16);
    let second = scan(&Fixture::new(Mode::Zero), 0., 16);
    assert_eq!(first.summary.matched_windows, 4 * MIB / 64);
    assert_eq!(first.summary.retained_windows, 16);
    assert_eq!(first.summary.omitted_windows, 4 * MIB / 64 - 16);
    assert_eq!(
        first.windows.iter().map(|w| w.address).collect::<Vec<_>>(),
        second.windows.iter().map(|w| w.address).collect::<Vec<_>>()
    );
    assert!(first.windows.last().unwrap().address > BASE + 2 * MIB as u64);
    let mut excluded = Ranges::default();
    excluded.add(BASE, 3 * MIB);
    let next = value_search::scan(
        &Fixture::new(Mode::Zero),
        &excluded,
        Summary {
            value: 0.,
            ..Summary::default()
        },
        MIB,
        16,
    )
    .unwrap();
    assert_eq!(next.summary.attempted_bytes, MIB as u64);
    assert!(
        next.windows
            .iter()
            .all(|w| w.address >= BASE + 3 * MIB as u64)
    );
}

#[test]
fn all_supported_direct_types_seed_but_unknown_encoding_is_not_claimed() {
    for mode in [Mode::Normal, Mode::Float, Mode::Double] {
        assert_eq!(scan(&Fixture::new(mode), 40., 16).windows.len(), 1);
    }
    assert!(
        scan(&Fixture::new(Mode::Encoded), 40., 16)
            .windows
            .is_empty()
    );
    assert!(
        scan(&Fixture::new(Mode::Encoded), 20.5, 16)
            .windows
            .is_empty()
    );
}

#[test]
fn selected_windows_need_wholly_new_trajectories_and_old_requests_expire() {
    let (mut session, store, folder, _) = setup();
    session.capture().unwrap();
    let id = session.view.investigation_id.clone();
    let track = session.view.tracks[0].id;
    session.search_number(&id, 1, 1, track, &store).unwrap();
    assert_eq!(session.view.coverage.batch, 2);
    assert_eq!(session.view.sample_count, 0);
    assert!(session.view.tracks.is_empty());
    assert_eq!(session.view.retained_bytes, 0);
    assert_eq!(session.view.planned_bytes, 64);
    assert_eq!(session.view.value_searches[0].read_bytes, 3 * MIB as u64);
    assert!(session.view.prediction_checks.is_empty());
    assert!(session.correlate().is_err());
    assert!(session.search_number(&id, 1, 1, track, &store).is_err());
    for _ in 0..12 {
        session.capture().unwrap();
    }
    session.correlate().unwrap();
    assert!(session.view.tracks.iter().any(|t| {
        t.matches
            .iter()
            .any(|m| m.address == format!("0x{FIELD:X}"))
    }));
    assert!(Path::new(&session.view.value_searches[0].evidence).exists());
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn stale_frames_and_exhausted_budget_do_not_read_or_replace_findings() {
    let (mut session, store, folder, calls) = setup();
    session.capture().unwrap();
    let id = session.view.investigation_id.clone();
    let track = session.view.tracks[0].id;
    let before = calls.get();
    assert!(session.search_number(&id, 1, 2, track, &store).is_err());
    session.view.coverage.attempted_bytes = reversebolt_core::coverage::MAX_READ_BYTES;
    assert!(session.search_number(&id, 1, 1, track, &store).is_err());
    assert_eq!(calls.get(), before);
    assert_eq!(session.view.coverage.batch, 1);
    assert_eq!(session.view.sample_count, 1);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn failed_evidence_save_keeps_old_batch_but_charges_completed_search_reads() {
    let (mut session, store, folder, _) = setup();
    session.capture().unwrap();
    let id = session.view.investigation_id.clone();
    let track = session.view.tracks[0].id;
    let before = session.view.coverage.attempted_bytes;
    std::fs::create_dir(folder.join("captures/value-search-01.json")).unwrap();
    assert!(session.search_number(&id, 1, 1, track, &store).is_err());
    assert_eq!(session.view.coverage.batch, 1);
    assert_eq!(session.view.sample_count, 1);
    assert_eq!(session.view.value_searches.len(), 1);
    assert_eq!(
        session.view.coverage.attempted_bytes - before,
        3 * MIB as u64
    );
    std::fs::remove_dir_all(folder).unwrap();
}
