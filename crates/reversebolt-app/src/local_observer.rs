//! Owned local model workers use immutable image copies. Only the engine publishes findings.
use reversebolt_core::{
    Result,
    live::LiveSession,
    visual::{self, ObservationInput, Quality, VisualValue},
};
use reversebolt_observer::{Frame, Kind, Observer, Receipt, review::ReviewReceipt};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[derive(Serialize)]
struct Reviewed {
    candidate: usize,
    receipt: ReviewReceipt,
}
#[derive(Default, Serialize)]
struct Pass {
    proposal: Option<Receipt>,
    reviews: Vec<Reviewed>,
    cancelled: bool,
    error: Option<String>,
}

pub fn check_model(model: &str) -> Result<()> {
    tokio::runtime::Runtime::new()
        .map_err(|e| e.to_string())?
        .block_on(Observer::connect(model))
        .map(|_| ())
}

async fn infer(model: &str, frames: &[Frame], cancel: &AtomicBool, pass: &mut Pass) -> Result<()> {
    let started = Instant::now();
    let check = |pass: &mut Pass| -> Result<bool> {
        pass.cancelled = cancel.load(Ordering::SeqCst);
        if started.elapsed() > Duration::from_secs(90) {
            return Err("The local observation pass exhausted its inference budget.".into());
        }
        Ok(!pass.cancelled)
    };
    if !check(pass)? {
        return Ok(());
    }
    let observer = Observer::connect(model).await?;
    if !check(pass)? {
        return Ok(());
    }
    pass.proposal = Some(observer.observe(frames).await?);
    if !check(pass)? {
        return Ok(());
    }
    let proposal = pass.proposal.as_ref().unwrap();
    let entities = proposal
        .interpretation
        .as_ref()
        .ok_or_else(|| {
            format!(
                "Local model returned an invalid proposal: {}",
                proposal
                    .rejection
                    .as_deref()
                    .unwrap_or("missing interpretation")
            )
        })?
        .entities
        .clone();
    for (candidate, entity) in entities.iter().enumerate() {
        if !matches!(entity.kind, Kind::Actor | Kind::HeldComponent)
            || !entity
                .localizations
                .iter()
                .any(|l| l.frame == frames.len() - 1)
        {
            continue;
        }
        if !check(pass)? {
            return Ok(());
        }
        let receipt = observer.review(frames, &entity.localizations).await?;
        pass.reviews.push(Reviewed { candidate, receipt });
    }
    check(pass)?;
    Ok(())
}

fn observations(
    pass: &Pass,
    frame: &visual::FrameRef,
    investigation: &str,
    id: &str,
) -> Vec<ObservationInput> {
    if pass.cancelled || pass.error.is_some() {
        return vec![];
    }
    let Some(proposal) = pass.proposal.as_ref() else {
        return vec![];
    };
    let Some(interpretation) = proposal.interpretation.as_ref() else {
        return vec![];
    };
    let last = proposal.frames.len() - 1;
    pass.reviews
        .iter()
        .filter_map(|reviewed| {
            let review = &reviewed.receipt;
            let judgment = review.judgment.as_ref()?;
            if !matches!(judgment.kind, Kind::Actor | Kind::HeldComponent)
                || !review.refined.iter().any(|l| l.frame == last)
            {
                return None;
            }
            let entity = interpretation.entities.get(reviewed.candidate)?;
            let location = entity.localizations.iter().find(|l| l.frame == last)?;
            // Crop review filters categories; its less accurate refined boxes are not adopted.
            let [l, t, r, b] = location.bounds;
            let bounds = [
                (l * frame.width as f64 / 1000.) as f32,
                (t * frame.height as f64 / 1000.) as f32,
                ((r - l) * frame.width as f64 / 1000.) as f32,
                ((b - t) * frame.height as f64 / 1000.) as f32,
            ];
            Some(ObservationInput {
                investigation_id: investigation.into(),
                frame_id: frame.id.clone(),
                image_sha256: frame.sha256.clone(),
                observer: format!("local vision {}", &proposal.model_digest[..16]),
                // Different passes do not imply that visually similar entities share identity.
                key: format!("{id}-{}", reviewed.candidate),
                label: judgment.name.clone(),
                unit: String::new(),
                value: Some(VisualValue::Text(
                    if judgment.kind == Kind::Actor {
                        "Character"
                    } else {
                        "Held object"
                    }
                    .into(),
                )),
                quality: Quality::Provisional,
                bounds: Some(bounds),
                explanation: format!(
                    "{} Unverified appearance. Evidence: {id}.",
                    judgment.explanation
                ),
            })
        })
        .collect()
}

/// Immutable, verified image copies; the worker never borrows the live session.
pub struct Prepared {
    frames: Vec<Frame>,
    sequence: reversebolt_core::temporal::Sequence,
    investigation: String,
    batch: usize,
    process: reversebolt_core::live::ProcessInfo,
}
impl Prepared {
    pub fn from_session(session: &LiveSession) -> Result<Self> {
        let ids: Vec<_> = session
            .view
            .frames
            .iter()
            .rev()
            .take(2)
            .map(|f| f.id.clone())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let (sequence, images) = session.read_sequence(&session.view.investigation_id, &ids)?;
        let latest = sequence
            .frames
            .last()
            .ok_or("No images for the local observer.")?;
        if latest.number != session.view.sample_count {
            return Err("The latest capture has no usable image.".into());
        }
        let frames = images
            .into_iter()
            .zip(&sequence.frames)
            .map(|(image, reference)| {
                let timestamp = reference
                    .capture_interval
                    .as_ref()
                    .ok_or("Missing capture timing.")?
                    .begin_us
                    .checked_mul(1000)
                    .ok_or("Capture timestamp overflow.")?;
                let frame = Frame::from_png(image, timestamp)?;
                if frame.evidence.sha256 != reference.sha256
                    || frame.evidence.width != reference.width
                    || frame.evidence.height != reference.height
                {
                    return Err("Local observer image differs from its capture evidence.".into());
                }
                Ok(frame)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            frames,
            sequence,
            investigation: session.view.investigation_id.clone(),
            batch: session.view.coverage.batch,
            process: session.view.process.clone(),
        })
    }
}

pub struct Completed {
    pub id: String,
    prepared: Prepared,
    pass: Pass,
    inputs: Vec<ObservationInput>,
}
impl Completed {
    pub fn publish(self, session: &mut LiveSession, cancel: &AtomicBool) -> Result<Value> {
        if let Some(error) = self.pass.error {
            return Err(format!(
                "Local observation stopped: {error} Evidence: {}.",
                self.id
            ));
        }
        let cancelled = self.pass.cancelled || cancel.load(Ordering::SeqCst);
        let count = if cancelled {
            0
        } else {
            if session.view.investigation_id != self.prepared.investigation
                || session.view.coverage.batch != self.prepared.batch
                || session.view.process.pid != self.prepared.process.pid
                || session.view.process.created != self.prepared.process.created
                || session.view.process.window != self.prepared.process.window
            {
                return Err(
                    "Local observation belongs to a replaced investigation or memory batch.".into(),
                );
            }
            session.submit_observations(self.inputs, "local-model")?
        };
        Ok(
            json!({"evidence_id":self.id,"recorded":count,"cancelled":cancelled,"provisional":true,
            "observed_frame":self.prepared.sequence.frames.last().map(|f|&f.id)}),
        )
    }
}

pub struct Worker {
    pub status: reversebolt_core::engine::AnalysisStatus,
    pub cancel: std::sync::Arc<AtomicBool>,
    receiver: std::sync::mpsc::Receiver<Result<Completed>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Worker {
    pub fn start(
        prepared: Prepared,
        model: String,
        job: u64,
        cancel: std::sync::Arc<AtomicBool>,
        directory: std::path::PathBuf,
    ) -> Result<Self> {
        Self::start_with(
            prepared,
            model,
            job,
            cancel,
            directory,
            |model, frames, cancel, pass| {
                tokio::runtime::Runtime::new()
                    .map_err(|e| e.to_string())?
                    .block_on(infer(model, frames, cancel, pass))
            },
        )
    }
    fn start_with(
        prepared: Prepared,
        model: String,
        job: u64,
        cancel: std::sync::Arc<AtomicBool>,
        directory: std::path::PathBuf,
        execute: impl FnOnce(&str, &[Frame], &AtomicBool, &mut Pass) -> Result<()> + Send + 'static,
    ) -> Result<Self> {
        let id = uuid::Uuid::new_v4().to_string();
        let status = reversebolt_core::engine::AnalysisStatus {
            job,
            evidence_id: id.clone(),
            model: model.clone(),
            investigation_id: prepared.investigation.clone(),
            batch: prepared.batch,
            frame_ids: prepared
                .sequence
                .frames
                .iter()
                .map(|f| f.id.clone())
                .collect(),
            observed_unix_ms: prepared.sequence.frames.last().unwrap().started_unix_ms,
            started_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            stopping: false,
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        let signal = cancel.clone();
        let thread=std::thread::Builder::new().name("local-vision".into()).spawn(move || {
            let result=(|| {
                let mut pass=Pass::default();
                pass.error=execute(&model,&prepared.frames,&signal,&mut pass).err();
                pass.cancelled|=signal.load(Ordering::SeqCst);
                let inputs=observations(&pass,prepared.sequence.frames.last().unwrap(),&prepared.investigation,&id);
                let evidence=json!({"schema_version":2,"id":id,"job":job,"investigation_id":prepared.investigation,
                    "batch":prepared.batch,"process":prepared.process,"sequence":prepared.sequence,"pass":pass,
                    "proposed_observations":inputs,"limitation":"Same-model image judgments, not independent validation, persistent identities or editable controls. Proposed observations are not a publication receipt."});
                let folder=directory.join("model-observations");
                std::fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
                let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(folder.join(format!("{id}.json"))).map_err(|e|e.to_string())?;
                file.write_all(&serde_json::to_vec_pretty(&evidence).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
                file.sync_all().map_err(|e|e.to_string())?;
                Ok(Completed{id,prepared,pass,inputs})
            })();
            let _=sender.send(result);
        }).map_err(|e|e.to_string())?;
        Ok(Self {
            status,
            cancel,
            receiver,
            thread: Some(thread),
        })
    }
    pub fn poll(&mut self) -> Option<Result<Completed>> {
        let result = match self.receiver.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err("Local observation worker ended without an outcome.".into())
            }
        };
        if self.thread.take().is_some_and(|t| t.join().is_err()) {
            return Some(Err(
                "Local observation worker failed. No findings were published.".into(),
            ));
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reversebolt_core::live::{NativeTarget, ProcessInfo, Region, Screen};
    use std::sync::{Arc, Mutex, mpsc};

    struct Target {
        info: ProcessInfo,
        size: Arc<Mutex<(u32, u32)>>,
    }
    impl NativeTarget for Target {
        fn info(&self) -> &ProcessInfo {
            &self.info
        }
        fn regions(&self) -> Result<Vec<Region>> {
            Ok(vec![Region {
                base: 4096,
                length: 4096,
                ..Default::default()
            }])
        }
        fn read(&self, _: u64, length: usize) -> Result<Vec<u8>> {
            Ok(vec![0; length])
        }
        fn alive(&self) -> bool {
            true
        }
        fn screen(&self, path: &std::path::Path) -> Result<Screen> {
            let (width, height) = *self.size.lock().unwrap();
            image::RgbImage::from_pixel(width, height, image::Rgb([15, 50, 100]))
                .save(path)
                .map_err(|e| e.to_string())?;
            Ok(Screen {
                path: path.to_string_lossy().into(),
                width,
                height,
                text: String::new(),
                words: vec![],
                warning: None,
                method: "fixture".into(),
            })
        }
    }

    fn session() -> (LiveSession, std::path::PathBuf, Arc<Mutex<(u32, u32)>>) {
        let directory =
            std::env::temp_dir().join(format!("reversebolt-observer-{}", uuid::Uuid::new_v4()));
        let size = Arc::new(Mutex::new((128, 96)));
        let target = Target {
            info: ProcessInfo {
                pid: 1,
                created: "fixture-creation".into(),
                window: 2,
                ..Default::default()
            },
            size: size.clone(),
        };
        let mut session = LiveSession::attach(Box::new(target), 1, directory.clone()).unwrap();
        session.capture().unwrap();
        session.capture().unwrap();
        (session, directory, size)
    }
    fn finish(worker: &mut Worker) -> Completed {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = worker.poll() {
                return result.unwrap();
            }
            assert!(Instant::now() < deadline, "Owned worker did not settle");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn successful_pass(frames: &[Frame], pass: &mut Pass) {
        *pass = proposed().0;
        pass.proposal.as_mut().unwrap().frames =
            frames.iter().map(|f| f.evidence.clone()).collect();
    }

    #[test]
    fn capture_continues_while_review_waits_and_late_results_keep_original_frame() {
        let (mut session, directory, _) = session();
        let captured = session.view.frames[1].clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let mut worker = Worker::start_with(
            Prepared::from_session(&session).unwrap(),
            "fixture".into(),
            7,
            cancel.clone(),
            directory.clone(),
            move |_, frames, _, pass| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                successful_pass(frames, pass);
                Ok(())
            },
        )
        .unwrap();
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        session.capture().unwrap();
        session.capture().unwrap();
        assert!(worker.poll().is_none());
        assert_eq!(session.view.sample_count, 4);
        release_tx.send(()).unwrap();
        let result = finish(&mut worker).publish(&mut session, &cancel).unwrap();
        assert_eq!(result["recorded"], 1);
        let observations = &session.view.visual_tracks[0].observations;
        assert_eq!(observations.len(), 4);
        assert_eq!(
            observations[1].as_ref().unwrap().input.frame_id,
            captured.id
        );
        assert!(observations[2..].iter().all(Option::is_none));
        let raw: Value = serde_json::from_slice(
            &std::fs::read(
                directory
                    .join("model-observations")
                    .join(format!("{}.json", result["evidence_id"].as_str().unwrap())),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(raw["sequence"]["frames"][1]["sha256"], captured.sha256);
        assert_eq!(raw["job"], 7);
    }

    #[test]
    fn late_review_cannot_publish_after_stop_replacement_or_resize() {
        for failure in ["cancel", "scope", "geometry"] {
            let (mut session, directory, size) = session();
            let cancel = Arc::new(AtomicBool::new(false));
            let mut worker = Worker::start_with(
                Prepared::from_session(&session).unwrap(),
                "fixture".into(),
                1,
                cancel.clone(),
                directory,
                |_, frames, _, pass| {
                    successful_pass(frames, pass);
                    Ok(())
                },
            )
            .unwrap();
            let completed = finish(&mut worker);
            match failure {
                "cancel" => cancel.store(true, Ordering::SeqCst),
                "scope" => session.view.investigation_id = "replaced".into(),
                _ => {
                    *size.lock().unwrap() = (192, 96);
                    session.capture().unwrap();
                }
            }
            let result = completed.publish(&mut session, &cancel);
            if failure == "cancel" {
                assert_eq!(result.unwrap()["recorded"], 0);
            } else {
                assert!(result.is_err());
            }
            assert!(session.view.visual_tracks.is_empty());
        }
    }

    fn proposed() -> (Pass, visual::FrameRef) {
        let frame = visual::FrameRef {
            id: "frame-2".into(),
            number: 2,
            sha256: "image-sha".into(),
            width: 1000,
            height: 500,
            epoch: 0,
            started_unix_ms: 0,
            capture_interval: None,
        };
        let proposal:Receipt=serde_json::from_value(json!({
            "schema_version":1,"model":"fixture","model_digest":"a".repeat(64),"server_version":"test",
            "prompt_sha256":"test","prompt":"test","output_schema":{},"options":{},"thinking_disabled":false,
            "frames":[{"sha256":"previous","width":1000,"height":500,"timestamp_ns":1},{"sha256":"image-sha","width":1000,"height":500,"timestamp_ns":2}],
            "elapsed_ms":1,"response":{},"interpretation":{"summary":"fixture","entities":[{"name":"Original label","kind":"actor","description":"Original description","localizations":[{"frame":1,"bounds":[100,200,300,400]}]}]},
            "rejection":null,"limitation":"test"
        })).unwrap();
        let receipt:ReviewReceipt=serde_json::from_value(json!({
            "schema_version":1,"model":"fixture","model_digest":"a".repeat(64),"server_version":"test",
            "prompt_sha256":"test","prompt":"test","output_schema":{},"options":{},"thinking_disabled":false,
            "crops":[],"response":{},"elapsed_ms":1,"rejection":null,"limitation":"test",
            "judgment":{"kind":"actor","name":"Small figure","explanation":"A visible humanoid shape.","localizations":[{"frame":1,"bounds":[400,400,500,500]}]},
            "refined":[{"frame":1,"bounds":[400,400,500,500]}]
        })).unwrap();
        (
            Pass {
                proposal: Some(proposal),
                reviews: vec![Reviewed {
                    candidate: 0,
                    receipt,
                }],
                ..Pass::default()
            },
            frame,
        )
    }

    #[test]
    fn publication_keeps_original_geometry_and_no_edit_or_identity_claim() {
        let (mut pass, frame) = proposed();
        let first = observations(&pass, &frame, "scope", "pass-a");
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].bounds, Some([100., 100., 200., 100.]));
        assert_eq!(first[0].quality, Quality::Provisional);
        assert_eq!(first[0].value, Some(VisualValue::Text("Character".into())));
        assert_eq!(first[0].image_sha256, frame.sha256);
        assert_ne!(
            first[0].key,
            observations(&pass, &frame, "scope", "pass-b")[0].key
        );
        pass.reviews[0].receipt.judgment.as_mut().unwrap().kind = Kind::Other;
        assert!(observations(&pass, &frame, "scope", "pass-a").is_empty());
        pass.reviews[0].receipt.judgment.as_mut().unwrap().kind = Kind::Actor;
        pass.reviews[0].receipt.refined.clear();
        assert!(observations(&pass, &frame, "scope", "pass-a").is_empty());
    }

    #[test]
    fn stopped_or_failed_pass_never_publishes_partial_findings() {
        let (mut pass, frame) = proposed();
        pass.error = Some("transport failure after one review".into());
        assert!(observations(&pass, &frame, "scope", "pass").is_empty());
        pass.error = None;
        pass.cancelled = true;
        assert!(observations(&pass, &frame, "scope", "pass").is_empty());
        // Cancellation is checked before connecting even to an invalid model.
        let mut empty = Pass::default();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(infer(
                "invalid model",
                &[],
                &AtomicBool::new(true),
                &mut empty,
            ))
            .unwrap();
        assert!(empty.cancelled && empty.proposal.is_none());
    }
}
