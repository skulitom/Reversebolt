//! Multi-image interpretations with immutable frame and timing evidence.
//! These are declared observations, never object identities or causal controls.
use crate::{
    Result,
    visual::{self, FrameRef, ObservationInput, Quality, VisualValue},
};
use serde::{Deserialize, Serialize};

pub const MAX_FRAMES: usize = 8;
pub const MAX_SEQUENCE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_TRACKS: usize = 16;
pub const MAX_OBSERVATIONS: usize = 20;
pub const LIMITATION: &str = "Interpretations of sampled images only. Capture-operation bounds are not game presentation times. Unobserved intervals can hide movement, animation and scene changes. No continuous identity, world speed, memory correspondence or causal effect is verified.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    Appearance,
    ScreenMotion,
    ObservedChange,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub frame_id: String,
    pub image_sha256: String,
    /// Independent image-space support, not a verified object correspondence.
    pub bounds: Option<[f32; 4]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub investigation_id: String,
    pub batch: usize,
    pub observer: String,
    pub key: String,
    pub label: String,
    pub unit: String,
    pub basis: Basis,
    pub frames: Vec<Anchor>,
    pub value: Option<VisualValue>,
    pub quality: Quality,
    pub explanation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sequence {
    pub frames: Vec<FrameRef>,
    pub clock_id: String,
    /// Minimum and maximum elapsed time allowed by the first/last operation bounds.
    pub elapsed_us: [u64; 2],
    /// Largest time between the end/start of consecutive supplied image operations.
    pub largest_gap_us: u64,
    /// Capture attempts between the endpoints that are not supplied as evidence.
    pub omitted_moments: usize,
    pub limitation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub input: Input,
    pub evidence: Sequence,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: usize,
    pub observer: String,
    pub submitted_by: String,
    pub key: String,
    pub label: String,
    pub unit: String,
    pub basis: Basis,
    pub epoch: usize,
    pub active: bool,
    /// Sorted by ending frame, not arrival order. At most one observation per end.
    pub observations: Vec<Observation>,
}

/// Resolve ordered samples without filling, resampling or inferring missing frames.
/// Wall clock is intentionally unused: it can jump while an investigation runs.
pub fn sequence(frames: &[FrameRef], ids: &[String]) -> Result<Sequence> {
    if !(2..=MAX_FRAMES).contains(&ids.len()) {
        return Err("Choose 2–8 distinct captures in chronological order.".into());
    }
    let selected: Vec<FrameRef> =
        ids.iter()
            .map(|id| {
                frames.iter().find(|f| &f.id == id).cloned().ok_or_else(|| {
                    "A requested frame is outside the current memory batch.".to_string()
                })
            })
            .collect::<Result<_>>()?;
    let first = &selected[0];
    fn timing(f: &FrameRef) -> Result<&visual::CaptureInterval> {
        f.capture_interval.as_ref().ok_or_else(|| {
            "Capture timing is unavailable. Collect fresh images for temporal evidence.".to_string()
        })
    }
    let first_time = timing(first)?;
    if first_time.clock_id.is_empty() {
        return Err("Capture timing has no clock identity.".into());
    }
    for frame in &selected {
        let t = timing(frame)?;
        if frame.epoch != first.epoch || frame.width != first.width || frame.height != first.height
        {
            return Err("Image geometry changed. Start a new temporal sequence.".into());
        }
        if t.clock_id != first_time.clock_id || t.end_us < t.begin_us {
            return Err("Capture times do not share a valid monotonic clock.".into());
        }
    }
    let mut largest_gap_us = 0;
    for pair in selected.windows(2) {
        let a = timing(&pair[0])?;
        let b = timing(&pair[1])?;
        if pair[0].number >= pair[1].number || a.end_us > b.begin_us {
            return Err(
                "Captures must be distinct, ordered and have non-overlapping timing bounds.".into(),
            );
        }
        largest_gap_us = largest_gap_us.max(b.begin_us - a.end_us);
    }
    let last = selected.last().unwrap();
    let last_time = timing(last)?;
    let elapsed_us = [
        last_time.begin_us - first_time.end_us,
        last_time.end_us - first_time.begin_us,
    ];
    let omitted_moments = last.number - first.number + 1 - selected.len();
    Ok(Sequence {
        clock_id: first_time.clock_id.clone(),
        elapsed_us,
        largest_gap_us,
        omitted_moments,
        frames: selected,
        limitation: LIMITATION.into(),
    })
}

/// Validate an entire declaration before changing any series. Persistence is the
/// caller's responsibility; use a cloned collection until the evidence is saved.
pub fn record(
    tracks: &mut Vec<Track>,
    input: Input,
    evidence: Sequence,
    source: &str,
) -> Result<bool> {
    if input.frames.len() != evidence.frames.len() {
        return Err("Temporal evidence does not match the declared images.".into());
    }
    let end = evidence
        .frames
        .last()
        .ok_or("Temporal evidence is empty.")?;
    for (anchor, frame) in input.frames.iter().zip(&evidence.frames) {
        visual::validate(
            &ObservationInput {
                investigation_id: input.investigation_id.clone(),
                frame_id: anchor.frame_id.clone(),
                image_sha256: anchor.image_sha256.clone(),
                observer: input.observer.clone(),
                key: input.key.clone(),
                label: input.label.clone(),
                unit: input.unit.clone(),
                value: input.value.clone(),
                quality: input.quality,
                bounds: anchor.bounds,
                explanation: input.explanation.clone(),
            },
            frame,
            end.number,
        )?;
    }
    if let Some(track) = tracks.iter_mut().find(|t| {
        t.epoch == end.epoch
            && t.observer == input.observer
            && t.key == input.key
            && t.submitted_by == source
    }) {
        if track.label != input.label || track.unit != input.unit || track.basis != input.basis {
            return Err(
                "This temporal series already has a different label, unit or basis. Use a new key."
                    .into(),
            );
        }
        if let Some(value) = &input.value
            && track
                .observations
                .iter()
                .filter_map(|o| o.input.value.as_ref())
                .any(|v| v.shape() != value.shape())
        {
            return Err(
                "A temporal series cannot change value type or dimensions. Use a new key.".into(),
            );
        }
        if let Some(old) = track
            .observations
            .iter()
            .find(|o| o.evidence.frames.last().unwrap().number == end.number)
        {
            return if old.input == input {
                Ok(false)
            } else {
                Err("This temporal series already has evidence ending at that frame. Use a new key to preserve an alternative.".into())
            };
        }
        if track.observations.len() >= MAX_OBSERVATIONS {
            return Err("This temporal series reached its 20 observation limit.".into());
        }
        track.observations.push(Observation { input, evidence });
        track
            .observations
            .sort_by_key(|o| o.evidence.frames.last().unwrap().number);
    } else {
        if tracks.len() >= MAX_TRACKS {
            return Err("This batch reached its 16 temporal series limit.".into());
        }
        tracks.push(Track {
            id: tracks.len(),
            observer: input.observer.clone(),
            submitted_by: source.into(),
            key: input.key.clone(),
            label: input.label.clone(),
            unit: input.unit.clone(),
            basis: input.basis,
            epoch: end.epoch,
            active: true,
            observations: vec![Observation { input, evidence }],
        });
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visual::CaptureInterval;
    fn frames() -> Vec<FrameRef> {
        (1..=4)
            .map(|i| FrameRef {
                id: i.to_string(),
                number: i,
                sha256: format!("hash{i}"),
                width: 100,
                height: 100,
                epoch: 0,
                started_unix_ms: 1000 - i as u128, // Wall clock goes backwards.
                capture_interval: Some(CaptureInterval {
                    clock_id: "clock".into(),
                    begin_us: i as u64 * 1000,
                    end_us: i as u64 * 1000 + 100,
                }),
            })
            .collect()
    }
    fn input(frames: &[FrameRef]) -> Input {
        Input {
            investigation_id: "investigation".into(),
            batch: 1,
            observer: "declared".into(),
            key: "shape".into(),
            label: "Shape change".into(),
            unit: "".into(),
            basis: Basis::Appearance,
            frames: frames
                .iter()
                .map(|f| Anchor {
                    frame_id: f.id.clone(),
                    image_sha256: f.sha256.clone(),
                    bounds: None,
                })
                .collect(),
            value: Some(VisualValue::Boolean(true)),
            quality: Quality::Provisional,
            explanation: "The sampled shape differs.".into(),
        }
    }
    fn selected(frames: &[FrameRef]) -> Sequence {
        sequence(
            frames,
            &frames.iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
        )
        .unwrap()
    }
    #[test]
    fn monotonic_operation_bounds_preserve_gaps_and_ignore_wall_clock_jumps() {
        let f = frames();
        let s = sequence(&f, &["1".into(), "4".into()]).unwrap();
        assert_eq!(s.elapsed_us, [2900, 3100]);
        assert_eq!(s.largest_gap_us, 2900);
        assert_eq!(s.omitted_moments, 2);
        assert!(sequence(&f, &["1".into(), "1".into()]).is_err());
        assert!(sequence(&f, &["4".into(), "1".into()]).is_err());
        assert!(sequence(&f, &["1".into()]).is_err());
        for change in 0..5 {
            let mut bad = f.clone();
            match change {
                0 => bad[1].capture_interval = None,
                1 => bad[1].capture_interval.as_mut().unwrap().clock_id = "other".into(),
                2 => bad[1].capture_interval.as_mut().unwrap().begin_us = 999,
                3 => bad[1].epoch = 1,
                _ => bad[1].width = 200,
            }
            assert!(sequence(&bad, &["1".into(), "2".into()]).is_err());
        }
    }
    #[test]
    fn immutable_windows_source_separation_and_out_of_order_arrival() {
        let f = frames();
        let mut tracks = vec![];
        let recent = input(&f[2..]);
        assert!(record(&mut tracks, recent.clone(), selected(&f[2..]), "agent").unwrap());
        assert!(record(&mut tracks, input(&f[..2]), selected(&f[..2]), "agent").unwrap());
        assert_eq!(tracks[0].observations.last().unwrap().input, recent);
        assert!(!record(&mut tracks, recent.clone(), selected(&f[2..]), "agent").unwrap());
        let mut bad = recent.clone();
        bad.frames[0].image_sha256 = "wrong".into();
        assert!(record(&mut tracks, bad, selected(&f[2..]), "agent").is_err());
        let mut bad = recent.clone();
        bad.value = Some(VisualValue::Boolean(false));
        assert!(record(&mut tracks, bad, selected(&f[2..]), "agent").is_err());
        assert!(record(&mut tracks, recent, selected(&f[2..]), "desktop").unwrap());
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].observations.len(), 2);
    }
    #[test]
    fn malformed_evidence_and_resource_limits_do_not_publish() {
        let f = frames();
        let mut tracks = vec![];
        for change in 0..4 {
            let mut bad = input(&f);
            match change {
                0 => bad.frames[0].bounds = Some([99., 0., 2., 2.]),
                1 => bad.value = Some(VisualValue::Number(f64::NAN)),
                2 => bad.value = None,
                _ => bad.label = "x".repeat(97),
            }
            assert!(record(&mut tracks, bad, selected(&f), "agent").is_err());
            assert!(tracks.is_empty());
        }
        for i in 0..MAX_TRACKS {
            let mut next = input(&f);
            next.key = i.to_string();
            record(&mut tracks, next, selected(&f), "agent").unwrap();
        }
        assert!(record(&mut tracks, input(&f), selected(&f), "agent").is_err());
        assert_eq!(tracks.len(), MAX_TRACKS);
    }
}
