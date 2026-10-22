//! Historical display snapshots of engine-produced comparison results.
//! Publishing status never reruns matching or treats a review as edit authority.
use reversebolt_core::{
    engine::Engine,
    runtime_tests::{Phase, Test},
    visual,
};
use serde_json::{Value, json};

fn candidate_results<'a>(action: &str, result: &'a Value) -> Vec<(&'a str, &'a Value)> {
    match action {
        "observe_runtime_test" => result["id"]
            .as_str()
            .map(|id| vec![(id, result)])
            .unwrap_or_default(),
        "read_runtime_test" => result["test"]["id"]
            .as_str()
            .map(|id| vec![(id, result)])
            .unwrap_or_default(),
        "plan_runtime_comparisons" => result["choices"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|choice| {
                choice["previous_test"]["id"]
                    .as_str()
                    .map(|id| (id, choice))
            })
            .collect(),
        _ => vec![],
    }
}

fn bound_assessment(test: &Test, sources: &Value, assessment: &Value) -> bool {
    assessment["policy"] == reversebolt_vision::trial_intervention::POLICY
        && assessment["test_id"] == test.id
        && assessment["change_id"] == test.change_id
        && assessment["test_sha256"] == sources["test_sha256"]
        && assessment["trial_sha256"] == sources["trial_sha256"]
        && matches!(
            assessment["status"].as_str(),
            Some(
                "regional_change_and_return_observed"
                    | "regional_change_not_observed"
                    | "inconclusive"
            )
        )
}

/// `result` is a successful result from the sole engine worker, never caller data.
/// Existing snapshots are retained only for unchanged test and trial records.
/// They describe the last review, not a fresh check of files or the live game.
pub(crate) fn publish(engine: &mut Engine, result: Option<(&str, &Value)>) {
    let incoming = result
        .map(|(action, value)| candidate_results(action, value))
        .unwrap_or_default();
    engine.view.runtime_tests = engine.runtime_tests.records.iter().map(|test| {
        let mut summary = test.summary();
        if test.phase != Phase::Complete || test.appearance.is_none() { return summary; }
        let prior = engine.view.runtime_tests.iter().find(|v| v["id"] == test.id)
            .and_then(|v| v.get("last_regional_review"));
        let fresh = incoming.iter().find(|(id, _)| *id == test.id).map(|(_, data)| *data)
            .filter(|data| data.get("regional_intervention_assessment").is_some()
                || data.get("regional_intervention_error").is_some()
                || data.get("trial_appearance_error").is_some());
        if prior.is_none() && fresh.is_none() { return summary; }
        let Ok(bytes) = serde_json::to_vec(test) else { return summary; };
        let trial_sha256 = engine.runtime_trials.records.iter().find(|t| t.id == test.change_id)
            .map(|t| serde_json::to_vec(t).map(|b| visual::digest(&b))).transpose();
        let Ok(trial_sha256) = trial_sha256 else { return summary; };
        let sources = json!({"test_sha256":visual::digest(&bytes), "trial_sha256":trial_sha256});
        if let Some(data) = fresh {
            let assessment = &data["regional_intervention_assessment"];
            let error = data.get("regional_intervention_error").or_else(|| data.get("trial_appearance_error"));
            // A failed or mismatched re-review replaces any older positive result.
            let (assessment, error) = if error.is_none() && bound_assessment(test, &sources, assessment) {
                (assessment.clone(), Value::Null)
            } else {
                (Value::Null, json!(error.and_then(Value::as_str).unwrap_or("The comparison result does not match the saved evidence.").chars().take(512).collect::<String>()))
            };
            summary["last_regional_review"] = json!({"sources":sources,
                "reviewed_unix_ms":reversebolt_core::changes::now_ms(), "assessment":assessment,"error":error});
        } else if let Some(prior) = prior
            && prior["sources"] == sources
            && prior["reviewed_unix_ms"].as_u64().is_some()
            && ((prior["error"].is_null() && bound_assessment(test, &sources, &prior["assessment"]))
                || (prior["assessment"].is_null() && prior["error"].is_string())) {
            summary["last_regional_review"] = prior.clone();
        }
        summary
    }).collect();
}

pub(crate) fn note(review: &Value) -> (&'static str, &'static str) {
    match review["assessment"]["status"].as_str() {
        Some("regional_change_and_return_observed") => (
            "Last comparison: response changed",
            "A watched area responded differently after the edit and returned after restoration. This earlier result does not confirm the proposed name or that the game is still in the same state.",
        ),
        Some("regional_change_not_observed") => (
            "Last comparison: no change found",
            "The edit and restoration were recorded, but the watched area's response stayed similar. Other effects may be outside this comparison.",
        ),
        Some("inconclusive") => (
            "Last comparison: uncertain",
            "The recorded comparison did not establish a clear response. The proposed name remains unverified.",
        ),
        _ => (
            "Last comparison: unavailable",
            "The last review could not check the recorded comparison. No earlier positive result is being reused.",
        ),
    }
}
