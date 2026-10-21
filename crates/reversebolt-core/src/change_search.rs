//! Four broad observations select addresses only. All fitting and prospective
//! validation must use new observations after selection; no game layouts enter.
use crate::{Result, coverage::Chunk, discovery::Kind, visual::FrameRef};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, BinaryHeap},
    time::Instant,
};

pub const MOMENTS: usize = 4;
pub const MAX_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_SEARCHES: usize = 8;
const WIDTH: usize = crate::value_search::WINDOW_BYTES;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Summary {
    pub source_batch: usize,
    pub source_track: usize,
    pub source_frame: FrameRef,
    pub label: String,
    pub bounds: [f32; 4],
    pub selection_batch: usize,
    pub planned_bytes: usize,
    pub status: String,
    pub reason: Option<String>,
    pub observations: Vec<f64>,
    pub basis: Vec<FrameRef>,
    pub complete_bytes: usize,
    pub matched_windows: usize,
    pub retained_windows: usize,
    pub omitted_windows: usize,
    pub boundary_matches: usize,
    pub elapsed_ms: u128,
    pub evidence: String,
}

pub struct Selection {
    pub summary: Summary,
    pub windows: Vec<Chunk>,
    pub witnesses: Vec<Vec<Vec<u8>>>,
}

fn rank(mut address: u64) -> u64 {
    address = (address ^ (address >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    address = (address ^ (address >> 27)).wrapping_mul(0x94d049bb133111eb);
    address ^ (address >> 31)
}

// These hypotheses choose where to collect new data, never an editable mapping.
fn follows(values: [f64; MOMENTS], observed: &[f64], pair: (usize, usize)) -> bool {
    let (a, b) = pair;
    let dx = values[b] - values[a];
    if dx.abs() < 1e-9 {
        return false;
    }
    let slope = (observed[b] - observed[a]) / dx;
    let intercept = observed[a] - slope * values[a];
    slope.is_finite()
        && (1e-9..=1e9).contains(&slope.abs())
        && values
            .iter()
            .zip(observed)
            .all(|(x, y)| (slope * x + intercept - y).abs() <= 0.55)
}

pub fn select(
    plan: &[Chunk],
    snapshots: &[BTreeMap<u64, Vec<u8>>],
    invalid: &BTreeSet<u64>,
    mut summary: Summary,
    max_windows: usize,
) -> Result<Selection> {
    if snapshots.len() != MOMENTS
        || summary.observations.len() != MOMENTS
        || summary.basis.len() != MOMENTS
    {
        return Err("Collect four readable observations before selecting changing areas.".into());
    }
    if max_windows == 0
        || max_windows > crate::value_search::MAX_WINDOWS
        || plan.iter().map(|c| c.length).sum::<usize>() > MAX_BYTES
    {
        return Err("Change search exceeds its memory or window limit.".into());
    }
    let observed = &summary.observations;
    if observed.iter().any(|v| !v.is_finite() || v.abs() > 1e12) {
        return Err("The selected number is outside the supported range.".into());
    }
    let mut sorted = observed.clone();
    sorted.sort_by(f64::total_cmp);
    if sorted[2] - sorted[1] <= 2.2 {
        return Err(
            "The number needs repeated changes during these four moments. No areas were selected."
                .into(),
        );
    }
    let a = observed.iter().position(|v| *v == sorted[0]).unwrap();
    let b = observed.iter().position(|v| *v == sorted[3]).unwrap();
    let start = Instant::now();
    let mut retained: BinaryHeap<(u64, u64, usize)> = BinaryHeap::new();
    for (index, chunk) in plan.iter().enumerate() {
        if invalid.contains(&chunk.address) {
            continue;
        }
        let bytes: Option<Vec<_>> = snapshots
            .iter()
            .map(|s| s.get(&chunk.address).filter(|v| v.len() == chunk.length))
            .collect();
        let Some(bytes) = bytes else { continue };
        summary.complete_bytes += chunk.length;
        let mut last = None;
        let first = ((4 - chunk.address % 4) % 4) as usize;
        for offset in (first..chunk.length.saturating_sub(3)).step_by(4) {
            let address = chunk.address + offset as u64;
            let found = [Kind::I32, Kind::F32, Kind::F64].into_iter().any(|kind| {
                let width = kind.width();
                if !address.is_multiple_of(width as u64) || offset + width > chunk.length {
                    return false;
                }
                if bytes[1..]
                    .iter()
                    .all(|v| v[offset..offset + width] == bytes[0][offset..offset + width])
                {
                    return false;
                }
                let values: Option<Vec<_>> =
                    bytes.iter().map(|v| kind.read(&v[offset..])).collect();
                values.is_some_and(|v| follows(v.try_into().unwrap(), observed, (a, b)))
            });
            if !found {
                continue;
            }
            let base = address / WIDTH as u64 * WIDTH as u64;
            if last == Some(base) {
                continue;
            }
            last = Some(base);
            // A retained witness must be wholly present in a single read chunk.
            if base < chunk.address || base + WIDTH as u64 > chunk.address + chunk.length as u64 {
                summary.boundary_matches += 1;
                continue;
            }
            summary.matched_windows += 1;
            let key = (rank(base), base, index);
            if retained.len() < max_windows || retained.peek().is_some_and(|top| key < *top) {
                if retained.len() == max_windows {
                    retained.pop();
                }
                retained.push(key);
            }
        }
    }
    let mut retained = retained.into_vec();
    retained.sort_by_key(|(_, a, _)| *a);
    let mut windows = vec![];
    let mut witnesses = vec![];
    for (_, address, index) in retained {
        let chunk = &plan[index];
        let offset = (address - chunk.address) as usize;
        windows.push(Chunk {
            address,
            length: WIDTH,
            region: chunk.region.clone(),
        });
        witnesses.push(
            snapshots
                .iter()
                .map(|s| s[&chunk.address][offset..offset + WIDTH].to_vec())
                .collect(),
        );
    }
    summary.retained_windows = windows.len();
    summary.omitted_windows = summary.matched_windows.saturating_sub(windows.len());
    summary.elapsed_ms = start.elapsed().as_millis();
    summary.status = if windows.is_empty() {
        "no_selection"
    } else {
        "selected"
    }
    .into();
    Ok(Selection {
        summary,
        windows,
        witnesses,
    })
}
