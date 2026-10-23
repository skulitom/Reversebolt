//! Use replayed state associations to scope a new parameter investigation.
//! Shared rooted context does not transfer activity, identity or edit authority.
use crate::{Result, runtime_inventory, runtime_neighbors, runtime_response, visual::digest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const POLICY: &str = "response_rooted_parameter_focus_v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchObservations {
    pub batch: usize,
    pub observations: Vec<runtime_response::Observation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Focus {
    pub policy: String,
    pub source_plan_sha256: String,
    pub observed_batches: Vec<usize>,
    pub unobserved_batches: Vec<usize>,
    /// Recomputed analyses, including nonresponding fields and original hashes.
    pub responses: Vec<Value>,
    pub responding_paths: Vec<String>,
    pub neighborhood: Option<runtime_neighbors::Plan>,
    /// Original report-local type IDs and only selected path samples.
    pub inventory: runtime_inventory::Inventory,
    pub review_sampled_paths: usize,
    pub edit_authority: bool,
    pub limitation: String,
}

pub fn build(
    report: &Value,
    plan: &runtime_neighbors::Plan,
    batches: &[BatchObservations],
) -> Result<Focus> {
    // Rebuild from the discovery, not the supplied names, roles or success flags.
    let canonical = runtime_neighbors::plan(report, &plan.seed_ids, plan.limit)?;
    let bytes = serde_json::to_vec(plan).map_err(|e| e.to_string())?;
    if bytes != serde_json::to_vec(&canonical).map_err(|e| e.to_string())? {
        return Err("Activity plan differs from its discovery and selection.".into());
    }
    if batches.is_empty()
        || batches.len() > plan.batches.len()
        || batches.windows(2).any(|b| b[0].batch >= b[1].batch)
    {
        return Err("Supply distinct observed batches in plan order.".into());
    }
    let mut responses = vec![];
    let mut responding_paths = vec![];
    for batch in batches {
        let response = runtime_response::analyze(plan, batch.batch, &batch.observations)?;
        for field in response["fields"]
            .as_array()
            .ok_or("Missing response fields")?
        {
            if field["repeated_association"] == true {
                responding_paths.push(
                    field["path_id"]
                        .as_str()
                        .ok_or("Missing response path")?
                        .to_owned(),
                );
            }
        }
        responses.push(response);
    }
    let neighborhood = if responding_paths.is_empty() {
        None
    } else {
        let seeds = runtime_neighbors::select_seeds(report, &responding_paths)?;
        if seeds.omitted_roles != 0 {
            return Err("Responding roles exceed the investigation budget.".into());
        }
        Some(runtime_neighbors::plan(
            report,
            &seeds.seeds,
            runtime_neighbors::MAX_PATHS,
        )?)
    };
    let paths: Vec<_> = neighborhood.as_ref().map_or_else(Vec::new, |p| {
        p.candidates.iter().map(|c| c.id.clone()).collect()
    });
    let inventory = runtime_inventory::for_paths(report, &paths)?;
    let review_sampled_paths = inventory
        .types
        .iter()
        .flat_map(|t| &t.fields)
        .map(|f| f.samples.len())
        .sum();
    let observed: BTreeSet<_> = batches.iter().map(|b| b.batch).collect();
    Ok(Focus {
        policy: POLICY.into(), source_plan_sha256: digest(&bytes),
        observed_batches: observed.iter().copied().collect(),
        unobserved_batches: (0..plan.batches.len()).filter(|i| !observed.contains(i)).collect(),
        responses, responding_paths, neighborhood, inventory, review_sampled_paths,
        edit_authority: false,
        limitation: "Repeated state associations nominate rooted contexts for further investigation, not actors or useful controls. Nearby constant parameters remain eligible, but do not inherit the observed response. Paths can alias or change occupants; current activity is not established at the earlier discovery time. Missing batches, unselected fields and unsampled paths remain unknown. No causal effect, semantic meaning, live binding, bounds or write authority is established.".into(),
    })
}
