//! Synthetic selection contracts; they do not establish real-game authority.
use reversebolt_core::{Result, changes::Store, coverage::Region, live::*};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    rc::Rc,
};
const MIB: usize = 1024 * 1024;
const BASE: u64 = 0x10000;
const FIELD: u64 = BASE + 2 * MIB as u64 + 72;
#[derive(Clone, Copy)]
enum Mode {
    Affine,
    Nonlinear,
    Partial,
    Changed,
    Missing,
    Resized,
}
struct Fixture {
    info: ProcessInfo,
    value: Rc<Cell<f64>>,
    mode: Mode,
    reads: Cell<usize>,
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
    fn query(&self, _: u64) -> Result<Option<Region>> {
        let mut r = self.regions()?.remove(0);
        if matches!(self.mode, Mode::Changed) && self.reads.get() > MIB / 65536 {
            r.protection = 8;
        }
        Ok(Some(r))
    }
    fn read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        self.reads.set(self.reads.get() + 1);
        let mut bytes = vec![0; length];
        if address <= FIELD && FIELD + 8 <= address + length as u64 {
            let value = self.value.get();
            let raw = if matches!(self.mode, Mode::Nonlinear) {
                value * value
            } else {
                -3.25 * value + 8192.
            };
            let offset = (FIELD - address) as usize;
            bytes[offset..offset + 8].copy_from_slice(&raw.to_le_bytes());
        }
        if matches!(self.mode, Mode::Partial) && address >= BASE + MIB as u64 {
            bytes.pop();
        }
        Ok(bytes)
    }
    fn screen(&self, path: &Path) -> Result<Screen> {
        std::fs::write(path, b"\x89PNG\r\n\x1a\nfixture").unwrap();
        Ok(Screen {
            sampling: None,
            path: path.to_string_lossy().into_owned(),
            width: if matches!(self.mode, Mode::Resized) && self.value.get() != 50. {
                200
            } else {
                100
            },
            height: 100,
            text: String::new(),
            words: if matches!(self.mode, Mode::Missing) && self.value.get() != 50. {
                vec![]
            } else {
                vec![Word {
                    text: self.value.get().to_string(),
                    bounds: [10., 10., 30., 20.],
                    line: 0,
                    source: TextSource::WindowsOcr,
                }]
            },
            warning: None,
            method: "fixture".into(),
        })
    }
}
fn setup(mode: Mode) -> (LiveSession, Store, Rc<Cell<f64>>, PathBuf) {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/change-search-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let value = Rc::new(Cell::new(50.));
    let mut session = LiveSession::attach(
        Box::new(Fixture {
            info: ProcessInfo::default(),
            value: value.clone(),
            mode,
            reads: Cell::new(0),
        }),
        1,
        folder.join("captures"),
    )
    .unwrap();
    let store = Store::open(folder.join("changes")).unwrap();
    session.capture().unwrap();
    (session, store, value, folder)
}
fn start(session: &mut LiveSession, store: &Store) {
    let id = session.view.investigation_id.clone();
    session.start_change_search(&id, 1, 1, 0, store).unwrap();
}
fn collect(session: &mut LiveSession, value: &Cell<f64>, values: &[f64]) {
    for v in values {
        value.set(*v);
        session.capture().unwrap();
    }
}
#[test]
fn affine_encoded_field_beyond_old_coverage_needs_entirely_fresh_fit_and_checks() {
    let (mut s, store, value, folder) = setup(Mode::Affine);
    let id = s.view.investigation_id.clone();
    start(&mut s, &store);
    assert_eq!(s.view.planned_bytes, 2 * MIB);
    assert!(s.collecting_change_search());
    collect(&mut s, &value, &[20., 80., 30., 70.]);
    let old_ids: Vec<_> = s.view.frames.iter().map(|f| f.id.clone()).collect();
    assert!(s.capture().is_err());
    s.finish_change_search(&id, 2, &store).unwrap();
    assert_eq!(s.view.coverage.batch, 3);
    assert_eq!(s.view.sample_count, 0);
    assert!(s.view.tracks.is_empty());
    assert!(s.view.prediction_checks.is_empty());
    assert_eq!(s.view.planned_bytes, 64);
    assert_eq!(s.view.change_searches[0].complete_bytes, 2 * MIB);
    assert!(s.finish_change_search(&id, 2, &store).is_err());
    assert!(s.correlate().is_err());
    let values = [
        40., 56., 31., 85., 64., 27., 75., 92., 44., 67., 38., 50., 54., 54., 50., 50.,
    ];
    for (i, v) in values.into_iter().enumerate() {
        value.set(v);
        s.capture_for_discovery(15 - i).unwrap();
    }
    assert!(s.view.frames.iter().all(|f| !old_ids.contains(&f.id)));
    let c = s
        .view
        .prediction_checks
        .iter()
        .flat_map(|c| &c.candidates)
        .find(|c| {
            c.address == format!("0x{FIELD:X}")
                && c.mapping.kind == reversebolt_core::discovery::Kind::F64
        })
        .unwrap();
    assert_eq!(c.samples.len(), 4);
    assert!(
        c.samples
            .iter()
            .all(|s| format!("{:?}", s.outcome) == "Agrees")
    );
    let receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&s.view.change_searches[0].evidence).unwrap())
            .unwrap();
    assert_eq!(receipt["witnesses"][0].as_array().unwrap().len(), 4);
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn missing_partial_changed_resized_and_nonlinear_data_do_not_become_selected_windows() {
    for mode in [
        Mode::Nonlinear,
        Mode::Partial,
        Mode::Changed,
        Mode::Missing,
        Mode::Resized,
    ] {
        let (mut s, store, value, folder) = setup(mode);
        let id = s.view.investigation_id.clone();
        start(&mut s, &store);
        collect(&mut s, &value, &[20., 80., 30., 70.]);
        assert!(s.finish_change_search(&id, 2, &store).is_err());
        assert_eq!(s.view.coverage.batch, 2);
        assert!(s.view.prediction_checks.is_empty());
        assert_eq!(s.view.change_searches[0].retained_windows, 0);
        std::fs::remove_dir_all(folder).unwrap();
    }
}
#[test]
fn constant_or_single_outlier_observations_abstain_without_reusing_old_contrast() {
    for values in [[50., 50., 50., 50.], [50., 50., 50., 90.]] {
        let (mut s, store, value, folder) = setup(Mode::Affine);
        let id = s.view.investigation_id.clone();
        start(&mut s, &store);
        collect(&mut s, &value, &values);
        assert!(
            s.finish_change_search(&id, 2, &store)
                .unwrap_err()
                .contains("repeated changes")
        );
        assert_eq!(s.view.change_searches[0].status, "no_selection");
        assert!(Path::new(&s.view.change_searches[0].evidence).exists());
        std::fs::remove_dir_all(folder).unwrap();
    }
}
#[test]
fn stale_scope_read_budget_and_failed_persistence_preserve_the_current_batch() {
    let (mut s, store, _, folder) = setup(Mode::Affine);
    let id = s.view.investigation_id.clone();
    assert!(s.start_change_search(&id, 1, 2, 0, &store).is_err());
    s.view.coverage.attempted_bytes = reversebolt_core::coverage::MAX_READ_BYTES;
    assert!(s.start_change_search(&id, 1, 1, 0, &store).is_err());
    s.view.coverage.attempted_bytes = MIB as u64;
    std::fs::create_dir(folder.join("captures/change-search-01-start.json")).unwrap();
    assert!(s.start_change_search(&id, 1, 1, 0, &store).is_err());
    assert_eq!(s.view.coverage.batch, 1);
    assert_eq!(s.view.sample_count, 1);
    assert!(s.view.change_searches.is_empty());
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn common_change_signatures_are_sampled_deterministically_with_explicit_omissions() {
    use reversebolt_core::{change_search, coverage::Chunk};
    use std::collections::{BTreeMap, BTreeSet};
    let (mut s, store, _, folder) = setup(Mode::Affine);
    start(&mut s, &store);
    let mut summary = s.view.change_searches[0].clone();
    summary.observations = vec![10., 30., 20., 40.];
    summary.basis = vec![summary.source_frame.clone(); 4];
    let plan = vec![Chunk {
        address: BASE,
        length: 8192,
        region: Region {
            base: BASE,
            length: 8192,
            allocation_base: BASE,
            protection: 4,
            kind: 0x20000,
            image: false,
        },
    }];
    let snapshots: Vec<_> = summary
        .observations
        .iter()
        .map(|v| {
            let bytes = ((v * 7. + 123.) as i32).to_le_bytes().repeat(2048);
            BTreeMap::from([(BASE, bytes)])
        })
        .collect();
    let a = change_search::select(&plan, &snapshots, &BTreeSet::new(), summary.clone(), 4).unwrap();
    let b = change_search::select(&plan, &snapshots, &BTreeSet::new(), summary.clone(), 4).unwrap();
    assert_eq!(a.summary.matched_windows, 128);
    assert_eq!(a.summary.omitted_windows, 124);
    assert_eq!(
        a.windows.iter().map(|w| w.address).collect::<Vec<_>>(),
        b.windows.iter().map(|w| w.address).collect::<Vec<_>>()
    );
    assert_eq!(a.witnesses.len(), 4);
    let invalid =
        change_search::select(&plan, &snapshots, &BTreeSet::from([BASE]), summary, 4).unwrap();
    assert_eq!(invalid.summary.complete_bytes, 0);
    assert!(invalid.windows.is_empty());
    std::fs::remove_dir_all(folder).unwrap();
}
