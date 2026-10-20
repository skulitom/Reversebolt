//! Observation and memory capture from unmodified external programs.
//! No executable-name rules, game offsets or source-derived layouts.
use crate::{
    Result,
    discovery::{Candidate, discover},
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Screen {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub text: String,
    pub words: Vec<Word>,
    pub warning: Option<String>,
}

pub trait NativeTarget {
    fn info(&self) -> &ProcessInfo;
    fn regions(&self) -> Result<Vec<Region>>;
    fn read(&self, address: u64, length: usize) -> Result<Vec<u8>>;
    fn screen(&self, path: &Path) -> Result<Screen>;
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaptureSummary {
    pub number: usize,
    pub started_unix_ms: u128,
    pub elapsed_ms: u128,
    pub memory_read_ms: u128,
    pub captured_bytes: usize,
    pub unreadable_bytes: usize,
    pub screen: Option<Screen>,
    pub screen_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveView {
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
        })
    }

    pub fn capture(&mut self) -> Result<()> {
        if !self.target.alive() {
            self.view.connected = false;
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
        let screen = self.target.screen(&screen_path);
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
        self.snapshots.push(snapshot);
        if let Ok(frame) = &screen {
            track_numbers(&mut self.view.tracks, &frame.words, number);
        } else {
            for track in &mut self.view.tracks {
                track.samples.push(None);
                track.matches.clear();
            }
        }
        self.view.sample_count = number;
        self.view.retained_bytes += captured;
        self.view.latest = Some(CaptureSummary {
            number,
            started_unix_ms: now,
            elapsed_ms: start.elapsed().as_millis(),
            memory_read_ms,
            captured_bytes: captured,
            unreadable_bytes: unreadable,
            screen_error: screen.as_ref().err().cloned(),
            screen: screen.ok(),
        });
        self.view.phase = format!(
            "Captured {number} moments. {} screen-number tracks. Correlation needs at least 12 varied observations.",
            self.view.tracks.len()
        );
        Ok(())
    }

    pub fn correlate(&mut self) -> Result<()> {
        if !self.target.alive() {
            self.view.connected = false;
            return Err("The selected process has exited.".into());
        }
        if self.snapshots.len() < 12 {
            return Err("Capture at least 12 moments with changing observations first.".into());
        }
        let varying: Vec<usize> = self
            .view
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let values: Vec<f64> = t.samples.iter().flatten().copied().collect();
                let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                (values.len() >= 10 && hi - lo > 2.2).then_some(i)
            })
            .take(16)
            .collect();
        for track in &mut self.view.tracks {
            track.matches.clear();
        }
        if varying.is_empty() {
            return Err("No screen-number track has enough changing observations. Try an activity that visibly changes a number.".into());
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
            for &index in &varying {
                let track = &mut self.view.tracks[index];
                for candidate in discover(&trajectory, &track.samples) {
                    if track.matches.len() >= 200 {
                        break;
                    }
                    track.matches.push(LocatedCandidate {
                        address:format!("0x{:X}",base+candidate.offset as u64),
                        mapping:candidate,
                        evidence:"Correlates with an OCR observation on held-out frames. No causal test has been performed; do not treat this as a verified control.".into(),
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
        );
        Ok(())
    }
}

/// Conservative screen-space association, not semantic identification.
/// Keep missing detections as gaps; never replace them with zero.
fn track_numbers(tracks: &mut Vec<NumberTrack>, words: &[Word], count: usize) {
    for track in tracks.iter_mut() {
        track.samples.push(None);
        track.matches.clear();
    }
    let mut assigned = std::collections::HashSet::new();
    for word in words {
        let text = word.text.trim().trim_end_matches('%');
        if !text
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
        {
            continue;
        }
        let Ok(value) = text.parse::<f64>() else {
            continue;
        };
        if !value.is_finite() {
            continue;
        }
        let closest = tracks
            .iter()
            .enumerate()
            .filter(|(i, t)| {
                !assigned.contains(i)
                    && (t.bounds[0] - word.bounds[0]).abs() < 24.0
                    && (t.bounds[1] - word.bounds[1]).abs() < 12.0
            })
            .min_by(|(_, a), (_, b)| {
                (a.bounds[0] - word.bounds[0])
                    .abs()
                    .total_cmp(&(b.bounds[0] - word.bounds[0]).abs())
            })
            .map(|(i, _)| i);
        if let Some(index) = closest {
            tracks[index].samples[count - 1] = Some(value);
            tracks[index].bounds = word.bounds;
            assigned.insert(index);
        } else if tracks.len() < 64 {
            let id = tracks.len();
            let mut samples = vec![None; count];
            samples[count - 1] = Some(value);
            let context = words
                .iter()
                .filter(|w| w.line == word.line && w.text.parse::<f64>().is_err())
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
            tracks.push(NumberTrack {
                id,
                label,
                bounds: word.bounds,
                samples,
                matches: vec![],
            });
            assigned.insert(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    struct SampleTarget {
        info: ProcessInfo,
        frame: Cell<usize>,
        alive: Rc<Cell<bool>>,
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
                height: 100,
                text: String::new(),
                warning: None,
                words: vec![Word {
                    text: (self.frame.get() * 7 + 13).to_string(),
                    bounds: [10.0, 20.0, 30.0, 20.0],
                    line: 0,
                }],
            })
        }
        fn read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
            if address == 0x20000 && self.frame.get() == 4 {
                return Err("Page was unmapped during capture".into());
            }
            let mut bytes = vec![0; length];
            let value = (self.frame.get() * 7 + 13) as i32;
            bytes[32..36].copy_from_slice(&value.to_le_bytes());
            Ok(bytes)
        }
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
        alive.set(false);
        assert!(session.refresh_connection());
        assert!(!session.view.connected);
        assert!(session.view.tracks[0].matches.is_empty());
        assert!(session.capture().is_err());
        assert!(!session.refresh_connection());
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn missing_ocr_is_a_gap_and_adjacent_numbers_keep_separate_tracks() {
        let word = |text: &str, x| Word {
            text: text.into(),
            bounds: [x, 20.0, 30.0, 15.0],
            line: 0,
        };
        let mut tracks = vec![];
        track_numbers(&mut tracks, &[word("90", 10.0), word("30", 70.0)], 1);
        track_numbers(&mut tracks, &[word("29", 71.0)], 2);
        track_numbers(&mut tracks, &[word("85", 11.0), word("28", 70.0)], 3);
        assert_eq!(tracks[0].samples, vec![Some(90.0), None, Some(85.0)]);
        assert_eq!(tracks[1].samples, vec![Some(30.0), Some(29.0), Some(28.0)]);
    }
}
