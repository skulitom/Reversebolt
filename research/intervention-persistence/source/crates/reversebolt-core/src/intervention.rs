//! Did the tested bytes remain at the requested value around a result capture?
//! Equal endpoints do not exclude ABA changes, display lag or hidden effects.
use crate::{
    changes::{Change, Model, Observation, ObservationStage},
    coverage::Chunk,
    live::NativeTarget,
    visual::CaptureInterval,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub const MAX_MODELS: usize = 200;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Read {
    pub bytes: Option<Vec<u8>>,
    pub interval: CaptureInterval,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub model: usize,
    pub address: u64,
    pub expected: Vec<u8>,
    pub before: Read,
    pub after: Option<Read>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    MemoryUnavailable,
    MemoryChanged,
    TimingUncertain,
    VisualUnavailable,
    Matched,
    DidNotFollow,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::MemoryUnavailable => "Test value couldn't be checked",
            Self::MemoryChanged => "Test value didn't persist",
            Self::TimingUncertain => "Result timing is uncertain",
            Self::VisualUnavailable => "Visible result couldn't be read",
            Self::Matched => "Visible value followed the test",
            Self::DidNotFollow => "Visible value didn't follow the test",
        }
    }

    pub fn explanation(self) -> &'static str {
        match self {
            Self::MemoryUnavailable => {
                "A full interpretation could not be read in the original allocation. Collect fresh evidence before drawing a conclusion about this location."
            }
            Self::MemoryChanged => {
                "At least one full interpretation differed from the expected bytes at a capture endpoint. Investigate persistence before treating the visible result as a test of this location."
            }
            Self::TimingUncertain => {
                "The memory reads did not bracket the result capture on one clock. This observation cannot distinguish a response from a timing mismatch."
            }
            Self::VisualUnavailable => {
                "The byte check completed but the selected visual reading was unavailable. A later observation may be useful; no response conclusion is available."
            }
            Self::Matched => {
                "All tested interpretations matched the expected bytes before and after capture, and the selected visible number agreed. Repeat under fresh conditions to investigate influence; this is not proof of gameplay authority."
            }
            Self::DidNotFollow => {
                "All tested interpretations matched the expected bytes before and after capture, but the selected visible number differed. Investigate another possibility or a delayed response. This does not rule out hidden effects or permanently eliminate this location."
            }
        }
    }
}

fn read(target: &dyn NativeTarget, model: &Model, clock: Instant, clock_id: &str) -> Read {
    let begin_us = clock.elapsed().as_micros() as u64;
    let result = (|| -> crate::Result<Vec<u8>> {
        if !target.alive() || model.before.len() != model.mapping.kind.width() {
            return Err("The target or interpretation is unavailable.".into());
        }
        let chunk = Chunk {
            address: model.address,
            length: model.before.len(),
            region: model.region.clone(),
        };
        let matches = || {
            target
                .query(model.address)
                .map(|region| region.is_some_and(|r| chunk.covered_by(&r)))
        };
        if !matches()? {
            return Err("The original allocation is unavailable.".into());
        }
        let bytes = target.read(model.address, model.before.len())?;
        if bytes.len() != model.before.len() {
            return Err("Incomplete interpretation bytes.".into());
        }
        if !target.alive() || !matches()? {
            return Err("The allocation changed during the read.".into());
        }
        Ok(bytes)
    })();
    Read {
        interval: CaptureInterval {
            clock_id: clock_id.into(),
            begin_us,
            end_us: clock.elapsed().as_micros() as u64,
        },
        bytes: result.as_ref().ok().cloned(),
        error: result.err(),
    }
}

pub fn begin(
    target: &dyn NativeTarget,
    change: &Change,
    stage: ObservationStage,
    clock: Instant,
    clock_id: &str,
) -> Vec<Sample> {
    // No guessed evidence for legacy/malformed records, and no unbounded reads.
    if change.models.is_empty()
        || change.models.len() > MAX_MODELS
        || change.models.iter().any(|m| {
            m.before.len() != m.mapping.kind.width()
                || m.before.len() > 8
                || m.address.checked_add(m.before.len() as u64).is_none()
        })
    {
        return vec![];
    }
    change
        .models
        .iter()
        .map(|model| {
            let mut expected = model.before.clone();
            if matches!(stage, ObservationStage::AfterEdit)
                && let Some(patch) = &change.patch
            {
                for (i, byte) in expected.iter_mut().enumerate() {
                    if let Some(offset) = (model.address + i as u64).checked_sub(patch.address)
                        && let Some(value) = usize::try_from(offset)
                            .ok()
                            .and_then(|o| patch.after.get(o))
                    {
                        *byte = *value;
                    }
                }
            }
            Sample {
                model: model.index,
                address: model.address,
                expected,
                before: read(target, model, clock, clock_id),
                after: None,
            }
        })
        .collect()
}

pub fn finish(
    target: &dyn NativeTarget,
    change: &Change,
    samples: &mut [Sample],
    clock: Instant,
    clock_id: &str,
) {
    for (sample, model) in samples.iter_mut().zip(&change.models) {
        sample.after = Some(read(target, model, clock, clock_id));
    }
}

pub fn assess(observation: &Observation, model_count: usize) -> Outcome {
    let samples = &observation.intervention_memory;
    // A detected mismatch remains a fact even if another interpretation is unreadable.
    if samples.iter().any(|s| {
        [&s.before, s.after.as_ref().unwrap_or(&s.before)]
            .iter()
            .any(|r| {
                r.error.is_none()
                    && r.bytes
                        .as_ref()
                        .is_some_and(|b| b.len() == s.expected.len() && *b != s.expected)
            })
    }) {
        return Outcome::MemoryChanged;
    }
    if samples.is_empty()
        || samples.len() != model_count
        || samples.iter().any(|s| {
            s.expected.is_empty()
                || s.expected.len() > 8
                || s.before.error.is_some()
                || s.before.bytes.as_ref() != Some(&s.expected)
                || s.after
                    .as_ref()
                    .is_none_or(|r| r.error.is_some() || r.bytes.as_ref() != Some(&s.expected))
        })
    {
        return Outcome::MemoryUnavailable;
    }
    let Some(capture) = &observation.capture_interval else {
        return Outcome::TimingUncertain;
    };
    if samples.iter().any(|s| {
        let first = &s.before.interval;
        let last = &s.after.as_ref().unwrap().interval;
        first.clock_id != capture.clock_id
            || last.clock_id != capture.clock_id
            || first.begin_us > first.end_us
            || first.end_us > capture.begin_us
            || capture.begin_us > capture.end_us
            || capture.end_us > last.begin_us
            || last.begin_us > last.end_us
    }) {
        return Outcome::TimingUncertain;
    }
    if observation.error.is_some() || observation.value.is_none_or(|v| !v.is_finite()) {
        return Outcome::VisualUnavailable;
    }
    if observation.agrees {
        Outcome::Matched
    } else {
        Outcome::DidNotFollow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation() -> Observation {
        let mut observation: Observation = serde_json::from_value(serde_json::json!({
            "time_ms": 1, "image": "fixture", "image_sha256": "fixture", "value": 53.,
            "expected": 53., "agrees": true, "error": null
        }))
        .unwrap();
        let interval = |begin_us, end_us| CaptureInterval {
            clock_id: "clock".into(),
            begin_us,
            end_us,
        };
        observation.capture_interval = Some(interval(3, 4));
        observation.intervention_memory.push(Sample {
            model: 0,
            address: 42,
            expected: vec![0, 0, 84, 66],
            before: Read {
                bytes: Some(vec![0, 0, 84, 66]),
                interval: interval(1, 2),
                error: None,
            },
            after: Some(Read {
                bytes: Some(vec![0, 0, 84, 66]),
                interval: interval(5, 6),
                error: None,
            }),
        });
        observation
    }

    #[test]
    fn visible_response_needs_complete_memory_and_capture_evidence() {
        let mut o = observation();
        assert_eq!(assess(&o, 1), Outcome::Matched);
        o.agrees = false;
        o.value = Some(50.);
        assert_eq!(assess(&o, 1), Outcome::DidNotFollow);
        o.error = Some("image resized".into());
        assert_eq!(assess(&o, 1), Outcome::VisualUnavailable);
        o.error = None;
        o.value = None;
        assert_eq!(assess(&o, 1), Outcome::VisualUnavailable);
        o.value = Some(53.);
        o.agrees = true;
        assert_eq!(assess(&o, 2), Outcome::MemoryUnavailable);
        o.intervention_memory[0].after = None;
        assert_eq!(assess(&o, 1), Outcome::MemoryUnavailable);
        o.intervention_memory.clear();
        assert_eq!(assess(&o, 0), Outcome::MemoryUnavailable);
    }

    #[test]
    fn changed_full_value_overrides_an_apparent_visual_response() {
        for before in [true, false] {
            let mut o = observation();
            let sample = &mut o.intervention_memory[0];
            let read = if before {
                &mut sample.before
            } else {
                sample.after.as_mut().unwrap()
            };
            // The low byte is outside the normalized one-byte trial patch.
            read.bytes.as_mut().unwrap()[0] ^= 1;
            assert_eq!(assess(&o, 1), Outcome::MemoryChanged);
        }
        let mut o = observation();
        o.intervention_memory[0].after.as_mut().unwrap().error = Some("mapping changed".into());
        assert_eq!(assess(&o, 1), Outcome::MemoryUnavailable);
        o.intervention_memory[0].after.as_mut().unwrap().error = None;
        o.intervention_memory[0].after.as_mut().unwrap().bytes = Some(vec![0]);
        assert_eq!(assess(&o, 1), Outcome::MemoryUnavailable);
    }

    #[test]
    fn equal_bytes_without_matching_ordered_clock_witnesses_are_inconclusive() {
        for fault in 0..4 {
            let mut o = observation();
            match fault {
                0 => o.capture_interval = None,
                1 => o.intervention_memory[0].before.interval.clock_id = "other".into(),
                2 => o.intervention_memory[0].before.interval.end_us = 4,
                _ => {
                    o.intervention_memory[0]
                        .after
                        .as_mut()
                        .unwrap()
                        .interval
                        .begin_us = 3
                }
            }
            assert_eq!(assess(&o, 1), Outcome::TimingUncertain);
        }
        let mut legacy = serde_json::to_value(observation()).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("intervention_memory");
        legacy
            .as_object_mut()
            .unwrap()
            .remove("intervention_outcome");
        let legacy: Observation = serde_json::from_value(legacy).unwrap();
        assert!(legacy.intervention_outcome.is_none());
        assert_eq!(assess(&legacy, 1), Outcome::MemoryUnavailable);
    }
}
