//! Local model work runs inside an owned capture job. It cannot write game memory.
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
    path::Path,
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

pub fn run(
    session: &mut LiveSession,
    model: &str,
    cancel: &AtomicBool,
    directory: &Path,
) -> Result<Value> {
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
    let id = uuid::Uuid::new_v4().to_string();
    let mut pass = Pass::default();
    let result = tokio::runtime::Runtime::new()
        .map_err(|e| e.to_string())?
        .block_on(infer(model, &frames, cancel, &mut pass));
    pass.error = result.err();
    pass.cancelled |= cancel.load(Ordering::SeqCst);
    let inputs = observations(&pass, latest, &session.view.investigation_id, &id);
    let evidence = json!({"schema_version":1,"id":id,"investigation_id":session.view.investigation_id,
        "batch":session.view.coverage.batch,"process":session.view.process,"sequence":sequence,"pass":pass,
        "proposed_observations":inputs,"limitation":"Same-model image judgments, not independent validation, persistent identities or editable controls. Proposed observations are not a publication receipt."});
    let folder = directory.join("model-observations");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(folder.join(format!("{id}.json")))
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(&evidence).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    if let Some(error) = pass.error {
        return Err(format!(
            "Local observation stopped: {error} Evidence: {id}."
        ));
    }
    let cancelled = pass.cancelled || cancel.load(Ordering::SeqCst);
    let count = if cancelled {
        0
    } else {
        session.submit_observations(inputs, "local-model")?
    };
    Ok(json!({"evidence_id":id,"recorded":count,"cancelled":cancelled,"provisional":true}))
}

#[cfg(test)]
mod tests {
    use super::*;

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
