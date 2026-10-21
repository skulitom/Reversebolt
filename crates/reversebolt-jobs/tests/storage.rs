use reversebolt_session_jobs::{
    fake::{FakeExecutor, MemoryJournal},
    journal::{FileJournal, Journal, MAX_LOG, MAX_RECORD, encode},
    *,
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

fn directory(name: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let parent = Path::new(env!("CARGO_MANIFEST_DIR")).join("results/test-journals");
    fs::create_dir_all(&parent).unwrap();
    // Windows can reuse a PID from an earlier test run. Never reopen its journal.
    loop {
        let path = parent.join(format!(
            "{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            name
        ));
        match fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("Could not create an isolated test journal: {error}"),
        }
    }
}

fn command<J: Journal>(j: &Jobs<J>, action: Action) -> Command {
    Command {
        request: j
            .snapshot()
            .clients
            .get("desktop")
            .map_or(1, |c| c.high_water + 1),
        actor: Actor {
            client: "desktop".into(),
            role: Role::Human,
        },
        boot: j.snapshot().boot,
        binding: j.snapshot().current.clone().unwrap(),
        action,
    }
}
fn plan() -> Action {
    Action::Submit {
        steps: vec![Step {
            kind: Kind::Effect,
            payload: "one-effect".into(),
        }],
        consent: true,
    }
}
fn checkpoints() -> Vec<Snapshot> {
    let mut j = Jobs::open(MemoryJournal::default(), "files", Limits::default()).unwrap();
    let mut states = vec![j.snapshot().clone()];
    let binding = j.bind(Some("disposable:created:100")).unwrap();
    states.push(j.snapshot().clone());
    j.command(command(&j, plan())).unwrap();
    states.push(j.snapshot().clone());
    let mut e = FakeExecutor::default();
    e.current = binding;
    for _ in 0..3 {
        j.tick(&mut e).unwrap();
        states.push(j.snapshot().clone());
    }
    assert_eq!(e.audit.effects.len(), 1);
    states
}
fn records(states: &[Snapshot]) -> Vec<u8> {
    states.iter().flat_map(|s| encode(s).unwrap()).collect()
}
fn unchecked_record(state: &Snapshot) -> Vec<u8> {
    raw_record(state.revision, &serde_json::to_vec(state).unwrap())
}
fn raw_record(revision: u64, bytes: &[u8]) -> Vec<u8> {
    let mut out = b"RBJOB001".to_vec();
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&revision.to_le_bytes());
    out.extend_from_slice(&crc32fast::hash(&out).to_le_bytes());
    out.extend_from_slice(&crc32fast::hash(bytes).to_le_bytes());
    out.extend_from_slice(bytes);
    out.extend_from_slice(b"COMMIT01");
    out
}

#[test]
fn exclusive_owner_and_durable_reopen_preserve_receipts_and_cursors() {
    let dir = directory("ownership");
    let mut j = Jobs::open(FileJournal::open(&dir).unwrap(), "files", Limits::default()).unwrap();
    assert!(FileJournal::open(&dir).is_err());
    let mut e = FakeExecutor::default();
    e.current = j.bind(Some("disposable:1")).unwrap();
    let c = command(&j, plan());
    let ack = j.command(c.clone()).unwrap().ack;
    for _ in 0..3 {
        j.tick(&mut e).unwrap();
    }
    let head = j.snapshot().event_head;
    drop(j);
    let mut recovered =
        Jobs::open(FileJournal::open(&dir).unwrap(), "files", Limits::default()).unwrap();
    assert_eq!(recovered.command(c).unwrap().ack, ack);
    assert!(recovered.snapshot().event_head > head);
    assert_eq!(recovered.snapshot().jobs[&1].phase, Phase::Completed);
    assert!(!recovered.snapshot().jobs[&1].evidence_valid);
    assert_eq!(recovered.tick(&mut e).unwrap(), Progress::Idle);
    assert_eq!(e.audit.effects.len(), 1);
}

#[test]
fn every_truncation_of_outcome_record_recovers_unknown_without_replay() {
    let states = checkpoints();
    let prefix = records(&states[..5]);
    let outcome = encode(&states[5]).unwrap();
    let dir = directory("every-tail-byte");
    for cut in 0..=outcome.len() {
        let mut bytes = prefix.clone();
        bytes.extend_from_slice(&outcome[..cut]);
        fs::write(dir.join("jobs.wal"), bytes).unwrap();
        let storage = FileJournal::open(&dir).unwrap();
        assert_eq!(
            storage.repaired_tail_bytes,
            if cut == outcome.len() { 0 } else { cut as u64 }
        );
        let mut j = Jobs::open(storage, "files", Limits::default()).unwrap();
        let mut e = FakeExecutor::default();
        if cut == outcome.len() {
            assert_eq!(j.snapshot().jobs[&1].phase, Phase::Completed);
            assert!(j.snapshot().unresolved.is_empty());
        } else {
            assert_eq!(j.snapshot().jobs[&1].phase, Phase::Interrupted);
            assert_eq!(
                j.snapshot().jobs[&1].checkpoints[0].outcome.effect,
                Knowledge::Unknown
            );
            assert_eq!(j.snapshot().unresolved.len(), 1);
        }
        j.tick(&mut e).unwrap();
        assert!(e.audit.begins.is_empty());
    }
    println!(
        "exhaustive outcome truncations checked: {}",
        outcome.len() + 1
    );
}

#[test]
fn complete_corrupt_records_fail_closed_without_modifying_evidence() {
    let states = checkpoints();
    let prefix = records(&states[..5]);
    let good = encode(&states[5]).unwrap();
    let dir = directory("corruption");
    let mut corrupt = Vec::new();
    for offset in [0, 8, 12, 20, 24, 28, good.len() - 1] {
        let mut frame = good.clone();
        frame[offset] ^= 1;
        corrupt.push(frame);
    }
    corrupt.push(raw_record(6, b"{invalid-json}"));
    let mut unknown_schema = serde_json::to_value(&states[5]).unwrap();
    unknown_schema["unexpected"] = true.into();
    corrupt.push(raw_record(6, &serde_json::to_vec(&unknown_schema).unwrap()));
    for change in 0..9 {
        let mut state = states[5].clone();
        match change {
            0 => state.version = 2,
            1 => state.revision += 1,
            2 => state.jobs.get_mut(&1).unwrap().next_step = 15,
            3 => state.next_job = 1,
            4 => {
                state.unresolved.insert(999);
            }
            5 => state.events.back_mut().unwrap().cursor += 1,
            6 => {
                state.jobs.get_mut(&1).unwrap().checkpoints[0]
                    .outcome
                    .effect = Knowledge::NotApplied
            }
            7 => {
                state.clients.clear();
            } // Cannot discard durable idempotency watermarks.
            8 => state.jobs.get_mut(&1).unwrap().binding.generation += 1,
            _ => unreachable!(),
        }
        corrupt.push(unchecked_record(&state));
    }
    for frame in corrupt {
        let mut bytes = prefix.clone();
        bytes.extend(frame);
        fs::write(dir.join("jobs.wal"), &bytes).unwrap();
        assert!(FileJournal::open(&dir).is_err());
        assert_eq!(fs::read(dir.join("jobs.wal")).unwrap(), bytes);
    }
}

#[test]
fn oversized_framing_and_files_are_rejected_before_unbounded_reads() {
    let dir = directory("bounds");
    let mut header = b"RBJOB001".to_vec();
    header.extend_from_slice(&((MAX_RECORD + 1) as u32).to_le_bytes());
    header.extend_from_slice(&1u64.to_le_bytes());
    header.extend_from_slice(&crc32fast::hash(&header).to_le_bytes());
    header.extend_from_slice(&0u32.to_le_bytes());
    fs::write(dir.join("jobs.wal"), header).unwrap();
    assert!(FileJournal::open(&dir).is_err());
    let file = OpenOptions::new()
        .write(true)
        .open(dir.join("jobs.wal"))
        .unwrap();
    file.set_len(MAX_LOG + 1).unwrap();
    drop(file);
    assert!(FileJournal::open(&dir).is_err());
    // Bound this disposable test artifact too.
    OpenOptions::new()
        .write(true)
        .open(dir.join("jobs.wal"))
        .unwrap()
        .set_len(0)
        .unwrap();
}

#[derive(Clone, Copy)]
enum Fault {
    Before,
    Partial,
    After,
}
struct FaultJournal {
    inner: FileJournal,
    path: PathBuf,
    fault: Arc<Mutex<Option<Fault>>>,
}
impl Journal for FaultJournal {
    fn load(&self) -> Option<Snapshot> {
        self.inner.load()
    }
    fn append(&mut self, state: &Snapshot, reserve: usize) -> Result<(), String> {
        let fault = self.fault.lock().unwrap().take();
        match fault {
            Some(Fault::Before) => Err("injected write refusal".into()),
            Some(Fault::Partial) => {
                let bytes = encode(state)?;
                let mut file = OpenOptions::new().append(true).open(&self.path).unwrap();
                file.write_all(&bytes[..bytes.len() / 2]).unwrap();
                file.sync_all().unwrap();
                Err("injected torn write".into())
            }
            Some(Fault::After) => {
                self.inner.append(state, reserve)?;
                Err("injected lost durable acknowledgement".into())
            }
            None => self.inner.append(state, reserve),
        }
    }
}

#[test]
fn real_file_failures_at_intent_and_outcome_preserve_effect_invariants() {
    for outcome_boundary in [false, true] {
        for fault in [Fault::Before, Fault::Partial, Fault::After] {
            let dir = directory("fault");
            let trigger = Arc::new(Mutex::new(None));
            let storage = FaultJournal {
                inner: FileJournal::open(&dir).unwrap(),
                path: dir.join("jobs.wal"),
                fault: trigger.clone(),
            };
            let mut j = Jobs::open(storage, "files", Limits::default()).unwrap();
            let mut e = FakeExecutor::default();
            e.current = j.bind(Some("disposable:2")).unwrap();
            j.command(command(&j, plan())).unwrap();
            j.tick(&mut e).unwrap(); // Running; no intent yet.
            if outcome_boundary {
                j.tick(&mut e).unwrap();
            }
            *trigger.lock().unwrap() = Some(fault);
            assert!(j.tick(&mut e).is_err());
            assert_eq!(e.audit.effects.len(), usize::from(outcome_boundary));
            assert_eq!(j.tick(&mut e).unwrap(), Progress::Fenced);
            drop(j);
            let storage = FileJournal::open(&dir).unwrap();
            assert_eq!(
                storage.repaired_tail_bytes > 0,
                matches!(fault, Fault::Partial)
            );
            let mut recovered = Jobs::open(storage, "files", Limits::default()).unwrap();
            let state = &recovered.snapshot().jobs[&1];
            if outcome_boundary && matches!(fault, Fault::After) {
                assert_eq!(state.phase, Phase::Completed);
            } else {
                assert_eq!(state.phase, Phase::Interrupted);
                assert_eq!(
                    !recovered.snapshot().unresolved.is_empty(),
                    outcome_boundary || matches!(fault, Fault::After)
                );
            }
            let begins = e.audit.begins.len();
            recovered.tick(&mut e).unwrap();
            assert_eq!(e.audit.begins.len(), begins);
        }
    }
}

#[test]
fn disk_quota_fences_before_start_and_retains_room_for_recovery() {
    let dir = directory("quota");
    let mut j = Jobs::open(
        FileJournal::with_ceiling(&dir, 4 * MAX_RECORD as u64).unwrap(),
        "files",
        Limits::default(),
    )
    .unwrap();
    j.bind(Some("disposable:3")).unwrap();
    let mut e = FakeExecutor::default();
    // Rejected commands are durable too. Their bounded receipts fill the log.
    let mut writes = 0;
    for _ in 0..2000 {
        let c = command(
            &j,
            Action::Submit {
                steps: vec![
                    Step {
                        kind: Kind::Observe,
                        payload: "x".repeat(257)
                    };
                    16
                ],
                consent: false,
            },
        );
        if j.command(c).is_err() {
            break;
        }
        writes += 1;
    }
    assert!(j.view().fence.is_some(), "test must reach storage quota");
    assert!(writes > 0 && writes < 2000);
    assert_eq!(j.tick(&mut e).unwrap(), Progress::Fenced);
    assert!(e.audit.effects.is_empty());
    drop(j);
    let j = Jobs::open(
        FileJournal::with_ceiling(&dir, 4 * MAX_RECORD as u64).unwrap(),
        "files",
        Limits::default(),
    )
    .unwrap();
    assert_eq!(j.snapshot().clients["desktop"].high_water, writes);
    assert!(fs::metadata(dir.join("jobs.wal")).unwrap().len() <= 4 * MAX_RECORD as u64);
}
