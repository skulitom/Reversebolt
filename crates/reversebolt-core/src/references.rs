//! Learned one-hop references from captured bytes. No symbols, known layouts,
//! game offsets or pointer-chain assumptions are supplied to this search.
use crate::{Result, coverage::Chunk, live::NativeTarget};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_OFFSET: u64 = 4096;
pub const MAX_ROUTES: usize = 4;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reference {
    pub root: Chunk,
    pub offset: u64,
    pub learned_pointer: u64,
    pub learned_captures: usize,
}

impl Reference {
    /// Query/read/query detects observed remapping, not allocation generations.
    pub fn resolve(&self, target: &dyn NativeTarget) -> Result<u64> {
        let compatible = || {
            target
                .query(self.root.address)
                .map(|r| r.is_some_and(|r| self.root.covered_by(&r)))
        };
        if self.root.length != 8 || self.offset > MAX_OFFSET || !compatible()? {
            return Err("The learned reference's memory mapping is unavailable.".into());
        }
        let bytes = target.read(self.root.address, 8)?;
        if !compatible()? {
            return Err("The learned reference changed mapping during its read.".into());
        }
        self.destination(&bytes)
            .ok_or_else(|| "The learned reference is null, incomplete or overflows.".into())
    }

    pub fn destination(&self, bytes: &[u8]) -> Option<u64> {
        let pointer = u64::from_le_bytes(bytes.try_into().ok()?);
        (pointer != 0).then_some(pointer)?.checked_add(self.offset)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Learned {
    pub routes: Vec<Reference>,
    /// More stable references existed than were retained. Never exhaustive.
    pub limited: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resolution {
    pub candidate: usize,
    pub root: u64,
    pub offset: u64,
    pub address: Option<u64>,
    pub error: Option<String>,
}

/// Search aligned little-endian 64-bit words in complete captured chunks. A
/// route must have pointed to the same candidate with the same offset in every
/// fitting sample. Retain at most four, ordered by offset then root address.
/// This does not prove pointer semantics or identify an object.
pub fn learn(
    chunks: &[&Chunk],
    snapshots: &[BTreeMap<u64, Vec<u8>>],
    candidates: &[(u64, usize)],
) -> Vec<Learned> {
    let mut result = vec![Learned::default(); candidates.len()];
    if snapshots.len() < 12 {
        return result;
    }
    let mut addresses: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (i, (address, _)) in candidates.iter().enumerate() {
        addresses.entry(*address).or_default().push(i);
    }
    for chunk in chunks {
        let Some(latest) = snapshots.last().and_then(|s| s.get(&chunk.address)) else {
            continue;
        };
        let alignment = ((8 - chunk.address % 8) % 8) as usize;
        for offset in (alignment..latest.len().saturating_sub(7)).step_by(8) {
            let bytes = &latest[offset..offset + 8];
            let pointer = u64::from_le_bytes(bytes.try_into().unwrap());
            if pointer == 0 {
                continue;
            }
            for (&address, indices) in addresses.range(pointer..=pointer.saturating_add(MAX_OFFSET))
            {
                let root = chunk.address + offset as u64;
                if !snapshots.iter().all(|s| {
                    s.get(&chunk.address)
                        .and_then(|b| b.get(offset..offset + 8))
                        == Some(bytes)
                }) {
                    continue;
                }
                for &i in indices {
                    if root < address.saturating_add(candidates[i].1 as u64)
                        && address < root.saturating_add(8)
                    {
                        continue;
                    }
                    let found = &mut result[i];
                    found.routes.push(Reference {
                        root: Chunk {
                            address: root,
                            length: 8,
                            region: chunk.region.clone(),
                        },
                        offset: address - pointer,
                        learned_pointer: pointer,
                        learned_captures: snapshots.len(),
                    });
                    found.routes.sort_by_key(|r| (r.offset, r.root.address));
                    if found.routes.len() > MAX_ROUTES {
                        found.routes.pop();
                        found.limited = true;
                    }
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coverage::Region;
    #[test]
    fn learns_offsets_only_from_stable_complete_evidence_and_caps_alternatives() {
        let chunk = Chunk {
            address: 0x10000,
            length: 128,
            region: Region::default(),
        };
        let mut snapshots = vec![BTreeMap::from([(chunk.address, vec![0; 128])]); 12];
        for (n, s) in snapshots.iter_mut().enumerate() {
            let b = s.get_mut(&chunk.address).unwrap();
            for i in 0..6 {
                b[i * 8..i * 8 + 8].copy_from_slice(&0x20000u64.to_le_bytes());
            }
            b[64..72].copy_from_slice(&(0x20000u64 + n as u64).to_le_bytes());
            b[80..88].copy_from_slice(&0x1E000u64.to_le_bytes());
        }
        let found = learn(&[&chunk], &snapshots, &[(0x20040, 4), (0x40000, 4)]);
        assert_eq!(found[0].routes.len(), 4);
        assert!(found[0].limited);
        assert!(
            found[0]
                .routes
                .iter()
                .all(|r| r.offset == 64 && r.learned_captures == 12)
        );
        assert!(found[1].routes.is_empty());
        assert!(
            learn(&[&chunk], &snapshots[..11], &[(0x20040, 4)])[0]
                .routes
                .is_empty()
        );
        assert_eq!(
            found[0].routes[0].destination(&0x50000u64.to_le_bytes()),
            Some(0x50040)
        );
        assert_eq!(found[0].routes[0].destination(&0u64.to_le_bytes()), None);
        assert_eq!(
            found[0].routes[0].destination(&u64::MAX.to_le_bytes()),
            None
        );
    }
}
