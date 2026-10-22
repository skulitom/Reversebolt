//! Materialize reviewed paths from retained discovery, without new target access.
use crate::{Result, runtime as r, runtime_hypotheses as h, runtime_inventory};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "runtime_cards_tests.rs"]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Publication {
    pub shown: Vec<String>,
    pub added: Vec<String>,
    pub omitted: Vec<String>,
    pub removed_preview: Vec<String>,
    pub revision: u64,
    pub card_generation: u64,
}

/// Include exact primary hypotheses in the bounded shared card list. Existing
/// selected/refreshed/protected cards keep their current readings, including
/// failed readings. A description never refreshes a value or authorizes a write.
pub fn include(
    view: &mut r::View,
    report: &Value,
    review: &h::Review,
    protected: &BTreeSet<String>,
) -> Result<Publication> {
    let source = view
        .catalog_source
        .as_ref()
        .ok_or("Discovery evidence unavailable.")?;
    let batch = review
        .batch
        .as_ref()
        .ok_or("A completed review is required.")?;
    if view.catalog_id.is_empty()
        || review.status != "complete"
        || review.error.is_some()
        || source.document_sha256 != review.report_sha256
        || batch.report_sha256 != review.report_sha256
        || batch.accepted.len() > 12
        || view.cards.len() > r::MAX_RUNTIME_CARDS
    {
        return Err(
            "Card suggestions belong to an incomplete or different catalogue review.".into(),
        );
    }
    let inventory = runtime_inventory::build(report)?;
    if inventory.report_sha256 != source.document_sha256 {
        return Err("Discovery evidence changed before card publication.".into());
    }
    let ids: Vec<_> = batch
        .accepted
        .iter()
        .map(|h| h.candidate.path_id.clone())
        .collect();
    let wanted: BTreeSet<_> = ids.iter().cloned().collect();
    if wanted.len() != ids.len() {
        return Err("A review repeats a primary candidate path.".into());
    }
    for id in &ids {
        h::resolve(
            review,
            &h::Reference {
                review_id: review.evidence_id.clone(),
                hypothesis_path_id: id.clone(),
                selected_path_id: id.clone(),
            },
            &inventory,
        )?;
    }
    let mut candidates = BTreeMap::new();
    if !ids.is_empty() {
        // Visit the complete validated graph once, not the first search page or
        // one traversal per hypothesis. Only selected paths are retained here.
        r::visit_candidates(report, "", 0, 1, &mut |c| {
            if wanted.contains(&c.id) {
                candidates.insert(c.id.clone(), c.clone());
            }
            Ok(())
        })?;
    }
    if candidates.len() != ids.len() {
        return Err("A reviewed path is absent from retained discovery.".into());
    }
    let previous: Vec<_> = view.cards.iter().map(|c| c.candidate.id.clone()).collect();
    if previous.iter().collect::<BTreeSet<_>>().len() != previous.len() {
        return Err("The card list repeats a candidate path.".into());
    }
    let mut cards: Vec<_> = view
        .cards
        .iter()
        .filter(|c| {
            protected.contains(&c.candidate.id)
                || view.selected_cards.contains(&c.candidate.id)
                || !c.readings.is_empty()
                || c.readings_dropped != 0
        })
        .cloned()
        .collect();
    for id in &ids {
        if cards.len() == r::MAX_RUNTIME_CARDS || cards.iter().any(|c| &c.candidate.id == id) {
            continue;
        }
        cards.push(
            view.cards
                .iter()
                .find(|c| &c.candidate.id == id)
                .cloned()
                .unwrap_or_else(|| {
                    let candidate = candidates
                        .remove(id)
                        .expect("all reviewed paths were resolved");
                    r::Card {
                        value: candidate.value.clone(),
                        candidate,
                        status: "read".into(),
                        observed_unix_ms: source.observed_unix_ms,
                        readings: vec![],
                        readings_dropped: 0,
                    }
                }),
        );
    }
    for card in &view.cards {
        if cards.len() == r::MAX_RUNTIME_CARDS {
            break;
        }
        if !cards.iter().any(|c| c.candidate.id == card.candidate.id) {
            cards.push(card.clone());
        }
    }
    let next: Vec<_> = cards.iter().map(|c| c.candidate.id.clone()).collect();
    let changed = previous != next;
    let revision = if changed {
        view.revision
            .checked_add(1)
            .ok_or("Runtime revision limit reached.")?
    } else {
        view.revision
    };
    let generation = if changed {
        view.card_generation
            .checked_add(1)
            .ok_or("Runtime card limit reached.")?
    } else {
        view.card_generation
    };
    let publication = Publication {
        shown: ids.iter().filter(|id| next.contains(id)).cloned().collect(),
        added: next
            .iter()
            .filter(|id| !previous.contains(id))
            .cloned()
            .collect(),
        omitted: ids
            .iter()
            .filter(|id| !next.contains(id))
            .cloned()
            .collect(),
        removed_preview: previous
            .iter()
            .filter(|id| !next.contains(id))
            .cloned()
            .collect(),
        revision,
        card_generation: generation,
    };
    view.cards = cards;
    view.revision = revision;
    view.card_generation = generation;
    Ok(publication)
}
