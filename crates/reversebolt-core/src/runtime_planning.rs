//! Suggestions from retained evidence. No input, writes or semantic promotion.
use crate::{
    Result,
    action_patterns::Exposure,
    play_context::Context,
    play_evidence::Evidence,
    runtime_hypotheses::{self as h, Reference, Review},
    runtime_inventory::Inventory,
    runtime_tests::{Action, InputWitness, Test},
    visual::{self, FrameRef},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const LIMITATION: &str = "A proposed comparison, not learned button meaning or verified gameplay semantics. Timing is estimated from sampled transitions. Repeating the observed sequence can move or otherwise change the game; restoration concerns the selected bytes, not world rollback. Different paths may alias or change occupants; sharing a declared field does not establish the same object. A question's name describes its primary hypothesis, not the meaning of a selected alternative. Outcomes describe only the tested path, action sequence and context.";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Episode {
    pub before: String,
    pub after: String,
    pub input_sha256: String,
    pub hold_bounds_us: [u64; 2],
    pub action: Action,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObservedActions {
    pub frame_metadata_sha256: String,
    pub episodes: Vec<Episode>,
    pub actions: Vec<Action>,
    /// When present, indexes the observed episodes used for each planned action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode_order: Option<Vec<usize>>,
}

pub fn actions(
    context: &Context,
    frames: &[FrameRef],
    evidence: &Evidence,
) -> Result<ObservedActions> {
    context.validate()?;
    let hash = visual::digest(&serde_json::to_vec(frames).map_err(|e| e.to_string())?);
    let regional =
        context.source.selection_basis == crate::play_context::SelectionBasis::RegionalCandidateV1;
    let source_matches = if regional {
        crate::play_context::regional_candidates(
            frames,
            evidence,
            &context.source.investigation_id,
            context.source.batch,
        )?
        .contains(&context.source)
    } else {
        crate::play_context::select(
            frames,
            evidence,
            &context.source.investigation_id,
            context.source.batch,
        )?
        .as_ref()
            == Some(&context.source)
    };
    if hash != context.source.frame_metadata_sha256 || !source_matches {
        return Err("The retained activity evidence changed.".into());
    }
    let mut episodes = vec![];
    for (index, e) in evidence.episodes.iter().enumerate() {
        if regional && !context.source.pairs.iter().any(|p| p.episode == index) {
            continue;
        }
        let Exposure::Key(key) = e.exposure else {
            continue;
        };
        if !matches!(key,32|37..=40|48..=57|65..=90)
            || !e.bounded_by_quiet
            || e.first_interval != e.last_interval
        {
            continue;
        }
        let Some(quiet) = evidence
            .actions
            .iter()
            .find(|a| a.key == key)
            .and_then(|a| a.quiet_median_difference)
        else {
            continue;
        };
        if !regional
            && e.median_difference
                .is_none_or(|d| !d.is_finite() || d < 5f64.max(quiet * 5.))
        {
            continue;
        }
        let Some(a) = frames.get(e.first_interval) else {
            continue;
        };
        let Some(b) = frames.get(e.first_interval + 1) else {
            continue;
        };
        let (Some(ta), Some(tb), Some(w)) =
            (&a.capture_interval, &b.capture_interval, &b.input_context)
        else {
            continue;
        };
        let pressed: Vec<_> = w
            .polls
            .iter()
            .enumerate()
            .filter(|(_, p)| p.down == [key])
            .collect();
        let (Some((first, down)), Some((last, up))) = (pressed.first(), pressed.last()) else {
            continue;
        };
        if *first == 0 || *last + 1 >= w.polls.len() {
            continue;
        }
        // The key query happens somewhere inside each poll, so only the gap
        // between the first poll's end and the last poll's start is certain.
        let lower = up.begin_us.saturating_sub(down.end_us);
        let upper = w.polls[*last + 1]
            .end_us
            .saturating_sub(w.polls[*first - 1].begin_us);
        if upper < lower || upper - lower > 100_000 {
            continue;
        }
        let Some(midpoint) = lower
            .checked_add((upper - lower) / 2)
            .and_then(|n| n.checked_add(500))
        else {
            continue;
        };
        let ms = midpoint / 1000;
        if !(100..=1000).contains(&ms) {
            continue;
        }
        let action = Action {
            keys: vec![key],
            hold_ms: ms as u32,
        };
        if !InputWitness::check(Some(w), Some(&action), tb, ta.end_us).matches {
            continue;
        }
        episodes.push(Episode {
            before: a.id.clone(),
            after: b.id.clone(),
            input_sha256: visual::digest(
                &serde_json::to_vec(&b.input_context).map_err(|e| e.to_string())?,
            ),
            hold_bounds_us: [lower, upper],
            action,
        });
        if episodes.len() == if regional { 2 } else { 4 } {
            break;
        }
    }
    if regional {
        if episodes.len() != 2
            || episodes
                .iter()
                .any(|e| e.action.keys != [context.source.key])
            || episodes[0]
                .action
                .hold_ms
                .abs_diff(episodes[1].action.hold_ms)
                > 50
        {
            return Err("Both locally measured responses need bounded, consistent sampled holds before proposing repetition.".into());
        }
        return Ok(ObservedActions {
            frame_metadata_sha256: hash,
            actions: [0, 1, 0, 1].map(|i| episodes[i].action.clone()).into(),
            episodes,
            episode_order: Some(vec![0, 1, 0, 1]),
        });
    }
    if episodes.len() != 4
        || !episodes
            .iter()
            .any(|e| e.action.keys == [context.source.key])
        || (0..2).any(|i| {
            episodes[i].action.keys != episodes[i + 2].action.keys
                || episodes[i]
                    .action
                    .hold_ms
                    .abs_diff(episodes[i + 2].action.hold_ms)
                    > 50
        })
    {
        return Err("Observe four separated responses with a repeated two-action sequence and consistent timing first.".into());
    }
    Ok(ObservedActions {
        frame_metadata_sha256: hash,
        actions: episodes.iter().map(|e| e.action.clone()).collect(),
        episodes,
        episode_order: None,
    })
}

pub fn choices(
    review: &Review,
    inventory: &Inventory,
    actions: &ObservedActions,
    tests: &[Test],
    investigation: &str,
    catalog: &str,
) -> Result<Vec<Value>> {
    let batch = review
        .batch
        .as_ref()
        .ok_or("No completed hypotheses are available.")?;
    let mut choices: Vec<Value> = vec![];
    // Prefer a path's own hypothesis when it also appears as an alternative to
    // another question. Retain all question references, but propose a path once.
    let candidates = batch.accepted.iter().map(|h| (h, &h.candidate, "primary"));
    let alternatives = batch.accepted.iter().flat_map(|h| {
        h.alternatives
            .iter()
            .map(move |row| (h, row, "alternative"))
    });
    for (hypothesis, row, role) in candidates.chain(alternatives) {
        let reference = Reference {
            review_id: review.evidence_id.clone(),
            hypothesis_path_id: hypothesis.candidate.path_id.clone(),
            selected_path_id: row.path_id.clone(),
        };
        h::resolve(review, &reference, inventory)?;
        let field = inventory
            .types
            .iter()
            .find(|t| t.id == row.type_id)
            .and_then(|t| t.fields.iter().find(|f| f.field_id == row.field_id))
            .ok_or("Hypothesis field disappeared.")?;
        let value = if field.type_code == 2 && row.value["boolean"] == true && row.value["raw"] == 1
        {
            Some("false")
        } else if (4..=13).contains(&field.type_code) {
            match &row.value {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.parse::<f64>().ok(),
                _ => None,
            }
            .filter(|n| n.is_finite() && *n > 0. && *n <= 20.)
            .map(|_| "0")
        } else {
            None
        };
        let Some(value) = value else { continue };
        // The same field can occur on different objects. Snapshot-local node
        // IDs cannot establish identity across captures, so do not merge paths
        // by field, value, label, or reused node ID.
        if let Some(existing) = choices
            .iter_mut()
            .find(|c| c["hypothesis_ref"]["selected_path_id"] == row.path_id)
        {
            existing["question_refs"]
                .as_array_mut()
                .unwrap()
                .push(json!(reference));
            continue;
        }
        let prior = tests
            .iter()
            .filter(|t| {
                t.investigation == investigation
                    && t.catalog_id == catalog
                    && t.candidate.id == row.path_id
                    && t.value == value
                    && t.actions == actions.actions
                    && t.origin.as_ref().is_some_and(|o| {
                        o.report_sha256 == review.report_sha256
                            && o.play_context.as_ref().is_some_and(|c| {
                                c.source.frame_metadata_sha256 == actions.frame_metadata_sha256
                            })
                    })
            })
            .max_by_key(|t| t.created_unix_ms);
        choices.push(json!({"hypothesis_ref":reference,"question_refs":[reference],"selection_role":role,"selected_candidate":row,
            "name":hypothesis.proposal.name,"explanation":hypothesis.proposal.explanation,"question":hypothesis.proposal.question,
            "type_id":row.type_id,"field_id":row.field_id,"observed_value":row.value,"value":value,"actions":actions.actions,
            "status":prior.map_or("untried",|t|t.assessment.as_ref().map_or(if t.active(){"in_progress"}else{"stopped"},|a|a.status.as_str())),
            "previous_test":prior.map(|t|t.summary()),"alternatives":hypothesis.alternatives}));
    }
    // Round-robin declared types; primary candidates precede alternatives within
    // each type. Model order remains visible within each of those queues.
    let order = batch.selected_types.clone();
    let mut sorted = vec![];
    while !choices.is_empty() {
        let before = choices.len();
        for id in &order {
            if let Some(i) = choices.iter().position(|v| v["type_id"] == *id) {
                sorted.push(choices.remove(i));
            }
        }
        if choices.len() == before {
            return Err("Hypothesis type selection is inconsistent.".into());
        }
    }
    Ok(sorted)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn observed() -> (Context, Vec<FrameRef>, Evidence) {
        let keys = [0, 68, 0, 65, 0, 68, 0, 65, 0];
        let (mut frames, _) = crate::action_patterns::tests::fixture(&keys, &[0.; 9]);
        for (i, f) in frames.iter_mut().enumerate() {
            f.sha256 = "a".repeat(64);
            if i == 0 || keys[i - 1] == 0 {
                continue;
            }
            let w = f.input_context.as_mut().unwrap();
            let start = w.requested_begin_us;
            for poll in &mut w.polls {
                if !(start + 200_000..start + 400_000).contains(&poll.begin_us) {
                    poll.down.clear();
                }
            }
        }
        let evidence = crate::play_evidence::analyze(
            &frames,
            keys.iter()
                .map(|k| crate::play_evidence::Measurement {
                    difference: Some(if *k == 0 { 0. } else { 12. }),
                    error: None,
                })
                .collect(),
        )
        .unwrap();
        let context = Context {
            evidence_frame_ids: vec![],
            basis: Default::default(),
            interpretation_source: None,
            motion: None,
            appearances: None,
            regions: None,
            source: crate::play_context::select(&frames, &evidence, "clock", 1)
                .unwrap()
                .unwrap(),
            observation: crate::play_context::Observation {
                consistent_visible_change: true,
                description: "A pattern shifts.".into(),
                uncertainty: "Its mechanism is unknown.".into(),
            },
        };
        (context, frames, evidence)
    }
    #[test]
    fn derives_sequence_and_bounds_from_complete_bracketed_samples() {
        let (context, frames, evidence) = observed();
        let plan = actions(&context, &frames, &evidence).unwrap();
        assert_eq!(
            plan.actions.iter().map(|a| a.keys[0]).collect::<Vec<_>>(),
            vec![68, 65, 68, 65]
        );
        assert!(
            plan.actions
                .iter()
                .all(|a| (180..=220).contains(&a.hold_ms))
        );
        assert!(
            plan.episodes
                .iter()
                .all(|e| e.hold_bounds_us[0] < 200_000 && e.hold_bounds_us[1] > 200_000)
        );
        for episode in &plan.episodes {
            let frame = frames.iter().find(|f| f.id == episode.after).unwrap();
            let polls: Vec<_> = frame
                .input_context
                .as_ref()
                .unwrap()
                .polls
                .iter()
                .filter(|p| p.down == episode.action.keys)
                .collect();
            let shortest_possible_hold = polls.last().unwrap().begin_us - polls[0].end_us;
            assert!(episode.hold_bounds_us[0] <= shortest_possible_hold);
        }
        let mut altered = frames.clone();
        altered[2].input_context.as_mut().unwrap().polls[3].down = vec![32];
        assert!(actions(&context, &altered, &evidence).is_err());
        let mut incomplete = evidence;
        incomplete.episodes.retain(|e| e.first_interval < 7);
        assert!(actions(&context, &frames, &incomplete).is_err());
    }

    #[test]
    fn scoped_activity_survives_unrelated_frames_but_rejects_changed_source_evidence() {
        let (legacy, frames, evidence) = observed();
        assert!(
            serde_json::to_value(&legacy)
                .unwrap()
                .get("evidence_frame_ids")
                .is_none()
        );
        let context = legacy.clone().bind_frames(&frames).unwrap();
        let mut available = frames.clone();
        let mut extra = frames.last().unwrap().clone();
        extra.id = "later-unrelated".into();
        extra.number += 1;
        available.push(extra);
        assert!(legacy.resolve_frames(&available).is_err());
        let resolved = context.resolve_frames(&available).unwrap();
        assert_eq!(
            actions(&context, &resolved, &evidence)
                .unwrap()
                .actions
                .len(),
            4
        );
        let report = "c".repeat(64);
        let review = Review {
            question_source: Default::default(),
            question_origin: None,
            question_policy: None,
            play_context: Some(context.clone()),
            evidence_id: uuid::Uuid::new_v4().to_string(),
            report_sha256: report.clone(),
            model: "image-model".into(),
            status: "complete".into(),
            error: None,
            batch: Some(h::Batch {
                report_sha256: report.clone(),
                selected_types: vec![],
                considered_samples: 0,
                omitted_samples: 0,
                accepted: vec![],
                rejected: vec![],
                limitation: h::LIMITATION.into(),
            }),
        };
        assert!(
            h::transfer_activity(Some(&review), &report, "clock", 1, &available)
                .1
                .retained
        );
        for change in ["input", "hash", "missing", "duplicate", "geometry", "order"] {
            let mut changed = available.clone();
            match change {
                "input" => changed[2].input_context = None,
                "hash" => changed[2].sha256 = "d".repeat(64),
                "missing" => {
                    changed.remove(2);
                }
                "duplicate" => changed.push(changed[2].clone()),
                "geometry" => changed.last_mut().unwrap().epoch += 1,
                _ => changed[2].number = 999,
            }
            assert!(context.resolve_frames(&changed).is_err(), "{change}");
        }
        let mut changed = context.clone();
        changed.evidence_frame_ids.swap(0, 1);
        assert!(changed.resolve_frames(&available).is_err());
        changed = context;
        changed.evidence_frame_ids[0] = changed.evidence_frame_ids[1].clone();
        assert!(changed.validate().is_err());
    }
    #[test]
    fn measured_contrast_keeps_model_ambiguity_and_all_timing_guards() {
        let (mut context, frames, evidence) = observed();
        context.observation.consistent_visible_change = false;
        context.observation.description = "The visible change is ambiguous.".into();
        // A legacy interpretation cannot silently become a measured-only basis.
        assert!(actions(&context, &frames, &evidence).is_err());
        let context = Context::from_observation(context.source, context.observation).unwrap();
        let plan = actions(&context, &frames, &evidence).unwrap();
        assert_eq!(plan.actions.len(), 4);
        assert!(!context.observation.consistent_visible_change);
        let saved = serde_json::to_value(&context).unwrap();
        assert_eq!(saved["basis"], "measured_contrast_v1");
        let restored: Context = serde_json::from_value(saved.clone()).unwrap();
        assert_eq!(restored, context);
        let mut unknown = saved;
        unknown["basis"] = json!("measured_contrast_future");
        assert!(serde_json::from_value::<Context>(unknown).is_err());
        let mut missing = frames.clone();
        missing[2].input_context = None;
        assert!(actions(&context, &missing, &evidence).is_err());
        let mut contradicted = evidence.clone();
        for action in &mut contradicted.actions {
            action.quiet_median_difference = Some(12.);
        }
        assert!(actions(&context, &frames, &contradicted).is_err());
        let (positive, _, _) = observed();
        assert!(
            serde_json::to_value(&positive)
                .unwrap()
                .get("basis")
                .is_none()
        );
        assert_eq!(
            serde_json::from_value::<Context>(serde_json::to_value(&positive).unwrap()).unwrap(),
            positive
        );
    }
    #[test]
    fn saved_activity_measurements_roundtrip_without_changing_their_bits() {
        let (mut context, _, _) = observed();
        context.source.pairs[0].difference = 12.996399059532505;
        context.source.quiet_difference = 0.6401399287801315;
        let encoded = serde_json::to_vec(&context).unwrap();
        let restored: Context = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored, context);
        assert_eq!(
            restored.source.pairs[0].difference.to_bits(),
            context.source.pairs[0].difference.to_bits()
        );
        assert_eq!(serde_json::to_vec(&restored).unwrap(), encoded);
    }
    #[test]
    fn agent_handoff_preserves_ambiguous_activity_and_original_attribution() {
        let (context, frames, evidence) = observed();
        let mut observation = context.observation;
        observation.consistent_visible_change = false;
        let context = Context::from_observation(context.source, observation).unwrap();
        let source_id = uuid::Uuid::new_v4().to_string();
        let report = "a".repeat(64);
        let mut review = Review {
            question_source: Default::default(),
            question_origin: None,
            question_policy: None,
            play_context: Some(context.clone()),
            evidence_id: source_id.clone(),
            report_sha256: report.clone(),
            model: "original-image-model".into(),
            status: "complete".into(),
            error: None,
            batch: Some(h::Batch {
                report_sha256: report.clone(),
                selected_types: vec![],
                considered_samples: 0,
                omitted_samples: 0,
                accepted: vec![],
                rejected: vec![],
                limitation: h::LIMITATION.into(),
            }),
        };
        let transfer =
            |r: Option<&Review>, f: &[FrameRef]| h::transfer_activity(r, &report, "clock", 1, f);
        let (retained, result) = transfer(Some(&review), &frames);
        assert!(result.retained);
        assert_eq!(result.from_review_id, Some(source_id.clone()));
        let retained = retained.unwrap();
        assert_eq!(retained.source, context.source);
        assert_eq!(retained.observation, context.observation);
        assert_eq!(
            actions(&retained, &frames, &evidence)
                .unwrap()
                .actions
                .len(),
            4
        );
        let producer = retained.interpretation_source.as_ref().unwrap();
        assert_eq!(producer.review_id, source_id);
        assert_eq!(producer.model, "original-image-model");
        assert!(
            retained
                .clone()
                .bind_interpretation(&source_id, "replacement")
                .is_err()
        );
        // A second agent may replace the questions, but cannot become the image's producer.
        review.play_context = Some(retained.clone());
        review.evidence_id = uuid::Uuid::new_v4().to_string();
        review.model = "agent:second".into();
        let (again, _) = transfer(Some(&review), &frames);
        assert_eq!(again.unwrap(), retained);
        let saved = serde_json::to_vec(&review).unwrap();
        let restored: Review = serde_json::from_slice(&saved).unwrap();
        assert_eq!(transfer(Some(&restored), &frames).0, Some(retained));

        let mut changed = frames.clone();
        changed[1].sha256 = "b".repeat(64);
        assert!(!transfer(Some(&review), &changed).1.retained);
        changed = frames.clone();
        changed[2].input_context = None;
        assert!(!transfer(Some(&review), &changed).1.retained);
        for (inv, batch, hash) in [
            ("other", 1, report.clone()),
            ("clock", 2, report.clone()),
            ("clock", 1, "b".repeat(64)),
        ] {
            assert!(
                h::transfer_activity(Some(&review), &hash, inv, batch, &frames)
                    .0
                    .is_none()
            );
        }
        for state in ["pending", "failed", "cancelled"] {
            let mut invalid = review.clone();
            invalid.status = state.into();
            assert!(!transfer(Some(&invalid), &frames).1.retained);
        }
        let mut invalid = review.clone();
        invalid.error = Some("inference failed".into());
        assert!(!transfer(Some(&invalid), &frames).1.retained);
        invalid = review.clone();
        invalid
            .play_context
            .as_mut()
            .unwrap()
            .interpretation_source
            .as_mut()
            .unwrap()
            .review_id = "invented".into();
        assert!(!transfer(Some(&invalid), &frames).1.retained);
        invalid = review;
        invalid.play_context = None;
        assert!(!transfer(Some(&invalid), &frames).1.retained);
        assert!(!transfer(None, &frames).1.retained);
    }
    fn question_fixture() -> (Review, Inventory, ObservedActions) {
        let report = "a".repeat(64);
        // Runtime integer samples are JSON numbers; floating samples are strings.
        // Exercise both through the real inventory/hypothesis representation.
        let make_field = |id, name: &str, paths: Vec<&str>| {
            let (type_code, value) = if id == 2 {
                (8, json!(8))
            } else {
                (12, json!("8"))
            };
            json!({"field_id":id,"name":name,"type_code":type_code,"values":[value],"path_count":paths.len(),"samples":paths.iter().map(|p|json!({"path_id":p.repeat(64),"value":value,"context":[]})).collect::<Vec<_>>()})
        };
        let inventory:Inventory=serde_json::from_value(json!({"schema":1,"report_sha256":report,"scope":{"pid":1,"created":"0000000000000001","session":3},
            "types":[{"id":0,"key":{"image":"game","type_name":"Motion","layout_sha256":report},"fields":[make_field(1,"walkSpeed",vec!["b","e"]),make_field(2,"runSpeed",vec!["c"])]},
                {"id":1,"key":{"image":"game","type_name":"View","layout_sha256":report},"fields":[make_field(3,"lookSpeed",vec!["d"])]}],
            "reachable_paths":4,"omitted_core_paths":0,"omitted_array_elements":0,"omitted_non_numeric_paths":0,"limitation":"fixture"})).unwrap();
        let selection = h::select(&inventory, &[0, 1]).unwrap();
        let hypotheses = [0, 2, 3, 1]
            .iter()
            .map(|i| {
                let r = &selection.rows[*i];
                h::Proposal {
                    candidate: r.id,
                    name: r.field_name.clone(),
                    explanation: "This labelled parameter needs an effect test.".into(),
                    question: format!("Does {} change the rendered response?", r.field_name),
                    citations: vec![h::Citation {
                        source: "field_name".into(),
                        index: None,
                        quote: r.field_name.clone(),
                    }],
                    alternatives: vec![],
                }
            })
            .collect();
        let (context, frames, evidence) = observed();
        let review = Review {
            question_source: Default::default(),
            question_origin: None,
            question_policy: None,
            play_context: Some(context.clone()),
            evidence_id: uuid::Uuid::new_v4().to_string(),
            report_sha256: report,
            model: "test".into(),
            status: "complete".into(),
            batch: Some(h::validate(&selection, h::Proposals { hypotheses }).unwrap()),
            error: None,
        };
        assert_eq!(review.batch.as_ref().unwrap().accepted.len(), 4);
        let plan = actions(&context, &frames, &evidence).unwrap();
        (review, inventory, plan)
    }
    #[test]
    fn questions_keep_distinct_paths_to_the_same_field() {
        let (review, inventory, plan) = question_fixture();
        let choices = choices(&review, &inventory, &plan, &[], "inv", "catalog").unwrap();
        assert_eq!(
            choices
                .iter()
                .map(|c| c["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["walkSpeed", "lookSpeed", "runSpeed", "walkSpeed"]
        );
        assert!(
            choices
                .iter()
                .all(|c| c["status"] == "untried" && c["value"] == "0")
        );
        assert_ne!(
            choices[0]["selected_candidate"]["path_id"],
            choices[3]["selected_candidate"]["path_id"]
        );
    }
    fn competing_questions(review: &mut Review, inventory: &Inventory) {
        let selection = h::select(inventory, &[0, 1]).unwrap();
        let batch = review.batch.as_ref().unwrap();
        let mut first = batch.accepted[0].proposal.clone();
        first.alternatives = vec![1, 2, 3];
        let mut second = batch.accepted[1].proposal.clone();
        second.alternatives = vec![1];
        review.batch = Some(
            h::validate(
                &selection,
                h::Proposals {
                    hypotheses: vec![first, second],
                },
            )
            .unwrap(),
        );
        assert_eq!(review.batch.as_ref().unwrap().accepted.len(), 2);
    }
    #[test]
    fn alternatives_keep_exact_provenance_and_share_only_identical_paths() {
        let (mut review, inventory, plan) = question_fixture();
        competing_questions(&mut review, &inventory);
        let proposed = choices(&review, &inventory, &plan, &[], "inv", "catalog").unwrap();
        assert_eq!(proposed.len(), 4);
        // The same path is mentioned by two questions, but is proposed once.
        let shared = &proposed[1];
        assert_eq!(shared["selection_role"], "alternative");
        assert_eq!(shared["selected_candidate"]["field_name"], "lookSpeed");
        assert_eq!(shared["name"], "walkSpeed"); // question label, not alternative semantics
        assert_eq!(shared["question_refs"].as_array().unwrap().len(), 2);
        // Prefer a path's own hypothesis, even when an earlier question also
        // mentions it as an alternative. Keep both references.
        assert_eq!(proposed[2]["selection_role"], "primary");
        assert_eq!(proposed[2]["name"], "runSpeed");
        assert_eq!(proposed[2]["question_refs"].as_array().unwrap().len(), 2);
        assert_eq!(proposed[3]["selection_role"], "alternative");
        for choice in &proposed {
            let references = choice["question_refs"].as_array().unwrap();
            assert_eq!(references[0], choice["hypothesis_ref"]);
            for reference in references {
                let origin = h::resolve(
                    &review,
                    &serde_json::from_value(reference.clone()).unwrap(),
                    &inventory,
                )
                .unwrap();
                assert_eq!(
                    json!(origin.selected().unwrap()),
                    choice["selected_candidate"]
                );
            }
        }
        let mut changed = inventory.clone();
        changed.types[1].fields[0].samples[0].value = json!("9");
        assert!(choices(&review, &changed, &plan, &[], "inv", "catalog").is_err());
    }
    #[test]
    fn ineligible_primary_does_not_hide_a_bounded_alternative() {
        let (mut review, mut inventory, plan) = question_fixture();
        // A zero primary cannot use the planner's zero probe. Its other object
        // and another field remain separate eligible questions.
        inventory.types[0].fields[0].samples[0].value = json!("0");
        inventory.types[0].fields[0].values = vec![json!("0"), json!("8")];
        // A large alternative is outside the existing probe bounds.
        inventory.types[1].fields[0].samples[0].value = json!("50");
        inventory.types[1].fields[0].values = vec![json!("50")];
        competing_questions(&mut review, &inventory);
        let proposed = choices(&review, &inventory, &plan, &[], "inv", "catalog").unwrap();
        assert_eq!(proposed.len(), 2);
        assert_eq!(proposed[0]["selection_role"], "primary");
        assert_eq!(proposed[0]["selected_candidate"]["field_name"], "runSpeed");
        assert_eq!(proposed[1]["selection_role"], "alternative");
        assert_eq!(proposed[1]["observed_value"], "8");
        // Boolean alternatives use false only for the canonical observed true.
        inventory.types[0].fields[1].type_code = 2;
        for (value, expected) in [
            (json!({"boolean":true,"raw":1}), 2),
            (json!({"boolean":false,"raw":0}), 1),
        ] {
            inventory.types[0].fields[1].samples[0].value = value.clone();
            inventory.types[0].fields[1].values = vec![value];
            competing_questions(&mut review, &inventory);
            let proposed = choices(&review, &inventory, &plan, &[], "inv", "catalog").unwrap();
            assert_eq!(proposed.len(), expected);
            if expected == 2 {
                assert_eq!(proposed[0]["value"], "false");
            }
        }
    }
    #[test]
    fn prior_results_apply_only_to_the_exact_path_and_comparison_context() {
        use crate::runtime_tests::{Assessment, Phase};
        let (mut review, inventory, plan) = question_fixture();
        competing_questions(&mut review, &inventory);
        let before = choices(&review, &inventory, &plan, &[], "inv", "catalog").unwrap();
        let reference: Reference =
            serde_json::from_value(before[0]["hypothesis_ref"].clone()).unwrap();
        let origin = h::resolve(&review, &reference, &inventory).unwrap();
        // Synthetic history entries exercise selection, not a completed native
        // experiment. This fixture has no target, byte receipt or causal claim.
        let mut prior: Test = serde_json::from_value(json!({
            "id":uuid::Uuid::new_v4().to_string(),"change_id":uuid::Uuid::new_v4().to_string(),"actor":"test",
            "investigation":"inv","catalog_id":"catalog","scope":inventory.scope,"window":1,
            "candidate":{"id":reference.selected_path_id,"name":"walkSpeed","description":"Fixture",
                "locator":{"scope":inventory.scope,"root":{"declaring":inventory.types[0].key,"field_id":0},"steps":[]},
                "value":"8","bytes":[0,0,0,65],"type_code":12,"snapshot_id":0,"node_id":0,"edit_authority":false},
            "value":"0","hypothesis":origin.hypothesis.proposal.question,"origin":origin,"actions":plan.actions,
            "prepared_revision":0,"created_unix_ms":1,"preparation":null,"metric":crate::runtime_tests::METRIC_ID,
            "revision":1,"phase":"stopped","pending_capture":false,"conditions":[{"frames":[]},{"frames":[]},{"frames":[]}],
            "assessment":null,"reason":"Synthetic stopped record"
        })).unwrap();
        let get = |history: &[Test]| {
            choices(&review, &inventory, &plan, history, "inv", "catalog").unwrap()
        };
        for status in [
            "stopped",
            "in_progress",
            "inconclusive",
            "suppression_observed",
            "suppression_not_observed",
        ] {
            prior.phase = match status {
                "stopped" => Phase::Stopped,
                "in_progress" => Phase::Baseline,
                _ => Phase::Complete,
            };
            prior.assessment = (!matches!(status, "stopped" | "in_progress")).then(|| Assessment {
                status: status.into(),
                reason: "Synthetic assessment".into(),
                medians: [None; 3],
                limitation: "Fixture".into(),
            });
            let proposed = get(&[prior.clone()]);
            assert_eq!(proposed[0]["status"], status);
            assert_eq!(proposed[0]["previous_test"]["id"], prior.id);
            assert!(proposed[1..].iter().all(|c| c["status"] == "untried"));
            // Even a positive result does not remove or validate alternatives,
            // including the other path to exactly the same declared field.
            assert_eq!(proposed.len(), 4);
        }
        let mut latest = prior.clone();
        latest.created_unix_ms += 1;
        latest.phase = Phase::Stopped;
        latest.assessment = None;
        assert_eq!(get(&[latest, prior.clone()])[0]["status"], "stopped");
        for mutation in 0..7 {
            let mut changed = prior.clone();
            match mutation {
                0 => changed.investigation.push('!'),
                1 => changed.catalog_id.push('!'),
                2 => changed.value = "1".into(),
                3 => changed.actions[0].hold_ms += 1,
                4 => changed.origin.as_mut().unwrap().report_sha256 = "0".repeat(64),
                5 => {
                    changed
                        .origin
                        .as_mut()
                        .unwrap()
                        .play_context
                        .as_mut()
                        .unwrap()
                        .source
                        .frame_metadata_sha256 = "0".repeat(64)
                }
                _ => changed.origin = None,
            }
            assert!(get(&[changed]).iter().all(|c| c["status"] == "untried"));
        }
    }
}
