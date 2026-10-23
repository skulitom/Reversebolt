//! Explicit references to exact supplied evidence, never inferred citation repair.
use crate::{Result, runtime_hypotheses as h, visual};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub const POLICY: &str = "candidate_evidence_references_v1";
const MAX_ENTRIES: usize = 8192;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: usize,
    pub candidate: usize,
    pub citation: h::Citation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    pub policy: String,
    pub report_sha256: String,
    pub rows_sha256: String,
    pub entries: Vec<Entry>,
}
impl Table {
    pub fn build(report: &str, rows: &[h::Row]) -> Result<Self> {
        let hash = |s: &str| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit());
        if !hash(report)
            || rows.len() > 256
            || rows.iter().any(|r| r.id >= 256 || !hash(&r.path_id))
            || rows.iter().map(|r| r.id).collect::<BTreeSet<_>>().len() != rows.len()
            || rows
                .iter()
                .map(|r| &r.path_id)
                .collect::<BTreeSet<_>>()
                .len()
                != rows.len()
        {
            return Err("Invalid citation evidence scope, size or repeated candidate.".into());
        }
        let mut table = Self {
            policy: POLICY.into(),
            report_sha256: report.into(),
            rows_sha256: visual::digest(&serde_json::to_vec(rows).map_err(|e| e.to_string())?),
            entries: vec![],
        };
        for row in rows {
            let mut append = |source: &str, index: Option<usize>, quote: &str| -> Result<()> {
                if table.entries.len() == MAX_ENTRIES {
                    return Err(
                        "Citation evidence exceeds its entry budget; no labels were truncated."
                            .into(),
                    );
                }
                table.entries.push(Entry {
                    id: table.entries.len(),
                    candidate: row.id,
                    citation: h::Citation {
                        source: source.into(),
                        index,
                        quote: quote.into(),
                    },
                });
                Ok(())
            };
            append("field_name", None, &row.field_name)?;
            append("type_name", None, &row.type_name)?;
            for (index, text) in row.object_text.iter().enumerate() {
                append("object_text", Some(index), text)?;
            }
        }
        if serde_json::to_vec(&table).map_err(|e| e.to_string())?.len() > 2 * 1024 * 1024 {
            return Err(
                "Citation evidence exceeds its byte budget; no labels were truncated.".into(),
            );
        }
        Ok(table)
    }
    pub fn check(&self, report: &str, rows: &[h::Row]) -> Result<()> {
        if self != &Self::build(report, rows)? {
            return Err("Citation evidence table differs from its exact source rows.".into());
        }
        Ok(())
    }
    pub fn prompt_row(&self, candidate: usize) -> Result<Value> {
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.candidate == candidate)
            .collect();
        if entries.len() < 2 {
            return Err("Candidate is absent from the citation evidence table.".into());
        }
        let text = |entry: &Entry| json!({"id":entry.id,"text":entry.citation.quote});
        Ok(
            json!({"candidate":candidate,"field_name":text(entries[0]),"type_name":text(entries[1]),
            "object_text":entries[2..].iter().map(|e|text(e)).collect::<Vec<_>>()}),
        )
    }
    pub fn resolve(
        &self,
        report: &str,
        rows: &[h::Row],
        candidate: usize,
        ids: &[usize],
    ) -> Result<Vec<h::Citation>> {
        self.check(report, rows)?;
        if ids.len() > 3
            || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
            || !rows.iter().any(|r| r.id == candidate)
        {
            return Err(
                "Evidence references require an observed candidate and at most three distinct IDs."
                    .into(),
            );
        }
        ids.iter()
            .map(|id| {
                self.entries
                    .get(*id)
                    .filter(|e| e.id == *id && e.candidate == candidate)
                    .map(|e| e.citation.clone())
                    .ok_or_else(|| {
                        "Evidence reference is absent or belongs to another candidate.".into()
                    })
            })
            .collect()
    }
}

pub fn references_schema(minimum: usize) -> Value {
    json!({"type":"array","minItems":minimum,"maxItems":3,"uniqueItems":true,
        "items":{"type":"integer","minimum":0,"maximum":MAX_ENTRIES-1}})
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub candidate: usize,
    pub name: String,
    pub explanation: String,
    pub evidence_ids: Vec<usize>,
    pub alternatives: Vec<usize>,
    pub question: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposals {
    pub hypotheses: Vec<Proposal>,
}
pub fn proposal_schema() -> Value {
    let mut schema = h::proposal_schema();
    let item = &mut schema["properties"]["hypotheses"]["items"];
    let properties = item["properties"].as_object_mut().unwrap();
    properties.remove("citations");
    properties.insert("evidence_ids".into(), references_schema(1));
    for key in item["required"].as_array_mut().unwrap() {
        if key == "citations" {
            *key = json!("evidence_ids");
        }
    }
    schema
}
/// Express the already enforced group boundary in the generation grammar.
pub fn group_schema(candidates: &[usize], quota: usize) -> Result<Value> {
    if candidates.is_empty()
        || candidates.len() > 256
        || candidates.iter().any(|id| *id >= 256)
        || candidates.iter().collect::<BTreeSet<_>>().len() != candidates.len()
        || !(1..=12).contains(&quota)
    {
        return Err("Invalid proposal group or question quota.".into());
    }
    let mut schema = proposal_schema();
    let hypotheses = &mut schema["properties"]["hypotheses"];
    hypotheses["maxItems"] = json!(quota);
    let properties = &mut hypotheses["items"]["properties"];
    properties["candidate"]["enum"] = json!(candidates);
    properties["alternatives"]["items"]["enum"] = json!(candidates);
    Ok(schema)
}
impl Proposals {
    pub fn resolve(self, table: &Table, selection: &h::Selection) -> Result<h::Proposals> {
        let hypotheses = self
            .hypotheses
            .into_iter()
            .map(|p| {
                if p.evidence_ids.is_empty() {
                    return Err(
                        "A named hypothesis requires at least one evidence reference.".into(),
                    );
                }
                Ok(h::Proposal {
                    citations: table.resolve(
                        &selection.report_sha256,
                        &selection.rows,
                        p.candidate,
                        &p.evidence_ids,
                    )?,
                    candidate: p.candidate,
                    name: p.name,
                    explanation: p.explanation,
                    alternatives: p.alternatives,
                    question: p.question,
                })
            })
            .collect::<Result<_>>()?;
        Ok(h::Proposals { hypotheses })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows() -> Vec<h::Row> {
        (0..2)
            .map(|id| h::Row {
                id,
                type_id: 0,
                field_id: id,
                path_id: format!("{id:064x}"),
                type_name: "ActorConfig".into(),
                field_name: format!("coefficient{id}"),
                value: json!(5),
                object_text: vec!["Speed".into(), "Speed".into()],
            })
            .collect()
    }
    #[test]
    fn references_preserve_the_exact_source_even_for_repeated_text() {
        let rows = rows();
        let table = Table::build(&"a".repeat(64), &rows).unwrap();
        let citations = table
            .resolve(&"a".repeat(64), &rows, 0, &[0, 2, 3])
            .unwrap();
        assert_eq!(citations[0].source, "field_name");
        assert_eq!(citations[1].source, "object_text");
        assert_eq!(citations[1].index, Some(0));
        assert_eq!(citations[2].index, Some(1));
        assert_eq!(citations[1].quote, citations[2].quote);
        assert_eq!(
            table.prompt_row(0).unwrap()["object_text"][0]["text"],
            "Speed"
        );
    }
    #[test]
    fn foreign_missing_duplicate_and_tampered_references_cannot_resolve() {
        let mut rows = rows();
        let table = Table::build(&"a".repeat(64), &rows).unwrap();
        for ids in [vec![6], vec![100], vec![2, 2], vec![0, 1, 2, 3]] {
            assert!(table.resolve(&"a".repeat(64), &rows, 0, &ids).is_err());
        }
        assert!(table.resolve(&"b".repeat(64), &rows, 0, &[2]).is_err());
        let mut forged = table.clone();
        forged.entries[2].citation.quote = "Health".into();
        assert!(forged.resolve(&"a".repeat(64), &rows, 0, &[2]).is_err());
        rows[0].value = json!(6);
        assert!(table.resolve(&"a".repeat(64), &rows, 0, &[2]).is_err());
    }
    #[test]
    fn resolving_a_reference_does_not_validate_an_invented_name() {
        let rows = rows();
        let report = "a".repeat(64);
        let table = Table::build(&report, &rows).unwrap();
        let selection = h::Selection {
            report_sha256: report,
            rows,
            selected_types: vec![0],
            omitted_samples: 0,
        };
        let make = |name: &str| Proposals {
            hypotheses: vec![Proposal {
                candidate: 0,
                name: name.into(),
                explanation: "The effect requires testing.".into(),
                evidence_ids: vec![2],
                alternatives: vec![],
                question: "Does this affect speed?".into(),
            }],
        };
        assert_eq!(
            h::validate(
                &selection,
                make("Speed").resolve(&table, &selection).unwrap()
            )
            .unwrap()
            .accepted
            .len(),
            1
        );
        assert_eq!(
            h::validate(
                &selection,
                make("Health").resolve(&table, &selection).unwrap()
            )
            .unwrap()
            .rejected
            .len(),
            1
        );
        assert!(
            serde_json::from_value::<Proposals>(json!({"hypotheses":[],"write":true})).is_err()
        );
    }
    #[test]
    fn group_contract_scopes_candidate_and_alternative_ids_without_relaxing_evidence() {
        let base = proposal_schema();
        let grouped = group_schema(&[2, 9, 16], 2).unwrap();
        let item = &grouped["properties"]["hypotheses"]["items"]["properties"];
        assert_eq!(item["candidate"]["enum"], json!([2, 9, 16]));
        assert_eq!(item["alternatives"]["items"]["enum"], json!([2, 9, 16]));
        assert_eq!(
            item["evidence_ids"],
            base["properties"]["hypotheses"]["items"]["properties"]["evidence_ids"]
        );
        for (ids, quota) in [
            (vec![], 1),
            (vec![2, 2], 1),
            (vec![256], 1),
            (vec![2], 0),
            (vec![2], 13),
        ] {
            assert!(group_schema(&ids, quota).is_err());
        }
    }
    #[test]
    fn oversize_and_duplicate_source_rows_fail_without_truncating_evidence() {
        let mut duplicate = rows();
        duplicate[1].path_id = duplicate[0].path_id.clone();
        assert!(Table::build(&"a".repeat(64), &duplicate).is_err());
        let mut excessive = rows();
        excessive[0].object_text = vec!["x".into(); MAX_ENTRIES];
        assert!(
            Table::build(&"a".repeat(64), &excessive)
                .unwrap_err()
                .contains("entry budget")
        );
    }
}
