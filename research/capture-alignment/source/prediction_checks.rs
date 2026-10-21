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
        Ok(Screen {
            path: path.to_string_lossy().into(),
            width: 100,
            height: if self.flags.resize.get() { 110 } else { 100 },
            text: String::new(),
            warning: None,
            method: "contract fixture".into(),
            words: vec![Word {
                text: observed.to_string(),
                bounds: [10., 10., 40., 20.],
                line: 0,
                source: TextSource::WindowsOcr,
            }],
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
    let flags = Rc::new(Flags::default());
    let root =
        std::env::temp_dir().join(format!("reversebolt-prediction-{}", uuid::Uuid::new_v4()));
    let target = Target {
        info: ProcessInfo::default(),
        flags: flags.clone(),
    };
    let mut session = LiveSession::attach(Box::new(target), 1, root.clone()).unwrap();
    for _ in 0..12 {
        capture(&mut session, &flags, mode);
    }
    (session, flags, root)
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
    let check = session.view.prediction_check.as_ref().unwrap();
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
        &session.view.prediction_check.as_ref().unwrap().candidates[0].mapping,
    )
    .unwrap();
    flags.frame.set(flags.frame.get() + 1);
    session.capture_for_report().unwrap();
    let check = session.view.prediction_check.as_ref().unwrap();
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
        std::fs::create_dir(root.join("prediction-check.json")).unwrap();
        assert!(start(&mut session, source).is_err());
        assert!(session.view.prediction_check.is_none());
        std::fs::remove_dir(root.join("prediction-check.json")).unwrap();
        start(&mut session, source).unwrap();
        let frozen = session.view.prediction_check.clone().unwrap();
        assert!(start(&mut session, source).is_err());
        assert_eq!(
            session.view.prediction_check.as_ref().unwrap().id,
            frozen.id
        );
        for _ in 0..4 {
            capture(&mut session, &flags, CaptureMode::Full);
        }
        if source == Source::External {
            let check = session.view.prediction_check.as_ref().unwrap();
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
        let check = session.view.prediction_check.as_ref().unwrap();
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
            serde_json::from_slice(&std::fs::read(root.join("prediction-check.json")).unwrap())
                .unwrap();
        assert_eq!(saved["check"]["after_frame"], 12);
        assert_eq!(
            saved["check"]["candidates"][0]["samples"]
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
                std::fs::remove_file(root.join("prediction-check.json")).unwrap();
                std::fs::create_dir(root.join("prediction-check.json")).unwrap();
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
        let check = session.view.prediction_check.as_ref().unwrap();
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
        let check = session.view.prediction_check.as_ref().unwrap();
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
        let check = session.view.prediction_check.as_ref().unwrap();
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
