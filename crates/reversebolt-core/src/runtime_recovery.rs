//! Reacquire an observed rooted route from a new discovery, never old addresses.
use crate::{
    Result,
    runtime::{self, Candidate, Scope},
    visual::digest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const MAX_RECOVERY_RECIPES: usize = 8;

/// Compact metadata saved with a test, not an address or permission to edit.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub version: u32,
    pub runtime: String,
    pub root_catalog_sha256: String,
    pub report_sha256: String,
    pub source: runtime::Locator,
    pub route: Value,
}

impl Recipe {
    pub fn capture(report: &Value, path_id: &str) -> Result<Self> {
        let candidate = runtime::candidate(report, path_id)?;
        let (runtime, root_catalog_sha256, _) = identity(report)?;
        let recipe = Self {
            version: 1,
            runtime,
            root_catalog_sha256,
            report_sha256: digest(&serde_json::to_vec(report).map_err(|e| e.to_string())?),
            route: Metadata::new(report)?.route(&candidate)?,
            source: candidate.locator,
        };
        recipe.validate()?;
        Ok(recipe)
    }

    pub fn validate(&self) -> Result<()> {
        self.source.validate(&self.source.scope)?;
        if self.version != 1
            || self.runtime.is_empty()
            || self.runtime.len() > 32768
            || [&self.root_catalog_sha256, &self.report_sha256]
                .iter()
                .any(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
            || serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 32768
            || self.route.as_object().is_none_or(|o| o.len() != 4)
            || self.route["steps"]
                .as_array()
                .is_none_or(|s| s.len() != self.source.steps.len())
            || self.route["scalar_type"].as_i64().is_none()
            || self.route["scalar_width"]
                .as_u64()
                .is_none_or(|n| n == 0 || n > 8)
        {
            return Err("Invalid remembered runtime route.".into());
        }
        let matches_type = |v: &Value, k: &runtime::TypeKey| {
            v["image"] == k.image && v["type_name"] == k.type_name
        };
        if !matches_type(
            &self.route["root"]["declaring"],
            &self.source.root.declaring,
        ) {
            return Err("Remembered root descriptor changed.".into());
        }
        for (descriptor, step) in self.route["steps"]
            .as_array()
            .unwrap()
            .iter()
            .zip(&self.source.steps)
        {
            let member_matches = match &step.member {
                runtime::Member::Field { field } => matches_type(
                    &descriptor["member"]["field"]["declaring"],
                    &field.declaring,
                ),
                runtime::Member::Element { index } => descriptor["member"]["element"] == *index,
            };
            if !matches_type(&descriptor["owner"], &step.owner) || !member_matches {
                return Err("Remembered member descriptor changed.".into());
            }
        }
        Ok(())
    }

    pub fn id(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(
            &serde_json::to_vec(self).map_err(|e| e.to_string())?,
        ))
    }

    pub fn check_candidate(&self, candidate: &Candidate) -> Result<()> {
        self.validate()?;
        if self.source != candidate.locator
            || self.source.id() != candidate.id
            || self.route["scalar_type"] != candidate.type_code
            || self.route["scalar_width"] != candidate.bytes.len()
        {
            return Err("Remembered route belongs to another test candidate.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reacquisition {
    pub recipe_sha256: String,
    pub current_report_sha256: String,
    pub current_scope: Scope,
    pub status: String,
    pub strict_status: String,
    pub matching_routes: usize,
    pub candidate: Option<Candidate>,
    pub edit_authority: bool,
}

/// Presentation for an exact candidate in one current catalogue. Historical
/// evidence stays separate from current value, meaning and editing authority.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveredControl {
    pub source_test_id: String,
    pub source_test_sha256: String,
    pub recipe_sha256: String,
    pub catalog_id: String,
    pub report_sha256: String,
    pub candidate_id: String,
    pub name: String,
    pub description: String,
    pub needs_revalidation: bool,
}

/// One complete graph traversal for a bounded batch of previously saved routes.
pub fn reacquire(recipes: &[Recipe], report: &Value) -> Result<Vec<Reacquisition>> {
    if recipes.len() > MAX_RECOVERY_RECIPES {
        return Err("Too many remembered routes in one recovery.".into());
    }
    let (module, roots, scope) = identity(report)?;
    let hash = digest(&serde_json::to_vec(report).map_err(|e| e.to_string())?);
    let metadata = Metadata::new(report)?;
    let mut results = recipes
        .iter()
        .map(|r| {
            Ok(Reacquisition {
                recipe_sha256: r.id()?,
                current_report_sha256: hash.clone(),
                current_scope: scope.clone(),
                status: "not_observed".into(),
                strict_status: "not_observed".into(),
                matching_routes: 0,
                candidate: None,
                edit_authority: false,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut strict = vec![false; recipes.len()];
    runtime::visit_candidates(report, "", 0, 1, &mut |candidate| {
        let mut descriptor = None;
        for (i, recipe) in recipes.iter().enumerate() {
            if candidate.locator.root == recipe.source.root
                && candidate.locator.steps == recipe.source.steps
                && recipe.route["scalar_type"] == candidate.type_code
            {
                strict[i] = true;
            }
            if recipe.runtime != module
                || !similar_route(&candidate.locator, &recipe.source)
                || recipe.route["scalar_type"] != candidate.type_code
            {
                continue;
            }
            let route = match &descriptor {
                Some(v) => v,
                None => descriptor.insert(metadata.route(candidate)?),
            };
            if *route == recipe.route {
                results[i].matching_routes += 1;
                results[i].candidate = Some(candidate.clone());
            }
        }
        Ok(())
    })?;
    for (i, (recipe, result)) in recipes.iter().zip(&mut results).enumerate() {
        result.strict_status = if recipe.runtime != module {
            "runtime_changed"
        } else if recipe.root_catalog_sha256 != roots {
            "catalog_changed"
        } else if strict[i] {
            "matched"
        } else {
            "not_observed"
        }
        .into();
        result.status = if recipe.runtime != module {
            "runtime_changed"
        } else {
            match result.matching_routes {
                0 => "not_observed",
                1 => "candidate_for_revalidation",
                _ => "ambiguous",
            }
        }
        .into();
        if result.status != "candidate_for_revalidation" {
            result.candidate = None;
        }
    }
    Ok(results)
}

fn similar_route(a: &runtime::Locator, b: &runtime::Locator) -> bool {
    let same_type = |a: &runtime::TypeKey, b: &runtime::TypeKey| {
        a.image == b.image && a.type_name == b.type_name
    };
    same_type(&a.root.declaring, &b.root.declaring)
        && a.steps.len() == b.steps.len()
        && a.steps.iter().zip(&b.steps).all(|(a, b)| {
            same_type(&a.owner, &b.owner)
                && match (&a.member, &b.member) {
                    (runtime::Member::Field { field: a }, runtime::Member::Field { field: b }) => {
                        same_type(&a.declaring, &b.declaring)
                    }
                    (
                        runtime::Member::Element { index: a },
                        runtime::Member::Element { index: b },
                    ) => a == b,
                    _ => false,
                }
        })
}

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

/// An explicitly weaker search policy for proposing a new effect investigation.
/// The strict result is retained; this does not migrate a locator or its authority.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proposal {
    pub policy: String,
    pub strict: Recovery,
    pub source_route: Value,
    pub status: String,
    pub matching_routes: usize,
    pub candidate: Option<Candidate>,
    pub edit_authority: bool,
    pub limitation: String,
}

struct Metadata<'a> {
    classes: BTreeMap<String, &'a Value>,
}

impl<'a> Metadata<'a> {
    fn new(report: &'a Value) -> Result<Self> {
        let mut classes = BTreeMap::new();
        for class in report["classes"]
            .as_array()
            .ok_or("Missing runtime classes.")?
        {
            let key = serde_json::to_string(&class["key"]).map_err(|e| e.to_string())?;
            if let Some(old) = classes.insert(key, class)
                && (old["fields"] != class["fields"]
                    || old["shape"] != class["shape"]
                    || old["element_inline_type"] != class["element_inline_type"])
            {
                return Err("Conflicting metadata for the same runtime type key.".into());
            }
        }
        Ok(Self { classes })
    }

    fn class(&self, key: &runtime::TypeKey) -> Result<&Value> {
        self.classes
            .get(
                &serde_json::to_value(key)
                    .map_err(|e| e.to_string())?
                    .to_string(),
            )
            .copied()
            .ok_or_else(|| "Route type is missing from its own report.".into())
    }

    fn named_type(value: &Value) -> Result<Value> {
        if value.is_null() {
            return Ok(Value::Null);
        }
        let key: runtime::TypeKey =
            serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        key.validate()?;
        Ok(serde_json::json!({"image":key.image,"type_name":key.type_name}))
    }

    fn owner(&self, key: &runtime::TypeKey) -> Result<Value> {
        let class = self.class(key)?;
        let shape = class["shape"].as_str().ok_or("Missing runtime shape.")?;
        if !matches!(shape, "object" | "array" | "string" | "unsupported_array") {
            return Err("Unsupported recovery route owner.".into());
        }
        Ok(
            serde_json::json!({"image":key.image,"type_name":key.type_name,
            "shape":shape,"element_inline_type":Self::named_type(&class["element_inline_type"])?}),
        )
    }

    fn field(&self, reference: &runtime::FieldRef) -> Result<Value> {
        let class = self.class(&reference.declaring)?;
        let fields = class["fields"]
            .as_array()
            .ok_or("Missing declared fields.")?;
        let field = fields
            .get(reference.field_id)
            .ok_or("Missing route field.")?;
        let name = field["name"].as_str().ok_or("Missing route field name.")?;
        if name.is_empty() || fields.iter().filter(|f| f["name"] == name).count() != 1 {
            return Err("Recovery requires an unambiguous declared field name.".into());
        }
        let inline = match field.get("inline") {
            None => false, // Schema 4 did not traverse embedded fields.
            Some(value) => value
                .as_bool()
                .ok_or("Invalid embedded field descriptor.")?,
        };
        if !field["declared_type"].is_null() && !field["declared_type"].is_string() {
            return Err("Invalid declared type name.".into());
        }
        Ok(
            serde_json::json!({"declaring":self.owner(&reference.declaring)?,
            "name":name,"type_code":field["type_code"],"width":field["width"],
            "reference":field["reference"],"inline":inline,"flags":field["flags"],
            "inline_type":Self::named_type(&field["inline_type"])?,"declared_type":field["declared_type"]}),
        )
    }

    fn route(&self, candidate: &Candidate) -> Result<Value> {
        let mut steps = Vec::new();
        for step in &candidate.locator.steps {
            let member = match &step.member {
                runtime::Member::Field { field } => serde_json::json!({"field":self.field(field)?}),
                runtime::Member::Element { index } => serde_json::json!({"element":index}),
            };
            steps.push(serde_json::json!({"owner":self.owner(&step.owner)?,"member":member}));
        }
        Ok(
            serde_json::json!({"root":self.field(&candidate.locator.root)?,"steps":steps,
            "scalar_type":candidate.type_code,"scalar_width":candidate.bytes.len()}),
        )
    }
}

/// Search declared member structure after a reader/layout change. Exact names,
/// qualified owner/declaring types, shapes, field representations, flags, embedded
/// type names and array indices must agree. Offsets, field ordinals, layout hashes
/// and values are deliberately not identity: the returned locator comes wholly
/// from the new observed graph. Zero or multiple matches grant no candidate.
pub fn propose(
    source_report: &Value,
    source_path: &str,
    current_report: &Value,
) -> Result<Proposal> {
    let strict = find(source_report, source_path, current_report)?;
    let source_route = Metadata::new(source_report)?.route(&strict.source)?;
    let metadata = Metadata::new(current_report)?;
    let mut matching_routes = 0;
    let mut candidate = None;
    // Visit the full graph even on a runtime mismatch, preserving its validation.
    runtime::visit_candidates(current_report, "", 0, 1, &mut |row| {
        let a = &row.locator;
        let b = &strict.source.locator;
        let same_type = |a: &runtime::TypeKey, b: &runtime::TypeKey| {
            a.image == b.image && a.type_name == b.type_name
        };
        if row.type_code != strict.source.type_code
            || !same_type(&a.root.declaring, &b.root.declaring)
            || a.steps.len() != b.steps.len()
            || !a.steps.iter().zip(&b.steps).all(|(a, b)| {
                same_type(&a.owner, &b.owner)
                    && match (&a.member, &b.member) {
                        (
                            runtime::Member::Field { field: a },
                            runtime::Member::Field { field: b },
                        ) => same_type(&a.declaring, &b.declaring),
                        (
                            runtime::Member::Element { index: a },
                            runtime::Member::Element { index: b },
                        ) => a == b,
                        _ => false,
                    }
            })
        {
            return Ok(());
        }
        if metadata.route(row)? == source_route {
            matching_routes += 1;
            candidate = Some(row.clone());
        }
        Ok(())
    })?;
    let status = if strict.status == "runtime_changed" {
        "runtime_changed"
    } else {
        match matching_routes {
            0 => "not_observed",
            1 => "candidate_for_revalidation",
            _ => "ambiguous",
        }
    };
    if status != "candidate_for_revalidation" {
        candidate = None;
    }
    Ok(Proposal {
        policy: "observed_declared_route_proposal_v1".into(), strict, source_route,
        status: status.into(), matching_routes, candidate, edit_authority: false,
        limitation: "A structural search proposal from a newly observed graph, not a compatible layout, persistent object, verified effect or edit authority. Runtime location and declared route descriptors agree, but catalogue/layout hashes, offsets and ordinals may differ. Preserve the strict result and require fresh reads, a new input-response comparison and restoration before making an effect claim.".into(),
    })
}
