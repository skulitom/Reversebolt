//! Compare one immutable baseline appearance across retained trial conditions.
use crate::{
    region_appearance as matcher, trial_evidence,
    trial_motion::{Action, changed, distance},
};
use reversebolt_core::{
    Result,
    inputs::Window,
    runtime_tests::{InputWitness, Test},
    visual,
    visual_regions::Region,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const POLICY: &str = "trial_baseline_appearance_displacement_v1";
pub const LIMITATION: &str = "Retrospective comparison of one caller-selected rectangle from baseline frame 0. Every match uses that original image and rectangle; each condition is an independent branch and stops on its first unresolved image. No template update or reacquisition within a condition. At most eighteen template checks. Visually identical replacements, matching background and camera motion can mislead; cross-condition matches do not prove persistent identity. Normalized image displacement is not physical speed, input delivery, gameplay meaning, causality or complete recovery. A region chosen after seeing results can bias the comparison. No saved assessment or edit authority is changed.";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub baseline_sha256: String,
    pub region: Region,
}

#[derive(Debug, Serialize)]
pub struct Frame {
    pub index: usize,
    pub image_sha256: String,
    pub measurement: Option<matcher::Report>,
    pub error: Option<String>,
    pub stopped: bool,
}
impl Frame {
    fn position(&self) -> Option<[i32; 2]> {
        let m = self.measurement.as_ref()?;
        if m.status != matcher::Status::Matched {
            return None;
        }
        m.forward
            .as_ref()?
            .best_at
            .map(|p| [p[0] as i32, p[1] as i32])
    }
}

#[derive(Debug, Serialize)]
pub struct Pair {
    pub before_frame: usize,
    pub after_frame: usize,
    pub input_sha256: String,
    pub displacement: Option<[i32; 2]>,
    pub original_pixel_displacement: Option<[f64; 2]>,
    pub source_error: Option<String>,
    pub input: Option<InputWitness>,
    pub input_error: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Condition {
    pub name: &'static str,
    pub frames: Vec<Frame>,
    pub pairs: Vec<Pair>,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub policy: &'static str,
    pub matcher_policy: &'static str,
    pub test_id: String,
    pub record_sha256: String,
    pub source_sha256: String,
    pub selection: Selection,
    pub measurement_dimensions: [u32; 2],
    pub attempted_frames: usize,
    pub maximum_attempted_frames: usize,
    pub conditions: [Condition; 3],
    pub actions: Vec<Action>,
    pub status: &'static str,
    pub description: &'static str,
    pub limitation: &'static str,
}

fn outcome(conditions: &[Condition; 3], actions: &[Action]) -> (&'static str, &'static str) {
    if conditions
        .iter()
        .any(|c| c.frames.len() != 6 || c.pairs.len() != 5)
    {
        return (
            "incomplete",
            "All three conditions need six retained images.",
        );
    }
    if conditions.iter().any(|c| {
        c.frames.iter().any(|f| f.error.is_some())
            || c.pairs
                .iter()
                .any(|p| p.source_error.is_some() || p.input_error.is_some())
    }) {
        return (
            "unavailable_evidence",
            "Some retained image or input evidence could not be verified.",
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
            "Input sampling does not support the same planned sequence in all conditions.",
        );
    }
    if conditions
        .iter()
        .flat_map(|c| &c.pairs)
        .any(|p| p.displacement.is_none())
    {
        return (
            "unresolved_appearance",
            "At least one interval lacks two accepted appearances. Missing displacement is not zero motion.",
        );
    }
    if conditions
        .iter()
        .any(|c| distance(c.pairs[0].displacement.unwrap(), [0, 0]) > 1)
    {
        return (
            "idle_displacement",
            "The appearance also moved during an idle interval; the action comparison remains inconclusive.",
        );
    }
    if actions
        .iter()
        .any(|a| distance(a.displacement[0].unwrap(), [0, 0]) < 2)
    {
        return (
            "below_threshold",
            "At least one baseline action is below two normalized image pixels.",
        );
    }
    if actions
        .iter()
        .all(|a| a.changed == Some(true) && a.returned == Some(true))
    {
        (
            "appearance_displacement_changed_and_returned",
            "For all four action indices, the appearance displacement differed by at least three normalized pixels in the changed condition and returned within one pixel per axis of baseline in recovery.",
        )
    } else if actions
        .iter()
        .all(|a| a.changed == Some(false) && a.returned == Some(true))
    {
        (
            "similar_appearance_displacement",
            "All four action displacements stayed within one normalized pixel per axis of baseline in both later conditions.",
        )
    } else {
        (
            "mixed_appearance_displacement",
            "The action indices do not all support the same change-and-return pattern; the two-pixel threshold gap stays inconclusive.",
        )
    }
}

pub fn analyze(
    test: &Test,
    selection: Selection,
    mut read_image: impl FnMut(usize, usize) -> Result<Vec<u8>>,
    read_input: impl FnMut(usize, usize) -> Result<Option<Window>>,
) -> Result<Report> {
    trial_evidence::validate(test)?;
    let first = test.conditions[0]
        .frames
        .first()
        .ok_or("Retain baseline frame 0 before selecting an appearance.")?;
    if selection.baseline_sha256 != first.sha256 {
        return Err("The selected baseline image changed.".into());
    }
    let dimensions = [first.width, first.height];
    selection.region.validate(dimensions)?;
    let original = trial_evidence::check_image(read_image(0, 0)?, first)?;
    let record_sha256 = visual::digest(&serde_json::to_vec(test).map_err(|e| e.to_string())?);
    let source_sha256 = visual::digest(
        &serde_json::to_vec(&json!({"policy":POLICY,
        "matcher_policy":matcher::POLICY,"record_sha256":record_sha256,"selection":selection}))
        .map_err(|e| e.to_string())?,
    );
    let mut conditions = ["baseline", "changed", "recovered"].map(|name| Condition {
        name,
        frames: vec![],
        pairs: vec![],
    });
    let mut stopped = [false; 3];
    let mut attempts = 0;
    trial_evidence::visit(test, read_image, read_input, |pair| {
        let c = pair.condition;
        let source_error = pair.images.as_ref().err().cloned();
        let mut record = |index: usize, hash: &str, image: Result<&[u8]>| -> Frame {
            let mut frame = Frame {
                index,
                image_sha256: hash.into(),
                measurement: None,
                error: None,
                stopped: stopped[c],
            };
            if frame.stopped {
                return frame;
            }
            let measurement = image.and_then(|image| {
                attempts += 1;
                let report = matcher::measure(&original, image, selection.region)?;
                if report.source_dimensions != dimensions {
                    return Err("PNG dimensions disagree with the retained frame.".into());
                }
                Ok(report)
            });
            match measurement {
                Ok(m) => frame.measurement = Some(m),
                Err(e) => frame.error = Some(e.chars().take(512).collect()),
            }
            stopped[c] = frame.position().is_none();
            frame
        };
        if pair.index == 1 {
            conditions[c].frames.push(record(
                0,
                &pair.before.sha256,
                pair.images.as_ref().map(|(a, _)| *a).map_err(Clone::clone),
            ));
        }
        let after = record(
            pair.index,
            &pair.after.sha256,
            pair.images.as_ref().map(|(_, b)| *b).map_err(Clone::clone),
        );
        let before = conditions[c].frames.last().and_then(Frame::position);
        let displacement = before
            .zip(after.position())
            .map(|(a, b)| [b[0] - a[0], b[1] - a[1]]);
        conditions[c].frames.push(after);
        conditions[c].pairs.push(Pair {
            before_frame: pair.index - 1,
            after_frame: pair.index,
            input_sha256: pair.after.input_sha256.clone(),
            displacement,
            original_pixel_displacement: displacement.map(|d| {
                [
                    f64::from(d[0]) * f64::from(dimensions[0]) / 512.,
                    f64::from(d[1]) * f64::from(dimensions[1]) / 288.,
                ]
            }),
            source_error,
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
            let displacement = conditions
                .each_ref()
                .map(|c| c.pairs.get(index + 1).and_then(|p| p.displacement));
            Action {
                index,
                keys: action.keys.clone(),
                hold_ms: action.hold_ms,
                changed: displacement[0]
                    .zip(displacement[1])
                    .and_then(|(a, b)| changed(a, b)),
                returned: displacement[0]
                    .zip(displacement[2])
                    .and_then(|(a, b)| changed(a, b))
                    .map(|c| !c),
                displacement,
            }
        })
        .collect::<Vec<_>>();
    let (status, description) = outcome(&conditions, &actions);
    Ok(Report {
        policy: POLICY,
        matcher_policy: matcher::POLICY,
        test_id: test.id.clone(),
        record_sha256,
        source_sha256,
        selection,
        measurement_dimensions: [512, 288],
        attempted_frames: attempts,
        maximum_attempted_frames: 18,
        conditions,
        actions,
        status,
        description,
        limitation: LIMITATION,
    })
}
