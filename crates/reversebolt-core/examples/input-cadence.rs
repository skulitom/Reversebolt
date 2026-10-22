//! Offline timing sensitivity only: synthetic frames have no captured pixels.
use reversebolt_core::{
    Result,
    action_patterns::{Exposure, exposure},
    inputs::{Buffer, Poll},
    visual::{self, CaptureInterval, FrameRef},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

fn overlaps(frame: &FrameRef) -> bool {
    let t = frame.capture_interval.as_ref().unwrap();
    frame
        .input_context
        .as_ref()
        .unwrap()
        .polls
        .iter()
        .any(|p| p.begin_us <= t.end_us && p.end_us >= t.begin_us && !p.down.is_empty())
}

fn classify(frames: &[FrameRef]) -> Value {
    let exposures: Vec<_> = frames
        .windows(2)
        .map(|f| exposure(&f[0], &f[1]).unwrap_or_else(|e| Exposure::Unknown(e.into())))
        .collect();
    let mut counts = BTreeMap::new();
    let mut reasons = BTreeMap::new();
    let mut opportunities = BTreeMap::<u8, Vec<usize>>::new();
    let mut accepted_overlap = vec![];
    for (i, e) in exposures.iter().enumerate() {
        let kind = match e {
            Exposure::Key(_) => "key",
            Exposure::NoKeysDetected => "quiet",
            Exposure::Unknown(reason) => {
                *reasons.entry(reason).or_insert(0) += 1;
                "unknown"
            }
        };
        *counts.entry(kind).or_insert(0) += 1;
        if !matches!(e, Exposure::Unknown(_)) && (overlaps(&frames[i]) || overlaps(&frames[i + 1]))
        {
            accepted_overlap.push(i);
        }
        if let Exposure::Key(key) = e
            && i > 0
            && i + 1 < exposures.len()
            && exposures[i - 1] == Exposure::NoKeysDetected
            && exposures[i + 1] == Exposure::NoKeysDetected
        {
            opportunities.entry(*key).or_default().push(i);
        }
    }
    let repeated_keys: Vec<_> = opportunities
        .iter()
        .filter(|(_, ids)| ids.len() >= 2)
        .map(|(k, _)| k)
        .collect();
    json!({"counts":counts,"unknown_reasons":reasons,"exposures":exposures,
        "accepted_capture_overlap_intervals":accepted_overlap,
        "timing_only_opportunities":opportunities,"timing_only_repeated_keys":repeated_keys})
}

fn evaluate(root: &Path, name: &str) -> Result<Value> {
    let path = root.join(format!(
        "test-results/independent-games-v1/{name}-attempt-01/final-status.json"
    ));
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let status: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let frames: Vec<FrameRef> = serde_json::from_value(status["state"]["live"]["frames"].clone())
        .map_err(|e| e.to_string())?;
    if frames.len() != 16 {
        return Err("Unexpected source frame count".into());
    }
    let first = frames[0].input_context.as_ref().ok_or("Missing input")?;
    let mut polls = BTreeMap::<(u64, u64), Poll>::new();
    let mut original_omitted = 0;
    for frame in &frames {
        let w = frame.input_context.as_ref().ok_or("Missing input")?;
        if w.clock_id != first.clock_id
            || w.generation != first.generation
            || w.monitor_started_us != first.monitor_started_us
        {
            return Err("Input monitor changed".into());
        }
        original_omitted += w.omitted_polls;
        for p in &w.polls {
            if let Some(old) = polls.insert((p.begin_us, p.end_us), p.clone())
                && old != *p
            {
                return Err("Conflicting duplicate input".into());
            }
        }
    }
    let count = polls.len();
    let mut buffer = Buffer::new(
        first.clock_id.clone(),
        first.generation.clone(),
        first.monitor_started_us,
    );
    for p in polls.into_values() {
        buffer.push(p)?;
    }
    let origin = frames[0].capture_interval.as_ref().unwrap().begin_us;
    let mut costs: Vec<_> = frames
        .iter()
        .map(|f| {
            let t = f.capture_interval.as_ref().unwrap();
            t.end_us - t.begin_us
        })
        .collect();
    costs.sort();
    let mut scenarios = vec![];
    for cost_ms in [100u64, 200, 400] {
        for delay_ms in [50u64, 100, 250, 500] {
            for offset_ms in (0..350u64).step_by(25) {
                let mut hypothetical: Vec<FrameRef> = vec![];
                for index in 0..15 {
                    let begin =
                        origin + offset_ms * 1000 + index as u64 * (cost_ms + delay_ms) * 1000;
                    let end = begin + cost_ms * 1000;
                    let prior = hypothetical
                        .last()
                        .and_then(|f| f.capture_interval.as_ref())
                        .map_or(begin, |t| t.end_us);
                    let id = format!(
                        "timing-only-no-image/{name}/{cost_ms}/{delay_ms}/{offset_ms}/{index}"
                    );
                    hypothetical.push(FrameRef {
                        sampling: None,
                        input_context: buffer.window(prior, end),
                        sha256: visual::digest(id.as_bytes()),
                        id,
                        number: index + 1,
                        width: frames[0].width,
                        height: frames[0].height,
                        epoch: 0,
                        started_unix_ms: 0,
                        capture_interval: Some(CaptureInterval {
                            clock_id: first.clock_id.clone(),
                            begin_us: begin,
                            end_us: end,
                        }),
                    });
                }
                scenarios.push(json!({"cost_ms":cost_ms,"delay_ms":delay_ms,"offset_ms":offset_ms,"classification":classify(&hypothetical)}));
            }
        }
    }
    Ok(
        json!({"name":name,"status_sha256":visual::digest(&bytes),"retained_unique_polls":count,
        "original_omitted_polls":original_omitted,"original":classify(&frames),
        "original_capture_us":{"min":costs[0],"median_upper":costs[costs.len()/2],"max":costs[costs.len()-1]},
        "scenarios":scenarios}),
    )
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("Supply repository root and a new result path".into());
    }
    let cases: Vec<_> = ["2048-1", "tetris-1", "2048-2", "tetris-2"]
        .iter()
        .map(|n| evaluate(Path::new(&args[0]), n))
        .collect::<Result<_>>()?;
    let result = json!({"method":"input_cadence_sensitivity_v1","synthetic_images_have_pixels":false,
        "native_target_access":false,"new_model_calls":0,"cases":cases});
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, &result).map_err(|e| e.to_string())
}
