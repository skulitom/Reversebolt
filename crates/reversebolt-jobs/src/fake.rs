//! Instrumented deterministic control-flow executor and storage faults.
//! These observations/effects are NOT game-discovery evidence.
use crate::{journal::Journal, *};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug)]
pub enum StorageFault {
    BeforeWrite,
    AfterDurableWrite,
}
#[derive(Default)]
struct Memory {
    last: Option<Snapshot>,
    fault: Option<StorageFault>,
    writes: usize,
}
#[derive(Clone, Default)]
pub struct MemoryJournal(Arc<Mutex<Memory>>);
impl MemoryJournal {
    pub fn fail_next(&self, fault: StorageFault) {
        self.0.lock().unwrap().fault = Some(fault);
    }
    pub fn writes(&self) -> usize {
        self.0.lock().unwrap().writes
    }
}
impl Journal for MemoryJournal {
    fn load(&self) -> Option<Snapshot> {
        self.0.lock().unwrap().last.clone()
    }
    fn append(&mut self, state: &Snapshot, _reserve: usize) -> Result<(), String> {
        state.validate()?;
        if serde_json::to_vec(state).unwrap().len() > crate::journal::MAX_RECORD {
            return Err("fake record budget".into());
        }
        let mut memory = self.0.lock().unwrap();
        if memory.writes >= 4096 {
            return Err("fake journal write budget".into());
        }
        let fault = memory.fault.take();
        if matches!(fault, Some(StorageFault::BeforeWrite)) {
            return Err("injected failed write".into());
        }
        if state.revision != memory.last.as_ref().map_or(1, |s| s.revision + 1) {
            return Err("fake revision mismatch".into());
        }
        memory.last = Some(state.clone());
        memory.writes += 1;
        if matches!(fault, Some(StorageFault::AfterDurableWrite)) {
            return Err("injected lost durability acknowledgement".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Behavior {
    pub delay: u32,
    pub apply_on_begin: bool,
    pub ignore_cancel: bool,
    pub fail_before: bool,
    pub fail_after: bool,
    pub result_bytes: usize,
}
impl Default for Behavior {
    fn default() -> Self {
        Self {
            delay: 0,
            apply_on_begin: true,
            ignore_cancel: true,
            fail_before: false,
            fail_after: false,
            result_bytes: 8,
        }
    }
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct Effect {
    pub operation: u64,
    pub binding: Binding,
}
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Audit {
    pub begins: Vec<Operation>,
    pub effects: Vec<Effect>,
    pub polls: usize,
    pub cancellation_seen: usize,
    pub intent_violations: usize,
}
struct Pending {
    op: Operation,
    behavior: Behavior,
    remaining: u32,
    applied: bool,
}
#[derive(Default)]
pub struct FakeExecutor {
    pub current: Option<Binding>,
    pub dead_targets: BTreeSet<String>,
    pub behaviors: BTreeMap<String, Behavior>,
    pub audit: Audit,
    pub intent_witness: Option<MemoryJournal>,
    pending: Option<Pending>,
}
impl FakeExecutor {
    pub fn attach(&mut self, binding: Binding) {
        self.current = Some(binding);
    }
    fn apply(&mut self, p: &mut Pending) {
        if p.op.instruction.kind == Kind::Effect && !p.applied {
            self.audit.effects.push(Effect {
                operation: p.op.id,
                binding: p.op.binding.clone(),
            });
            p.applied = true;
        }
    }
    fn result(&self, p: Pending, failed: bool, note: &str) -> Poll {
        Poll::Ready(Outcome {
            success: !failed,
            effect: if p.op.instruction.kind == Kind::Observe {
                Knowledge::NotApplicable
            } else if p.applied {
                Knowledge::Applied
            } else {
                Knowledge::NotApplied
            },
            data: if failed {
                vec![]
            } else {
                vec![42; p.behavior.result_bytes.min(1024 * 1024)]
            },
            note: note.into(),
        })
    }
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
}
impl Executor for FakeExecutor {
    fn begin(&mut self, op: &Operation) -> Poll {
        assert!(
            self.pending.is_none(),
            "core dispatched concurrent executor work"
        );
        assert!(self.audit.begins.len() < 4096, "fake audit budget");
        if let Some(journal) = &self.intent_witness {
            let valid = journal
                .load()
                .and_then(|s| s.jobs.get(&op.job).and_then(|j| j.pending.clone()))
                .as_ref()
                == Some(op);
            if !valid {
                self.audit.intent_violations += 1;
            }
        }
        self.audit.begins.push(op.clone());
        let behavior = self
            .behaviors
            .get(&op.instruction.payload)
            .cloned()
            .unwrap_or_default();
        let delay = behavior.delay;
        let mut p = Pending {
            op: op.clone(),
            behavior,
            remaining: delay,
            applied: false,
        };
        if self.current.as_ref() != Some(&op.binding)
            || self.dead_targets.contains(&op.binding.target)
            || p.behavior.fail_before
        {
            return self.result(
                p,
                true,
                "not applied: target mismatch/exit or injected rejection",
            );
        }
        if p.behavior.apply_on_begin {
            self.apply(&mut p);
        }
        if p.remaining == 0 {
            self.apply(&mut p);
            let failed = p.behavior.fail_after;
            self.result(p, failed, "fake executor quiescent")
        } else {
            self.pending = Some(p);
            Poll::Pending
        }
    }
    fn poll(&mut self, id: u64, cancel: bool) -> Poll {
        self.audit.polls += 1;
        if cancel {
            self.audit.cancellation_seen += 1;
        }
        let mut p = self.pending.take().expect("poll without pending operation");
        assert_eq!(id, p.op.id);
        if self.dead_targets.contains(&p.op.binding.target) || (cancel && !p.behavior.ignore_cancel)
        {
            return self.result(p, true, "quiescent after target exit/cancellation");
        }
        p.remaining = p.remaining.saturating_sub(1);
        if p.remaining > 0 {
            self.pending = Some(p);
            return Poll::Pending;
        }
        self.apply(&mut p);
        let failed = p.behavior.fail_after;
        self.result(p, failed, "fake executor quiescent")
    }
}
