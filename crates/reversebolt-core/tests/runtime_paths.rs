use reversebolt_core::runtime::{self, Member};
use serde_json::{Value, json};

fn key(name: &str) -> Value {
    json!({"image":"Example.dll","type_name":name,"layout_sha256":"a".repeat(64)})
}
fn report(ids: usize, address: &str, value: i32) -> Value {
    json!({"schema":4,"pid":77,"created":"0123456789abcdef","session":2,"edit_authority":false,
      "classes":[
        {"id":ids,"key":key("Roots"),"shape":"object","fields":[
          {"id":0,"name":"Current","reference":true,"type_code":18,"width":8,"flags":16}]},
        {"id":ids+1,"key":key("Box<Actor>"),"shape":"object","fields":[
          {"id":0,"name":"Child","reference":true,"type_code":18,"width":8,"flags":0}]},
        {"id":ids+2,"key":key("Actor"),"shape":"object","fields":[
          {"id":0,"name":"Energy","reference":false,"type_code":8,"width":4,"flags":0}]}],
      "snapshots":[{"snapshot_id":0,"nodes":[
        {"id":0,"class":ids+1,"address":address},{"id":1,"class":ids+2,"address":"0x2222"}],
        "roots":[{"class":ids,"field":0,"read_ok":true,"node":{"id":0}}],
        "reads":[
          {"node":0,"declaring_class":ids+1,"field":0,"element_index":null,"type_code":18,"bytes":[34,34,0,0,0,0,0,0],"value":null,"reference":{"id":1},"read_ok":true},
          {"node":1,"declaring_class":ids+2,"field":0,"element_index":null,"type_code":8,"bytes":value.to_le_bytes(),"value":value,"reference":null,"read_ok":true}]}]})
}

fn labeled_report(ids: usize, label: &str) -> Value {
    let mut r = report(ids, "0x1234", 50);
    r["classes"][0]["key"]["type_name"] = json!(format!("Roots{ids}"));
    r["classes"][2]["fields"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":1,"name":"Caption","reference":true,"type_code":14,"width":8,"flags":0}));
    r["classes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":ids+3,"key":key("Text"),"shape":"string","fields":[]}));
    r["snapshots"][0]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":2,"class":ids+3}));
    r["snapshots"][0]["reads"].as_array_mut().unwrap().push(json!({"node":1,"declaring_class":ids+2,"field":1,"element_index":null,"type_code":14,"bytes":vec![0;8],"value":null,"reference":{"id":2},"read_ok":true}));
    r["snapshots"][0]["strings"] =
        json!([{"node":2,"read_ok":true,"text":label,"truncated":false}]);
    r
}

#[test]
fn same_object_text_is_searchable_evidence_not_path_identity() {
    let r = labeled_report(0, "Harbor reserve");
    let a = runtime::search(&r, "harbor energy", 0).unwrap();
    assert_eq!(a.matches, 1);
    assert_eq!(a.candidates[0].context[0].text, "Harbor reserve");
    assert_eq!(a.candidates[0].context[0].field.field_id, 1);
    assert_eq!(runtime::search(&r, "harbor missing", 0).unwrap().matches, 0);
    let b = runtime::search(&labeled_report(0, "A different occupant"), "occupant", 0).unwrap();
    assert_eq!(a.candidates[0].id, b.candidates[0].id);
    assert!(!a.candidates[0].edit_authority);
    let mut unread = r.clone();
    unread["snapshots"][0]["reads"][2]["read_ok"] = json!(false);
    assert_eq!(runtime::search(&unread, "harbor", 0).unwrap().matches, 0);
    let mut malformed = r;
    malformed["snapshots"][0]["strings"][0]["node"] = json!(1);
    assert!(runtime::search(&malformed, "harbor", 0).is_err());
}

#[test]
fn search_reaches_beyond_display_limit_and_pages_without_changing_ids() {
    let mut r = labeled_report(0, "Shared item 0");
    for i in 1..65 {
        let more = labeled_report(i * 10, &format!("Shared item {i}"));
        r["classes"]
            .as_array_mut()
            .unwrap()
            .extend(more["classes"].as_array().unwrap().clone());
        r["snapshots"]
            .as_array_mut()
            .unwrap()
            .extend(more["snapshots"].as_array().unwrap().clone());
    }
    let shown = runtime::candidates(&r, 64).unwrap();
    let late = runtime::search(&r, "item 64", 0).unwrap();
    assert_eq!(late.matches, 1);
    assert!(!shown.iter().any(|c| c.id == late.candidates[0].id));
    let first = runtime::search(&r, "shared", 0).unwrap();
    let second = runtime::search(&r, "shared", 20).unwrap();
    assert_eq!(first.matches, 65);
    assert_eq!(first.candidates.len(), 20);
    assert_eq!(second.candidates.len(), 20);
    assert!(
        first
            .candidates
            .iter()
            .all(|a| second.candidates.iter().all(|b| a.id != b.id))
    );
    assert_eq!(first.candidates[0].id, shown[0].id);
    assert!(runtime::search(&r, "", 4097).is_err());
}

#[test]
fn showing_search_results_preserves_newer_readings_and_rejects_old_catalogues() {
    let page = runtime::search(&labeled_report(0, "Harbor reserve"), "harbor", 0).unwrap();
    let candidate = page.candidates[0].clone();
    let mut v = runtime::View {
        catalog_id: "catalog".into(),
        search: Some(page),
        catalog_source: Some(runtime::CatalogSource {
            path: "saved.json".into(),
            document_sha256: "a".repeat(64),
            observed_unix_ms: 10,
        }),
        ..Default::default()
    };
    let ids = vec![candidate.id.clone()];
    assert!(v.show_values("older", 0, &ids).is_err());
    assert!(v.cards.is_empty());
    v.show_values("catalog", 0, &ids).unwrap();
    assert_eq!(v.cards[0].observed_unix_ms, 10);
    v.cards[0].value = json!(31);
    v.cards[0].observed_unix_ms = 20;
    v.show_values("catalog", 1, &ids).unwrap();
    assert_eq!(v.cards[0].value, 31);
    assert_eq!(v.cards[0].observed_unix_ms, 20);
    assert_eq!(v.cards[0].candidate.value, 50);
    assert_eq!(v.card_generation, 2);
    assert!(v.show_values("catalog", 1, &ids).is_err());
    assert_eq!(v.revision, 2);
}

#[test]
fn path_identity_ignores_addresses_values_and_catalogue_numbering() {
    let old = runtime::candidates(&report(0, "0x1234", 50), 8)
        .unwrap()
        .remove(0);
    let new = runtime::candidates(&report(100, "0xabcd", 19), 8)
        .unwrap()
        .remove(0);
    assert_eq!(old.id, new.id);
    assert_eq!(old.locator, new.locator);
    assert_ne!(old.value, new.value);
    assert_eq!(old.locator.steps.len(), 2);
    assert!(matches!(old.locator.steps[0].member, Member::Field { .. }));
    assert_eq!(old.locator.steps[1].owner.type_name, "Actor");
    assert_eq!(old.name, "Energy");
    assert!(!old.edit_authority);
    let encoded = serde_json::to_string(&old.locator).unwrap();
    assert!(!encoded.contains("0x1234") && !encoded.contains("0x2222"));
}

#[test]
fn generic_type_arguments_and_process_scope_change_the_reference() {
    let a = runtime::candidates(&report(0, "0x1234", 50), 8)
        .unwrap()
        .remove(0);
    let mut changed = report(0, "0x1234", 50);
    changed["classes"][1]["key"]["type_name"] = json!("Box<Enemy>");
    let b = runtime::candidates(&changed, 8).unwrap().remove(0);
    assert_ne!(a.id, b.id);
    let mut scope = a.locator.scope.clone();
    scope.created = "0123456789abcdee".into();
    assert!(a.locator.validate(&scope).is_err());
}

#[test]
fn contradictory_bytes_and_dangling_edges_never_create_candidates() {
    let mut value = report(0, "0x1234", 50);
    value["snapshots"][0]["reads"][1]["value"] = json!(90);
    assert!(runtime::candidates(&value, 8).is_err());
    let mut edge = report(0, "0x1234", 50);
    edge["snapshots"][0]["reads"][0]["reference"] = json!({"id":9});
    assert!(runtime::candidates(&edge, 8).is_err());
    let mut missing = report(0, "0x1234", 50);
    missing["snapshots"][0]["reads"][0]["reference"] = json!({"status":"null_or_not_followed"});
    assert!(runtime::candidates(&missing, 8).unwrap().is_empty());
}

#[test]
fn unknown_runtime_types_and_unread_edges_abstain() {
    let mut value = report(0, "0x1234", 50);
    value["snapshots"][0]["nodes"][0]["class"] = Value::Null;
    assert!(runtime::candidates(&value, 8).unwrap().is_empty());
    let mut value = report(0, "0x1234", 50);
    value["snapshots"][0]["roots"][0]["read_ok"] = json!(false);
    assert!(runtime::candidates(&value, 8).unwrap().is_empty());
}

#[test]
fn late_non_core_values_displace_early_framework_values_at_the_limit() {
    let mut value = report(0, "0x100", 50);
    value["core_image"] = json!("Example.dll");
    let mut extra = report(10, "0x200", 70);
    for c in extra["classes"].as_array_mut().unwrap() {
        c["key"]["image"] = json!("FreshApplication.dll");
    }
    value["classes"]
        .as_array_mut()
        .unwrap()
        .extend(extra["classes"].as_array().unwrap().clone());
    value["snapshots"]
        .as_array_mut()
        .unwrap()
        .extend(extra["snapshots"].as_array().unwrap().clone());
    let chosen = runtime::candidates(&value, 1).unwrap();
    assert_eq!(chosen.len(), 1);
    assert_eq!(chosen[0].value, 70);
    assert_eq!(
        chosen[0].locator.steps.last().unwrap().owner.image,
        "FreshApplication.dll"
    );
}

#[test]
fn refresh_rejects_stale_revisions_and_invalidates_unavailable_current_values() {
    let candidate = runtime::candidates(&report(0, "0x100", 50), 1)
        .unwrap()
        .remove(0);
    let mut view = runtime::View {
        available: true,
        catalog_id: "catalog".into(),
        cards: vec![runtime::Card {
            value: candidate.value.clone(),
            status: "read".into(),
            observed_unix_ms: 1,
            candidate: candidate.clone(),
        }],
        ..Default::default()
    };
    let task = runtime::Task::Refresh {
        catalog_id: "catalog".into(),
        after_revision: 0,
        ids: vec![candidate.id.clone()],
    };
    let failure = json!({"scope":candidate.locator.scope,"edit_authority":false,"checks":[{"path_id":candidate.id,"status":"null_reference"}]});
    view.apply_refresh(&task, &failure, 200).unwrap();
    assert_eq!(view.cards[0].value, Value::Null);
    assert_eq!(view.cards[0].candidate.value, 50);
    assert_eq!(view.cards[0].status, "null_reference");
    assert_eq!(view.revision, 1);
    assert!(view.apply_refresh(&task, &failure, 300).is_err());
    assert_eq!(view.cards[0].observed_unix_ms, 200);
}
