//! Bounded memory batches. Coverage is over observed addresses, not object lifetimes.
use crate::Result;
use serde::{Deserialize, Serialize};

pub const CHUNK_BYTES: usize = 64 * 1024;
pub const MAX_BATCHES: usize = 24;
pub const MAX_READ_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const MAX_IMAGE_DISK_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_ARCHIVE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    pub base: u64,
    pub length: usize,
    pub image: bool,
    pub allocation_base: u64,
    pub protection: u32,
    pub kind: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chunk {
    pub address: u64,
    pub length: usize,
    pub region: Region,
}

impl Chunk {
    /// VirtualQueryEx at an interior address can describe a shorter suffix.
    /// Equal metadata does not prove that an allocation was never reused.
    pub fn covered_by(&self, region: &Region) -> bool {
        self.region.allocation_base == region.allocation_base
            && self.region.protection == region.protection
            && self.region.kind == region.kind
            && self.region.image == region.image
            && region.base <= self.address
            && region
                .base
                .checked_add(region.length as u64)
                .is_some_and(|end| {
                    self.address
                        .checked_add(self.length as u64)
                        .is_some_and(|last| last <= end)
                })
    }
}

/// Sorted, disjoint half-open intervals. Adjacent ranges merge; repeats do not
/// inflate coverage. Bounded by the selected batches, not target address space.
#[derive(Clone, Debug, Default)]
pub struct Ranges(Vec<(u64, u64)>);
impl Ranges {
    pub fn add(&mut self, address: u64, length: usize) {
        let Some(end) = address.checked_add(length as u64) else {
            return;
        };
        if address == end {
            return;
        }
        self.0.push((address, end));
        self.0.sort_unstable();
        let mut merged: Vec<(u64, u64)> = Vec::with_capacity(self.0.len());
        for &(start, end) in &self.0 {
            if let Some(last) = merged.last_mut()
                && start <= last.1
            {
                last.1 = last.1.max(end);
            } else {
                merged.push((start, end));
            }
        }
        self.0 = merged;
    }
    pub fn bytes(&self) -> u64 {
        self.0.iter().map(|(a, b)| b - a).sum()
    }
    fn gaps(&self, start: u64, end: u64) -> Vec<(u64, u64)> {
        let mut cursor = start;
        let mut out = vec![];
        for &(a, b) in &self.0 {
            if b <= cursor {
                continue;
            }
            if a >= end {
                break;
            }
            if a > cursor {
                out.push((cursor, a));
            }
            cursor = cursor.max(b);
        }
        if cursor < end {
            out.push((cursor, end));
        }
        out
    }
}

pub fn plan(
    mut regions: Vec<Region>,
    visited: &Ranges,
    budget: usize,
) -> Result<(Vec<Chunk>, u64)> {
    if regions.len() > 32768 {
        return Err("Memory inventory exceeded 32,768 eligible regions.".into());
    }
    regions.sort_by_key(|r| r.base);
    let mut end = 0;
    let mut eligible = 0u64;
    for r in &regions {
        if r.length == 0 || r.base < end {
            return Err("Invalid or overlapping memory inventory.".into());
        }
        end = r
            .base
            .checked_add(r.length as u64)
            .ok_or("Memory inventory overflow.")?;
        eligible = eligible
            .checked_add(r.length as u64)
            .ok_or("Memory size overflow.")?;
    }
    regions.sort_by_key(|r| (!r.image, r.length, r.base));
    let mut remaining = budget;
    let mut chunks = vec![];
    for region in regions {
        for (mut cursor, end) in visited.gaps(region.base, region.base + region.length as u64) {
            while cursor < end && remaining > 0 {
                let length = CHUNK_BYTES.min((end - cursor) as usize).min(remaining);
                chunks.push(Chunk {
                    address: cursor,
                    length,
                    region: region.clone(),
                });
                cursor += length as u64;
                remaining -= length;
            }
            if remaining == 0 {
                break;
            }
        }
        if remaining == 0 {
            break;
        }
    }
    Ok((chunks, eligible))
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Coverage {
    pub batch: usize,
    pub selected_unique_bytes: u64,
    pub read_unique_bytes: u64,
    pub complete_trajectory_bytes: u64,
    pub attempted_bytes: u64,
    pub read_bytes: u64,
    pub changed_or_unavailable_bytes: u64,
    pub image_disk_bytes: u64,
    /// Includes reserved space for a renderer that may finish after a timeout.
    pub image_budget_bytes: u64,
    pub archive_bytes: usize,
    pub analyzed_current_bytes: usize,
    pub batches: Vec<BatchSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchSummary {
    pub number: usize,
    pub captures: usize,
    pub planned_bytes: usize,
    pub complete_trajectory_bytes: usize,
    pub analyzed_bytes: usize,
    pub hypotheses: usize,
    pub retained_witnesses: usize,
    pub archive: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn region(base: u64, length: usize) -> Region {
        Region {
            base,
            length,
            allocation_base: base,
            ..Region::default()
        }
    }
    #[test]
    fn partial_windows_and_changing_maps_do_not_repeat_or_skip_unvisited_bytes() {
        let mut visited = Ranges::default();
        let regions = vec![region(100, 100), region(1000, 100)];
        let (first, total) = plan(regions.clone(), &visited, 65).unwrap();
        assert_eq!(total, 200);
        for c in &first {
            visited.add(c.address, c.length);
        }
        // A protection change at the start does not starve later addresses.
        let mut changed = regions.clone();
        changed[0].protection = 7;
        let (second, _) = plan(changed, &visited, 65).unwrap();
        assert_eq!((second[0].address, second[0].length), (165, 35));
        assert_eq!((second[1].address, second[1].length), (1000, 30));
        for c in second {
            visited.add(c.address, c.length);
        }
        assert_eq!(visited.bytes(), 130);
        let (last, _) = plan(regions, &visited, 200).unwrap();
        assert_eq!((last[0].address, last[0].length), (1030, 70));
        visited.add(120, 10);
        assert_eq!(visited.bytes(), 130);
    }
    #[test]
    fn suffix_queries_and_invalid_maps_are_checked() {
        let original = region(100, 100);
        let chunk = Chunk {
            address: 130,
            length: 20,
            region: original.clone(),
        };
        let mut suffix = original.clone();
        suffix.base = 130;
        suffix.length = 70;
        assert!(chunk.covered_by(&suffix));
        suffix.allocation_base = 101;
        assert!(!chunk.covered_by(&suffix));
        assert!(plan(vec![original, region(150, 100)], &Ranges::default(), 100).is_err());
        assert!(plan(vec![region(u64::MAX - 10, 20)], &Ranges::default(), 100).is_err());
    }
}
