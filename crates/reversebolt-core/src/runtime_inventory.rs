//! Address-free, report-bound evidence for naming. A field group is not an actor,
//! a gameplay concept, a numeric range, or permission to edit any of its paths.
use crate::{
    Result,
    runtime::{self, Candidate, FieldRef, Member, Scope, TextHint, TypeKey},
    visual::digest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const MAX_FIELDS: usize = 8192;
pub const MAX_PATH_SAMPLES: usize = 4;
pub const MAX_VALUE_SAMPLES: usize = 3;
pub const MAX_EVIDENCE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    pub schema: u32,
    /// Canonical JSON digest, matching CatalogSource.document_sha256.
    pub report_sha256: String,
    pub scope: Scope,
    pub types: Vec<ObservedType>,
    pub reachable_paths: usize,
    pub omitted_core_paths: usize,
    pub omitted_array_elements: usize,
    pub omitted_non_numeric_paths: usize,
    pub limitation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObservedType {
    /// Index local to this exact inventory/report; never a runtime address.
    pub id: usize,
    pub key: TypeKey,
    pub fields: Vec<ObservedField>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObservedField {
    pub field_id: usize,
    pub name: String,
    pub type_code: i32,
    pub values: Vec<Value>,
    /// Counts distinct reachable paths, not distinct instances.
    pub path_count: usize,
    /// Same-object text belongs to this sample only, never the whole field group.
    pub samples: Vec<PathSample>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathSample {
    pub path_id: String,
    pub value: Value,
    pub context: Vec<TextHint>,
}

struct TypeBuilder {
    key: TypeKey,
    fields: BTreeMap<usize, ObservedField>,
}

/// Uses the same graph, type, byte/value and scope checks as runtime candidates.
/// All reachable paths are visited before limits; exceeding the field budget
/// fails instead of silently presenting the first page as a complete inventory.
pub fn build(report: &Value) -> Result<Inventory> {
    let scope: Scope = serde_json::from_value(serde_json::json!({
        "pid": report["pid"], "created": report["created"], "session": report["session"]
    }))
    .map_err(|e| e.to_string())?;
    if scope.pid == 0
        || scope.created.len() != 16
        || !scope.created.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Invalid runtime inventory scope.".into());
    }
    let mut inventory = Inventory {
        schema: 1,
        report_sha256: digest(&serde_json::to_vec(report).map_err(|e| e.to_string())?),
        scope,
        types: Vec::new(), reachable_paths: 0, omitted_core_paths: 0,
        omitted_array_elements: 0, omitted_non_numeric_paths: 0,
        limitation: "Retained reachable observations only. Paths can alias or change occupants; names and nearby text are untrusted clues. No gameplay meaning, bounds, persistent identity or edit authority is established.".into(),
    };
    let mut groups: BTreeMap<String, TypeBuilder> = BTreeMap::new();
    let mut field_count = 0;
    let mut retained_bytes = 1024;
    runtime::visit_candidates(report, "", 0, 1, &mut |candidate| {
        inventory.reachable_paths += 1;
        let Some(Member::Field { field }) = candidate.locator.steps.last().map(|s| &s.member)
        else {
            inventory.omitted_array_elements += 1;
            return Ok(());
        };
        if report["core_image"].as_str() == Some(field.declaring.image.as_str()) {
            inventory.omitted_core_paths += 1;
            return Ok(());
        }
        if candidate.type_code == 3 || !numeric(candidate) {
            inventory.omitted_non_numeric_paths += 1;
            return Ok(());
        }
        let key = serde_json::to_string(&field.declaring).map_err(|e| e.to_string())?;
        if !groups.contains_key(&key) {
            retained_bytes += key.len() + 128;
        }
        let group = groups.entry(key).or_insert_with(|| TypeBuilder {
            key: field.declaring.clone(),
            fields: BTreeMap::new(),
        });
        if !group.fields.contains_key(&field.field_id) {
            field_count += 1;
            retained_bytes += serde_json::to_vec(&candidate.name)
                .map_err(|e| e.to_string())?
                .len()
                + 128;
            if field_count > MAX_FIELDS {
                return Err("Runtime naming inventory exceeds 8,192 fields.".into());
            }
        }
        let entry = group
            .fields
            .entry(field.field_id)
            .or_insert_with(|| ObservedField {
                field_id: field.field_id,
                name: candidate.name.clone(),
                type_code: candidate.type_code,
                values: Vec::new(),
                path_count: 0,
                samples: Vec::new(),
            });
        if entry.name != candidate.name || entry.type_code != candidate.type_code {
            return Err("Conflicting runtime field descriptors.".into());
        }
        entry.path_count += 1;
        if entry.values.len() < MAX_VALUE_SAMPLES && !entry.values.contains(&candidate.value) {
            retained_bytes += serde_json::to_vec(&candidate.value)
                .map_err(|e| e.to_string())?
                .len()
                + 1;
            entry.values.push(candidate.value.clone());
        }
        if entry.samples.len() < MAX_PATH_SAMPLES {
            let sample = PathSample {
                path_id: candidate.id.clone(),
                value: candidate.value.clone(),
                context: candidate.context.clone(),
            };
            retained_bytes += serde_json::to_vec(&sample)
                .map_err(|e| e.to_string())?
                .len()
                + 1;
            entry.samples.push(sample);
        }
        if retained_bytes > MAX_EVIDENCE_BYTES {
            return Err("Runtime naming evidence exceeds 8 MiB.".into());
        }
        Ok(())
    })?;
    inventory.types = groups
        .into_values()
        .enumerate()
        .map(|(id, g)| ObservedType {
            id,
            key: g.key,
            fields: g.fields.into_values().collect(),
        })
        .collect();
    Ok(inventory)
}

fn numeric(candidate: &Candidate) -> bool {
    match candidate.type_code {
        2 => candidate.bytes.first().is_some_and(|b| *b <= 1),
        4..=11 => true,
        12 | 13 => candidate
            .value
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .is_some_and(f64::is_finite),
        _ => false,
    }
}

#[derive(Debug, Serialize)]
pub struct ResolvedField {
    pub matches: usize,
    pub candidates: Vec<Candidate>,
}

/// Resolve an exact report-local selection back to existing guarded runtime paths.
/// Rebuilds/checks the report binding; never converts names into search expressions.
pub fn resolve(
    report: &Value,
    report_sha256: &str,
    type_id: usize,
    field_id: usize,
    limit: usize,
) -> Result<ResolvedField> {
    if !(1..=32).contains(&limit) {
        return Err("Resolve between 1 and 32 path samples.".into());
    }
    let inventory = build(report)?;
    if inventory.report_sha256 != report_sha256 {
        return Err("Naming evidence belongs to a different runtime report.".into());
    }
    let group = inventory
        .types
        .get(type_id)
        .filter(|g| g.id == type_id)
        .ok_or("Observed type is missing.")?;
    if !group.fields.iter().any(|f| f.field_id == field_id) {
        return Err("Observed field is missing.".into());
    }
    let reference = FieldRef {
        declaring: group.key.clone(),
        field_id,
    };
    let mut resolved = ResolvedField {
        matches: 0,
        candidates: Vec::new(),
    };
    runtime::visit_candidates(report, "", 0, 1, &mut |candidate| {
        if numeric(candidate)
            && candidate.locator.steps.last().is_some_and(
                |s| matches!(&s.member, Member::Field { field } if field == &reference),
            )
        {
            resolved.matches += 1;
            if resolved.candidates.len() < limit {
                resolved.candidates.push(candidate.clone());
            }
        }
        Ok(())
    })?;
    Ok(resolved)
}
