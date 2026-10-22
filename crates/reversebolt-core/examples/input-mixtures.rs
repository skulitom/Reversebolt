//! Saved-input feature experiment only; no native, image capture or model access.
use reversebolt_core::{
    Result,
    inputs::{Buffer, Focus, Poll},
    visual::{self, FrameRef},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

fn profiles(a: &FrameRef, b: &FrameRef) -> Result<Vec<Value>> {
    let ta = a
        .capture_interval
        .as_ref()
        .ok_or("Missing capture timing")?;
    let tb = b
        .capture_interval
        .as_ref()
        .ok_or("Missing capture timing")?;
    let wa = a.input_context.as_ref().ok_or("Missing previous input")?;
    let wb = b.input_context.as_ref().ok_or("Missing input")?;
    if a.number.checked_add(1) != Some(b.number)
        || a.epoch != b.epoch
        || a.width != b.width
        || a.height != b.height
        || a.width == 0
        || a.height == 0
        || ta.clock_id.is_empty()
        || ta.clock_id != tb.clock_id
        || wa.clock_id != ta.clock_id
        || wb.clock_id != ta.clock_id
        || wa.generation.is_empty()
        || wa.generation != wb.generation
        || wa.monitor_started_us != wb.monitor_started_us
        || ta.begin_us > ta.end_us
        || tb.begin_us > tb.end_us
        || ta.end_us >= tb.begin_us
        || tb.end_us - ta.begin_us > 5_000_000
        || wa.requested_begin_us > ta.begin_us
        || wa.requested_end_us != ta.end_us
        || wb.requested_begin_us != ta.end_us
        || wb.requested_end_us != tb.end_us
        || wa.monitor_started_us > ta.begin_us
        || wa.retained_from_us > ta.begin_us
        || wb.retained_from_us > ta.end_us
        || wa.omitted_polls != 0
        || wb.omitted_polls != 0
        || wa.polls.len() > reversebolt_core::inputs::MAX_WINDOW_POLLS
        || wb.polls.len() > reversebolt_core::inputs::MAX_WINDOW_POLLS
    {
        return Err("Incompatible or incomplete source timing/input scope".into());
    }
    let mut merged = BTreeMap::<(u64, u64), Poll>::new();
    for window in [wa, wb] {
        let mut cursor = window.requested_begin_us;
        let mut largest = 0;
        for p in &window.polls {
            if p.begin_us < cursor || p.end_us < p.begin_us || p.end_us > window.requested_end_us {
                return Err("Invalid source poll order".into());
            }
            largest = largest.max(p.begin_us - cursor);
            cursor = p.end_us;
            if let Some(old) = merged.insert((p.begin_us, p.end_us), p.clone())
                && old != *p
            {
                return Err("Conflicting duplicate sample".into());
            }
        }
        largest = largest.max(window.requested_end_us - cursor);
        if largest != window.largest_unsampled_us {
            return Err("Source sampling-gap receipt changed".into());
        }
    }
    let mut buffer = Buffer::new(
        wa.clock_id.clone(),
        wa.generation.clone(),
        wa.monitor_started_us,
    );
    let polls: Vec<_> = merged.into_values().collect();
    for p in &polls {
        buffer.push(p.clone())?;
    }
    let variants = [
        ("interior", ta.end_us, tb.begin_us),
        ("early_early", ta.begin_us, tb.begin_us),
        ("late_late", ta.end_us, tb.end_us),
        ("outer", ta.begin_us, tb.end_us),
    ];
    Ok(variants.into_iter().map(|(name,begin,end)| {
        // Preserve the union of two individually bounded receipts (<=512 polls).
        // Buffer::window would truncate a longer union to one receipt's 256 cap.
        let selected:Vec<_> = polls.iter().filter(|p|p.begin_us>=begin && p.end_us<=end).collect();
        let mut cursor=begin;
        let mut largest_gap=0;
        for p in &selected { largest_gap=largest_gap.max(p.begin_us-cursor); cursor=p.end_us; }
        largest_gap=largest_gap.max(end-cursor);
        if selected.len()<3 || largest_gap>100_000
            || selected.iter().any(|p|p.focus!=Focus::SelectedForeground || p.end_us-p.begin_us>100_000) {
            return json!({"variant":name,"error":"Incomplete foreground or input sampling coverage"});
        }
        let mut keys = BTreeMap::<u8,usize>::new();
        for p in &selected { for k in &p.down { *keys.entry(*k).or_default()+=1; } }
        json!({"variant":name,"begin_us":begin,"end_us":end,"duration_us":end-begin,
            "polls":selected.len(),"key_samples":keys,
            "chord_samples":selected.iter().filter(|p|p.down.len()>1).count(),
            "largest_gap_us":largest_gap,"error":null})
    }).collect())
}

fn evaluate(root: &Path, name: &str) -> Result<Value> {
    let status_path = root.join(format!(
        "test-results/independent-games-v1/{name}-attempt-01/final-status.json"
    ));
    let bytes = std::fs::read(&status_path).map_err(|e| e.to_string())?;
    let status: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let frames: Vec<FrameRef> = serde_json::from_value(status["state"]["live"]["frames"].clone())
        .map_err(|e| e.to_string())?;
    let result_path = root.join(format!(
        "test-results/browser-activity-replay-v1/{name}/result.json"
    ));
    let result_bytes = std::fs::read(&result_path).map_err(|e| e.to_string())?;
    let cached: Value = serde_json::from_slice(&result_bytes).map_err(|e| e.to_string())?;
    let intervals = cached["evidence"]["evidence"]["intervals"]
        .as_array()
        .ok_or("Missing cached measurements")?;
    if frames.len() != 16 || intervals.len() != 15 {
        return Err("Unexpected source sample count".into());
    }
    let mut rows = vec![];
    for (pair, measurement) in frames.windows(2).zip(intervals) {
        let (a, b) = (&pair[0], &pair[1]);
        if measurement["before"] != a.id
            || measurement["after"] != b.id
            || measurement["before_sha256"] != a.sha256
            || measurement["after_sha256"] != b.sha256
        {
            return Err("Cached image measurement source changed".into());
        }
        let features = profiles(a, b);
        rows.push(json!({"before":a.id,"after":b.id,"before_sha256":a.sha256,"after_sha256":b.sha256,
            "measurement":measurement["measurement"],"variants":features.as_ref().ok(),"error":features.err()}));
    }
    Ok(
        json!({"name":name,"source_sha256":visual::digest(&bytes),"cached_result_sha256":visual::digest(&result_bytes),"rows":rows}),
    )
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("Supply repository root and a new output file".into());
    }
    let root = Path::new(&args[0]);
    let mut cases = vec![];
    for name in ["2048-1", "tetris-1", "2048-2", "tetris-2"] {
        cases.push(match evaluate(root, name) {
            Ok(case) => case,
            Err(error) => json!({"name":name,"error":error}),
        });
    }
    let output = json!({"policy":"sample_fraction_mixtures_v1","cases":cases,
        "native_target_access":false,"new_captures":0,"inputs_sent":0,"new_model_calls":0,
        "limitation":"Research sample fractions, not holds, isolated effects or executable actions. Product inference unchanged."});
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, &output).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(step: usize, first_end: u64, second_begin: u64, end: u64) -> (FrameRef, FrameRef) {
        let mut buffer = Buffer::new("clock".into(), "generation".into(), 0);
        for t in (0..end).step_by(step) {
            buffer
                .push(Poll {
                    begin_us: t,
                    end_us: t + 100,
                    focus: Focus::SelectedForeground,
                    down: if (200_000..350_000).contains(&t) {
                        vec![65]
                    } else if (500_000..700_000).contains(&t) {
                        vec![66]
                    } else {
                        vec![]
                    },
                })
                .unwrap();
        }
        let frame = |n, begin, end, window| {
            serde_json::from_value(json!({
                "id":format!("frame-{n}"),"number":n,"width":100,"height":100,
                "sha256":"a".repeat(64),"epoch":0,"started_unix_ms":0,
                "capture_interval":{"clock_id":"clock","begin_us":begin,"end_us":end},
                "input_context":window
            }))
            .unwrap()
        };
        (
            frame(1, 0, first_end, buffer.window(0, first_end)),
            frame(2, second_begin, end, buffer.window(first_end, end)),
        )
    }
    #[test]
    fn mixtures_preserve_both_keys_without_turning_them_into_isolated_actions() {
        let (a, b) = fixture(10_000, 100_000, 900_000, 1_000_000);
        assert!(reversebolt_core::action_patterns::exposure(&a, &b).is_err());
        let values = profiles(&a, &b).unwrap();
        assert_eq!(values[0]["polls"], 80);
        assert_eq!(values[0]["key_samples"], json!({"65":15,"66":20}));
        assert_eq!(values[0]["chord_samples"], 0);
        assert!(values.iter().all(|v| v["error"].is_null()));
    }
    #[test]
    fn complete_receipt_union_does_not_silently_truncate_outer_samples() {
        let (a, b) = fixture(16_000, 200_000, 4_000_000, 4_200_000);
        let values = profiles(&a, &b).unwrap();
        assert_eq!(values[3]["polls"], 263);
        assert!(values.iter().all(|v| v["error"].is_null()));
    }
    #[test]
    fn invalid_input_and_foreground_remain_errors() {
        let (a, b) = fixture(10_000, 100_000, 900_000, 1_000_000);
        for case in 0..4 {
            let mut changed = b.clone();
            let window = changed.input_context.as_mut().unwrap();
            match case {
                0 => window.generation = "other".into(),
                1 => window.omitted_polls = 1,
                2 => window.polls.swap(0, 1),
                _ => window.largest_unsampled_us += 1,
            }
            assert!(profiles(&a, &changed).is_err(), "case {case}");
        }
        let mut changed = b;
        let p = &mut changed.input_context.as_mut().unwrap().polls[15];
        p.focus = Focus::OutsideSelected;
        p.down.clear();
        assert!(
            profiles(&a, &changed)
                .unwrap()
                .iter()
                .all(|v| v["error"].is_string())
        );
    }
}
