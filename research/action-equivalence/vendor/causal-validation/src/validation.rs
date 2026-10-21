use crate::discovery::Candidate;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub bytes: Vec<u8>,
    pub tracks: Vec<f64>,
    /// Identity/generation, not an address. Must change on reload/relocation.
    pub epoch: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

impl ByteRange {
    fn contains(&self, start: usize, length: usize) -> bool {
        start >= self.start && start.checked_add(length).is_some_and(|end| end <= self.end)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scope {
    pub writable: Vec<ByteRange>,
    pub track: usize,
    pub tolerances: Vec<f64>,
    pub minimum: f64,
    pub maximum: f64,
    /// A trusted harness assertion about effect observation, never inferred from
    /// a successful probe. A live backend must default this to false.
    pub closed_effect_scope: bool,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Policy {
    pub max_interventions: usize,
    pub max_patch_bytes: usize,
    pub max_delta: f64,
    pub horizons: Vec<usize>,
    pub recovery_ticks: usize,
    pub minimum_witnesses: usize,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            max_interventions: 24,
            max_patch_bytes: 8,
            max_delta: 20.0,
            horizons: vec![1, 2, 8],
            recovery_ticks: 8,
            minimum_witnesses: 2,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Patch {
    pub offset: usize,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct WriteReceipt {
    /// The exact post-write bytes if known, including a known partial write.
    /// None means unknown outcome; recovery must fail closed.
    pub expected_after: Option<Vec<u8>>,
    pub error: Option<String>,
}

/// Each instance is owned and disposable. There is deliberately no PID/address
/// attachment, whole-region restoration, reset, or evaluator-truth operation.
pub trait DisposableTarget {
    fn read(&mut self) -> Result<Frame, String>;
    fn tick(&mut self) -> Result<Frame, String>;
    /// Atomic compare/write inside the backend's paused/exclusive transaction.
    fn compare_write(&mut self, epoch: u64, patch: &Patch) -> WriteReceipt;
    /// Must destroy this target, including after unknown write outcomes.
    fn dispose(&mut self) -> Result<(), String>;
}

pub trait TrialFactory {
    /// Fresh isolated replay at the same baseline; never reset a live game.
    fn open(&mut self) -> Result<Box<dyn DisposableTarget>, String>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Recovery {
    Verified,
    AlreadyOriginal,
    NoWrite,
    Conflict,
    UnknownWrite,
    ReadFailed,
    WriteFailed,
    EffectsRemain,
    EpochChanged,
}

impl Recovery {
    pub fn failed(&self) -> bool {
        !matches!(self, Self::Verified | Self::AlreadyOriginal | Self::NoWrite)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrialEvidence {
    pub candidate: usize,
    pub desired: f64,
    pub patch: Patch,
    pub predicted: Vec<Option<f64>>,
    pub predicted_separations: usize,
    pub observed: Vec<Vec<f64>>,
    pub eliminated: Vec<usize>,
    pub outside_patch_changes: Vec<usize>,
    pub unexpected_changes: Vec<usize>,
    pub recovery: Recovery,
    pub disposed: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Decision {
    IdentifiedWithinScope,
    Abstain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub decision: Decision,
    pub reason: String,
    pub selected: Option<usize>,
    pub survivors: Vec<usize>,
    pub overlaps: Vec<(usize, usize)>,
    pub trials: Vec<TrialEvidence>,
    pub interventions: usize,
    pub rollback_failures: usize,
    pub disposal_failures: usize,
}

impl Report {
    fn new(candidates: &[Candidate]) -> Self {
        let mut overlaps = vec![];
        for (i, a) in candidates.iter().enumerate() {
            for (j, b) in candidates.iter().enumerate().skip(i + 1) {
                if a.offset < b.offset.saturating_add(b.kind.width())
                    && b.offset < a.offset.saturating_add(a.kind.width())
                {
                    overlaps.push((i, j));
                }
            }
        }
        Self {
            decision: Decision::Abstain,
            reason: String::new(),
            selected: None,
            survivors: (0..candidates.len()).collect(),
            overlaps,
            trials: vec![],
            interventions: 0,
            rollback_failures: 0,
            disposal_failures: 0,
        }
    }
}

fn estimate(c: &Candidate, bytes: &[u8]) -> Option<f64> {
    let value = c.slope * c.kind.read(bytes.get(c.offset..)?)? + c.intercept;
    value.is_finite().then_some(value)
}

fn valid_frame(frame: &Frame, baseline: &Frame, scope: &Scope) -> bool {
    frame.epoch == baseline.epoch
        && frame.bytes.len() == baseline.bytes.len()
        && frame.tracks.len() == baseline.tracks.len()
        && frame.tracks.len() == scope.tolerances.len()
        && frame.tracks.iter().all(|v| v.is_finite())
}

fn same_tracks(frame: &Frame, baseline: &Frame, scope: &Scope) -> bool {
    valid_frame(frame, baseline, scope)
        && frame
            .tracks
            .iter()
            .zip(&baseline.tracks)
            .zip(&scope.tolerances)
            .all(|((a, b), tolerance)| (a - b).abs() <= *tolerance)
}

fn patch_bytes(baseline: &[u8], patch: &Patch) -> Option<Vec<u8>> {
    let mut bytes = baseline.to_vec();
    bytes
        .get_mut(patch.offset..patch.offset.checked_add(patch.after.len())?)?
        .copy_from_slice(&patch.after);
    Some(bytes)
}

fn recover(
    target: &mut dyn DisposableTarget,
    baseline: &Frame,
    scope: &Scope,
    policy: &Policy,
    patch: &Patch,
    receipt: &WriteReceipt,
) -> Recovery {
    let Ok(current) = target.read() else {
        return Recovery::ReadFailed;
    };
    if current.epoch != baseline.epoch {
        return Recovery::EpochChanged;
    }
    let range = patch.offset..patch.offset + patch.before.len();
    let Some(now) = current.bytes.get(range.clone()) else {
        return Recovery::ReadFailed;
    };
    let already_original = now == patch.before;
    if !already_original {
        let Some(expected) = &receipt.expected_after else {
            return Recovery::UnknownWrite;
        };
        if now != expected {
            return Recovery::Conflict;
        }
        let undo = Patch {
            offset: patch.offset,
            before: expected.clone(),
            after: patch.before.clone(),
        };
        if target.compare_write(baseline.epoch, &undo).error.is_some() {
            return Recovery::WriteFailed;
        }
    }
    for _ in 0..policy.recovery_ticks {
        if target.tick().is_err() {
            return Recovery::ReadFailed;
        }
    }
    let Ok(restored) = target.read() else {
        return Recovery::ReadFailed;
    };
    if restored.epoch != baseline.epoch {
        return Recovery::EpochChanged;
    }
    if restored.bytes.get(range) != Some(patch.before.as_slice()) {
        return Recovery::WriteFailed;
    }
    // Full *comparison* is useful in a quiescent fixture; never a full write.
    if !same_tracks(&restored, baseline, scope) || restored.bytes != baseline.bytes {
        return Recovery::EffectsRemain;
    }
    if already_original {
        Recovery::AlreadyOriginal
    } else {
        Recovery::Verified
    }
}

#[derive(Clone)]
struct Proposal {
    owner: usize,
    desired: f64,
    patch: Patch,
    predicted: Vec<Option<f64>>,
    separations: usize,
    score: f64,
}

fn propose(
    candidates: &[Candidate],
    baseline: &Frame,
    scope: &Scope,
    policy: &Policy,
    report: &Report,
    witnesses: &[usize],
) -> Option<Proposal> {
    let value = baseline.tracks[scope.track];
    let lo = scope.minimum.max(value - policy.max_delta);
    let hi = scope.maximum.min(value + policy.max_delta);
    let tolerance = scope.tolerances[scope.track];
    // A fixed, preregistered design grid, NOT seed-specific tuning. It includes
    // legacy deltas and fractional, endpoint, and interior discrimination probes.
    let mut desired = vec![value + 3.0, value - 4.0];
    for f in [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0] {
        desired.push(lo + (hi - lo) * f);
    }
    let mut best: Option<Proposal> = None;
    for &owner in &report.survivors {
        let c = &candidates[owner];
        for &y in &desired {
            if y < lo || y > hi || (y - value).abs() <= 2.0 * tolerance {
                continue;
            }
            let Some(after) = c.encode_observation(y) else {
                continue;
            };
            if after.len() > policy.max_patch_bytes
                || !scope
                    .writable
                    .iter()
                    .any(|r| r.contains(c.offset, after.len()))
            {
                continue;
            }
            let Some(before) = baseline.bytes.get(c.offset..c.offset + after.len()) else {
                continue;
            };
            let patch = Patch {
                offset: c.offset,
                before: before.to_vec(),
                after,
            };
            if patch.before == patch.after
                || report
                    .trials
                    .iter()
                    .any(|t| t.patch.offset == patch.offset && t.patch.after == patch.after)
            {
                continue;
            }
            let patched = patch_bytes(&baseline.bytes, &patch)?;
            let predicted: Vec<_> = candidates.iter().map(|h| estimate(h, &patched)).collect();
            // Robust bounds: every still-live explanation must predict a finite,
            // bounded result, not just the candidate whose encoding was chosen.
            if report
                .survivors
                .iter()
                .any(|&i| predicted[i].is_none_or(|p| p < lo - tolerance || p > hi + tolerance))
                || predicted[owner].is_none_or(|p| (p - y).abs() > tolerance)
            {
                continue;
            }
            let mut separations = 0;
            for (position, &i) in report.survivors.iter().enumerate() {
                for &j in &report.survivors[position + 1..] {
                    if (predicted[i]? - predicted[j]?).abs() > 2.0 * tolerance {
                        separations += 1;
                    }
                }
            }
            let changed_bytes = patch
                .before
                .iter()
                .zip(&patch.after)
                .filter(|(a, b)| a != b)
                .count();
            let score = separations as f64 * 100.0
                + if witnesses[owner] < policy.minimum_witnesses {
                    10.0
                } else {
                    0.0
                }
                - changed_bytes as f64 * 0.01
                - (y - value).abs() * 0.001;
            if best.as_ref().is_none_or(|b| score > b.score) {
                best = Some(Proposal {
                    owner,
                    desired: y,
                    patch,
                    predicted,
                    separations,
                    score,
                });
            }
        }
    }
    best
}

/// `dependencies` are passive candidate models for *every* monitored track,
/// including derived copies; they are not field labels or authoritative layouts.
/// A changed byte outside the patch must be explained by a matching observed
/// track model, otherwise the declared single-effect scope is rejected.
pub fn validate(
    factory: &mut dyn TrialFactory,
    baseline: &Frame,
    candidates: &[Candidate],
    dependencies: &[Vec<Candidate>],
    scope: &Scope,
    policy: &Policy,
) -> Report {
    let mut report = Report::new(candidates);
    let mut witnesses = vec![0_usize; candidates.len()];
    if scope.track >= baseline.tracks.len()
        || !valid_frame(baseline, baseline, scope)
        || dependencies.len() != baseline.tracks.len()
        || scope.tolerances.iter().any(|v| !v.is_finite() || *v <= 0.0)
        || !scope.minimum.is_finite()
        || !scope.maximum.is_finite()
        || scope.minimum >= scope.maximum
        || policy.horizons.is_empty()
        || policy.horizons.contains(&0)
        || policy.max_delta <= 0.0
        || !policy.max_delta.is_finite()
        || policy.minimum_witnesses < 2
        || policy.recovery_ticks < policy.horizons.iter().copied().max().unwrap_or(1)
        || candidates
            .iter()
            .chain(dependencies.iter().flatten())
            .any(|c| {
                c.offset
                    .checked_add(c.kind.width())
                    .is_none_or(|end| end > baseline.bytes.len())
                    || !c.slope.is_finite()
                    || c.slope.abs() < 1.0e-12
                    || !c.intercept.is_finite()
            })
    {
        report.reason = "invalid scope or policy".into();
        return report;
    }
    if candidates.is_empty() {
        report.reason = "no supported passive representation".into();
        return report;
    }
    // Do not silently discard an unencodable/out-of-scope rival and promote the
    // remaining one: unresolved alternatives must remain in the report.
    loop {
        if report.survivors.is_empty() {
            report.reason = "all supplied causal models contradicted".into();
            break;
        }
        if report.survivors.len() == 1 && witnesses[report.survivors[0]] >= policy.minimum_witnesses
        {
            if scope.closed_effect_scope {
                report.decision = Decision::IdentifiedWithinScope;
                report.selected = Some(report.survivors[0]);
                report.reason =
                    "unique tested model within declared scope and representation family".into();
            } else {
                report.reason =
                    "observed influence only; unobserved effects or authority remain possible"
                        .into();
            }
            break;
        }
        if report.interventions >= policy.max_interventions {
            report.reason = "intervention budget exhausted with unresolved hypotheses".into();
            break;
        }
        let Some(proposal) = propose(candidates, baseline, scope, policy, &report, &witnesses)
        else {
            report.reason = "no unused bounded discriminating or confirmation probe".into();
            break;
        };
        if proposal.separations == 0
            && report.survivors.len() > 1
            && report
                .survivors
                .iter()
                .all(|&i| witnesses[i] >= policy.minimum_witnesses)
        {
            report.reason =
                "representations indistinguishable on the available bounded design".into();
            break;
        }
        let mut target = match factory.open() {
            Ok(target) => target,
            Err(error) => {
                report.reason = format!("cannot create isolated trial: {error}");
                break;
            }
        };
        // Every write is preceded by baseline/identity checks and a sham replay.
        let preflight = (|| -> Result<(), String> {
            let frame = target.read()?;
            if !same_tracks(&frame, baseline, scope) || frame.bytes != baseline.bytes {
                return Err("baseline mismatch".into());
            }
            for _ in 0..*policy.horizons.iter().max().unwrap() {
                let sham = target.tick()?;
                if !same_tracks(&sham, baseline, scope) || sham.bytes != baseline.bytes {
                    return Err("sham drift or epoch change".into());
                }
            }
            Ok(())
        })();
        if let Err(error) = preflight {
            report.disposal_failures += usize::from(target.dispose().is_err());
            report.reason = error;
            break;
        }
        let receipt = target.compare_write(baseline.epoch, &proposal.patch);
        report.interventions += 1;
        let mut evidence = TrialEvidence {
            candidate: proposal.owner,
            desired: proposal.desired,
            patch: proposal.patch.clone(),
            predicted: proposal.predicted.clone(),
            predicted_separations: proposal.separations,
            observed: vec![],
            eliminated: vec![],
            outside_patch_changes: vec![],
            unexpected_changes: vec![],
            recovery: Recovery::NoWrite,
            disposed: false,
            error: receipt.error.clone(),
        };
        let mut frames = vec![];
        if receipt.error.is_none() {
            for tick in 1..=*policy.horizons.iter().max().unwrap() {
                match target.tick() {
                    Ok(frame) if valid_frame(&frame, baseline, scope) => {
                        if policy.horizons.contains(&tick) {
                            evidence.observed.push(frame.tracks.clone());
                        }
                        // Check side effects at *every* tick, even between scoring horizons.
                        if frame.tracks.iter().enumerate().any(|(i, value)| {
                            i != scope.track
                                && (value - baseline.tracks[i]).abs() > scope.tolerances[i]
                        }) || frame.tracks[scope.track] < scope.minimum
                            || frame.tracks[scope.track] > scope.maximum
                        {
                            evidence.error = Some("effect outside requested track/domain".into());
                        }
                        let patched = patch_bytes(&baseline.bytes, &proposal.patch).unwrap();
                        for (offset, (&before, &after)) in
                            baseline.bytes.iter().zip(&frame.bytes).enumerate()
                        {
                            if before == after
                                || (offset >= proposal.patch.offset
                                    && offset < proposal.patch.offset + proposal.patch.after.len())
                            {
                                continue;
                            }
                            evidence.outside_patch_changes.push(offset);
                            let explained = dependencies.iter().enumerate().any(|(track, models)| {
                                models.iter().any(|model| {
                                    offset >= model.offset && offset < model.offset + model.kind.width()
                                        && estimate(model, &baseline.bytes).is_some_and(|y| (y - baseline.tracks[track]).abs() <= scope.tolerances[track])
                                        && estimate(model, &frame.bytes).is_some_and(|y| (y - frame.tracks[track]).abs() <= scope.tolerances[track])
                                        // A constant unrelated observed track cannot explain new changes.
                                        && (frame.tracks[track] - baseline.tracks[track]).abs() > scope.tolerances[track]
                                })
                            });
                            if !explained {
                                evidence.unexpected_changes.push(offset);
                            }
                        }
                        frames.push((tick, frame, patched));
                    }
                    Ok(_) => {
                        evidence.error =
                            Some("invalid observation shape, nonfinite value, or epoch".into());
                        break;
                    }
                    Err(error) => {
                        evidence.error = Some(error);
                        break;
                    }
                }
            }
        }
        evidence.outside_patch_changes.sort_unstable();
        evidence.outside_patch_changes.dedup();
        evidence.unexpected_changes.sort_unstable();
        evidence.unexpected_changes.dedup();
        if !evidence.unexpected_changes.is_empty() {
            evidence.error = Some("unexplained memory effects outside patch".into());
        }
        if evidence.error.is_none() {
            for &i in &report.survivors {
                let h = &candidates[i];
                let overlaps = h.offset < proposal.patch.offset + proposal.patch.after.len()
                    && proposal.patch.offset < h.offset + h.kind.width();
                let contradicted = frames.iter().any(|(tick, frame, patched)| {
                    if !policy.horizons.contains(tick) {
                        return false;
                    }
                    // A disjoint hypothesis whose storage was updated by the
                    // target may mediate the effect. Do not falsely eliminate it.
                    let externally_changed = frame.bytes.get(h.offset..h.offset + h.kind.width())
                        != patched.get(h.offset..h.offset + h.kind.width());
                    if !overlaps && externally_changed {
                        return false;
                    }
                    proposal.predicted[i].is_none_or(|predicted| {
                        (predicted - frame.tracks[scope.track]).abs()
                            > scope.tolerances[scope.track]
                    })
                });
                if contradicted {
                    evidence.eliminated.push(i);
                }
            }
            if !evidence.eliminated.contains(&proposal.owner) {
                witnesses[proposal.owner] += 1;
            }
        }
        // Recovery and disposal run even after partial writes / bad observations.
        evidence.recovery = recover(
            &mut *target,
            baseline,
            scope,
            policy,
            &proposal.patch,
            &receipt,
        );
        evidence.disposed = target.dispose().is_ok();
        report.rollback_failures += usize::from(evidence.recovery.failed());
        report.disposal_failures += usize::from(!evidence.disposed);
        let fatal = evidence.error.is_some() || evidence.recovery.failed() || !evidence.disposed;
        if !fatal {
            report
                .survivors
                .retain(|i| !evidence.eliminated.contains(i));
        }
        report.trials.push(evidence);
        if fatal {
            report.reason =
                "trial or recovery failed; target quarantined/disposed, no promotion".into();
            break;
        }
    }
    report
}

/// Historical scalar-only comparator: +3/-4, two ticks, other tracks stable.
/// Recovery is measured but (like cloning/resetting the original fixture) is
/// not causal evidence. Do NOT integrate this baseline into a live backend.
pub fn fixed_deltas(
    factory: &mut dyn TrialFactory,
    baseline: &Frame,
    candidates: &[Candidate],
    scope: &Scope,
) -> Report {
    let mut report = Report::new(candidates);
    report.survivors.clear();
    for (i, c) in candidates.iter().enumerate() {
        let mut passed = true;
        for delta in [3.0, -4.0] {
            let Some(after) = c.encode_observation(baseline.tracks[scope.track] + delta) else {
                passed = false;
                continue;
            };
            let Some(before) = baseline.bytes.get(c.offset..c.offset + after.len()) else {
                passed = false;
                continue;
            };
            let Ok(mut target) = factory.open() else {
                passed = false;
                continue;
            };
            let patch = Patch {
                offset: c.offset,
                before: before.to_vec(),
                after,
            };
            let receipt = target.compare_write(baseline.epoch, &patch);
            report.interventions += 1;
            let mut observed = vec![];
            if receipt.error.is_some() {
                passed = false;
            } else {
                let _ = target.tick();
                match target.tick() {
                    Ok(frame) => {
                        passed &= valid_frame(&frame, baseline, scope)
                            && frame.tracks.iter().enumerate().all(|(j, value)| {
                                (value
                                    - baseline.tracks[j]
                                    - if j == scope.track { delta } else { 0.0 })
                                .abs()
                                    <= 0.6
                            });
                        observed.push(frame.tracks);
                    }
                    Err(_) => passed = false,
                }
            }
            let recovery = recover(
                &mut *target,
                baseline,
                scope,
                &Policy::default(),
                &patch,
                &receipt,
            );
            let disposed = target.dispose().is_ok();
            report.rollback_failures += usize::from(recovery.failed());
            report.disposal_failures += usize::from(!disposed);
            report.trials.push(TrialEvidence {
                candidate: i,
                desired: baseline.tracks[scope.track] + delta,
                patch,
                predicted: vec![],
                predicted_separations: 0,
                observed,
                eliminated: vec![],
                outside_patch_changes: vec![],
                unexpected_changes: vec![],
                recovery,
                disposed,
                error: receipt.error,
            });
        }
        if passed {
            report.survivors.push(i);
        }
    }
    if report.survivors.len() == 1 {
        report.selected = Some(report.survivors[0]);
        report.decision = Decision::IdentifiedWithinScope;
    }
    report.reason =
        "historical two fixed deltas; scalar-only, no authority/scope claim justified".into();
    report
}
