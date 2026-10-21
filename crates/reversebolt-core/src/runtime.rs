//! Runtime paths describe a current role, not persistent object identity or edit authority.
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MAX_PATH_STEPS: usize = 8;
pub const MAX_REFRESH_PATHS: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub pid: u32,
    pub created: String,
    pub session: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeKey {
    pub image: String,
    /// Runtime's qualified type name, including constructed generic arguments.
    pub type_name: String,
    /// Ordered declared field layout, shape and instance size, derived at runtime.
    pub layout_sha256: String,
}

impl TypeKey {
    pub fn validate(&self) -> Result<()> {
        if self.image.is_empty()
            || self.image.len() > 512
            || self.type_name.is_empty()
            || self.type_name.len() > 4096
            || self.layout_sha256.len() != 64
            || !self.layout_sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("Invalid runtime type descriptor.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldRef {
    pub declaring: TypeKey,
    /// Index in the ordered layout covered by declaring.layout_sha256.
    pub field_id: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Member {
    Field { field: FieldRef },
    Element { index: usize },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub owner: TypeKey,
    pub member: Member,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Locator {
    pub scope: Scope,
    pub root: FieldRef,
    /// Reference hops followed by exactly one scalar field/array element.
    pub steps: Vec<Step>,
}

impl Locator {
    pub fn validate(&self, expected: &Scope) -> Result<()> {
        if &self.scope != expected {
            return Err("Runtime path belongs to a different process identity.".into());
        }
        if self.scope.pid == 0
            || self.scope.created.len() != 16
            || !self.scope.created.bytes().all(|b| b.is_ascii_hexdigit())
            || self.steps.is_empty()
            || self.steps.len() > MAX_PATH_STEPS
        {
            return Err("Invalid runtime path scope or length.".into());
        }
        self.root.declaring.validate()?;
        if self.root.field_id >= 200_000 {
            return Err("Runtime root field exceeds metadata budget.".into());
        }
        for step in &self.steps {
            step.owner.validate()?;
            match &step.member {
                Member::Field { field } => {
                    field.declaring.validate()?;
                    if field.field_id >= 200_000 {
                        return Err("Runtime field exceeds metadata budget.".into());
                    }
                }
                Member::Element { index } if *index >= 1_000_000 => {
                    return Err("Runtime array index exceeds refresh budget.".into());
                }
                _ => {}
            }
        }
        Ok(())
    }
    pub fn id(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("serializable locator"))
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    /// Stable for this path and process; not the identity of its current occupant.
    pub id: String,
    pub name: String,
    pub description: String,
    pub locator: Locator,
    pub value: Value,
    pub bytes: Vec<u8>,
    pub type_code: i32,
    pub snapshot_id: usize,
    pub node_id: usize,
    pub edit_authority: bool,
}

// Only fields required to construct and check a path are deserialized here.
// The raw discovery file retains budgets and other coverage evidence.
#[derive(Deserialize)]
struct Report {
    schema: u32,
    pid: u32,
    created: String,
    session: u32,
    edit_authority: bool,
    classes: Vec<Class>,
    snapshots: Vec<Snapshot>,
}
#[derive(Deserialize)]
struct Class {
    id: usize,
    key: TypeKey,
    shape: String,
    fields: Vec<Field>,
}
#[derive(Deserialize)]
struct Field {
    id: usize,
    name: String,
    reference: bool,
    type_code: i32,
    width: usize,
    flags: i32,
}
#[derive(Deserialize)]
struct Snapshot {
    snapshot_id: usize,
    nodes: Vec<Node>,
    roots: Vec<Root>,
    reads: Vec<Read>,
}
#[derive(Deserialize)]
struct Node {
    id: usize,
    class: Option<usize>,
}
#[derive(Deserialize)]
struct Root {
    class: usize,
    field: usize,
    node: Value,
    read_ok: bool,
}
#[derive(Deserialize)]
struct Read {
    node: usize,
    declaring_class: usize,
    field: Option<usize>,
    element_index: Option<usize>,
    type_code: i32,
    bytes: Vec<u8>,
    value: Value,
    reference: Value,
    read_ok: bool,
}

/// Build scalar candidates from actual observed reference edges. No addresses are
/// copied into locators. Unknown types, unread edges and budget omissions abstain.
pub fn candidates(report: &Value, limit: usize) -> Result<Vec<Candidate>> {
    if limit == 0 || limit > 4096 {
        return Err("Candidate limit must be between 1 and 4096.".into());
    }
    let report: Report = serde_json::from_value(report.clone()).map_err(|e| e.to_string())?;
    if report.schema != 4
        || report.edit_authority
        || report.classes.len() > 50_000
        || report.snapshots.len() > 10_000
    {
        return Err("Unsupported runtime discovery report.".into());
    }
    let scope = Scope {
        pid: report.pid,
        created: report.created,
        session: report.session,
    };
    let classes: BTreeMap<_, _> = report.classes.iter().map(|c| (c.id, c)).collect();
    if classes.len() != report.classes.len() {
        return Err("Duplicate runtime class ID.".into());
    }
    for class in classes.values() {
        class.key.validate()?;
        if class.fields.len() > 200_000 || class.fields.iter().enumerate().any(|(i, f)| f.id != i) {
            return Err("Invalid runtime field catalogue.".into());
        }
    }
    let mut answer = Vec::new();
    let mut seen = BTreeSet::new();
    for shot in report.snapshots {
        if shot.nodes.len() > 1024 || shot.reads.len() > 16384 || shot.roots.len() > 64 {
            return Err("Runtime snapshot exceeds its contract.".into());
        }
        let nodes: BTreeMap<_, _> = shot.nodes.iter().map(|n| (n.id, n)).collect();
        if nodes.len() != shot.nodes.len() {
            return Err("Duplicate runtime node ID.".into());
        }
        let mut routes: BTreeMap<usize, (FieldRef, Vec<Step>)> = BTreeMap::new();
        let mut queue = VecDeque::new();
        for root in &shot.roots {
            let class = classes.get(&root.class).ok_or("Root class missing.")?;
            let field = class.fields.get(root.field).ok_or("Root field missing.")?;
            if !field.reference || field.flags & 0x10 == 0 {
                return Err("Root is not a static reference.".into());
            }
            if root.read_ok
                && let Some(id) = root.node["id"].as_u64()
            {
                let id = usize::try_from(id).map_err(|_| "Invalid node ID.")?;
                if !nodes.contains_key(&id) {
                    return Err("Dangling root edge.".into());
                }
                if let std::collections::btree_map::Entry::Vacant(entry) = routes.entry(id) {
                    entry.insert((
                        FieldRef {
                            declaring: class.key.clone(),
                            field_id: field.id,
                        },
                        Vec::new(),
                    ));
                    queue.push_back(id);
                }
            }
        }
        let mut reads: BTreeMap<usize, Vec<&Read>> = BTreeMap::new();
        for read in &shot.reads {
            if !nodes.contains_key(&read.node) {
                return Err("Read node missing.".into());
            }
            if let Some(id) = read.reference["id"].as_u64()
                && !nodes.contains_key(&(id as usize))
            {
                return Err("Dangling reference edge.".into());
            }
            reads.entry(read.node).or_default().push(read);
        }
        while let Some(id) = queue.pop_front() {
            let (root, prefix) = routes[&id].clone();
            if prefix.len() >= MAX_PATH_STEPS {
                continue;
            }
            let Some(class_id) = nodes[&id].class else {
                continue;
            };
            let owner = classes.get(&class_id).ok_or("Node class missing.")?;
            for read in reads.get(&id).into_iter().flatten() {
                if !read.read_ok {
                    continue;
                }
                let declaring = classes
                    .get(&read.declaring_class)
                    .ok_or("Field owner missing.")?;
                let (member, name, is_reference) = match (read.field, read.element_index) {
                    (Some(field), None) => {
                        let field = declaring.fields.get(field).ok_or("Field missing.")?;
                        if field.width != read.bytes.len()
                            || field.type_code != read.type_code
                            || field.flags & 0x10 != 0
                        {
                            return Err("Field read schema mismatch.".into());
                        }
                        (
                            Member::Field {
                                field: FieldRef {
                                    declaring: declaring.key.clone(),
                                    field_id: field.id,
                                },
                            },
                            field.name.clone(),
                            field.reference,
                        )
                    }
                    (None, Some(index)) if owner.shape == "array" && declaring.id == owner.id => (
                        Member::Element { index },
                        format!("Element {index}"),
                        !read.reference.is_null(),
                    ),
                    _ => return Err("Invalid runtime member read.".into()),
                };
                let mut steps = prefix.clone();
                steps.push(Step {
                    owner: owner.key.clone(),
                    member,
                });
                if is_reference {
                    if let Some(child) = read.reference["id"].as_u64() {
                        let child = child as usize;
                        if let std::collections::btree_map::Entry::Vacant(entry) =
                            routes.entry(child)
                        {
                            entry.insert((root.clone(), steps));
                            queue.push_back(child);
                        }
                    }
                } else {
                    let decoded = scalar_value(read.type_code, &read.bytes)?;
                    if decoded != read.value {
                        return Err("Runtime value disagrees with retained bytes.".into());
                    }
                    let locator = Locator {
                        scope: scope.clone(),
                        root: root.clone(),
                        steps,
                    };
                    locator.validate(&scope)?;
                    let key = locator.id();
                    if seen.insert(key.clone()) && answer.len() < limit {
                        answer.push(Candidate {
                            id: key,
                            name,
                            description: "Runtime value; gameplay effect unverified.".into(),
                            locator,
                            value: decoded,
                            bytes: read.bytes.clone(),
                            type_code: read.type_code,
                            snapshot_id: shot.snapshot_id,
                            node_id: id,
                            edit_authority: false,
                        });
                    }
                }
            }
        }
    }
    Ok(answer)
}

pub fn scalar_value(kind: i32, bytes: &[u8]) -> Result<Value> {
    use serde_json::json;
    let width = match kind {
        2 | 4 | 5 => 1,
        3 | 6 | 7 => 2,
        8 | 9 | 12 => 4,
        10 | 11 | 13 => 8,
        _ => 0,
    };
    if width == 0 || bytes.len() != width {
        return Err("Unsupported scalar width/type.".into());
    }
    Ok(match kind {
        2 => json!({"boolean":bytes[0]!=0,"raw":bytes[0]}),
        4 => json!(bytes[0] as i8),
        5 => json!(bytes[0]),
        3 | 7 => json!(u16::from_le_bytes(bytes.try_into().unwrap())),
        6 => json!(i16::from_le_bytes(bytes.try_into().unwrap())),
        8 => json!(i32::from_le_bytes(bytes.try_into().unwrap())),
        9 => json!(u32::from_le_bytes(bytes.try_into().unwrap())),
        10 => json!(i64::from_le_bytes(bytes.try_into().unwrap()).to_string()),
        11 => json!(u64::from_le_bytes(bytes.try_into().unwrap()).to_string()),
        12 => json!(f32::from_le_bytes(bytes.try_into().unwrap()).to_string()),
        13 => json!(f64::from_le_bytes(bytes.try_into().unwrap()).to_string()),
        _ => unreachable!(),
    })
}
