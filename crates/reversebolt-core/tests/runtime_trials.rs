use reversebolt_core::{
    changes::{Phase, now_ms},
    runtime::{FieldRef, Locator, Member, Scope, Step, TypeKey, scalar_value},
    runtime_patch::Patch,
    runtime_trials::{Context, Evidence, Store, Trial},
};
use serde_json::{Value, json};
use std::{cell::Cell, fs, path::PathBuf};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "reversebolt-runtime-trials-{}",
            uuid::Uuid::new_v4()
        )))
    }
    fn store(&self) -> Store {
        Store::open(self.0.clone()).unwrap()
    }
    fn record(&self, trial: &Trial) -> PathBuf {
        self.0.join(format!("{}.json", trial.id))
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn review() -> Trial {
    let kind = TypeKey {
        image: "Owned.dll".into(),
        type_name: "Parameter".into(),
        layout_sha256: "a".repeat(64),
    };
    let path = Locator {
        scope: Scope {
            pid: 7,
            created: "0123456789abcdef".into(),
            session: 2,
        },
        root: FieldRef {
            declaring: kind.clone(),
            field_id: 0,
        },
        steps: vec![Step {
            owner: kind.clone(),
            member: Member::Field {
                field: FieldRef {
                    declaring: kind,
                    field_id: 1,
                },
            },
        }],
    };
    let check = json!({"path_id":path.id(),"status":"read","type_code":12,"bytes":3.5f32.to_le_bytes(),"value":"3.5",
        "field_address":"0x1010","snapshot_object_addresses":["0x1000"]});
    let patch = Patch::prepare(path.clone(), &check, "4.5", now_ms()).unwrap();
    let context = Context {
        investigation: "investigation".into(),
        catalog_id: "catalogue".into(),
        revision: 2,
        process: path.scope.clone(),
    };
    let evidence = Evidence::new(
        "baseline.json".into(),
        json!({"operation":"refresh_runtime_paths","scope":path.scope,"checks":[check]}),
    )
    .unwrap();
    Trial::prepare(
        context,
        "Coefficient".into(),
        "test".into(),
        patch,
        evidence,
    )
    .unwrap()
}

fn receipt(patch: &Patch, written: bool) -> Evidence {
    let check = json!({"path_id":patch.path.id(),"status":"read","type_code":patch.type_code,"bytes":patch.expected,
        "value":scalar_value(patch.type_code,&patch.expected).unwrap(),"field_address":format!("0x{:x}",patch.field_address),
        "snapshot_object_addresses":patch.object_addresses.iter().map(|a|format!("0x{a:x}")).collect::<Vec<_>>()});
    Evidence::new("result.json".into(), json!({"operation":"patch_runtime_path","scope":patch.path.scope,"patch_id":patch.id(),"checks":[check],
        "patch":{"state":if written {"written"} else {"refused"},"attempted":written,"transferred":if written {patch.replacement.len()} else {0},
        "guard":if written {"ready"} else {"value_changed"},"observed_after":if written {json!(patch.replacement)} else {Value::Null}}})).unwrap()
}

#[test]
fn review_requires_matching_baseline_and_new_identity() {
    let dir = Directory::new();
    let mut store = dir.store();
    let trial = review();
    let mut invalid = trial.clone();
    invalid.id = "../escape".into();
    assert!(store.insert(invalid).is_err());
    let mut invalid = trial.clone();
    invalid.baseline.report["checks"][0]["value"] = json!("7");
    assert!(store.insert(invalid).is_err());
    let mut invalid = trial.clone();
    invalid.patch.expected = 7f32.to_le_bytes().to_vec();
    assert!(store.insert(invalid).is_err());
    store.insert(trial.clone()).unwrap();
    assert!(store.insert(trial).is_err());
    assert_eq!(store.records.len(), 1);
}

#[test]
fn intent_precedes_each_effect_and_duplicate_requests_return_saved_results() {
    let dir = Directory::new();
    let mut store = dir.store();
    let trial = store.insert(review()).unwrap();
    let memory = Cell::new(3.5f32);
    let written = store
        .execute(&trial.id, &trial.context, false, |patch| {
            let saved: Trial =
                serde_json::from_slice(&fs::read(dir.record(&trial)).unwrap()).unwrap();
            assert_eq!(saved.phase, Phase::Writing);
            assert_eq!(saved.write.unwrap().outcome, "unknown");
            assert_eq!(patch.expected, memory.get().to_le_bytes());
            memory.set(4.5);
            Ok(receipt(patch, true))
        })
        .unwrap();
    assert_eq!(written.phase, Phase::Written);
    assert_eq!(memory.get(), 4.5);
    assert_eq!(
        store
            .execute(&trial.id, &trial.context, false, |_| panic!(
                "duplicate write"
            ))
            .unwrap()
            .phase,
        Phase::Written
    );
    let mut later = trial.context.clone();
    later.revision += 1;
    let restored = store
        .execute(&trial.id, &later, true, |patch| {
            let saved: Trial =
                serde_json::from_slice(&fs::read(dir.record(&trial)).unwrap()).unwrap();
            assert_eq!(saved.phase, Phase::Restoring);
            assert_eq!(patch.expected, memory.get().to_le_bytes());
            memory.set(3.5);
            Ok(receipt(patch, true))
        })
        .unwrap();
    assert_eq!(restored.phase, Phase::Restored);
    assert_eq!(memory.get(), 3.5);
    assert_eq!(
        store
            .execute(&trial.id, &later, true, |_| panic!("duplicate restore"))
            .unwrap()
            .phase,
        Phase::Restored
    );
    assert_eq!(dir.store().get(&trial.id).unwrap().phase, Phase::Restored);
}

#[test]
fn changed_revision_scope_and_expired_review_cannot_call_backend() {
    for variant in 0..5 {
        let dir = Directory::new();
        let mut store = dir.store();
        let mut trial = review();
        if variant == 4 {
            trial.patch.prepared_unix_ms -= 60_001;
        }
        let trial = store.insert(trial).unwrap();
        let mut context = trial.context.clone();
        match variant {
            0 => context.revision += 1,
            1 => context.process.pid += 1,
            2 => context.investigation.push('x'),
            3 => context.catalog_id.push('x'),
            _ => {}
        }
        let result = store.execute(&trial.id, &context, false, |_| {
            panic!("invalid review dispatched")
        });
        if variant == 0 || variant == 4 {
            assert_eq!(result.unwrap().phase, Phase::Expired);
        } else {
            assert!(result.is_err());
        }
        assert!(store.get(&trial.id).unwrap().write.is_none());
    }
}

#[test]
fn refused_restoration_preserves_recorded_change_and_is_not_retried() {
    let dir = Directory::new();
    let mut store = dir.store();
    let trial = store.insert(review()).unwrap();
    store
        .execute(&trial.id, &trial.context, false, |p| Ok(receipt(p, true)))
        .unwrap();
    let refused = store
        .execute(&trial.id, &trial.context, true, |p| Ok(receipt(p, false)))
        .unwrap();
    assert_eq!(refused.phase, Phase::Written);
    assert_eq!(refused.restoration.unwrap().outcome, "refused");
    assert_eq!(
        store
            .execute(&trial.id, &trial.context, true, |_| panic!("retry"))
            .unwrap()
            .phase,
        Phase::Written
    );
    assert!(!store.holds(&trial.context.process));
    assert_eq!(dir.store().get(&trial.id).unwrap().phase, Phase::Written);
}

fn original_already_present(inverse: &Patch) -> Evidence {
    let mut report = receipt(inverse, false).report;
    report["checks"][0]["bytes"] = json!(inverse.replacement);
    report["checks"][0]["value"] = scalar_value(inverse.type_code, &inverse.replacement).unwrap();
    Evidence::new("already-original.json".into(), report).unwrap()
}

#[test]
fn already_original_restoration_is_durable_nonwriting_and_replayable() {
    let dir = Directory::new();
    let mut store = dir.store();
    let trial = store.insert(review()).unwrap();
    store
        .execute(&trial.id, &trial.context, false, |p| Ok(receipt(p, true)))
        .unwrap();
    let recovered = store
        .execute(&trial.id, &trial.context, true, |inverse| {
            // The game has independently returned to the original bytes.
            Ok(original_already_present(inverse))
        })
        .unwrap();
    assert_eq!(recovered.phase, Phase::Restored);
    let attempt = recovered.restoration.as_ref().unwrap();
    assert_eq!(attempt.outcome, "already_original");
    assert_eq!(
        attempt.evidence.as_ref().unwrap().report["patch"]["attempted"],
        false
    );
    assert_eq!(
        store
            .execute(&trial.id, &trial.context, true, |_| panic!(
                "restore replay wrote"
            ))
            .unwrap()
            .phase,
        Phase::Restored
    );
    assert_eq!(dir.store().get(&trial.id).unwrap().phase, Phase::Restored);

    // Previously saved refusal records remain readable and are not silently upgraded.
    let mut legacy = recovered;
    legacy.phase = Phase::Written;
    legacy.restoration.as_mut().unwrap().outcome = "refused".into();
    fs::write(dir.record(&legacy), serde_json::to_vec(&legacy).unwrap()).unwrap();
    assert_eq!(dir.store().get(&legacy.id).unwrap().phase, Phase::Written);
}

#[test]
fn already_original_requires_exact_identity_value_and_no_write_evidence() {
    use reversebolt_core::runtime_trials::{classify, classify_restoration};
    let trial = review();
    let inverse = trial
        .patch
        .restoration(&receipt(&trial.patch, true).report, now_ms())
        .unwrap();
    let original = original_already_present(&inverse).report;
    assert_eq!(classify(&inverse, &original), "refused");
    assert_eq!(
        classify_restoration(&inverse, &original),
        "already_original"
    );
    for (pointer, replacement) in [
        ("/checks/0/field_address", json!("0x2010")),
        ("/checks/0/snapshot_object_addresses", json!(["0x2000"])),
        ("/checks/0/path_id", json!("another")),
        ("/checks/0/type_code", json!(8)),
        ("/checks/0/status", json!("missing")),
        ("/checks/0/bytes", json!(7.5f32.to_le_bytes())),
        ("/checks/0/value", json!("7.5")),
        ("/patch/guard", json!("expired")),
        ("/patch/attempted", json!(true)),
        ("/patch/transferred", json!(4)),
        ("/patch/observed_after", json!(trial.patch.expected)),
        ("/scope/pid", json!(8)),
        ("/patch_id", json!("another")),
    ] {
        let mut report = original.clone();
        *report.pointer_mut(pointer).unwrap() = replacement;
        assert_ne!(
            classify_restoration(&inverse, &report),
            "already_original",
            "{pointer}"
        );
    }
}

#[test]
fn unknown_or_invalid_receipt_holds_same_process_and_survives_restart() {
    for variant in 0..4 {
        let dir = Directory::new();
        let mut store = dir.store();
        let trial = store.insert(review()).unwrap();
        let other = store.insert(review()).unwrap();
        let result = store
            .execute(&trial.id, &trial.context, false, |patch| {
                if variant == 0 {
                    return Err("helper result lost".into());
                }
                let mut evidence = receipt(patch, true);
                if variant == 1 {
                    evidence.report_sha256 = "wrong".into();
                } else {
                    if variant == 2 {
                        evidence.report["patch_id"] = json!("other");
                    } else {
                        evidence.report["patch"]["transferred"] = json!(2);
                    }
                    evidence = Evidence::new(evidence.path, evidence.report).unwrap();
                }
                Ok(evidence)
            })
            .unwrap();
        assert_eq!(result.phase, Phase::Uncertain);
        assert!(store.holds(&trial.context.process));
        assert!(
            store
                .execute(&other.id, &other.context, false, |_| panic!("held process"))
                .is_err()
        );
        assert_eq!(
            store
                .execute(&trial.id, &trial.context, false, |_| panic!("retry"))
                .unwrap()
                .phase,
            Phase::Uncertain
        );
        let reopened = dir.store();
        assert!(reopened.holds(&trial.context.process));
        let mut independent = trial.context.process.clone();
        independent.pid += 1;
        assert!(!reopened.holds(&independent));
    }
}

#[test]
fn restart_invalidates_review_and_retains_interrupted_intents_without_replay() {
    for restore in [false, true] {
        let dir = Directory::new();
        let mut store = dir.store();
        let trial = store.insert(review()).unwrap();
        let untouched = store.insert(review()).unwrap();
        if restore {
            store
                .execute(&trial.id, &trial.context, false, |p| Ok(receipt(p, true)))
                .unwrap();
        }
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                store.execute(&trial.id, &trial.context, restore, |_| {
                    panic!("simulated process loss")
                })
            }))
            .is_err()
        );
        let mut reopened = dir.store();
        assert_eq!(reopened.get(&untouched.id).unwrap().phase, Phase::Expired);
        assert_eq!(
            reopened
                .execute(&trial.id, &trial.context, restore, |_| panic!("replayed"))
                .unwrap()
                .phase,
            Phase::Uncertain
        );
        assert!(reopened.holds(&trial.context.process));
    }
}

#[test]
fn failed_intent_persistence_prevents_dispatch() {
    let dir = Directory::new();
    let mut store = dir.store();
    let trial = store.insert(review()).unwrap();
    fs::create_dir(dir.record(&trial).with_extension("pending")).unwrap();
    assert!(
        store
            .execute(&trial.id, &trial.context, false, |_| panic!(
                "unsaved intent dispatched"
            ))
            .is_err()
    );
    assert!(store.holds(&trial.context.process));
    assert!(Store::open(dir.0.clone()).is_err());
    fs::remove_dir(dir.record(&trial).with_extension("pending")).unwrap();
    assert_eq!(dir.store().get(&trial.id).unwrap().phase, Phase::Expired);
}

#[test]
fn failed_result_persistence_keeps_actual_result_in_memory_and_unknown_intent_on_disk() {
    let dir = Directory::new();
    let mut store = dir.store();
    let trial = store.insert(review()).unwrap();
    assert!(
        store
            .execute(&trial.id, &trial.context, false, |p| {
                fs::create_dir(dir.record(&trial).with_extension("pending")).unwrap();
                Ok(receipt(p, true))
            })
            .is_err()
    );
    assert_eq!(store.get(&trial.id).unwrap().phase, Phase::Written);
    assert!(store.holds(&trial.context.process));
    assert!(
        store
            .execute(&trial.id, &trial.context, true, |_| panic!("store failed"))
            .is_err()
    );
    fs::remove_dir(dir.record(&trial).with_extension("pending")).unwrap();
    assert_eq!(dir.store().get(&trial.id).unwrap().phase, Phase::Uncertain);
}

#[test]
fn corrupted_attempt_or_phase_cannot_reopen_as_a_trustworthy_history() {
    for variant in 0..4 {
        let dir = Directory::new();
        let mut store = dir.store();
        let trial = store.insert(review()).unwrap();
        store
            .execute(&trial.id, &trial.context, false, |p| Ok(receipt(p, true)))
            .unwrap();
        let mut saved: Value =
            serde_json::from_slice(&fs::read(dir.record(&trial)).unwrap()).unwrap();
        match variant {
            0 => {
                saved["write"]["evidence"]["report"]["patch"]["observed_after"] =
                    json!([0, 0, 0, 0])
            }
            1 => saved["write"]["patch"]["field_address"] = json!(0x2020),
            2 => saved["phase"] = json!("restored"),
            _ => saved["context"]["process"]["pid"] = json!(8),
        }
        fs::write(dir.record(&trial), serde_json::to_vec(&saved).unwrap()).unwrap();
        assert!(Store::open(dir.0.clone()).is_err());
    }
}
