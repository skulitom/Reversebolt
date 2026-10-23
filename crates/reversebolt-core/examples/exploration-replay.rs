//! Offline policy checks against genuine retained native comparisons.
use reversebolt_core::{
    runtime_exploration::{self as e, Context},
    runtime_hypotheses::Review,
    runtime_planning::{self as p, ObservedActions},
    runtime_tests::Test,
    runtime_trials::Trial,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "exploration-replay CASE OUTPUT");
    let root = PathBuf::from(&args[1]);
    let discovery = read(root.join("discovery.json"));
    let runtime = &discovery["state"]["live"]["runtime"];
    let review: Review = serde_json::from_value(runtime["naming"].clone()).unwrap();
    let source = runtime["catalog_source"]["path"].as_str().unwrap();
    let inventory = reversebolt_core::runtime_inventory::build(&read(source)).unwrap();
    let actions: ObservedActions = serde_json::from_value(
        read(root.join("chosen-plan.json"))["plan"]["observed_actions"].clone(),
    )
    .unwrap();
    let test: Test =
        serde_json::from_value(read(root.join("comparison.json"))["test"].clone()).unwrap();
    let frozen = read(root.join("frozen.json"));
    let trial: Trial = serde_json::from_value(read(
        Path::new(frozen["data"].as_str().unwrap())
            .join("runtime-trials")
            .join(format!("{}.json", test.change_id)),
    ))
    .unwrap();
    let choices = p::choices(
        &review,
        &inventory,
        &actions,
        std::slice::from_ref(&test),
        &test.investigation,
        &test.catalog_id,
    )
    .unwrap();
    let context = || Context {
        review: &review,
        actions: &actions,
        investigation: &test.investigation,
        catalog: &test.catalog_id,
    };
    let run = |limit, records: &[Test], trials: &[Trial], choices: &[Value]| {
        e::plan(context(), limit, records, trials, choices).unwrap()
    };
    let one = std::slice::from_ref(&test);
    let restored = std::slice::from_ref(&trial);
    let mut cases = vec![];
    let negative = test.assessment.as_ref().unwrap().status == "suppression_not_observed";
    let actual = run(3, one, restored, &choices);
    assert_eq!(
        actual.status,
        if negative {
            "continue"
        } else {
            "response_found"
        }
    );
    cases.push(json!({"case":"native_history","decision":actual}));
    let capped = run(1, one, restored, &choices);
    assert_eq!(
        capped.status,
        if negative {
            "budget_exhausted"
        } else {
            "response_found"
        }
    );
    cases.push(json!({"case":"one_test_limit","decision":capped}));
    for mutation in 0..6 {
        let mut tests = vec![test.clone()];
        let mut trials = vec![trial.clone()];
        let mut c = choices.clone();
        match mutation {
            0 => trials.clear(),
            1 => trials[0].patch.replacement[0] ^= 1,
            2 => tests[0].phase = reversebolt_core::runtime_tests::Phase::Stopped,
            3 => c.clear(),
            4 => tests[0].assessment.as_mut().unwrap().status = "invented".into(),
            _ => tests[0].pending_capture = true,
        }
        let d = run(3, &tests, &trials, &c);
        assert_eq!(d.status, "needs_review");
        assert_eq!(d.test_ids.len(), 1);
        cases.push(json!({"case":format!("incomplete_or_changed_{mutation}"),"decision":d}));
    }
    for mutation in 0..5 {
        let mut changed = test.clone();
        match mutation {
            0 => changed.investigation.push('!'),
            1 => changed.catalog_id.push('!'),
            2 => changed.actions[0].hold_ms += 1,
            3 => changed.origin.as_mut().unwrap().report_sha256 = "0".repeat(64),
            _ => {
                changed
                    .origin
                    .as_mut()
                    .unwrap()
                    .play_context
                    .as_mut()
                    .unwrap()
                    .source
                    .frame_metadata_sha256 = "0".repeat(64)
            }
        }
        let d = run(1, &[changed], restored, &choices);
        assert_eq!(d.status, "continue");
        assert!(d.test_ids.is_empty());
        cases.push(json!({"case":format!("other_context_{mutation}"),"decision":d}));
    }
    assert!(e::plan(context(), 0, one, restored, &choices).is_err());
    assert!(e::plan(context(), 9, one, restored, &choices).is_err());
    fs::write(
        &args[2],
        serde_json::to_vec_pretty(&json!({"case":root,"checks":cases.len()+2,"results":cases}))
            .unwrap(),
    )
    .unwrap();
    println!("15 native-history and mutation checks passed");
}
