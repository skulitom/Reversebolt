//! Bounded value-guided coverage, not a correspondence or an editable control.
//! Only the observed numeric value guides the scan; layouts and game identities
//! never participate. New trajectories must be collected after window selection.
use crate::{
    Result,
    coverage::{self, Chunk, Ranges},
    discovery::Kind,
    live::NativeTarget,
};
use serde::{Deserialize, Serialize};
use std::{collections::BinaryHeap, time::Instant};

pub const MAX_SCAN_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_WINDOWS: usize = 4096;
pub const WINDOW_BYTES: usize = 64;
pub const MAX_SEARCHES: usize = 8;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Summary {
    pub source_batch: usize,
    pub source_frame: usize,
    pub source_track: usize,
    pub source_label: String,
    pub source_frame_id: String,
    pub source_image_sha256: String,
    pub value: f64,
    pub eligible_bytes: u64,
    pub planned_scan_bytes: usize,
    pub attempted_bytes: u64,
    pub read_bytes: u64,
    pub changed_or_unavailable_bytes: u64,
    pub matched_windows: usize,
    pub retained_windows: usize,
    pub omitted_windows: usize,
    pub boundary_matches: usize,
    pub elapsed_ms: u128,
    pub stop: String,
    pub evidence: String,
}

pub struct Outcome {
    pub summary: Summary,
    pub windows: Vec<Chunk>,
    pub read_ranges: Vec<(u64, usize)>,
}

// Stable address mixing spreads a bounded sample over the scanned range instead
// of letting the first common-value region consume every retained window.
fn rank(mut address: u64) -> u64 {
    address = (address ^ (address >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    address = (address ^ (address >> 27)).wrapping_mul(0x94d049bb133111eb);
    address ^ (address >> 31)
}

struct Window {
    key: (u64, u64),
    chunk: Chunk,
}
impl PartialEq for Window {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Window {}
impl PartialOrd for Window {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Window {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

/// Scans at most `budget` bytes in 64 KiB requests, retaining addresses only.
/// Failed/partial/changed reads contribute no candidates. A 10-second cooperative
/// deadline is checked between native calls; it cannot interrupt a blocked call.
pub fn scan(
    target: &dyn NativeTarget,
    excluded: &Ranges,
    mut summary: Summary,
    budget: usize,
    max_windows: usize,
) -> Result<Outcome> {
    if !summary.value.is_finite() || summary.value.abs() > 1e12 {
        return Err("The observed number is outside the supported search range.".into());
    }
    if budget == 0 || budget > MAX_SCAN_BYTES || max_windows == 0 || max_windows > MAX_WINDOWS {
        return Err("Value search is outside its read or retained-window budget.".into());
    }
    let (plan, eligible) = coverage::plan(target.regions()?, excluded, budget)?;
    summary.eligible_bytes = eligible;
    summary.planned_scan_bytes = plan.iter().map(|c| c.length).sum();
    let mut p32 = vec![];
    if summary.value.fract() == 0.0
        && let Some(bytes) = Kind::I32.encode(summary.value)
    {
        p32.push(u32::from_le_bytes(bytes.try_into().unwrap()));
    }
    if let Some(bytes) = Kind::F32.encode(summary.value) {
        p32.push(u32::from_le_bytes(bytes.try_into().unwrap()));
    }
    let p64 = summary.value.to_le_bytes();
    let start = Instant::now();
    let mut windows: BinaryHeap<Window> = BinaryHeap::new();
    let mut read_ranges = vec![];
    let mut last_match = None;
    summary.stop = "planned range completed".into();
    for chunk in plan {
        if start.elapsed().as_secs() >= 10 {
            summary.stop = "time budget reached; scan incomplete".into();
            break;
        }
        if !target.alive() {
            summary.stop = "selected process exited; no transition".into();
            break;
        }
        let mapped = target
            .query(chunk.address)
            .is_ok_and(|r| r.is_some_and(|r| chunk.covered_by(&r)));
        if !mapped {
            summary.changed_or_unavailable_bytes += chunk.length as u64;
            continue;
        }
        summary.attempted_bytes += chunk.length as u64;
        let bytes = target.read(chunk.address, chunk.length);
        let unchanged = target
            .query(chunk.address)
            .is_ok_and(|r| r.is_some_and(|r| chunk.covered_by(&r)));
        let Ok(bytes) = bytes else {
            summary.changed_or_unavailable_bytes += chunk.length as u64;
            continue;
        };
        if bytes.len() != chunk.length || !unchanged {
            summary.changed_or_unavailable_bytes += chunk.length as u64;
            continue;
        }
        summary.read_bytes += chunk.length as u64;
        read_ranges.push((chunk.address, chunk.length));
        let first = ((4 - chunk.address % 4) % 4) as usize;
        for offset in (first..bytes.len().saturating_sub(3)).step_by(4) {
            let address = chunk.address + offset as u64;
            let match32 = p32.contains(&u32::from_le_bytes(
                bytes[offset..offset + 4].try_into().unwrap(),
            ));
            let match64 =
                address.is_multiple_of(8) && bytes.get(offset..offset + 8) == Some(p64.as_slice());
            if !match32 && !match64 {
                continue;
            }
            let base = address / WINDOW_BYTES as u64 * WINDOW_BYTES as u64;
            if last_match == Some(base) {
                continue;
            }
            last_match = Some(base);
            let window = Chunk {
                address: base,
                length: WINDOW_BYTES,
                region: chunk.region.clone(),
            };
            if !window.covered_by(&chunk.region) {
                summary.boundary_matches += 1;
                continue;
            }
            summary.matched_windows += 1;
            let key = (rank(base), base);
            if windows.len() < max_windows || windows.peek().is_some_and(|w| key < w.key) {
                if windows.len() == max_windows {
                    windows.pop();
                }
                windows.push(Window { key, chunk: window });
            }
        }
    }
    summary.elapsed_ms = start.elapsed().as_millis();
    summary.retained_windows = windows.len();
    summary.omitted_windows = summary.matched_windows.saturating_sub(windows.len());
    let mut windows: Vec<_> = windows.into_iter().map(|w| w.chunk).collect();
    windows.sort_by_key(|w| w.address);
    Ok(Outcome {
        summary,
        windows,
        read_ranges,
    })
}
