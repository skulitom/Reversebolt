//! Fixture code and private truth are linked only into the evaluator binary.
use reversebolt_action_equivalence::action::{self, Outcome};
use reversebolt_causal_validation::{
    discover_bytewise,
    discovery::{Candidate, Kind},
    validation::*,
};
use serde::Serialize;

#[path = "fixtures/inherited.rs"]
mod inherited;
#[path = "fixtures/withheld.rs"]
pub mod withheld;

#[derive(Serialize)]
pub struct Audit {
    pub effects_ok: bool,
    pub restored: bool,
    pub hidden_effect: bool,
}

#[derive(Serialize)]
pub struct Run {
    pub suite: String,
    pub scenario: String,
    pub seed: u64,
    pub channel: usize,
    pub candidates: Vec<Candidate>,
    pub prior: Report,
    pub actions: Vec<action::Result>,
    pub scope: Scope,
    pub truth_after_answer: Option<(usize, Kind)>,
    pub hidden_prior: usize,
    pub hidden_actions: usize,
    pub audits: Vec<Option<Audit>>,
}

pub fn infer(
    factory: &mut dyn TrialFactory,
    action_factory: &mut dyn TrialFactory,
    baseline: &Frame,
    dependencies: &[Vec<Candidate>],
    scope: &Scope,
) -> (Report, Vec<action::Result>) {
    let candidates = &dependencies[scope.track];
    let policy = Policy::default();
    let prior = validate(factory, baseline, candidates, dependencies, scope, &policy);
    let mut actions: Vec<action::Result> = vec![];
    for delta in [3., -4.] {
        let desired = baseline.tracks[scope.track] + delta;
        if actions.iter().any(|a| a.outcome == Outcome::Failed) {
            let models: Vec<_> = prior
                .survivors
                .iter()
                .map(|&i| candidates[i].clone())
                .collect();
            actions.push(action::Result {
                desired,
                plan: reversebolt_action_equivalence::planner::plan(
                    &baseline.bytes,
                    &models,
                    baseline.tracks[scope.track],
                    desired,
                    scope.tolerances[scope.track],
                ),
                outcome: Outcome::NotTested,
                reason: "earlier exact-action trial failed; no further trials".into(),
                attempts: vec![],
            });
        } else {
            actions.push(action::test_request(
                action_factory,
                baseline,
                candidates,
                dependencies,
                &prior,
                scope,
                &policy,
                desired,
            ));
        }
    }
    (prior, actions)
}

/// Post-answer audit extends the observation horizon. It does not feed back
/// into inference. A separate private-state audit in withheld.rs adds hidden effects.
pub fn audit_after_answer(
    target: &mut dyn DisposableTarget,
    baseline: &Frame,
    scope: &Scope,
    result: &action::Result,
) -> Option<Audit> {
    if !matches!(
        result.outcome,
        Outcome::ObservedWithinScope | Outcome::ObservedWithUnknownEffects
    ) {
        return None;
    }
    let agreed = result.plan.agreement().unwrap();
    let patch = Patch {
        offset: agreed.offset,
        before: agreed.before.clone(),
        after: agreed.after.clone(),
    };
    let receipt = target.compare_write(baseline.epoch, &patch);
    let mut effects_ok = receipt.error.is_none();
    for _ in 0..32 {
        effects_ok &= target.tick().is_ok_and(|f| {
            f.epoch == baseline.epoch
                && f.tracks.len() == baseline.tracks.len()
                && f.tracks.iter().enumerate().all(|(i, v)| {
                    v.is_finite()
                        && (v - if i == scope.track {
                            result.desired
                        } else {
                            baseline.tracks[i]
                        })
                        .abs()
                            <= scope.tolerances[i]
                })
        });
    }
    let undo = Patch {
        offset: patch.offset,
        before: patch.after,
        after: patch.before,
    };
    let mut restored = target.compare_write(baseline.epoch, &undo).error.is_none();
    for _ in 0..32 {
        restored &= target.tick().is_ok();
    }
    restored &= target.read().is_ok_and(|f| {
        f.epoch == baseline.epoch
            && f.bytes == baseline.bytes
            && f.tracks.len() == baseline.tracks.len()
            && f.tracks
                .iter()
                .zip(&baseline.tracks)
                .zip(&scope.tolerances)
                .all(|((a, b), t)| a.is_finite() && (a - b).abs() <= *t)
    });
    Some(Audit {
        effects_ok,
        restored,
        hidden_effect: false,
    })
}

#[derive(Clone)]
struct Factory(inherited::World);
struct Target {
    world: inherited::World,
    disposed: bool,
}
impl TrialFactory for Factory {
    fn open(&mut self) -> Result<Box<dyn DisposableTarget>, String> {
        Ok(Box::new(Target {
            world: self.0.clone(),
            disposed: false,
        }))
    }
}
impl DisposableTarget for Target {
    fn read(&mut self) -> Result<Frame, String> {
        if self.disposed {
            return Err("disposed".into());
        }
        Ok(Frame {
            bytes: self.world.bytes.clone(),
            tracks: self.world.observe().to_vec(),
            epoch: 1,
        })
    }
    fn tick(&mut self) -> Result<Frame, String> {
        self.world.idle();
        self.read()
    }
    fn compare_write(&mut self, epoch: u64, patch: &Patch) -> WriteReceipt {
        let destination = self
            .world
            .bytes
            .get_mut(patch.offset..patch.offset + patch.before.len());
        if self.disposed
            || epoch != 1
            || destination.as_deref() != Some(patch.before.as_slice())
            || patch.before.len() != patch.after.len()
        {
            return WriteReceipt {
                expected_after: None,
                error: Some("compare/write conflict".into()),
            };
        }
        destination.unwrap().copy_from_slice(&patch.after);
        WriteReceipt {
            expected_after: Some(patch.after.clone()),
            error: None,
        }
    }
    fn dispose(&mut self) -> Result<(), String> {
        self.disposed = true;
        self.world.bytes.clear();
        Ok(())
    }
}

pub fn inherited_suite(count: usize, start: u64) -> Vec<Run> {
    let mut results = vec![];
    for scenario in inherited::Scenario::ALL {
        for seed in start..start + count as u64 {
            let mut world = inherited::World::new(seed, scenario);
            let mut snapshots = vec![];
            let mut tracks = [vec![], vec![]];
            for _ in 0..inherited::FRAMES {
                let visible = world.advance();
                snapshots.push(world.bytes.clone());
                for channel in 0..2 {
                    tracks[channel].push(visible[channel]);
                }
            }
            world.idle();
            let baseline = Frame {
                bytes: world.bytes.clone(),
                tracks: world.observe().to_vec(),
                epoch: 1,
            };
            let dependencies: Vec<_> = tracks
                .iter()
                .map(|t| discover_bytewise(&snapshots, t))
                .collect();
            for channel in 0..2 {
                let scope = Scope {
                    writable: vec![ByteRange {
                        start: 0,
                        end: baseline.bytes.len(),
                    }],
                    track: channel,
                    tolerances: vec![0.6; 2],
                    minimum: tracks[channel]
                        .iter()
                        .flatten()
                        .copied()
                        .fold(f64::INFINITY, f64::min)
                        - 20.,
                    maximum: tracks[channel]
                        .iter()
                        .flatten()
                        .copied()
                        .fold(f64::NEG_INFINITY, f64::max)
                        + 20.,
                    closed_effect_scope: true,
                    description:
                        "Owned synthetic arena; both scalar tracks and all bytes through tick 8"
                            .into(),
                };
                let (prior, actions) = infer(
                    &mut Factory(world.clone()),
                    &mut Factory(world.clone()),
                    &baseline,
                    &dependencies,
                    &scope,
                );
                let truth = Some(world.authority(channel)); // First authority access, after answers.
                let audits = actions
                    .iter()
                    .map(|a| {
                        let mut target = Factory(world.clone()).open().unwrap();
                        let audit = audit_after_answer(&mut *target, &baseline, &scope, a);
                        target.dispose().unwrap();
                        audit
                    })
                    .collect();
                results.push(Run {
                    suite: "inherited".into(),
                    scenario: format!("{scenario:?}"),
                    seed,
                    channel,
                    candidates: dependencies[channel].clone(),
                    prior,
                    actions,
                    scope,
                    truth_after_answer: truth,
                    hidden_prior: 0,
                    hidden_actions: 0,
                    audits,
                });
            }
        }
        eprintln!("completed inherited {scenario:?}");
    }
    results
}
