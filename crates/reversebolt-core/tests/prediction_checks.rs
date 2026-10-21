//! Controlled native-target contract tests, not game-discovery evidence.
use reversebolt_core::{
    Result,
    live::*,
    prediction::{Assessment, Source},
    visual::*,
};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Default)]
struct Flags {
    frame: Cell<usize>,
    dead: Cell<bool>,
    resize: Cell<bool>,
    remap: Cell<bool>,
    missing: Cell<bool>,
    change_during_image: Cell<bool>,
    read_bias: Cell<i32>,
    missing_prefix: Cell<bool>,
    multiple: Cell<bool>,
    secondary_mismatch: Cell<bool>,
}
struct Target {
    info: ProcessInfo,
    flags: Rc<Flags>,
}
fn value(frame: usize) -> i32 {
    if frame <= 12 {
        13 + frame as i32 * 7
    } else {
        [104, 104, 118, 118, 130, 130, 140, 140][(frame - 13).min(7)]
    }
}
fn second_value(frame: usize) -> i32 {
    if frame <= 12 {
        [72, 29, 81, 44, 60, 35, 91, 67, 25, 88, 53, 76][frame.saturating_sub(1)]
    } else {
        [64, 64, 43, 43, 35, 35, 56, 56][(frame - 13).min(7)]
    }
}
impl NativeTarget for Target {
    fn info(&self) -> &ProcessInfo {
        &self.info
    }
    fn alive(&self) -> bool {
        !self.flags.dead.get()
    }
    fn regions(&self) -> Result<Vec<Region>> {
        Ok(vec![Region {
            base: 0x10000,
            length: 128,
            allocation_base: if self.flags.remap.get() {
                0x9000
            } else {
                0x10000
            },
            ..Region::default()
        }])
    }
    fn screen(&self, path: &Path) -> Result<Screen> {
        let frame = self.flags.frame.get();
        let observed = value(frame) + self.flags.read_bias.get();
        if self.flags.change_during_image.get() {
            self.flags.read_bias.set(self.flags.read_bias.get() + 10);
        }
        let mut bytes = b"\x89PNG\r\n\x1a\ncontract fixture".to_vec();
        bytes.extend(frame.to_le_bytes());
        std::fs::write(path, bytes).map_err(|e| e.to_string())?;
        let mut words = vec![Word {
            text: observed.to_string(),
            bounds: [10., 10., 40., 20.],
            line: 0,
            source: TextSource::WindowsOcr,
        }];
        if self.flags.multiple.get() {
            words.push(Word {
                text: second_value(frame).to_string(),
                bounds: [10., 50., 40., 20.],
                line: 1,
                source: TextSource::WindowsOcr,
            });
        }
        Ok(Screen {
            sampling: None,
            path: path.to_string_lossy().into(),
            width: 100,
            height: if self.flags.resize.get() { 110 } else { 100 },
            text: String::new(),
            warning: None,
            method: "contract fixture".into(),
            words,
        })
    }
    fn read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        if self.flags.missing.get() || (self.flags.missing_prefix.get() && length <= 8) {
            return Err("injected incomplete read".into());
        }
        let frame = self.flags.frame.get();
        let mut bytes = [0; 128];
        // Two observationally indistinguishable copies, plus a time confound.
        for offset in [32, 48] {
            bytes[offset..offset + 4]
                .copy_from_slice(&(value(frame) + self.flags.read_bias.get()).to_le_bytes());
        }
        bytes[64..68].copy_from_slice(&(13 + frame as i32 * 7).to_le_bytes());
        if self.flags.multiple.get() {
            let v = second_value(frame)
                + if self.flags.secondary_mismatch.get() {
                    9
                } else {
                    0
                };
            bytes[80..84].copy_from_slice(&v.to_le_bytes());
        }
        let offset = (address - 0x10000) as usize;
        Ok(bytes[offset..offset + length].to_vec())
    }
}
fn setup() -> (LiveSession, Rc<Flags>, PathBuf) {
    setup_mode(CaptureMode::Full)
}

fn capture(session: &mut LiveSession, flags: &Flags, mode: CaptureMode) {
    // Fixture state advances between captures, independently of screen/read calls.
    flags.frame.set(flags.frame.get() + 1);
    session.capture_mode(mode).unwrap();
}

fn setup_mode(mode: CaptureMode) -> (LiveSession, Rc<Flags>, PathBuf) {
    let (mut session, flags, root) = fresh();
    for _ in 0..12 {
        capture(&mut session, &flags, mode);
    }
    (session, flags, root)
}
fn fresh() -> (LiveSession, Rc<Flags>, PathBuf) {
    let flags = Rc::new(Flags::default());
    let root =
        std::env::temp_dir().join(format!("reversebolt-prediction-{}", uuid::Uuid::new_v4()));
    let target = Target {
        info: ProcessInfo::default(),
        flags: flags.clone(),
    };
    let session = LiveSession::attach(Box::new(target), 1, root.clone()).unwrap();
    (session, flags, root)
}

#[test]
fn automatic_discovery_freezes_before_fresh_evidence_without_manual_commands() {
    let (mut session, flags, root) = fresh();
    for n in 1..=12 {
        flags.frame.set(n);
        session.capture_for_discovery(16 - n).unwrap();
        if n < 12 {
            assert!(session.view.prediction_checks.is_empty());
        }
    }
    let frozen = session.view.prediction_checks.first().unwrap().clone();
    assert_eq!(frozen.after_frame, 12);
    assert!(frozen.candidates.iter().all(|c| c.samples.is_empty()));
    let selection = frozen.automatic_selection.as_ref().unwrap();
    assert_eq!(selection.scheduled_fresh_captures, 4);
    assert_eq!(selection.ranked[0].track, frozen.track);
    assert!(root.join("prediction-checks.json").exists());
    for n in 13..=16 {
        flags.frame.set(n);
        session.capture_for_discovery(16 - n).unwrap();
    }
    let check = session.view.prediction_checks.first().unwrap();
    assert_eq!(check.id, frozen.id);
    for (before, after) in frozen.candidates.iter().zip(&check.candidates) {
        assert_eq!(
            serde_json::to_value(&before.mapping).unwrap(),
            serde_json::to_value(&after.mapping).unwrap()
        );
        assert_eq!(
            after.samples.iter().map(|s| s.frame).collect::<Vec<_>>(),
            vec![13, 14, 15, 16]
        );
    }
    assert!(
        check
            .candidates
            .iter()
            .any(|c| c.assessment(check) == Assessment::ConsistentInObservedScope)
    );
    assert!(
        check
            .candidates
            .iter()
            .any(|c| c.assessment(check) == Assessment::Contradicted)
    );
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("prediction-checks.json")).unwrap())
            .unwrap();
    assert_eq!(
        saved["checks"][0]["automatic_selection"]["ranked"][0]["track"],
        check.track
    );
    // Cancelling the job does not enqueue captures or experiments in the core.
    assert_eq!(session.view.sample_count, 16);
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn several_automatic_checks_keep_separate_evidence_and_track_scopes() {
    for mismatch in [false, true] {
        let (mut session, flags, root) = fresh();
        flags.multiple.set(true);
        for n in 1..=12 {
            flags.frame.set(n);
            session.capture_for_discovery(16 - n).unwrap();
        }
        assert_eq!(session.view.prediction_checks.len(), 2);
        let frozen = session.view.prediction_checks.clone();
        assert_ne!(frozen[0].id, frozen[1].id);
        for check in &frozen {
            assert_eq!(check.after_frame, 12);
            assert!(check.candidates.iter().all(|c| c.samples.is_empty()));
            assert_eq!(
                check
                    .automatic_selection
                    .as_ref()
                    .unwrap()
                    .planned_tracks
                    .len(),
                2
            );
            assert_eq!(
                session.change_scope(check.track).unwrap().prediction,
                check.id
            );
        }
        flags.secondary_mismatch.set(mismatch);
        for n in 13..=16 {
            flags.frame.set(n);
            session.capture_for_discovery(16 - n).unwrap();
            let reads = &session.view.latest.as_ref().unwrap().prediction_reads;
            let addresses = reads
                .iter()
                .map(|r| (&r.address, r.requested))
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                addresses.len(),
                reads.len(),
                "Equal address/width reads are shared across series"
            );
            assert!(reads.len() <= reversebolt_core::prediction::MAX_CANDIDATES);
        }
        for check in &session.view.prediction_checks {
            let original = frozen.iter().find(|c| c.id == check.id).unwrap();
            for (before, after) in original.candidates.iter().zip(&check.candidates) {
                assert_eq!(
                    serde_json::to_value(&before.mapping).unwrap(),
                    serde_json::to_value(&after.mapping).unwrap()
                );
                assert_eq!(after.samples.len(), 4);
                assert!(after.samples.iter().all(|s| s.supports_comparison()));
                for sample in &after.samples {
                    let expected = if check.track == 0 {
                        value(sample.frame)
                    } else {
                        second_value(sample.frame)
                    };
                    assert_eq!(sample.observed, Some(expected as f64));
                }
            }
            if mismatch && check.track == 1 {
                assert!(
                    check
                        .candidates
                        .iter()
                        .all(|c| c.assessment(check) == Assessment::Contradicted)
                );
            } else {
                assert!(
                    check
                        .candidates
                        .iter()
                        .any(|c| c.assessment(check) == Assessment::ConsistentInObservedScope)
                );
            }
        }
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("prediction-checks.json")).unwrap())
                .unwrap();
        assert_eq!(saved["checks"].as_array().unwrap().len(), 2);
        // New status has one list; historical single-check sessions still deserialize.
        let mut legacy = serde_json::to_value(&session.view).unwrap();
        legacy.as_object_mut().unwrap().remove("prediction_checks");
        legacy["prediction_check"] = serde_json::to_value(&frozen[0]).unwrap();
        let old: LiveView = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(old.prediction_checks[0].id, frozen[0].id);
        legacy["prediction_check"] = serde_json::Value::Null;
        assert!(
            serde_json::from_value::<LiveView>(legacy)
                .unwrap()
                .prediction_checks
                .is_empty()
        );
        assert_eq!(
            serde_json::to_value(&session.view)
                .unwrap()
                .get("prediction_check"),
            None
        );
        assert!(root.starts_with(std::env::temp_dir()));
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn additional_series_share_limits_and_failed_persistence_keeps_existing_check() {
    let (mut session, flags, root) = fresh();
    flags.multiple.set(true);
    for _ in 0..12 {
        capture(&mut session, &flags, CaptureMode::Full);
    }
    session.correlate().unwrap();
    start(&mut session, Source::Ocr).unwrap();
    let first = session.view.prediction_checks[0].clone();
    let original = session.view.tracks[1].matches.clone();
    session.view.tracks[1].matches =
        vec![original[0].clone(); reversebolt_core::prediction::MAX_CANDIDATES];
    let add = |session: &mut LiveSession| {
        session.start_prediction_check(
            &session.view.investigation_id.clone(),
            1,
            12,
            Source::Ocr,
            1,
        )
    };
    assert!(add(&mut session).unwrap_err().contains("200-candidate"));
    assert_eq!(session.view.prediction_checks.len(), 1);
    session.view.tracks[1].matches = original;
    std::fs::rename(root.join("prediction-checks.json"), root.join("saved.json")).unwrap();
    std::fs::create_dir(root.join("prediction-checks.json")).unwrap();
    assert!(add(&mut session).is_err());
    assert_eq!(session.view.prediction_checks.len(), 1);
    assert_eq!(session.view.prediction_checks[0].id, first.id);
    std::fs::remove_dir(root.join("prediction-checks.json")).unwrap();
    std::fs::rename(root.join("saved.json"), root.join("prediction-checks.json")).unwrap();
    add(&mut session).unwrap();
    assert_eq!(session.view.prediction_checks.len(), 2);
    session.view.prediction_checks[0].invalidated = Some("trial changed state".into());
    let mut third = session.view.tracks[1].clone();
    third.id = 2;
    session.view.tracks.push(third);
    assert!(
        session
            .start_prediction_check(
                &session.view.investigation_id.clone(),
                1,
                12,
                Source::Ocr,
                2
            )
            .is_err()
    );
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn overlapping_sources_share_reads_but_keep_distinct_check_identity() {
    let (mut session, flags, root) = setup();
    for n in 1..=12 {
        submit(&mut session, n);
    }
    session.correlate().unwrap();
    start(&mut session, Source::External).unwrap();
    start(&mut session, Source::Ocr).unwrap();
    assert_eq!(session.view.prediction_checks.len(), 2);
    assert_eq!(
        session.change_scope(0).unwrap().prediction,
        session.view.prediction_checks[1].id
    );
    capture(&mut session, &flags, CaptureMode::Full);
    let count = session.view.prediction_checks[0].candidates.len();
    assert_eq!(
        session.view.latest.as_ref().unwrap().prediction_reads.len(),
        count
    );
    assert!(
        session.view.prediction_checks[0]
            .candidates
            .iter()
            .all(|c| c.samples[0].observed.is_none())
    );
    submit(&mut session, 13);
    for check in &session.view.prediction_checks {
        assert!(
            check
                .candidates
                .iter()
                .all(|c| c.samples[0].supports_comparison() && c.samples[0].observed == Some(104.))
        );
    }
    let status = serde_json::to_value(&session.view).unwrap();
    let loaded: LiveView = serde_json::from_value(status).unwrap();
    assert_eq!(loaded.prediction_checks.len(), 2);
    assert_eq!(loaded.prediction_checks[0].source, Source::External);
    let old_input = session.view.visual_tracks[0].observations[12]
        .as_ref()
        .unwrap()
        .input
        .clone();
    let store = reversebolt_core::changes::Store::open(root.join("changes")).unwrap();
    session
        .recheck_findings(&session.view.investigation_id.clone(), 1, &store)
        .unwrap();
    assert_eq!(session.view.prediction_checks.len(), 2);
    assert!(session.view.visual_tracks[0].observations.is_empty());
    assert!(
        session
            .submit_observation(old_input.clone(), "agent")
            .is_err()
    );
    for n in 1..=4 {
        flags.frame.set(12 + n);
        session.capture().unwrap();
        let frame = session.view.frames.last().unwrap();
        let mut input = old_input.clone();
        input.frame_id = frame.id.clone();
        input.image_sha256 = frame.sha256.clone();
        input.value = Some(VisualValue::Number(value(12 + n) as f64));
        session.submit_observation(input, "agent").unwrap();
    }
    assert_eq!(
        session.view.latest.as_ref().unwrap().prediction_reads.len(),
        count
    );
    for check in &session.view.prediction_checks {
        assert!(check.candidates.iter().all(|c| c.samples.len() == 4));
        assert!(
            check
                .candidates
                .iter()
                .any(|c| c.assessment(check) == Assessment::ConsistentInObservedScope)
        );
        assert!(
            check
                .candidates
                .iter()
                .any(|c| c.assessment(check) == Assessment::Contradicted)
        );
    }
    let assessments = |check: &reversebolt_core::prediction::PredictionCheck| {
        check
            .candidates
            .iter()
            .map(|c| c.assessment(check))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        assessments(&session.view.prediction_checks[0]),
        assessments(&session.view.prediction_checks[1])
    );
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn automatic_checks_need_a_future_budget_and_preserve_explicit_checks() {
    let (mut session, flags, root) = fresh();
    for n in 1..=12 {
        flags.frame.set(n);
        session.capture_for_discovery(15 - n).unwrap();
    }
    assert!(session.view.prediction_checks.is_empty());
    assert!(session.view.tracks.iter().any(|t| !t.matches.is_empty()));
    start(&mut session, Source::Ocr).unwrap();
    let id = session.view.prediction_checks.first().unwrap().id.clone();
    flags.frame.set(13);
    session.capture_for_discovery(7).unwrap();
    let check = session.view.prediction_checks.first().unwrap();
    assert_eq!(check.id, id);
    assert!(check.automatic_selection.is_none());
    // Older persisted checks remain readable and never invent automatic provenance.
    let mut old = serde_json::to_value(check).unwrap();
    old.as_object_mut().unwrap().remove("automatic_selection");
    let old: reversebolt_core::prediction::PredictionCheck = serde_json::from_value(old).unwrap();
    assert!(old.automatic_selection.is_none());
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::remove_dir_all(root).unwrap();
}
fn submit(session: &mut LiveSession, number: usize) {
    let frame = session
        .view
        .frames
        .iter()
        .find(|f| f.number == number)
        .unwrap();
    let input = ObservationInput {
        investigation_id: session.view.investigation_id.clone(),
        frame_id: frame.id.clone(),
        image_sha256: frame.sha256.clone(),
        observer: "fixture adapter".into(),
        key: "scalar".into(),
        label: "anonymous".into(),
        unit: "".into(),
        value: Some(VisualValue::Number(value(number) as f64)),
        quality: Quality::Provisional,
        bounds: None,
        explanation: "Fixture-only observations, not real image interpretation.".into(),
    };
    session.submit_observation(input, "agent").unwrap();
}
fn start(session: &mut LiveSession, source: Source) -> Result<()> {
    session.start_prediction_check(&session.view.investigation_id.clone(), 1, 12, source, 0)
}

#[test]
fn visual_samples_keep_image_bound_external_matching_and_fresh_checks() {
    let (mut session, flags, root) = setup_mode(CaptureMode::Visual);
    assert!(session.view.tracks.is_empty());
    assert_eq!(session.view.frames.len(), 12);
    for n in 1..=12 {
        submit(&mut session, n);
    }
    session.correlate().unwrap();
    assert!(!session.view.visual_tracks[0].matches.is_empty());
    start(&mut session, Source::External).unwrap();
    for n in 13..=16 {
        capture(&mut session, &flags, CaptureMode::Visual);
        submit(&mut session, n);
    }
    let check = session.view.prediction_checks.first().unwrap();
    assert!(
        check
            .candidates
            .iter()
            .any(|c| c.assessment(check) == Assessment::ConsistentInObservedScope)
    );
    assert!(
        check
            .candidates
            .iter()
            .all(|c| c.samples.iter().all(|s| s.bytes.is_some()))
    );
    assert!(session.view.tracks.is_empty());
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn automatic_report_compares_and_keeps_frozen_predictions_unchanged() {
    let (mut session, flags, _) = setup();
    flags.frame.set(flags.frame.get() + 1);
    session.capture_for_report().unwrap();
    assert!(session.view.tracks.iter().any(|t| !t.matches.is_empty()));
    session
        .start_prediction_check(
            &session.view.investigation_id.clone(),
            1,
            13,
            Source::Ocr,
            0,
        )
        .unwrap();
    let frozen = serde_json::to_value(
        &session.view.prediction_checks.first().unwrap().candidates[0].mapping,
    )
    .unwrap();
    flags.frame.set(flags.frame.get() + 1);
    session.capture_for_report().unwrap();
    let check = session.view.prediction_checks.first().unwrap();
    assert_eq!(check.candidates[0].samples.len(), 1);
    assert_eq!(
        serde_json::to_value(&check.candidates[0].mapping).unwrap(),
        frozen
    );
}

#[test]
fn prospective_checks_keep_predictions_and_do_not_distinguish_identical_copies() {
    for source in [Source::Ocr, Source::External] {
        let (mut session, flags, root) = setup();
        if source == Source::External {
            for n in 1..=12 {
                submit(&mut session, n);
            }
        }
        session.correlate().unwrap();
        assert!(
            session
                .start_prediction_check("stale", 1, 12, source, 0)
                .is_err()
        );
        // Failed persistence must not publish a prospective start.
        std::fs::create_dir(root.join("prediction-checks.json")).unwrap();
        assert!(start(&mut session, source).is_err());
        assert!(session.view.prediction_checks.is_empty());
        std::fs::remove_dir(root.join("prediction-checks.json")).unwrap();
        start(&mut session, source).unwrap();
        let frozen = session.view.prediction_checks.first().unwrap().clone();
        assert!(start(&mut session, source).is_err());
        assert_eq!(
            session.view.prediction_checks.first().unwrap().id,
            frozen.id
        );
        for _ in 0..4 {
            capture(&mut session, &flags, CaptureMode::Full);
        }
        if source == Source::External {
            let check = session.view.prediction_checks.first().unwrap();
            assert!(
                check
                    .candidates
                    .iter()
                    .all(|c| c.assessment(check) == Assessment::NeedsContrastingEvidence)
            );
            for n in (13..=16).rev() {
                submit(&mut session, n);
            }
        }
        let check = session.view.prediction_checks.first().unwrap();
        assert_eq!(check.after_frame, 12);
        for (before, after) in frozen.candidates.iter().zip(&check.candidates) {
            assert_eq!(
                serde_json::to_value(&before.mapping).unwrap(),
                serde_json::to_value(&after.mapping).unwrap()
            );
            assert_eq!(after.samples.len(), 4);
            assert!(after.samples.iter().all(|s| s.frame > 12
                && s.image.is_some()
                && s.bytes.is_some()
                && s.read.is_some()));
            let expected = if after.address == "0x10040" {
                Assessment::Contradicted
            } else {
                Assessment::ConsistentInObservedScope
            };
            assert_eq!(after.assessment(check), expected);
        }
        assert_eq!(
            check
                .candidates
                .iter()
                .filter(|c| c.assessment(check) == Assessment::ConsistentInObservedScope)
                .count(),
            2
        );
        assert!(
            session.correlate().is_err(),
            "Must not refit the prospective interval"
        );
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("prediction-checks.json")).unwrap())
                .unwrap();
        assert_eq!(saved["checks"][0]["after_frame"], 12);
        assert_eq!(
            saved["checks"][0]["candidates"][0]["samples"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
    }
}

#[test]
fn missing_memory_geometry_mapping_exit_and_storage_failures_never_promote_checks() {
    for fault in ["missing", "resize", "remap", "exit", "storage"] {
        let (mut session, flags, root) = setup();
        session.correlate().unwrap();
        start(&mut session, Source::Ocr).unwrap();
        match fault {
            "missing" => flags.missing.set(true),
            "resize" => flags.resize.set(true),
            "remap" => flags.remap.set(true),
            "exit" => flags.dead.set(true),
            "storage" => {
                std::fs::remove_file(root.join("prediction-checks.json")).unwrap();
                std::fs::create_dir(root.join("prediction-checks.json")).unwrap();
            }
            _ => unreachable!(),
        }
        if fault == "exit" {
            assert!(session.capture().is_err());
        } else {
            for _ in 0..4 {
                capture(&mut session, &flags, CaptureMode::Full);
            }
        }
        let check = session.view.prediction_checks.first().unwrap();
        for candidate in &check.candidates {
            assert_eq!(
                candidate.assessment(check),
                if fault == "missing" {
                    Assessment::NeedsContrastingEvidence
                } else {
                    Assessment::Invalidated
                }
            );
        }
    }
}

#[test]
fn changing_or_missing_brackets_abstain_then_recover_on_later_stable_captures() {
    use reversebolt_core::prediction::Outcome;
    for missing in [false, true] {
        let (mut session, flags, root) = setup();
        assert!(
            session
                .view
                .latest
                .as_ref()
                .unwrap()
                .prediction_reads
                .is_empty()
        );
        session.correlate().unwrap();
        start(&mut session, Source::Ocr).unwrap();
        let before = session.view.coverage.attempted_bytes;
        flags.change_during_image.set(!missing);
        flags.missing_prefix.set(missing);
        let mut prefix_bytes = 0;
        for _ in 0..4 {
            capture(&mut session, &flags, CaptureMode::Full);
            let reads = &session.view.latest.as_ref().unwrap().prediction_reads;
            assert!(!reads.is_empty());
            assert!(reads.iter().all(|r| r.requested <= 8));
            prefix_bytes += reads.iter().map(|r| r.requested as u64).sum::<u64>();
        }
        assert_eq!(
            session.view.coverage.attempted_bytes - before,
            4 * session.view.planned_bytes as u64 + prefix_bytes
        );
        let check = session.view.prediction_checks.first().unwrap();
        for c in check.candidates.iter().filter(|c| c.address != "0x10040") {
            assert!(
                c.samples
                    .iter()
                    .all(|s| s.outcome == Outcome::TimingUncertain)
            );
            assert_eq!(c.assessment(check), Assessment::NeedsContrastingEvidence);
        }
        flags.change_during_image.set(false);
        flags.missing_prefix.set(false);
        for _ in 0..4 {
            capture(&mut session, &flags, CaptureMode::Full);
        }
        let check = session.view.prediction_checks.first().unwrap();
        for c in check.candidates.iter().filter(|c| c.address != "0x10040") {
            assert_eq!(c.assessment(check), Assessment::ConsistentInObservedScope);
            assert!(c.samples[4..].iter().all(|s| s.supports_comparison()));
        }
        assert!(root.starts_with(std::env::temp_dir()));
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn bracket_reads_are_reserved_before_any_capture_side_effect() {
    let (mut session, _, root) = setup();
    session.correlate().unwrap();
    start(&mut session, Source::Ocr).unwrap();
    session.view.coverage.attempted_bytes =
        reversebolt_core::coverage::MAX_READ_BYTES - session.view.planned_bytes as u64;
    let before = session.view.coverage.attempted_bytes;
    assert!(session.capture().unwrap_err().contains("read budget"));
    assert_eq!(session.view.sample_count, 12);
    assert_eq!(session.view.coverage.attempted_bytes, before);
    assert!(!root.join("frame-013.png").exists());
    std::fs::remove_dir_all(root).unwrap();
}
