//! Bounded path-scoped text examples for choosing types, never control names.
use crate::{Result, runtime_inventory::Inventory};
use serde::{Deserialize, Serialize};

pub const MAX_CONTEXT_BYTES: usize = 16 * 1024;
pub const MAX_SAMPLES_PER_TYPE: usize = 4;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObjectSample {
    pub field_name: String,
    pub path_id: String,
    pub object_text: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Type {
    pub id: usize,
    pub name: String,
    pub name_shortened: bool,
    pub field_labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub object_samples: Vec<ObjectSample>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Index {
    pub types: Vec<Type>,
    pub available_object_samples: usize,
    pub omitted_object_samples: usize,
}

pub fn build(inventory: &Inventory, byte_budget: usize) -> Result<Index> {
    let mut index = Index {
        types: inventory
            .types
            .iter()
            .map(|t| Type {
                id: t.id,
                name: t.key.type_name.chars().take(120).collect(),
                name_shortened: t.key.type_name.chars().count() > 120,
                field_labels: t.fields.iter().map(|f| f.name.clone()).collect(),
                object_samples: vec![],
            })
            .collect(),
        available_object_samples: 0,
        omitted_object_samples: 0,
    };
    let candidates: Vec<Vec<ObjectSample>> = inventory
        .types
        .iter()
        .map(|t| {
            let mut rows = vec![];
            // Visit fields before additional paths to the same field.
            for sample_index in 0..crate::runtime_inventory::MAX_PATH_SAMPLES {
                for field in &t.fields {
                    let Some(sample) = field.samples.get(sample_index) else {
                        continue;
                    };
                    let mut text = vec![];
                    for hint in &sample.context {
                        if !hint.truncated
                            && hint.text.len() <= 256
                            && !hint.text.trim().is_empty()
                            && !text.contains(&hint.text)
                        {
                            text.push(hint.text.clone());
                        }
                    }
                    if !text.is_empty() {
                        rows.push(ObjectSample {
                            field_name: field.name.clone(),
                            path_id: sample.path_id.clone(),
                            object_text: text,
                        });
                    }
                }
            }
            rows
        })
        .collect();
    index.available_object_samples = candidates.iter().map(Vec::len).sum();
    index.omitted_object_samples = index.available_object_samples;
    let base = serde_json::to_vec(&index).map_err(|e| e.to_string())?.len();
    if base > byte_budget {
        return Err("Observed type/field labels exceed the selection prompt budget.".into());
    }
    let mut remaining = (byte_budget - base).min(MAX_CONTEXT_BYTES);
    let rounds = candidates.iter().map(Vec::len).max().unwrap_or(0);
    // One sample from each type before additional examples. No semantic scoring
    // or game/field-name preference is used to allocate the text budget.
    for row in 0..rounds {
        for (t, samples) in index.types.iter_mut().zip(&candidates) {
            if t.object_samples.len() == MAX_SAMPLES_PER_TYPE {
                continue;
            }
            let Some(sample) = samples.get(row) else {
                continue;
            };
            let overhead = if t.object_samples.is_empty() {
                ",\"object_samples\":[]".len()
            } else {
                1
            };
            let cost = serde_json::to_vec(sample).map_err(|e| e.to_string())?.len() + overhead;
            if cost > remaining {
                continue;
            }
            remaining -= cost;
            t.object_samples.push(sample.clone());
            index.omitted_object_samples -= 1;
        }
    }
    // The omission count only shrinks, so its decimal representation cannot
    // grow beyond the conservative base allocation above.
    if serde_json::to_vec(&index).map_err(|e| e.to_string())?.len() > byte_budget {
        return Err("Type selection index exceeded its byte budget.".into());
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtime::{FieldRef, Scope, TextHint, TypeKey},
        runtime_inventory::{ObservedField, ObservedType, PathSample},
    };

    fn inventory() -> Inventory {
        let types = (0..2)
            .map(|id| {
                let key = TypeKey {
                    image: "observed".into(),
                    type_name: format!("T{id}"),
                    layout_sha256: "a".repeat(64),
                };
                let fields = (0..8)
                    .map(|field_id| ObservedField {
                        field_id,
                        name: format!("value{field_id}"),
                        type_code: 12,
                        values: vec![],
                        path_count: 2,
                        samples: (0..2)
                            .map(|sample| PathSample {
                                path_id: format!("{:064x}", id * 100 + field_id * 2 + sample),
                                value: serde_json::json!(3),
                                context: vec![TextHint {
                                    field: FieldRef {
                                        declaring: key.clone(),
                                        field_id: 9,
                                    },
                                    text: if id == 0 { "Speed" } else { "Energy" }.into(),
                                    truncated: false,
                                }],
                            })
                            .collect(),
                    })
                    .collect();
                ObservedType { id, key, fields }
            })
            .collect();
        Inventory {
            schema: 1,
            report_sha256: "b".repeat(64),
            scope: Scope {
                pid: 1,
                created: "0000000000000001".into(),
                session: 1,
            },
            types,
            reachable_paths: 32,
            omitted_core_paths: 0,
            omitted_array_elements: 0,
            omitted_non_numeric_paths: 0,
            limitation: "fixture".into(),
        }
    }

    #[test]
    fn same_object_text_retains_its_path_and_complete_field_labels() {
        let mut inventory = inventory();
        let mut ignored = inventory.types[0].fields[0].samples[0].context[0].clone();
        ignored.text = "Truncated name".into();
        ignored.truncated = true;
        inventory.types[0].fields[0].samples[0]
            .context
            .push(ignored);
        let index = build(&inventory, 10_000).unwrap();
        assert_eq!(index.available_object_samples, 32);
        assert_eq!(index.omitted_object_samples, 24);
        for (observed, t) in inventory.types.iter().zip(&index.types) {
            assert_eq!(
                t.field_labels,
                observed
                    .fields
                    .iter()
                    .map(|f| f.name.clone())
                    .collect::<Vec<_>>()
            );
            assert_eq!(t.object_samples.len(), 4);
            for (i, sample) in t.object_samples.iter().enumerate() {
                assert_eq!(sample.field_name, format!("value{i}"));
                assert_eq!(sample.path_id, observed.fields[i].samples[0].path_id);
                assert_eq!(
                    sample.object_text,
                    vec![if t.id == 0 { "Speed" } else { "Energy" }]
                );
            }
        }
    }

    #[test]
    fn byte_pressure_shares_context_across_types_without_dropping_fields() {
        let inventory = inventory();
        let mut base = build(&inventory, 10_000).unwrap();
        for t in &mut base.types {
            t.object_samples.clear();
        }
        base.omitted_object_samples = base.available_object_samples;
        let base_bytes = serde_json::to_vec(&base).unwrap().len();
        let byte_budget = base_bytes + 340;
        let limited = build(&inventory, byte_budget).unwrap();
        assert!(serde_json::to_vec(&limited).unwrap().len() <= byte_budget);
        assert!(limited.types.iter().all(|t| t.object_samples.len() == 1));
        assert_eq!(limited.omitted_object_samples, 30);
        assert_eq!(build(&inventory, base_bytes).unwrap(), base);
        assert!(build(&inventory, base_bytes - 1).is_err());
    }

    #[test]
    fn a_type_hint_never_authorizes_naming_a_different_object() {
        use crate::runtime_hypotheses as h;
        let inventory = inventory();
        let index = build(&inventory, 10_000).unwrap();
        assert_eq!(index.types[1].object_samples[0].object_text, ["Energy"]);
        let selection = h::select(&inventory, &[0, 1]).unwrap();
        let proposal = |name: &str| h::Proposal {
            candidate: 0,
            name: name.into(),
            explanation: "Its role is unverified and needs an effect test.".into(),
            citations: vec![h::Citation {
                source: "object_text".into(),
                index: Some(0),
                quote: name.into(),
            }],
            alternatives: vec![],
            question: "Does this parameter affect the observed behavior?".into(),
        };
        let batch = h::validate(
            &selection,
            h::Proposals {
                hypotheses: vec![proposal("Energy")],
            },
        )
        .unwrap();
        assert!(batch.accepted.is_empty());
        assert_eq!(batch.rejected.len(), 1);
        let batch = h::validate(
            &selection,
            h::Proposals {
                hypotheses: vec![proposal("Speed")],
            },
        )
        .unwrap();
        assert_eq!(batch.accepted.len(), 1);
        assert!(batch.rejected.is_empty());
    }
}
