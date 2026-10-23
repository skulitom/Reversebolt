//! Research-only classification of raw labels. Never a semantic verifier or gate.
use crate::{Result, runtime_hypotheses as h, visual};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub const POLICY: &str = "raw_label_relevance_v1";
pub const LIMITATION: &str = "Fallible classification of labels and same-object text, independent of the generated question. Exact citations validate provenance, not relevance, semantics, ownership, liveness or effects. Unknown candidates are retained. No candidate is removed and no edit is authorized by this research assessment.";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub report_sha256: String,
    pub candidates: Vec<h::Row>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Gameplay,
    ApplicationConfiguration,
    ImplementationDetail,
    Unclear,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Judgment {
    /// Index in this exact input, not a runtime address or the source row ID.
    pub candidate: usize,
    pub category: Category,
    pub reason: String,
    pub citations: Vec<h::Citation>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub judgments: Vec<Judgment>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub policy: String,
    pub input_sha256: String,
    pub judgments: Vec<Judgment>,
}
impl Input {
    pub fn from_batch(selection: &h::Selection, batch: &h::Batch) -> Result<Self> {
        if batch.report_sha256 != selection.report_sha256
            || batch.selected_types != selection.selected_types
        {
            return Err("Relevance evidence belongs to a different selection.".into());
        }
        let checked = h::validate(
            selection,
            h::Proposals {
                hypotheses: batch.accepted.iter().map(|h| h.proposal.clone()).collect(),
            },
        )?;
        if checked.accepted != batch.accepted || !checked.rejected.is_empty() {
            return Err("Relevance evidence differs from the validated source rows.".into());
        }
        let input = Self {
            report_sha256: batch.report_sha256.clone(),
            candidates: batch.accepted.iter().map(|h| h.candidate.clone()).collect(),
        };
        input.validate()?;
        Ok(input)
    }
    pub fn validate(&self) -> Result<()> {
        let hash = |s: &str| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit());
        if !hash(&self.report_sha256)
            || self.candidates.len() > 12
            || self
                .candidates
                .iter()
                .any(|r| !hash(&r.path_id) || r.id >= 256)
            || self
                .candidates
                .iter()
                .map(|r| &r.path_id)
                .collect::<BTreeSet<_>>()
                .len()
                != self.candidates.len()
            || self
                .candidates
                .iter()
                .map(|r| r.id)
                .collect::<BTreeSet<_>>()
                .len()
                != self.candidates.len()
        {
            return Err("Invalid relevance evidence scope, size or duplicate candidate.".into());
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        Ok(visual::digest(
            &serde_json::to_vec(self).map_err(|e| e.to_string())?,
        ))
    }
    pub fn prompt_data(&self) -> Result<Value> {
        self.validate()?;
        // Hide names/questions/explanations, values and opaque path identifiers.
        // Complete nearby text remains attached only to its original candidate.
        Ok(json!(self.candidates.iter().enumerate().map(|(i, r)| json!({
            "candidate":i,"type_name":r.type_name,"field_name":r.field_name,"object_text":r.object_text
        })).collect::<Vec<_>>()))
    }
}
pub fn accept(input: &Input, mut proposed: Proposal) -> Result<Assessment> {
    input.validate()?;
    let ids: BTreeSet<_> = proposed.judgments.iter().map(|j| j.candidate).collect();
    if proposed.judgments.len() != input.candidates.len()
        || ids != (0..input.candidates.len()).collect()
    {
        return Err(
            "Classify every supplied candidate exactly once; no extra or missing IDs.".into(),
        );
    }
    for judgment in &proposed.judgments {
        let row = &input.candidates[judgment.candidate];
        if judgment.reason.trim().is_empty()
            || judgment.reason.len() > 400
            || judgment.reason.chars().any(char::is_control)
            || judgment.citations.len() > 3
            || (judgment.category != Category::Unclear && judgment.citations.is_empty())
        {
            return Err(
                "A relevance judgment needs a bounded reason and exact observed citations.".into(),
            );
        }
        for citation in &judgment.citations {
            let observed = match (citation.source.as_str(), citation.index) {
                ("field_name", None) => Some(&row.field_name),
                ("type_name", None) => Some(&row.type_name),
                ("object_text", Some(i)) => row.object_text.get(i),
                _ => None,
            };
            if observed != Some(&citation.quote) || citation.quote.trim().is_empty() {
                return Err(
                    "Relevance citation must exactly match this candidate's own evidence.".into(),
                );
            }
        }
    }
    proposed.judgments.sort_by_key(|j| j.candidate);
    Ok(Assessment {
        policy: POLICY.into(),
        input_sha256: input.digest()?,
        judgments: proposed.judgments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Input {
        Input {
            report_sha256: "a".repeat(64),
            candidates: (0..2)
                .map(|i| h::Row {
                    id: i + 5,
                    type_id: 0,
                    field_id: i,
                    path_id: format!("{i:064x}"),
                    type_name: "UnknownContainer".into(),
                    field_name: format!("field{i}"),
                    value: json!(777),
                    object_text: vec![format!("Label{i}")],
                })
                .collect(),
        }
    }
    fn proposal() -> Proposal {
        Proposal {
            judgments: (0..2)
                .map(|i| Judgment {
                    candidate: i,
                    category: Category::Unclear,
                    reason: "The labels do not establish its role.".into(),
                    citations: vec![],
                })
                .collect(),
        }
    }
    #[test]
    fn every_candidate_is_accounted_for_and_unknown_is_not_rejection() {
        let input = input();
        let before = input.clone();
        let mut proposed = proposal();
        proposed.judgments.reverse();
        let accepted = accept(&input, proposed).unwrap();
        assert_eq!(accepted.judgments.len(), 2);
        assert_eq!(accepted.judgments[0].candidate, 0);
        assert_eq!(input, before);
        for ids in [vec![0], vec![0, 0], vec![0, 2], vec![0, 1, 2]] {
            let mut proposed = proposal();
            proposed.judgments = ids
                .into_iter()
                .map(|id| {
                    let mut j = proposed.judgments[0].clone();
                    j.candidate = id;
                    j
                })
                .collect();
            assert!(accept(&input, proposed).is_err());
        }
    }
    #[test]
    fn a_citation_cannot_borrow_another_object_or_use_a_substring() {
        let input = input();
        for quote in ["Label1", "Label", "field1"] {
            let mut p = proposal();
            p.judgments[0].category = Category::Gameplay;
            p.judgments[0].citations = vec![h::Citation {
                source: "object_text".into(),
                index: Some(0),
                quote: quote.into(),
            }];
            assert!(accept(&input, p).is_err());
        }
        let mut p = proposal();
        p.judgments[0].category = Category::Gameplay;
        assert!(accept(&input, p.clone()).is_err());
        p.judgments[0].citations = vec![h::Citation {
            source: "object_text".into(),
            index: Some(0),
            quote: "Label0".into(),
        }];
        assert!(accept(&input, p).is_ok());
    }
    #[test]
    fn prompt_excludes_values_and_path_ids_while_digest_binds_them() {
        let mut input = input();
        let prompt = input.prompt_data().unwrap();
        let old = input.digest().unwrap();
        assert!(prompt[0].get("value").is_none() && prompt[0].get("path_id").is_none());
        assert_eq!(prompt[0]["object_text"], json!(["Label0"]));
        input.candidates[0].value = json!(99);
        assert_eq!(input.prompt_data().unwrap(), prompt);
        assert_ne!(input.digest().unwrap(), old);
        input.candidates[1].path_id = input.candidates[0].path_id.clone();
        assert!(input.validate().is_err());
    }
    #[test]
    fn malformed_classification_or_extra_authority_is_not_an_assessment() {
        for value in [
            json!({"judgments":[],"write":true}),
            json!({"judgments":[{"candidate":0,"category":"verified","reason":"Yes","citations":[]}]}),
        ] {
            assert!(serde_json::from_value::<Proposal>(value).is_err());
        }
        let input = Input {
            report_sha256: "a".repeat(64),
            candidates: vec![],
        };
        assert!(
            accept(&input, Proposal { judgments: vec![] })
                .unwrap()
                .judgments
                .is_empty()
        );
    }
}
