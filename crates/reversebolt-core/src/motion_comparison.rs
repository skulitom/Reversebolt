//! Compare retained image displacements without trusting an interpretation's prose.
use crate::{Result, activity_motion, play_context::Source, visual};
use serde::Serialize;

pub const POLICY: &str = "selected_patch_displacement_relation_v1";
pub const LIMITATION: &str = "Comparison of selected image-patch displacements in two discovery intervals. It does not establish object identity, whole-scene motion, velocity, physical speed, input delivery, causality, gameplay meaning or a trial outcome. Thresholds are heuristic. Small or missing displacement is not proof of no movement.";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    MissingMeasurement,
    InsufficientSupport,
    BelowThreshold,
    MatchingDisplacement,
    AlignedDirection,
    OpposedDirection,
    DifferentDirection,
}
#[derive(Debug, Serialize)]
pub struct Pair {
    pub before_id: String,
    pub before_sha256: String,
    pub after_id: String,
    pub after_sha256: String,
    pub selected: Option<usize>,
    pub matched: Option<usize>,
    pub matched_cells: Option<usize>,
    pub displacement: Option<[i32; 2]>,
    pub supporting_patches: Option<usize>,
    pub supporting_cells: Option<usize>,
    pub error: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Comparison {
    pub policy: &'static str,
    pub metric: String,
    pub scope: &'static str,
    pub source_sha256: String,
    pub motion_summary_sha256: String,
    pub measurement_dimensions: [u32; 2],
    pub relation: Relation,
    pub pairs: [Pair; 2],
    pub description: String,
    pub limitation: &'static str,
}
fn squared([x, y]: [i32; 2]) -> i64 {
    i64::from(x) * i64::from(x) + i64::from(y) * i64::from(y)
}
fn direction([x, y]: [i32; 2]) -> String {
    let mut parts = vec![];
    if x != 0 {
        parts.push(format!(
            "{} pixels {}",
            x.abs(),
            if x > 0 { "right" } else { "left" }
        ));
    }
    if y != 0 {
        parts.push(format!(
            "{} pixels {}",
            y.abs(),
            if y > 0 { "down" } else { "up" }
        ));
    }
    if parts.is_empty() {
        "zero integer displacement".into()
    } else {
        parts.join(" and ")
    }
}
fn relation(pairs: &[Pair; 2]) -> Relation {
    if pairs.iter().any(|p| p.selected.is_none()) {
        return Relation::MissingMeasurement;
    }
    let [Some(a), Some(b)] = pairs.each_ref().map(|p| p.displacement) else {
        return Relation::InsufficientSupport;
    };
    if squared(a) < 4 || squared(b) < 4 {
        return Relation::BelowThreshold;
    }
    if (0..2).all(|i| (a[i] - b[i]).abs() <= 1) {
        return Relation::MatchingDisplacement;
    }
    let dot = i64::from(a[0]) * i64::from(b[0]) + i64::from(a[1]) * i64::from(b[1]);
    if dot * dot * 100 >= squared(a) * squared(b) * 81 {
        if dot > 0 {
            Relation::AlignedDirection
        } else {
            Relation::OpposedDirection
        }
    } else {
        Relation::DifferentDirection
    }
}
/// Only accepts validated, hash-bound inputs; no images, model, target or mutation.
pub fn compare(source: &Source, summary: &activity_motion::Summary) -> Result<Comparison> {
    summary.validate(source)?;
    let pairs = summary.pairs.each_ref().map(|p| {
        let m = p.measurement.as_ref();
        let c = m.and_then(|m| m.consensus.as_ref());
        Pair {
            before_id: p.before_id.clone(),
            before_sha256: p.before_sha256.clone(),
            after_id: p.after_id.clone(),
            after_sha256: p.after_sha256.clone(),
            selected: m.map(|m| m.selected),
            matched: m.map(|m| m.matches.len()),
            matched_cells: m.map(|m| m.matched_cells),
            displacement: c.map(|c| c.delta),
            supporting_patches: c.map(|c| c.supporting_patches),
            supporting_cells: c.map(|c| c.supporting_cells),
            error: p.error.clone(),
        }
    });
    let relation = relation(&pairs);
    let description=match relation {
        Relation::MissingMeasurement=>"At least one image pair has no usable displacement measurement.".into(),
        Relation::InsufficientSupport=>"At least one image pair lacks enough distributed matching patches to compare displacement.".into(),
        Relation::BelowThreshold=>"At least one supported displacement is below the two-pixel comparison threshold. This does not establish no movement.".into(),
        _=>{
            let a=direction(pairs[0].displacement.unwrap());let b=direction(pairs[1].displacement.unwrap());
            let conclusion=match relation {
                Relation::MatchingDisplacement=>"They agree within one normalized pixel per axis.",
                Relation::AlignedDirection=>"They point in similar directions, but their displacements differ.",
                Relation::OpposedDirection=>"They point in opposing directions.",
                _=>"Their directions differ beyond this comparison's alignment threshold.",
            };
            format!("Supported patch displacements are {a}, then {b}. {conclusion}")
        }
    };
    Ok(Comparison {
        policy: POLICY,
        metric: summary.metric.clone(),
        scope: "discovery_activity_pairs",
        source_sha256: visual::digest(&serde_json::to_vec(source).map_err(|e| e.to_string())?),
        motion_summary_sha256: visual::digest(
            &serde_json::to_vec(summary).map_err(|e| e.to_string())?,
        ),
        measurement_dimensions: summary.measurement_dimensions,
        relation,
        pairs,
        description,
        limitation: LIMITATION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{activity_motion as m, play_context as pc, visual::CaptureInterval};
    fn fixture(a: [i32; 2], b: [i32; 2]) -> (Source, m::Summary) {
        let frame = |n| pc::Frame {
            id: format!("f{n}"),
            sha256: format!("{n:064x}"),
            number: n,
            width: 256,
            height: 144,
            interval: CaptureInterval {
                clock_id: "synthetic".into(),
                begin_us: n as u64 * 1_000_000,
                end_us: n as u64 * 1_000_000 + 1000,
            },
        };
        let source = Source {
            investigation_id: "synthetic".into(),
            batch: 1,
            frame_metadata_sha256: "a".repeat(64),
            key: 68,
            quiet_difference: 0.,
            pairs: [1, 4].map(|n| pc::Pair {
                episode: n,
                before: frame(n),
                after: frame(n + 1),
                input_sha256: "b".repeat(64),
                difference: 20.,
            }),
        };
        let pairs = [a, b]
            .into_iter()
            .zip(&source.pairs)
            .map(|(delta, p)| m::Pair {
                before_id: p.before.id.clone(),
                before_sha256: p.before.sha256.clone(),
                after_id: p.after.id.clone(),
                after_sha256: p.after.sha256.clone(),
                error: None,
                measurement: Some(m::Measurement {
                    selected: 12,
                    matched_cells: 6,
                    unresolved: Default::default(),
                    matches: [20, 62]
                        .into_iter()
                        .flat_map(|y| {
                            [20, 34, 62, 76, 104, 118].map(|x| m::Match { at: [x, y], delta })
                        })
                        .collect(),
                    consensus: Some(m::Consensus {
                        delta,
                        supporting_patches: 12,
                        supporting_cells: 6,
                    }),
                }),
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let summary = m::Summary {
            metric: m::METRIC_V2.into(),
            frame_metadata_sha256: source.frame_metadata_sha256.clone(),
            measurement_dimensions: [256, 144],
            maximum_selected: 48,
            grid: [6, 4],
            pairs,
        };
        (source, summary)
    }
    #[test]
    fn matching_aligned_opposed_and_orthogonal_vectors_keep_their_signs() {
        for (a, b, expected) in [
            ([18, 0], [18, 0], Relation::MatchingDisplacement),
            ([18, 0], [19, 1], Relation::MatchingDisplacement),
            ([18, 0], [26, 0], Relation::AlignedDirection),
            ([18, 0], [-18, 0], Relation::OpposedDirection),
            ([18, 0], [0, 18], Relation::DifferentDirection),
            ([-18, -4], [-18, -4], Relation::MatchingDisplacement),
        ] {
            let (source, summary) = fixture(a, b);
            let result = compare(&source, &summary).unwrap();
            assert_eq!(result.relation, expected);
            assert_eq!(result.pairs[0].displacement, Some(a));
            assert_eq!(result.pairs[1].displacement, Some(b));
            assert_eq!(result.pairs[0].supporting_cells, Some(6));
        }
        let (source, summary) = fixture([-18, -4], [18, 4]);
        let result = compare(&source, &summary).unwrap();
        assert!(
            result
                .description
                .contains("18 pixels left and 4 pixels up, then 18 pixels right and 4 pixels down")
        );
        assert_eq!(result.relation, Relation::OpposedDirection);
    }
    #[test]
    fn small_sparse_and_missing_evidence_never_become_stationary_or_matching() {
        for (a, b) in [([0, 0], [0, 0]), ([1, 1], [18, 0])] {
            let (source, summary) = fixture(a, b);
            assert_eq!(
                compare(&source, &summary).unwrap().relation,
                Relation::BelowThreshold
            );
        }
        let (source, mut summary) = fixture([18, 0], [18, 0]);
        summary.pairs[1].measurement.as_mut().unwrap().consensus = None;
        assert_eq!(
            compare(&source, &summary).unwrap().relation,
            Relation::InsufficientSupport
        );
        summary.pairs[1].measurement = None;
        summary.pairs[1].error = Some("Image unavailable.".into());
        let result = compare(&source, &summary).unwrap();
        assert_eq!(result.relation, Relation::MissingMeasurement);
        assert!(result.pairs[1].matched.is_none());
        assert!(result.pairs[1].displacement.is_none());
    }
    #[test]
    fn source_rebinding_and_invalid_consensus_fail_before_arithmetic() {
        let (mut source, mut summary) = fixture([18, 0], [18, 0]);
        let first = compare(&source, &summary).unwrap();
        source.pairs[0].before.sha256 = "c".repeat(64);
        assert!(compare(&source, &summary).is_err());
        summary.pairs[0].before_sha256 = source.pairs[0].before.sha256.clone();
        let rebound = compare(&source, &summary).unwrap();
        assert_ne!(first.source_sha256, rebound.source_sha256);
        assert_ne!(first.motion_summary_sha256, rebound.motion_summary_sha256);
        summary.pairs[0]
            .measurement
            .as_mut()
            .unwrap()
            .consensus
            .as_mut()
            .unwrap()
            .delta = [i32::MIN, 0];
        assert!(compare(&source, &summary).is_err());
    }
}
