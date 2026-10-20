//! Image-grounded observations from humans or external agents.
//! Claimed meanings remain provisional and independent from OCR and causality.
use crate::{Result, live::LocatedCandidate};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_TRACKS: usize = 32;

/// Bounds of the capture operation in an investigation-local monotonic clock.
/// These are not game presentation timestamps or an atomic memory snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureInterval {
    pub clock_id: String,
    pub begin_us: u64,
    pub end_us: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameRef {
    pub id: String,
    pub number: usize,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub epoch: usize,
    pub started_unix_ms: u128,
    #[serde(default)]
    pub capture_interval: Option<CaptureInterval>,
}

pub fn image_bytes(path: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_IMAGE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_IMAGE_BYTES || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("The saved capture is not a PNG within the 10 MiB image budget.".into());
    }
    Ok(bytes)
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum VisualValue {
    Number(f64),
    Boolean(bool),
    Text(String),
    Vector(Vec<f64>),
}

impl VisualValue {
    pub fn display(&self) -> String {
        match self {
            Self::Number(n) => n.to_string(),
            Self::Boolean(b) => b.to_string(),
            Self::Text(t) => t.clone(),
            Self::Vector(v) => format!("{v:?}"),
        }
    }
    pub(crate) fn shape(&self) -> (&'static str, usize) {
        match self {
            Self::Number(_) => ("number", 1),
            Self::Boolean(_) => ("boolean", 1),
            Self::Text(_) => ("text", 1),
            Self::Vector(v) => ("vector", v.len()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Provisional,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationInput {
    pub investigation_id: String,
    pub frame_id: String,
    pub image_sha256: String,
    /// Declared observer/model name, not an authenticated identity.
    pub observer: String,
    pub key: String,
    pub label: String,
    pub unit: String,
    pub value: Option<VisualValue>,
    pub quality: Quality,
    pub bounds: Option<[f32; 4]>,
    pub explanation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub input: ObservationInput,
    /// Engine-assigned transport provenance, not chosen by the observer.
    pub submitted_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisualTrack {
    pub id: usize,
    pub observer: String,
    pub submitted_by: String,
    pub key: String,
    pub label: String,
    pub unit: String,
    pub epoch: usize,
    pub active: bool,
    pub observations: Vec<Option<Observation>>,
    pub matches: Vec<LocatedCandidate>,
}

impl VisualTrack {
    pub fn numeric_samples(&self) -> Vec<Option<f64>> {
        self.observations
            .iter()
            .map(|record| {
                let input = &record.as_ref()?.input;
                match (&input.quality, &input.value) {
                    (Quality::Provisional, Some(VisualValue::Number(n))) => Some(*n),
                    _ => None,
                }
            })
            .collect()
    }
}

fn bounded(text: &str, min: usize, max: usize) -> bool {
    text.trim().len() >= min && text.len() <= max && !text.chars().any(char::is_control)
}

/// Validate the entire input before changing any track. A replay is a no-op;
/// a different interpretation of an already-filled slot is never a silent edit.
pub fn record(
    tracks: &mut Vec<VisualTrack>,
    input: ObservationInput,
    frame: &FrameRef,
    source: &str,
    sample_count: usize,
) -> Result<bool> {
    validate(&input, frame, sample_count)?;
    record_validated(tracks, input, frame, source, sample_count)
}

pub(crate) fn validate(
    input: &ObservationInput,
    frame: &FrameRef,
    sample_count: usize,
) -> Result<()> {
    if input.frame_id != frame.id || input.image_sha256 != frame.sha256 {
        return Err("The observation references a different captured image.".into());
    }
    if !bounded(&input.observer, 1, 64)
        || !bounded(&input.key, 1, 64)
        || !bounded(&input.label, 1, 96)
        || !bounded(&input.unit, 0, 32)
        || !bounded(&input.explanation, 1, 512)
    {
        return Err("Supply a short observer name, key, label, unit and explanation.".into());
    }
    if frame.number == 0 || frame.number > sample_count {
        return Err("The capture is outside this investigation.".into());
    }
    if input.quality == Quality::Provisional && input.value.is_none() {
        return Err(
            "A provisional observation needs a value; use uncertain when unreadable.".into(),
        );
    }
    let valid_value = match &input.value {
        Some(VisualValue::Number(n)) => n.is_finite(),
        Some(VisualValue::Vector(v)) => {
            (2..=4).contains(&v.len()) && v.iter().all(|n| n.is_finite())
        }
        Some(VisualValue::Text(t)) => bounded(t, 1, 128),
        _ => true,
    };
    if !valid_value {
        return Err("Use finite numbers, a 2–4 component vector, or short text.".into());
    }
    if let Some([x, y, w, h]) = input.bounds
        && (![x, y, w, h].iter().all(|v| v.is_finite())
            || x < 0.0
            || y < 0.0
            || w <= 0.0
            || h <= 0.0
            || x + w > frame.width as f32
            || y + h > frame.height as f32)
    {
        return Err("Observation bounds must lie inside the referenced image.".into());
    }
    Ok(())
}

fn record_validated(
    tracks: &mut Vec<VisualTrack>,
    input: ObservationInput,
    frame: &FrameRef,
    source: &str,
    sample_count: usize,
) -> Result<bool> {
    if let Some(track) = tracks.iter_mut().find(|t| {
        t.epoch == frame.epoch
            && t.observer == input.observer
            && t.submitted_by == source
            && t.key == input.key
    }) {
        if track.label != input.label || track.unit != input.unit {
            return Err("This series already has a different label or unit. Use a new key.".into());
        }
        if let Some(value) = &input.value
            && track
                .observations
                .iter()
                .flatten()
                .filter_map(|o| o.input.value.as_ref())
                .any(|v| v.shape() != value.shape())
        {
            return Err(
                "A series cannot change value type or vector dimensions. Use a new key.".into(),
            );
        }
        if let Some(old) = &track.observations[frame.number - 1] {
            return if old.input == input {
                Ok(false)
            } else {
                Err("This frame already has a different observation in that series. Use a new key to preserve an alternative interpretation.".into())
            };
        }
        track.observations[frame.number - 1] = Some(Observation {
            input,
            submitted_by: source.into(),
        });
        track.matches.clear();
    } else {
        if tracks.len() >= MAX_TRACKS {
            return Err(
                "The investigation has reached its 32 external observation series limit.".into(),
            );
        }
        let mut observations = vec![None; sample_count];
        observations[frame.number - 1] = Some(Observation {
            input: input.clone(),
            submitted_by: source.into(),
        });
        tracks.push(VisualTrack {
            id: tracks.len(),
            observer: input.observer,
            submitted_by: source.into(),
            key: input.key,
            label: input.label,
            unit: input.unit,
            epoch: frame.epoch,
            active: true,
            observations,
            matches: vec![],
        });
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame() -> FrameRef {
        FrameRef {
            id: "frame".into(),
            number: 1,
            sha256: "hash".into(),
            width: 100,
            height: 100,
            epoch: 0,
            started_unix_ms: 1,
            capture_interval: None,
        }
    }
    fn input() -> ObservationInput {
        ObservationInput {
            investigation_id: "investigation".into(),
            frame_id: "frame".into(),
            image_sha256: "hash".into(),
            observer: "test observer".into(),
            key: "counter".into(),
            label: "Counter".into(),
            unit: "count".into(),
            value: Some(VisualValue::Number(42.0)),
            quality: Quality::Provisional,
            bounds: Some([10.0, 10.0, 30.0, 30.0]),
            explanation: "Fixture interpretation; no semantic accuracy claim.".into(),
        }
    }
    #[test]
    fn replay_conflict_source_separation_and_uncertainty() {
        let mut tracks = vec![];
        assert!(record(&mut tracks, input(), &frame(), "agent", 2).unwrap());
        assert!(!record(&mut tracks, input(), &frame(), "agent", 2).unwrap());
        let mut alternate = input();
        alternate.value = Some(VisualValue::Number(99.0));
        assert!(record(&mut tracks, alternate, &frame(), "agent", 2).is_err());
        assert_eq!(tracks[0].numeric_samples(), vec![Some(42.0), None]);
        assert!(record(&mut tracks, input(), &frame(), "desktop", 2).unwrap());
        assert_eq!(tracks.len(), 2);
        let mut uncertain = input();
        uncertain.key = "unclear".into();
        uncertain.quality = Quality::Uncertain;
        assert!(record(&mut tracks, uncertain, &frame(), "agent", 2).unwrap());
        assert_eq!(tracks[2].numeric_samples(), vec![None, None]);
        let before = serde_json::to_string(&tracks).unwrap();
        let mut bad = input();
        bad.bounds = Some([99.0, 0.0, 2.0, 1.0]);
        assert!(record(&mut tracks, bad, &frame(), "agent", 2).is_err());
        assert_eq!(before, serde_json::to_string(&tracks).unwrap());
    }
    #[test]
    fn types_limits_and_nonfinite_values_are_checked_before_mutation() {
        let mut tracks = vec![];
        for value in [
            VisualValue::Number(f64::NAN),
            VisualValue::Vector(vec![1.0]),
            VisualValue::Vector(vec![1.0, f64::INFINITY]),
            VisualValue::Text("x".repeat(129)),
        ] {
            let mut bad = input();
            bad.value = Some(value);
            assert!(record(&mut tracks, bad, &frame(), "agent", 2).is_err());
        }
        assert!(tracks.is_empty());
        record(&mut tracks, input(), &frame(), "agent", 2).unwrap();
        let mut next = frame();
        next.number = 2;
        let mut bad = input();
        bad.value = Some(VisualValue::Boolean(true));
        assert!(record(&mut tracks, bad, &next, "agent", 2).is_err());
        let mut bad = input();
        bad.unit = "pixels".into();
        assert!(record(&mut tracks, bad, &next, "agent", 2).is_err());
        assert!(tracks[0].observations[1].is_none());
        for n in 1..MAX_TRACKS {
            let mut more = input();
            more.key = n.to_string();
            record(&mut tracks, more, &frame(), "agent", 2).unwrap();
        }
        let mut overflow = input();
        overflow.key = "overflow".into();
        assert!(record(&mut tracks, overflow, &frame(), "agent", 2).is_err());
    }
}
