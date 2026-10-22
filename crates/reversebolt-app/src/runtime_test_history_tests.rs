//! Real dispatcher/history checks with no attached target or native operations.
use super::*;
use reversebolt_core::runtime::{FieldRef, Locator, Member, Scope, Step, TypeKey};

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn saved(investigation: &str) -> Test {
    let scope = Scope {
        pid: 1,
        created: "0000000000000001".into(),
        session: 1,
    };
    let key = TypeKey {
        image: "Fixture".into(),
        type_name: "Actor".into(),
        layout_sha256: "a".repeat(64),
    };
    let field = FieldRef {
        declaring: key.clone(),
        field_id: 0,
    };
    let path = Locator {
        scope: scope.clone(),
        root: field.clone(),
        steps: vec![Step {
            owner: key,
            member: Member::Field { field },
        }],
    };
    serde_json::from_value(json!({
        "id":id(),"change_id":id(),"actor":"agent","investigation":investigation,"catalog_id":id(),
        "scope":scope,"window":1,
        "candidate":{"id":path.id(),"name":"Speed","description":"Synthetic fixture.","locator":path,
            "value":8,"bytes":8i32.to_le_bytes(),"type_code":8,"snapshot_id":0,"node_id":0,"edit_authority":false},
        "value":"0","hypothesis":"Does the observed response change?","actions":vec![Action{keys:vec![68],hold_ms:200};4],
        "prepared_revision":1,"created_unix_ms":1,"preparation":null,"metric":runtime_tests::METRIC_ID,
        "revision":2,"phase":"stopped","pending_capture":false,"conditions":[{"frames":[]},{"frames":[]},{"frames":[]}],
        "assessment":null,"reason":"Synthetic stopped comparison."
    })).unwrap()
}

fn prepare(t: &Test) -> Command {
    Command {
        action: "prepare_runtime_test".into(),
        source: t.actor.clone(),
        args: json!({
            "test_id":t.id,"change_id":t.change_id,"investigation_id":t.investigation,"catalog_id":t.catalog_id,
            "after_revision":t.prepared_revision,"candidate_id":t.candidate.id,"value":t.value,
            "hypothesis":t.hypothesis,"actions":t.actions
        }),
    }
}

#[test]
fn comparison_capacity_and_old_request_replay_survive_engine_restart() {
    let parent = std::env::temp_dir().canonicalize().unwrap();
    let directory = parent.join(format!("reversebolt-history-command-{}", id()));
    std::fs::create_dir(&directory).unwrap();
    let state = directory.join("state.json");
    let mut engine = Engine::load(state.clone()).unwrap();
    let published = Arc::new(RwLock::new(engine.view.clone()));
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    let investigation = id();
    let first = saved(&investigation);
    engine.runtime_tests.record(first.clone()).unwrap();
    for _ in 1..runtime_tests::MAX_TESTS_PER_INVESTIGATION {
        engine.runtime_tests.record(saved(&investigation)).unwrap();
    }
    let request = prepare(&first);
    assert_eq!(
        session
            .command(&mut engine, &request, &directory, &published)
            .unwrap(),
        first.summary()
    );
    let mut changed = request.clone();
    changed.args["value"] = json!("1");
    assert!(
        session
            .command(&mut engine, &changed, &directory, &published)
            .unwrap_err()
            .contains("different plan")
    );
    let full = prepare(&saved(&investigation));
    assert!(
        session
            .command(&mut engine, &full, &directory, &published)
            .unwrap_err()
            .contains("eight-comparison")
    );
    let next = saved(&id());
    // Capacity permits a new investigation. The next real guard requires a live
    // target; this test deliberately never supplies or launches one.
    assert_eq!(
        session
            .command(&mut engine, &prepare(&next), &directory, &published)
            .unwrap_err(),
        "Connect an application first."
    );
    engine.runtime_tests.record(next.clone()).unwrap();
    checkpoint(&mut engine, &published).unwrap();
    assert_eq!(published.read().unwrap().runtime_tests.len(), 9);
    drop(session);
    drop(engine);
    let mut engine = Engine::load(state).unwrap();
    assert_eq!(engine.view.runtime_tests.len(), 9);
    let mut session = Session::new(Arc::new(AtomicBool::new(false)));
    assert_eq!(
        session
            .command(&mut engine, &request, &directory, &published)
            .unwrap(),
        first.summary()
    );
    let read = Command {
        action: "read_runtime_test".into(),
        source: "agent".into(),
        args: json!({"test_id":first.id}),
    };
    assert_eq!(
        session
            .command(&mut engine, &read, &directory, &published)
            .unwrap()["test"],
        json!(first)
    );
    assert_eq!(
        engine
            .runtime_tests
            .capacity(&next.investigation)
            .remaining_record_slots,
        7
    );
    drop(session);
    drop(engine);
    let resolved = directory.canonicalize().unwrap();
    assert_eq!(resolved.parent().unwrap(), parent);
    assert!(
        resolved
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("reversebolt-history-command-")
    );
    std::fs::remove_dir_all(resolved).unwrap();
}
