//! Recompute a bounded image-displacement comparison from retained trial evidence.
use crate::patch_motion;
use reversebolt_core::{
    Result,
    inputs::Window,
    runtime_tests::{InputWitness, Test},
    visual,
};
use serde::Serialize;

pub const POLICY: &str = "trial_selected_patch_displacement_v1";
pub const LIMITATION: &str = "Selected image-patch displacements across the recorded baseline, changed and recovered conditions. The same action index is compared, not different actions pooled together. This is retrospective image evidence, not an edit assessment, object identity, physical speed, input delivery, causality or a verified gameplay control. Scene changes and different sampled patches can confound it. Zero displacement is not proof of no movement. Existing trial outcomes are unchanged.";

#[derive(Debug, Serialize)]
pub struct Motion {
    pub selected: usize,
    pub matched: usize,
    pub matched_cells: usize,
    pub displacement: Option<[i32; 2]>,
    pub supporting_patches: Option<usize>,
    pub supporting_cells: Option<usize>,
}
#[derive(Debug, Serialize)]
pub struct Pair {
    pub before_frame: usize,
    pub after_frame: usize,
    pub before_sha256: String,
    pub after_sha256: String,
    pub input_sha256: String,
    pub motion: Option<Motion>,
    pub measurement_error: Option<String>,
    pub input: Option<InputWitness>,
    pub input_error: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Condition {
    pub name: &'static str,
    pub pairs: Vec<Pair>,
}
#[derive(Debug, Serialize)]
pub struct Action {
    pub index: usize,
    pub keys: Vec<u8>,
    pub hold_ms: u32,
    pub displacement: [Option<[i32; 2]>; 3],
    pub changed: Option<bool>,
    pub returned: Option<bool>,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub policy: &'static str,
    pub metric: &'static str,
    pub scope: &'static str,
    pub test_id: String,
    pub record_sha256: String,
    pub measurement_dimensions: [u32; 2],
    pub conditions: [Condition; 3],
    pub actions: Vec<Action>,
    pub status: &'static str,
    pub description: &'static str,
    pub limitation: &'static str,
}

pub(crate) fn distance(a: [i32; 2], b: [i32; 2]) -> i32 {
    (a[0] - b[0]).abs().max((a[1] - b[1]).abs())
}
pub(crate) fn changed(a: [i32; 2], b: [i32; 2]) -> Option<bool> {
    match distance(a, b) {
        0..=1 => Some(false),
        2 => None, // Leave a gap between agreement and a detected difference.
        _ => Some(true),
    }
}
fn verdict(conditions: &[Condition; 3], actions: &[Action]) -> (&'static str, &'static str) {
    if conditions.iter().any(|c| c.pairs.len() != 5) {
        return (
            "incomplete",
            "All three conditions need their six retained images before displacement can be compared.",
        );
    }
    if conditions
        .iter()
        .flat_map(|c| &c.pairs)
        .any(|p| p.measurement_error.is_some() || p.input_error.is_some())
    {
        return (
            "unavailable_evidence",
            "Some image or sampled-input evidence could not be verified. Available measurements are retained.",
        );
    }
    let generations: std::collections::BTreeSet<_> = conditions
        .iter()
        .flat_map(|c| &c.pairs)
        .filter_map(|p| p.input.as_ref().map(|i| &i.generation))
        .collect();
    if generations.len() != 1
        || conditions
            .iter()
            .flat_map(|c| &c.pairs)
            .any(|p| p.input.as_ref().is_none_or(|i| !i.matches))
    {
        return (
            "input_mismatch",
            "The retained input sampling does not support the same planned sequence in all three conditions.",
        );
    }
    if conditions
        .iter()
        .flat_map(|c| &c.pairs)
        .any(|p| p.motion.as_ref().is_none_or(|m| m.displacement.is_none()))
    {
        return (
            "insufficient_support",
            "At least one interval lacks enough distributed patch matches. Missing displacement is not zero motion.",
        );
    }
    if conditions.iter().any(|c| {
        distance(
            c.pairs[0].motion.as_ref().unwrap().displacement.unwrap(),
            [0, 0],
        ) > 1
    }) {
        return (
            "idle_displacement",
            "Displacement was also measured during an idle interval, so the action comparisons remain inconclusive.",
        );
    }
    if actions
        .iter()
        .any(|a| distance(a.displacement[0].unwrap(), [0, 0]) < 2)
    {
        return (
            "below_threshold",
            "At least one baseline action displacement is below two normalized pixels. This does not establish no movement.",
        );
    }
    if actions
        .iter()
        .all(|a| a.changed == Some(true) && a.returned == Some(true))
    {
        (
            "displacement_changed_and_returned",
            "For all four action indices, selected-patch displacement differed in the changed condition and returned within one normalized pixel per axis of baseline in recovery.",
        )
    } else if actions
        .iter()
        .all(|a| a.changed == Some(false) && a.returned == Some(true))
    {
        (
            "similar_displacement",
            "For all four action indices, selected-patch displacement stayed within one normalized pixel per axis of baseline in both later conditions.",
        )
    } else {
        (
            "mixed_displacement",
            "The four action comparisons do not all support the same change-and-return pattern. Inspect their separate displacements.",
        )
    }
}

/// Readers supply only saved bytes. Hashes, dimensions and sampled input are
/// checked here too, so the file replay and engine use the same evidence gates.
pub fn analyze(
    test: &Test,
    read_image: impl FnMut(usize, usize) -> Result<Vec<u8>>,
    read_input: impl FnMut(usize, usize) -> Result<Option<Window>>,
) -> Result<Report> {
    let mut conditions = ["baseline", "changed", "recovered"].map(|name| Condition {
        name,
        pairs: vec![],
    });
    crate::trial_evidence::visit(test, read_image, read_input, |pair| {
        let motion = pair.images.and_then(|(a, b)| -> Result<Motion> {
            let report = patch_motion::measure(a, b)?;
            if report.source_dimensions != [pair.after.width, pair.after.height] {
                return Err("PNG dimensions disagree with the retained frame.".into());
            }
            Ok(Motion {
                selected: report.selection.selected_patches,
                matched: report.matched_patches,
                matched_cells: report.matched_cells,
                displacement: report.consensus.as_ref().map(|c| c.displacement),
                supporting_patches: report.consensus.as_ref().map(|c| c.supporting_patches),
                supporting_cells: report.consensus.as_ref().map(|c| c.supporting_cells),
            })
        });
        conditions[pair.condition].pairs.push(Pair {
            before_frame: pair.index - 1,
            after_frame: pair.index,
            before_sha256: pair.before.sha256.clone(),
            after_sha256: pair.after.sha256.clone(),
            input_sha256: pair.after.input_sha256.clone(),
            measurement_error: motion.as_ref().err().map(|e| e.chars().take(512).collect()),
            motion: motion.ok(),
            input_error: pair
                .input
                .as_ref()
                .err()
                .map(|e| e.chars().take(512).collect()),
            input: pair.input.ok(),
        });
    })?;
    let actions = test
        .actions
        .iter()
        .enumerate()
        .map(|(index, action)| {
            let displacement = conditions.each_ref().map(|c| {
                c.pairs
                    .get(index + 1)
                    .and_then(|p| p.motion.as_ref())
                    .and_then(|m| m.displacement)
            });
            Action {
                index,
                keys: action.keys.clone(),
                hold_ms: action.hold_ms,
                displacement,
                changed: displacement[0]
                    .zip(displacement[1])
                    .and_then(|(a, b)| changed(a, b)),
                returned: displacement[0]
                    .zip(displacement[2])
                    .and_then(|(a, b)| changed(a, b))
                    .map(|v| !v),
            }
        })
        .collect::<Vec<_>>();
    let (status, description) = verdict(&conditions, &actions);
    Ok(Report {
        policy: POLICY,
        metric: patch_motion::METRIC,
        scope: "retained_trial_conditions",
        test_id: test.id.clone(),
        record_sha256: visual::digest(&serde_json::to_vec(test).map_err(|e| e.to_string())?),
        measurement_dimensions: [256, 144],
        conditions,
        actions,
        status,
        description,
        limitation: LIMITATION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> ([Condition; 3], Vec<Action>) {
        let conditions = ["baseline", "changed", "recovered"].map(|name| Condition {
            name,
            pairs: (0..5)
                .map(|i| Pair {
                    before_frame: i,
                    after_frame: i + 1,
                    before_sha256: "a".repeat(64),
                    after_sha256: "b".repeat(64),
                    input_sha256: "c".repeat(64),
                    motion: Some(Motion {
                        selected: 20,
                        matched: 20,
                        matched_cells: 8,
                        displacement: Some([0, 0]),
                        supporting_patches: Some(20),
                        supporting_cells: Some(8),
                    }),
                    measurement_error: None,
                    input_error: None,
                    input: Some(InputWitness {
                        generation: "test".into(),
                        matches: true,
                        held_us: vec![200000],
                        polls: 40,
                        largest_gap_us: 16000,
                        reason: "Synthetic input summary.".into(),
                    }),
                })
                .collect(),
        });
        let actions = (0..4)
            .map(|index| {
                let base = if index % 2 == 0 { [8, 0] } else { [-8, 0] };
                Action {
                    index,
                    keys: vec![68],
                    hold_ms: 200,
                    displacement: [Some(base), Some([0, 0]), Some(base)],
                    changed: Some(true),
                    returned: Some(true),
                }
            })
            .collect();
        (conditions, actions)
    }
    #[test]
    fn opposite_action_directions_are_compared_separately_and_recovery_is_required() {
        let (conditions, mut actions) = fixture();
        assert_eq!(
            verdict(&conditions, &actions).0,
            "displacement_changed_and_returned"
        );
        // Opposite baseline vectors would cancel in a pooled vector average.
        actions[3].returned = Some(false);
        actions[3].displacement[2] = Some([8, 0]);
        assert_eq!(verdict(&conditions, &actions).0, "mixed_displacement");
        assert_eq!(changed([8, 0], [7, 1]), Some(false));
        assert_eq!(changed([8, 0], [6, 0]), None);
        assert_eq!(changed([8, 0], [5, 0]), Some(true));
    }
    #[test]
    fn missing_support_idle_displacement_and_bad_inputs_never_pass() {
        let (mut conditions, actions) = fixture();
        conditions[1].pairs[2].motion.as_mut().unwrap().displacement = None;
        assert_eq!(verdict(&conditions, &actions).0, "insufficient_support");
        conditions[1].pairs[2].motion.as_mut().unwrap().displacement = Some([0, 0]);
        conditions[1].pairs[0].motion.as_mut().unwrap().displacement = Some([2, 0]);
        assert_eq!(verdict(&conditions, &actions).0, "idle_displacement");
        conditions[1].pairs[1].input.as_mut().unwrap().matches = false;
        assert_eq!(verdict(&conditions, &actions).0, "input_mismatch");
        conditions[1].pairs[1].input_error = Some("Bad hash".into());
        assert_eq!(verdict(&conditions, &actions).0, "unavailable_evidence");
        conditions[2].pairs.pop();
        assert_eq!(verdict(&conditions, &actions).0, "incomplete");
    }
}
