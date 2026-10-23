//! Page complete type rows rather than dropping labels or enlarging timeouts.
use super::*;
use reversebolt_core::runtime_type_index::Index;
use std::collections::BTreeSet;

pub const PAGE_PROMPT_BYTES: usize = 12_000;
const MAX_ROUNDS: usize = 4;
const MAX_STAGES: usize = 24;
// The catalog is paged. Its aggregate budget must not equal one request's cap.
pub(super) const INDEX_BYTES: usize = PAGE_PROMPT_BYTES * MAX_STAGES;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PagedSelection {
    pub policy: String,
    pub prompt_bytes: usize,
    pub rounds: Vec<SelectionRound>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelectionRound {
    pub pages: Vec<SelectionPage>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelectionPage {
    pub type_ids: Vec<usize>,
    pub stage_index: usize,
    // Absent in historical receipts, whose pages all allowed eight nominations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_types: Option<usize>,
    // None retains an attempted page whose model call/response did not complete.
    pub selected: Option<Vec<usize>>,
}

fn schema(max_types: usize) -> Value {
    json!({"type":"object","additionalProperties":false,"required":["types"],"properties":{
        "types":{"type":"array","maxItems":max_types,"uniqueItems":true,"items":{"type":"integer","minimum":0}}
    }})
}

fn allowance(max_types: usize) -> String {
    format!(
        "This page permits at most {max_types} type IDs. This is a hard page allowance within the overall eight-type limit. Choose fewer or none when evidence is insufficient; do not fill slots merely to use the allowance.\n"
    )
}

fn page_quotas(counts: &[usize]) -> Vec<usize> {
    if counts.len() > 8 {
        // Screen every page. Small pages also need to narrow, unless they contain
        // just one row. Inseparable large singleton rows may still exhaust budgets.
        return counts.iter().map(|n| (*n / 2).clamp(1, 8)).collect();
    }
    // Once every page can have a slot, share eight slots evenly, redistributing
    // unused capacity from small pages. Ordering breaks ties, never field names.
    let mut quotas = vec![0; counts.len()];
    let mut remaining = counts.iter().sum::<usize>().min(8);
    while remaining > 0 {
        for (quota, count) in quotas.iter_mut().zip(counts) {
            if remaining > 0 && *quota < *count {
                *quota += 1;
                remaining -= 1;
            }
        }
    }
    quotas
}

fn pages(index: &Index, bytes: usize) -> Result<Vec<Index>> {
    let mut page = index.clone();
    page.types.clear();
    let mut result = vec![];
    if serde_json::to_vec(&page).map_err(|e| e.to_string())?.len() > bytes {
        return Err("Type selection page budget is too small.".into());
    }
    for row in &index.types {
        page.types.push(row.clone());
        if serde_json::to_vec(&page).map_err(|e| e.to_string())?.len() > bytes {
            page.types.pop();
            if !page.types.is_empty() {
                result.push(page.clone());
                page.types.clear();
            }
            page.types.push(row.clone());
            if serde_json::to_vec(&page).map_err(|e| e.to_string())?.len() > bytes {
                return Err("One observed type exceeds the selection page budget; its labels were not truncated.".into());
            }
        }
    }
    if !page.types.is_empty() || result.is_empty() {
        result.push(page);
    }
    Ok(result)
}

fn selected_types(text: &str, page: &Index, max_types: usize) -> Result<Vec<usize>> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Types {
        types: Vec<usize>,
    }
    let selected: Types = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let unique: BTreeSet<_> = selected.types.iter().copied().collect();
    if selected.types.len() > max_types
        || unique.len() != selected.types.len()
        || unique
            .iter()
            .any(|id| !page.types.iter().any(|t| t.id == *id))
    {
        return Err(format!(
            "Type selection must contain at most {max_types} distinct IDs from its own page."
        ));
    }
    Ok(selected.types)
}

fn narrow(mut index: Index, selected: &BTreeSet<usize>) -> Result<Index> {
    let before = index.types.len();
    index.types.retain(|t| selected.contains(&t.id));
    if index.types.len() >= before {
        return Err("Type selection did not narrow the catalog within its bounded review. No hypotheses were published.".into());
    }
    Ok(index)
}

impl Observer {
    pub(super) async fn select_types(
        &self,
        mut index: Index,
        prefix: &str,
        cancel: &AtomicBool,
        receipt: &mut Receipt,
    ) -> Result<Vec<usize>> {
        // Small catalogs still use one request; context sizing is shared with proposals.
        if format!("{prefix}{}", json!(index)).len() <= PAGE_PROMPT_BYTES {
            let stage_index = self
                .naming_stage(
                    format!("{prefix}{}", json!(index)),
                    schema(8),
                    600,
                    49152,
                    receipt,
                )
                .await?;
            return selected_types(naming_text(&receipt.stages[stage_index])?, &index, 8);
        }
        receipt.paged_selection = Some(PagedSelection {
            policy: "complete_type_pages_with_quotas_v2".into(),
            prompt_bytes: PAGE_PROMPT_BYTES,
            rounds: vec![],
        });
        let budget = PAGE_PROMPT_BYTES
            .checked_sub(prefix.len() + allowance(8).len())
            .ok_or("Type selection instructions exceed the page budget.")?;
        for _ in 0..MAX_ROUNDS {
            let partition = pages(&index, budget)?;
            let quotas = page_quotas(&partition.iter().map(|p| p.types.len()).collect::<Vec<_>>());
            if partition.len() > MAX_STAGES.saturating_sub(receipt.stages.len()) {
                return Err("Complete type pages exceed the remaining request budget.".into());
            }
            receipt
                .paged_selection
                .as_mut()
                .unwrap()
                .rounds
                .push(SelectionRound { pages: vec![] });
            let mut nominated = BTreeSet::new();
            for (page, max_types) in partition.iter().zip(quotas) {
                if cancel.load(Ordering::SeqCst) {
                    return Ok(vec![]);
                }
                if receipt.stages.len() >= MAX_STAGES {
                    return Err("Type selection exhausted its request budget.".into());
                }
                receipt
                    .paged_selection
                    .as_mut()
                    .unwrap()
                    .rounds
                    .last_mut()
                    .unwrap()
                    .pages
                    .push(SelectionPage {
                        type_ids: page.types.iter().map(|t| t.id).collect(),
                        stage_index: receipt.stages.len(),
                        max_types: Some(max_types),
                        selected: None,
                    });
                let stage_index = self
                    .naming_stage(
                        format!("{}{prefix}{}", allowance(max_types), json!(page)),
                        schema(max_types),
                        600,
                        16384,
                        receipt,
                    )
                    .await?;
                let selected =
                    selected_types(naming_text(&receipt.stages[stage_index])?, page, max_types)?;
                nominated.extend(selected.iter().copied());
                receipt
                    .paged_selection
                    .as_mut()
                    .unwrap()
                    .rounds
                    .last_mut()
                    .unwrap()
                    .pages
                    .last_mut()
                    .unwrap()
                    .selected = Some(selected);
            }
            if nominated.len() <= 8 {
                return Ok(nominated.into_iter().collect());
            }
            // Reconsider only model-nominated rows, with every original label intact.
            // No local field-name ranking or first-page truncation substitutes for it.
            index = narrow(index, &nominated)?;
        }
        Err("Type selection exhausted its review-round budget.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reversebolt_core::runtime_type_index::{ObjectSample, Type};

    fn index() -> Index {
        Index {
            types: (0..37)
                .map(|i| Type {
                    id: i * 3,
                    name: format!("Type{i}"),
                    name_shortened: false,
                    field_labels: vec!["complete \"quoted\" field 日本語".into(); 11],
                    object_samples: vec![ObjectSample {
                        field_name: "value".into(),
                        path_id: format!("{i:064x}"),
                        object_text: vec!["Same object".into()],
                    }],
                })
                .collect(),
            available_object_samples: 50,
            omitted_object_samples: 13,
        }
    }

    #[test]
    fn pages_keep_every_label_and_path_once_within_the_encoded_budget() {
        let index = index();
        let divided = pages(&index, 2000).unwrap();
        assert!(divided.len() > 1);
        assert_eq!(
            divided
                .iter()
                .flat_map(|p| p.types.clone())
                .collect::<Vec<_>>(),
            index.types
        );
        for page in divided {
            assert!(serde_json::to_vec(&page).unwrap().len() <= 2000);
            assert_eq!(page.available_object_samples, 50);
            assert_eq!(page.omitted_object_samples, 13);
        }
        let exact = serde_json::to_vec(&index).unwrap().len();
        assert_eq!(pages(&index, exact).unwrap(), vec![index]);
    }

    #[test]
    fn aggregate_catalog_can_exceed_one_prompt_without_enlarging_any_page() {
        let mut large = index();
        let prototype = large.types.clone();
        for copy in 1..7 {
            large.types.extend(prototype.iter().cloned().map(|mut row| {
                row.id += copy * 111;
                row
            }));
        }
        let encoded = serde_json::to_vec(&large).unwrap().len();
        assert!(encoded > PROMPT_BYTES && encoded < INDEX_BYTES);
        let prefix = format!("{SELECT_PROMPT}{OBJECT_TEXT_GUIDANCE}{INDEX_INTRO}");
        let prefix = format!("{}{prefix}", allowance(8));
        let divided = pages(&large, PAGE_PROMPT_BYTES - prefix.len()).unwrap();
        assert!(divided.len() < MAX_STAGES);
        assert_eq!(
            divided.iter().map(|p| p.types.len()).sum::<usize>(),
            large.types.len()
        );
        assert!(
            divided
                .iter()
                .all(|p| format!("{prefix}{}", json!(p)).len() <= PAGE_PROMPT_BYTES)
        );
    }

    #[test]
    fn oversized_type_is_rejected_without_discarding_or_truncating_labels() {
        let mut index = index();
        index.types[16].field_labels = vec!["x".repeat(4000)];
        assert!(pages(&index, 2000).unwrap_err().contains("not truncated"));
        assert_eq!(index.types[16].field_labels[0].len(), 4000);
        assert!(pages(&index, 1).is_err());
    }

    #[test]
    fn nominations_cannot_reference_another_page_duplicate_or_exceed_the_cap() {
        let page = pages(&index(), 2000).unwrap().remove(0);
        for text in [
            "{\"types\":[1]}",
            "{\"types\":[108]}",
            "{\"types\":[0,0]}",
            "{\"types\":[-1]}",
            "{\"types\":[],\"write\":true}",
        ] {
            assert!(selected_types(text, &page, 8).is_err(), "{text}");
        }
        assert!(
            selected_types(
                &json!({"types":(0..9).map(|i|i*3).collect::<Vec<_>>()}).to_string(),
                &index(),
                8
            )
            .is_err()
        );
        assert_eq!(
            selected_types("{\"types\":[3,0]}", &page, 8).unwrap(),
            [3, 0]
        );
        assert!(
            selected_types("{\"types\":[]}", &page, 8)
                .unwrap()
                .is_empty()
        );
        assert!(selected_types("{\"types\":[3,0]}", &page, 1).is_err());
        assert_eq!(selected_types("{\"types\":[3]}", &page, 1).unwrap(), [3]);
    }

    #[test]
    fn small_multi_page_pools_finish_even_when_every_allowance_is_filled() {
        for counts in [
            vec![6, 4],
            vec![6, 6],
            vec![10, 10, 10],
            vec![1, 29],
            vec![1; 8],
        ] {
            let quotas = page_quotas(&counts);
            assert_eq!(quotas.iter().sum::<usize>(), 8);
            assert!(quotas.iter().zip(&counts).all(|(q, n)| *q > 0 && q <= n));
            let index = index();
            let mut offset = 0;
            let mut selected = BTreeSet::new();
            for (count, quota) in counts.iter().zip(quotas) {
                let mut page = index.clone();
                page.types = index.types[offset..offset + count].to_vec();
                offset += count;
                let response = json!({"types":page.types.iter().rev().take(quota).map(|t|t.id).collect::<Vec<_>>()});
                selected.extend(selected_types(&response.to_string(), &page, quota).unwrap());
            }
            assert_eq!(selected.len(), 8);
        }
        assert_eq!(page_quotas(&[1, 29]), [1, 7]);
        assert_eq!(page_quotas(&[1, 1]), [1, 1]);
        assert!(page_quotas(&[]).is_empty());
    }

    #[test]
    fn screening_keeps_every_page_and_can_refuse_inseparable_singleton_rows() {
        let counts = [1, 2, 3, 4, 8, 15, 16, 28, 50];
        assert_eq!(page_quotas(&counts), [1, 1, 1, 2, 4, 7, 8, 8, 8]);
        let mut index = index();
        index.types.truncate(9);
        let all = index.types.iter().map(|t| t.id).collect();
        assert_eq!(page_quotas(&[1; 9]), [1; 9]);
        assert!(narrow(index, &all).is_err());
    }

    #[test]
    fn historical_page_receipts_remain_readable_without_a_quota() {
        let page: SelectionPage = serde_json::from_value(json!({
            "type_ids":[0,1], "stage_index":0, "selected":[1]
        }))
        .unwrap();
        assert!(page.max_types.is_none());
        assert!(
            serde_json::to_value(page)
                .unwrap()
                .get("max_types")
                .is_none()
        );
    }

    #[test]
    fn reductions_preserve_full_nominee_rows_and_cannot_repeat_without_progress() {
        let index = index();
        let all = index.types.iter().map(|t| t.id).collect();
        assert!(narrow(index.clone(), &all).is_err());
        let wanted = [3, 18, 105].into_iter().collect();
        let reduced = narrow(index.clone(), &wanted).unwrap();
        assert_eq!(
            reduced.types,
            [
                index.types[1].clone(),
                index.types[6].clone(),
                index.types[35].clone()
            ]
        );
    }
}
