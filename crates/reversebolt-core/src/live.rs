//! Observation and memory capture from unmodified external programs.
//! No executable-name rules, game offsets or source-derived layouts.
use crate::{
    Result,
    coverage::{self, BatchSummary, Chunk, Coverage, Ranges},
    discovery::{Candidate, discover},
    prediction::{self, FrozenCandidate, PredictionCheck},
    temporal,
    visual::{self, CaptureInterval, FrameRef, ObservationInput, VisualTrack},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

const MAX_SAMPLES: usize = 20;
const RETAINED_MEMORY_BUDGET: usize = 512 * 1024 * 1024;
type PredictionReads = BTreeMap<(u64, usize), prediction::Witness>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    #[default]
    Full,
    Visual,
}

impl CaptureMode {
    pub fn is_full(&self) -> bool {
        *self == Self::Full
    }
}

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

pub use crate::coverage::Region;

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
    #[serde(rename = "pp_ocr_v6")]
    PpOcrV6,
    #[serde(rename = "pp_ocr_v6_region")]
    PpOcrV6Region,
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
    fn start_inputs(
        &self,
        _clock_id: &str,
        _origin: Instant,
        _cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<()> {
        Ok(())
    }
    fn input_window(&self, _begin_us: u64, _end_us: u64) -> Option<crate::inputs::Window> {
        None
    }
    fn info(&self) -> &ProcessInfo;
    /// The fixed address space for every region, read and write in this target.
    /// It can differ from the process whose window supplies image/input evidence.
    fn memory_info(&self) -> &ProcessInfo {
        self.info()
    }
    fn memory_sources(&self) -> Result<Vec<ProcessInfo>> {
        Ok(vec![self.info().clone()])
    }
    fn regions(&self) -> Result<Vec<Region>>;
    fn query(&self, address: u64) -> Result<Option<Region>> {
        Ok(self.regions()?.into_iter().find(|r| {
            r.base <= address
                && r.base
                    .checked_add(r.length as u64)
                    .is_some_and(|end| address < end)
        }))
    }
    fn read(&self, address: u64, length: usize) -> Result<Vec<u8>>;
    /// Best-effort expected-byte guard; never an atomic compare-and-swap.
    fn guarded_write(
        &self,
        _patch: &crate::changes::BytePatch,
        _region: &Region,
    ) -> crate::changes::WriteReceipt {
        crate::changes::WriteReceipt::refused("This target does not support trial writes.")
    }
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
    /// Visual mode deliberately omits OCR; image and memory evidence still exist.
    #[serde(default)]
    pub mode: CaptureMode,
    pub number: usize,
    pub started_unix_ms: u128,
    /// Wall-clock start of the image request, after any candidate prefix reads.
    /// Historical records used the overall capture start instead.
    #[serde(default)]
    pub screen_started_unix_ms: Option<u128>,
    pub elapsed_ms: u128,
    pub memory_read_ms: u128,
    #[serde(default)]
    pub screen_capture_ms: u128,
    #[serde(default)]
    pub recognition_ms: u128,
    pub captured_bytes: usize,
    pub unreadable_bytes: usize,
    #[serde(default)]
    pub changed_mapping_bytes: usize,
    #[serde(default)]
    pub memory_reads: Vec<MemoryRead>,
    /// Small candidate reads before the image, charged to the same read budget.
    #[serde(default)]
    pub prediction_reads: Vec<MemoryRead>,
    pub screen: Option<Screen>,
    pub screen_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryRead {
    pub address: String,
    pub requested: usize,
    pub retained: usize,
    /// Milliseconds from this capture's start; screen and memory are sequential.
    pub begin_ms: u128,
    pub end_ms: u128,
    /// Same investigation-local monotonic clock as image capture bounds.
    /// Includes mapping queries; does not make a read atomic with the image.
    #[serde(default)]
    pub capture_interval: Option<CaptureInterval>,
    pub disposition: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalObservationWarning {
    pub evidence_id: String,
    pub job: u64,
    pub frame_ids: Vec<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveView {
    /// None in historical records means the selected window process.
    #[serde(default)]
    pub memory_process: Option<ProcessInfo>,
    #[serde(default)]
    pub memory_sources: Vec<ProcessInfo>,
    #[serde(default)]
    pub memory_sources_error: Option<String>,
    /// Bounded notices for this batch. Optional image interpretation can fail
    /// while independent image, text and memory collection remains available.
    #[serde(default)]
    pub local_observation_warnings: Vec<LocalObservationWarning>,
    #[serde(default)]
    pub rule_checks: Vec<crate::rule_checks::Check>,
    #[serde(default)]
    pub rule_error: Option<String>,
    #[serde(default)]
    pub action_patterns: Vec<crate::action_patterns::Evidence>,
    #[serde(default)]
    pub prediction_check: Option<PredictionCheck>,
    #[serde(default)]
    pub coverage: Coverage,
    #[serde(default)]
    pub investigation_id: String,
    #[serde(default)]
    pub frames: Vec<FrameRef>,
    #[serde(default)]
    pub visual_tracks: Vec<VisualTrack>,
    #[serde(default)]
    pub temporal_tracks: Vec<temporal::Track>,
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
    plan: Vec<Chunk>,
    budget_bytes: usize,
    selected: Ranges,
    read_ranges: Ranges,
    complete_ranges: Ranges,
    invalid_chunks: BTreeSet<u64>,
    root_directory: PathBuf,
    snapshots: Vec<BTreeMap<u64, Vec<u8>>>,
    receipts: Vec<Vec<MemoryRead>>,
    directory: PathBuf,
    screen_size: Option<(u32, u32)>,
    image_epoch: usize,
    capture_clock: Instant,
}

impl LiveSession {
    pub fn refresh_memory_sources(&mut self) {
        match self.target.memory_sources() {
            Ok(sources) => {
                self.view.memory_sources = sources;
                self.view.memory_sources_error = None;
            }
            Err(error) => {
                self.view.memory_sources.clear();
                self.view.memory_sources_error = Some(error);
            }
        }
    }
    pub fn start_inputs(
        &self,
        cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<()> {
        self.target
            .start_inputs(&self.view.investigation_id, self.capture_clock, cancel)
    }
    pub fn change_scope(&self, track: usize) -> Result<crate::changes::Scope> {
        let check = self
            .view
            .prediction_check
            .as_ref()
            .ok_or("Check this value against fresh observations first.")?;
        let series = self
            .view
            .tracks
            .iter()
            .find(|t| t.id == track && t.active)
            .ok_or("That screen value is not active.")?;
        let (width, height) = self
            .screen_size
            .ok_or("The window geometry is unavailable.")?;
        Ok(crate::changes::Scope {
            investigation: self.view.investigation_id.clone(),
            batch: self.view.coverage.batch,
            epoch: self.image_epoch,
            frame: self.view.sample_count,
            prediction: check.id.clone(),
            process: self.target.memory_info().clone(),
            track,
            label: series.label.clone(),
            bounds: series.bounds,
            image_size: [width, height],
        })
    }

    pub fn prepare_change(
        &mut self,
        track: usize,
        desired: f64,
        actor: &str,
        store: &mut crate::changes::Store,
    ) -> Result<crate::changes::Change> {
        self.prepare_change_inner(track, desired, actor, store, None)
    }

    pub fn prepare_probe(
        &mut self,
        track: usize,
        desired: f64,
        selection: (&str, usize),
        actor: &str,
        store: &mut crate::changes::Store,
    ) -> Result<crate::changes::Change> {
        self.prepare_change_inner(track, desired, actor, store, Some(selection))
    }

    fn prepare_change_inner(
        &mut self,
        track: usize,
        desired: f64,
        actor: &str,
        store: &mut crate::changes::Store,
        probe: Option<(&str, usize)>,
    ) -> Result<crate::changes::Change> {
        if self.refresh_connection() || !self.view.connected {
            return Err("The original application is no longer connected.".into());
        }
        let scope = self.change_scope(track)?;
        let check = self.view.prediction_check.as_ref().unwrap();
        if check.source != prediction::Source::Ocr
            || check.track != track
            || check.invalidated.is_some()
            || check.epoch != self.image_epoch
        {
            return Err("This value needs a current screen-number prediction check before a trial can be prepared.".into());
        }
        let group = if let Some((id, selected)) = probe {
            if id != check.id {
                return Err(
                    "The prediction check changed. Review the current possibilities.".into(),
                );
            }
            Some(crate::probes::select(check, selected)?)
        } else {
            None
        };
        // Ordinary edits require consensus across every remaining explanation.
        // An explicit location test keeps all overlapping rivals in its vote;
        // other locations are retained below as competing predictions.
        let survivors: Vec<_> = check
            .candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| c.assessment(check) != prediction::Assessment::Contradicted)
            .filter(|(i, _)| group.as_ref().is_none_or(|g| g.contains(i)))
            .collect();
        if survivors.is_empty()
            || survivors.iter().any(|(_, c)| {
                c.assessment(check) != prediction::Assessment::ConsistentInObservedScope
            })
        {
            return Err("Some interpretations still need contrasting evidence. Keep observing before preparing an edit.".into());
        }
        let observed = self
            .view
            .tracks
            .iter()
            .find(|t| t.id == track)
            .and_then(|t| t.samples.last().copied().flatten())
            .ok_or("The latest screen value is uncertain.")?;
        let mut models = vec![];
        for (index, c) in survivors {
            let address = u64::from_str_radix(c.address.trim_start_matches("0x"), 16)
                .map_err(|_| "Invalid candidate address.")?;
            let chunk = self
                .plan
                .iter()
                .find(|p| p.address == c.chunk_address)
                .ok_or("The candidate is outside the current batch.")?;
            if self.invalid_chunks.contains(&chunk.address) {
                return Err("A candidate allocation is no longer available.".into());
            }
            let region = self
                .target
                .query(address)?
                .ok_or("A candidate allocation disappeared.")?;
            if !chunk.covered_by(
                &self
                    .target
                    .query(chunk.address)?
                    .ok_or("A candidate allocation disappeared.")?,
            ) {
                return Err("A candidate allocation changed.".into());
            }
            let before = self.target.read(address, c.mapping.kind.width())?;
            models.push(crate::changes::Model {
                index,
                address,
                mapping: c.mapping.clone(),
                region,
                before,
            });
        }
        let mut change = crate::changes::Change::prepare(scope, models, observed, desired, actor)?;
        if let Some((_, selected)) = probe {
            let group = group.unwrap();
            let alternatives = check
                .candidates
                .iter()
                .enumerate()
                .map(|(index, c)| {
                    let address = c.chunk_address + c.offset_in_chunk as u64;
                    let bytes = if let Some(model) = change.models.iter().find(|m| m.index == index)
                    {
                        Ok(model.before.clone())
                    } else {
                        (|| -> Result<Vec<u8>> {
                            let chunk = self
                                .plan
                                .iter()
                                .find(|p| p.address == c.chunk_address)
                                .ok_or("The rival is outside the memory batch.")?;
                            if self.invalid_chunks.contains(&chunk.address)
                                || !self
                                    .target
                                    .query(chunk.address)?
                                    .is_some_and(|r| chunk.covered_by(&r))
                            {
                                return Err("The rival's memory mapping is unavailable.".into());
                            }
                            let bytes = self.target.read(address, c.mapping.kind.width())?;
                            if bytes.len() != c.mapping.kind.width() {
                                return Err("Incomplete rival bytes.".into());
                            }
                            Ok(bytes)
                        })()
                    };
                    let predicted_if_written = bytes.as_ref().ok().and_then(|b| {
                        change
                            .patch
                            .as_ref()
                            .and_then(|p| crate::probes::prediction(&c.mapping, address, b, p))
                    });
                    let (before, error) = match bytes {
                        Ok(b) => (Some(b), None),
                        Err(e) => (None, Some(e)),
                    };
                    crate::probes::Alternative {
                        index,
                        address,
                        mapping: c.mapping.clone(),
                        assessment: c.assessment(check),
                        in_test_group: group.contains(&index),
                        before,
                        predicted_if_written,
                        error,
                    }
                })
                .collect();
            change.probe = Some(crate::probes::Probe { selected, group, alternatives,
                limitation: "Tests one possible location. Other locations remain rival explanations. Predictions assume these byte mappings and unchanged other bytes; reads are sequential. A visible response is not proof of gameplay authority, and restoring bytes cannot undo all effects.".into() });
            if change.phase == crate::changes::Phase::Prepared {
                change.reason = "Overlapping interpretations agree on this location test. Other locations remain competing explanations. Review the predicted outcomes before explicitly trying it.".into();
            }
        }
        if change.phase == crate::changes::Phase::Prepared {
            let mut tracks: Vec<_> = self.view.tracks.iter().filter(|t| t.active).collect();
            tracks.sort_by_key(|t| (t.id != track, t.id));
            change.omitted_tracks = tracks.len().saturating_sub(crate::effects::MAX_WATCHES);
            change.watches = tracks
                .into_iter()
                .take(crate::effects::MAX_WATCHES)
                .map(|t| crate::effects::Watch {
                    track: t.id,
                    label: t.label.clone(),
                    bounds: t.bounds,
                })
                .collect();
            let baseline = self.capture_change(
                &change,
                store,
                crate::changes::ObservationStage::BeforeEdit,
                observed,
            )?;
            if !baseline.agrees || baseline.error.is_some() {
                change.phase = crate::changes::Phase::Unavailable;
                change.reason = "The fresh screen reading does not support this trial. Keep observing before preparing again.".into();
            }
            change.baseline = Some(baseline);
        }
        store.insert(change)
    }

    pub fn execute_change(
        &mut self,
        id: &str,
        restore: bool,
        store: &mut crate::changes::Store,
    ) -> Result<crate::changes::Change> {
        self.refresh_connection();
        let change = store.get(id)?;
        let scope = self.change_scope(change.scope.track)?;
        let result = store.execute(id, &*self.target, &scope, restore)?;
        // Keep the frozen record for provenance, but do not treat observations
        // after our own intervention as independent predictive confirmation.
        if result.write.as_ref().is_some_and(|r| r.attempted) {
            if let Some(check) = &mut self.view.prediction_check {
                check.invalidated =
                    Some("An explicit trial edit interrupted passive observation.".into());
            }
            self.save_prediction_check()?;
        }
        Ok(result)
    }

    pub fn observe_change(
        &mut self,
        id: &str,
        store: &mut crate::changes::Store,
    ) -> Result<crate::changes::Change> {
        use crate::changes::{ObservationStage, Phase};
        let mut change = store.get(id)?;
        if !matches!(change.phase, Phase::Written | Phase::Restored) {
            return Err("There is no settled write to observe.".into());
        }
        if change.observations.len() >= 8 {
            return Err("This trial reached its eight-observation limit.".into());
        }
        let expected = if change.phase == Phase::Restored {
            change.observed
        } else {
            change.desired
        };
        let stage = if change.phase == Phase::Restored {
            ObservationStage::AfterRestoration
        } else {
            ObservationStage::AfterEdit
        };
        let mut observation = self.capture_change(&change, store, stage, expected)?;
        if change.phase == Phase::Written
            && let Some(probe) = &change.probe
        {
            observation.probe_comparisons =
                probe.compare(observation.value.filter(|_| observation.error.is_none()));
        }
        if let Some(baseline) = &change.baseline {
            observation.comparison = Some(crate::effects::compare(
                &change.watches,
                change.scope.track,
                change.omitted_tracks,
                &baseline.readings,
                &observation.readings,
            ));
        }
        change.observations.push(observation);
        store.record(change)
    }

    fn capture_change(
        &self,
        change: &crate::changes::Change,
        store: &crate::changes::Store,
        stage: crate::changes::ObservationStage,
        expected: f64,
    ) -> Result<crate::changes::Observation> {
        use crate::changes::{Observation, now_ms};
        change.check_target(&*self.target, &self.change_scope(change.scope.track)?)?;
        let (path, reservation) = store.reserve_image(&change.id)?;
        let mut observation = Observation {
            time_ms: now_ms(),
            image: path.to_string_lossy().into_owned(),
            image_sha256: String::new(),
            value: None,
            expected,
            agrees: false,
            error: None,
            warning: None,
            stage: Some(stage),
            capture_interval: None,
            readings: vec![],
            untracked_numbers: 0,
            comparison: None,
            probe_comparisons: vec![],
        };
        let result = (|| -> Result<()> {
            let begin_us = self.capture_clock.elapsed().as_micros() as u64;
            let screen = self.target.screen(&path);
            observation.capture_interval = Some(CaptureInterval {
                clock_id: self.view.investigation_id.clone(),
                begin_us,
                end_us: self.capture_clock.elapsed().as_micros() as u64,
            });
            let mut screen = screen?;
            if std::fs::metadata(&path).map_err(|e| e.to_string())?.len()
                > visual::MAX_IMAGE_BYTES as u64
            {
                return Err("Trial image exceeded its 10 MiB budget.".into());
            }
            // A successful capture has finished writing; an error keeps the
            // reservation for a possible late worker completion.
            std::fs::remove_file(&reservation).map_err(|e| e.to_string())?;
            if [screen.width, screen.height] != change.scope.image_size {
                return Err(
                    "Window geometry changed; the old reading region cannot be reused.".into(),
                );
            }
            observation.image_sha256 = visual::digest(&visual::image_bytes(&path)?);
            // A supplemental recognizer can fail after Windows OCR supplied
            // valid words. Keep that warning and evaluate the actual readings.
            observation.warning = self
                .target
                .recognize(&mut screen)
                .err()
                .or(screen.warning.clone());
            let readings = number_readings(&screen.words);
            // Legacy records still support their original single-region check.
            let fallback = vec![crate::effects::Watch {
                track: change.scope.track,
                label: change.scope.label.clone(),
                bounds: change.scope.bounds,
            }];
            let watches = if change.watches.is_empty() {
                &fallback
            } else {
                &change.watches
            };
            for watch in watches {
                let matches: Vec<_> = readings
                    .iter()
                    .filter(|r| same_text_region(r.bounds, watch.bounds))
                    .collect();
                let unique = matches.len() == 1
                    && watches
                        .iter()
                        .filter(|w| same_text_region(matches[0].bounds, w.bounds))
                        .count()
                        == 1;
                let (value, reason) = if unique {
                    (
                        matches[0].value,
                        matches[0].reason.chars().take(160).collect(),
                    )
                } else {
                    (
                        None,
                        "The region could not be associated unambiguously.".into(),
                    )
                };
                observation.readings.push(crate::effects::Reading {
                    track: watch.track,
                    value,
                    reason,
                });
            }
            observation.untracked_numbers = readings
                .iter()
                .filter(|r| !watches.iter().any(|w| same_text_region(r.bounds, w.bounds)))
                .count();
            observation.value = observation
                .readings
                .iter()
                .find(|r| r.track == change.scope.track)
                .and_then(|r| r.value);
            observation.agrees = observation
                .value
                .is_some_and(|v| (v - expected).abs() <= 0.55);
            if observation.value.is_none() {
                return Err("The selected number is uncertain.".into());
            }
            Ok(())
        })();
        observation.error = result.err();
        Ok(observation)
    }

    /// Retain the evidence, but revoke the connection and hypotheses on exit.
    pub fn refresh_connection(&mut self) -> bool {
        if self.view.connected && !self.target.alive() {
            self.view.connected = false;
            if let Some(check) = &mut self.view.prediction_check {
                check.invalidated = Some("The original process exited.".into());
            }
            let _ = self.save_prediction_check();
            self.update_rule_checks();
            self.view.phase =
                "The application closed. Connect again to start a new investigation.".into();
            for track in &mut self.view.tracks {
                track.matches.clear();
            }
            for track in &mut self.view.visual_tracks {
                track.matches.clear();
                track.active = false;
            }
            for track in &mut self.view.temporal_tracks {
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
        let budget_bytes = budget_mib * 1024 * 1024;
        let (plan, eligible_bytes) =
            coverage::plan(target.regions()?, &Ranges::default(), budget_bytes)?;
        let planned_bytes = plan.iter().map(|c| c.length).sum();
        if planned_bytes == 0 {
            return Err("No readable writable memory regions were found.".into());
        }
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let view = LiveView {
            memory_process: Some(target.memory_info().clone()),
            memory_sources: vec![],
            memory_sources_error: None,
            local_observation_warnings: vec![],
            rule_checks: vec![],
            rule_error: None,
            action_patterns: vec![],
            prediction_check: None,
            coverage: Coverage {
                batch: 1,
                selected_unique_bytes: planned_bytes as u64,
                ..Coverage::default()
            },
            investigation_id: uuid::Uuid::new_v4().to_string(),
            frames: vec![],
            visual_tracks: vec![],
            temporal_tracks: vec![],
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
        let mut session = Self {
            view,
            target,
            plan,
            budget_bytes,
            selected: Ranges::default(),
            read_ranges: Ranges::default(),
            complete_ranges: Ranges::default(),
            invalid_chunks: BTreeSet::new(),
            root_directory: directory.clone(),
            snapshots: vec![],
            receipts: vec![],
            directory,
            screen_size: None,
            image_epoch: 0,
            capture_clock: Instant::now(),
        };
        session.refresh_memory_sources();
        Ok(session)
    }

    pub fn capture(&mut self) -> Result<()> {
        self.capture_mode(CaptureMode::Full)
    }

    fn prediction_read_plan(&self) -> Vec<(u64, Chunk)> {
        let Some(check) = &self.view.prediction_check else {
            return vec![];
        };
        let number = self.view.sample_count + 1;
        if check.invalidated.is_some() || number <= check.after_frame || number > check.last_frame {
            return vec![];
        }
        let mut reads = BTreeMap::new();
        for candidate in check.candidates.iter().filter(|c| c.invalidated.is_none()) {
            if let Some(chunk) = self
                .plan
                .iter()
                .find(|c| c.address == candidate.chunk_address)
            {
                let address = chunk.address + candidate.offset_in_chunk as u64;
                let length = candidate.mapping.kind.width();
                reads.insert(
                    (address, length),
                    (
                        chunk.address,
                        Chunk {
                            address,
                            length,
                            region: chunk.region.clone(),
                        },
                    ),
                );
            }
        }
        reads.into_values().collect()
    }

    fn read_before_image(&mut self, plan: &[(u64, Chunk)], start: Instant) -> PredictionReads {
        let mut witnesses = BTreeMap::new();
        for (original_chunk, chunk) in plan {
            let begin_us = self.capture_clock.elapsed().as_micros() as u64;
            let begin_ms = start.elapsed().as_millis();
            let compatible = || {
                self.target
                    .query(chunk.address)
                    .ok()
                    .flatten()
                    .is_some_and(|r| chunk.covered_by(&r))
            };
            let mut bytes = None;
            let mut disposition = "unavailable";
            if self.invalid_chunks.contains(original_chunk) || !compatible() {
                self.invalid_chunks.insert(*original_chunk);
                disposition = "mapping_changed_or_unavailable";
            } else {
                self.view.coverage.attempted_bytes += chunk.length as u64;
                if let Ok(value) = self.target.read(chunk.address, chunk.length)
                    && value.len() == chunk.length
                {
                    self.view.coverage.read_bytes += value.len() as u64;
                    if compatible() {
                        self.read_ranges.add(chunk.address, chunk.length);
                        bytes = Some(value);
                        disposition = "retained_generation_unverified";
                    } else {
                        self.invalid_chunks.insert(*original_chunk);
                        disposition = "mapping_changed_during_read";
                    }
                }
            }
            if bytes.is_none() {
                self.view.coverage.changed_or_unavailable_bytes += chunk.length as u64;
            }
            let read = MemoryRead {
                address: format!("0x{:X}", chunk.address),
                requested: chunk.length,
                retained: bytes.as_ref().map_or(0, Vec::len),
                begin_ms,
                end_ms: start.elapsed().as_millis(),
                capture_interval: Some(CaptureInterval {
                    clock_id: self.view.investigation_id.clone(),
                    begin_us,
                    end_us: self.capture_clock.elapsed().as_micros() as u64,
                }),
                disposition: disposition.into(),
            };
            witnesses.insert(
                (chunk.address, chunk.length),
                prediction::Witness { bytes, read },
            );
        }
        witnesses
    }

    pub fn capture_mode(&mut self, mode: CaptureMode) -> Result<()> {
        if self.refresh_connection() || !self.view.connected {
            return Err(
                "The selected process has exited. Reconnect before collecting more evidence."
                    .into(),
            );
        }
        if self.snapshots.len() >= MAX_SAMPLES
            || (self.snapshots.len() + 1) * self.view.planned_bytes > RETAINED_MEMORY_BUDGET
        {
            return Err("This memory batch reached its capture budget. Compare its observations, then search more memory.".into());
        }
        let prediction_plan = self.prediction_read_plan();
        let prediction_bytes: usize = prediction_plan.iter().map(|(_, c)| c.length).sum();
        if self.view.coverage.attempted_bytes
            + self.view.planned_bytes as u64
            + prediction_bytes as u64
            > coverage::MAX_READ_BYTES
        {
            return Err("This investigation reached its 4 GiB read budget. Start a new connection to continue.".into());
        }
        if self.view.coverage.image_budget_bytes + visual::MAX_IMAGE_BYTES as u64
            > coverage::MAX_IMAGE_DISK_BYTES
        {
            return Err("This investigation reached its image storage budget.".into());
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let start = Instant::now();
        let number = self.snapshots.len() + 1;
        let screen_path = self.directory.join(format!("frame-{number:03}.png"));
        let before_image = self.read_before_image(&prediction_plan, start);
        let prediction_read_ms = start.elapsed().as_millis();
        let screen_start = Instant::now();
        let screen_started_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let capture_begin_us = self.capture_clock.elapsed().as_micros() as u64;
        let mut screen = self.target.screen(&screen_path);
        let capture_end_us = self.capture_clock.elapsed().as_micros() as u64;
        let image_length = std::fs::metadata(&screen_path)
            .map(|m| m.len())
            .unwrap_or(0);
        if image_length > visual::MAX_IMAGE_BYTES as u64 {
            let _ = std::fs::remove_file(&screen_path);
            screen = Err("The image exceeded the 10 MiB capture budget.".into());
        } else {
            self.view.coverage.image_disk_bytes += image_length;
        }
        // A timed-out renderer can still be writing on its bounded worker.
        // Charge its full allowance even if its file is not visible yet.
        self.view.coverage.image_budget_bytes += if screen.is_err() {
            visual::MAX_IMAGE_BYTES as u64
        } else {
            image_length
        };
        let screen_capture_ms = screen_start.elapsed().as_millis();
        let memory_start = Instant::now();
        let mut snapshot = BTreeMap::new();
        let mut unreadable = 0;
        let mut changed = 0;
        let mut receipts = vec![];
        for chunk in &self.plan {
            let (address, length) = (chunk.address, chunk.length);
            let begin_us = self.capture_clock.elapsed().as_micros() as u64;
            let begin_ms = start.elapsed().as_millis();
            let mut disposition = "unavailable";
            let compatible = |chunk: &Chunk| {
                self.target
                    .query(chunk.address)
                    .ok()
                    .flatten()
                    .is_some_and(|r| chunk.covered_by(&r))
            };
            if self.invalid_chunks.contains(&address) || !compatible(chunk) {
                self.invalid_chunks.insert(address);
                changed += length;
                disposition = "mapping_changed_or_unavailable";
            } else {
                self.view.coverage.attempted_bytes += length as u64;
                match self.target.read(address, length) {
                    Ok(bytes) if bytes.len() == length => {
                        self.view.coverage.read_bytes += length as u64;
                        if compatible(chunk) {
                            self.read_ranges.add(address, length);
                            snapshot.insert(address, bytes);
                            disposition = "retained_generation_unverified";
                        } else {
                            self.invalid_chunks.insert(address);
                            changed += length;
                            disposition = "mapping_changed_during_read";
                        }
                    }
                    _ => {
                        unreadable += length;
                    }
                }
            }
            receipts.push(MemoryRead {
                address: format!("0x{address:X}"),
                requested: length,
                retained: snapshot.get(&address).map_or(0, Vec::len),
                begin_ms,
                end_ms: start.elapsed().as_millis(),
                capture_interval: Some(CaptureInterval {
                    clock_id: self.view.investigation_id.clone(),
                    begin_us,
                    end_us: self.capture_clock.elapsed().as_micros() as u64,
                }),
                disposition: disposition.into(),
            });
        }
        let captured = snapshot.values().map(Vec::len).sum::<usize>();
        let memory_read_ms = prediction_read_ms + memory_start.elapsed().as_millis();
        self.view.coverage.read_unique_bytes = self.read_ranges.bytes();
        self.view.coverage.changed_or_unavailable_bytes += (changed + unreadable) as u64;
        self.view.coverage.analyzed_current_bytes = 0;
        let recognition_start = Instant::now();
        if mode == CaptureMode::Visual {
            // Even adapters which supply text with screen() must not turn an
            // unrecognized capture into a numeric observation.
            if let Ok(frame) = &mut screen {
                frame.text.clear();
                frame.words.clear();
            }
        } else if let Ok(frame) = &mut screen
            && let Err(error) = self.target.recognize(frame)
        {
            frame.warning = Some(match frame.warning.take() {
                Some(previous) => format!("{previous} {error}"),
                None => error,
            });
        }
        let recognition_ms = if mode == CaptureMode::Full {
            recognition_start.elapsed().as_millis()
        } else {
            0
        };
        self.snapshots.push(snapshot);
        self.receipts.push(receipts.clone());
        self.update_complete_coverage();
        if let Ok(frame) = &screen {
            let size = (frame.width, frame.height);
            if self.screen_size.is_some_and(|previous| previous != size) {
                self.image_epoch += 1;
                for track in &mut self.view.temporal_tracks {
                    track.active = false;
                }
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
                    input_context: self.target.input_window(
                        self.view
                            .frames
                            .last()
                            .and_then(|f| f.capture_interval.as_ref())
                            .map_or(capture_begin_us, |t| t.end_us),
                        capture_end_us,
                    ),
                    id: uuid::Uuid::new_v4().to_string(),
                    number,
                    sha256: visual::digest(&bytes),
                    width: frame.width,
                    height: frame.height,
                    epoch: self.image_epoch,
                    started_unix_ms: screen_started_unix_ms,
                    capture_interval: Some(CaptureInterval {
                        clock_id: self.view.investigation_id.clone(),
                        begin_us: capture_begin_us,
                        end_us: capture_end_us,
                    }),
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
            mode,
            number,
            started_unix_ms: now,
            screen_started_unix_ms: Some(screen_started_unix_ms),
            elapsed_ms: start.elapsed().as_millis(),
            memory_read_ms,
            screen_capture_ms,
            recognition_ms,
            captured_bytes: captured,
            unreadable_bytes: unreadable,
            changed_mapping_bytes: changed,
            memory_reads: receipts,
            prediction_reads: before_image.values().map(|w| w.read.clone()).collect(),
            screen_error: screen.as_ref().err().cloned(),
            screen: screen.ok(),
        });
        self.view.action_patterns = crate::action_patterns::analyze(
            &self.view.frames,
            &self.view.tracks,
            &self.view.visual_tracks,
        );
        self.update_rule_checks();
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
        self.view.phase = if mode == CaptureMode::Visual {
            format!(
                "Captured {number} moments. Visual sampling keeps image and memory evidence; text recognition was skipped."
            )
        } else {
            format!(
                "Captured {number} moments. {} screen-number tracks. Correlation needs at least 12 varied observations.",
                self.view.tracks.len()
            )
        };
        self.update_prediction_check(true, Some(&before_image));
        Ok(())
    }

    /// Automatic first-pass reporting uses the same captures and matcher.
    /// Insufficient variation is a normal observation result, not a failed capture.
    pub fn capture_for_report(&mut self) -> Result<()> {
        self.capture()?;
        if self.view.sample_count >= 12
            && self.view.prediction_check.is_none()
            && let Err(reason) = self.correlate()
        {
            self.view.phase = format!("Observations collected; no comparison yet. {reason}");
        }
        Ok(())
    }

    fn complete_chunks(&self) -> Vec<&Chunk> {
        if self.snapshots.len() < 12 {
            return vec![];
        }
        self.plan
            .iter()
            .filter(|c| {
                !self.invalid_chunks.contains(&c.address)
                    && self
                        .snapshots
                        .iter()
                        .all(|s| s.get(&c.address).is_some_and(|b| b.len() == c.length))
            })
            .collect()
    }

    fn update_complete_coverage(&mut self) {
        let mut complete = self.complete_ranges.clone();
        for c in self.complete_chunks() {
            complete.add(c.address, c.length);
        }
        self.view.coverage.complete_trajectory_bytes = complete.bytes();
    }

    /// Archive the current batch before dropping its bulk bytes. New samples
    /// never join an old memory trajectory, even if a label or address repeats.
    pub fn advance_memory(&mut self, investigation: &str, batch: usize) -> Result<()> {
        if investigation != self.view.investigation_id || batch != self.view.coverage.batch {
            return Err(
                "The investigation or memory batch changed. Read status before continuing.".into(),
            );
        }
        if self.refresh_connection() || !self.view.connected {
            return Err("The selected process closed.".into());
        }
        if self.snapshots.is_empty() {
            return Err("Capture this memory batch before moving on.".into());
        }
        if batch >= coverage::MAX_BATCHES {
            return Err("The investigation reached its 24-batch limit.".into());
        }
        let mut selected = self.selected.clone();
        for chunk in &self.plan {
            selected.add(chunk.address, chunk.length);
        }
        let (next, eligible) =
            coverage::plan(self.target.regions()?, &selected, self.budget_bytes)?;
        if next.is_empty() {
            return Err("All currently inventoried eligible addresses have been selected. This does not prove every value was recognized or every allocation lifetime observed.".into());
        }
        let planned_bytes: usize = next.iter().map(|c| c.length).sum();
        if self.view.coverage.attempted_bytes + planned_bytes as u64 * 12 > coverage::MAX_READ_BYTES
        {
            return Err(
                "Not enough of the 4 GiB read budget remains for another 12-capture batch.".into(),
            );
        }
        let matches: Vec<_> = self
            .view
            .tracks
            .iter()
            .flat_map(|t| {
                t.matches
                    .iter()
                    .map(move |m| ("ocr", t.id, t.label.as_str(), m))
            })
            .chain(self.view.visual_tracks.iter().flat_map(|t| {
                t.matches
                    .iter()
                    .map(move |m| ("external", t.id, t.label.as_str(), m))
            }))
            .collect();
        let mut witnesses = vec![];
        for (source, track, label, candidate) in matches.iter().take(256) {
            let address = u64::from_str_radix(candidate.address.trim_start_matches("0x"), 16)
                .map_err(|_| "Invalid candidate address.")?;
            let chunk = self
                .plan
                .iter()
                .find(|c| c.address <= address && address < c.address + c.length as u64)
                .ok_or("Candidate is outside its sampling batch.")?;
            let offset = (address - chunk.address) as usize;
            let width = candidate.mapping.kind.width();
            let bytes: Vec<_> = self
                .snapshots
                .iter()
                .map(|s| {
                    s.get(&chunk.address)
                        .and_then(|b| b.get(offset..offset + width))
                        .map(|b| b.to_vec())
                })
                .collect();
            let timings: Vec<_> = self
                .receipts
                .iter()
                .map(|r| {
                    r.iter()
                        .find(|r| r.address == format!("0x{:X}", chunk.address))
                })
                .collect();
            witnesses.push(serde_json::json!({"source":source,"track":track,"label":label,
                "candidate":candidate,"region":chunk.region,"bytes":bytes,"reads":timings,
                "continuity":"Historical batch only; allocation generation and editing authority unverified."}));
        }
        let archive_path = self.directory.join("memory-batch.json");
        let summary = BatchSummary {
            number: batch,
            captures: self.snapshots.len(),
            planned_bytes: self.view.planned_bytes,
            complete_trajectory_bytes: self.complete_chunks().iter().map(|c| c.length).sum(),
            analyzed_bytes: self.view.coverage.analyzed_current_bytes,
            hypotheses: matches.len(),
            retained_witnesses: witnesses.len(),
            archive: archive_path.to_string_lossy().into_owned(),
        };
        let bytes=serde_json::to_vec(&serde_json::json!({"schema_version":1,"summary":summary,
            "state":self.view,"plan":self.plan,"witnesses":witnesses,
            "scope":"Historical observations; raw noncandidate memory discarded; no current control authority."}))
            .map_err(|e| e.to_string())?;
        if bytes.len() + self.view.coverage.archive_bytes > coverage::MAX_ARCHIVE_BYTES {
            return Err(
                "The 32 MiB batch-archive budget is exhausted. The current batch is preserved."
                    .into(),
            );
        }
        // Prepare all fallible work before releasing the previous snapshots.
        let next_directory = self.root_directory.join(format!("batch-{:03}", batch + 1));
        std::fs::create_dir_all(&next_directory).map_err(|e| e.to_string())?;
        write_evidence(&archive_path, &bytes)?;
        let complete: Vec<_> = self
            .complete_chunks()
            .into_iter()
            .map(|c| (c.address, c.length))
            .collect();
        for (a, n) in complete {
            self.complete_ranges.add(a, n);
        }
        self.selected = selected;
        let mut all_selected = self.selected.clone();
        for c in &next {
            all_selected.add(c.address, c.length);
        }
        self.view.coverage.selected_unique_bytes = all_selected.bytes();
        self.view.coverage.archive_bytes += bytes.len();
        self.view.coverage.batches.push(summary);
        self.view.coverage.batch += 1;
        self.view.coverage.analyzed_current_bytes = 0;
        self.view.eligible_bytes = eligible;
        self.view.planned_bytes = planned_bytes;
        self.plan = next;
        self.directory = next_directory;
        self.snapshots.clear();
        self.receipts.clear();
        self.invalid_chunks.clear();
        self.view.sample_count = 0;
        self.view.retained_bytes = 0;
        self.view.latest = None;
        self.view.frames.clear();
        self.view.action_patterns.clear();
        self.view.rule_checks.clear();
        self.view.rule_error = None;
        self.view.tracks.clear();
        self.view.visual_tracks.clear();
        self.view.local_observation_warnings.clear();
        self.view.temporal_tracks.clear();
        self.view.prediction_check = None;
        self.image_epoch += 1;
        self.screen_size = None;
        self.view.phase = format!(
            "Searching memory batch {}. Earlier evidence is archived. Collect fresh observations as the application changes.",
            batch + 1
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
        self.submit_observations(vec![input], source)
            .map(|count| count != 0)
    }

    /// Validate and save the entire group before publishing any of its observations.
    pub fn submit_observations(
        &mut self,
        inputs: Vec<ObservationInput>,
        source: &str,
    ) -> Result<usize> {
        if inputs.len() > visual::MAX_TRACKS {
            return Err("Too many observations in one group.".into());
        }
        if self.refresh_connection() || !self.view.connected {
            return Err(
                "Reconnect before submitting observations to a closed investigation.".into(),
            );
        }
        let mut tracks = self.view.visual_tracks.clone();
        let mut changed = 0;
        for input in inputs {
            let (frame, _) = self.read_frame(&input.investigation_id, &input.frame_id)?;
            if input.image_sha256 != frame.sha256 {
                return Err(
                    "The observation does not identify the captured image's checksum.".into(),
                );
            }
            if frame.epoch != self.image_epoch {
                return Err(
                    "The window size changed. Submit observations for the current image geometry."
                        .into(),
                );
            }
            changed += usize::from(visual::record(
                &mut tracks,
                input,
                &frame,
                source,
                self.view.sample_count,
            )?);
        }
        if changed != 0 {
            self.save_visual_evidence(&tracks)?;
            self.view.visual_tracks = tracks;
            self.view.action_patterns = crate::action_patterns::analyze(
                &self.view.frames,
                &self.view.tracks,
                &self.view.visual_tracks,
            );
            self.update_rule_checks();
            self.update_prediction_check(false, None);
        }
        Ok(changed)
    }

    /// Read a bounded sequence using engine-owned paths and verified image hashes.
    /// This does not acquire new images or modify the investigation.
    pub fn read_sequence(
        &self,
        investigation: &str,
        ids: &[String],
    ) -> Result<(temporal::Sequence, Vec<Vec<u8>>)> {
        if investigation != self.view.investigation_id {
            return Err("That sequence belongs to a different investigation.".into());
        }
        let sequence = temporal::sequence(&self.view.frames, ids)?;
        let mut images = vec![];
        let mut bytes = 0;
        for frame in &sequence.frames {
            let (_, image) = self.read_frame(investigation, &frame.id)?;
            bytes += image.len();
            if bytes > temporal::MAX_SEQUENCE_BYTES {
                return Err("The selected sequence exceeds 16 MiB. Request fewer images.".into());
            }
            images.push(image);
        }
        Ok((sequence, images))
    }

    pub fn submit_temporal_observation(
        &mut self,
        input: temporal::Input,
        source: &str,
    ) -> Result<bool> {
        if self.refresh_connection() || !self.view.connected {
            return Err("Reconnect before submitting evidence to a closed investigation.".into());
        }
        if input.batch != self.view.coverage.batch {
            return Err("This temporal observation belongs to a different memory batch.".into());
        }
        let ids: Vec<_> = input.frames.iter().map(|f| f.frame_id.clone()).collect();
        let (sequence, _) = self.read_sequence(&input.investigation_id, &ids)?;
        if sequence.frames[0].epoch != self.image_epoch {
            return Err("Image geometry changed. Use the current sequence.".into());
        }
        let mut tracks = self.view.temporal_tracks.clone();
        let changed = temporal::record(&mut tracks, input, sequence, source)?;
        if changed {
            let bytes = serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1, "investigation_id": self.view.investigation_id,
                "memory_batch": self.view.coverage.batch, "process": self.view.process,"memory_process":self.view.memory_process,
                "temporal_tracks": tracks, "limitation": temporal::LIMITATION,
            }))
            .map_err(|e| e.to_string())?;
            write_evidence(&self.directory.join("temporal-evidence.json"), &bytes)?;
            self.view.temporal_tracks = tracks;
        }
        Ok(changed)
    }

    fn save_visual_evidence(&self, tracks: &[VisualTrack]) -> Result<()> {
        // Archived evidence survives the active engine's restart invalidation.
        // This file is not a journal or a transaction with session.json.
        let bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version":1,
            "investigation_id":self.view.investigation_id,
            "process":self.view.process,"memory_process":self.view.memory_process,
            "frames":self.view.frames,
            "visual_tracks":tracks,
            "number_tracks":self.view.tracks,
            "action_patterns":crate::action_patterns::analyze(&self.view.frames, &self.view.tracks, tracks),
            "memory_batch":self.view.coverage.batch,
            "scope":"Historical image interpretations; no current address or editing authority."
        }))
        .map_err(|e| e.to_string())?;
        write_evidence(&self.directory.join("evidence.json"), &bytes)
    }

    fn update_rule_checks(&mut self) {
        if self.view.rule_error.is_some() {
            return;
        }
        let mut checks =
            crate::rule_checks::update(&self.view, self.capture_clock.elapsed().as_micros() as u64);
        // Stamp new forecasts after computation. A start-of-computation timestamp
        // could incorrectly precede a press that happened while fitting ran.
        let prepared_us = self.capture_clock.elapsed().as_micros() as u64;
        crate::rule_checks::stamp_prepared(&mut checks, &self.view.rule_checks, prepared_us);
        if checks.is_empty() {
            return;
        }
        let persist = || -> Result<()> {
            let bytes=serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version":1,"investigation_id":self.view.investigation_id,"batch":self.view.coverage.batch,
                "process":self.view.process,"memory_process":self.view.memory_process,"checks":checks,"frames":self.view.frames,
                "number_tracks":self.view.tracks,"visual_tracks":self.view.visual_tracks,
                "action_patterns":self.view.action_patterns,"scope":crate::rule_checks::LIMITATION,
            })).map_err(|e|e.to_string())?;
            write_evidence(&self.directory.join("rule-evidence.json"), &bytes)
        };
        match persist() {
            Ok(()) => self.view.rule_checks = checks,
            Err(error) => {
                let reason = format!(
                    "Pattern predictions stopped because their evidence could not be saved: {error}"
                );
                for check in &mut self.view.rule_checks {
                    check.invalidated = Some(reason.clone());
                }
                self.view.rule_error = Some(reason);
            }
        }
    }

    /// Freeze one series' existing candidates before any prospective samples.
    /// A stale call cannot freeze a different series, batch or fitting interval.
    pub fn start_prediction_check(
        &mut self,
        investigation: &str,
        batch: usize,
        after_frame: usize,
        source: prediction::Source,
        track_id: usize,
    ) -> Result<()> {
        if self.refresh_connection() || !self.view.connected {
            return Err("The selected process closed.".into());
        }
        if investigation != self.view.investigation_id
            || batch != self.view.coverage.batch
            || after_frame != self.view.sample_count
        {
            return Err(
                "The investigation, batch or capture count changed. Read status again.".into(),
            );
        }
        if self.view.prediction_check.is_some() {
            return Err("This batch already has frozen predictions. Keep collecting evidence or move to another memory batch.".into());
        }
        let last_frame = MAX_SAMPLES.min(RETAINED_MEMORY_BUDGET / self.view.planned_bytes.max(1));
        if after_frame < 12 || last_frame.saturating_sub(after_frame) < prediction::MIN_FRESH {
            return Err("A prediction check needs at least four captures remaining after comparison. Compare after 12 moments in a fresh batch.".into());
        }
        let (label, matches) = match source {
            prediction::Source::Ocr => self
                .view
                .tracks
                .iter()
                .find(|t| t.id == track_id && t.active)
                .map(|t| (t.label.clone(), t.matches.clone())),
            prediction::Source::External => self
                .view
                .visual_tracks
                .iter()
                .find(|t| t.id == track_id && t.active)
                .map(|t| (t.label.clone(), t.matches.clone())),
        }
        .ok_or("That observation series is not active.")?;
        if matches.is_empty() || matches.len() > 200 {
            return Err("Compare changes to obtain candidates for this series first.".into());
        }
        let mut candidates = vec![];
        for candidate in matches {
            let address = u64::from_str_radix(candidate.address.trim_start_matches("0x"), 16)
                .map_err(|_| "Invalid candidate address.")?;
            let chunk = self
                .plan
                .iter()
                .find(|c| c.address <= address && address < c.address + c.length as u64)
                .ok_or("Candidate is outside its memory batch.")?;
            let offset = (address - chunk.address) as usize;
            if offset + candidate.mapping.kind.width() > chunk.length
                || self.invalid_chunks.contains(&chunk.address)
            {
                return Err("Candidate memory is no longer available in this batch.".into());
            }
            candidates.push(FrozenCandidate {
                address: candidate.address,
                chunk_address: chunk.address,
                offset_in_chunk: offset,
                mapping: candidate.mapping,
                invalidated: None,
                samples: vec![],
            });
        }
        let check = PredictionCheck {
            id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation.into(),
            batch,
            epoch: self.image_epoch,
            source,
            track: track_id,
            label,
            after_frame,
            last_frame,
            tolerance: prediction::TOLERANCE,
            basis: self.view.frames.clone(),
            invalidated: None,
            candidates,
        };
        // Prepare the frozen evidence before publishing it. A failed save cannot
        // silently start a new prospective interval on a later retry.
        self.persist_prediction_check(&check)?;
        self.view.prediction_check = Some(check);
        self.view.phase = "Predictions frozen. Capture at least four more moments: include a value staying still and a distinct change. Continue normal gameplay; no memory edits are made.".into();
        Ok(())
    }

    fn persist_prediction_check(&self, check: &PredictionCheck) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&serde_json::json!({"schema_version":1,
            "process":self.view.process,"memory_process":self.view.memory_process, "check":check,
            "scope":"Historical frozen predictions. Non-atomic captures, provisional observations and unverified allocation continuity. Never editing authority."}))
            .map_err(|e| e.to_string())?;
        write_evidence(&self.directory.join("prediction-check.json"), &bytes)
    }

    fn save_prediction_check(&self) -> Result<()> {
        if let Some(check) = &self.view.prediction_check {
            self.persist_prediction_check(check)?;
        }
        Ok(())
    }

    fn update_prediction_check(&mut self, captured: bool, before_image: Option<&PredictionReads>) {
        let Some(check) = &mut self.view.prediction_check else {
            return;
        };
        if check.epoch != self.image_epoch {
            check.invalidated = Some(
                "Image geometry changed; observation identity is no longer comparable.".into(),
            );
        }
        let number = self.view.sample_count;
        if captured && number > check.after_frame && number <= check.last_frame {
            for candidate in &mut check.candidates {
                if self.invalid_chunks.contains(&candidate.chunk_address) {
                    candidate.invalidated =
                        Some("Memory mapping changed or became unavailable.".into());
                }
                let bytes = self
                    .snapshots
                    .last()
                    .and_then(|s| s.get(&candidate.chunk_address))
                    .and_then(|b| {
                        b.get(
                            candidate.offset_in_chunk
                                ..candidate.offset_in_chunk + candidate.mapping.kind.width(),
                        )
                    })
                    .map(|b| b.to_vec());
                let predicted = bytes
                    .as_ref()
                    .and_then(|b| candidate.mapping.kind.read(b))
                    .map(|x| candidate.mapping.slope * x + candidate.mapping.intercept)
                    .filter(|p| p.is_finite());
                let read = self
                    .receipts
                    .last()
                    .and_then(|reads| {
                        reads
                            .iter()
                            .find(|r| r.address == format!("0x{:X}", candidate.chunk_address))
                    })
                    .cloned();
                candidate.samples.push(prediction::Sample {
                    frame: number,
                    before_image: before_image
                        .and_then(|reads| {
                            reads.get(&(
                                candidate.chunk_address + candidate.offset_in_chunk as u64,
                                candidate.mapping.kind.width(),
                            ))
                        })
                        .cloned(),
                    image: self
                        .view
                        .frames
                        .iter()
                        .find(|f| f.number == number)
                        .cloned(),
                    bytes,
                    read,
                    predicted,
                    observed: None,
                    outcome: prediction::Outcome::Pending,
                });
            }
        }
        let values = match check.source {
            prediction::Source::Ocr => self
                .view
                .tracks
                .iter()
                .find(|t| t.active && t.id == check.track)
                .map(|t| t.samples.clone()),
            prediction::Source::External => self
                .view
                .visual_tracks
                .iter()
                .find(|t| t.active && t.id == check.track)
                .map(VisualTrack::numeric_samples),
        };
        check.observe(&values.unwrap_or_default());
        self.view.phase = check.summary();
        if let Err(error) = self.save_prediction_check() {
            if let Some(check) = &mut self.view.prediction_check {
                check.invalidated =
                    Some(format!("Prediction evidence could not be saved: {error}"));
            }
            self.view.phase =
                format!("Prediction check invalidated: evidence could not be saved. {error}");
        }
    }

    pub fn correlate(&mut self) -> Result<()> {
        if self.view.prediction_check.is_some() {
            return Err("Predictions are frozen in this batch. New captures challenge them without refitting. Search more memory for a new discovery batch.".into());
        }
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
            return Err("No reliable changing number has been observed yet. More varied observations are needed before comparison.".into());
        }
        let mut analyzed = 0usize;
        for chunk in &self.plan {
            let (base, length) = (chunk.address, chunk.length);
            if self.invalid_chunks.contains(&base) {
                continue;
            }
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
                        evidence:format!("Correlates with {} on held-out frames in memory batch {}. Allocation generation, meaning and causal authority are unverified; this is not an editable control.", if *external { "an externally supplied visual interpretation" } else { "an OCR observation" },self.view.coverage.batch),
                    });
                }
            }
        }
        self.view.coverage.analyzed_current_bytes = analyzed;
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

fn write_evidence(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let temporary = path.with_extension("pending");
    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|e| format!("Evidence was not saved: {e}"))
}

/// Conservative screen-space association, not semantic identification.
/// Keep missing detections as gaps; never replace them with zero.
pub fn track_numbers(tracks: &mut Vec<NumberTrack>, words: &[Word], count: usize) {
    for track in tracks.iter_mut() {
        track.samples.push(None);
        track.readings.push(None);
        track.matches.clear();
    }
    let readings = number_readings(words);
    // Compute association for the whole frame before assigning anything. A
    // nearby, disjoint region must not compete with a direct spatial overlap.
    // Otherwise small neighboring detections can erase an unchanged HUD track,
    // and iteration order decides which reading consumes a track first.
    let overlaps: Vec<Vec<usize>> = readings
        .iter()
        .map(|reading| {
            tracks
                .iter()
                .enumerate()
                .filter(|(_, t)| t.active && same_text_region(t.bounds, reading.bounds))
                .map(|(i, _)| i)
                .collect()
        })
        .collect();
    let reserved: std::collections::HashSet<usize> = overlaps.iter().flatten().copied().collect();
    let choices: Vec<Vec<usize>> = readings
        .iter()
        .zip(overlaps)
        .map(|(reading, overlap)| {
            if !overlap.is_empty() {
                return overlap;
            }
            tracks
                .iter()
                .enumerate()
                .filter(|(i, t)| {
                    t.active
                        && !reserved.contains(i)
                        && (t.bounds[0] - reading.bounds[0]).abs() < 24.0
                        && (t.bounds[1] - reading.bounds[1]).abs() < 12.0
                })
                .map(|(i, _)| i)
                .collect()
        })
        .collect();
    let mut users = vec![0; tracks.len()];
    for &index in choices.iter().flatten() {
        users[index] += 1;
    }
    for (mut reading, possible) in readings.into_iter().zip(choices) {
        if possible.len() > 1 || possible.iter().any(|&i| users[i] > 1) {
            reading.value = None;
            reading.reason = "ambiguous_track_assignment".into();
            for index in possible {
                tracks[index].readings[count - 1] = Some(reading.clone());
            }
        } else if let Some(&index) = possible.first() {
            tracks[index].samples[count - 1] = reading.value;
            tracks[index].bounds = reading.bounds;
            tracks[index].readings[count - 1] = Some(reading);
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
        recognized: Rc<Cell<usize>>,
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
                    ..Region::default()
                },
                Region {
                    base: 0x20000,
                    length: 128,
                    image: false,
                    ..Region::default()
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
    fn visual_sampling_keeps_memory_and_numeric_gaps_without_calling_recognition() {
        let alive = Rc::new(Cell::new(true));
        let target = SampleTarget {
            info: ProcessInfo::default(),
            frame: Cell::new(0),
            alive,
            recognized: Rc::new(Cell::new(0)),
        };
        let recognized = target.recognized.clone();
        let directory = std::env::temp_dir().join(format!(
            "reversebolt-visual-sampling-{}",
            uuid::Uuid::new_v4()
        ));
        let mut session = LiveSession::attach(Box::new(target), 1, directory.clone()).unwrap();
        session.capture().unwrap();
        assert_eq!(session.view.tracks[0].samples, vec![Some(20.)]);
        session.capture_mode(CaptureMode::Visual).unwrap();
        let latest = session.view.latest.as_ref().unwrap();
        assert_eq!(recognized.get(), 1, "Visual mode must not invoke OCR");
        assert_eq!(latest.mode, CaptureMode::Visual);
        assert_eq!(latest.recognition_ms, 0);
        assert_eq!(latest.captured_bytes, 256);
        assert!(latest.screen.as_ref().unwrap().words.is_empty());
        assert!(latest.screen.as_ref().unwrap().text.is_empty());
        assert_eq!(session.view.tracks[0].samples, vec![Some(20.), None]);
        assert!(session.view.tracks[0].readings[1].is_none());
        // Resume text reading without carrying or interpolating the skipped value.
        session.capture().unwrap();
        assert_eq!(
            session.view.tracks[0].samples,
            vec![Some(20.), None, Some(34.)]
        );
        assert!(directory.starts_with(std::env::temp_dir()));
        std::fs::remove_dir_all(directory).unwrap();
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
                recognized: Rc::new(Cell::new(0)),
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
        let mut bad_group = inputs[1].clone();
        bad_group.image_sha256 = "wrong".into();
        assert!(
            session
                .submit_observations(vec![inputs[0].clone(), bad_group], "local-model")
                .is_err()
        );
        assert!(
            session.view.visual_tracks.is_empty(),
            "A later invalid item published the earlier item"
        );
        let archive = directory.join("evidence.json");
        std::fs::remove_file(&archive).unwrap();
        std::fs::create_dir(&archive).unwrap();
        assert!(
            session
                .submit_observations(inputs[..2].to_vec(), "agent")
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
                recognized: Rc::new(Cell::new(0)),
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
    fn adjacent_disjoint_regions_do_not_displace_direct_spatial_matches() {
        let word = |text, b| reading_word(text, b, TextSource::PpOcrV6);
        for reverse in [false, true] {
            let mut tracks = vec![];
            let mut first = vec![
                word("90", [100., 50., 50., 30.]),
                word("1", [88., 54., 8., 20.]),
            ];
            let mut second = vec![
                word("89", [101., 50., 49., 30.]),
                word("3", [87., 54., 8., 20.]),
            ];
            if reverse {
                first.reverse();
                second.reverse();
            }
            track_numbers(&mut tracks, &first, 1);
            track_numbers(&mut tracks, &second, 2);
            assert_eq!(tracks.len(), 2);
            assert_eq!(tracks[0].samples, vec![Some(90.), Some(89.)]);
            assert_eq!(tracks[1].samples, vec![Some(1.), Some(3.)]);
        }
    }

    #[test]
    fn two_current_readings_cannot_consume_one_previous_track_by_iteration_order() {
        let word = |text, b| reading_word(text, b, TextSource::PpOcrV6);
        let mut tracks = vec![];
        track_numbers(&mut tracks, &[word("123", [0., 0., 100., 30.])], 1);
        track_numbers(
            &mut tracks,
            &[
                word("4", [0., 0., 30., 30.]),
                word("5", [70., 0., 30., 30.]),
            ],
            2,
        );
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].samples, vec![Some(123.), None]);
        assert_eq!(
            tracks[0].readings[1].as_ref().unwrap().reason,
            "ambiguous_track_assignment"
        );
        assert_eq!(tracks[0].bounds, [0., 0., 100., 30.]);
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
