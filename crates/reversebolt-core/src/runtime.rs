//! Runtime paths describe a current role, not persistent object identity or edit authority.
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MAX_PATH_STEPS: usize = 8;
pub const MAX_REFRESH_PATHS: usize = 32;
pub const MAX_RUNTIME_READINGS: usize = 16;

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
    /// Text reached directly from this object's fields in the discovery snapshot.
    /// These hints are untrusted observations, not durable identity or instructions.
    #[serde(default)]
    pub context: Vec<TextHint>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextHint {
    pub field: FieldRef,
    pub text: String,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogSource {
    pub path: String,
    /// SHA-256 of the parsed report's canonical serde_json encoding.
    pub document_sha256: String,
    pub observed_unix_ms: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchPage {
    pub query: String,
    pub offset: usize,
    pub matches: usize,
    pub candidates: Vec<Candidate>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct View {
    pub available: bool,
    pub phase: String,
    pub catalog_id: String,
    pub revision: u64,
    pub cards: Vec<Card>,
    pub notice: Option<String>,
    pub evidence: Option<String>,
    #[serde(default)]
    pub catalog_source: Option<CatalogSource>,
    #[serde(default)]
    pub search: Option<SearchPage>,
    #[serde(default)]
    pub card_generation: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Card {
    pub candidate: Candidate,
    pub value: Value,
    pub status: String,
    pub observed_unix_ms: u128,
    /// Fresh refreshes only; discovery snapshots are retained separately.
    #[serde(default)]
    pub readings: Vec<Reading>,
    #[serde(default)]
    pub readings_dropped: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reading {
    pub revision: u64,
    /// Bounds of the complete inspection operation, not an exact sampling instant.
    pub started_unix_ms: u128,
    pub completed_unix_ms: u128,
    pub status: String,
    pub value: Value,
    pub bytes: Vec<u8>,
    pub evidence_path: String,
    /// Canonical JSON digest of the refresh report.
    pub report_sha256: String,
}

impl Card {
    /// Differences between consecutive successful observations; never bridge a gap.
    /// A path may change occupants, so this does not establish one object's motion.
    pub fn observed_differences(&self) -> usize {
        self.readings
            .windows(2)
            .filter(|pair| {
                pair[0].status == "read"
                    && pair[1].status == "read"
                    && pair[0].bytes != pair[1].bytes
            })
            .count()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Task {
    Discover,
    Refresh {
        catalog_id: String,
        after_revision: u64,
        ids: Vec<String>,
    },
}

impl Task {
    /// Pin every future refresh to its expected revision; never silently rebase.
    pub fn sequence(&self, count: usize) -> Result<Vec<Self>> {
        self.validate()?;
        if !(1..=MAX_RUNTIME_READINGS).contains(&count) {
            return Err("Use one to sixteen internal-value checks.".into());
        }
        match self {
            Self::Discover if count == 1 => Ok(vec![self.clone()]),
            Self::Discover => Err("Repeated checks require selected values, not discovery.".into()),
            Self::Refresh {
                catalog_id,
                after_revision,
                ids,
            } => {
                // Every step also needs room to publish its resulting revision.
                after_revision
                    .checked_add(count as u64)
                    .ok_or("Runtime revision limit reached.")?;
                Ok((0..count)
                    .map(|index| Self::Refresh {
                        catalog_id: catalog_id.clone(),
                        after_revision: after_revision + index as u64,
                        ids: ids.clone(),
                    })
                    .collect())
            }
        }
    }
    pub fn validate(&self) -> Result<()> {
        if let Self::Refresh {
            catalog_id, ids, ..
        } = self
            && (catalog_id.is_empty()
                || catalog_id.len() > 64
                || ids.is_empty()
                || ids.len() > 4
                || ids
                    .iter()
                    .any(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
                || ids.iter().collect::<BTreeSet<_>>().len() != ids.len())
        {
            return Err(
                "Select one to four distinct runtime values from the current catalogue.".into(),
            );
        }
        Ok(())
    }
}

impl View {
    pub fn show_values(&mut self, catalog_id: &str, revision: u64, ids: &[String]) -> Result<()> {
        if catalog_id != self.catalog_id || revision != self.revision {
            return Err("The catalogue changed. Choose values from the current search.".into());
        }
        if ids.is_empty() || ids.len() > 8 || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
        {
            return Err("Choose one to eight distinct search results.".into());
        }
        let page = self
            .search
            .as_ref()
            .ok_or("Search internal values first.")?;
        let observed = self
            .catalog_source
            .as_ref()
            .ok_or("Discovery evidence unavailable.")?
            .observed_unix_ms;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or("Runtime revision limit reached.")?;
        let generation = self
            .card_generation
            .checked_add(1)
            .ok_or("Runtime card limit reached.")?;
        let mut selected = Vec::new();
        for id in ids {
            let candidate = page
                .candidates
                .iter()
                .find(|c| &c.id == id)
                .ok_or("Choose a value from the current search results.")?;
            selected.push(
                self.cards
                    .iter()
                    .find(|c| &c.candidate.id == id)
                    .cloned()
                    .unwrap_or_else(|| Card {
                        candidate: candidate.clone(),
                        value: candidate.value.clone(),
                        status: "read".into(),
                        observed_unix_ms: observed,
                        readings: Vec::new(),
                        readings_dropped: 0,
                    }),
            );
        }
        selected.extend(
            self.cards
                .iter()
                .filter(|c| !ids.contains(&c.candidate.id))
                .cloned(),
        );
        selected.truncate(64);
        self.cards = selected;
        self.revision = revision;
        self.card_generation = generation;
        Ok(())
    }
    pub fn paths(&self, task: &Task) -> Result<Vec<Locator>> {
        task.validate()?;
        match task {
            Task::Discover => Ok(Vec::new()),
            Task::Refresh {
                catalog_id,
                after_revision,
                ids,
            } => {
                if &self.catalog_id != catalog_id || self.revision != *after_revision {
                    return Err(
                        "These runtime readings changed. Select them from current status.".into(),
                    );
                }
                ids.iter()
                    .map(|id| {
                        self.cards
                            .iter()
                            .find(|c| &c.candidate.id == id)
                            .map(|c| c.candidate.locator.clone())
                            .ok_or_else(|| "Runtime value is no longer in this catalogue.".into())
                    })
                    .collect()
            }
        }
    }
    pub fn apply_refresh(
        &mut self,
        task: &Task,
        report: &Value,
        window: [u128; 2],
        evidence: &str,
    ) -> Result<()> {
        if window[1] < window[0] {
            return Err("Runtime inspection clock moved backwards; timing is unavailable.".into());
        }
        let paths = self.paths(task)?;
        if !matches!(task, Task::Refresh { .. }) {
            return Err("Expected a refresh task.".into());
        }
        let checks = report["checks"]
            .as_array()
            .ok_or("Missing refresh checks.")?;
        if checks.len() != paths.len() || report["edit_authority"] != false {
            return Err("Invalid refresh result.".into());
        }
        let mut updates = Vec::new();
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or("Runtime revision limit reached.")?;
        for (path, check) in paths.iter().zip(checks) {
            let id = path.id();
            if check["path_id"] != id
                || serde_json::from_value::<Scope>(report["scope"].clone())
                    .ok()
                    .as_ref()
                    != Some(&path.scope)
            {
                return Err("Refresh result belongs to another path or process.".into());
            }
            let status = check["status"].as_str().ok_or("Missing refresh status.")?;
            let mut recorded_bytes = Vec::new();
            let value = if status == "read" {
                let bytes: Vec<u8> = serde_json::from_value(check["bytes"].clone())
                    .map_err(|_| "Invalid value bytes.")?;
                let kind = check["type_code"]
                    .as_i64()
                    .and_then(|k| i32::try_from(k).ok())
                    .ok_or("Missing scalar kind.")?;
                let original = &self
                    .cards
                    .iter()
                    .find(|c| c.candidate.id == id)
                    .unwrap()
                    .candidate;
                if kind != original.type_code || bytes.len() != original.bytes.len() {
                    return Err("Refresh scalar representation differs from discovery.".into());
                }
                let value = scalar_value(kind, &bytes)?;
                if value != check["value"] {
                    return Err("Refresh value contradicts its bytes.".into());
                }
                recorded_bytes = bytes;
                value
            } else {
                Value::Null
            };
            updates.push((id, status.to_owned(), value, recorded_bytes));
        }
        let report_sha256 = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(report).expect("JSON report"))
        );
        for (id, status, value, bytes) in updates {
            let card = self
                .cards
                .iter_mut()
                .find(|c| c.candidate.id == id)
                .unwrap();
            card.readings.push(Reading {
                revision: next_revision,
                started_unix_ms: window[0],
                completed_unix_ms: window[1],
                status: status.clone(),
                value: value.clone(),
                bytes,
                evidence_path: evidence.into(),
                report_sha256: report_sha256.clone(),
            });
            if card.readings.len() > MAX_RUNTIME_READINGS {
                let dropped = card.readings.len() - MAX_RUNTIME_READINGS;
                card.readings.drain(..dropped);
                card.readings_dropped = card.readings_dropped.saturating_add(dropped as u64);
            }
            card.value = value;
            card.status = status;
            card.observed_unix_ms = window[0];
        }
        self.revision = next_revision;
        Ok(())
    }
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
    #[serde(default)]
    core_image: Option<String>,
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
    #[serde(default)]
    strings: Vec<TextRead>,
}
#[derive(Deserialize)]
struct TextRead {
    node: usize,
    read_ok: bool,
    text: Option<String>,
    truncated: bool,
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
    Ok(select(report, "", 0, limit)?.candidates)
}

/// Search all observed candidates before applying the result limit. A search
/// does not read the target; results belong to the retained discovery snapshot.
pub fn search(report: &Value, query: &str, offset: usize) -> Result<SearchPage> {
    if query.chars().count() > 96 || query.split_whitespace().count() > 12 || offset > 4096 {
        return Err(
            "Use a shorter search and narrow it before browsing beyond 4,096 results.".into(),
        );
    }
    select(report, query, offset, 20)
}

fn select(report: &Value, query: &str, offset: usize, limit: usize) -> Result<SearchPage> {
    visit_candidates(report, query, offset, limit, &mut |_| Ok(()))
}

/// Internal streaming view of every reachable scalar, before the display limit.
/// The visitor must not publish partial results if later report validation fails.
pub(crate) fn visit_candidates(
    report: &Value,
    query: &str,
    offset: usize,
    limit: usize,
    visitor: &mut dyn FnMut(&Candidate) -> Result<()>,
) -> Result<SearchPage> {
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
    let mut groups: BTreeMap<(u8, bool), BTreeMap<String, VecDeque<Candidate>>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let terms: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut matches = 0;
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
        let mut texts = BTreeMap::new();
        if shot.strings.len() > 1024 {
            return Err("Too many runtime text observations.".into());
        }
        for text in &shot.strings {
            let node = nodes.get(&text.node).ok_or("Text node missing.")?;
            if texts.insert(text.node, text).is_some() {
                return Err("Duplicate runtime text observation.".into());
            }
            if text.read_ok
                && (node
                    .class
                    .and_then(|id| classes.get(&id))
                    .is_none_or(|c| c.shape != "string")
                    || text
                        .text
                        .as_ref()
                        .is_none_or(|s| s.encode_utf16().count() > 128))
            {
                return Err("Invalid runtime text observation.".into());
            }
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
            let mut context = Vec::new();
            for read in reads.get(&id).into_iter().flatten() {
                if !read.read_ok {
                    continue;
                }
                let Some(child) = read.reference["id"].as_u64() else {
                    continue;
                };
                let Some(text) = texts.get(&(child as usize)).filter(|t| t.read_ok) else {
                    continue;
                };
                let Some(label) = text.text.as_ref().filter(|s| !s.trim().is_empty()) else {
                    continue;
                };
                let declaring = classes
                    .get(&read.declaring_class)
                    .ok_or("Text field owner missing.")?;
                let Some(field) = read.field.and_then(|f| declaring.fields.get(f)) else {
                    continue;
                };
                if !field.reference
                    || field.width != read.bytes.len()
                    || field.type_code != read.type_code
                    || field.flags & 0x10 != 0
                {
                    return Err("Text reference schema mismatch.".into());
                }
                if context.len() < 8 {
                    context.push(TextHint {
                        field: FieldRef {
                            declaring: declaring.key.clone(),
                            field_id: field.id,
                        },
                        text: label.clone(),
                        truncated: text.truncated,
                    });
                }
            }
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
                    let searchable = if terms.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "{} {} {} {} {}",
                            name,
                            owner.key.type_name,
                            root.declaring.type_name,
                            declaring.key.image,
                            context
                                .iter()
                                .map(|c| c.text.as_str())
                                .collect::<Vec<_>>()
                                .join(" ")
                        )
                        .to_lowercase()
                    };
                    if !terms.iter().all(|term| searchable.contains(term))
                        || !seen.insert(key.clone())
                    {
                        continue;
                    }
                    matches += 1;
                    let image = &declaring.key.image;
                    // Runtime-supplied core library identity, never game names.
                    // Non-core declarations lead, then core containers rooted elsewhere.
                    let priority = match report.core_image.as_deref() {
                        Some(core) if image != core => 3,
                        Some(core) if root.declaring.image != core => 2,
                        Some(_) => 0,
                        None => 1,
                    };
                    let bucket = groups
                        .entry((priority, context.iter().any(|c| !c.truncated)))
                        .or_default()
                        .entry(image.clone())
                        .or_default();
                    let candidate = Candidate {
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
                        context: context.clone(),
                    };
                    visitor(&candidate)?;
                    if bucket.len() < limit + offset {
                        bucket.push_back(candidate);
                    }
                }
            }
        }
    }
    let mut answer = Vec::new();
    for (_, mut images) in groups.into_iter().rev() {
        while answer.len() < limit + offset {
            let before = answer.len();
            for bucket in images.values_mut() {
                if answer.len() == limit + offset {
                    break;
                }
                if let Some(candidate) = bucket.pop_front() {
                    answer.push(candidate);
                }
            }
            if answer.len() == before {
                break;
            }
        }
    }
    Ok(SearchPage {
        query: query.to_owned(),
        offset,
        matches,
        candidates: answer.into_iter().skip(offset).collect(),
    })
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
