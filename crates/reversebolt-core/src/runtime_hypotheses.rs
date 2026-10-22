//! Metadata-grounded questions, never identified gameplay controls or write authority.
use crate::{Result, runtime_inventory::Inventory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const LIMITATION: &str = "Unverified hypotheses from internal labels and same-object text. Citations bind the guess to observations; they do not prove its meaning, controlling state, scope, numeric bounds or edit authority.";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionSource {
    #[default]
    Declared,
    ObjectText,
}
impl QuestionSource {
    fn is_declared(&self) -> bool {
        *self == Self::Declared
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Review {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question_policy: Option<String>,
    #[serde(default, skip_serializing_if = "QuestionSource::is_declared")]
    pub question_source: QuestionSource,
    #[serde(default)]
    pub play_context: Option<crate::play_context::Context>,
    pub evidence_id: String,
    pub report_sha256: String,
    pub model: String,
    pub status: String,
    pub batch: Option<Batch>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActivityTransfer {
    pub retained: bool,
    pub from_review_id: Option<String>,
    pub reason: String,
}

/// Keep current observation evidence when an agent replaces metadata questions.
/// No capture, interpretation, target access or semantic promotion occurs here.
pub fn transfer_activity(
    previous: Option<&Review>,
    report_sha256: &str,
    investigation: &str,
    batch: usize,
    frames: &[crate::visual::FrameRef],
) -> (Option<crate::play_context::Context>, ActivityTransfer) {
    let transfer = || -> Result<crate::play_context::Context> {
        let previous = previous.ok_or("There is no previous review with activity evidence.")?;
        if previous.status != "complete"
            || previous.error.is_some()
            || previous.report_sha256 != report_sha256
            || previous
                .batch
                .as_ref()
                .is_none_or(|b| b.report_sha256 != report_sha256)
        {
            return Err(
                "The previous review is incomplete or belongs to another discovery report.".into(),
            );
        }
        let context = previous
            .play_context
            .as_ref()
            .ok_or("The previous review has no activity context.")?;
        context.validate()?;
        let frame_hash =
            crate::visual::digest(&serde_json::to_vec(frames).map_err(|e| e.to_string())?);
        if context.source.investigation_id != investigation
            || context.source.batch != batch
            || context.source.frame_metadata_sha256 != frame_hash
        {
            return Err(
                "The observed images, sampled input or investigation batch changed.".into(),
            );
        }
        if context.interpretation_source.is_some() {
            Ok(context.clone())
        } else {
            context
                .clone()
                .bind_interpretation(&previous.evidence_id, &previous.model)
        }
    };
    let (context, reason) = match transfer() {
        Ok(context) => (Some(context), "Retained unchanged activity evidence and its original interpretation source; this does not validate the new hypotheses.".into()),
        Err(reason) => (None, reason),
    };
    let outcome = ActivityTransfer {
        retained: context.is_some(),
        from_review_id: previous.map(|r| r.evidence_id.clone()),
        reason,
    };
    (context, outcome)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub id: usize,
    pub type_id: usize,
    pub field_id: usize,
    pub path_id: String,
    pub type_name: String,
    pub field_name: String,
    pub value: serde_json::Value,
    pub object_text: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Selection {
    pub report_sha256: String,
    pub rows: Vec<Row>,
    pub selected_types: Vec<usize>,
    pub omitted_samples: usize,
}

pub fn select(inventory: &Inventory, ids: &[usize]) -> Result<Selection> {
    if ids.len() > 8 || ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err("Select up to eight distinct observed types.".into());
    }
    let mut selected = Selection {
        report_sha256: inventory.report_sha256.clone(),
        rows: vec![],
        selected_types: ids.to_vec(),
        omitted_samples: 0,
    };
    let types = ids
        .iter()
        .map(|id| {
            inventory
                .types
                .iter()
                .find(|t| t.id == *id)
                .ok_or_else(|| "Unknown observed type.".to_string())
        })
        .collect::<Result<Vec<_>>>()?;
    let fields = types.iter().map(|t| t.fields.len()).max().unwrap_or(0);
    // Share the budget across types and fields before taking additional paths.
    for sample_index in 0..crate::runtime_inventory::MAX_PATH_SAMPLES {
        for field_index in 0..fields {
            for t in &types {
                let Some(f) = t.fields.get(field_index) else {
                    continue;
                };
                let Some(sample) = f.samples.get(sample_index) else {
                    continue;
                };
                if selected.rows.len() == 256 {
                    selected.omitted_samples += 1;
                    continue;
                }
                selected.rows.push(Row {
                    id: selected.rows.len(),
                    type_id: t.id,
                    field_id: f.field_id,
                    path_id: sample.path_id.clone(),
                    type_name: t.key.type_name.clone(),
                    field_name: f.name.clone(),
                    value: sample.value.clone(),
                    object_text: sample
                        .context
                        .iter()
                        .filter(|c| !c.truncated && c.text.len() <= 256)
                        .map(|c| c.text.clone())
                        .collect(),
                });
            }
        }
    }
    Ok(selected)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    pub source: String,
    /// Only object_text uses an index; field/type citations require null.
    pub index: Option<usize>,
    pub quote: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub candidate: usize,
    pub name: String,
    pub explanation: String,
    pub citations: Vec<Citation>,
    pub alternatives: Vec<usize>,
    pub question: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposals {
    pub hypotheses: Vec<Proposal>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hypothesis {
    pub proposal: Proposal,
    pub candidate: Row,
    /// Distinct possibilities to investigate, never a merged value/control.
    pub alternatives: Vec<Row>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rejected {
    pub proposal: Proposal,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Batch {
    pub report_sha256: String,
    pub selected_types: Vec<usize>,
    pub considered_samples: usize,
    pub omitted_samples: usize,
    pub accepted: Vec<Hypothesis>,
    pub rejected: Vec<Rejected>,
    pub limitation: String,
}

/// An explicit choice of one path within one retained, unverified hypothesis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub review_id: String,
    pub hypothesis_path_id: String,
    pub selected_path_id: String,
}

/// Immutable provenance retained with a comparison, including competing paths.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestOrigin {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question_policy: Option<String>,
    #[serde(default, skip_serializing_if = "QuestionSource::is_declared")]
    pub question_source: QuestionSource,
    #[serde(default)]
    pub play_context: Option<crate::play_context::Context>,
    pub reference: Reference,
    pub report_sha256: String,
    pub hypothesis: Hypothesis,
}
impl TestOrigin {
    pub fn selected(&self) -> Result<&Row> {
        if self.question_source == QuestionSource::ObjectText {
            if self.question_policy.as_deref() != Some(crate::runtime_object_questions::POLICY) {
                return Err("The object-text question policy is unsupported.".into());
            }
            crate::runtime_object_questions::validate_origin(&self.hypothesis)?;
        } else if self.question_policy.is_some() {
            return Err("A declared question cannot claim a deterministic policy.".into());
        }
        if let Some(context) = &self.play_context {
            context.validate()?;
        }
        let h = &self.hypothesis;
        if !uuid::Uuid::parse_str(&self.reference.review_id)
            .is_ok_and(|id| id.to_string() == self.reference.review_id)
            || self.report_sha256.len() != 64
            || !self.report_sha256.bytes().all(|c| c.is_ascii_hexdigit())
            || self.reference.hypothesis_path_id != h.candidate.path_id
            || h.proposal.candidate != h.candidate.id
            || h.proposal.alternatives != h.alternatives.iter().map(|r| r.id).collect::<Vec<_>>()
        {
            return Err("The hypothesis provenance changed.".into());
        }
        // Recheck the same lexical/citation contract without retaining unrelated rows.
        let mut rows = vec![h.candidate.clone()];
        rows.extend(h.alternatives.clone());
        if rows.iter().any(|r| {
            r.id >= 256
                || r.path_id.len() != 64
                || !r.path_id.bytes().all(|c| c.is_ascii_hexdigit())
        }) || rows.iter().map(|r| r.id).collect::<BTreeSet<_>>().len() != rows.len()
            || rows
                .iter()
                .map(|r| &r.path_id)
                .collect::<BTreeSet<_>>()
                .len()
                != rows.len()
        {
            return Err("Hypothesis paths or candidate identifiers are inconsistent.".into());
        }
        for (id, row) in rows.iter_mut().enumerate() {
            row.id = id;
        }
        let mut proposal = h.proposal.clone();
        proposal.candidate = 0;
        proposal.alternatives = (1..rows.len()).collect();
        check(
            &proposal,
            &Selection {
                report_sha256: self.report_sha256.clone(),
                rows,
                selected_types: vec![],
                omitted_samples: 0,
            },
        )?;
        std::iter::once(&h.candidate)
            .chain(&h.alternatives)
            .find(|r| r.path_id == self.reference.selected_path_id)
            .ok_or_else(|| {
                "Choose the hypothesis candidate or one of its retained alternatives.".into()
            })
    }
}

pub fn resolve(
    review: &Review,
    reference: &Reference,
    inventory: &Inventory,
) -> Result<TestOrigin> {
    let batch = review
        .batch
        .as_ref()
        .ok_or("No completed hypothesis review is available.")?;
    if review.status != "complete"
        || review.error.is_some()
        || review.evidence_id != reference.review_id
        || review.report_sha256 != inventory.report_sha256
        || batch.report_sha256 != inventory.report_sha256
    {
        return Err("The hypothesis belongs to a different or incomplete discovery review.".into());
    }
    let hypothesis = batch
        .accepted
        .iter()
        .find(|h| h.candidate.path_id == reference.hypothesis_path_id)
        .ok_or("Choose an accepted hypothesis from the current review.")?;
    let selection = select(inventory, &batch.selected_types)?;
    let checked = check(&hypothesis.proposal, &selection)?;
    if &checked != hypothesis {
        return Err("The hypothesis no longer matches the exact discovery evidence.".into());
    }
    if review.question_source == QuestionSource::ObjectText
        && !crate::runtime_object_questions::plan(inventory, &selection)?
            .batch
            .accepted
            .contains(&checked)
    {
        return Err("The question no longer follows the recorded object-label policy.".into());
    }
    let origin = TestOrigin {
        question_policy: review.question_policy.clone(),
        question_source: review.question_source,
        play_context: review.play_context.clone(),
        reference: reference.clone(),
        report_sha256: inventory.report_sha256.clone(),
        hypothesis: checked,
    };
    origin.selected()?;
    Ok(origin)
}

fn text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}

/// Extract words, including camelCase, but never interpret numbers as semantics.
fn words(value: &str) -> BTreeSet<String> {
    let mut separated = String::new();
    let mut lower = false;
    for c in value.chars() {
        if !c.is_alphabetic() || (c.is_uppercase() && lower) {
            separated.push(' ');
        }
        if c.is_alphabetic() {
            separated.extend(c.to_lowercase());
        }
        lower = c.is_lowercase();
    }
    separated
        .split_whitespace()
        .filter(|s| s.chars().count() >= 3)
        .map(str::to_owned)
        .collect()
}

fn check(proposal: &Proposal, selection: &Selection) -> Result<Hypothesis> {
    let row = selection
        .rows
        .get(proposal.candidate)
        .filter(|r| r.id == proposal.candidate)
        .ok_or("Candidate is absent from this exact evidence selection.")?;
    if !text(&proposal.name, 60)
        || !text(&proposal.explanation, 240)
        || !text(&proposal.question, 180)
        || !proposal.question.ends_with('?')
        || !proposal.explanation.ends_with(['.', '?'])
        || proposal.citations.is_empty()
        || proposal.citations.len() > 3
        || proposal.alternatives.len() > 4
        || proposal.alternatives.iter().collect::<BTreeSet<_>>().len()
            != proposal.alternatives.len()
    {
        return Err("Invalid or unfinished hypothesis text, citations or alternatives.".into());
    }
    let mut anchors = BTreeSet::new();
    for citation in &proposal.citations {
        let source = match (citation.source.as_str(), citation.index) {
            ("field_name", None) => Some(&row.field_name),
            ("type_name", None) => Some(&row.type_name),
            ("object_text", Some(index)) => row.object_text.get(index),
            _ => None,
        }
        .ok_or("Citation is not text from this candidate's observed object.")?;
        if !text(&citation.quote, 256) || citation.quote != *source {
            return Err(
                "Citation must quote the exact observed label, not another field or a substring."
                    .into(),
            );
        }
        // Hashed/opaque identifiers contribute no words, even when they contain hex words.
        let trimmed = source.trim_start_matches(['T', 'F', 't', 'f']);
        if !(trimmed.len() >= 8 && trimmed.bytes().all(|c| c.is_ascii_hexdigit())) {
            anchors.extend(words(source));
        }
    }
    let label = words(&proposal.name);
    if label.is_empty()
        || !label.is_subset(&anchors)
        || label.iter().all(|w| {
            [
                "value",
                "values",
                "field",
                "data",
                "setting",
                "state",
                "parameter",
                "number",
                "property",
            ]
            .contains(&w.as_str())
        })
    {
        return Err("The proposed name adds meaning absent from its cited labels or is only a generic placeholder.".into());
    }
    let alternatives = proposal
        .alternatives
        .iter()
        .map(|id| {
            selection
                .rows
                .get(*id)
                .filter(|r| r.id == *id && r.path_id != row.path_id)
                .cloned()
                .ok_or_else(|| "Alternative is missing or repeats the same path.".to_string())
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Hypothesis {
        proposal: proposal.clone(),
        candidate: row.clone(),
        alternatives,
    })
}

pub fn validate(selection: &Selection, proposals: Proposals) -> Result<Batch> {
    if proposals.hypotheses.len() > 12 {
        return Err("At most twelve metadata hypotheses per pass.".into());
    }
    let mut batch = Batch {
        report_sha256: selection.report_sha256.clone(),
        selected_types: selection.selected_types.clone(),
        considered_samples: selection.rows.len(),
        omitted_samples: selection.omitted_samples,
        accepted: vec![],
        rejected: vec![],
        limitation: LIMITATION.into(),
    };
    let mut paths = BTreeSet::new();
    for proposal in proposals.hypotheses {
        match check(&proposal, selection).and_then(|h| {
            if paths.insert(h.candidate.path_id.clone()) {
                Ok(h)
            } else {
                Err("Repeated candidate path in this pass.".into())
            }
        }) {
            Ok(h) => batch.accepted.push(h),
            Err(reason) => batch.rejected.push(Rejected { proposal, reason }),
        }
    }
    Ok(batch)
}

pub fn proposal_schema() -> serde_json::Value {
    serde_json::json!({"type":"object","additionalProperties":false,"required":["hypotheses"],"properties":{
        "hypotheses":{"type":"array","maxItems":12,"items":{"type":"object","additionalProperties":false,
            "required":["candidate","name","explanation","citations","alternatives","question"],"properties":{
                "candidate":{"type":"integer","minimum":0,"maximum":255},
                "name":{"type":"string","minLength":1,"maxLength":60},
                "explanation":{"type":"string","minLength":1,"maxLength":240},
                "question":{"type":"string","minLength":1,"maxLength":180},
                "alternatives":{"type":"array","maxItems":4,"uniqueItems":true,"items":{"type":"integer","minimum":0,"maximum":255}},
                "citations":{"type":"array","minItems":1,"maxItems":3,"items":{"type":"object","additionalProperties":false,
                    "required":["source","index","quote"],"properties":{
                        "source":{"enum":["field_name","type_name","object_text"]},
                        "index":{"type":["integer","null"],"minimum":0},"quote":{"type":"string","minLength":1,"maxLength":256}
                    }}}
            }}}
    }})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn selection() -> Selection {
        Selection {
            report_sha256: "a".repeat(64),
            selected_types: vec![3],
            omitted_samples: 0,
            rows: vec![
                Row {
                    id: 0,
                    type_id: 3,
                    field_id: 2,
                    path_id: "b".repeat(64),
                    type_name: "AgentSettings".into(),
                    field_name: "walkSpeed".into(),
                    value: serde_json::json!(8.2),
                    object_text: vec!["Speed".into()],
                },
                Row {
                    id: 1,
                    type_id: 3,
                    field_id: 4,
                    path_id: "c".repeat(64),
                    type_name: "AgentSettings".into(),
                    field_name: "moveSpeed".into(),
                    value: serde_json::json!(8.2),
                    object_text: vec![],
                },
            ],
        }
    }
    fn proposal() -> Proposal {
        Proposal {
            candidate: 0,
            name: "Walk speed".into(),
            explanation: "May relate to walking speed; its effect is untested.".into(),
            citations: vec![Citation {
                source: "field_name".into(),
                index: None,
                quote: "walkSpeed".into(),
            }],
            alternatives: vec![1],
            question: "Does changing this alter the observed walking response?".into(),
        }
    }
    #[test]
    fn exact_paths_and_alternatives_survive_without_becoming_controls() {
        let b = validate(
            &selection(),
            Proposals {
                hypotheses: vec![proposal()],
            },
        )
        .unwrap();
        assert_eq!(b.accepted.len(), 1);
        assert_eq!(b.accepted[0].candidate.field_id, 2);
        assert_eq!(b.accepted[0].alternatives[0].field_id, 4);
        assert_eq!(
            b.accepted[0].candidate.value,
            b.accepted[0].alternatives[0].value
        );
    }
    #[test]
    fn invented_semantics_wrong_references_and_unfinished_text_are_retained_as_rejections() {
        let mut variants = vec![];
        let mut p = proposal();
        p.name = "Damage".into();
        variants.push(p);
        let mut p = proposal();
        p.citations[0].quote = "moveSpeed".into();
        variants.push(p);
        let mut p = proposal();
        p.candidate = 7;
        variants.push(p);
        let mut p = proposal();
        p.alternatives = vec![0];
        variants.push(p);
        let mut p = proposal();
        p.explanation = "May change the".into();
        variants.push(p);
        let mut p = proposal();
        p.citations[0] = Citation {
            source: "object_text".into(),
            index: Some(1),
            quote: "Speed".into(),
        };
        variants.push(p);
        let b = validate(
            &selection(),
            Proposals {
                hypotheses: variants,
            },
        )
        .unwrap();
        assert!(b.accepted.is_empty());
        assert_eq!(b.rejected.len(), 6);
    }
    #[test]
    fn opaque_values_cannot_supply_the_name_but_same_object_text_can() {
        let mut s = selection();
        s.rows[0].field_name = "Fdeadbeef12".into();
        let mut p = proposal();
        p.citations[0].quote = s.rows[0].field_name.clone();
        assert_eq!(
            validate(
                &s,
                Proposals {
                    hypotheses: vec![p.clone()]
                }
            )
            .unwrap()
            .rejected
            .len(),
            1
        );
        p.name = "Speed".into();
        p.citations[0] = Citation {
            source: "object_text".into(),
            index: Some(0),
            quote: "Speed".into(),
        };
        assert_eq!(
            validate(
                &s,
                Proposals {
                    hypotheses: vec![p]
                }
            )
            .unwrap()
            .accepted
            .len(),
            1
        );
    }

    #[test]
    fn large_type_cannot_consume_all_evidence_before_other_types() {
        use crate::runtime_inventory::{ObservedField, ObservedType, PathSample};
        let key = crate::runtime::TypeKey {
            image: "observed".into(),
            type_name: "LargeType".into(),
            layout_sha256: "a".repeat(64),
        };
        let field = |id| ObservedField {
            field_id: id,
            name: format!("field{id}"),
            type_code: 8,
            values: vec![],
            path_count: 1,
            samples: vec![PathSample {
                path_id: format!("{id:064x}"),
                value: serde_json::json!(1),
                context: vec![],
            }],
        };
        let inventory = Inventory {
            schema: 1,
            report_sha256: "b".repeat(64),
            scope: crate::runtime::Scope {
                pid: 1,
                created: "0".repeat(16),
                session: 1,
            },
            types: vec![
                ObservedType {
                    id: 0,
                    key: key.clone(),
                    fields: (0..300).map(field).collect(),
                },
                ObservedType {
                    id: 1,
                    key,
                    fields: vec![field(1000)],
                },
            ],
            reachable_paths: 301,
            omitted_core_paths: 0,
            omitted_array_elements: 0,
            omitted_non_numeric_paths: 0,
            limitation: String::new(),
        };
        let selected = select(&inventory, &[0, 1]).unwrap();
        assert_eq!(selected.rows.len(), 256);
        assert_eq!(selected.omitted_samples, 45);
        assert_eq!(selected.rows[1].type_id, 1);
        assert!(select(&inventory, &[0, 0]).is_err());
        assert!(select(&inventory, &[2]).is_err());
    }
}
