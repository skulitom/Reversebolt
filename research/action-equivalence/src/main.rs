use reversebolt_causal_validation::discovery;
mod evaluator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let count: usize = args.get(1).map(|s| s.parse()).transpose()?.unwrap_or(1);
    let start: u64 = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(41000);
    let output = args
        .get(3)
        .map(String::as_str)
        .unwrap_or("../../test-results/action-equivalence/development.json");
    let mut cases = evaluator::inherited_suite(count, start);
    cases.extend(evaluator::withheld::action_suite(count, start));
    let mut summary = std::collections::BTreeMap::<String, serde_json::Value>::new();
    for suite in ["inherited", "previously_seen_withheld"] {
        let runs: Vec<_> = cases.iter().filter(|r| r.suite == suite).collect();
        let actions: Vec<_> = runs.iter().flat_map(|r| &r.actions).collect();
        use reversebolt_action_equivalence::action::Outcome;
        let counts = serde_json::json!({
            "searches": runs.len(), "requested_actions": actions.len(),
            "canonical_identified": runs.iter().filter(|r| r.prior.decision == reversebolt_causal_validation::validation::Decision::IdentifiedWithinScope).count(),
            "observed_within_scope": actions.iter().filter(|a| a.outcome == Outcome::ObservedWithinScope).count(),
            "observed_unknown_effects": actions.iter().filter(|a| a.outcome == Outcome::ObservedWithUnknownEffects).count(),
            "failed": actions.iter().filter(|a| a.outcome == Outcome::Failed).count(),
            "not_tested": actions.iter().filter(|a| a.outcome == Outcome::NotTested).count(),
            "observed_actions_after_canonical_abstention": runs.iter().filter(|r| r.prior.decision == reversebolt_causal_validation::validation::Decision::Abstain)
                .flat_map(|r| &r.actions).filter(|a| a.outcome == Outcome::ObservedWithinScope).count(),
            "action_attempts": actions.iter().map(|a| a.attempts.len()).sum::<usize>(),
            "prior_rollback_failures": runs.iter().map(|r| r.prior.rollback_failures).sum::<usize>(),
            "action_rollback_failures": actions.iter().flat_map(|a| &a.attempts).filter(|a| a.recovery.failed()).count(),
            "prior_private_effects_remaining": runs.iter().map(|r| r.hidden_prior).sum::<usize>(),
            "action_private_effects_remaining": runs.iter().map(|r| r.hidden_actions).sum::<usize>(),
            "audited_actions": runs.iter().flat_map(|r| &r.audits).flatten().count(),
            "failed_extended_audits": runs.iter().flat_map(|r| &r.audits).flatten().filter(|a| !a.effects_ok || !a.restored || a.hidden_effect).count(),
        });
        println!("{suite}: {counts}");
        summary.insert(suite.into(), counts);
    }
    let path = std::path::Path::new(output);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1, "seeds_per_case": count, "start": start,
            "requested_deltas": [3, -4], "action_replays": 2, "audit_ticks": 32,
            "summary": summary, "cases": cases
        }))?,
    )?;
    Ok(())
}
