use super::*;
use serde_json::json;

fn fixture() -> (Value, r::View, h::Review, String) {
    let key = |name| json!({"image":"Fixture.dll","type_name":name,"layout_sha256":"a".repeat(64)});
    let report = json!({"schema":4,"pid":77,"created":"0123456789abcdef","session":2,"edit_authority":false,
        "classes":[
            {"id":0,"key":key("Roots"),"shape":"object","fields":[{"id":0,"name":"Current","reference":true,"type_code":18,"width":8,"flags":16}]},
            {"id":1,"key":key("Actor"),"shape":"object","fields":(0..80).map(|id| json!({"id":id,"name":format!("speed{id:03}"),"reference":false,"type_code":8,"width":4,"flags":0})).collect::<Vec<_>>()}],
        "snapshots":[{"snapshot_id":0,"nodes":[{"id":0,"class":1,"address":"0x1000"}],
            "roots":[{"class":0,"field":0,"read_ok":true,"node":{"id":0}}],
            "reads":(0..80).map(|id| json!({"node":0,"declaring_class":1,"field":id,"element_index":null,"type_code":8,"bytes":8i32.to_le_bytes(),"value":8,"reference":null,"read_ok":true})).collect::<Vec<_>>()}]});
    let inventory = runtime_inventory::build(&report).unwrap();
    let selected_types: Vec<_> = inventory.types.iter().map(|t| t.id).collect();
    let selection = h::select(&inventory, &selected_types).unwrap();
    let cards: Vec<_> = r::candidates(&report, r::MAX_RUNTIME_CARDS)
        .unwrap()
        .into_iter()
        .map(|candidate| r::Card {
            value: candidate.value.clone(),
            candidate,
            status: "read".into(),
            observed_unix_ms: 700,
            readings: vec![],
            readings_dropped: 0,
        })
        .collect();
    assert_eq!(cards.len(), 64);
    let outside = selection
        .rows
        .iter()
        .find(|r| !cards.iter().any(|c| c.candidate.id == r.path_id))
        .unwrap();
    let hypotheses = [&cards[0].candidate.id, &outside.path_id]
        .into_iter()
        .map(|id| {
            let row = selection.rows.iter().find(|r| &r.path_id == id).unwrap();
            h::Proposal {
                candidate: row.id,
                name: "Speed".into(),
                explanation: "May change movement speed; its effect is untested.".into(),
                citations: vec![h::Citation {
                    source: "field_name".into(),
                    index: None,
                    quote: row.field_name.clone(),
                }],
                alternatives: vec![],
                question: "Does this value change movement speed?".into(),
            }
        })
        .collect();
    let review = h::Review {
        question_source: Default::default(),
        question_policy: None,
        play_context: None,
        evidence_id: uuid::Uuid::new_v4().to_string(),
        report_sha256: inventory.report_sha256.clone(),
        model: "fixture".into(),
        status: "complete".into(),
        error: None,
        batch: Some(h::validate(&selection, h::Proposals { hypotheses }).unwrap()),
    };
    assert_eq!(review.batch.as_ref().unwrap().accepted.len(), 2);
    let view = r::View {
        catalog_id: uuid::Uuid::new_v4().to_string(),
        cards,
        revision: 4,
        card_generation: 3,
        catalog_source: Some(r::CatalogSource {
            path: "unused".into(),
            document_sha256: inventory.report_sha256,
            observed_unix_ms: 700,
        }),
        ..Default::default()
    };
    (report, view, review, outside.path_id.clone())
}

#[test]
fn explained_paths_outside_preview_appear_without_overwriting_readings_or_selected_controls() {
    let (report, mut view, review, outside) = fixture();
    view.cards[61].status = "unavailable".into();
    view.cards[61].value = Value::Null;
    view.cards[61].observed_unix_ms = 900;
    view.cards[61].readings.push(r::Reading {
        revision: 4,
        started_unix_ms: 900,
        completed_unix_ms: 901,
        status: "unavailable".into(),
        value: Value::Null,
        bytes: vec![],
        evidence_path: "unused".into(),
        report_sha256: "c".repeat(64),
    });
    view.selected_cards
        .push(view.cards[62].candidate.id.clone());
    let protected = BTreeSet::from([view.cards[63].candidate.id.clone()]);
    let originals: Vec<_> = view.cards[61..]
        .iter()
        .map(|c| (c.candidate.id.clone(), json!(c)))
        .collect();
    let result = include(&mut view, &report, &review, &protected).unwrap();
    assert_eq!(result.added.as_slice(), std::slice::from_ref(&outside));
    assert_eq!(result.shown.len(), 2);
    assert!(result.omitted.is_empty());
    assert_eq!(result.removed_preview.len(), 1);
    assert_eq!(view.cards.len(), 64);
    assert_eq!((view.revision, view.card_generation), (5, 4));
    for (id, original) in originals {
        assert_eq!(
            json!(view.cards.iter().find(|c| c.candidate.id == id).unwrap()),
            original
        );
        assert!(!result.removed_preview.contains(&id));
    }
    let added = view
        .cards
        .iter()
        .find(|c| c.candidate.id == outside)
        .unwrap();
    assert_eq!(added.observed_unix_ms, 700);
    assert!(added.readings.is_empty());
    assert!(!added.candidate.edit_authority);
    assert_eq!(
        json!(added.candidate),
        json!(r::candidate(&report, &outside).unwrap())
    );
    let saved = json!(view);
    let repeat = include(&mut view, &report, &review, &protected).unwrap();
    assert!(repeat.added.is_empty() && repeat.removed_preview.is_empty());
    assert_eq!(json!(view), saved);
}

#[test]
fn explicit_selection_and_full_capacity_are_preserved_and_reported() {
    let (report, mut view, review, outside) = fixture();
    view.selected_cards = view.cards.iter().map(|c| c.candidate.id.clone()).collect();
    let before = json!(view);
    let result = include(&mut view, &report, &review, &BTreeSet::new()).unwrap();
    assert_eq!(result.omitted.as_slice(), std::slice::from_ref(&outside));
    assert_eq!(json!(view), before);

    // Explicit Add controls goes through the same core selection used by MCP.
    view.search = Some(r::SearchPage {
        query: "speed".into(),
        offset: 0,
        matches: 1,
        candidates: vec![r::candidate(&report, &outside).unwrap()],
    });
    view.show_values(
        &view.catalog_id.clone(),
        view.revision,
        std::slice::from_ref(&outside),
    )
    .unwrap();
    assert!(view.selected_cards.contains(&outside));
    assert_eq!(view.selected_cards.len(), 64);
    let result = include(&mut view, &report, &review, &BTreeSet::new()).unwrap();
    assert!(result.omitted.is_empty());
    assert_eq!(view.cards[0].candidate.id, outside);
}

#[test]
fn changed_evidence_rejected_paths_and_revision_exhaustion_publish_nothing() {
    let (report, view, review, _) = fixture();
    for defect in 0..7 {
        let mut view = view.clone();
        let mut review = review.clone();
        let mut report = report.clone();
        match defect {
            0 => report["snapshots"][0]["reads"][0]["bytes"] = json!([1, 2, 3, 4]),
            1 => {
                review.batch.as_mut().unwrap().accepted[0]
                    .candidate
                    .field_name = "Other".into()
            }
            2 => {
                review.batch.as_mut().unwrap().accepted[0]
                    .proposal
                    .citations[0]
                    .quote = "Invented".into()
            }
            3 => review.status = "cancelled".into(),
            4 => review.report_sha256 = "c".repeat(64),
            5 => view.revision = u64::MAX,
            _ => view.card_generation = u64::MAX,
        }
        let before = json!(view);
        assert!(
            include(&mut view, &report, &review, &BTreeSet::new()).is_err(),
            "defect {defect}"
        );
        assert_eq!(json!(view), before);
    }
}
