use crate::{
    Command, Result,
    discovery::{Candidate, discover},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

/// The engine sees observation tracks and a bounded byte arena, never field names
/// or authoritative offsets from a target's implementation.
pub trait PracticeTarget {
    fn pid(&self) -> u32;
    fn read(&self) -> Result<Vec<u8>>;
    fn write(&self, offset: usize, bytes: &[u8]) -> Result<()>;
    fn sample(&mut self, advance: bool) -> Result<Vec<f64>>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Match {
    pub candidate: Candidate,
    pub tested: bool,
    pub passed: bool,
    pub evidence: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub label: String,
    pub value: f64,
    pub samples: Vec<Option<f64>>,
    pub matches: Vec<Match>,
}

impl Track {
    pub fn verified(&self) -> Option<&Candidate> {
        let mut matches = self.matches.iter().filter(|m| m.tested && m.passed);
        let first = matches.next()?;
        matches.next().is_none().then_some(&first.candidate)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub time: u64,
    pub source: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Patch {
    pub session: String,
    pub track: usize,
    pub offset: usize,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
    pub before_value: f64,
    pub after_value: f64,
    pub undone: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct View {
    pub schema_version: u32,
    pub session: String,
    pub connected: bool,
    pub pid: Option<u32>,
    pub origin: String,
    pub phase: String,
    pub tracks: Vec<Track>,
    pub history: Vec<Event>,
    pub patches: Vec<Patch>,
    pub error: Option<String>,
}

impl Default for View {
    fn default() -> Self {
        Self {
            schema_version: 1,
            session: String::new(),
            connected: false,
            pid: None,
            origin: "Practice environment · structured numeric observations".into(),
            phase: "Open the practice environment to begin.".into(),
            tracks: vec![],
            history: vec![],
            patches: vec![],
            error: None,
        }
    }
}

pub struct Engine {
    pub view: View,
    pub target: Option<Box<dyn PracticeTarget>>,
    snapshots: Vec<Vec<u8>>,
    path: PathBuf,
}

impl Engine {
    pub fn load(path: PathBuf) -> Result<Self> {
        let mut view: View = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Could not read session history: {e}"))?
        } else {
            View::default()
        };
        if view.schema_version != 1 {
            return Err("This session format is unsupported.".into());
        }
        // Never resurrect process addresses from a previous engine lifetime.
        view.connected = false;
        view.pid = None;
        view.tracks.clear();
        view.error = None;
        view.phase =
            "Open the practice environment to begin. Previous history is preserved.".into();
        Ok(Self {
            view,
            target: None,
            snapshots: vec![],
            path,
        })
    }

    pub fn save(&self) -> Result<()> {
        let temporary = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(&self.view).map_err(|e| e.to_string())?;
        use std::io::Write;
        let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&temporary, &self.path).map_err(|e| e.to_string())
    }

    pub fn event(&mut self, source: &str, text: impl Into<String>) {
        self.view.history.push(Event {
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            source: source.chars().take(40).collect(),
            text: text.into(),
        });
        if self.view.history.len() > 500 {
            self.view.history.remove(0);
        }
    }

    pub fn open(&mut self, mut target: Box<dyn PracticeTarget>, source: &str) -> Result<()> {
        let values = target.sample(false)?;
        if values.len() != 2 || values.iter().any(|v| !v.is_finite()) {
            return Err("Practice observation protocol mismatch.".into());
        }
        self.view.session = uuid::Uuid::new_v4().to_string();
        self.view.connected = true;
        self.view.pid = Some(target.pid());
        self.view.tracks = ["Health", "Ammunition"]
            .into_iter()
            .zip(values)
            .map(|(label, value)| Track {
                label: label.into(),
                value,
                samples: vec![],
                matches: vec![],
            })
            .collect();
        self.snapshots.clear();
        self.target = Some(target);
        self.view.phase = "Ready to observe. The practice adapter supplies the two labels.".into();
        self.event(
            source,
            "Opened a new practice process with a randomized layout.",
        );
        Ok(())
    }

    pub fn execute(&mut self, command: &Command) -> Result<Value> {
        if matches!(command.action.as_str(), "status" | "history") {
            return Ok(json!(self.view));
        }
        self.view.error = None;
        match command.action.as_str() {
            "status" | "history" => return Ok(json!(self.view)),
            "close_target" => {
                self.target = None;
                self.view.connected = false;
                self.view.pid = None;
                self.view.tracks.clear();
                self.snapshots.clear();
                self.view.phase = "Practice closed. History retained.".into();
                self.event(&command.source, "Closed the practice process.");
            }
            "observe" => self.observe(&command.source)?,
            "discover" => self.discover(&command.source)?,
            "test_candidates" => self.test_candidates(&command.source)?,
            "set_value" => {
                let track = command.args["track"].as_u64().ok_or("Choose a track.")? as usize;
                let value = command.args["value"]
                    .as_f64()
                    .ok_or("Enter a numeric value.")?;
                self.set_value(track, value, &command.source)?;
            }
            "undo" => self.undo(&command.source)?,
            _ => return Err(format!("Unsupported action: {}", command.action)),
        }
        Ok(json!(self.view))
    }

    fn observe(&mut self, source: &str) -> Result<()> {
        let target = self
            .target
            .as_mut()
            .ok_or("Open a practice environment first.")?;
        self.snapshots.clear();
        for track in &mut self.view.tracks {
            track.samples.clear();
            track.matches.clear();
        }
        for _ in 0..20 {
            let values = target.sample(true)?;
            if values.len() != self.view.tracks.len() {
                return Err("Observation track count changed.".into());
            }
            self.snapshots.push(target.read()?);
            for (track, value) in self.view.tracks.iter_mut().zip(values) {
                track.samples.push(value.is_finite().then_some(value));
                track.value = value;
            }
        }
        self.view.phase = "20 synchronized samples captured. Ready to find matches.".into();
        self.event(
            source,
            "Observed 20 state changes and read 20 process-memory snapshots (256 KiB each).",
        );
        Ok(())
    }

    fn discover(&mut self, source: &str) -> Result<()> {
        if self.snapshots.len() < 12 {
            return Err("Observe some changing values first.".into());
        }
        for track in &mut self.view.tracks {
            track.matches = discover(&self.snapshots, &track.samples)
                .into_iter()
                .map(|candidate| Match {
                    candidate,
                    tested: false,
                    passed: false,
                    evidence:
                        "Matches observations in held-out samples. Control is not yet established."
                            .into(),
                })
                .collect();
        }
        self.view.phase = "Matches found. Test their effects before editing.".into();
        self.event(
            source,
            format!(
                "Found {} numeric matches. These include display copies.",
                self.view
                    .tracks
                    .iter()
                    .map(|t| t.matches.len())
                    .sum::<usize>()
            ),
        );
        Ok(())
    }

    fn test_candidates(&mut self, source: &str) -> Result<()> {
        if self.view.tracks.iter().all(|t| t.matches.is_empty()) {
            return Err("Find matches before testing them.".into());
        }
        let target = self.target.as_mut().ok_or("No active practice process.")?;
        let before = target.sample(false)?;
        let baseline = target.read()?;
        // Whole-arena rollback is valid only for this paused, disposable fixture.
        // This experiment strategy must not be exposed to arbitrary live games.
        let experiment = (|| -> Result<()> {
            for (channel, track) in self.view.tracks.iter_mut().enumerate() {
                for item in &mut track.matches {
                    let mut passed = true;
                    for delta in [3.0, -4.0] {
                        target.write(0, &baseline)?;
                        if let Some(bytes) =
                            item.candidate.encode_observation(before[channel] + delta)
                        {
                            target.write(item.candidate.offset, &bytes)?;
                            target.sample(false)?;
                            let after = target.sample(false)?;
                            passed &= after.len() == before.len()
                                && after.iter().enumerate().all(|(i, v)| {
                                    (*v - before[i] - if i == channel { delta } else { 0.0 }).abs()
                                        <= 0.6
                                });
                        } else {
                            passed = false;
                        }
                    }
                    item.tested = true;
                    item.passed = passed;
                    item.evidence = if passed {
                        "Both +3 and -4 interventions survived two simulation ticks; the other track stayed unchanged."
                    } else {
                        "The requested effect did not survive, or another observed track changed."
                    }.into();
                }
            }
            Ok(())
        })();
        let restore = target
            .write(0, &baseline)
            .and_then(|_| target.sample(false));
        match restore {
            Ok(restored)
                if restored
                    .iter()
                    .zip(&before)
                    .all(|(a, b)| (a - b).abs() <= 0.6) => {}
            _ => {
                self.view.connected = false;
                self.target = None;
                return Err("Practice rollback failed; the disposable target was closed.".into());
            }
        }
        experiment?;
        for (track, value) in self.view.tracks.iter_mut().zip(before) {
            track.value = value;
        }
        let verified = self
            .view
            .tracks
            .iter()
            .filter(|t| t.verified().is_some())
            .count();
        self.view.phase = format!(
            "{verified} uniquely supported controls. Practice state restored after testing."
        );
        self.event(source, self.view.phase.clone());
        Ok(())
    }

    fn set_value(&mut self, channel: usize, desired: f64, source: &str) -> Result<()> {
        if !desired.is_finite() || desired.abs() > 1.0e6 {
            return Err(
                "This preview accepts finite values between -1,000,000 and 1,000,000.".into(),
            );
        }
        let track = self
            .view
            .tracks
            .get(channel)
            .ok_or("Unknown observation track.")?;
        let candidate = track
            .verified()
            .cloned()
            .ok_or("A single tested control is required before editing.")?;
        let encoded = candidate
            .encode_observation(desired)
            .ok_or("That value cannot be represented by this control.")?;
        let target = self.target.as_mut().ok_or("No active practice process.")?;
        let before = target.sample(false)?;
        let baseline = target.read()?;
        let saved = baseline
            .get(candidate.offset..candidate.offset + encoded.len())
            .ok_or("The learned location is outside the current snapshot.")?
            .to_vec();
        let result = (|| -> Result<Vec<f64>> {
            target.write(candidate.offset, &encoded)?;
            target.sample(false)?;
            let after = target.sample(false)?;
            if after.len() != before.len()
                || after
                    .iter()
                    .enumerate()
                    .any(|(i, v)| (*v - if i == channel { desired } else { before[i] }).abs() > 0.6)
            {
                return Err("The observed effect did not match the requested edit.".into());
            }
            Ok(after)
        })();
        let after = match result {
            Ok(values) => values,
            Err(error) => {
                if target
                    .write(0, &baseline)
                    .and_then(|_| target.sample(false))
                    .is_err()
                {
                    self.target = None;
                    self.view.connected = false;
                    return Err(format!("{error} Rollback failed; practice was closed."));
                }
                return Err(format!("{error} The practice state was restored."));
            }
        };
        self.view.patches.push(Patch {
            session: self.view.session.clone(),
            track: channel,
            offset: candidate.offset,
            before: saved,
            after: encoded,
            before_value: before[channel],
            after_value: after[channel],
            undone: false,
        });
        for (track, value) in self.view.tracks.iter_mut().zip(after) {
            track.value = value;
        }
        self.event(
            source,
            format!(
                "Changed {} from {} to {}; observed effect verified.",
                self.view.tracks[channel].label, before[channel], self.view.tracks[channel].value
            ),
        );
        self.view.phase = "Edit verified in the practice process. Restore is available.".into();
        Ok(())
    }

    fn undo(&mut self, source: &str) -> Result<()> {
        let index = self
            .view
            .patches
            .iter()
            .rposition(|p| !p.undone && p.session == self.view.session)
            .ok_or("There is no edit to restore in this process.")?;
        let patch = self.view.patches[index].clone();
        let target = self
            .target
            .as_mut()
            .ok_or("The original process is no longer active.")?;
        let bytes = target.read()?;
        if bytes.get(patch.offset..patch.offset + patch.after.len()) != Some(patch.after.as_slice())
        {
            return Err("The value changed since this edit. Restore was skipped to avoid overwriting newer state.".into());
        }
        target.write(patch.offset, &patch.before)?;
        target.sample(false)?;
        let values = target.sample(false)?;
        if (values[patch.track] - patch.before_value).abs() > 0.6 {
            return Err(
                "The previous bytes were restored, but the expected effect was not observed."
                    .into(),
            );
        }
        self.view.patches[index].undone = true;
        for (track, value) in self.view.tracks.iter_mut().zip(values) {
            track.value = value;
        }
        self.view.phase = "Previous value restored and verified.".into();
        self.event(
            source,
            format!(
                "Restored {} to {}.",
                self.view.tracks[patch.track].label, patch.before_value
            ),
        );
        Ok(())
    }
}
