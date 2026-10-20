//! Observation and memory capture from unmodified external programs.
//! No executable-name rules, game offsets or source-derived layouts.
use crate::{
    Result,
    discovery::{Candidate, discover},
    visual::{self, FrameRef, ObservationInput, VisualTrack},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

const CHUNK_BYTES: usize = 64 * 1024;
const MAX_SAMPLES: usize = 20;
const RETAINED_MEMORY_BUDGET: usize = 512 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub title: String,
    pub window: u64,
    pub session: u32,
    /// Windows process creation time, checked again at attach.
    pub created: String,
}

#[derive(Clone, Debug)]
pub struct Region {
    pub base: u64,
    pub length: usize,
    pub image: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Word {
    pub text: String,
    pub bounds: [f32; 4],
    pub line: usize,
    #[serde(default)]
    pub source: TextSource,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSource {
    #[default]
    WindowsOcr,
    Ocrs,
    OcrsRegion,
}

/// A provisional reading, never a verified semantic or authoritative value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NumberReading {
    pub bounds: [f32; 4],
    pub value: Option<f64>,
    pub reason: String,
    pub alternatives: Vec<Word>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Screen {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub text: String,
    pub words: Vec<Word>,
    pub warning: Option<String>,
    #[serde(default)]
    pub method: String,
}

pub trait NativeTarget {
    fn info(&self) -> &ProcessInfo;
    fn regions(&self) -> Result<Vec<Region>>;
    fn read(&self, address: u64, length: usize) -> Result<Vec<u8>>;
    fn screen(&self, path: &Path) -> Result<Screen>;
    /// Run recognition after memory capture to avoid adding inference time to
    /// the screen-to-memory sampling delay. Fixtures may supply words directly.
    fn recognize(&self, _screen: &mut Screen) -> Result<()> {
        Ok(())
    }
    fn alive(&self) -> bool;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocatedCandidate {
    pub address: String,
    pub mapping: Candidate,
    pub evidence: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NumberTrack {
    pub id: usize,
    pub label: String,
    pub bounds: [f32; 4],
    pub samples: Vec<Option<f64>>,
    pub matches: Vec<LocatedCandidate>,
    #[serde(default)]
    pub readings: Vec<Option<NumberReading>>,
    #[serde(default = "active_default")]
    pub active: bool,
}

fn active_default() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaptureSummary {
    pub number: usize,
    pub started_unix_ms: u128,
    pub elapsed_ms: u128,
    pub memory_read_ms: u128,
    #[serde(default)]
    pub screen_capture_ms: u128,
    #[serde(default)]
    pub recognition_ms: u128,
    pub captured_bytes: usize,
    pub unreadable_bytes: usize,
    pub screen: Option<Screen>,
    pub screen_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveView {
    #[serde(default)]
    pub investigation_id: String,
    #[serde(default)]
    pub frames: Vec<FrameRef>,
    #[serde(default)]
    pub visual_tracks: Vec<VisualTrack>,
    pub process: ProcessInfo,
    pub eligible_bytes: u64,
    pub planned_bytes: usize,
    pub sample_count: usize,
    pub retained_bytes: usize,
    pub latest: Option<CaptureSummary>,
    pub tracks: Vec<NumberTrack>,
    pub phase: String,
    pub connected: bool,
}

pub struct LiveSession {
    pub view: LiveView,
    target: Box<dyn NativeTarget>,
    plan: Vec<(u64, usize)>,
    snapshots: Vec<BTreeMap<u64, Vec<u8>>>,
    directory: PathBuf,
    screen_size: Option<(u32, u32)>,
    image_epoch: usize,
}

impl LiveSession {
    /// Retain the evidence, but revoke the connection and hypotheses on exit.
    pub fn refresh_connection(&mut self) -> bool {
        if self.view.connected && !self.target.alive() {
            self.view.connected = false;
            self.view.phase =
                "The application closed. Connect again to start a new investigation.".into();
            for track in &mut self.view.tracks {
                track.matches.clear();
            }
            for track in &mut self.view.visual_tracks {
                track.matches.clear();
                track.active = false;
            }
            return true;
        }
        false
    }

    pub fn attach(
        target: Box<dyn NativeTarget>,
        budget_mib: usize,
        directory: PathBuf,
    ) -> Result<Self> {
        if !(1..=32).contains(&budget_mib) {
            return Err("Memory capture budget must be 1–32 MiB.".into());
        }
        let mut regions = target.regions()?;
        let eligible_bytes = regions.iter().map(|r| r.length as u64).sum();
        // Writable image pages first, then smaller private regions. This is a
        // general prioritization under a budget, not a game layout heuristic.
        regions.sort_by_key(|r| (!r.image, r.length, r.base));
        let mut remaining = budget_mib * 1024 * 1024;
        let mut plan = vec![];
        for region in regions {
            let mut offset = 0;
            while offset < region.length && remaining > 0 {
                let length = CHUNK_BYTES.min(region.length - offset).min(remaining);
                plan.push((region.base + offset as u64, length));
                offset += length;
                remaining -= length;
            }
            if remaining == 0 {
                break;
            }
        }
        let planned_bytes = plan.iter().map(|(_, n)| *n).sum();
        if planned_bytes == 0 {
            return Err("No readable writable memory regions were found.".into());
        }
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let view = LiveView {
            investigation_id: uuid::Uuid::new_v4().to_string(),
            frames: vec![],
            visual_tracks: vec![],
            process: target.info().clone(),
            eligible_bytes,
            planned_bytes,
            sample_count: 0,
            retained_bytes: 0,
            latest: None,
            tracks: vec![],
            connected: true,
            phase: "Connected for observation. Capture while the program changes.".into(),
        };
        Ok(Self {
            view,
            target,
            plan,
            snapshots: vec![],
            directory,
            screen_size: None,
            image_epoch: 0,
        })
    }

    pub fn capture(&mut self) -> Result<()> {
        if self.refresh_connection() || !self.view.connected {
            return Err(
                "The selected process has exited. Reconnect before collecting more evidence."
                    .into(),
            );
        }
        if self.snapshots.len() >= MAX_SAMPLES
            || (self.snapshots.len() + 1) * self.view.planned_bytes > RETAINED_MEMORY_BUDGET
        {
            return Err("This capture session reached its memory/sample budget. Analyze it or reconnect for a fresh session.".into());
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let start = Instant::now();
        let number = self.snapshots.len() + 1;
        let screen_path = self.directory.join(format!("frame-{number:03}.png"));
        let mut screen = self.target.screen(&screen_path);
        let screen_capture_ms = start.elapsed().as_millis();
        let memory_start = Instant::now();
        let mut snapshot = BTreeMap::new();
        let mut unreadable = 0;
        for &(address, length) in &self.plan {
            match self.target.read(address, length) {
                Ok(bytes) if bytes.len() == length => {
                    snapshot.insert(address, bytes);
                }
                _ => unreadable += length,
            }
        }
        let captured = snapshot.values().map(Vec::len).sum::<usize>();
        let memory_read_ms = memory_start.elapsed().as_millis();
        if captured == 0 {
            return Err(
                "None of the planned memory could be read. Reconnect to refresh the memory map."
                    .into(),
            );
        }
        let recognition_start = Instant::now();
        if let Ok(frame) = &mut screen
            && let Err(error) = self.target.recognize(frame)
        {
            frame.warning = Some(match frame.warning.take() {
                Some(previous) => format!("{previous} {error}"),
                None => error,
            });
        }
        let recognition_ms = recognition_start.elapsed().as_millis();
        self.snapshots.push(snapshot);
        if let Ok(frame) = &screen {
            let size = (frame.width, frame.height);
            if self.screen_size.is_some_and(|previous| previous != size) {
                self.image_epoch += 1;
                for track in &mut self.view.visual_tracks {
                    track.active = false;
                    track.matches.clear();
                }
                for track in &mut self.view.tracks {
                    track.active = false;
                    track.matches.clear();
                }
            }
            self.screen_size = Some(size);
            track_numbers(&mut self.view.tracks, &frame.words, number);
        } else {
            for track in &mut self.view.tracks {
                track.samples.push(None);
                track.readings.push(None);
                track.matches.clear();
            }
        }
        for track in &mut self.view.visual_tracks {
            track.observations.push(None);
            track.matches.clear();
        }
        if let Ok(frame) = &mut screen {
            match visual::image_bytes(&screen_path) {
                Ok(bytes) => self.view.frames.push(FrameRef {
                    id: uuid::Uuid::new_v4().to_string(),
                    number,
                    sha256: visual::digest(&bytes),
                    width: frame.width,
                    height: frame.height,
                    epoch: self.image_epoch,
                    started_unix_ms: now,
                }),
                Err(error) => {
                    let note = format!("Image evidence unavailable: {error}");
                    frame.warning = Some(match frame.warning.take() {
                        Some(previous) => format!("{previous} {note}"),
                        None => note,
                    });
                }
            }
        }
        self.view.sample_count = number;
        self.view.retained_bytes += captured;
        self.view.latest = Some(CaptureSummary {
            number,
            started_unix_ms: now,
            elapsed_ms: start.elapsed().as_millis(),
            memory_read_ms,
            screen_capture_ms,
            recognition_ms,
            captured_bytes: captured,
            unreadable_bytes: unreadable,
            screen_error: screen.as_ref().err().cloned(),
            screen: screen.ok(),
        });
        if !self.view.frames.is_empty()
            && let Err(error) = self.save_visual_evidence(&self.view.visual_tracks)
            && let Some(screen) = self.view.latest.as_mut().and_then(|c| c.screen.as_mut())
        {
            let note = format!("Could not save image evidence: {error}");
            screen.warning = Some(match screen.warning.take() {
                Some(previous) => format!("{previous} {note}"),
                None => note,
            });
        }
        if let Err(error) = std::fs::write(
            self.directory.join(format!("frame-{number:03}.json")),
            serde_json::to_vec_pretty(&self.view.latest).map_err(|e| e.to_string())?,
        ) && let Some(screen) = self.view.latest.as_mut().and_then(|c| c.screen.as_mut())
        {
            let warning = format!("Could not save observation metadata: {error}");
            screen.warning = Some(match screen.warning.take() {
                Some(previous) => format!("{previous} {warning}"),
                None => warning,
            });
        }
        self.view.phase = format!(
            "Captured {number} moments. {} screen-number tracks. Correlation needs at least 12 varied observations.",
            self.view.tracks.len()
        );
        Ok(())
    }

    pub fn read_frame(&self, investigation: &str, frame_id: &str) -> Result<(FrameRef, Vec<u8>)> {
        if investigation != self.view.investigation_id {
            return Err(
                "That image belongs to a different investigation. Read the current status.".into(),
            );
        }
        let frame = self
            .view
            .frames
            .iter()
            .find(|f| f.id == frame_id)
            .ok_or("The requested image is not part of this investigation.")?;
        let bytes = visual::image_bytes(
            &self
                .directory
                .join(format!("frame-{:03}.png", frame.number)),
        )?;
        if visual::digest(&bytes) != frame.sha256 {
            return Err(
                "The saved image has changed; it can no longer support this observation.".into(),
            );
        }
        Ok((frame.clone(), bytes))
    }

    pub fn submit_observation(&mut self, input: ObservationInput, source: &str) -> Result<bool> {
        if self.refresh_connection() || !self.view.connected {
            return Err(
                "Reconnect before submitting observations to a closed investigation.".into(),
            );
        }
        let (frame, _) = self.read_frame(&input.investigation_id, &input.frame_id)?;
        if input.image_sha256 != frame.sha256 {
            return Err("The observation does not identify the captured image's checksum.".into());
        }
        if frame.epoch != self.image_epoch {
            return Err(
                "The window size changed. Submit observations for the current image geometry."
                    .into(),
            );
        }
        let mut tracks = self.view.visual_tracks.clone();
        let changed = visual::record(&mut tracks, input, &frame, source, self.view.sample_count)?;
        if changed {
            self.save_visual_evidence(&tracks)?;
            self.view.visual_tracks = tracks;
        }
        Ok(changed)
    }

    fn save_visual_evidence(&self, tracks: &[VisualTrack]) -> Result<()> {
        // Archived evidence survives the active engine's restart invalidation.
        // This file is not a journal or a transaction with session.json.
        use std::io::Write;
        let bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version":1,
            "investigation_id":self.view.investigation_id,
            "process":self.view.process,
            "frames":self.view.frames,
            "visual_tracks":tracks,
            "scope":"Historical image interpretations; no current address or editing authority."
        }))
        .map_err(|e| e.to_string())?;
        let temporary = self.directory.join("evidence.pending");
        let result = (|| -> std::io::Result<()> {
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temporary, self.directory.join("evidence.json"))
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result.map_err(|e| format!("Image interpretation was not saved: {e}"))
    }

    pub fn correlate(&mut self) -> Result<()> {
        if self.refresh_connection() || !self.view.connected {
            return Err("The selected process has exited.".into());
        }
        if self.snapshots.len() < 12 {
            return Err("Capture at least 12 moments with changing observations first.".into());
        }
        let mut varying: Vec<(bool, usize, Vec<Option<f64>>)> = self
            .view
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let values: Vec<f64> = t.samples.iter().flatten().copied().collect();
                let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                (t.active && values.len() >= 10 && hi - lo > 2.2)
                    .then(|| (false, i, t.samples.clone()))
            })
            .take(16)
            .collect();
        for (i, track) in self.view.visual_tracks.iter().enumerate() {
            let samples = track.numeric_samples();
            let values: Vec<_> = samples.iter().flatten().copied().collect();
            let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            if track.active && values.len() >= 10 && hi - lo > 2.2 {
                varying.push((true, i, samples));
            }
        }
        // Each source is separately bounded; external interpretations do not
        // replace OCR or silently consume its candidate capacity.
        for track in &mut self.view.visual_tracks {
            track.matches.clear();
        }
        for track in &mut self.view.tracks {
            track.matches.clear();
        }
        if varying.is_empty() {
            return Err("Reversebolt could not follow a changing number across enough captures. Review missing or uncertain readings, or add image-grounded observations, then capture more moments.".into());
        }
        let mut analyzed = 0usize;
        for &(base, length) in &self.plan {
            let Some(trajectory) = self
                .snapshots
                .iter()
                .map(|s| s.get(&base).cloned())
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            if trajectory.iter().any(|s| s.len() != length) {
                continue;
            }
            analyzed += length;
            for (external, index, samples) in &varying {
                let matches = if *external {
                    &mut self.view.visual_tracks[*index].matches
                } else {
                    &mut self.view.tracks[*index].matches
                };
                for candidate in discover(&trajectory, samples) {
                    if matches.len() >= 200 {
                        break;
                    }
                    matches.push(LocatedCandidate {
                        address:format!("0x{:X}",base+candidate.offset as u64),
                        mapping:candidate,
                        evidence:format!("Correlates with {} on held-out frames. Its meaning and causal authority are unverified; this is not an editable control.", if *external { "an externally supplied visual interpretation" } else { "an OCR observation" }),
                    });
                }
            }
        }
        self.view.phase = format!(
            "Compared {} changing tracks across {:.1} MiB present in every frame. {} hypotheses; none causally verified.",
            varying.len(),
            analyzed as f64 / (1024.0 * 1024.0),
            self.view
                .tracks
                .iter()
                .map(|t| t.matches.len())
                .sum::<usize>()
                + self
                    .view
                    .visual_tracks
                    .iter()
                    .map(|t| t.matches.len())
                    .sum::<usize>()
        );
        Ok(())
    }
}

/// Conservative screen-space association, not semantic identification.
/// Keep missing detections as gaps; never replace them with zero.
pub fn track_numbers(tracks: &mut Vec<NumberTrack>, words: &[Word], count: usize) {
    for track in tracks.iter_mut() {
        track.samples.push(None);
        track.readings.push(None);
        track.matches.clear();
    }
    let mut assigned = std::collections::HashSet::new();
    for mut reading in number_readings(words) {
        let possible: Vec<_> = tracks
            .iter()
            .enumerate()
            .filter(|(i, t)| {
                t.active
                    && !assigned.contains(i)
                    && (same_text_region(t.bounds, reading.bounds)
                        || ((t.bounds[0] - reading.bounds[0]).abs() < 24.0
                            && (t.bounds[1] - reading.bounds[1]).abs() < 12.0))
            })
            .map(|(i, _)| i)
            .collect();
        if possible.len() > 1 {
            reading.value = None;
            reading.reason = "ambiguous_track_assignment".into();
            for index in possible {
                tracks[index].readings[count - 1] = Some(reading.clone());
                assigned.insert(index);
            }
        } else if let Some(&index) = possible.first() {
            tracks[index].samples[count - 1] = reading.value;
            tracks[index].bounds = reading.bounds;
            tracks[index].readings[count - 1] = Some(reading);
            assigned.insert(index);
        } else if tracks.len() < 64 {
            let id = tracks.len();
            let mut samples = vec![None; count];
            samples[count - 1] = reading.value;
            let word = &reading.alternatives[0];
            let context = words
                .iter()
                .filter(|w| {
                    w.source == word.source
                        && w.line == word.line
                        && parse_number(&w.text).is_none()
                })
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let label = if context.is_empty() {
                format!(
                    "Number at {}, {}",
                    word.bounds[0] as u32, word.bounds[1] as u32
                )
            } else {
                format!("Near “{}”", context.chars().take(60).collect::<String>())
            };
            let mut readings = vec![None; count];
            let bounds = reading.bounds;
            readings[count - 1] = Some(reading);
            tracks.push(NumberTrack {
                id,
                label,
                bounds,
                samples,
                matches: vec![],
                readings,
                active: true,
            });
            assigned.insert(id);
        }
    }
}

pub fn parse_number(text: &str) -> Option<f64> {
    let text = text.trim().trim_end_matches('%');
    if !text
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
    {
        return None;
    }
    text.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Intersection over the smaller area groups a partial word with its full
/// reading. It never treats neighboring, non-overlapping words as votes.
pub fn same_text_region(a: [f32; 4], b: [f32; 4]) -> bool {
    if !a.iter().chain(&b).all(|x| x.is_finite())
        || a[2] <= 0.0
        || a[3] <= 0.0
        || b[2] <= 0.0
        || b[3] <= 0.0
    {
        return false;
    }
    let overlap = ((a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0])).max(0.0)
        * ((a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1])).max(0.0);
    overlap > 0.5 * (a[2] * a[3]).min(b[2] * b[3])
}

pub fn number_readings(words: &[Word]) -> Vec<NumberReading> {
    let mut remaining: Vec<_> = words
        .iter()
        .filter(|w| parse_number(&w.text).is_some() && same_text_region(w.bounds, w.bounds))
        .take(512)
        .cloned()
        .collect();
    let mut result = vec![];
    while let Some(first) = remaining.pop() {
        let mut group = vec![first];
        loop {
            let before = group.len();
            let mut i = 0;
            while i < remaining.len() {
                if group
                    .iter()
                    .any(|w| same_text_region(w.bounds, remaining[i].bounds))
                {
                    group.push(remaining.remove(i));
                } else {
                    i += 1;
                }
            }
            if before == group.len() {
                break;
            }
        }
        group.sort_by(|a, b| {
            a.bounds[0]
                .total_cmp(&b.bounds[0])
                .then(a.bounds[1].total_cmp(&b.bounds[1]))
                .then(a.text.cmp(&b.text))
        });
        let value = parse_number(&group[0].text).unwrap();
        let conflicting = group.iter().any(|w| parse_number(&w.text) != Some(value));
        // A large box spanning two distinct numeric fields must not merge them,
        // even when both happen to display the same number.
        let ambiguous = group.iter().enumerate().any(|(i, a)| {
            group[i + 1..]
                .iter()
                .any(|b| !same_text_region(a.bounds, b.bounds))
        });
        let left = group
            .iter()
            .map(|w| w.bounds[0])
            .fold(f32::INFINITY, f32::min);
        let top = group
            .iter()
            .map(|w| w.bounds[1])
            .fold(f32::INFINITY, f32::min);
        let right = group
            .iter()
            .map(|w| w.bounds[0] + w.bounds[2])
            .fold(0.0, f32::max);
        let bottom = group
            .iter()
            .map(|w| w.bounds[1] + w.bounds[3])
            .fold(0.0, f32::max);
        result.push(NumberReading {
            bounds: [left, top, right - left, bottom - top],
            value: (!conflicting && !ambiguous).then_some(value),
            reason: if ambiguous {
                "ambiguous_region"
            } else if conflicting {
                "conflicting_readings"
            } else {
                "provisional_reading"
            }
            .into(),
            alternatives: group,
        });
    }
    result.sort_by(|a, b| {
        a.bounds[1]
            .total_cmp(&b.bounds[1])
            .then(a.bounds[0].total_cmp(&b.bounds[0]))
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    struct SampleTarget {
        info: ProcessInfo,
        frame: Cell<usize>,
        alive: Rc<Cell<bool>>,
        recognized: Cell<usize>,
    }
    impl NativeTarget for SampleTarget {
        fn info(&self) -> &ProcessInfo {
            &self.info
        }
        fn alive(&self) -> bool {
            self.alive.get()
        }
        fn regions(&self) -> Result<Vec<Region>> {
            Ok(vec![
                Region {
                    base: 0x10000,
                    length: 128,
                    image: false,
                },
                Region {
                    base: 0x20000,
                    length: 128,
                    image: false,
                },
            ])
        }
        fn screen(&self, path: &Path) -> Result<Screen> {
            self.frame.set(self.frame.get() + 1);
            Ok(Screen {
                path: path.to_string_lossy().into_owned(),
                width: 100,
                height: if self.frame.get() <= 12 { 100 } else { 110 },
                text: String::new(),
                warning: None,
                method: "fixture".into(),
                words: vec![Word {
                    text: (self.frame.get() * 7 + 13).to_string(),
                    bounds: [10.0, 20.0, 30.0, 20.0],
                    line: 0,
                    source: TextSource::WindowsOcr,
                }],
            })
        }
        fn read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
            assert!(
                self.recognized.get() < self.frame.get(),
                "Recognition delayed the memory sample"
            );
            if address == 0x20000 && self.frame.get() == 4 {
                return Err("Page was unmapped during capture".into());
            }
            let mut bytes = vec![0; length];
            let value = (self.frame.get() * 7 + 13) as i32;
            bytes[32..36].copy_from_slice(&value.to_le_bytes());
            Ok(bytes)
        }
        fn recognize(&self, _screen: &mut Screen) -> Result<()> {
            self.recognized.set(self.frame.get());
            Ok(())
        }
    }

    #[test]
    fn visual_evidence_is_bound_to_images_sessions_and_geometry() {
        use crate::visual::{ObservationInput, Quality, VisualValue};
        struct ImageTarget(SampleTarget);
        impl NativeTarget for ImageTarget {
            fn info(&self) -> &ProcessInfo {
                self.0.info()
            }
            fn alive(&self) -> bool {
                self.0.alive()
            }
            fn regions(&self) -> Result<Vec<Region>> {
                self.0.regions()
            }
            fn read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
                self.0.read(address, length)
            }
            fn recognize(&self, frame: &mut Screen) -> Result<()> {
                self.0.recognize(frame)
            }
            fn screen(&self, path: &Path) -> Result<Screen> {
                let screen = self.0.screen(path)?;
                // Opaque fixture bytes exercise binding, not PNG decoding or OCR.
                let mut bytes = b"\x89PNG\r\n\x1a\nfixture".to_vec();
                bytes.extend(self.0.frame.get().to_le_bytes());
                std::fs::write(path, bytes).unwrap();
                Ok(screen)
            }
        }
        let alive = Rc::new(Cell::new(true));
        let directory =
            std::env::temp_dir().join(format!("reversebolt-grounding-{}", uuid::Uuid::new_v4()));
        let mut session = LiveSession::attach(
            Box::new(ImageTarget(SampleTarget {
                info: ProcessInfo::default(),
                frame: Cell::new(0),
                alive: alive.clone(),
                recognized: Cell::new(0),
            })),
            1,
            directory.clone(),
        )
        .unwrap();
        for _ in 0..12 {
            session.capture().unwrap();
        }
        let frames = session.view.frames.clone();
        assert_eq!(frames.len(), 12);
        let input = |f: &FrameRef| ObservationInput {
            investigation_id: session.view.investigation_id.clone(),
            frame_id: f.id.clone(),
            image_sha256: f.sha256.clone(),
            observer: "fixture labels".into(),
            key: "visible-counter".into(),
            label: "Fixture counter".into(),
            unit: "count".into(),
            value: Some(VisualValue::Number((f.number * 7 + 13) as f64)),
            quality: Quality::Provisional,
            bounds: Some([10.0, 20.0, 30.0, 20.0]),
            explanation: "Adapter-supplied fixture label; validates transport and matching only."
                .into(),
        };
        let inputs: Vec<_> = frames.iter().map(input).collect();
        let mut wrong = inputs[0].clone();
        wrong.investigation_id = uuid::Uuid::new_v4().to_string();
        assert!(session.submit_observation(wrong, "agent").is_err());
        let mut wrong = inputs[0].clone();
        wrong.image_sha256 = "incorrect".into();
        assert!(session.submit_observation(wrong, "agent").is_err());
        assert!(session.view.visual_tracks.is_empty());
        let archive = directory.join("evidence.json");
        std::fs::remove_file(&archive).unwrap();
        std::fs::create_dir(&archive).unwrap();
        assert!(
            session
                .submit_observation(inputs[0].clone(), "agent")
                .is_err()
        );
        assert!(
            session.view.visual_tracks.is_empty(),
            "Failed persistence published an observation"
        );
        std::fs::remove_dir(&archive).unwrap();
        // Backfill in reverse order, preserving frame alignment and gaps.
        for mut input in inputs.iter().rev().cloned() {
            if input.frame_id == frames[3].id {
                input.quality = Quality::Uncertain;
            }
            session.submit_observation(input, "agent").unwrap();
        }
        assert!(
            !session
                .submit_observation(inputs[0].clone(), "agent")
                .unwrap()
        );
        assert_eq!(session.view.visual_tracks[0].numeric_samples()[3], None);
        let archived: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&archive).unwrap()).unwrap();
        assert_eq!(archived["frames"].as_array().unwrap().len(), 12);
        assert_eq!(
            archived["visual_tracks"][0]["observations"]
                .as_array()
                .unwrap()
                .len(),
            12
        );
        session.correlate().unwrap();
        assert!(
            session.view.visual_tracks[0]
                .matches
                .iter()
                .any(|m| m.address == "0x10020")
        );
        assert!(
            session.view.visual_tracks[0]
                .matches
                .iter()
                .all(|m| m.evidence.contains("unverified"))
        );
        let file = directory.join("frame-001.png");
        let original = std::fs::read(&file).unwrap();
        std::fs::write(&file, b"\x89PNG\r\n\x1a\naltered").unwrap();
        assert!(
            session
                .read_frame(&session.view.investigation_id, &frames[0].id)
                .is_err()
        );
        assert!(
            session
                .submit_observation(inputs[0].clone(), "agent")
                .is_err()
        );
        std::fs::write(&file, original).unwrap();
        session.capture().unwrap();
        assert!(!session.view.visual_tracks[0].active);
        assert!(session.view.visual_tracks[0].matches.is_empty());
        assert!(
            session
                .submit_observation(inputs[0].clone(), "agent")
                .is_err()
        );
        alive.set(false);
        session.refresh_connection();
        assert!(
            session
                .submit_observation(inputs[0].clone(), "agent")
                .is_err()
        );
        for n in 1..=13 {
            for extension in ["png", "json"] {
                std::fs::remove_file(directory.join(format!("frame-{n:03}.{extension}"))).unwrap();
            }
        }
        std::fs::remove_file(archive).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn correlation_excludes_incomplete_pages_and_exit_revokes_matches() {
        let alive = Rc::new(Cell::new(true));
        let directory =
            std::env::temp_dir().join(format!("reversebolt-test-{}", uuid::Uuid::new_v4()));
        let mut session = LiveSession::attach(
            Box::new(SampleTarget {
                info: ProcessInfo::default(),
                frame: Cell::new(0),
                alive: alive.clone(),
                recognized: Cell::new(0),
            }),
            1,
            directory.clone(),
        )
        .unwrap();
        for _ in 0..12 {
            session.capture().unwrap();
        }
        session.correlate().unwrap();
        let matches = &session.view.tracks[0].matches;
        assert!(matches.iter().any(|m| m.address == "0x10020"));
        assert!(matches.iter().all(|m| !m.address.starts_with("0x2")));
        assert_eq!(session.view.retained_bytes, 12 * 256 - 128);
        session.capture().unwrap();
        assert!(!session.view.tracks[0].active);
        assert!(session.view.tracks[0].matches.is_empty());
        assert_eq!(session.view.tracks[0].samples.last(), Some(&None));
        assert!(session.view.tracks[1].active);
        assert!(
            session.view.tracks[1].samples[..12]
                .iter()
                .all(Option::is_none)
        );
        alive.set(false);
        assert!(session.refresh_connection());
        assert!(!session.view.connected);
        assert!(session.view.tracks[0].matches.is_empty());
        assert!(session.capture().is_err());
        assert!(!session.refresh_connection());
        for number in 1..=13 {
            std::fs::remove_file(directory.join(format!("frame-{number:03}.json"))).unwrap();
        }
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn missing_ocr_is_a_gap_and_adjacent_numbers_keep_separate_tracks() {
        let word = |text: &str, x| Word {
            text: text.into(),
            bounds: [x, 20.0, 30.0, 15.0],
            line: 0,
            source: TextSource::WindowsOcr,
        };
        let mut tracks = vec![];
        track_numbers(&mut tracks, &[word("90", 10.0), word("30", 70.0)], 1);
        track_numbers(&mut tracks, &[word("29", 71.0)], 2);
        track_numbers(&mut tracks, &[word("85", 11.0), word("28", 70.0)], 3);
        assert_eq!(tracks[0].samples, vec![Some(90.0), None, Some(85.0)]);
        assert_eq!(tracks[1].samples, vec![Some(30.0), Some(29.0), Some(28.0)]);
    }

    fn reading_word(text: &str, bounds: [f32; 4], source: TextSource) -> Word {
        Word {
            text: text.into(),
            bounds,
            line: 0,
            source,
        }
    }

    #[test]
    fn conflicting_partial_readings_abstain_regardless_of_order_or_vote_count() {
        let full = reading_word("17", [100.0, 50.0, 40.0, 30.0], TextSource::WindowsOcr);
        let partial = reading_word("7", [122.0, 51.0, 15.0, 28.0], TextSource::Ocrs);
        let mut words = vec![full.clone(), full, partial];
        for _ in 0..2 {
            let readings = number_readings(&words);
            assert_eq!(readings.len(), 1);
            assert_eq!(readings[0].value, None);
            assert_eq!(readings[0].reason, "conflicting_readings");
            assert_eq!(readings[0].alternatives.len(), 3);
            words.reverse();
        }
        let mut tracks = vec![];
        track_numbers(&mut tracks, &words, 1);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].samples, vec![None]);
        assert!(tracks[0].readings[0].is_some());
    }

    #[test]
    fn overlapping_agreement_is_one_reading_but_spanning_fields_are_ambiguous() {
        let a = reading_word("10", [0.0, 0.0, 20.0, 20.0], TextSource::WindowsOcr);
        let b = reading_word("10", [40.0, 0.0, 20.0, 20.0], TextSource::WindowsOcr);
        let duplicate = reading_word("10", [1.0, 1.0, 20.0, 20.0], TextSource::Ocrs);
        let readings = number_readings(&[a.clone(), duplicate]);
        assert_eq!(readings.len(), 1);
        assert_eq!(readings[0].value, Some(10.0));
        let bridge = reading_word("10", [0.0, 0.0, 60.0, 20.0], TextSource::OcrsRegion);
        let readings = number_readings(&[a, b, bridge]);
        assert_eq!(readings.len(), 1);
        assert_eq!(readings[0].value, None);
        assert_eq!(readings[0].reason, "ambiguous_region");
    }

    #[test]
    fn letters_nonfinite_and_invalid_geometry_cannot_become_numbers() {
        for text in ["IOO", "BOSS", "NaN", "inf", "1e3", "", "1/2"] {
            assert!(parse_number(text).is_none());
        }
        assert_eq!(parse_number(" -12.5% "), Some(-12.5));
        let word = reading_word("100", [f32::NAN, 0.0, 10.0, 10.0], TextSource::Ocrs);
        assert!(number_readings(&[word]).is_empty());
    }
}
