//! Recovery through the shared engine using retained synthetic evidence only.
use super::*;
use reversebolt_core::{live::*, runtime, visual};

pub(super) fn report(test: &Test) -> Value {
    json!({"schema":5,"pid":test.scope.pid,"created":test.scope.created,"session":test.scope.session,
        "runtime":"C:/Fixture/Runtime.dll","root_catalog_sha256":"b".repeat(64),"edit_authority":false,
        "classes":[
            {"id":0,"key":test.candidate.locator.root.declaring,"shape":"object","fields":[
                {"id":0,"name":"Current","reference":true,"type_code":18,"width":8,"flags":16}]},
            {"id":1,"key":test.candidate.locator.steps[0].owner,"shape":"object","fields":[
                {"id":0,"name":"Speed","reference":false,"type_code":8,"width":4,"flags":0}]}],
        "snapshots":[{"snapshot_id":0,"nodes":[{"id":0,"class":1,"address":"0x1000"}],
            "roots":[{"class":0,"field":0,"read_ok":true,"node":{"id":0}}],
            "reads":[{"node":0,"declaring_class":1,"field":0,"element_index":null,"type_code":8,
                "bytes":8i32.to_le_bytes(),"value":8,"reference":null,"read_ok":true}]}]})
}

struct NoAccess(ProcessInfo);
impl NativeTarget for NoAccess {
    fn info(&self) -> &ProcessInfo {
        &self.0
    }
    fn regions(&self) -> Result<Vec<Region>> {
        Ok(vec![Region {
            base: 0x1000,
            length: 4096,
            ..Default::default()
        }])
    }
    fn read(&self, _: u64, _: usize) -> Result<Vec<u8>> {
        panic!("Recovery must not access the target")
    }
    fn screen(&self, _: &std::path::Path) -> Result<Screen> {
        panic!("Recovery must not capture")
    }
    fn alive(&self) -> bool {
        true
    }
}

fn attach(engine: &mut Engine, directory: &std::path::Path, report: &Value) {
    let path = directory.join("current-report.json");
    std::fs::write(&path, serde_json::to_vec(report).unwrap()).unwrap();
    let info = ProcessInfo {
        pid: report["pid"].as_u64().unwrap() as u32,
        created: report["created"].as_str().unwrap().into(),
        session: report["session"].as_u64().unwrap() as u32,
        window: 1,
        ..Default::default()
    };
    let mut live =
        LiveSession::attach(Box::new(NoAccess(info)), 1, directory.join("captures")).unwrap();
    live.view.runtime = runtime::View {
        available: true,
        phase: "ready".into(),
        catalog_id: id(),
        catalog_source: Some(runtime::CatalogSource {
            path: path.to_string_lossy().into_owned(),
            document_sha256: visual::digest(&serde_json::to_vec(report).unwrap()),
            observed_unix_ms: 42,
        }),
        ..Default::default()
    };
    engine.live = Some(live);
}

fn recover(engine: &mut Engine, test: &Test) -> Result<Value> {
    let view = &engine.live.as_ref().unwrap().view.runtime;
    crate::runtime_recovery_bridge::command(
        engine,
        &Command {
            action: "recover_runtime_control".into(),
            source: "agent".into(),
            args: json!({"test_id":test.id,"catalog_id":view.catalog_id,"after_revision":view.revision}),
        },
    )
}

#[test]
fn recovery_reopens_evidence_reacquires_new_scope_and_publishes_shared_cards_without_target_access()
{
    for regional in [false, true] {
        let (directory, state_path, engine, test, trial) = complete_motion_fixture(regional);
        if !regional {
            assert_eq!(
                test.assessment.as_ref().unwrap().status,
                "suppression_observed"
            );
            reversebolt_vision::response::verify_recorded_response(
                &test,
                &trial,
                |c, i| engine.runtime_tests.read_image(&test, c, i),
                |c, i| engine.runtime_tests.read_input(&test, c, i),
            )
            .unwrap();
        }
        engine.save().unwrap();
        drop(engine);
        let mut engine = Engine::load(state_path).unwrap();
        assert!(
            engine
                .runtime_tests
                .get(&test.id)
                .unwrap()
                .recovery_recipe
                .is_some()
        );
        let mut current = report(&test);
        current["pid"] = json!(99);
        current["created"] = json!("0000000000000099");
        current["classes"][1]["key"]["layout_sha256"] = json!("c".repeat(64));
        current["root_catalog_sha256"] = json!("d".repeat(64));
        current["snapshots"][0]["reads"][0]["value"] = json!(12);
        current["snapshots"][0]["reads"][0]["bytes"] = json!(12i32.to_le_bytes());
        attach(&mut engine, &directory, &current);
        let result = recover(&mut engine, &test).unwrap();
        assert_eq!(result["shown"], true);
        assert_eq!(result["edit_authority"], false);
        assert_eq!(result["recovery"]["strict_status"], "catalog_changed");
        assert_eq!(result["preparation"]["after_revision"], 1);
        assert_eq!(result["preparation"]["actions"], json!(test.actions));
        let view = &mut engine.live.as_mut().unwrap().view.runtime;
        let candidate = view.cards[0].candidate.clone();
        assert_ne!(candidate.id, test.candidate.id);
        assert_eq!(candidate.value, 12);
        assert_eq!(candidate.locator.scope.pid, 99);
        assert!(!candidate.edit_authority);
        assert_eq!(view.recovered_controls[0].source_test_id, test.id);
        assert!(view.recovered_controls[0].needs_revalidation);
        assert_eq!(view.cards[0].observed_unix_ms, 42);
        view.cards[0].value = json!(15); // A later reading must survive recovery.
        view.search = Some(runtime::SearchPage {
            query: "kept".into(),
            offset: 0,
            matches: 0,
            candidates: vec![],
        });
        let result = crate::runtime_recovery_bridge::automatic(
            &mut engine,
            &current,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(result["shown"], 1);
        let view = &engine.live.as_ref().unwrap().view.runtime;
        assert_eq!(view.cards[0].value, 15);
        assert_eq!(view.search.as_ref().unwrap().query, "kept");
        assert_eq!(view.recovered_controls.len(), 1);
        assert_eq!(engine.runtime_tests.get(&test.id).unwrap().scope.pid, 1);
        assert_eq!(
            json!(engine.runtime_trials.get(&trial.id).unwrap()),
            json!(trial)
        );

        // Latest negative/stopped evidence blocks an automatic older success.
        let mut stopped = test.clone();
        stopped.id = id();
        stopped.phase = Phase::Stopped;
        engine.runtime_tests.records.push(stopped);
        let snapshot = json!(engine.live.as_ref().unwrap().view.runtime);
        let result = crate::runtime_recovery_bridge::automatic(
            &mut engine,
            &current,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(result["shown"], 0);
        assert_eq!(result["unavailable"], 1);
        assert_eq!(json!(engine.live.as_ref().unwrap().view.runtime), snapshot);
        engine.runtime_tests.records.pop();
        assert_eq!(
            crate::runtime_recovery_bridge::automatic(
                &mut engine,
                &current,
                &AtomicBool::new(true)
            )
            .unwrap()["shown"],
            0
        );
        assert_eq!(json!(engine.live.as_ref().unwrap().view.runtime), snapshot);
        drop(engine);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn recovery_rejects_changed_evidence_and_bindings_before_publishing() {
    let (directory, _, mut engine, test, trial) = complete_motion_fixture(false);
    let current = report(&test);
    attach(&mut engine, &directory, &current);
    let snapshot = json!(engine.live.as_ref().unwrap().view.runtime);
    let image_path = directory
        .join("runtime-tests")
        .join(&test.id)
        .join("1-2.png");
    let original = std::fs::read(&image_path).unwrap();
    std::fs::write(&image_path, b"changed").unwrap();
    assert!(recover(&mut engine, &test).is_err());
    assert_eq!(json!(engine.live.as_ref().unwrap().view.runtime), snapshot);
    std::fs::write(&image_path, original).unwrap();
    let original_trial = engine.runtime_trials.records.pop().unwrap();
    assert!(recover(&mut engine, &test).is_err());
    engine.runtime_trials.records.push(original_trial);

    for mutation in ["metric", "input", "readback", "geometry", "recipe"] {
        let mut bad = test.clone();
        match mutation {
            "metric" => bad.conditions[1].frames[2].difference = Some(1.),
            "input" => bad.conditions[1].frames[2].input.held_us[0] += 1,
            "readback" => bad.conditions[1].runtime_evidence = None,
            "geometry" => {
                for c in &mut bad.conditions {
                    for f in &mut c.frames {
                        f.width += 1;
                    }
                }
            }
            _ => bad.recovery_recipe.as_mut().unwrap().source.scope.pid += 1,
        }
        engine.runtime_tests.records[0] = bad;
        assert!(recover(&mut engine, &test).is_err(), "{mutation}");
        assert_eq!(json!(engine.live.as_ref().unwrap().view.runtime), snapshot);
    }
    engine.runtime_tests.records[0] = test.clone();
    let mut removed = test.clone();
    removed.recovery_recipe = None;
    assert!(
        engine
            .runtime_tests
            .record(removed)
            .unwrap_err()
            .contains("metadata")
    );
    let args = json!({"test_id":test.id,"catalog_id":"old","after_revision":0});
    assert!(
        crate::runtime_recovery_bridge::command(
            &mut engine,
            &Command {
                action: "recover_runtime_control".into(),
                source: "agent".into(),
                args
            }
        )
        .is_err()
    );
    let live = &mut engine.live.as_mut().unwrap().view;
    live.memory_process
        .as_mut()
        .unwrap_or(&mut live.process)
        .pid += 1;
    assert!(recover(&mut engine, &test).is_err());
    assert_eq!(json!(engine.live.as_ref().unwrap().view.runtime), snapshot);
    assert_eq!(
        json!(engine.runtime_trials.get(&trial.id).unwrap()),
        json!(trial)
    );
    drop(engine);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "Requires a manifest of local archived game evidence; never contacts a target"]
fn replay_remembered_archive() {
    use std::{io::Read, path::PathBuf};
    let read = |path: &std::path::Path| -> Value {
        let mut bytes = vec![];
        std::fs::File::open(path)
            .unwrap()
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() <= 64 * 1024 * 1024);
        serde_json::from_slice(&bytes).unwrap()
    };
    let manifest_path = PathBuf::from(
        std::env::var("REVERSEBOLT_RECOVERY_REPLAY").expect("Supply an archive manifest path"),
    );
    let manifest = read(&manifest_path);
    let path = |key: &str| PathBuf::from(manifest[key].as_str().unwrap());
    let output = path("output");
    std::fs::create_dir(&output).unwrap(); // Never replace an earlier replay.
    let old_value = read(&path("test"));
    let mut test: Test = serde_json::from_value(old_value.clone()).unwrap();
    assert!(test.recovery_recipe.is_none());
    let old_report = read(&path("source_report"));
    let current = read(&path("current_report"));
    // Explicit research backfill in an isolated copy. Existing production records
    // are never migrated or treated as though they originally stored a recipe.
    test.recovery_recipe = Some(
        reversebolt_core::runtime_recovery::Recipe::capture(&old_report, &test.candidate.id)
            .unwrap(),
    );
    let data = output.join("data");
    let state = data.join("state.json");
    let engine = Engine::load(state.clone()).unwrap();
    let test_dir = data.join("runtime-tests").join(&test.id);
    std::fs::create_dir(&test_dir).unwrap();
    let source_data = path("source_data");
    let mut files = vec![];
    for condition in &test.conditions {
        for frame in &condition.frames {
            for name in [&frame.path, &frame.input_path] {
                let source = source_data.join("runtime-tests").join(name);
                let destination = data.join("runtime-tests").join(name);
                let bytes = std::fs::read(&source).unwrap();
                assert!(bytes.len() <= 10 * 1024 * 1024);
                std::fs::write(&destination, &bytes).unwrap();
                files.push(json!({"path":source,"sha256":visual::digest(&bytes)}));
            }
        }
    }
    std::fs::write(
        data.join("runtime-tests").join(format!("{}.json", test.id)),
        serde_json::to_vec_pretty(&test).unwrap(),
    )
    .unwrap();
    let trial_path = source_data
        .join("runtime-trials")
        .join(format!("{}.json", test.change_id));
    let trial = std::fs::read(&trial_path).unwrap();
    std::fs::write(
        data.join("runtime-trials")
            .join(format!("{}.json", test.change_id)),
        &trial,
    )
    .unwrap();
    engine.save().unwrap();
    drop(engine);
    let mut engine = Engine::load(state).unwrap();
    assert_eq!(engine.runtime_tests.records.len(), 1);
    attach(&mut engine, &data, &current);
    let start = std::time::Instant::now();
    let manual = recover(&mut engine, &test).unwrap();
    assert_eq!(manual["shown"], true);
    assert_ne!(manual["recovery"]["candidate"]["id"], test.candidate.id);
    let automatic =
        crate::runtime_recovery_bridge::automatic(&mut engine, &current, &AtomicBool::new(false))
            .unwrap();
    assert_eq!(automatic["shown"], 1);
    assert_eq!(
        engine
            .live
            .as_ref()
            .unwrap()
            .view
            .runtime
            .recovered_controls
            .len(),
        1
    );
    let result = json!({"passed":true,"research_backfill":true,"target_access":false,"model_calls":0,
        "elapsed_ms":start.elapsed().as_millis(),"manifest_sha256":visual::digest(&std::fs::read(manifest_path).unwrap()),
        "source_test_sha256":visual::digest(&serde_json::to_vec(&old_value).unwrap()),
        "source_trial_sha256":visual::digest(&trial),"evidence":files,
        "manual":manual,"automatic":automatic,"runtime":engine.live.as_ref().unwrap().view.runtime});
    std::fs::write(
        output.join("result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
}
