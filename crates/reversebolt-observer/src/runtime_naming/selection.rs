//! Page complete type rows rather than dropping labels or enlarging timeouts.
use super::*;
use reversebolt_core::runtime_type_index::Index;
use std::collections::BTreeSet;

pub const PAGE_PROMPT_BYTES: usize = 12_000;
const MAX_ROUNDS: usize = 4;
const MAX_STAGES: usize = 24;

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
    // None retains an attempted page whose model call/response did not complete.
    pub selected: Option<Vec<usize>>,
}

fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["types"],"properties":{
        "types":{"type":"array","maxItems":8,"uniqueItems":true,"items":{"type":"integer","minimum":0}}
    }})
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

fn selected_types(text: &str, page: &Index) -> Result<Vec<usize>> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Types {
        types: Vec<usize>,
    }
    let selected: Types = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let unique: BTreeSet<_> = selected.types.iter().copied().collect();
    if selected.types.len() > 8
        || unique.len() != selected.types.len()
        || unique
            .iter()
            .any(|id| !page.types.iter().any(|t| t.id == *id))
    {
        return Err(
            "Type selection must contain at most eight distinct IDs from its own page.".into(),
        );
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
        // Preserve the original one-request behavior and receipt for small catalogs.
        if format!("{prefix}{}", json!(index)).len() <= PAGE_PROMPT_BYTES {
            let stage = self
                .naming_stage(format!("{prefix}{}", json!(index)), schema(), 600, 49152)
                .await?;
            receipt.stages.push(stage);
            return selected_types(naming_text(receipt.stages.last().unwrap())?, &index);
        }
        receipt.paged_selection = Some(PagedSelection {
            policy: "complete_type_pages_v1".into(),
            prompt_bytes: PAGE_PROMPT_BYTES,
            rounds: vec![],
        });
        let budget = PAGE_PROMPT_BYTES
            .checked_sub(prefix.len())
            .ok_or("Type selection instructions exceed the page budget.")?;
        for _ in 0..MAX_ROUNDS {
            let partition = pages(&index, budget)?;
            receipt
                .paged_selection
                .as_mut()
                .unwrap()
                .rounds
                .push(SelectionRound { pages: vec![] });
            let mut nominated = BTreeSet::new();
            for page in &partition {
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
                        selected: None,
                    });
                let stage = self
                    .naming_stage(format!("{prefix}{}", json!(page)), schema(), 600, 16384)
                    .await?;
                receipt.stages.push(stage);
                let selected = selected_types(naming_text(receipt.stages.last().unwrap())?, page)?;
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
            assert!(selected_types(text, &page).is_err(), "{text}");
        }
        assert!(
            selected_types(
                &json!({"types":(0..9).map(|i|i*3).collect::<Vec<_>>()}).to_string(),
                &index()
            )
            .is_err()
        );
        assert_eq!(selected_types("{\"types\":[3,0]}", &page).unwrap(), [3, 0]);
        assert!(selected_types("{\"types\":[]}", &page).unwrap().is_empty());
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
