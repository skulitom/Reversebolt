//! Literal object-label questions, never inferred gameplay meanings or edit authority.
use crate::{Result, runtime_hypotheses as h, runtime_inventory::Inventory};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const POLICY: &str = "complete-object-clue-v2";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coverage {
    pub policy: String,
    pub report_sha256: String,
    pub rows_without_labels: usize,
    pub ambiguous_rows: usize,
    pub incomplete_context_rows: usize,
    pub invalid_label_rows: usize,
    pub groups_available: usize,
    pub groups_deferred: usize,
    pub accepted_paths: Vec<String>,
    pub limitation: String,
}
pub struct Plan {
    pub batch: h::Batch,
    pub coverage: Coverage,
}

fn one_label(row: &h::Row) -> Option<(usize, &str)> {
    let labels: BTreeSet<_> = row.object_text.iter().collect();
    (labels.len() == 1).then(|| (0, row.object_text[0].as_str()))
}
fn proposal(row: &h::Row, index: usize, label: &str) -> h::Proposal {
    let name = label.trim().to_owned();
    h::Proposal {
        candidate: row.id,
        explanation: format!(
            "The object contains the complete text “{name}”. Its relationship to this value is untested."
        ),
        question: format!(
            "Does changing this value affect what the object's text “{name}” refers to?"
        ),
        name,
        citations: vec![h::Citation {
            source: "object_text".into(),
            index: Some(index),
            quote: label.into(),
        }],
        alternatives: vec![],
    }
}

pub fn plan(inventory: &Inventory, selection: &h::Selection) -> Result<Plan> {
    if h::select(inventory, &selection.selected_types)? != *selection {
        return Err("Object-label questions require the exact current inventory selection.".into());
    }
    let mut coverage = Coverage {
        policy:POLICY.into(), report_sha256:inventory.report_sha256.clone(),
        rows_without_labels:0, ambiguous_rows:0, incomplete_context_rows:0, invalid_label_rows:0,
        groups_available:0, groups_deferred:0, accepted_paths:vec![],
        limitation:"A literal clue, not a model interpretation or verified gameplay property. Other text may be incomplete or omitted; this clue is not a unique property identity. Paths may alias or change occupants. No card meaning, range or write permission is established.".into(),
    };
    let mut groups: BTreeMap<usize, BTreeMap<(usize, String), Vec<&h::Row>>> = BTreeMap::new();
    for row in &selection.rows {
        let sample = inventory
            .types
            .iter()
            .filter(|t| t.id == row.type_id)
            .flat_map(|t| t.fields.iter())
            .filter(|f| f.field_id == row.field_id)
            .flat_map(|f| f.samples.iter())
            .find(|s| s.path_id == row.path_id)
            .ok_or("The object's source path is unavailable.")?;
        if sample
            .context
            .iter()
            .any(|hint| hint.truncated || hint.text.len() > 256)
        {
            coverage.incomplete_context_rows += 1;
        }
        let Some((index, label)) = one_label(row) else {
            if row.object_text.is_empty() {
                coverage.rows_without_labels += 1;
            } else {
                coverage.ambiguous_rows += 1;
            }
            continue;
        };
        let p = proposal(row, index, label);
        if h::validate(
            selection,
            h::Proposals {
                hypotheses: vec![p],
            },
        )?
        .accepted
        .is_empty()
        {
            coverage.invalid_label_rows += 1;
            continue;
        }
        groups
            .entry(row.type_id)
            .or_default()
            .entry((row.field_id, label.into()))
            .or_default()
            .push(row);
    }
    coverage.groups_available = groups.values().map(BTreeMap::len).sum();
    let by_type: Vec<Vec<_>> = groups
        .into_values()
        .map(|groups| groups.into_values().collect())
        .collect();
    let rounds = by_type.iter().map(Vec::len).max().unwrap_or(0);
    let mut proposals = vec![];
    for round in 0..rounds {
        for groups in &by_type {
            let Some(rows) = groups.get(round) else {
                continue;
            };
            // A large ambiguous group is deferred whole, never stripped of rivals.
            if rows.len() > 5 || proposals.len() == 12 {
                continue;
            }
            let mut paths = rows.clone();
            paths.sort_by_key(|r| &r.path_id);
            let (index, label) = one_label(paths[0]).unwrap();
            let mut p = proposal(paths[0], index, label);
            p.alternatives = paths[1..].iter().map(|r| r.id).collect();
            proposals.push(p);
        }
    }
    let batch = h::validate(
        selection,
        h::Proposals {
            hypotheses: proposals,
        },
    )?;
    coverage.groups_deferred = coverage.groups_available - batch.accepted.len();
    coverage.accepted_paths = batch
        .accepted
        .iter()
        .map(|h| h.candidate.path_id.clone())
        .collect();
    Ok(Plan { batch, coverage })
}

/// Recheck deterministic provenance when an archived test is read or resumed.
pub fn validate_origin(hypothesis: &h::Hypothesis) -> Result<()> {
    let row = &hypothesis.candidate;
    let (index, label) = one_label(row).ok_or("The object-label origin is ambiguous.")?;
    let mut expected = proposal(row, index, label);
    expected.alternatives = hypothesis.alternatives.iter().map(|r| r.id).collect();
    if expected != hypothesis.proposal
        || hypothesis.alternatives.iter().any(|r| {
            r.type_id != row.type_id
                || r.field_id != row.field_id
                || r.field_name != row.field_name
                || r.type_name != row.type_name
                || one_label(r).map(|(_, text)| text) != Some(label)
        })
    {
        return Err("The object-label question or its alternatives changed.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtime::{FieldRef, Scope, TextHint, TypeKey},
        runtime_inventory::*,
    };

    fn inventory(fields: usize, paths: usize) -> Inventory {
        Inventory {
            schema: 1,
            report_sha256: "a".repeat(64),
            scope: Scope {
                pid: 1,
                created: "0000000000000001".into(),
                session: 1,
            },
            types: (0..2)
                .map(|id| {
                    let key = TypeKey {
                        image: "observed".into(),
                        type_name: format!("Container{id}"),
                        layout_sha256: "b".repeat(64),
                    };
                    ObservedType {
                        id,
                        key: key.clone(),
                        fields: (0..fields)
                            .map(|field_id| ObservedField {
                                field_id,
                                name: format!("Coefficient{field_id}"),
                                type_code: 12,
                                values: vec![],
                                path_count: paths,
                                samples: (0..paths)
                                    .map(|path| PathSample {
                                        path_id: format!(
                                            "{:064x}",
                                            id * 10000 + field_id * 100 + path
                                        ),
                                        value: serde_json::json!("3.5"),
                                        context: vec![TextHint {
                                            field: FieldRef {
                                                declaring: key.clone(),
                                                field_id: 99,
                                            },
                                            text: "Speed".into(),
                                            truncated: false,
                                        }],
                                    })
                                    .collect(),
                            })
                            .collect(),
                    }
                })
                .collect(),
            reachable_paths: 2 * fields * paths,
            omitted_core_paths: 0,
            omitted_array_elements: 0,
            omitted_non_numeric_paths: 0,
            limitation: "fixture".into(),
        }
    }

    #[test]
    fn literal_questions_keep_competing_paths_and_source_through_test_provenance() {
        let inventory = inventory(1, 3);
        let selection = h::select(&inventory, &[0]).unwrap();
        let plan = plan(&inventory, &selection).unwrap();
        assert_eq!(plan.batch.accepted.len(), 1);
        let hypothesis = &plan.batch.accepted[0];
        assert_eq!(hypothesis.proposal.name, "Speed");
        assert_eq!(hypothesis.alternatives.len(), 2);
        assert_eq!(hypothesis.proposal.citations[0].source, "object_text");
        let reference = h::Reference {
            review_id: "00000000-0000-4000-8000-000000000001".into(),
            hypothesis_path_id: hypothesis.candidate.path_id.clone(),
            selected_path_id: hypothesis.alternatives[1].path_id.clone(),
        };
        let review = h::Review {
            question_policy: Some(POLICY.into()),
            question_source: h::QuestionSource::ObjectText,
            play_context: None,
            evidence_id: reference.review_id.clone(),
            report_sha256: inventory.report_sha256.clone(),
            model: "saved model".into(),
            status: "complete".into(),
            batch: Some(plan.batch),
            error: None,
        };
        let origin = h::resolve(&review, &reference, &inventory).unwrap();
        assert_eq!(origin.question_source, h::QuestionSource::ObjectText);
        assert_eq!(
            origin.selected().unwrap().path_id,
            reference.selected_path_id
        );
        let mut incomplete = inventory.clone();
        let mut hint = incomplete.types[0].fields[0].samples[0].context[0].clone();
        hint.truncated = true;
        incomplete.types[0].fields[0].samples[0].context.push(hint);
        // An extra incomplete hint is disclosed, but does not negate a complete clue.
        assert!(h::resolve(&review, &reference, &incomplete).is_ok());
        let mut unknown_policy = origin.clone();
        unknown_policy.question_policy = Some("unknown-future-policy".into());
        assert!(unknown_policy.selected().is_err());
        let mut saved: h::TestOrigin =
            serde_json::from_value(serde_json::to_value(&origin).unwrap()).unwrap();
        saved.hypothesis.proposal.question = "Does this certainly change player speed?".into();
        assert!(saved.selected().is_err());
        let mut legacy = origin;
        legacy.question_source = h::QuestionSource::Declared;
        legacy.question_policy = None;
        let serialized = serde_json::to_value(legacy).unwrap();
        assert!(serialized.get("question_source").is_none());
        assert!(serialized.get("question_policy").is_none());
        let restored: h::TestOrigin = serde_json::from_value(serialized).unwrap();
        assert_eq!(restored.question_source, h::QuestionSource::Declared);
    }

    #[test]
    fn ambiguous_or_generic_labels_do_not_become_questions_and_changed_evidence_fails() {
        let mut inventory = inventory(1, 4);
        let samples = &mut inventory.types[0].fields[0].samples;
        let mut other = samples[0].context[0].clone();
        other.text = "Energy".into();
        samples[0].context.push(other);
        samples[1].context[0].text = "value".into();
        samples[2].context.clear();
        let mut incomplete = samples[3].context[0].clone();
        incomplete.text = "Another unfinished label".into();
        incomplete.truncated = true;
        samples[3].context.push(incomplete);
        let mut selection = h::select(&inventory, &[0]).unwrap();
        let p = plan(&inventory, &selection).unwrap();
        assert_eq!(p.batch.accepted.len(), 1);
        assert_eq!(p.coverage.incomplete_context_rows, 1);
        assert_eq!(
            (
                p.coverage.ambiguous_rows,
                p.coverage.invalid_label_rows,
                p.coverage.rows_without_labels
            ),
            (1, 1, 1)
        );
        selection.rows[0].object_text = vec!["Speed".into()];
        assert!(plan(&inventory, &selection).is_err());
    }

    #[test]
    fn budget_is_shared_across_types_and_selection_order_does_not_choose_paths() {
        let inventory = inventory(8, 2);
        let a = plan(&inventory, &h::select(&inventory, &[0, 1]).unwrap()).unwrap();
        let b = plan(&inventory, &h::select(&inventory, &[1, 0]).unwrap()).unwrap();
        assert_eq!(a.coverage.accepted_paths, b.coverage.accepted_paths);
        assert_eq!(
            (a.coverage.groups_available, a.coverage.groups_deferred),
            (16, 4)
        );
        for id in [0, 1] {
            assert_eq!(
                a.batch
                    .accepted
                    .iter()
                    .filter(|h| h.candidate.type_id == id)
                    .count(),
                6
            );
        }
    }

    #[test]
    fn oversized_groups_are_deferred_whole_instead_of_dropping_alternatives() {
        let mut inventory = inventory(1, 3);
        let mut extra = inventory.types[0].fields[0].clone();
        for (i, sample) in extra.samples.iter_mut().enumerate() {
            sample.path_id = format!("{:064x}", 9000 + i);
        }
        inventory.types[0].fields.push(extra);
        let p = plan(&inventory, &h::select(&inventory, &[0]).unwrap()).unwrap();
        assert!(p.batch.accepted.is_empty());
        assert_eq!(
            (p.coverage.groups_available, p.coverage.groups_deferred),
            (1, 1)
        );
    }
}
