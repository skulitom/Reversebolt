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
    secondary: Cell<Option<f64>>,
    screen_bias: Cell<f32>,
    mirror: Cell<bool>,
}
impl State {
    fn set(&self, value: f32) {
        self.bytes.borrow_mut()[32..36].copy_from_slice(&value.to_le_bytes());
        if self.mirror.get() {
            self.bytes.borrow_mut()[64..68].copy_from_slice(&value.to_le_bytes());
        }
    }
}
struct Target {
    state: Rc<State>,
    info: ProcessInfo,
    memory: Option<ProcessInfo>,
}
impl NativeTarget for Target {
    fn memory_info(&self) -> &ProcessInfo {
        self.memory.as_ref().unwrap_or(&self.info)
    }
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
        let value = f32::from_le_bytes(self.state.bytes.borrow()[32..36].try_into().unwrap())
            + self.state.screen_bias.get();
        let mut words = vec![Word {
            text: value.to_string(),
            bounds: [10., 10., 40., 20.],
            line: 0,
            source: TextSource::WindowsOcr,
        }];
        if let Some(value) = self.state.secondary.get() {
            words.push(Word {
                text: value.to_string(),
                bounds: [10., 55., 40., 20.],
                line: 1,
                source: TextSource::WindowsOcr,
            });
        }
        Ok(Screen {
            path: path.to_string_lossy().into(),
            width: 100,
            height: if self.state.resize.get() { 110 } else { 100 },
            text: String::new(),
            words,
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
        self.state.secondary.set(Some(11.));
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
    session_contract(false);
}

#[test]
fn child_address_space_keeps_screen_evidence_but_binds_edits_to_its_own_identity() {
    session_contract(true);
}

#[test]
fn explicit_probes_test_a_location_without_erasing_nonoverlapping_rivals() {
    for selected_offset in [32, 64] {
        let root = std::env::temp_dir().join(format!("reversebolt-probe-{}", uuid::Uuid::new_v4()));
        let state = Rc::new(State {
            bytes: RefCell::new(vec![0; 128]),
            calls: Cell::new(0),
            resize: Cell::new(false),
            dead: Cell::new(false),
            secondary: Cell::new(Some(25.)),
            screen_bias: Cell::new(0.),
            mirror: Cell::new(true),
        });
        let mut session = LiveSession::attach(
            Box::new(Target {
                state: state.clone(),
                memory: None,
                info: ProcessInfo {
                    pid: 91,
                    created: "probe-fixture".into(),
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
        let track = session.view.tracks[0].id;
        session
            .start_prediction_check(
                &session.view.investigation_id.clone(),
                1,
                12,
                Source::Ocr,
                track,
            )
            .unwrap();
        for value in [52., 52., 50., 50.] {
            state.set(value);
            session.capture().unwrap();
        }
        let check = session.view.prediction_check.as_ref().unwrap();
        let id = check.id.clone();
        let selected = check
            .candidates
            .iter()
            .position(|c| c.address == format!("0x{:X}", 65536 + selected_offset))
            .unwrap();
        let choices = reversebolt_core::probes::choices(check).unwrap();
        assert_eq!(choices.len(), 2);
        assert!(choices.iter().all(|c| c.eligible && c.indices.len() >= 2));
        let ordinary = session
            .prepare_change(track, 53., "test", &mut store)
            .unwrap();
        assert_eq!(
            ordinary.phase,
            Phase::Unavailable,
            "Different addresses cannot imply consensus"
        );
        assert!(
            session
                .prepare_probe(track, 53., ("stale", selected), "test", &mut store)
                .is_err()
        );
        assert!(
            session
                .prepare_probe(track, 53., (&id, 200), "test", &mut store)
                .is_err()
        );
        // An unresolved overlapping interpretation blocks the probe; a rival at
        // another location stays in the evidence without voting on this patch.
        let group = reversebolt_core::probes::select(
            session.view.prediction_check.as_ref().unwrap(),
            selected,
        )
        .unwrap();
        let alias = *group.iter().find(|i| **i != selected).unwrap();
        let samples = std::mem::take(
            &mut session.view.prediction_check.as_mut().unwrap().candidates[alias].samples,
        );
        assert!(
            session
                .prepare_probe(track, 53., (&id, selected), "test", &mut store)
                .is_err()
        );
        session.view.prediction_check.as_mut().unwrap().candidates[alias].samples = samples;
        let rival = choices
            .iter()
            .find(|c| !c.indices.contains(&selected))
            .unwrap()
            .indices[0];
        session.view.prediction_check.as_mut().unwrap().candidates[rival]
            .samples
            .clear();
        let prepared = session
            .prepare_probe(track, 53., (&id, selected), "test", &mut store)
            .unwrap();
        assert_eq!(prepared.phase, Phase::Prepared);
        assert_eq!(state.calls.get(), 0, "Preparation is read-only");
        let probe = prepared.probe.as_ref().unwrap();
        assert_eq!(
            probe.alternatives.len(),
            session
                .view
                .prediction_check
                .as_ref()
                .unwrap()
                .candidates
                .len()
        );
        assert_eq!(
            probe.alternatives[rival].assessment,
            reversebolt_core::prediction::Assessment::NeedsContrastingEvidence
        );
        for a in &probe.alternatives {
            assert!(
                (a.predicted_if_written.unwrap() - if a.in_test_group { 53. } else { 50. }).abs()
                    < 0.55
            );
        }
        let applied = session
            .execute_change(&prepared.id, false, &mut store)
            .unwrap();
        assert_eq!(applied.phase, Phase::Written);
        let observed = session.observe_change(&prepared.id, &mut store).unwrap();
        assert_eq!(observed.observations[0].agrees, selected_offset == 32);
        assert_eq!(
            observed.observations[0].value,
            Some(if selected_offset == 32 { 53. } else { 50. })
        );
        for result in &observed.observations[0].probe_comparisons {
            let agrees = probe.alternatives[result.index].in_test_group == (selected_offset == 32);
            assert_eq!(
                result.fit,
                if agrees {
                    reversebolt_core::probes::Fit::Agreed
                } else {
                    reversebolt_core::probes::Fit::Differed
                }
            );
        }
        assert!(
            session
                .view
                .prediction_check
                .as_ref()
                .unwrap()
                .invalidated
                .is_some()
        );
        assert!(
            session
                .prepare_probe(track, 56., (&id, selected), "test", &mut store)
                .is_err()
        );
        session
            .execute_change(&prepared.id, false, &mut store)
            .unwrap();
        assert_eq!(state.calls.get(), 1, "Completed probes are never replayed");
        let restored = session
            .execute_change(&prepared.id, true, &mut store)
            .unwrap();
        assert_eq!(restored.phase, Phase::Restored);
        assert_eq!(
            f32::from_le_bytes(
                state.bytes.borrow()[selected_offset..selected_offset + 4]
                    .try_into()
                    .unwrap()
            ),
            50.
        );
        drop(store);
        let reopened = Store::open(root.join("changes")).unwrap();
        assert_eq!(
            reopened
                .get(&prepared.id)
                .unwrap()
                .probe
                .unwrap()
                .alternatives
                .len(),
            probe.alternatives.len()
        );
        assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(root).unwrap();
    }
}

fn session_contract(separate: bool) {
    let root =
        std::env::temp_dir().join(format!("reversebolt-live-change-{}", uuid::Uuid::new_v4()));
    let state = Rc::new(State {
        bytes: RefCell::new(vec![0; 128]),
        calls: Cell::new(0),
        resize: Cell::new(false),
        dead: Cell::new(false),
        secondary: Cell::new(Some(25.)),
        screen_bias: Cell::new(0.),
        mirror: Cell::new(false),
    });
    let mut session = LiveSession::attach(
        Box::new(Target {
            memory: separate.then(|| ProcessInfo {
                pid: 92,
                created: "child-a".into(),
                ..Default::default()
            }),
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
    assert_eq!(session.view.process.pid, 91);
    assert_eq!(
        session.view.memory_process.as_ref().unwrap().pid,
        if separate { 92 } else { 91 }
    );
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
    state.screen_bias.set(2.);
    let stale_screen = session
        .prepare_change(id, 53., "agent", &mut store)
        .unwrap();
    assert_eq!(stale_screen.phase, Phase::Unavailable);
    assert!(!stale_screen.baseline.as_ref().unwrap().agrees);
    assert!(
        session
            .execute_change(&stale_screen.id, false, &mut store)
            .is_err()
    );
    assert_eq!(state.calls.get(), 0);
    state.screen_bias.set(0.);
    let change = session
        .prepare_change(id, 53., "agent", &mut store)
        .unwrap();
    assert_eq!(change.phase, Phase::Prepared);
    assert_eq!(change.scope.process.pid, if separate { 92 } else { 91 });
    if separate {
        let mut wrong = Target {
            state: state.clone(),
            info: session.view.process.clone(),
            memory: Some(ProcessInfo {
                pid: 92,
                created: "child-restarted".into(),
                ..Default::default()
            }),
        };
        let scope = session.change_scope(id).unwrap();
        assert!(change.check_target(&wrong, &scope).is_err());
        wrong.memory.as_mut().unwrap().created = "child-a".into();
        wrong.memory.as_mut().unwrap().pid = 93;
        assert!(change.check_target(&wrong, &scope).is_err());
        assert_eq!(state.calls.get(), 0);
        let evidence: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("captures/evidence.json")).unwrap())
                .unwrap();
        assert_eq!(evidence["process"]["pid"], 91);
        assert_eq!(evidence["memory_process"]["pid"], 92);
    }
    assert_eq!(state.calls.get(), 0);
    assert!(change.models.len() >= 2);
    assert_eq!(change.watches.len(), 2);
    assert_eq!(
        change.baseline.as_ref().unwrap().readings[1].value,
        Some(25.)
    );
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
    assert_eq!(
        checked.observations[0]
            .comparison
            .as_ref()
            .unwrap()
            .other_values[0]
            .status,
        reversebolt_core::effects::Status::Changed
    );
    assert!(
        change
            .baseline
            .as_ref()
            .unwrap()
            .capture_interval
            .as_ref()
            .unwrap()
            .end_us
            <= checked.observations[0]
                .capture_interval
                .as_ref()
                .unwrap()
                .begin_us
    );
    assert!(
        session
            .prepare_change(id, 56., "agent", &mut store)
            .is_err()
    );
    session
        .execute_change(&change.id, false, &mut store)
        .unwrap();
    assert_eq!(state.calls.get(), 1);
    state.secondary.set(None);
    let missing = session.observe_change(&change.id, &mut store).unwrap();
    assert_eq!(
        missing.observations[1]
            .comparison
            .as_ref()
            .unwrap()
            .other_values[0]
            .status,
        reversebolt_core::effects::Status::Unreadable
    );
    state.resize.set(true);
    let resized = session.observe_change(&change.id, &mut store).unwrap();
    assert!(!resized.observations[2].agrees);
    assert!(resized.observations[2].error.is_some());
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
    // Evidence remains inspectable after target loss and engine restart. It
    // cannot silently load replaced files or escape the operation directory.
    drop(store);
    let mut reopened = Store::open(root.join("changes")).unwrap();
    let (baseline, after, images) = reopened.read_images(&change.id, 0).unwrap();
    assert_eq!(images.len(), 2);
    assert!(after.comparison.unwrap().other_values[0].before == Some(25.));
    std::fs::write(&baseline.image, b"\x89PNG\r\n\x1a\nchanged").unwrap();
    assert!(reopened.read_images(&change.id, 0).is_err());
    let mut substituted = reopened.get(&change.id).unwrap();
    let outside = root.join("outside.png");
    std::fs::write(&outside, &images[0]).unwrap();
    substituted.baseline.as_mut().unwrap().image = outside.to_string_lossy().into();
    reopened.record(substituted).unwrap();
    assert!(reopened.read_images(&change.id, 0).is_err());
    assert!(reopened.read_images(&change.id, 99).is_err());
    assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
    std::fs::remove_dir_all(root).unwrap();
}
