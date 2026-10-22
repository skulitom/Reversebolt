use super::*;
use reversebolt_core::{
    play_context::{Context, Observation},
    runtime_hypotheses::Row,
    runtime_planning::ObservedActions,
    runtime_tests::Action,
};

fn fixture() -> (p::Input, Receipt) {
    let source = crate::runtime_activity::tests::fixture().source;
    let input = p::Input {
        report_sha256: "c".repeat(64),
        observed_actions: ObservedActions {
            frame_metadata_sha256: source.frame_metadata_sha256.clone(),
            episodes: vec![],
            episode_order: None,
            actions: [68, 65, 68, 65]
                .map(|key| Action {
                    keys: vec![key],
                    hold_ms: 200,
                })
                .into(),
        },
        activity: Context::from_observation(
            source,
            Observation {
                consistent_visible_change: true,
                description: "A pattern shifts.".into(),
                uncertainty: "Its mechanism is unknown.".into(),
            },
        )
        .unwrap(),
        candidates: vec![p::Candidate {
            row: Row {
                id: 0,
                type_id: 0,
                field_id: 0,
                path_id: "d".repeat(64),
                type_name: "Observed".into(),
                field_name: "Coefficient".into(),
                object_text: vec![],
                value: json!(1),
            },
            proposed_value: "0".into(),
        }],
    };
    let proposal = p::Proposal {
        order: vec![0],
        reason: "A provisional experiment.".into(),
        uncertainty: "No semantic or causal claim is established.".into(),
    };
    let text = prompt(&input).unwrap();
    let receipt = Receipt {
        input_sha256: input.digest().unwrap(),
        model: "synthetic".into(),
        model_digest: "e".repeat(64),
        server_version: "synthetic".into(),
        stage: Some(Stage {
            prompt_sha256: digest(text.as_bytes()),
            prompt: text,
            schema: schema(),
            options: json!({"temperature":0,"seed":42,"num_ctx":16384,"num_predict":800}),
            response: json!({"done":true,"done_reason":"stop","prompt_eval_count":1000,
                "message":{"role":"assistant","content":serde_json::to_string(&proposal).unwrap()}}),
            elapsed_ms: 0,
        }),
        preference: Some(p::accept(&input, proposal).unwrap()),
        error: None,
        cancelled: false,
        limitation: p::LIMITATION.into(),
    };
    (input, receipt)
}

#[test]
fn replay_rejects_changed_evidence_prompt_options_and_parsed_output() {
    let (input, receipt) = fixture();
    assert!(receipt.checked_preference(&input).is_ok());
    let mut changed = input.clone();
    changed.activity.observation.description.push('!');
    assert!(receipt.checked_preference(&changed).is_err());
    let mut variants = vec![receipt.clone(); 6];
    variants[0].stage.as_mut().unwrap().prompt.push('!');
    variants[1].stage.as_mut().unwrap().prompt_sha256 = "f".repeat(64);
    variants[2].stage.as_mut().unwrap().options["seed"] = json!(43);
    variants[3].stage.as_mut().unwrap().schema["required"] = json!([]);
    variants[4]
        .preference
        .as_mut()
        .unwrap()
        .proposal
        .order
        .clear();
    variants[5].input_sha256 = "f".repeat(64);
    for altered in variants {
        assert!(altered.checked_preference(&input).is_err());
    }
}

#[test]
fn cancellation_incomplete_or_over_budget_responses_cannot_supply_preferences() {
    let (input, receipt) = fixture();
    let mut variants = vec![receipt; 10];
    variants[0].cancelled = true;
    variants[1].error = Some("Model changed".into());
    variants[2].stage = None;
    variants[3].preference = None;
    variants[4].stage.as_mut().unwrap().response["done"] = json!(false);
    variants[5].stage.as_mut().unwrap().response["done_reason"] = json!("length");
    variants[6].stage.as_mut().unwrap().response["prompt_eval_count"] = Value::Null;
    variants[7].stage.as_mut().unwrap().response["prompt_eval_count"] = json!(16384);
    variants[8].stage.as_mut().unwrap().response["message"]["tool_calls"] =
        json!([{"name":"change"}]);
    variants[9].stage.as_mut().unwrap().response["message"]["content"] = json!("{}");
    for altered in variants {
        assert!(altered.checked_preference(&input).is_err());
    }
}

#[test]
fn oversized_evidence_is_rejected_before_inference() {
    let (mut input, _) = fixture();
    input.candidates[0].row.object_text = vec!["x".repeat(12001)];
    assert!(prompt(&input).unwrap_err().contains("12,000-byte"));
}
