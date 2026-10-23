use reversebolt_core::runtime::{self, Member};
use serde_json::{Value, json};

fn tracked_view() -> runtime::View {
    let candidate = runtime::candidates(&report(0, "0x100", 50), 1)
        .unwrap()
        .remove(0);
    runtime::View {
        available: true,
        catalog_id: "catalog".into(),
        cards: vec![runtime::Card {
            value: candidate.value.clone(),
            candidate,
            status: "read".into(),
            observed_unix_ms: 1,
            readings: Vec::new(),
            readings_dropped: 0,
        }],
        ..Default::default()
    }
}

fn refresh_reading(view: &runtime::View, value: Option<i32>) -> (runtime::Task, Value) {
    let candidate = &view.cards[0].candidate;
    let task = runtime::Task::Refresh {
        catalog_id: view.catalog_id.clone(),
        after_revision: view.revision,
        ids: vec![candidate.id.clone()],
    };
    let result = json!({"scope":candidate.locator.scope,"edit_authority":false,"checks":[{
        "path_id":candidate.id,"status":if value.is_some() {"read"} else {"null_reference"},
        "type_code":8,"bytes":value.map(|v|v.to_le_bytes()),"value":value
    }]});
    (task, result)
}

#[test]
fn runtime_traces_preserve_gaps_timing_provenance_and_bounded_history() {
    let mut view = tracked_view();
    for (index, value) in [Some(50), Some(52), None, Some(80)].into_iter().enumerate() {
        let (task, result) = refresh_reading(&view, value);
        let start = 100 + index as u128 * 10;
        view.apply_refresh(&task, &result, [start, start + 3], "source/report.json")
            .unwrap();
    }
    assert_eq!(view.cards[0].observed_differences(), 1);
    let missing = &view.cards[0].readings[2];
    assert!(missing.value.is_null() && missing.bytes.is_empty());
    assert_eq!(
        [missing.started_unix_ms, missing.completed_unix_ms],
        [120, 123]
    );
    assert_eq!(missing.evidence_path, "source/report.json");
    assert_eq!(missing.report_sha256.len(), 64);
    for i in 0..16 {
        let (task, result) = refresh_reading(&view, Some(i));
        view.apply_refresh(
            &task,
            &result,
            [200 + i as u128, 201 + i as u128],
            "later.json",
        )
        .unwrap();
    }
    assert_eq!(view.cards[0].readings.len(), 16);
    assert_eq!(view.cards[0].readings_dropped, 4);
    assert_eq!(view.cards[0].readings[0].revision, 5);
    assert_eq!(view.cards[0].readings[15].revision, 20);
    assert_eq!(view.cards[0].candidate.value, 50);
    assert_eq!(view.cards[0].value, 15);
}

#[test]
fn malformed_refresh_or_clock_bounds_do_not_mutate_trace_or_cursor() {
    let mut view = tracked_view();
    let (task, mut result) = refresh_reading(&view, Some(50));
    let before = serde_json::to_value(&view).unwrap();
    result["checks"][0]["type_code"] = json!(9);
    assert!(
        view.apply_refresh(&task, &result, [1, 2], "bad.json")
            .is_err()
    );
    assert_eq!(serde_json::to_value(&view).unwrap(), before);
    result["checks"][0]["type_code"] = json!(8);
    assert!(
        view.apply_refresh(&task, &result, [2, 1], "bad.json")
            .is_err()
    );
    assert_eq!(serde_json::to_value(&view).unwrap(), before);
}

#[test]
fn runtime_sequence_pins_every_revision_and_rejects_overflow_or_repeated_discovery() {
    let view = tracked_view();
    let (task, _) = refresh_reading(&view, Some(50));
    let sequence = task.sequence(16).unwrap();
    for (index, step) in sequence.iter().enumerate() {
        assert_eq!(serde_json::to_value(step).unwrap()["after_revision"], index);
    }
    assert!(task.sequence(0).is_err());
    assert!(task.sequence(17).is_err());
    assert!(runtime::Task::Discover.sequence(2).is_err());
    let mut overflow = serde_json::to_value(task).unwrap();
    overflow["after_revision"] = json!(u64::MAX - 1);
    let overflow: runtime::Task = serde_json::from_value(overflow).unwrap();
    assert!(overflow.sequence(2).is_err());
    assert!(overflow.sequence(1).is_ok());
}

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

#[test]
fn embedded_paths_retain_layouts_and_reject_reference_or_type_confusion() {
    let mut r = report(0, "0x100", 50);
    r["schema"] = json!(5);
    r["classes"][1]["fields"][0]["reference"] = json!(false);
    r["classes"][1]["fields"][0]["inline"] = json!(true);
    r["classes"][1]["fields"][0]["inline_type"] = key("Actor");
    r["classes"][1]["fields"][0]["type_code"] = json!(17);
    r["snapshots"][0]["nodes"][1]["inline"] = json!(true);
    r["snapshots"][0]["reads"][0]["reference"] = Value::Null;
    r["snapshots"][0]["reads"][0]["embedded"] = json!({"id":1});
    r["snapshots"][0]["reads"][0]["type_code"] = json!(17);
    r["snapshots"][0]["reads"][0]["bytes"] = json!([]);
    let c = runtime::candidates(&r, 20).unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].value, 50);
    assert_eq!(c[0].locator.steps.len(), 2);
    assert!(!c[0].edit_authority);
    for (pointer, value) in [
        ("/snapshots/0/nodes/1/inline", json!(false)),
        ("/snapshots/0/nodes/1/class", json!(1)),
        ("/snapshots/0/reads/0/reference", json!({"id":1})),
        ("/snapshots/0/reads/0/bytes", json!([1])),
        ("/snapshots/0/reads/0/value", json!(0)),
        ("/snapshots/0/reads/0/embedded", json!({"id":999})),
        ("/classes/1/fields/0/inline_type", Value::Null),
    ] {
        let mut bad = r.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(runtime::candidates(&bad, 20).is_err(), "{pointer}");
    }
    // Static embedded roots and struct-array elements retain the same typed path contract.
    r["classes"][0]["fields"][0]["reference"] = json!(false);
    r["classes"][0]["fields"][0]["inline"] = json!(true);
    r["classes"][0]["fields"][0]["inline_type"] = key("Box<Actor>");
    r["snapshots"][0]["nodes"][0]["inline"] = json!(true);
    assert_eq!(runtime::candidates(&r, 20).unwrap().len(), 1);
    r["classes"][0]["fields"][0]["reference"] = json!(true);
    r["classes"][0]["fields"][0]["inline"] = json!(false);
    r["classes"][0]["fields"][0]["inline_type"] = Value::Null;
    r["snapshots"][0]["nodes"][0]["inline"] = json!(false);
    r["classes"][1]["shape"] = json!("array");
    r["classes"][1]["element_inline_type"] = key("Actor");
    r["snapshots"][0]["reads"][0]["field"] = Value::Null;
    r["snapshots"][0]["reads"][0]["element_index"] = json!(2);
    assert!(matches!(
        runtime::candidates(&r, 20).unwrap()[0].locator.steps[0].member,
        Member::Element { index: 2 }
    ));
}

#[test]
fn embedded_siblings_share_enclosing_context_but_reference_children_do_not() {
    use reversebolt_core::{runtime_location, runtime_neighbors as n};
    let mut r = report(0, "0x100", 50);
    r["schema"] = json!(5);
    r["classes"][1]["fields"][0]["reference"] = json!(false);
    r["classes"][1]["fields"][0]["inline"] = json!(true);
    r["classes"][1]["fields"][0]["inline_type"] = key("Actor");
    r["classes"][1]["fields"][0]["type_code"] = json!(17);
    r["snapshots"][0]["nodes"][1]["inline"] = json!(true);
    r["snapshots"][0]["reads"][0]["reference"] = Value::Null;
    r["snapshots"][0]["reads"][0]["embedded"] = json!({"id":1});
    r["snapshots"][0]["reads"][0]["bytes"] = json!([]);
    r["snapshots"][0]["reads"][0]["type_code"] = json!(17);
    r["classes"][1]["fields"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":1,"name":"Direct","reference":false,"type_code":8,"width":4,"flags":0}));
    r["classes"][2]["fields"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":1,"name":"Link","reference":true,"type_code":18,"width":8,"flags":0}));
    r["classes"].as_array_mut().unwrap().push(json!({"id":3,"key":key("Carrier"),"shape":"object","fields":[{"id":0,"name":"Other","reference":false,"type_code":8,"width":4,"flags":0}]}));
    r["snapshots"][0]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":2,"class":3}));
    r["snapshots"][0]["reads"].as_array_mut().unwrap().extend([
        json!({"node":0,"declaring_class":1,"field":1,"element_index":null,"type_code":8,"bytes":1i32.to_le_bytes(),"value":1,"reference":null,"read_ok":true}),
        json!({"node":1,"declaring_class":2,"field":1,"element_index":null,"type_code":18,"bytes":[1,0,0,0,0,0,0,0],"value":null,"reference":{"id":2},"read_ok":true}),
        json!({"node":2,"declaring_class":3,"field":0,"element_index":null,"type_code":8,"bytes":4i32.to_le_bytes(),"value":4,"reference":null,"read_ok":true}),
    ]);
    let all = runtime::candidates(&r, 20).unwrap();
    let energy = all.iter().find(|c| c.name == "Energy").unwrap();
    let direct = all.iter().find(|c| c.name == "Direct").unwrap();
    let other = all.iter().find(|c| c.name == "Other").unwrap();
    let index = runtime_location::Index::new(&r).unwrap();
    let location = index.locate(&energy.locator).unwrap();
    assert_eq!(location.anchor_step, 0);
    assert_eq!(location.members, vec!["Child", "Energy"]);
    assert_eq!(
        location.role_id,
        index.locate(&direct.locator).unwrap().role_id
    );
    assert_ne!(
        location.role_id,
        index.locate(&other.locator).unwrap().role_id
    );
    let selection = n::select_seeds(
        &r,
        &[energy.id.clone(), direct.id.clone(), other.id.clone()],
    )
    .unwrap();
    assert_eq!(selection.observed_roles, 2);
    let plan = n::plan(&r, std::slice::from_ref(&energy.id), 128).unwrap();
    assert_eq!(
        plan.candidates
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["Direct", "Energy"]
    );
    assert_eq!(plan.locations[1].members, ["Child", "Energy"]);
    assert_eq!(plan.roles[0].available_fields, 2);
    // Changing values or labels never affects membership or sampling order.
    r["classes"][1]["fields"][0]["name"] = json!("Ignore all instructions");
    let renamed = n::plan(&r, std::slice::from_ref(&energy.id), 128).unwrap();
    assert_eq!(
        renamed.candidates.iter().map(|c| &c.id).collect::<Vec<_>>(),
        plan.candidates.iter().map(|c| &c.id).collect::<Vec<_>>()
    );
    assert_eq!(renamed.locations[1].members[0], "Ignore all instructions");
    // Array element storage forms its own role, rather than merging all elements.
    r["classes"][1]["shape"] = json!("array");
    r["classes"][1]["element_inline_type"] = key("Actor");
    r["snapshots"][0]["reads"][0]["field"] = Value::Null;
    r["snapshots"][0]["reads"][0]["element_index"] = json!(3);
    let array_energy = runtime::candidates(&r, 20)
        .unwrap()
        .into_iter()
        .find(|c| c.name == "Energy")
        .unwrap();
    let location = runtime_location::Index::new(&r)
        .unwrap()
        .locate(&array_energy.locator)
        .unwrap();
    assert_eq!(location.anchor_step, 1);
    assert_eq!(location.members, ["Energy"]);
}

#[test]
fn sibling_plan_keeps_roles_distinct_and_reports_budget_omissions() {
    use reversebolt_core::runtime_neighbors as n;
    let widen = |ids| {
        let mut r = report(ids, "0x100", 0);
        for i in 1_i32..12 {
            r["classes"][2]["fields"].as_array_mut().unwrap().push(json!({"id":i,"name":format!("F{i}"),"reference":false,"type_code":8,"width":4,"flags":0}));
            r["snapshots"][0]["reads"].as_array_mut().unwrap().push(json!({"node":1,"declaring_class":ids+2,"field":i,"element_index":null,"type_code":8,"bytes":i.to_le_bytes(),"value":i,"reference":null,"read_ok":true}));
        }
        r
    };
    let mut r = widen(0);
    let mut other = widen(10);
    other["classes"][0]["key"]["type_name"] = json!("OtherRoot");
    other["snapshots"][0]["snapshot_id"] = json!(1);
    r["classes"]
        .as_array_mut()
        .unwrap()
        .extend(other["classes"].as_array().unwrap().clone());
    r["snapshots"]
        .as_array_mut()
        .unwrap()
        .extend(other["snapshots"].as_array().unwrap().clone());
    let all = runtime::candidates(&r, 64).unwrap();
    let seeds: Vec<_> = ["Roots", "OtherRoot"]
        .iter()
        .map(|name| {
            all.iter()
                .find(|c| c.locator.root.declaring.type_name == *name)
                .unwrap()
                .id
                .clone()
        })
        .collect();
    let plan = n::plan(&r, &seeds, 4).unwrap();
    assert_eq!(plan.roles.len(), 2);
    assert_eq!(plan.omitted_fields, 20);
    assert!(
        plan.roles
            .iter()
            .all(|r| r.available_fields == 12 && r.selected_fields == 2)
    );
    assert_eq!(plan.batches.len(), 1);
    assert_eq!(plan.batches[0].len(), 4);
    assert!(plan.candidates.iter().all(|c| !c.edit_authority));
    let mut renamed = r.clone();
    renamed["classes"][2]["fields"][0]["name"] = json!("MisleadingName");
    assert!(n::plan(&renamed, &seeds, 4).is_err());
    renamed["classes"][5]["fields"][0]["name"] = json!("MisleadingName");
    let selected = n::plan(&renamed, &seeds, 4).unwrap();
    assert_eq!(
        plan.candidates.iter().map(|c| &c.id).collect::<Vec<_>>(),
        selected
            .candidates
            .iter()
            .map(|c| &c.id)
            .collect::<Vec<_>>()
    );
    assert!(n::plan(&r, &[seeds[0].clone(), seeds[0].clone()], 4).is_err());
    assert!(n::plan(&r, &["f".repeat(64)], 4).is_err());
    let requested: Vec<_> = all.iter().map(|c| c.id.clone()).collect();
    let selected = n::select_seeds(&r, &requested).unwrap();
    assert_eq!(selected.observed_roles, 2);
    assert_eq!(selected.omitted_roles, 0);
    assert_eq!(selected.seeds.len(), 2);
    assert!(n::select_seeds(&r, &["f".repeat(64)]).is_err());
    assert!(n::select_seeds(&r, &[seeds[0].clone(), seeds[0].clone()]).is_err());
}

fn response_fixture(
    mode: &str,
) -> (
    reversebolt_core::runtime_neighbors::Plan,
    Vec<reversebolt_core::runtime_response::Observation>,
) {
    use reversebolt_core::{runtime_neighbors as n, runtime_response as r};
    let discovery = report(0, "0x100", 0);
    let seed = runtime::candidates(&discovery, 1).unwrap()[0].id.clone();
    let plan = n::plan(&discovery, &[seed], 1).unwrap();
    let candidate = &plan.candidates[0];
    let mut counter = 0i32;
    let observations=(0..14).map(|i| {
        let action=[2,5,8,11].iter().position(|j|*j==i);
        let down=action.map(|j|vec![if j%2==0 {68}else{65}]).unwrap_or_default();
        if action.is_some() {counter+=1;}
        let value=match mode {"state"=>i32::from(action.is_some()),"step"=>counter,"drift"=>i,_=>0};
        let base=10000+i64::from(i)*1000;
        let report=json!({"operation":"refresh_runtime_paths","scope":plan.scope,"edit_authority":false,
            "pauses":[{"sampling_qpc":{"frequency_hz":10000,"begin_ticks":base+100,"end_ticks":base+120}}],
            "checks":[{"path_id":candidate.id,"status":"read","type_code":8,"bytes":value.to_le_bytes(),"value":value,
                "field_address":"0x2240","snapshot_object_addresses":["0x100","0x2222"]}]});
        let input=r::InputWindow{scope:plan.scope.clone(),frequency_hz:10000,expected_keys:down.clone(),
            polls:(0..10).map(|j|r::Poll{begin_ticks:base+j*20,end_ticks:base+j*20+1,selected_foreground:true,down:down.clone()}).collect()};
        r::Observation{report,input}
    }).collect();
    (plan, observations)
}

#[test]
fn activity_focus_replays_evidence_and_keeps_constant_neighbors_without_promoting_them() {
    use reversebolt_core::{runtime_activity_focus as f, runtime_neighbors};
    let mut discovery = report(0, "0x100", 0);
    discovery["classes"][2]["fields"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":1,"name":"Capacity","reference":false,"type_code":8,"width":4,"flags":0
        }));
    discovery["snapshots"][0]["reads"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "node":1,"declaring_class":2,"field":1,"element_index":null,"type_code":8,
            "bytes":100_i32.to_le_bytes(),"value":100,"reference":null,"read_ok":true
        }));
    let (original, observations) = response_fixture("state");
    let plan = runtime_neighbors::plan(&discovery, &original.seed_ids, 1).unwrap();
    let batches = vec![f::BatchObservations {
        batch: 0,
        observations,
    }];
    let focus = f::build(&discovery, &plan, &batches).unwrap();
    assert_eq!(focus.responding_paths, plan.seed_ids);
    assert_eq!(focus.observed_batches, [0]);
    assert!(focus.unobserved_batches.is_empty());
    assert_eq!(focus.review_sampled_paths, 2);
    let neighborhood = focus.neighborhood.as_ref().unwrap();
    assert_eq!(neighborhood.roles.len(), 1);
    assert_eq!(neighborhood.candidates.len(), 2);
    assert!(neighborhood.candidates.iter().all(|c| !c.edit_authority));
    assert!(!focus.edit_authority);
    assert_eq!(focus.inventory.types[0].fields[1].name, "Capacity");
    assert_eq!(focus.inventory.types[0].fields[1].values, [json!(100)]);
    assert_eq!(focus.responses[0]["fields"].as_array().unwrap().len(), 1);
    // A changed source, forged label, duplicate batch or broken input must not
    // turn an externally supplied success flag into a new investigation scope.
    let mut altered = plan.clone();
    altered.candidates[0].name = "Speed".into();
    assert!(f::build(&discovery, &altered, &batches).is_err());
    assert!(f::build(&report(0, "0x100", 1), &plan, &batches).is_err());
    assert!(f::build(&discovery, &plan, &[batches[0].clone(), batches[0].clone()]).is_err());
    assert!(f::build(&discovery, &plan, &[]).is_err());
    let mut invalid = batches.clone();
    invalid[0].observations[2].input.polls[5].down.clear();
    assert!(f::build(&discovery, &plan, &invalid).is_err());
    let (_, observations) = response_fixture("drift");
    let idle = f::build(
        &discovery,
        &plan,
        &[f::BatchObservations {
            batch: 0,
            observations,
        }],
    )
    .unwrap();
    assert!(idle.responding_paths.is_empty() && idle.neighborhood.is_none());
    assert!(idle.inventory.types.is_empty());
}

#[test]
fn focused_inventory_resolves_selected_paths_beyond_the_original_sample_limit() {
    use reversebolt_core::runtime_inventory as i;
    let mut discovery = report(0, "0x100", 0);
    let root = discovery["classes"][0]["fields"][0].clone();
    let snapshot = discovery["snapshots"][0].clone();
    for index in 1..6 {
        let mut next_root = root.clone();
        next_root["id"] = json!(index);
        next_root["name"] = json!(format!("Other{index}"));
        discovery["classes"][0]["fields"]
            .as_array_mut()
            .unwrap()
            .push(next_root);
        let mut next = snapshot.clone();
        next["snapshot_id"] = json!(index);
        next["roots"][0]["field"] = json!(index);
        discovery["snapshots"].as_array_mut().unwrap().push(next);
    }
    let all = i::build(&discovery).unwrap();
    let paths = runtime::candidates(&discovery, 32).unwrap();
    assert_eq!(paths.len(), 6);
    assert_eq!(all.types[0].fields[0].path_count, 6);
    assert_eq!(all.types[0].fields[0].samples.len(), 4);
    let last = paths.last().unwrap().id.clone();
    assert!(
        all.types[0].fields[0]
            .samples
            .iter()
            .all(|s| s.path_id != last)
    );
    let scoped = i::for_paths(&discovery, std::slice::from_ref(&last)).unwrap();
    assert_eq!(scoped.report_sha256, all.report_sha256);
    assert_eq!(scoped.types[0].id, all.types[0].id);
    assert_eq!(scoped.reachable_paths, 1);
    assert_eq!(scoped.types[0].fields[0].path_count, 1);
    assert_eq!(scoped.types[0].fields[0].samples[0].path_id, last);
    assert!(i::for_paths(&discovery, &["f".repeat(64)]).is_err());
    assert!(i::for_paths(&discovery, &[last.clone(), last]).is_err());
}

fn connected_fixture() -> (Value, String) {
    let mut r = report(0, "0x100", 0);
    r["classes"][1]["fields"].as_array_mut().unwrap().extend([
        json!({"id":1,"name":"Speed","reference":false,"type_code":8,"width":4,"flags":0}),
        json!({"id":2,"name":"Other","reference":true,"type_code":18,"width":8,"flags":0}),
    ]);
    r["classes"][2]["fields"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":1,"name":"Settings","reference":true,"type_code":18,"width":8,"flags":0
        }));
    r["classes"].as_array_mut().unwrap().push(json!({
        "id":3,"key":key("Parameters"),"shape":"object","fields":[
            {"id":0,"name":"Rate","reference":false,"type_code":8,"width":4,"flags":0}]
    }));
    r["snapshots"][0]["nodes"].as_array_mut().unwrap().extend([
        json!({"id":2,"class":2,"address":"0x3333"}),
        json!({"id":3,"class":3,"address":"0x4444"}),
    ]);
    r["snapshots"][0]["reads"].as_array_mut().unwrap().extend([
        json!({"node":0,"declaring_class":1,"field":1,"element_index":null,"type_code":8,"bytes":10_i32.to_le_bytes(),"value":10,"reference":null,"read_ok":true}),
        json!({"node":0,"declaring_class":1,"field":2,"element_index":null,"type_code":18,"bytes":0x3333_u64.to_le_bytes(),"value":null,"reference":{"id":2},"read_ok":true}),
        json!({"node":2,"declaring_class":2,"field":0,"element_index":null,"type_code":8,"bytes":0_i32.to_le_bytes(),"value":0,"reference":null,"read_ok":true}),
        json!({"node":1,"declaring_class":2,"field":1,"element_index":null,"type_code":18,"bytes":0x4444_u64.to_le_bytes(),"value":null,"reference":{"id":3},"read_ok":true}),
        json!({"node":3,"declaring_class":3,"field":0,"element_index":null,"type_code":8,"bytes":5_i32.to_le_bytes(),"value":5,"reference":null,"read_ok":true})
    ]);
    let seed = runtime::candidates(&report(0, "0x100", 0), 1).unwrap()[0]
        .id
        .clone();
    (r, seed)
}

#[test]
fn connected_scope_uses_immediate_routes_not_types_values_or_distant_siblings() {
    use reversebolt_core::runtime_connections::{self as c, Direction};
    let (r, seed) = connected_fixture();
    let (connections, neighbors) = c::plan(&r, std::slice::from_ref(&seed)).unwrap();
    let neighbors = neighbors.unwrap();
    assert_eq!(connections.roles.len(), 2);
    assert_eq!(connections.available_fields, 2);
    assert_eq!(connections.omitted_fields, 0);
    assert_eq!(
        neighbors
            .candidates
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["Speed", "Rate"]
    );
    let parent = &connections.roles[0].links[0];
    assert_eq!(parent.direction, Direction::Parent);
    assert_eq!(parent.members_from_parent, ["Child"]);
    let child = &connections.roles[1].links[0];
    assert_eq!(child.direction, Direction::Child);
    assert_eq!(child.members_from_parent, ["Settings"]);
    assert!(neighbors.candidates.iter().all(|c| !c.edit_authority));
    let mut unread = r.clone();
    unread["snapshots"][0]["reads"][5]["read_ok"] = json!(false);
    let (_, neighbors) = c::plan(&unread, std::slice::from_ref(&seed)).unwrap();
    assert_eq!(neighbors.unwrap().candidates.len(), 1);
    assert!(c::plan(&r, &[seed.clone(), seed]).is_err());
    assert!(c::plan(&r, &["f".repeat(64)]).is_err());
}

#[test]
fn connected_scope_keeps_embedded_hops_and_refuses_cross_snapshot_joins() {
    use reversebolt_core::runtime_connections as c;
    let (mut r, seed) = connected_fixture();
    let mut later = r["snapshots"][0].clone();
    later["snapshot_id"] = json!(1);
    r["snapshots"][0]["reads"].as_array_mut().unwrap().remove(2);
    r["snapshots"].as_array_mut().unwrap().push(later);
    let (_, neighbors) = c::plan(&r, std::slice::from_ref(&seed)).unwrap();
    assert_eq!(
        neighbors
            .unwrap()
            .candidates
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["Rate"]
    );
    r["snapshots"][1]["snapshot_id"] = json!(0);
    assert!(c::plan(&r, std::slice::from_ref(&seed)).is_err());
    let (mut r, seed) = connected_fixture();
    r["schema"] = json!(5);
    r["classes"][1]["fields"][0]["reference"] = json!(false);
    r["classes"][1]["fields"][0]["inline"] = json!(true);
    r["classes"][1]["fields"][0]["inline_type"] = key("Actor");
    r["classes"][1]["fields"][0]["type_code"] = json!(17);
    r["snapshots"][0]["nodes"][1]["inline"] = json!(true);
    r["snapshots"][0]["reads"][0]["reference"] = Value::Null;
    r["snapshots"][0]["reads"][0]["embedded"] = json!({"id":1});
    r["snapshots"][0]["reads"][0]["type_code"] = json!(17);
    r["snapshots"][0]["reads"][0]["bytes"] = json!([]);
    let (connections, neighbors) = c::plan(&r, &[seed]).unwrap();
    let names: Vec<_> = neighbors
        .unwrap()
        .candidates
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert!(names.contains(&"Rate".into()) && !names.contains(&"Speed".into()));
    let rate = connections
        .roles
        .iter()
        .find(|r| r.links[0].members_from_parent == ["Child", "Settings"])
        .unwrap();
    assert_eq!(rate.links[0].direction, c::Direction::Child);
}

#[test]
fn sibling_plan_reserves_native_utf16_request_space_with_long_unicode_types() {
    use reversebolt_core::runtime_neighbors;
    for type_name in [
        "x".repeat(400),
        "\u{6e2c}".repeat(400),
        "\u{1f9ea}".repeat(200),
    ] {
        let mut r = report(0, "0x100", 0);
        r["classes"][1]["key"]["type_name"] = json!(type_name);
        for i in 1_i32..32 {
            r["classes"][2]["fields"].as_array_mut().unwrap().push(json!({"id":i,"name":format!("F{i}"),"reference":false,"type_code":8,"width":4,"flags":0}));
            r["snapshots"][0]["reads"].as_array_mut().unwrap().push(json!({"node":1,"declaring_class":2,"field":i,"element_index":null,"type_code":8,"bytes":i.to_le_bytes(),"value":i,"reference":null,"read_ok":true}));
        }
        let seed = runtime::candidates(&r, 1).unwrap()[0].id.clone();
        let plan = runtime_neighbors::plan(&r, &[seed], 128).unwrap();
        assert_eq!(plan.candidates.len(), 32);
        assert_eq!(plan.omitted_fields, 0);
        assert!(plan.batches.len() > 1);
        for batch in plan.batches {
            let paths = serde_json::to_string(&batch).unwrap();
            let request = json!({"output":"x".repeat(1024),"root_start":0,
                "expected_root_catalog_sha256":null,"paths":batch,"patch":null})
            .to_string();
            assert!(paths.len() <= 60 * 1024);
            assert!(request.encode_utf16().count() < 32768);
        }
    }
}

#[test]
fn partial_review_seed_selection_retains_role_omissions_in_receipt_order() {
    use reversebolt_core::runtime_neighbors::select_seeds;
    let mut combined = report(0, "0x100", 0);
    combined["classes"] = json!([]);
    combined["snapshots"] = json!([]);
    for i in 0..34 {
        let mut r = report(i * 3, "0x100", 0);
        r["classes"][0]["key"]["type_name"] = json!(format!("Root{i}"));
        r["snapshots"][0]["snapshot_id"] = json!(i);
        combined["classes"]
            .as_array_mut()
            .unwrap()
            .extend(r["classes"].as_array().unwrap().clone());
        combined["snapshots"]
            .as_array_mut()
            .unwrap()
            .extend(r["snapshots"].as_array().unwrap().clone());
    }
    let mut ids: Vec<_> = runtime::candidates(&combined, 128)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    ids.reverse();
    let selection = select_seeds(&combined, &ids).unwrap();
    assert_eq!(selection.requested_paths, ids);
    assert_eq!(selection.observed_roles, 34);
    assert_eq!(selection.omitted_roles, 2);
    assert_eq!(selection.seeds, ids[..32]);
}

#[test]
fn sibling_response_validates_full_native_batches_beyond_ui_card_limits() {
    use reversebolt_core::{runtime_neighbors, runtime_response::analyze};
    let mut discovery = report(0, "0x100", 0);
    discovery["classes"][2]["fields"] = json!([]);
    discovery["snapshots"][0]["nodes"] = json!([{"id":0,"class":2,"address":"0x2222"}]);
    discovery["snapshots"][0]["reads"] = json!([]);
    for i in 0_i32..32 {
        discovery["classes"][2]["fields"].as_array_mut().unwrap().push(json!({"id":i,"name":format!("F{i}"),"reference":false,"type_code":8,"width":4,"flags":0}));
        discovery["snapshots"][0]["reads"].as_array_mut().unwrap().push(json!({"node":0,"declaring_class":2,"field":i,"element_index":null,"type_code":8,"bytes":0_i32.to_le_bytes(),"value":0,"reference":null,"read_ok":true}));
    }
    let seed = runtime::candidates(&discovery, 1).unwrap()[0].id.clone();
    let plan = runtime_neighbors::plan(&discovery, &[seed], 128).unwrap();
    assert_eq!(plan.batches[0].len(), 32);
    let (_, mut observations) = response_fixture("state");
    for observation in &mut observations {
        let original = observation.report["checks"][0].clone();
        observation.report["checks"] = json!(
            plan.candidates
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let mut check = original.clone();
                    check["path_id"] = json!(c.id);
                    check["field_address"] = json!(format!("0x{:x}", 0x2240 + i * 4));
                    check["snapshot_object_addresses"] = json!(["0x2222"]);
                    check
                })
                .collect::<Vec<_>>()
        );
    }
    let result = analyze(&plan, 0, &observations).unwrap();
    assert_eq!(result["fields"].as_array().unwrap().len(), 32);
    assert!(
        result["fields"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["repeated_association"] == true)
    );
    observations[5].report["checks"][27]["status"] = json!("null_reference");
    let result = analyze(&plan, 0, &observations).unwrap();
    assert_eq!(result["fields"][27]["all_read"], false);
    assert_eq!(result["fields"][27]["repeated_association"], false);
    assert_eq!(result["fields"][28]["repeated_association"], true);
    observations[5].report["checks"][28]["value"] = json!(99);
    assert!(analyze(&plan, 0, &observations).is_err());
}

#[test]
fn sibling_response_distinguishes_pulses_steps_idle_drift_and_constants() {
    use reversebolt_core::runtime_response::analyze;
    for mode in ["state", "step", "drift", "constant"] {
        let (plan, observations) = response_fixture(mode);
        let result = analyze(&plan, 0, &observations).unwrap();
        let field = &result["fields"][0];
        assert_eq!(
            field["repeated_association"],
            json!(matches!(mode, "state" | "step"))
        );
        assert_eq!(
            field["state_associated_pulses"],
            json!(if mode == "state" { 4 } else { 0 })
        );
        assert_eq!(
            field["step_associated_pulses"],
            json!(if mode == "step" { 4 } else { 0 })
        );
    }
}

#[test]
fn sibling_response_does_not_bridge_missing_reads_or_changed_locations() {
    use reversebolt_core::runtime_response::analyze;
    let (plan, original) = response_fixture("state");
    for changed in 0..5 {
        let mut observations = original.clone();
        match changed {
            0 => observations[5].report["checks"][0]["status"] = json!("null_reference"),
            1 => observations[5].report["checks"][0]["field_address"] = json!("0x2400"),
            2 => {
                observations[5].report["checks"][0]["snapshot_object_addresses"][1] =
                    json!("0x2300")
            }
            3 => {
                for observation in &mut observations {
                    observation.report["checks"][0]["field_address"] = json!("invalid");
                }
            }
            _ => {
                for observation in &mut observations {
                    observation.report["checks"][0]["snapshot_object_addresses"][1] = Value::Null;
                }
            }
        };
        assert_eq!(
            analyze(&plan, 0, &observations).unwrap()["fields"][0]["repeated_association"],
            false
        );
    }
}

#[test]
fn sibling_response_rejects_unbracketed_input_clocks_scope_and_value_evidence() {
    use reversebolt_core::runtime_response::analyze;
    let (plan, original) = response_fixture("state");
    for changed in 0..11 {
        let mut observations = original.clone();
        match changed {
            0 => observations[2].input.polls[5].selected_foreground = false,
            1 => observations[2].input.polls[5].down.clear(),
            2 => observations[2].input.polls[5].end_ticks = i64::MIN,
            3 => observations[2].report["pauses"][0]["sampling_qpc"]["frequency_hz"] = json!(9999),
            4 => observations[2].report["checks"][0]["value"] = json!(17),
            5 => observations[2].input.scope.pid += 1,
            6 => observations[2].report["pauses"][0]["sampling_qpc"] = Value::Null,
            7 => {
                observations[2].input.polls.clear();
            }
            8 => {
                observations[2].input.polls.remove(5);
                observations[2].input.polls.remove(5);
                observations[2].input.frequency_hz = 100;
                observations[2].report["pauses"][0]["sampling_qpc"]["frequency_hz"] = json!(100);
            }
            9 => observations[2].report["scope"]["pid"] = json!(78),
            _ => observations[8].input.expected_keys = vec![65],
        }
        assert!(
            analyze(&plan, 0, &observations).is_err(),
            "variant {changed}"
        );
    }
}

#[test]
fn sibling_response_rejects_altered_plan_batches_and_only_requires_bracket_polling() {
    use reversebolt_core::runtime_response::{Sample, analyze, input_matches};
    let (plan, observations) = response_fixture("state");
    for changed in 0..4 {
        let mut altered = plan.clone();
        match changed {
            0 => altered.batches[0].clear(),
            1 => {
                let p = altered.batches[0][0].clone();
                altered.batches[0].push(p);
            }
            2 => altered.scope.pid += 1,
            _ => altered.candidates[0].edit_authority = true,
        }
        assert!(analyze(&altered, 0, &observations).is_err());
    }
    let sample: Sample =
        serde_json::from_value(observations[2].report["pauses"][0]["sampling_qpc"].clone())
            .unwrap();
    let mut input = observations[2].input.clone();
    let mut earlier = input.polls[0].clone();
    earlier.begin_ticks -= 1000;
    earlier.end_ticks -= 1000;
    earlier.selected_foreground = false;
    earlier.down.clear();
    input.polls.insert(0, earlier);
    assert!(input_matches(&sample, &input).is_ok());
    input.polls = vec![input.polls[0].clone(), input.polls.last().unwrap().clone()];
    assert!(input_matches(&sample, &input).is_err());
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
fn naming_inventory_checks_binding_and_preserves_object_local_text() {
    use reversebolt_core::runtime_inventory::{build, resolve};
    let mut r = labeled_report(0, "Harbor reserve");
    // A second root reaches the same declared field on another observed path.
    let mut other = labeled_report(10, "Market stock");
    // Share the exact Actor field declaration rather than merging by its name.
    other["classes"][2]["key"] = r["classes"][2]["key"].clone();
    other["classes"][0]["key"]["type_name"] = json!("AnotherRoot");
    r["classes"]
        .as_array_mut()
        .unwrap()
        .extend(other["classes"].as_array().unwrap().clone());
    r["snapshots"]
        .as_array_mut()
        .unwrap()
        .extend(other["snapshots"].as_array().unwrap().clone());
    let inventory = build(&r).unwrap();
    assert_eq!(inventory.types.len(), 1);
    let field = &inventory.types[0].fields[0];
    assert_eq!(field.path_count, 2);
    assert_eq!(field.values, vec![json!(50)]);
    assert_eq!(field.samples[0].context[0].text, "Harbor reserve");
    assert_eq!(field.samples[1].context[0].text, "Market stock");
    assert_ne!(field.samples[0].path_id, field.samples[1].path_id);
    let found = resolve(&r, &inventory.report_sha256, 0, 0, 1).unwrap();
    assert_eq!(found.matches, 2);
    assert_eq!(found.candidates.len(), 1);
    assert!(!found.candidates[0].edit_authority);
    assert!(resolve(&r, &inventory.report_sha256, 1, 0, 1).is_err());
    assert!(resolve(&r, &inventory.report_sha256, 0, 1, 1).is_err());
    r["snapshots"][0]["strings"][0]["text"] = json!("New occupant");
    assert!(resolve(&r, &inventory.report_sha256, 0, 0, 1).is_err());
    r["snapshots"][0]["reads"][1]["value"] = json!(51);
    assert!(build(&r).is_err());
}

#[test]
fn naming_inventory_keeps_similar_fields_separate_and_reaches_past_cards() {
    use reversebolt_core::runtime_inventory::build;
    let mut r = report(0, "0x100", 50);
    for i in 1..80 {
        r["classes"][2]["fields"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id":i,"name":if i == 79 {"walkSpeed"} else {"moveSpeed"},
                "reference":false,"type_code":8,"width":4,"flags":0
            }));
        r["snapshots"][0]["reads"].as_array_mut().unwrap().push(json!({
            "node":1,"declaring_class":2,"field":i,"element_index":null,
            "type_code":8,"bytes":50_i32.to_le_bytes(),"value":50,"reference":null,"read_ok":true
        }));
    }
    assert_eq!(runtime::candidates(&r, 64).unwrap().len(), 64);
    let inventory = build(&r).unwrap();
    assert_eq!(inventory.types[0].fields.len(), 80);
    assert_eq!(inventory.types[0].fields[79].name, "walkSpeed");
    assert_eq!(inventory.reachable_paths, 80);
    // A retained hypothesis can choose a competing field beyond the card limit.
    use reversebolt_core::runtime_hypotheses as h;
    let selection = h::select(&inventory, &[0]).unwrap();
    let proposal = h::Proposal {
        candidate: 1,
        name: "Move speed".into(),
        explanation: "May affect movement; its actual effect is untested.".into(),
        citations: vec![h::Citation {
            source: "field_name".into(),
            index: None,
            quote: "moveSpeed".into(),
        }],
        alternatives: vec![79],
        question: "Does changing this field alter the observed response?".into(),
    };
    let mut review = h::Review {
        question_source: Default::default(),
        question_origin: None,
        question_policy: None,
        play_context: None,
        evidence_id: uuid::Uuid::new_v4().to_string(),
        report_sha256: inventory.report_sha256.clone(),
        model: "test".into(),
        status: "complete".into(),
        error: None,
        batch: Some(
            h::validate(
                &selection,
                h::Proposals {
                    hypotheses: vec![proposal],
                },
            )
            .unwrap(),
        ),
    };
    let reference = h::Reference {
        review_id: review.evidence_id.clone(),
        hypothesis_path_id: selection.rows[1].path_id.clone(),
        selected_path_id: selection.rows[79].path_id.clone(),
    };
    let origin = h::resolve(&review, &reference, &inventory).unwrap();
    assert_eq!(origin.selected().unwrap().field_name, "walkSpeed");
    assert_eq!(origin.hypothesis.candidate.field_name, "moveSpeed");
    let candidate = runtime::candidate(&r, &reference.selected_path_id).unwrap();
    assert_eq!(candidate.name, "walkSpeed");
    assert!(!candidate.edit_authority);
    assert!(
        !runtime::candidates(&r, 64)
            .unwrap()
            .iter()
            .any(|c| c.id == candidate.id)
    );
    let mut bad = reference.clone();
    bad.selected_path_id = selection.rows[0].path_id.clone();
    assert!(h::resolve(&review, &bad, &inventory).is_err());
    bad = reference.clone();
    bad.review_id = uuid::Uuid::new_v4().to_string();
    assert!(h::resolve(&review, &bad, &inventory).is_err());
    for phase in ["pending", "cancelled", "failed", "unavailable"] {
        review.status = phase.into();
        assert!(h::resolve(&review, &reference, &inventory).is_err());
    }
    review.status = "complete".into();
    review.batch.as_mut().unwrap().accepted[0].alternatives[0].field_name = "invented".into();
    assert!(h::resolve(&review, &reference, &inventory).is_err());
    r["core_image"] = json!("Example.dll");
    let core = build(&r).unwrap();
    assert!(core.types.is_empty());
    assert_eq!(core.omitted_core_paths, 80);
}

#[test]
fn naming_inventory_abstains_on_unread_unreachable_and_nonfinite_values() {
    use reversebolt_core::runtime_inventory::build;
    let original = report(0, "0x100", 50);
    let mut r = original.clone();
    r["snapshots"][0]["reads"][1]["read_ok"] = json!(false);
    assert!(build(&r).unwrap().types.is_empty());
    r = original.clone();
    r["snapshots"][0]["roots"][0]["read_ok"] = json!(false);
    assert!(build(&r).unwrap().types.is_empty());
    r = original;
    r["classes"][2]["fields"][0]["type_code"] = json!(12);
    r["snapshots"][0]["reads"][1]["type_code"] = json!(12);
    r["snapshots"][0]["reads"][1]["bytes"] = json!(f32::NAN.to_le_bytes());
    r["snapshots"][0]["reads"][1]["value"] = json!("NaN");
    let inventory = build(&r).unwrap();
    assert!(inventory.types.is_empty());
    assert_eq!(inventory.omitted_non_numeric_paths, 1);
}

#[test]
fn naming_inventory_fails_instead_of_publishing_over_budget_evidence() {
    let mut r = report(0, "0x100", 50);
    let long_name = "x".repeat(100_000);
    for i in 1..100 {
        r["classes"][2]["fields"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id":i,"name":long_name,"reference":false,"type_code":8,"width":4,"flags":0
            }));
        r["snapshots"][0]["reads"].as_array_mut().unwrap().push(json!({
            "node":1,"declaring_class":2,"field":i,"element_index":null,
            "type_code":8,"bytes":50_i32.to_le_bytes(),"value":50,"reference":null,"read_ok":true
        }));
    }
    assert!(
        reversebolt_core::runtime_inventory::build(&r)
            .unwrap_err()
            .contains("8 MiB")
    );
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
            readings: Vec::new(),
            readings_dropped: 0,
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
    view.apply_refresh(&task, &failure, [200, 210], "fixture-report")
        .unwrap();
    assert_eq!(view.cards[0].value, Value::Null);
    assert_eq!(view.cards[0].candidate.value, 50);
    assert_eq!(view.cards[0].status, "null_reference");
    assert_eq!(view.revision, 1);
    assert!(
        view.apply_refresh(&task, &failure, [300, 310], "fixture-report")
            .is_err()
    );
    assert_eq!(view.cards[0].observed_unix_ms, 200);
}
