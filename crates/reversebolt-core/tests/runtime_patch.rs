use reversebolt_core::{
    runtime::{FieldRef, Locator, Member, Scope, Step, TypeKey},
    runtime_patch::{Guard, LIFETIME_MS, Patch, encode},
};
use serde_json::{Value, json};

fn reading() -> (Locator, Value) {
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
    (path, check)
}

#[test]
fn numeric_encoding_preserves_integer_limits_and_rejects_unsupported_values() {
    assert_eq!(
        encode(11, "18446744073709551615").unwrap(),
        u64::MAX.to_le_bytes()
    );
    assert_eq!(
        encode(10, "-9223372036854775808").unwrap(),
        i64::MIN.to_le_bytes()
    );
    for (kind, text) in [
        (11, "18446744073709551616"),
        (11, "-1"),
        (8, "1.5"),
        (4, "128"),
        (12, "NaN"),
        (13, "inf"),
        (12, "1e100"),
        (3, "7"),
        (2, "1"),
        (8, " 5"),
    ] {
        assert!(encode(kind, text).is_err(), "{kind} {text}");
    }
}

#[test]
fn boolean_toggle_requires_canonical_bytes_and_exact_receipt_to_restore() {
    let (path, mut check) = reading();
    check["type_code"] = json!(2);
    check["bytes"] = json!([0]);
    check["value"] = json!({"boolean": false, "raw": 0});
    let patch = Patch::prepare(path.clone(), &check, "true", 100).unwrap();
    assert_eq!(patch.expected, [0]);
    assert_eq!(patch.replacement, [1]);
    assert_eq!(patch.check(2, 0x1010, &[0x1000], &[0], 101), Guard::Ready);
    assert_eq!(
        patch.check(2, 0x1010, &[0x1000], &[1], 101),
        Guard::ValueChanged
    );
    let report = json!({"operation":"patch_runtime_path","scope":path.scope,"patch_id":patch.id(),"checks":[check.clone()],
        "patch":{"state":"written","attempted":true,"transferred":1,"guard":"ready","observed_after":[1]}});
    let inverse = patch.restoration(&report, 200).unwrap();
    assert_eq!(inverse.expected, [1]);
    assert_eq!(inverse.replacement, [0]);
    assert!(Patch::prepare(path.clone(), &check, "false", 100).is_err());
    check["bytes"] = json!([2]);
    check["value"] = json!({"boolean":true,"raw":2});
    assert!(Patch::prepare(path, &check, "false", 100).is_err());
    for text in ["0", "1", "True", " false "] {
        assert!(encode(2, text).is_err());
    }
}

#[test]
fn trial_preparation_checks_evidence_bounds_and_finite_values() {
    let (path, check) = reading();
    let patch = Patch::prepare(path.clone(), &check, "4.5", 100).unwrap();
    assert_eq!(patch.replacement, 4.5f32.to_le_bytes());
    for desired in ["3.5", "24", "NaN"] {
        assert!(Patch::prepare(path.clone(), &check, desired, 100).is_err());
    }
    for (key, value) in [
        ("path_id", json!("wrong")),
        ("value", json!("9")),
        ("status", json!("null_reference")),
        ("snapshot_object_addresses", json!([])),
    ] {
        let mut bad = check.clone();
        bad[key] = value;
        assert!(Patch::prepare(path.clone(), &bad, "4.5", 100).is_err());
    }
    let mut bad = patch.clone();
    bad.replacement = f32::INFINITY.to_le_bytes().to_vec();
    assert!(bad.validate(&path.scope, 100).is_err());
    let mut other = path.scope.clone();
    other.created = "fedcba9876543210".into();
    assert!(patch.validate(&other, 100).is_err());
}

#[test]
fn paused_guard_distinguishes_replacement_drift_and_expiry() {
    let (path, check) = reading();
    let patch = Patch::prepare(path, &check, "4.5", 100).unwrap();
    let before = 3.5f32.to_le_bytes();
    assert_eq!(
        patch.check(12, 0x1010, &[0x1000], &before, 101),
        Guard::Ready
    );
    assert_eq!(
        patch.check(13, 0x1010, &[0x1000], &before, 101),
        Guard::RepresentationChanged
    );
    assert_eq!(
        patch.check(12, 0x1010, &[0x1000], &before[..2], 101),
        Guard::RepresentationChanged
    );
    assert_eq!(
        patch.check(12, 0x2010, &[0x1000], &before, 101),
        Guard::PathMoved
    );
    assert_eq!(
        patch.check(12, 0x1010, &[0x2000], &before, 101),
        Guard::PathMoved
    );
    assert_eq!(
        patch.check(12, 0x1010, &[0x1000], &4.5f32.to_le_bytes(), 101),
        Guard::ValueChanged
    );
    assert_eq!(
        patch.check(12, 0x1010, &[0x1000], &before, 99),
        Guard::Expired
    );
    assert_eq!(
        patch.check(12, 0x1010, &[0x1000], &before, 101 + LIFETIME_MS),
        Guard::Expired
    );
}

#[test]
fn restoration_requires_exact_confirmed_receipt_and_keeps_original_chain() {
    let (path, check) = reading();
    let patch = Patch::prepare(path.clone(), &check, "4.5", 100).unwrap();
    let report = json!({"operation":"patch_runtime_path","scope":path.scope,"patch_id":patch.id(),"checks":[check],
        "patch":{"state":"written","attempted":true,"transferred":4,"guard":"ready","observed_after":patch.replacement}});
    let inverse = patch.restoration(&report, 200).unwrap();
    assert_eq!(inverse.expected, patch.replacement);
    assert_eq!(inverse.replacement, patch.expected);
    assert_eq!(inverse.object_addresses, patch.object_addresses);
    assert_eq!(inverse.field_address, patch.field_address);
    assert_ne!(inverse.id(), patch.id());
    for (key, value) in [
        ("state", json!("uncertain")),
        ("transferred", json!(2)),
        ("attempted", json!(false)),
        ("observed_after", json!([0, 0, 0, 0])),
    ] {
        let mut bad = report.clone();
        bad["patch"][key] = value;
        assert!(patch.restoration(&bad, 200).is_err());
    }
    let mut bad = report.clone();
    bad["patch_id"] = json!("unrelated");
    assert!(patch.restoration(&bad, 200).is_err());
    let mut bad = report;
    bad["checks"][0]["snapshot_object_addresses"] = json!(["0x2000"]);
    assert!(patch.restoration(&bad, 200).is_err());
}
