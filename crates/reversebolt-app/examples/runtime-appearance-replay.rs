//! Exercise the production appearance worker and reader on saved images only.
//! Module unit tests run in the app target; this executable has no live backend.
#![cfg(not(test))]
#[allow(dead_code)]
#[path = "../src/local_observer.rs"]
mod local_observer;
#[allow(dead_code)]
#[path = "../src/play_evidence.rs"]
mod play_evidence;

use reversebolt_core::{
    Result,
    live::{LiveSession, NativeTarget, ProcessInfo, Region, Screen},
    visual::{self, FrameRef},
};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

struct SavedOnly(ProcessInfo);
impl NativeTarget for SavedOnly {
    fn info(&self) -> &ProcessInfo {
        &self.0
    }
    fn alive(&self) -> bool {
        true
    }
    fn regions(&self) -> Result<Vec<Region>> {
        Ok(vec![Region {
            base: 4096,
            length: 4096,
            ..Default::default()
        }])
    }
    fn read(&self, _: u64, _: usize) -> Result<Vec<u8>> {
        panic!("No native reads in saved replay")
    }
    fn screen(&self, _: &Path) -> Result<Screen> {
        panic!("No new captures in saved replay")
    }
    fn guarded_write(
        &self,
        _: &reversebolt_core::changes::BytePatch,
        _: &Region,
    ) -> reversebolt_core::changes::WriteReceipt {
        panic!("No writes in saved replay")
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    file: String,
    reference: FrameRef,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    investigation_id: String,
    batch: usize,
    frames: Vec<Image>,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 && !(args.len() == 5 && args[3] == "--observations") {
        return Err("usage: runtime-appearance-replay CASE_JSON MODEL NEW_OUTPUT_DIRECTORY [--observations PRIOR_RESULT_JSON]".into());
    }
    let mut bytes = vec![];
    fs::File::open(&args[0])?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Manifest exceeds budget".into());
    }
    let case: Case = serde_json::from_slice(&bytes)?;
    if !(3..=20).contains(&case.frames.len())
        || case
            .frames
            .iter()
            .enumerate()
            .any(|(i, f)| f.reference.number != i + 1 || f.reference.epoch != 0)
    {
        return Err(
            "Replay requires 3-20 consecutive saved captures in their initial epoch".into(),
        );
    }
    let output = Path::new(&args[2]);
    fs::create_dir(output)?;
    for frame in &case.frames {
        let png = visual::image_bytes(Path::new(&frame.file))?;
        if visual::digest(&png) != frame.reference.sha256 {
            return Err("Saved image changed".into());
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(format!("frame-{:03}.png", frame.reference.number)))?;
        file.write_all(&png)?;
    }
    let target = SavedOnly(ProcessInfo {
        pid: 1,
        created: "0000000000000001".into(),
        window: 1,
        name: "saved-image-replay".into(),
        ..Default::default()
    });
    let mut session = LiveSession::attach(Box::new(target), 1, output.to_owned())?;
    session.view.investigation_id = case.investigation_id;
    session.view.coverage.batch = case.batch;
    session.view.frames = case.frames[..2]
        .iter()
        .map(|f| f.reference.clone())
        .collect();
    session.view.sample_count = 2;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut worker = if args.len() == 3 {
        Some(local_observer::Worker::start(
            local_observer::Prepared::from_session(&session)?,
            args[1].clone(),
            1,
            cancel.clone(),
            output.to_owned(),
        )?)
    } else {
        None
    };
    // Simulate completed later captures without acquiring any new evidence.
    session.view.frames = case.frames.iter().map(|f| f.reference.clone()).collect();
    session.view.sample_count = case.frames.len();
    let ids: Vec<_> = session.view.frames.iter().map(|f| f.id.clone()).collect();
    let request = json!({"investigation_id":session.view.investigation_id,"batch":case.batch,"frame_ids":ids});
    let without_observations = play_evidence::read(&session, &request)?;
    let started = Instant::now();
    let publication = if let Some(worker) = &mut worker {
        let completed = loop {
            if let Some(result) = worker.poll() {
                break result?;
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        completed.publish(&mut session, &cancel)?
    } else {
        let mut cached = vec![];
        fs::File::open(&args[4])?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut cached)?;
        if cached.len() > 8 * 1024 * 1024 {
            return Err("Observation receipt exceeds budget".into());
        }
        let data: serde_json::Value = serde_json::from_slice(&cached)?;
        let tracks: Vec<visual::VisualTrack> =
            serde_json::from_value(data["visual_tracks"].clone())?;
        if tracks.len() > visual::MAX_TRACKS {
            return Err("Too many retained tracks".into());
        }
        let mut observations = vec![];
        for track in tracks {
            for observation in track.observations.into_iter().flatten() {
                if observation.submitted_by != "local-model" {
                    return Err("Only retained local-model observations are accepted".into());
                }
                observations.push(observation.input);
            }
        }
        let recorded = session.submit_observations(observations, "local-model")?;
        json!({"recorded":recorded,"replayed_observations":true,"new_model_calls":0,"receipt_sha256":visual::digest(&cached)})
    };
    let evidence = play_evidence::read(&session, &request)?;
    let result = json!({"schema":1,"name":case.name,"manifest_sha256":visual::digest(&bytes),
        "model":if worker.is_some() {Some(&args[1])} else {None},"seconds":started.elapsed().as_secs_f64(),"publication":publication,
        "visual_tracks":session.view.visual_tracks,"request":request,"evidence":evidence,
        "without_observations":without_observations,
        "native_target_access":false,"new_captures":0,"inputs_sent":0,"field_writes":0,
        "limitation":"Replay of previously exposed images through the actual worker and reader. Not a new game run, ground truth, persistent identity or verified control. Native scheduling is covered separately by automated tests."});
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("result.json"))?;
    serde_json::to_writer_pretty(file, &result)?;
    println!(
        "{}",
        json!({"name":case.name,"publication":result["publication"],"seconds":result["seconds"]})
    );
    Ok(())
}
