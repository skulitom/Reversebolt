use super::*;

fn fixture() -> (Input, Vec<Value>) {
    let (context, frames, evidence) = crate::runtime_planning::tests::observed();
    let actions = crate::runtime_planning::actions(&context, &frames, &evidence).unwrap();
    let choices: Vec<_>=(0..4).map(|i|json!({"selected_candidate":Row{id:i,type_id:i,field_id:0,path_id:format!("{i:064x}"),
        type_name:format!("Observed{i}"),field_name:format!("Field{i}"),value:json!(2),object_text:vec![]},
        "value":"0","status":if i==2 {"suppression_not_observed"}else{"untried"},"untouched":{"data":[1,2,3]}})).collect();
    (
        Input::from_choices(&"a".repeat(64), &context, &actions, &choices).unwrap(),
        choices,
    )
}
fn proposal(order: Vec<usize>) -> Proposal {
    Proposal {
        order,
        reason: "A provisional relevance preference.".into(),
        uncertainty: "The effect remains unknown.".into(),
    }
}
#[test]
fn ordering_retains_every_candidate_and_history_without_promoting_a_result() {
    let (input, mut choices) = fixture();
    let before = choices.clone();
    let preference = accept(&input, proposal(vec![2, 1])).unwrap();
    apply(&input, &preference, &mut choices).unwrap();
    assert_eq!(
        choices,
        vec![
            before[2].clone(),
            before[1].clone(),
            before[0].clone(),
            before[3].clone()
        ]
    );
    assert_eq!(
        choices.iter().find(|c| c["status"] == "untried").unwrap(),
        &before[1]
    );
    let prompt = input.prompt_data().unwrap();
    for (i, symbol) in [0, 1, 0, 1].iter().enumerate() {
        assert_eq!(prompt["repeat_sequence"][i]["symbol"], *symbol);
        assert_eq!(
            prompt["repeat_sequence"][i]["hold_ms"],
            input.observed_actions.actions[i].hold_ms
        );
    }
}
#[test]
fn changed_evidence_values_order_and_actions_invalidate_priorities() {
    let (input, choices) = fixture();
    let pref = accept(&input, proposal(vec![1])).unwrap();
    let mut variants = vec![input.clone(); 5];
    variants[0].activity.observation.description = "A different visible response.".into();
    variants[1].report_sha256 = "b".repeat(64);
    variants[2].candidates[0].row.value = json!(3);
    variants[3].observed_actions.actions[0].hold_ms += 1;
    variants[4].candidates.swap(0, 1);
    for changed in variants {
        assert!(apply(&changed, &pref, &mut choices.clone()).is_err());
    }
    let mut changed = choices.clone();
    changed[0]["selected_candidate"]["field_name"] = json!("Other");
    assert!(apply(&input, &pref, &mut changed).is_err());
    let mut altered = pref;
    altered.policy = "future".into();
    assert!(apply(&input, &altered, &mut choices.clone()).is_err());
}
#[test]
fn abstention_preserves_order_and_invalid_nominations_are_rejected() {
    let (input, mut choices) = fixture();
    let before = choices.clone();
    apply(
        &input,
        &accept(&input, proposal(vec![])).unwrap(),
        &mut choices,
    )
    .unwrap();
    assert_eq!(choices, before);
    for order in [vec![4], vec![0, 0], vec![0; 9]] {
        assert!(accept(&input, proposal(order)).is_err());
    }
    let mut bad = proposal(vec![0]);
    bad.uncertainty.clear();
    assert!(accept(&input, bad).is_err());
    assert!(
        serde_json::from_value::<Proposal>(
            json!({"order":[0],"reason":"x","uncertainty":"y","value":"99"})
        )
        .is_err()
    );
}
