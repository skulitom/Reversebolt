//! Bounded sibling observations selected by rooted object paths, never game labels.
use crate::{
    Result,
    runtime::{self, Candidate, Locator, Member, Scope},
    visual::digest,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const POLICY: &str = "rooted_role_sibling_sampling_v1";
pub const LIMITATION: &str = "Neighbouring values share a rooted object path in this discovery, not persistent actor identity. Paths can alias or change occupants. Changing siblings may help investigate activity but do not establish meaning, control or causal relevance. Names and values do not determine selection; omitted fields remain unknown.";
pub const MAX_PATHS: usize = 128;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub seed_ids: Vec<String>,
    pub available_fields: usize,
    pub selected_fields: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub policy: String,
    pub limit: usize,
    pub scope: Scope,
    pub report_sha256: String,
    pub seed_ids: Vec<String>,
    pub roles: Vec<Role>,
    pub candidates: Vec<Candidate>,
    pub batches: Vec<Vec<Locator>>,
    pub omitted_fields: usize,
    pub limitation: String,
}
pub fn role_id(path: &Locator) -> Result<String> {
    path.validate(&path.scope)?;
    let terminal = path.steps.last().ok_or("Empty runtime path")?;
    Ok(digest(
        &serde_json::to_vec(&json!({"scope":path.scope,"root":path.root,
        "references":&path.steps[..path.steps.len()-1],"owner":terminal.owner}))
        .map_err(|e| e.to_string())?,
    ))
}
fn eligible(candidate: &Candidate) -> bool {
    matches!(candidate.type_code, 2 | 4..=13)
        && matches!(
            candidate.locator.steps.last().map(|s| &s.member),
            Some(Member::Field { .. })
        )
        && match candidate.type_code {
            2 => candidate.bytes.first().is_some_and(|b| *b <= 1),
            12 | 13 => candidate
                .value
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .is_some_and(f64::is_finite),
            _ => true,
        }
}
pub fn plan(report: &Value, seeds: &[String], limit: usize) -> Result<Plan> {
    if seeds.is_empty()
        || seeds.len() > 32
        || seeds.iter().collect::<BTreeSet<_>>().len() != seeds.len()
        || !(1..=MAX_PATHS).contains(&limit)
    {
        return Err("Select one to 32 distinct seed paths and a one to 128 field budget.".into());
    }
    let mut found = BTreeMap::new();
    runtime::visit_candidates(report, "", 0, 1, &mut |c| {
        if seeds.contains(&c.id) && eligible(c) {
            found.insert(c.id.clone(), c.clone());
        }
        Ok(())
    })?;
    if found.len() != seeds.len() {
        return Err("A seed is absent or is not a supported scalar field.".into());
    }
    let scope = found.values().next().unwrap().locator.scope.clone();
    let mut roles: Vec<Role> = vec![];
    for seed in seeds {
        let id = role_id(&found[seed].locator)?;
        if let Some(role) = roles.iter_mut().find(|r| r.id == id) {
            role.seed_ids.push(seed.clone());
        } else {
            roles.push(Role {
                id,
                seed_ids: vec![seed.clone()],
                available_fields: 0,
                selected_fields: 0,
            });
        }
    }
    let mut queues: BTreeMap<String, VecDeque<Candidate>> = roles
        .iter()
        .map(|r| (r.id.clone(), VecDeque::new()))
        .collect();
    let mut counts: BTreeMap<String, usize> = roles.iter().map(|r| (r.id.clone(), 0)).collect();
    runtime::visit_candidates(report, "", 0, 1, &mut |c| {
        if eligible(c)
            && let Some(queue) = queues.get_mut(&role_id(&c.locator)?)
        {
            *counts.get_mut(&role_id(&c.locator)?).unwrap() += 1;
            if queue.len() < limit {
                queue.push_back(c.clone());
            }
        }
        Ok(())
    })?;
    for role in &mut roles {
        role.available_fields = counts[&role.id];
    }
    let available: usize = roles.iter().map(|r| r.available_fields).sum();
    // Keep report traversal order within each role, interleaving roles so one
    // large object cannot consume the entire observation budget.
    let mut candidates = vec![];
    while candidates.len() < limit && candidates.len() < available {
        for role in &mut roles {
            if candidates.len() == limit {
                break;
            }
            if let Some(c) = queues.get_mut(&role.id).unwrap().pop_front() {
                candidates.push(c);
                role.selected_fields += 1;
            }
        }
    }
    let mut batches: Vec<Vec<Locator>> = vec![];
    for candidate in &candidates {
        if serde_json::to_vec(&vec![&candidate.locator])
            .map_err(|e| e.to_string())?
            .len()
            > 60 * 1024
        {
            return Err("One sibling path exceeds the reader's byte budget.".into());
        }
        if batches
            .last()
            .is_none_or(|b| b.len() == runtime::MAX_REFRESH_PATHS)
        {
            batches.push(vec![]);
        }
        let last = batches.last_mut().unwrap();
        last.push(candidate.locator.clone());
        if serde_json::to_vec(last).map_err(|e| e.to_string())?.len() > 60 * 1024 {
            let path = last.pop().unwrap();
            if last.is_empty() {
                return Err("One sibling path exceeds the reader's byte budget.".into());
            }
            batches.push(vec![path]);
        }
    }
    Ok(Plan {
        policy: POLICY.into(),
        limit,
        scope,
        report_sha256: digest(&serde_json::to_vec(report).map_err(|e| e.to_string())?),
        seed_ids: seeds.to_vec(),
        omitted_fields: available - candidates.len(),
        roles,
        candidates,
        batches,
        limitation: LIMITATION.into(),
    })
}
