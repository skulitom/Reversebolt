//! Reacquire an observed rooted route from a new discovery, never old addresses.
use crate::{
    Result,
    runtime::{self, Candidate, Scope},
    visual::digest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recovery {
    pub policy: String,
    pub source_report_sha256: String,
    pub current_report_sha256: String,
    pub source: Candidate,
    pub current_scope: Scope,
    pub same_process: bool,
    pub status: String,
    pub candidate: Option<Candidate>,
    pub edit_authority: bool,
    pub limitation: String,
}

fn identity(report: &Value) -> Result<(String, String, Scope)> {
    let path = report["runtime"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 32768)
        .ok_or("Runtime module path is missing or oversized.")?;
    let roots = report["root_catalog_sha256"]
        .as_str()
        .filter(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or("Runtime root catalogue fingerprint is unavailable.")?;
    let scope: Scope = serde_json::from_value(serde_json::json!({
        "pid":report["pid"],"created":report["created"],"session":report["session"]
    }))
    .map_err(|e| e.to_string())?;
    if scope.pid == 0
        || scope.created.len() != 16
        || !scope.created.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err("Invalid discovery process identity.".into());
    }
    Ok((
        path.replace('\\', "/").to_ascii_lowercase(),
        roots.to_ascii_lowercase(),
        scope,
    ))
}

pub fn find(source_report: &Value, source_path: &str, current_report: &Value) -> Result<Recovery> {
    // Both paths must arise from their own completely validated observed graph.
    // We never replace a scope in an old locator and publish the manufactured path.
    let source = runtime::candidate(source_report, source_path)?;
    let (old_module, old_roots, old_scope) = identity(source_report)?;
    let (module, roots, current_scope) = identity(current_report)?;
    let mut candidate = None;
    runtime::visit_candidates(current_report, "", 0, 1, &mut |row| {
        if row.locator.root == source.locator.root
            && row.locator.steps == source.locator.steps
            && row.type_code == source.type_code
        {
            if candidate.is_some() {
                return Err("Discovery contains duplicate recovery routes.".into());
            }
            candidate = Some(row.clone());
        }
        Ok(())
    })?;
    let status = if module != old_module {
        "runtime_changed"
    } else if roots != old_roots {
        "catalog_changed"
    } else if candidate.is_some() {
        "matched"
    } else {
        "not_observed"
    };
    if status != "matched" {
        candidate = None;
    }
    Ok(Recovery {
        policy: "observed_rooted_route_recovery_v1".into(),
        source_report_sha256: digest(&serde_json::to_vec(source_report).map_err(|e| e.to_string())?),
        current_report_sha256: digest(&serde_json::to_vec(current_report).map_err(|e| e.to_string())?),
        source, same_process: old_scope == current_scope, current_scope,
        status: status.into(), candidate, edit_authority: false,
        limitation: "A newly observed route with matching runtime location, root catalogue and typed member structure, not persistent object identity, the same binary, verified semantics or edit authority. Values, allocations and occupants can change. Missing routes may reflect scene or discovery coverage. Fresh reads, input-response testing and restoration are required before reusing an earlier effect claim.".into(),
    })
}
