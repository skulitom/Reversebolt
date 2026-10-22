//! Propose comparison regions using only retained baseline images and inputs.
use crate::{region_appearance as matcher, response, trial_evidence};
use image::{
    GrayImage,
    imageops::{self, FilterType},
};
use reversebolt_core::{
    Result,
    inputs::Window,
    runtime_tests::{AppearanceSelection, InputWitness, Test},
    visual,
    visual_regions::Region,
};
use serde::Serialize;
use serde_json::json;

pub const POLICY: &str = "baseline_action_region_suggestions_v1";
pub const LIMITATION: &str = "Heuristic proposals from baseline evidence only. First-action image change minus idle change ranks fixed multiscale tiles; up to four candidates with limited overlap are checked against all six baseline images using the original template. At most twenty-four frame checks; stop each candidate at its first unresolved correspondence. Camera/background motion and identical appearances can pass. A suggestion is not a character identity, physical speed, gameplay meaning, causal control or edit authority. Nothing is selected or edited automatically. Later conditions are never read or used to rank suggestions.";
const WIDTH: u32 = 512;
const HEIGHT: u32 = 288;
const MAX_CANDIDATES: usize = 4;

#[derive(Debug, Serialize)]
pub struct PairEvidence {
    pub after_frame: usize,
    pub input: Option<InputWitness>,
    pub input_error: Option<String>,
    pub image_error: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Suggestion {
    pub selection: AppearanceSelection,
    pub action_minus_idle_change: f64,
    pub frames: Vec<matcher::Report>,
    pub displacement: [Option<[i32; 2]>; 5],
    pub status: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub policy: &'static str,
    pub matcher_policy: &'static str,
    pub test_id: String,
    pub baseline_record_sha256: String,
    pub source_sha256: String,
    pub image_sha256: Vec<String>,
    pub input_sha256: Vec<String>,
    pub measurement_dimensions: [u32; 2],
    pub evidence: Vec<PairEvidence>,
    pub status: &'static str,
    pub ranked_regions: usize,
    pub attempted_frames: usize,
    pub maximum_attempted_frames: usize,
    pub suggestions: Vec<Suggestion>,
    pub recommended: Option<AppearanceSelection>,
    pub limitation: &'static str,
}

fn resized(bytes: &[u8], dimensions: [u32; 2]) -> Result<GrayImage> {
    let gray = response::grayscale(bytes)?;
    if [gray.width(), gray.height()] != dimensions {
        return Err("PNG dimensions disagree with the baseline record.".into());
    }
    Ok(imageops::resize(&gray, WIDTH, HEIGHT, FilterType::Triangle))
}

fn overlap(a: Region, b: Region) -> bool {
    let w = (a.x + a.width)
        .min(b.x + b.width)
        .saturating_sub(a.x.max(b.x));
    let h = (a.y + a.height)
        .min(b.y + b.height)
        .saturating_sub(a.y.max(b.y));
    w * h * 4 > (a.width * a.height).min(b.width * b.height)
}

fn regions(images: &[GrayImage], dimensions: [u32; 2]) -> Vec<(f64, Region, Region)> {
    let mut integral = vec![0u32; ((WIDTH + 1) * (HEIGHT + 1)) as usize];
    let at = |x, y| (y * (WIDTH + 1) + x) as usize;
    for y in 0..HEIGHT {
        let mut row = 0;
        for x in 0..WIDTH {
            let idle = images[0][(x, y)][0].abs_diff(images[1][(x, y)][0]);
            let action = images[1][(x, y)][0].abs_diff(images[2][(x, y)][0]);
            row += u32::from(action.saturating_sub(idle));
            integral[at(x + 1, y + 1)] = integral[at(x + 1, y)] + row;
        }
    }
    let mut ranked = vec![];
    for side in [16, 32, 64] {
        for y in (0..=HEIGHT - side).step_by((side / 2) as usize) {
            for x in (0..=WIDTH - side).step_by((side / 2) as usize) {
                let sum = integral[at(x + side, y + side)] + integral[at(x, y)]
                    - integral[at(x + side, y)]
                    - integral[at(x, y + side)];
                let score = f64::from(sum) / f64::from(side * side);
                if score < 8. {
                    continue;
                }
                let left = u64::from(x) * u64::from(dimensions[0]) / u64::from(WIDTH);
                let top = u64::from(y) * u64::from(dimensions[1]) / u64::from(HEIGHT);
                let right =
                    (u64::from(x + side) * u64::from(dimensions[0])).div_ceil(u64::from(WIDTH));
                let bottom =
                    (u64::from(y + side) * u64::from(dimensions[1])).div_ceil(u64::from(HEIGHT));
                let region = Region {
                    x: left as u32,
                    y: top as u32,
                    width: (right - left) as u32,
                    height: (bottom - top) as u32,
                };
                if region.validate(dimensions).is_ok() {
                    ranked.push((
                        score,
                        region,
                        Region {
                            x,
                            y,
                            width: side,
                            height: side,
                        },
                    ));
                }
            }
        }
    }
    ranked.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then(a.2.width.cmp(&b.2.width))
            .then(a.2.y.cmp(&b.2.y))
            .then(a.2.x.cmp(&b.2.x))
    });
    ranked
}

pub fn analyze(
    test: &Test,
    read_image: impl FnMut(usize, usize) -> Result<Vec<u8>>,
    read_input: impl FnMut(usize, usize) -> Result<Option<Window>>,
) -> Result<Report> {
    // Later outcomes, saved assessments and choices never influence selection.
    let mut baseline = test.clone();
    baseline.conditions[1..].fill_with(Default::default);
    trial_evidence::validate(&baseline)?;
    let condition = &baseline.conditions[0];
    let baseline_record_sha256 =
        visual::digest(&serde_json::to_vec(condition).map_err(|e| e.to_string())?);
    let source_sha256 = visual::digest(
        &serde_json::to_vec(&json!({"policy":POLICY,"matcher":matcher::POLICY,
        "test_id":test.id,"investigation":test.investigation,"scope":test.scope,
        "baseline_record_sha256":baseline_record_sha256,"actions":test.actions}))
        .map_err(|e| e.to_string())?,
    );
    let mut report = Report {
        policy: POLICY,
        matcher_policy: matcher::POLICY,
        test_id: test.id.clone(),
        baseline_record_sha256,
        source_sha256,
        image_sha256: condition.frames.iter().map(|f| f.sha256.clone()).collect(),
        input_sha256: condition
            .frames
            .iter()
            .skip(1)
            .map(|f| f.input_sha256.clone())
            .collect(),
        measurement_dimensions: [WIDTH, HEIGHT],
        evidence: vec![],
        status: "incomplete_baseline",
        ranked_regions: 0,
        attempted_frames: 0,
        maximum_attempted_frames: 24,
        suggestions: vec![],
        recommended: None,
        limitation: LIMITATION,
    };
    if condition.frames.len() != 6 {
        return Ok(report);
    }
    let dimensions = [condition.frames[0].width, condition.frames[0].height];
    let mut images: [Option<GrayImage>; 6] = Default::default();
    trial_evidence::visit(&baseline, read_image, read_input, |pair| {
        let image_result = pair.images.and_then(|(a, b)| {
            if pair.index == 1 {
                images[0] = Some(resized(a, dimensions)?);
            }
            images[pair.index] = Some(resized(b, dimensions)?);
            Ok(())
        });
        report.evidence.push(PairEvidence {
            after_frame: pair.index,
            image_error: image_result.err(),
            input_error: pair.input.as_ref().err().cloned(),
            input: pair.input.ok(),
        });
    })?;
    if report
        .evidence
        .iter()
        .any(|p| p.image_error.is_some() || p.input_error.is_some())
    {
        report.status = "unavailable_evidence";
        return Ok(report);
    }
    let generations: std::collections::BTreeSet<_> = report
        .evidence
        .iter()
        .filter_map(|p| p.input.as_ref().map(|i| &i.generation))
        .collect();
    if generations.len() != 1
        || report
            .evidence
            .iter()
            .any(|p| p.input.as_ref().is_none_or(|i| !i.matches))
    {
        report.status = "input_mismatch";
        return Ok(report);
    }
    let images: Vec<_> = images
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("Missing verified baseline image.")?;
    let ranked = regions(&images, dimensions);
    report.ranked_regions = ranked.len();
    let mut chosen = vec![];
    for (score, region, tile) in ranked {
        if chosen.iter().any(|&r| overlap(tile, r)) {
            continue;
        }
        chosen.push(tile);
        let mut suggestion = Suggestion {
            selection: AppearanceSelection {
                baseline_sha256: condition.frames[0].sha256.clone(),
                region,
            },
            action_minus_idle_change: score,
            frames: vec![],
            displacement: [None; 5],
            status: "unresolved_appearance",
        };
        let mut positions = vec![];
        for (i, image) in images.iter().enumerate() {
            let measured = matcher::prepared(&images[0], image, region, dimensions);
            report.attempted_frames += 1;
            let position = (measured.status == matcher::Status::Matched)
                .then(|| measured.forward.as_ref().and_then(|s| s.best_at))
                .flatten();
            suggestion.frames.push(measured);
            let Some(p) = position else {
                break;
            };
            if let Some(previous) = positions.last() {
                let previous: &[u32; 2] = previous;
                suggestion.displacement[i - 1] = Some([
                    p[0] as i32 - previous[0] as i32,
                    p[1] as i32 - previous[1] as i32,
                ]);
            }
            positions.push(p);
        }
        if positions.len() == 6 {
            let magnitude = |d: [i32; 2]| d[0].abs().max(d[1].abs());
            suggestion.status = if magnitude(suggestion.displacement[0].unwrap()) > 1 {
                "idle_displacement"
            } else if suggestion.displacement[1..]
                .iter()
                .any(|d| magnitude(d.unwrap()) < 2)
            {
                "below_threshold"
            } else {
                "supported_baseline"
            };
        }
        if report.recommended.is_none() && suggestion.status == "supported_baseline" {
            report.recommended = Some(suggestion.selection.clone());
        }
        report.suggestions.push(suggestion);
        if chosen.len() == MAX_CANDIDATES {
            break;
        }
    }
    report.status = if report.recommended.is_some() {
        "suggestions_available"
    } else if report.suggestions.is_empty() {
        "no_action_regions"
    } else {
        "no_supported_region"
    };
    Ok(report)
}
