//! Metadata context for a checked scalar path; labels do not establish meaning.
use crate::{
    Result,
    runtime::{Locator, Member, TypeKey},
    visual::digest,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Location {
    pub path_id: String,
    /// First step within the nearest enclosing reference object or array element.
    pub anchor_step: usize,
    pub role_id: String,
    /// Complete runtime labels from anchor through leaf; untrusted observations.
    pub members: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
struct Field {
    id: usize,
    name: String,
    #[serde(default)]
    inline: bool,
    #[serde(default)]
    inline_type: Option<TypeKey>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
struct Class {
    key: TypeKey,
    fields: Vec<Field>,
    #[serde(default)]
    element_inline_type: Option<TypeKey>,
}
pub struct Index(BTreeMap<String, Class>);
fn key(typ: &TypeKey) -> Result<String> {
    serde_json::to_string(typ).map_err(|e| e.to_string())
}
impl Index {
    /// Consumers must also validate the report's graph through runtime candidates.
    /// This index supplies labels/storage context, never independent path authority.
    pub fn new(report: &Value) -> Result<Self> {
        let rows = report["classes"]
            .as_array()
            .ok_or("Missing runtime classes")?;
        if rows.len() > 50_000 {
            return Err("Runtime class context exceeds budget".into());
        }
        let mut classes = BTreeMap::new();
        for row in rows {
            let class: Class = serde_json::from_value(row.clone()).map_err(|e| e.to_string())?;
            class.key.validate()?;
            if class.fields.len() > 200_000
                || class.fields.iter().enumerate().any(|(i, f)| f.id != i)
            {
                return Err("Invalid field context".into());
            }
            let id = key(&class.key)?;
            if classes.get(&id).is_some_and(|old| old != &class) {
                return Err("Conflicting metadata for one runtime type".into());
            }
            classes.insert(id, class);
        }
        Ok(Self(classes))
    }
    pub fn locate(&self, path: &Locator) -> Result<Location> {
        path.validate(&path.scope)?;
        let mut names = vec![];
        let mut embedded = vec![];
        for (i, step) in path.steps.iter().enumerate() {
            let owner = self
                .0
                .get(&key(&step.owner)?)
                .ok_or("Missing owner context")?;
            let (name, inline_type) = match &step.member {
                Member::Field { field } => {
                    let f = self
                        .0
                        .get(&key(&field.declaring)?)
                        .and_then(|c| c.fields.get(field.field_id))
                        .ok_or("Missing field context")?;
                    if f.inline != f.inline_type.is_some() {
                        return Err("Incomplete embedded type context".into());
                    }
                    (f.name.clone(), f.inline_type.as_ref())
                }
                Member::Element { index } => {
                    (format!("[{index}]"), owner.element_inline_type.as_ref())
                }
            };
            if let Some(typ) = inline_type
                && path.steps.get(i + 1).is_none_or(|s| &s.owner != typ)
            {
                return Err("Embedded path has a different owner type".into());
            }
            names.push(name);
            // Array elements remain distinct roles. Only inline field hops join
            // their enclosing object; references never cross the role boundary.
            embedded.push(inline_type.is_some() && matches!(step.member, Member::Field { .. }));
        }
        let mut anchor = path.steps.len() - 1;
        while anchor > 0 && embedded[anchor - 1] {
            anchor -= 1;
        }
        Ok(Location {
            path_id: path.id(),
            anchor_step: anchor,
            role_id: role_id_at(path, anchor)?,
            members: names[anchor..].to_vec(),
        })
    }
}
pub fn role_id_at(path: &Locator, anchor: usize) -> Result<String> {
    path.validate(&path.scope)?;
    let owner = &path
        .steps
        .get(anchor)
        .ok_or("Invalid enclosing role step")?
        .owner;
    Ok(digest(
        &serde_json::to_vec(&json!({"scope":path.scope,"root":path.root,
        "references":&path.steps[..anchor],"owner":owner}))
        .map_err(|e| e.to_string())?,
    ))
}
