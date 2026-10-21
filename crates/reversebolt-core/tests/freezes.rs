use reversebolt_core::{
    engine::{Engine, View},
    freezes::Freeze,
    runtime::{FieldRef, Locator, Member, Scope, Step, TypeKey},
};
use serde_json::{Value, json};
fn fixture() -> (Freeze, Value) {
    let typ = TypeKey {
        image: "Fixture.dll".into(),
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
            declaring: typ.clone(),
            field_id: 0,
        },
        steps: vec![Step {
            owner: typ.clone(),
            member: Member::Field {
                field: FieldRef {
                    declaring: typ,
                    field_id: 1,
                },
            },
        }],
    };
    let check = json!({"path_id":path.id(),"status":"read","type_code":12,"bytes":3.5f32.to_le_bytes(),"value":"3.5","field_address":"0x1010","snapshot_object_addresses":["0x1000"]});
    (
        Freeze {
            id: uuid::Uuid::new_v4().to_string(),
            actor: "desktop".into(),
            investigation: "i".into(),
            catalog: "c".into(),
            requested_revision: 0,
            candidate: path.id(),
            value: "3.5".into(),
            kind: 12,
            path,
            identity: None,
            phase: "active".into(),
            reason: String::new(),
            checks: 0,
            writes: 0,
            checked_ms: 0,
            intent: None,
            evidence: None,
        },
        check,
    )
}
#[test]
fn unchanged_reads_do_not_write_and_drift_uses_a_fresh_expected_value() {
    let (mut f, mut c) = fixture();
    assert!(f.check(&c, 100).unwrap().is_none());
    c["bytes"] = json!(2.5f32.to_le_bytes());
    c["value"] = json!("2.5");
    let p = f.check(&c, 200).unwrap().unwrap();
    assert_eq!(p.expected, 2.5f32.to_le_bytes());
    assert_eq!(p.replacement, 3.5f32.to_le_bytes());
    assert_eq!(f.checks, 2);
}
#[test]
fn changed_objects_and_large_drift_are_not_held_even_when_the_value_matches() {
    let (mut f, c) = fixture();
    f.check(&c, 100).unwrap();
    let mut changed = c.clone();
    changed["snapshot_object_addresses"] = json!(["0x2000"]);
    assert!(
        f.check(&changed, 200)
            .unwrap_err()
            .contains("object changed")
    );
    let mut changed = c.clone();
    changed["field_address"] = json!("0x2010");
    assert!(f.check(&changed, 200).is_err());
    let mut changed = c.clone();
    changed["bytes"] = json!(100f32.to_le_bytes());
    changed["value"] = json!("100");
    assert!(f.check(&changed, 200).is_err());
    let mut changed = c;
    changed["bytes"] = json!([0, 0, 0, 0]);
    assert!(f.check(&changed, 200).is_err());
}
#[test]
fn restarting_never_resumes_holds_and_interrupted_intents_remain_fenced() {
    let (mut active, c) = fixture();
    active.check(&c, 100).unwrap();
    let mut writing = active.clone();
    writing.id = uuid::Uuid::new_v4().to_string();
    writing.phase = "writing".into();
    let dir = std::env::temp_dir().join(format!("reversebolt-freeze-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("session.json");
    let view = View {
        freezes: vec![active, writing],
        ..View::default()
    };
    std::fs::write(&path, serde_json::to_vec(&view).unwrap()).unwrap();
    let engine = Engine::load(path).unwrap();
    assert_eq!(engine.view.freezes[0].phase, "stopped");
    assert_eq!(engine.view.freezes[1].phase, "uncertain");
    assert!(engine.view.freezes[1].holds(&engine.view.freezes[1].path.scope));
    let mut unknown = engine.view.freezes[1].clone();
    unknown.stop("off");
    assert_eq!(unknown.phase, "uncertain");
}
