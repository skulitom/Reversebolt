//! New targets authored after the policy freeze. No imports from this module
//! exist in the inference library. This implementation does not reuse World.
use super::{Run, audit_after_answer, infer};
use reversebolt_causal_validation::{discover_bytewise, discovery::Kind, validation::*};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Case {
    FloatBoundary,
    DoubleBoundary,
    NegativeFloat,
    NarrowFloat,
    NarrowBits,
    TelemetryOnly,
    LaggedDisplay,
    MediatedPair,
    CoupledEffect,
    DelayedEffect,
    TransientEffect,
    UnmodelledByte,
    IrreversibleEffect,
    HiddenClean,
    HiddenLatent,
    HiddenImmediate,
    LocalNonlinear,
    Clamp,
    WriteDenied,
    PartialWrite,
    UnknownPartial,
    ReadFailure,
    RollbackDenied,
    ConcurrentWrite,
    EpochChange,
    NonfiniteObservation,
    MissingTrack,
    DisposalFailure,
    ShamDrift,
    NoWriteScope,
}

impl Case {
    pub const ALL: [Self; 30] = [
        Self::FloatBoundary,
        Self::DoubleBoundary,
        Self::NegativeFloat,
        Self::NarrowFloat,
        Self::NarrowBits,
        Self::TelemetryOnly,
        Self::LaggedDisplay,
        Self::MediatedPair,
        Self::CoupledEffect,
        Self::DelayedEffect,
        Self::TransientEffect,
        Self::UnmodelledByte,
        Self::IrreversibleEffect,
        Self::HiddenClean,
        Self::HiddenLatent,
        Self::HiddenImmediate,
        Self::LocalNonlinear,
        Self::Clamp,
        Self::WriteDenied,
        Self::PartialWrite,
        Self::UnknownPartial,
        Self::ReadFailure,
        Self::RollbackDenied,
        Self::ConcurrentWrite,
        Self::EpochChange,
        Self::NonfiniteObservation,
        Self::MissingTrack,
        Self::DisposalFailure,
        Self::ShamDrift,
        Self::NoWriteScope,
    ];
}

#[derive(Clone)]
struct Target {
    case: Case,
    bytes: Vec<u8>,
    primary: usize,
    mirror: usize,
    scale: f64,
    bias: f64,
    baseline_raw: f64,
    last_raw: f64,
    elapsed: usize,
    since_write: usize,
    writes: usize,
    epoch: u64,
    disposed: bool,
    latent_event: bool,
    residual: bool,
    last_patch: Option<usize>,
    private_audit: Arc<Mutex<Vec<bool>>>,
}

impl Target {
    fn kind(&self) -> Kind {
        if self.case == Case::DoubleBoundary {
            Kind::F64
        } else {
            Kind::F32
        }
    }
    fn load(&self, at: usize) -> f64 {
        if self.case == Case::NarrowBits && at == self.primary {
            let bits = i32::from_le_bytes(self.bytes[at..at + 4].try_into().unwrap());
            // Exactly the same function as f32 decoding on the bounded bin.
            32.0 + (bits as f64 - 1107296256.0) / 262144.0
        } else {
            self.kind().read(&self.bytes[at..]).unwrap_or(f64::NAN)
        }
    }
    fn store(&mut self, at: usize, raw: f64) {
        let bytes = self.kind().encode(raw).unwrap();
        self.bytes[at..at + bytes.len()].copy_from_slice(&bytes);
    }
    fn visible(&self, raw: f64) -> f64 {
        if self.case == Case::LocalNonlinear && (raw - 60.0).abs() > 4.0 {
            60.0 + (raw - 60.0).signum() * (raw - 60.0).abs().sqrt() * 2.0
        } else {
            raw * self.scale + self.bias
        }
    }
    fn frame(&self) -> Frame {
        let raw = if self.case == Case::TelemetryOnly {
            self.baseline_raw
        } else if self.case == Case::LaggedDisplay && self.writes > 0 && self.since_write <= 2 {
            self.load(self.mirror)
        } else {
            self.load(self.primary)
        };
        let changed = (self.load(self.primary) - self.baseline_raw).abs() > 0.01;
        let side = match self.case {
            Case::CoupledEffect if changed => 4.0,
            Case::DelayedEffect if changed && self.since_write >= 5 => 4.0,
            Case::TransientEffect if changed && self.since_write == 3 => 4.0,
            Case::IrreversibleEffect if self.residual => 4.0,
            Case::ShamDrift => self.elapsed as f64,
            _ => 0.0,
        };
        let mut tracks = vec![self.visible(raw), 7.0 + side];
        if self.writes > 0 {
            if self.case == Case::NonfiniteObservation {
                tracks[0] = f64::NAN;
            }
            if self.case == Case::MissingTrack {
                tracks.pop();
            }
        }
        Frame {
            bytes: self.bytes.clone(),
            tracks,
            epoch: self.epoch,
        }
    }
    fn authority_after_answer(&self) -> Option<(usize, Kind)> {
        if self.case == Case::TelemetryOnly {
            None
        } else {
            Some((
                self.primary,
                if self.case == Case::NarrowBits {
                    Kind::I32
                } else {
                    self.kind()
                },
            ))
        }
    }
}

impl DisposableTarget for Target {
    fn read(&mut self) -> Result<Frame, String> {
        if self.disposed {
            return Err("disposed".into());
        }
        if self.case == Case::ReadFailure && self.writes > 0 {
            return Err("injected read failure".into());
        }
        Ok(self.frame())
    }
    fn tick(&mut self) -> Result<Frame, String> {
        self.elapsed += 1;
        if self.writes > 0 {
            self.since_write += 1;
            if self.since_write == 1 {
                if self.case == Case::ConcurrentWrite {
                    self.bytes[self.last_patch.unwrap()] ^= 1;
                }
                if self.case == Case::EpochChange {
                    self.epoch += 1;
                }
            }
        }
        let mut raw = self.load(self.primary);
        if self.case == Case::TelemetryOnly {
            raw = self.baseline_raw;
            self.store(self.primary, raw);
        }
        if self.case == Case::MediatedPair {
            if (raw - self.last_raw).abs() <= 1e-9 {
                raw = self.load(self.mirror);
            }
            self.store(self.primary, raw);
        }
        if self.case == Case::Clamp {
            raw = raw.clamp(0.0, 64.0);
            self.store(self.primary, raw);
        }
        if self.case != Case::LaggedDisplay || self.since_write > 2 || self.writes == 0 {
            self.store(self.mirror, raw);
        }
        let changed = (raw - self.baseline_raw).abs() > 0.01;
        if self.case == Case::UnmodelledByte {
            self.bytes[255] = if changed { 199 } else { 17 };
        }
        if self.case == Case::IrreversibleEffect && changed {
            self.residual = true;
            self.bytes[255] = 199;
        }
        if self.case == Case::HiddenLatent && changed && self.since_write >= 16 {
            self.latent_event = true; // Deliberately outside every supplied observation.
        }
        if self.case == Case::HiddenImmediate && changed {
            self.latent_event = true; // An irreversible private effect, never in Frame.
        }
        self.last_raw = raw;
        self.read()
    }
    fn compare_write(&mut self, epoch: u64, patch: &Patch) -> WriteReceipt {
        if self.disposed
            || epoch != self.epoch
            || patch.before.len() != patch.after.len()
            || self
                .bytes
                .get(patch.offset..patch.offset + patch.before.len())
                != Some(patch.before.as_slice())
        {
            return WriteReceipt {
                expected_after: None,
                error: Some("CAS conflict".into()),
            };
        }
        self.writes += 1;
        self.since_write = 0;
        self.last_patch = Some(patch.offset);
        if self.case == Case::WriteDenied || (self.case == Case::RollbackDenied && self.writes >= 2)
        {
            return WriteReceipt {
                expected_after: Some(patch.before.clone()),
                error: Some("injected denied write".into()),
            };
        }
        if self.writes == 1 && matches!(self.case, Case::PartialWrite | Case::UnknownPartial) {
            // At least one changed byte is applied; partial outcome is observable.
            let index = patch
                .before
                .iter()
                .zip(&patch.after)
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            self.bytes[patch.offset + index] = patch.after[index];
            return WriteReceipt {
                expected_after: (self.case == Case::PartialWrite)
                    .then(|| self.bytes[patch.offset..patch.offset + patch.after.len()].to_vec()),
                error: Some("injected partial write".into()),
            };
        }
        self.bytes[patch.offset..patch.offset + patch.after.len()].copy_from_slice(&patch.after);
        WriteReceipt {
            expected_after: Some(patch.after.clone()),
            error: None,
        }
    }
    fn dispose(&mut self) -> Result<(), String> {
        // Private audit is recorded but only read by the evaluator AFTER answer.
        self.private_audit.lock().unwrap().push(self.latent_event);
        self.bytes.clear();
        self.disposed = true;
        if self.case == Case::DisposalFailure {
            Err("injected destruction acknowledgement failure (owned buffer still dropped)".into())
        } else {
            Ok(())
        }
    }
}

struct Factory(Target);
impl Factory {
    fn new(target: &Target) -> Self {
        let mut target = target.clone();
        target.private_audit = Arc::default();
        Self(target)
    }
    fn hidden_effects_after_answer(&self) -> usize {
        self.0
            .private_audit
            .lock()
            .unwrap()
            .iter()
            .filter(|&&effect| effect)
            .count()
    }
}
impl TrialFactory for Factory {
    fn open(&mut self) -> Result<Box<dyn DisposableTarget>, String> {
        Ok(Box::new(self.0.clone()))
    }
}

struct Inputs {
    target: Target,
    snapshots: Vec<Vec<u8>>,
    tracks: Vec<Vec<Option<f64>>>,
    scope: Scope,
}

fn inputs(case: Case, seed: u64) -> Inputs {
    let mut state = seed;
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state
    };
    let bytes = (0..256).map(|_| (next() >> 32) as u8).collect();
    let primary = (next() % 12) as usize * 8;
    let mirror = 128 + (next() % 12) as usize * 8;
    let scale = if case == Case::DoubleBoundary {
        100.0
    } else if case == Case::NegativeFloat {
        -1.0
    } else if case == Case::FloatBoundary {
        0.75 + (next() % 5) as f64 * 0.125
    } else {
        1.0
    };
    let bias = if case == Case::FloatBoundary {
        (next() % 17) as f64
    } else {
        0.0
    };
    let baseline_raw = if case == Case::DoubleBoundary {
        1.96
    } else {
        60.0
    };
    let mut target = Target {
        case,
        bytes,
        primary,
        mirror,
        scale,
        bias,
        baseline_raw,
        last_raw: baseline_raw,
        elapsed: 0,
        since_write: 0,
        writes: 0,
        epoch: 11,
        disposed: false,
        latent_event: false,
        residual: false,
        last_patch: None,
        private_audit: Arc::default(),
    };
    let mut snapshots = vec![];
    let mut tracks = vec![vec![], vec![]];
    for _ in 0..24 {
        let raw = if case == Case::DoubleBoundary {
            1.6 + (next() % 39) as f64 * 0.01
        } else if case == Case::LocalNonlinear {
            57.0 + (next() % 7) as f64
        } else {
            40.0 + (next() % 24) as f64
        };
        target.store(primary, raw);
        target.store(mirror, raw);
        let observation = target.visible(raw);
        snapshots.push(target.bytes.clone());
        tracks[0].push(Some(observation));
        tracks[1].push(Some(7.0));
    }
    target.store(primary, baseline_raw);
    target.store(mirror, baseline_raw);
    if matches!(case, Case::UnmodelledByte | Case::IrreversibleEffect) {
        target.bytes[255] = 17;
    }
    let base_value = target.visible(baseline_raw);
    let narrow = matches!(case, Case::NarrowFloat | Case::NarrowBits);
    let scope = Scope {
        writable: if case == Case::NoWriteScope {
            vec![]
        } else {
            vec![ByteRange { start: 0, end: 256 }]
        },
        track: 0,
        tolerances: vec![0.6; 2],
        minimum: if narrow { 40.0 } else { base_value - 20.0 },
        maximum: if narrow { 63.0 } else { base_value + 20.0 },
        closed_effect_scope: !matches!(
            case,
            Case::HiddenClean | Case::HiddenLatent | Case::HiddenImmediate
        ),
        description: if matches!(
            case,
            Case::HiddenClean | Case::HiddenLatent | Case::HiddenImmediate
        ) {
            "Owned target; only bytes and two displayed tracks observed; hidden consequences not covered".into()
        } else {
            "Owned deterministic synthetic target; bytes and both tracks in a finite eight-tick scope".into()
        },
    };
    Inputs {
        target,
        snapshots,
        tracks,
        scope,
    }
}

pub fn action_suite(count: usize, start: u64) -> Vec<Run> {
    let mut results = vec![];
    for case in Case::ALL {
        for seed in start..start + count as u64 {
            let Inputs {
                target,
                snapshots,
                tracks,
                scope,
            } = inputs(case, seed);
            let baseline = target.frame();
            let dependencies: Vec<_> = tracks
                .iter()
                .map(|t| discover_bytewise(&snapshots, t))
                .collect();
            let mut prior_factory = Factory::new(&target);
            let mut action_factory = Factory::new(&target);
            let (prior, actions) = infer(
                &mut prior_factory,
                &mut action_factory,
                &baseline,
                &dependencies,
                &scope,
            );
            // Private truth and audits are consulted only after all inference.
            let truth = target.authority_after_answer();
            let audits = actions
                .iter()
                .map(|a| {
                    let mut replay = target.clone();
                    replay.private_audit = Arc::default();
                    let mut audit = audit_after_answer(&mut replay, &baseline, &scope, a);
                    if let Some(audit) = &mut audit {
                        audit.hidden_effect = replay.latent_event;
                    }
                    let _ = replay.dispose();
                    audit
                })
                .collect();
            results.push(Run {
                suite: "previously_seen_withheld".into(),
                scenario: format!("{case:?}"),
                seed,
                channel: 0,
                candidates: dependencies[0].clone(),
                prior,
                actions,
                scope,
                truth_after_answer: truth,
                hidden_prior: prior_factory.hidden_effects_after_answer(),
                hidden_actions: action_factory.hidden_effects_after_answer(),
                audits,
            });
        }
        eprintln!("completed previously-seen {case:?}");
    }
    results
}
