//! Shared-engine provenance/lifecycle checks, not motion-recognition evidence.
use reversebolt_core::{
    Result,
    live::*,
    temporal,
    visual::{Quality, VisualValue},
};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Default)]
struct Flags {
    missing: Cell<bool>,
    resized: Cell<bool>,
    dead: Cell<bool>,
    number: Cell<usize>,
}
struct Target {
    info: ProcessInfo,
    flags: Rc<Flags>,
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
            length: 2 * 1024 * 1024,
            ..Region::default()
        }])
    }
    fn read(&self, _: u64, length: usize) -> Result<Vec<u8>> {
        Ok(vec![0; length])
    }
    fn screen(&self, path: &Path) -> Result<Screen> {
        if self.flags.missing.get() {
            return Err("injected missing image".into());
        }
        self.flags.number.set(self.flags.number.get() + 1);
        // The core hashes opaque PNG bytes; decoding is exercised by native smoke.
        let bytes = [
            b"\x89PNG\r\n\x1a\ncontract fixture".as_slice(),
            &self.flags.number.get().to_le_bytes(),
        ]
        .concat();
        std::fs::write(path, bytes).map_err(|e| e.to_string())?;
        Ok(Screen {
            path: path.to_string_lossy().into(),
            width: 100,
            height: if self.flags.resized.get() { 110 } else { 100 },
            text: String::new(),
            words: vec![],
            warning: None,
            method: "contract fixture".into(),
        })
    }
}
fn setup() -> (LiveSession, Rc<Flags>, PathBuf) {
    let flags = Rc::new(Flags::default());
    let directory =
        std::env::temp_dir().join(format!("reversebolt-temporal-{}", uuid::Uuid::new_v4()));
    let target = Target {
        info: ProcessInfo::default(),
        flags: flags.clone(),
    };
    (
        LiveSession::attach(Box::new(target), 1, directory.clone()).unwrap(),
        flags,
        directory,
    )
}
fn input(session: &LiveSession) -> temporal::Input {
    temporal::Input {
        investigation_id: session.view.investigation_id.clone(),
        batch: session.view.coverage.batch,
        observer: "fixture observer".into(),
        key: "appearance".into(),
        label: "Shape change".into(),
        unit: "".into(),
        basis: temporal::Basis::Appearance,
        frames: session
            .view
            .frames
            .iter()
            .map(|f| temporal::Anchor {
                frame_id: f.id.clone(),
                image_sha256: f.sha256.clone(),
                bounds: None,
            })
            .collect(),
        value: Some(VisualValue::Boolean(true)),
        quality: Quality::Provisional,
        explanation: "Contract-only declaration; no recognition claim.".into(),
    }
}
#[test]
fn temporal_evidence_survives_archive_but_never_crosses_batches() {
    let (mut session, flags, directory) = setup();
    session.capture().unwrap();
    flags.missing.set(true);
    session.capture().unwrap();
    flags.missing.set(false);
    session.capture().unwrap();
    let declaration = input(&session);
    assert!(
        session
            .submit_temporal_observation(declaration.clone(), "agent")
            .unwrap()
    );
    assert!(
        !session
            .submit_temporal_observation(declaration.clone(), "agent")
            .unwrap()
    );
    let evidence = &session.view.temporal_tracks[0].observations[0].evidence;
    assert_eq!(evidence.omitted_moments, 1);
    assert_eq!(evidence.frames.len(), 2);
    assert_eq!(evidence.clock_id, session.view.investigation_id);
    assert!(evidence.elapsed_us[0] <= evidence.elapsed_us[1]);
    let image_time = evidence
        .frames
        .last()
        .unwrap()
        .capture_interval
        .as_ref()
        .unwrap();
    for read in &session.view.latest.as_ref().unwrap().memory_reads {
        let read_time = read.capture_interval.as_ref().unwrap();
        assert_eq!(image_time.clock_id, read_time.clock_id);
        assert!(image_time.end_us <= read_time.begin_us);
        assert!(read_time.begin_us <= read_time.end_us);
    }
    assert!(directory.join("temporal-evidence.json").is_file());
    let original = std::fs::read(directory.join("temporal-evidence.json")).unwrap();
    session
        .advance_memory(&declaration.investigation_id, 1)
        .unwrap();
    assert!(session.view.temporal_tracks.is_empty());
    assert!(
        session
            .submit_temporal_observation(declaration.clone(), "agent")
            .is_err()
    );
    assert_eq!(
        std::fs::read(directory.join("temporal-evidence.json")).unwrap(),
        original
    );
    let archive: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("memory-batch.json")).unwrap())
            .unwrap();
    assert_eq!(
        archive["state"]["temporal_tracks"][0]["observations"][0]["input"]["label"],
        "Shape change"
    );
    session.capture().unwrap();
    session.capture().unwrap();
    let mut stale = input(&session);
    stale.investigation_id = "other".into();
    assert!(session.submit_temporal_observation(stale, "agent").is_err());
    let current = input(&session);
    session
        .submit_temporal_observation(current, "agent")
        .unwrap();
    flags.dead.set(true);
    assert!(session.refresh_connection());
    assert!(!session.view.temporal_tracks[0].active);
    assert!(
        session
            .submit_temporal_observation(input(&session), "agent")
            .is_err()
    );
    std::fs::remove_dir_all(directory).unwrap();
}
#[test]
fn changed_images_failed_persistence_and_geometry_do_not_publish_evidence() {
    let (mut session, flags, directory) = setup();
    session.capture().unwrap();
    session.capture().unwrap();
    let declaration = input(&session);
    let image_path = directory.join("frame-001.png");
    let original = std::fs::read(&image_path).unwrap();
    std::fs::write(&image_path, b"\x89PNG\r\n\x1a\nchanged").unwrap();
    assert!(
        session
            .submit_temporal_observation(declaration.clone(), "agent")
            .is_err()
    );
    assert!(session.view.temporal_tracks.is_empty());
    std::fs::write(image_path, original).unwrap();
    let evidence_path = directory.join("temporal-evidence.json");
    std::fs::create_dir(&evidence_path).unwrap();
    assert!(
        session
            .submit_temporal_observation(declaration.clone(), "agent")
            .is_err()
    );
    assert!(session.view.temporal_tracks.is_empty());
    std::fs::remove_dir(evidence_path).unwrap();
    session
        .submit_temporal_observation(declaration.clone(), "agent")
        .unwrap();
    flags.resized.set(true);
    session.capture().unwrap();
    assert!(!session.view.temporal_tracks[0].active);
    assert!(
        session
            .submit_temporal_observation(declaration, "agent")
            .is_err()
    );
    assert!(
        session
            .submit_temporal_observation(input(&session), "agent")
            .is_err()
    );
    assert_eq!(session.view.temporal_tracks.len(), 1);
    std::fs::remove_dir_all(directory).unwrap();
}
