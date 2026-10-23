//! Bounded continuation of exploratory response tests, without semantic promotion.
use crate::{
    Result, changes,
    runtime_hypotheses::Review,
    runtime_planning::ObservedActions,
    runtime_tests::{Phase, Test},
    runtime_trials::Trial,
};
use serde::Serialize;
use serde_json::Value;

pub const POLICY: &str = "bounded_response_exploration_v1";
pub const QUESTION: &str = "Does this edit suppress the rendered response to the recorded input sequence, with response returning after restoration?";
pub const LIMITATION: &str = "Exploratory response search, not an answer to the metadata question. A negative permits another distinct candidate only after a complete restored comparison. Positive, stopped, inconclusive, missing or inconsistent evidence ends this search. Other properties and activities remain untested. This plan grants no input or edit authority.";

#[derive(Clone, Debug, Serialize)]
pub struct Decision {
    pub policy: &'static str,
    pub comparison_limit: usize,
    pub test_ids: Vec<String>,
    pub status: &'static str,
    pub reason: &'static str,
    pub tested_question: &'static str,
    pub limitation: &'static str,
}

pub struct Context<'a> {
    pub review: &'a Review,
    pub actions: &'a ObservedActions,
    pub investigation: &'a str,
    pub catalog: &'a str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Negative,
    Positive,
    Unresolved,
}

fn decide(limit: usize, outcomes: &[Outcome]) -> (&'static str, &'static str) {
    if outcomes.contains(&Outcome::Unresolved) {
        (
            "needs_review",
            "A comparison stopped, was inconclusive, or lacks checked restoration/evidence.",
        )
    } else if outcomes.contains(&Outcome::Positive) {
        (
            "response_found",
            "A restored comparison observed a response change. Its gameplay meaning still needs investigation.",
        )
    } else if outcomes.len() >= limit {
        (
            "budget_exhausted",
            "The comparison limit for this exact evidence context has been reached.",
        )
    } else {
        (
            "continue",
            "No positive or unresolved result remains; another untried candidate may be investigated.",
        )
    }
}

/// Count all matching records, including candidates absent from the latest review.
/// Their absence cannot reset the budget or erase an unresolved result.
pub fn plan(
    context: Context<'_>,
    limit: usize,
    tests: &[Test],
    trials: &[Trial],
    choices: &[Value],
) -> Result<Decision> {
    if !(1..=8).contains(&limit) {
        return Err("comparison_limit must be an integer from 1 through 8.".into());
    }
    let relevant: Vec<_> = tests
        .iter()
        .filter(|t| {
            t.investigation == context.investigation
                && t.catalog_id == context.catalog
                && t.actions == context.actions.actions
                && t.origin.as_ref().is_some_and(|o| {
                    o.report_sha256 == context.review.report_sha256
                        && o.play_context.as_ref().is_some_and(|c| {
                            c.source.frame_metadata_sha256 == context.actions.frame_metadata_sha256
                        })
                })
        })
        .collect();
    let outcomes: Vec<_> = relevant
        .iter()
        .map(|t| {
            if t.phase != Phase::Complete || t.pending_capture {
                return Outcome::Unresolved;
            }
            let Some(trial) = trials.iter().find(|r| r.id == t.change_id) else {
                return Outcome::Unresolved;
            };
            if trial.phase != changes::Phase::Restored {
                return Outcome::Unresolved;
            }
            // Reuse the native trial linkage and completion checks, on a private copy.
            let mut checked = (*t).clone();
            checked.phase = Phase::Restored;
            if checked.complete(trial, checked.revision).is_err()
                || serde_json::to_value(&checked.assessment).ok()
                    != serde_json::to_value(&t.assessment).ok()
            {
                return Outcome::Unresolved;
            }
            let Some(choice) = choices.iter().find(|c| c["previous_test"]["id"] == t.id) else {
                return Outcome::Unresolved;
            };
            if choice["previous_test"] != t.summary()
                || choice["selected_candidate"]["path_id"] != t.candidate.id
                || choice["value"] != t.value
                || choice["actions"] != serde_json::json!(t.actions)
            {
                return Outcome::Unresolved;
            }
            match choice["status"].as_str() {
                Some("suppression_not_observed" | "regional_change_not_observed") => {
                    Outcome::Negative
                }
                Some("suppression_observed" | "regional_change_and_return_observed") => {
                    Outcome::Positive
                }
                _ => Outcome::Unresolved,
            }
        })
        .collect();
    let (status, reason) = decide(limit, &outcomes);
    Ok(Decision {
        policy: POLICY,
        comparison_limit: limit,
        test_ids: relevant.iter().map(|t| t.id.clone()).collect(),
        status,
        reason,
        tested_question: QUESTION,
        limitation: LIMITATION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuation_is_bounded_and_unknown_evidence_is_never_a_negative() {
        use Outcome::*;
        assert_eq!(decide(3, &[]).0, "continue");
        assert_eq!(decide(3, &[Negative, Negative]).0, "continue");
        assert_eq!(decide(3, &[Negative; 3]).0, "budget_exhausted");
        for prior in [
            vec![Positive],
            vec![Negative, Positive],
            vec![Positive, Negative],
        ] {
            assert_eq!(decide(3, &prior).0, "response_found");
        }
        for prior in [
            vec![Unresolved],
            vec![Negative, Unresolved],
            vec![Positive, Unresolved],
            vec![Unresolved, Positive],
        ] {
            assert_eq!(decide(3, &prior).0, "needs_review");
        }
    }
}
