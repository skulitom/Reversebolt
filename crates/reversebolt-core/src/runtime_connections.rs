//! Bounded immediate relationships on checked discovery routes. No alias joins.
use crate::{Result, runtime, runtime_location::Index, runtime_neighbors, visual::digest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const POLICY: &str = "immediate_rooted_connections_v1";
const MAX_ROLES: usize = 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Parent,
    Child,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub responding_path: String,
    pub responding_role: String,
    pub snapshot_id: usize,
    pub direction: Direction,
    /// Always oriented from the parent to its child, even for a Parent link.
    pub members_from_parent: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub seed_path: String,
    pub available_fields: usize,
    pub selected: bool,
    pub links: Vec<Link>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Connections {
    pub policy: String,
    pub report_sha256: String,
    pub responding_paths: Vec<String>,
    pub roles: Vec<Role>,
    pub omitted_roles: usize,
    pub available_fields: usize,
    pub selected_fields: usize,
    pub omitted_fields: usize,
    pub limitation: String,
}

pub fn plan(
    report: &Value,
    paths: &[String],
) -> Result<(Connections, Option<runtime_neighbors::Plan>)> {
    if paths.len() > runtime_neighbors::MAX_PATHS
        || paths.iter().collect::<BTreeSet<_>>().len() != paths.len()
    {
        return Err("Use up to 128 distinct responding paths.".into());
    }
    // Snapshot-local relationships cannot be joined through reused snapshot IDs.
    let snapshots = report["snapshots"]
        .as_array()
        .ok_or("Missing discovery snapshots")?;
    let mut snapshot_ids = BTreeSet::new();
    for snapshot in snapshots {
        if !snapshot_ids.insert(
            snapshot["snapshot_id"]
                .as_u64()
                .ok_or("Missing snapshot ID")?,
        ) {
            return Err("Duplicate discovery snapshot ID.".into());
        }
    }
    let metadata = Index::new(report)?;
    let mut seeds = BTreeMap::new();
    runtime::visit_candidates(report, "", 0, 1, &mut |c| {
        if paths.contains(&c.id) && runtime_neighbors::eligible(c) {
            seeds.insert(c.id.clone(), (c.clone(), metadata.role_chain(&c.locator)?));
        }
        Ok(())
    })?;
    if seeds.len() != paths.len() {
        return Err("A responding path is absent or unsupported.".into());
    }
    let responding_roles: BTreeSet<_> = seeds
        .values()
        .map(|(_, chain)| &chain.last().unwrap().role_id)
        .collect();
    let mut roles: Vec<Role> = vec![];
    let mut indices = BTreeMap::new();
    runtime::visit_candidates(report, "", 0, 1, &mut |c| {
        if !runtime_neighbors::eligible(c) {
            return Ok(());
        }
        let chain = metadata.role_chain(&c.locator)?;
        let target = chain.last().unwrap();
        if responding_roles.contains(&target.role_id) {
            return Ok(());
        }
        let mut links = vec![];
        for id in paths {
            let (seed, source_chain) = &seeds[id];
            if seed.snapshot_id != c.snapshot_id {
                continue;
            }
            let source = source_chain.last().unwrap();
            let relation = if source_chain.len() > 1
                && source_chain[source_chain.len() - 2].role_id == target.role_id
            {
                Some((Direction::Parent, &source.members_from_parent))
            } else if chain.len() > 1 && chain[chain.len() - 2].role_id == source.role_id {
                Some((Direction::Child, &target.members_from_parent))
            } else {
                None
            };
            if let Some((direction, members)) = relation {
                links.push(Link {
                    responding_path: id.clone(),
                    responding_role: source.role_id.clone(),
                    snapshot_id: seed.snapshot_id,
                    direction,
                    members_from_parent: members.clone(),
                });
            }
        }
        if links.is_empty() {
            return Ok(());
        }
        let index = if let Some(index) = indices.get(&target.role_id) {
            *index
        } else {
            if roles.len() == MAX_ROLES {
                return Err("Immediate connected roles exceed the 1,024-role budget.".into());
            }
            let index = roles.len();
            indices.insert(target.role_id.clone(), index);
            roles.push(Role {
                id: target.role_id.clone(),
                seed_path: c.id.clone(),
                available_fields: 0,
                selected: index < 32,
                links: vec![],
            });
            index
        };
        let role = &mut roles[index];
        role.available_fields += 1;
        for link in links {
            if !role.links.contains(&link) {
                role.links.push(link);
            }
        }
        Ok(())
    })?;
    let selected: Vec<_> = roles
        .iter()
        .filter(|r| r.selected)
        .map(|r| r.seed_path.clone())
        .collect();
    let neighbors = if selected.is_empty() {
        None
    } else {
        Some(runtime_neighbors::plan(
            report,
            &selected,
            runtime_neighbors::MAX_PATHS,
        )?)
    };
    // The sibling planner groups rooted routes across the report. Keep this
    // operation strict: every chosen path must be observed in its link snapshot.
    if let Some(neighbors) = &neighbors {
        for (c, loc) in neighbors.candidates.iter().zip(&neighbors.locations) {
            let role = &roles[indices[&loc.role_id]];
            if !role.links.iter().any(|l| l.snapshot_id == c.snapshot_id) {
                return Err("A connected field is outside the supporting snapshot.".into());
            }
        }
    }
    let available_fields: usize = roles.iter().map(|r| r.available_fields).sum();
    let selected_fields = neighbors.as_ref().map_or(0, |p| p.candidates.len());
    Ok((Connections {
        policy: POLICY.into(), report_sha256: digest(&serde_json::to_vec(report).map_err(|e|e.to_string())?),
        responding_paths: paths.to_vec(), omitted_roles: roles.len().saturating_sub(32), roles,
        available_fields, selected_fields, omitted_fields: available_fields.saturating_sub(selected_fields),
        limitation: "Immediate parent/child contexts on observed canonical rooted routes in the same discovery snapshot. This is structural proximity, not ownership, causality, activity or gameplay meaning. Embedded field hops stay within an object; array elements remain separate contexts. No type/address-based joins, cross-snapshot alias resolution, noncanonical references or unobserved objects are inferred. Responding contexts are excluded from this next review, not ruled out as useful controls. Missing and omitted paths remain unknown.".into(),
    }, neighbors))
}
