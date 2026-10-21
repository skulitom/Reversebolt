//! Replay exact agreed actions on disposable, exclusively owned targets only.
//! A successful observation is bounded by the supplied tracks and horizon.
use crate::planner::{Plan, plan};
use reversebolt_causal_validation::{discovery::Candidate, validation::*};
use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub enum Outcome {
    NotTested,
    Failed,
    ObservedWithinScope,
    ObservedWithUnknownEffects,
}

#[derive(Debug, Serialize)]
pub struct Attempt {
    pub error: Option<String>,
    pub wrote: bool,
    pub observations: Vec<Vec<f64>>,
    pub recovery: Recovery,
    pub disposed: bool,
}

#[derive(Debug, Serialize)]
pub struct Result {
    pub desired: f64,
    pub plan: Plan,
    pub outcome: Outcome,
    pub reason: String,
    pub attempts: Vec<Attempt>,
}

fn estimate(c: &Candidate, bytes: &[u8]) -> Option<f64> {
    let value = c.slope * c.kind.read(bytes.get(c.offset..)?)? + c.intercept;
    value.is_finite().then_some(value)
}

fn valid(frame: &Frame, baseline: &Frame, scope: &Scope) -> bool {
    frame.epoch == baseline.epoch
        && frame.bytes.len() == baseline.bytes.len()
        && frame.tracks.len() == baseline.tracks.len()
        && frame.tracks.len() == scope.tolerances.len()
        && frame.tracks.iter().all(|v| v.is_finite())
}

fn original(frame: &Frame, baseline: &Frame, scope: &Scope) -> bool {
    valid(frame, baseline, scope)
        && frame.bytes == baseline.bytes
        && frame
            .tracks
            .iter()
            .zip(&baseline.tracks)
            .zip(&scope.tolerances)
            .all(|((a, b), t)| (a - b).abs() <= *t)
}

fn restore(
    target: &mut dyn DisposableTarget,
    baseline: &Frame,
    scope: &Scope,
    policy: &Policy,
    patch: &Patch,
    receipt: &WriteReceipt,
) -> Recovery {
    let Ok(frame) = target.read() else {
        return Recovery::ReadFailed;
    };
    if frame.epoch != baseline.epoch {
        return Recovery::EpochChanged;
    }
    let range = patch.offset..patch.offset + patch.before.len();
    let Some(current) = frame.bytes.get(range.clone()) else {
        return Recovery::ReadFailed;
    };
    let already_original = current == patch.before;
    if !already_original {
        let Some(expected) = &receipt.expected_after else {
            return Recovery::UnknownWrite;
        };
        if current != expected {
            return Recovery::Conflict;
        }
        let undo = Patch {
            offset: patch.offset,
            before: expected.clone(),
            after: patch.before.clone(),
        };
        let undo_receipt = target.compare_write(baseline.epoch, &undo);
        if undo_receipt.error.is_some()
            || undo_receipt.expected_after.as_ref() != Some(&patch.before)
        {
            return Recovery::WriteFailed;
        }
    }
    for _ in 0..policy.recovery_ticks {
        match target.tick() {
            Ok(frame) if frame.epoch != baseline.epoch => return Recovery::EpochChanged,
            Ok(frame) if !valid(&frame, baseline, scope) => return Recovery::ReadFailed,
            Err(_) => return Recovery::ReadFailed,
            _ => {}
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
    if !original(&restored, baseline, scope) {
        return Recovery::EffectsRemain;
    }
    if already_original {
        Recovery::AlreadyOriginal
    } else {
        Recovery::Verified
    }
}

fn replay(
    factory: &mut dyn TrialFactory,
    baseline: &Frame,
    scope: &Scope,
    policy: &Policy,
    dependencies: &[Vec<Candidate>],
    patch: &Patch,
    desired: f64,
) -> Attempt {
    let mut result = Attempt {
        error: None,
        wrote: false,
        observations: vec![],
        recovery: Recovery::NoWrite,
        disposed: false,
    };
    let mut target = match factory.open() {
        Ok(target) => target,
        Err(error) => {
            result.error = Some(error);
            return result;
        }
    };
    let horizon = *policy.horizons.iter().max().unwrap();
    let preflight = (|| -> std::result::Result<(), String> {
        if !original(&target.read()?, baseline, scope) {
            return Err("baseline mismatch".into());
        }
        for _ in 0..horizon {
            if !original(&target.tick()?, baseline, scope) {
                return Err("sham drift or epoch change".into());
            }
        }
        Ok(())
    })();
    if let Err(error) = preflight {
        result.error = Some(error);
        result.disposed = target.dispose().is_ok();
        return result;
    }
    let receipt = target.compare_write(baseline.epoch, patch);
    result.wrote = true; // Attempted, including a denied or partial write.
    result.error = receipt.error.clone();
    if result.error.is_none() && receipt.expected_after.as_ref() != Some(&patch.after) {
        result.error = Some("unacknowledged exact write outcome".into());
    }
    if result.error.is_none() {
        for tick in 1..=horizon {
            let frame = match target.tick() {
                Ok(frame) if valid(&frame, baseline, scope) => frame,
                Ok(_) => {
                    result.error = Some("invalid frame or changed epoch".into());
                    break;
                }
                Err(error) => {
                    result.error = Some(error);
                    break;
                }
            };
            // Check other observed effects at EVERY tick, including between scoring horizons.
            if frame.tracks.iter().enumerate().any(|(i, v)| {
                if i == scope.track {
                    *v < scope.minimum || *v > scope.maximum
                } else {
                    (*v - baseline.tracks[i]).abs() > scope.tolerances[i]
                }
            }) {
                result.error = Some("effect outside requested track/domain".into());
            }
            if policy.horizons.contains(&tick) {
                result.observations.push(frame.tracks.clone());
                if (frame.tracks[scope.track] - desired).abs() > scope.tolerances[scope.track] {
                    result.error = Some("requested effect was not observed".into());
                }
            }
            // Retain the original research's byte-effect check. This cannot see
            // private state, and passive dependency models may themselves be wrong.
            for (offset, (&before, &after)) in baseline.bytes.iter().zip(&frame.bytes).enumerate() {
                if before == after
                    || (offset >= patch.offset && offset < patch.offset + patch.after.len())
                {
                    continue;
                }
                let explained = dependencies.iter().enumerate().any(|(track, models)| {
                    models.iter().any(|model| {
                        offset >= model.offset
                            && offset < model.offset + model.kind.width()
                            && estimate(model, &baseline.bytes).is_some_and(|v| {
                                (v - baseline.tracks[track]).abs() <= scope.tolerances[track]
                            })
                            && estimate(model, &frame.bytes).is_some_and(|v| {
                                (v - frame.tracks[track]).abs() <= scope.tolerances[track]
                            })
                            && (frame.tracks[track] - baseline.tracks[track]).abs()
                                > scope.tolerances[track]
                    })
                });
                if !explained {
                    result.error = Some("unexplained memory effect outside patch".into());
                }
            }
        }
    }
    result.recovery = restore(&mut *target, baseline, scope, policy, patch, &receipt);
    result.disposed = target.dispose().is_ok();
    result
}

/// Canonical validation is a prior experiment, not authority to write. Do not
/// resume experiments after any prior failed trial/recovery/disposal. Successful
/// prior probes retain every surviving interpretation, including shifted ones.
#[allow(clippy::too_many_arguments)]
pub fn test_request(
    factory: &mut dyn TrialFactory,
    baseline: &Frame,
    candidates: &[Candidate],
    dependencies: &[Vec<Candidate>],
    prior: &Report,
    scope: &Scope,
    policy: &Policy,
    desired: f64,
) -> Result {
    let models: Vec<_> = prior
        .survivors
        .iter()
        .filter_map(|&i| candidates.get(i).cloned())
        .collect();
    let mut result = Result {
        desired,
        plan: plan(
            &baseline.bytes,
            &models,
            baseline
                .tracks
                .get(scope.track)
                .copied()
                .unwrap_or(f64::NAN),
            desired,
            scope
                .tolerances
                .get(scope.track)
                .copied()
                .unwrap_or(f64::NAN),
        ),
        outcome: Outcome::NotTested,
        reason: String::new(),
        attempts: vec![],
    };
    if scope.track >= baseline.tracks.len()
        || !valid(baseline, baseline, scope)
        || dependencies.len() != baseline.tracks.len()
        || scope.tolerances.iter().any(|t| !t.is_finite() || *t <= 0.)
        || !scope.minimum.is_finite()
        || !scope.maximum.is_finite()
        || scope.minimum >= scope.maximum
        || policy.horizons.is_empty()
        || policy.horizons.contains(&0)
        || policy.recovery_ticks < policy.horizons.iter().copied().max().unwrap_or(1)
        || !policy.max_delta.is_finite()
        || policy.max_delta <= 0.
        || dependencies.iter().flatten().any(|c| {
            c.offset
                .checked_add(c.kind.width())
                .is_none_or(|end| end > baseline.bytes.len())
        })
        || models.len() != prior.survivors.len()
    {
        result.reason = "invalid scope, policy, or survivor identity".into();
        return result;
    }
    if prior.rollback_failures > 0
        || prior.disposal_failures > 0
        || prior
            .trials
            .iter()
            .any(|t| t.error.is_some() || t.recovery.failed() || !t.disposed)
        || prior.trials.len() < 2
    {
        result.reason = "prior evidence incomplete or a trial/recovery failed".into();
        return result;
    }
    let Some(agreed) = result.plan.agreement() else {
        result.reason = "interpretations do not specify one nonzero byte action".into();
        return result;
    };
    if desired < scope.minimum
        || desired > scope.maximum
        || (desired - baseline.tracks[scope.track]).abs() > policy.max_delta
        || (desired - baseline.tracks[scope.track]).abs() <= 2. * scope.tolerances[scope.track]
        || agreed.after.len() > policy.max_patch_bytes
        || !scope
            .writable
            .iter()
            .any(|r| agreed.offset >= r.start && agreed.offset + agreed.after.len() <= r.end)
    {
        result.reason = "request outside the declared action bounds".into();
        return result;
    }
    let patch = Patch {
        offset: agreed.offset,
        before: agreed.before.clone(),
        after: agreed.after.clone(),
    };
    for _ in 0..2 {
        let attempt = replay(
            factory,
            baseline,
            scope,
            policy,
            dependencies,
            &patch,
            desired,
        );
        let failed = attempt.error.is_some() || attempt.recovery.failed() || !attempt.disposed;
        result.attempts.push(attempt);
        if failed {
            result.outcome = Outcome::Failed;
            result.reason = "exact action failed observation or recovery; no promotion".into();
            return result;
        }
    }
    result.outcome = if scope.closed_effect_scope {
        Outcome::ObservedWithinScope
    } else {
        Outcome::ObservedWithUnknownEffects
    };
    result.reason = "same exact action replayed twice; original type remains unresolved; effects limited to supplied observations and horizon".into();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use reversebolt_causal_validation::discovery::Kind;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone, Copy, PartialEq)]
    enum Mode {
        Clean,
        NarrowSensitive,
        UnknownReceipt,
        UndoDenied,
        Conflict,
        Drift,
    }
    struct Factory {
        mode: Mode,
        opens: Arc<AtomicUsize>,
    }
    struct Target {
        mode: Mode,
        bytes: Vec<u8>,
        writes: usize,
        last: Option<Patch>,
        ticks: usize,
    }
    impl Factory {
        fn new(mode: Mode) -> Self {
            Self {
                mode,
                opens: Arc::default(),
            }
        }
    }
    impl TrialFactory for Factory {
        fn open(&mut self) -> std::result::Result<Box<dyn DisposableTarget>, String> {
            self.opens.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(Target {
                mode: self.mode,
                bytes: 50f32.to_le_bytes().to_vec(),
                writes: 0,
                last: None,
                ticks: 0,
            }))
        }
    }
    impl DisposableTarget for Target {
        fn read(&mut self) -> std::result::Result<Frame, String> {
            let raw = f32::from_le_bytes(self.bytes[..4].try_into().unwrap()) as f64;
            let side = if self.mode == Mode::NarrowSensitive
                && raw != 50.
                && self.last.as_ref().is_some_and(|p| p.after.len() < 4)
            {
                8.
            } else {
                7.
            };
            Ok(Frame {
                bytes: self.bytes.clone(),
                tracks: vec![raw, side],
                epoch: 1,
            })
        }
        fn tick(&mut self) -> std::result::Result<Frame, String> {
            self.ticks += 1;
            if self.mode == Mode::Conflict
                && self.writes == 1
                && let Some(patch) = &self.last
            {
                self.bytes[patch.offset] = patch.after[0] ^ 1;
            }
            let mut f = self.read()?;
            if self.mode == Mode::Drift {
                f.tracks[1] += self.ticks as f64;
            }
            Ok(f)
        }
        fn compare_write(&mut self, epoch: u64, patch: &Patch) -> WriteReceipt {
            if epoch != 1
                || self
                    .bytes
                    .get(patch.offset..patch.offset + patch.before.len())
                    != Some(&patch.before)
            {
                return WriteReceipt {
                    expected_after: None,
                    error: Some("conflict".into()),
                };
            }
            self.writes += 1;
            if self.mode == Mode::UndoDenied && self.writes > 1 {
                return WriteReceipt {
                    expected_after: Some(patch.before.clone()),
                    error: Some("undo denied".into()),
                };
            }
            self.bytes[patch.offset..patch.offset + patch.after.len()]
                .copy_from_slice(&patch.after);
            self.last = Some(patch.clone());
            WriteReceipt {
                expected_after: if self.mode == Mode::UnknownReceipt {
                    None
                } else {
                    Some(patch.after.clone())
                },
                error: None,
            }
        }
        fn dispose(&mut self) -> std::result::Result<(), String> {
            self.bytes.clear();
            Ok(())
        }
    }
    fn inputs() -> (
        Frame,
        Vec<Candidate>,
        Vec<Vec<Candidate>>,
        Scope,
        Policy,
        Report,
    ) {
        let base = Factory::new(Mode::Clean).open().unwrap().read().unwrap();
        let candidates = vec![Candidate {
            offset: 0,
            kind: Kind::F32,
            slope: 1.,
            intercept: 0.,
            train_inliers: 10,
            validation_inliers: 10,
            exact: true,
        }];
        let deps = vec![candidates.clone(), vec![]];
        let scope = Scope {
            writable: vec![ByteRange { start: 0, end: 4 }],
            track: 0,
            tolerances: vec![0.01; 2],
            minimum: 20.,
            maximum: 100.,
            closed_effect_scope: true,
            description: "owned test".into(),
        };
        let policy = Policy::default();
        let prior = validate(
            &mut Factory::new(Mode::Clean),
            &base,
            &candidates,
            &deps,
            &scope,
            &policy,
        );
        assert_eq!(prior.decision, Decision::IdentifiedWithinScope);
        (base, candidates, deps, scope, policy, prior)
    }
    #[test]
    fn same_bytes_are_not_evidence_for_a_different_write_footprint() {
        let (base, models, deps, scope, policy, prior) = inputs();
        assert!(prior.trials.iter().all(|t| t.patch.after.len() == 4));
        let result = test_request(
            &mut Factory::new(Mode::NarrowSensitive),
            &base,
            &models,
            &deps,
            &prior,
            &scope,
            &policy,
            55.,
        );
        assert_eq!(result.plan.agreement().unwrap().after.len(), 1);
        assert_eq!(result.outcome, Outcome::Failed);
        assert_eq!(result.attempts.len(), 1);
        assert!(
            result.attempts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("outside requested")
        );
        assert!(!result.attempts[0].recovery.failed());
    }
    #[test]
    fn exact_action_replays_restore_and_unknown_scope_stays_unknown() {
        let (base, models, deps, mut scope, policy, prior) = inputs();
        let mut factory = Factory::new(Mode::Clean);
        let result = test_request(
            &mut factory,
            &base,
            &models,
            &deps,
            &prior,
            &scope,
            &policy,
            55.,
        );
        assert_eq!(result.outcome, Outcome::ObservedWithinScope);
        assert_eq!(factory.opens.load(Ordering::SeqCst), 2);
        assert!(
            result
                .attempts
                .iter()
                .all(|a| a.recovery == Recovery::Verified && a.disposed)
        );
        scope.closed_effect_scope = false;
        let result = test_request(
            &mut factory,
            &base,
            &models,
            &deps,
            &prior,
            &scope,
            &policy,
            55.,
        );
        assert_eq!(result.outcome, Outcome::ObservedWithUnknownEffects);
    }
    #[test]
    fn unknown_receipt_conflict_and_denied_undo_stop_after_one_trial() {
        let (base, models, deps, scope, policy, prior) = inputs();
        for (mode, recovery) in [
            (Mode::UnknownReceipt, Recovery::UnknownWrite),
            (Mode::UndoDenied, Recovery::WriteFailed),
            (Mode::Conflict, Recovery::Conflict),
        ] {
            let mut factory = Factory::new(mode);
            let result = test_request(
                &mut factory,
                &base,
                &models,
                &deps,
                &prior,
                &scope,
                &policy,
                55.,
            );
            assert_eq!(result.outcome, Outcome::Failed);
            assert_eq!(result.attempts[0].recovery, recovery);
            assert!(result.attempts[0].disposed);
            assert_eq!(factory.opens.load(Ordering::SeqCst), 1);
        }
    }
    #[test]
    fn incomplete_evidence_and_sham_drift_do_not_write() {
        let (base, models, deps, scope, policy, mut prior) = inputs();
        let mut factory = Factory::new(Mode::Drift);
        let result = test_request(
            &mut factory,
            &base,
            &models,
            &deps,
            &prior,
            &scope,
            &policy,
            55.,
        );
        assert_eq!(result.outcome, Outcome::Failed);
        assert!(!result.attempts[0].wrote);
        assert!(result.attempts[0].disposed);
        prior.rollback_failures = 1;
        let mut factory = Factory::new(Mode::Clean);
        let result = test_request(
            &mut factory,
            &base,
            &models,
            &deps,
            &prior,
            &scope,
            &policy,
            55.,
        );
        assert_eq!(result.outcome, Outcome::NotTested);
        assert_eq!(factory.opens.load(Ordering::SeqCst), 0);
    }
}
