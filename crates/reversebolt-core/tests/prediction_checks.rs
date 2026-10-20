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
        let frame = self.flags.frame.get() + 1;
        self.flags.frame.set(frame);
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
                text: value(frame).to_string(),
                bounds: [10., 10., 40., 20.],
                line: 0,
                source: TextSource::WindowsOcr,
            }],
        })
    }
    fn read(&self, _: u64, length: usize) -> Result<Vec<u8>> {
        if self.flags.missing.get() {
            return Err("injected incomplete read".into());
        }
        let frame = self.flags.frame.get();
        let mut bytes = vec![0; length];
        // Two observationally indistinguishable copies, plus a time confound.
        for offset in [32, 48] {
            bytes[offset..offset + 4].copy_from_slice(&value(frame).to_le_bytes());
        }
        bytes[64..68].copy_from_slice(&(13 + frame as i32 * 7).to_le_bytes());
        Ok(bytes)
    }
}
fn setup() -> (LiveSession, Rc<Flags>, PathBuf) {
    let flags = Rc::new(Flags::default());
    let root =
        std::env::temp_dir().join(format!("reversebolt-prediction-{}", uuid::Uuid::new_v4()));
    let target = Target {
        info: ProcessInfo::default(),
        flags: flags.clone(),
    };
    let mut session = LiveSession::attach(Box::new(target), 1, root.clone()).unwrap();
    for _ in 0..12 {
        session.capture().unwrap();
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
fn automatic_report_compares_and_keeps_frozen_predictions_unchanged() {
    let (mut session, _, _) = setup();
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
        let (mut session, _, root) = setup();
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
            session.capture().unwrap();
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
                session.capture().unwrap();
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
