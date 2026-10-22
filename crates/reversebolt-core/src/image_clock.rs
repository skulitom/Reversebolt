//! Diagnostic correlation of compositor QPC timestamps and the input clock.
//! Never replaces capture-operation bounds or proves a game/presentation event.
use crate::{
    Result,
    visual::{CaptureInterval, FrameRef},
};
use serde::{Deserialize, Serialize};

pub const POLICY: &str = "qpc_to_monotonic_brackets_v1";
const MAX_BRACKET_US: u64 = 5_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub frequency_hz: u64,
    pub counter_ticks: i64,
    pub local_before_us: u64,
    pub local_after_us: u64,
}

impl Sample {
    fn validate(&self) -> Result<()> {
        if self.frequency_hz == 0
            || self.frequency_hz > i64::MAX as u64
            || self.counter_ticks < 0
            || self.local_after_us < self.local_before_us
            || self.local_after_us - self.local_before_us > MAX_BRACKET_US
        {
            return Err("Invalid or imprecise clock sample.".into());
        }
        Ok(())
    }

    fn bounds(&self, timestamp_100ns: i64) -> [i128; 2] {
        // i128 preserves large uptime counters. Outward rounding includes one
        // QPC tick for cross-thread ordering, one TimeSpan unit and the local
        // elapsed().as_micros() truncation; no floating-point epoch subtraction.
        let f = i128::from(self.frequency_hz);
        let q = i128::from(self.counter_ticks);
        let t = i128::from(timestamp_100ns);
        let denominator = 10 * f;
        let low = ((t - 1) * f - (q + 1) * 10_000_000).div_euclid(denominator);
        let high = -(-((t + 1) * f - (q - 1) * 10_000_000)).div_euclid(denominator);
        [
            i128::from(self.local_before_us) + low,
            i128::from(self.local_after_us) + 1 + high,
        ]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub policy: String,
    pub clock_id: String,
    pub before: Sample,
    pub after: Sample,
}

impl Receipt {
    pub fn project(
        &self,
        timestamp_100ns: i64,
        operation: &CaptureInterval,
    ) -> Result<CaptureInterval> {
        self.before.validate()?;
        self.after.validate()?;
        if self.policy != POLICY
            || self.clock_id.is_empty()
            || self.clock_id != operation.clock_id
            || timestamp_100ns <= 0
            || operation.end_us < operation.begin_us
            || self.before.frequency_hz != self.after.frequency_hz
            || self.after.counter_ticks <= self.before.counter_ticks
            || self.before.local_after_us > self.after.local_before_us
            || self.before.local_before_us < operation.begin_us
            || self.after.local_after_us > operation.end_us
        {
            return Err("Clock correlation has incompatible scope or chronology.".into());
        }
        let a = self.before.bounds(timestamp_100ns);
        let b = self.after.bounds(timestamp_100ns);
        let begin = a[0].max(b[0]);
        let end = a[1].min(b[1]);
        if begin > end
            || end - begin > i128::from(MAX_BRACKET_US)
            || begin < i128::from(self.before.local_before_us)
            || end > i128::from(self.after.local_after_us)
        {
            return Err("Compositor timestamp is stale, future, imprecise or inconsistent with the clock samples.".into());
        }
        Ok(CaptureInterval {
            clock_id: self.clock_id.clone(),
            begin_us: begin.try_into().map_err(|_| "Clock conversion underflow")?,
            end_us: end.try_into().map_err(|_| "Clock conversion overflow")?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameTime {
    pub policy: String,
    pub frame_id: String,
    pub image_sha256: String,
    pub selected: usize,
    pub rgba_sha256: String,
    pub compositor_100ns: i64,
    pub correlated_interval: CaptureInterval,
}

/// The caller must check the PNG hash and geometry, then hash its decoded RGBA pixels.
pub fn project_frame(frame: &FrameRef, rgba_sha256: &str) -> Result<Option<FrameTime>> {
    let Some(sampling) = &frame.sampling else {
        return Ok(None);
    };
    if let Some(error) = &sampling.clock_error {
        return Err(format!("Capture clock unavailable: {error}"));
    }
    let Some(receipt) = &sampling.clock else {
        return Ok(None);
    };
    if sampling.policy != "last_completed_in_fixed_window_v1"
        || sampling.max_frames != 8
        || sampling.settle_ms != 80
        || sampling.frames.is_empty()
        || sampling.frames.len() > 8
        || sampling.selected != sampling.frames.len() - 1
        || sampling
            .frames
            .windows(2)
            .any(|p| p[1].completed_us < p[0].completed_us)
        || sampling
            .frames
            .last()
            .unwrap()
            .completed_us
            .saturating_sub(sampling.frames[0].completed_us)
            >= 80_000
    {
        return Err("Unsupported or inconsistent captured-frame selection.".into());
    }
    let selected = &sampling.frames[sampling.selected];
    if frame.width == 0
        || frame.height == 0
        || selected.width != frame.width
        || selected.height != frame.height
        || selected.rgba_sha256.len() != 64
        || !selected.rgba_sha256.bytes().all(|b| b.is_ascii_hexdigit())
        || selected.rgba_sha256 != rgba_sha256
    {
        return Err("Compositor timing does not match the saved image pixels.".into());
    }
    let timestamp = selected
        .compositor_100ns
        .ok_or("Selected frame has no compositor timestamp.")?;
    let interval = receipt.project(
        timestamp,
        frame
            .capture_interval
            .as_ref()
            .ok_or("Missing capture-operation timing.")?,
    )?;
    Ok(Some(FrameTime {
        policy: POLICY.into(),
        frame_id: frame.id.clone(),
        image_sha256: frame.sha256.clone(),
        selected: sampling.selected,
        rgba_sha256: rgba_sha256.into(),
        compositor_100ns: timestamp,
        correlated_interval: interval,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(local: u64, q: i64, f: u64) -> Sample {
        Sample {
            frequency_hz: f,
            counter_ticks: q,
            local_before_us: local,
            local_after_us: local + 2,
        }
    }
    fn example() -> (Receipt, CaptureInterval) {
        (
            Receipt {
                policy: POLICY.into(),
                clock_id: "clock".into(),
                before: sample(1_000, 10_000_000, 10_000_000),
                after: sample(201_000, 12_000_000, 10_000_000),
            },
            CaptureInterval {
                clock_id: "clock".into(),
                begin_us: 900,
                end_us: 202_000,
            },
        )
    }
    #[test]
    fn conversion_contains_known_times_and_rejects_invalid_clock_evidence() {
        let (r, op) = example();
        let t = r.project(11_000_000, &op).unwrap();
        assert_eq!([t.begin_us, t.end_us], [100_999, 101_004]);
        for mode in 0..9 {
            let mut changed = r.clone();
            let stamp = match mode {
                0 => 9_999_000,
                1 => 12_001_000,
                2 => -1,
                3 => {
                    changed.after.frequency_hz = 3;
                    11_000_000
                }
                4 => {
                    changed.after.local_before_us += 20;
                    changed.after.local_after_us += 20;
                    11_000_000
                }
                5 => {
                    changed.before.local_after_us += 6_000;
                    11_000_000
                }
                6 => {
                    changed.clock_id = "other".into();
                    11_000_000
                }
                7 => {
                    changed.before.counter_ticks = i64::MAX;
                    11_000_000
                }
                _ => {
                    changed.after.frequency_hz = 0;
                    11_000_000
                }
            };
            assert!(changed.project(stamp, &op).is_err(), "case {mode}");
        }
        // Non-divisible counter frequencies and large uptimes retain integer precision.
        for f in [3_579_545u64, 10_000_000, 1_000_000_000] {
            let epoch = 100_000i64;
            let r = Receipt {
                before: sample(1_000, epoch * f as i64, f),
                after: sample(1_001_000, (epoch + 1) * f as i64, f),
                ..r.clone()
            };
            let op = CaptureInterval {
                end_us: 1_002_000,
                ..op.clone()
            };
            let t = r.project(epoch * 10_000_000 + 5_000_000, &op).unwrap();
            assert!(t.begin_us <= 501_000 && t.end_us >= 501_002);
            assert!(t.end_us - t.begin_us < 10);
        }
    }

    #[test]
    fn selected_image_binding_rejects_wrong_pixels_selection_scope_and_missing_time() {
        let (receipt, operation) = example();
        let pixels = "a".repeat(64);
        let mut frame: FrameRef = serde_json::from_value(serde_json::json!({
            "id":"frame-1", "number":1, "sha256":"b".repeat(64),
            "width":40, "height":30, "epoch":0, "started_unix_ms":1,
            "capture_interval":operation,
            "sampling":{"policy":"last_completed_in_fixed_window_v1","settle_ms":80,
                "max_frames":8,"selected":0,"clock":receipt,
                "frames":[{"completed_us":10000,"compositor_100ns":11000000,
                    "width":40,"height":30,"rgba_sha256":pixels,"detail_fraction":0.1}]}
        }))
        .unwrap();
        let bound = project_frame(&frame, &pixels).unwrap().unwrap();
        assert_eq!(bound.image_sha256, frame.sha256);
        assert_eq!(bound.frame_id, frame.id);
        assert_eq!(bound.selected, 0);
        assert!(project_frame(&frame, &"c".repeat(64)).is_err());
        for case in 0..8 {
            let mut bad = frame.clone();
            let sampling = bad.sampling.as_mut().unwrap();
            match case {
                0 => sampling.selected = 1,
                1 => sampling.policy = "another-policy".into(),
                2 => sampling.frames[0].compositor_100ns = None,
                3 => sampling.clock_error = Some("Counter unavailable".into()),
                4 => bad.capture_interval = None,
                5 => bad.width = 41,
                6 => {
                    sampling.frames.push(sampling.frames[0].clone());
                    sampling.frames[1].completed_us += 80_000;
                    sampling.selected = 1;
                }
                _ => sampling.clock.as_mut().unwrap().clock_id = "different".into(),
            }
            assert!(project_frame(&bad, &pixels).is_err(), "case {case}");
        }
        // Old receipts keep their exact JSON shape and yield no correlation.
        frame.sampling.as_mut().unwrap().clock = None;
        let legacy = serde_json::to_value(&frame).unwrap();
        assert!(legacy["sampling"].get("clock").is_none());
        assert!(legacy["sampling"].get("clock_error").is_none());
        let decoded: FrameRef = serde_json::from_value(legacy.clone()).unwrap();
        assert!(project_frame(&decoded, &pixels).unwrap().is_none());
        assert_eq!(serde_json::to_value(decoded).unwrap(), legacy);
    }
}
