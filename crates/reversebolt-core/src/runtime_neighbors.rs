//! Bounded sibling observations selected by rooted object paths, never game labels.
use crate::{
    Result,
    runtime::{self, Candidate, Locator, Member, Scope},
    visual::digest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const POLICY: &str = "rooted_role_sibling_sampling_v3";
pub const LIMITATION: &str = "Neighbouring values share a rooted object path in this discovery, not persistent actor identity. Paths can alias or change occupants. Changing siblings may help investigate activity but do not establish meaning, control or causal relevance. Names and values do not determine selection; omitted fields remain unknown.";
pub const MAX_PATHS: usize = 128;
// The loader has a 32K UTF-16 request limit as well as a 64KiB file limit.
// Reserve space for the output path and surrounding request fields.
const MAX_BATCH_UTF16: usize = 30 * 1024;
fn batch_fits(paths: &[Locator]) -> Result<bool> {
    let json = serde_json::to_string(paths).map_err(|e| e.to_string())?;
    Ok(json.len() <= 60 * 1024 && json.encode_utf16().count() <= MAX_BATCH_UTF16)
}
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
    #[serde(default)]
    pub locations: Vec<crate::runtime_location::Location>,
    pub batches: Vec<Vec<Locator>>,
    pub omitted_fields: usize,
    pub limitation: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedSelection {
    pub requested_paths: Vec<String>,
    pub seeds: Vec<String>,
    pub observed_roles: usize,
    pub omitted_roles: usize,
}
/// Collapse report-bound partial-review paths to one seed per rooted role.
/// This uses no proposed names and grants no semantic or edit authority.
pub fn select_seeds(report: &Value, paths: &[String]) -> Result<SeedSelection> {
    if paths.is_empty()
        || paths.len() > 512
        || paths.iter().collect::<BTreeSet<_>>().len() != paths.len()
    {
        return Err("Select one to 512 distinct report-bound paths.".into());
    }
    let requested: BTreeSet<_> = paths.iter().collect();
    let metadata = crate::runtime_location::Index::new(report)?;
    let mut found = BTreeMap::new();
    runtime::visit_candidates(report, "", 0, 1, &mut |c| {
        if requested.contains(&c.id) && eligible(c) {
            found.insert(c.id.clone(), metadata.locate(&c.locator)?.role_id);
        }
        Ok(())
    })?;
    if found.len() != paths.len() {
        return Err("A selected path is absent or unsupported.".into());
    }
    let mut roles = BTreeSet::new();
    let mut seeds = vec![];
    for path in paths {
        if roles.insert(&found[path]) && seeds.len() < 32 {
            seeds.push(path.clone());
        }
    }
    Ok(SeedSelection {
        requested_paths: paths.to_vec(),
        omitted_roles: roles.len() - seeds.len(),
        observed_roles: roles.len(),
        seeds,
    })
}
pub fn role_id(path: &Locator) -> Result<String> {
    crate::runtime_location::role_id_at(
        path,
        path.steps
            .len()
            .checked_sub(1)
            .ok_or("Empty runtime path")?,
    )
}
pub(crate) fn eligible(candidate: &Candidate) -> bool {
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
    let metadata = crate::runtime_location::Index::new(report)?;
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
        let id = metadata.locate(&found[seed].locator)?.role_id;
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
    let mut queues: BTreeMap<String, [VecDeque<Candidate>; 2]> = roles
        .iter()
        .map(|r| (r.id.clone(), [VecDeque::new(), VecDeque::new()]))
        .collect();
    let mut counts: BTreeMap<String, usize> = roles.iter().map(|r| (r.id.clone(), 0)).collect();
    runtime::visit_candidates(report, "", 0, 1, &mut |c| {
        if !eligible(c) {
            return Ok(());
        }
        let location = metadata.locate(&c.locator)?;
        if let Some(queues) = queues.get_mut(&location.role_id) {
            *counts.get_mut(&location.role_id).unwrap() += 1;
            let queue = &mut queues[usize::from(location.members.len() > 1)];
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
    // Interleave roles and alternate direct/embedded leaves within each role.
    // Preserve traversal order in each storage group; no label/value ranking.
    let mut candidates = vec![];
    while candidates.len() < limit && candidates.len() < available {
        for role in &mut roles {
            if candidates.len() == limit {
                break;
            }
            let groups = queues.get_mut(&role.id).unwrap();
            let preferred = role.selected_fields % 2;
            if let Some(c) = groups[preferred]
                .pop_front()
                .or_else(|| groups[1 - preferred].pop_front())
            {
                candidates.push(c);
                role.selected_fields += 1;
            }
        }
    }
    let mut batches: Vec<Vec<Locator>> = vec![];
    for candidate in &candidates {
        if !batch_fits(std::slice::from_ref(&candidate.locator))? {
            return Err("One sibling path exceeds the reader's request budget.".into());
        }
        if batches
            .last()
            .is_none_or(|b| b.len() == runtime::MAX_REFRESH_PATHS)
        {
            batches.push(vec![]);
        }
        let last = batches.last_mut().unwrap();
        last.push(candidate.locator.clone());
        if !batch_fits(last)? {
            let path = last.pop().unwrap();
            if last.is_empty() {
                return Err("One sibling path exceeds the reader's request budget.".into());
            }
            batches.push(vec![path]);
        }
    }
    let locations = candidates
        .iter()
        .map(|c| metadata.locate(&c.locator))
        .collect::<Result<Vec<_>>>()?;
    Ok(Plan {
        policy: POLICY.into(),
        limit,
        scope,
        report_sha256: digest(&serde_json::to_vec(report).map_err(|e| e.to_string())?),
        seed_ids: seeds.to_vec(),
        omitted_fields: available - candidates.len(),
        roles,
        candidates,
        locations,
        batches,
        limitation: LIMITATION.into(),
    })
}
