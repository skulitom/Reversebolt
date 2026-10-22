//! Local clock APIs only. No window capture, process access, input or model calls.
#[path = "../src/capture_clock.rs"]
mod capture_clock;
use reversebolt_core::{
    Result,
    live::Screen,
    visual::{CaptureInterval, CaptureSampling, SampledFrame},
};
use serde_json::json;
use std::{
    path::Path,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .ok_or("Supply a new local result file")?;
    let origin = Instant::now();
    let mut results = vec![];
    for index in 0..100 {
        let begin = origin.elapsed().as_micros() as u64;
        let mut center = None;
        let screen = capture_clock::capture("clock-only-probe", origin, || {
            std::thread::sleep(Duration::from_millis(1));
            let sample = if index % 2 == 0 {
                capture_clock::sample(origin)
            } else {
                std::thread::spawn(move || capture_clock::sample(origin))
                    .join()
                    .map_err(|_| "Clock worker failed")?
            }?;
            let stamp = (i128::from(sample.counter_ticks) * 10_000_000
                / i128::from(sample.frequency_hz)) as i64;
            center = Some(sample);
            std::thread::sleep(Duration::from_millis(1));
            Ok(Screen {
                sampling: Some(CaptureSampling {
                    clock: None,
                    clock_error: None,
                    policy: "last_completed_in_fixed_window_v1".into(),
                    settle_ms: 80,
                    max_frames: 8,
                    selected: 0,
                    frames: vec![SampledFrame {
                        completed_us: 0,
                        compositor_100ns: Some(stamp),
                        width: 1,
                        height: 1,
                        rgba_sha256: String::new(),
                        detail_fraction: 0.,
                    }],
                }),
                path: String::new(),
                width: 1,
                height: 1,
                text: String::new(),
                words: vec![],
                warning: None,
                method: "clock-only-no-image".into(),
            })
        })?;
        let operation = CaptureInterval {
            clock_id: "clock-only-probe".into(),
            begin_us: begin,
            end_us: origin.elapsed().as_micros() as u64,
        };
        let sampling = screen.sampling.unwrap();
        let center = center.unwrap();
        let result = sampling
            .clock
            .as_ref()
            .ok_or("Clock anchors unavailable".into())
            .and_then(|r| r.project(sampling.frames[0].compositor_100ns.unwrap(), &operation));
        let record = match result {
            Ok(interval) => {
                json!({"index":index,"other_thread":index%2==1,"overlaps_measured_clock_sample":interval.begin_us<=center.local_after_us+1 && interval.end_us>=center.local_before_us,
                "width_us":interval.end_us-interval.begin_us,"interval":interval,"error":null})
            }
            Err(error) => {
                json!({"index":index,"other_thread":index%2==1,"overlaps_measured_clock_sample":false,"error":error})
            }
        };
        results.push(json!({"measurement":record,"center":center,"operation":operation,"anchors":sampling.clock,"clock_error":sampling.clock_error}));
    }
    let passed = results
        .iter()
        .filter(|r| r["measurement"]["overlaps_measured_clock_sample"] == true)
        .count();
    let result = json!({"method":"clock_api_correlation_probe_v1","samples":100,"consistent":passed,"results":results,
        "image_captured":false,"compositor_api_called":false,"new_model_calls":0,"native_target_access":false});
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(&output))
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, &result).map_err(|e| e.to_string())
}
