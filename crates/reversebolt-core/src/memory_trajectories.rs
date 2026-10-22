//! Memory-only research proposals. Never semantic labels or editing authority.
//! Prefix selection is independent of later bytes, screen values and input labels.
use crate::{Result, discovery::Kind, memory_evidence::Archive};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const POLICY: &str = "memory_order_trajectories_v2";
pub const PREFIX: usize = 6;
pub const MIN_SAMPLES: usize = 12;
pub const MAX_GROUPS: usize = 128;
pub const MAX_PER_KIND: usize = 4;
pub const MAX_MEMBERS: usize = MAX_PER_KIND * 3;
pub const LIMITATION: &str = "Research proposals from aligned little-endian I32/F32/F64 views of retained sampled memory. No screen values, images, names or inputs enter selection. Original range selection can still be screen-guided. Order-pattern groups are coincident trajectories, not proven aliases or common objects. Numeric type, allocation identity, gameplay meaning, causal control and editing authority remain unverified. Constant fields, unaligned fields, other encodings, excluded numeric ranges and omitted proposals can be missed. Later consistency is descriptive on the same historical recording, not an independent game or causal test.";

type Signature = [u8; PREFIX];
type GroupKey = (u8, u8, u64, Signature);
type MemberKey = (u64, u64);
type Representatives = [BTreeMap<MemberKey, Member>; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Behavior {
    TwoLevels,
    Nondecreasing,
    Nonincreasing,
    Varying,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Selection {
    pub address: u64,
    pub kind: Kind,
    pub values: [f64; PREFIX],
    pub behavior: Behavior,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Consistent,
    Contradicted,
    Insufficient,
    NotPredicted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Assessment {
    pub values: Vec<Option<f64>>,
    pub missing_reads: usize,
    pub invalid_numbers: usize,
    pub checked: usize,
    pub violations: usize,
    pub verdict: Verdict,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    pub selection: Selection,
    pub assessment: Assessment,
    pub chunk_address: u64,
    pub chunk_invalidated_in_recording: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub order_signature: Signature,
    pub candidate_count: usize,
    pub omitted_members: usize,
    pub members: Vec<Member>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Coverage {
    pub planned_bytes: usize,
    pub complete_prefix_bytes: usize,
    pub missing_prefix_bytes: usize,
    pub typed_locations: usize,
    pub unchanged_bytes_locations: usize,
    pub invalid_numbers: usize,
    pub fewer_than_two_changes: usize,
    pub qualifying_candidates: usize,
    pub distinct_groups: usize,
    pub omitted_groups: usize,
    pub retained_members: usize,
    pub omitted_candidates: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub policy: String,
    pub manifest_sha256: String,
    pub investigation_id: String,
    pub batch: usize,
    pub selection_captures: Vec<usize>,
    pub assessment_captures: Vec<usize>,
    pub coverage: Coverage,
    pub groups: Vec<Group>,
    pub limitation: String,
}

fn mix(mut n: u64) -> u64 {
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    n ^ (n >> 31)
}

// Weak order, invariant to increasing affine transforms. Canonical reversal also
// groups opposing directions, but does not claim equality or a causal relation.
fn signature(values: &[f64; PREFIX]) -> Signature {
    let mut sorted = *values;
    sorted.sort_by(f64::total_cmp);
    let mut unique = 1;
    for i in 1..PREFIX {
        if sorted[i] != sorted[unique - 1] {
            sorted[unique] = sorted[i];
            unique += 1;
        }
    }
    let ranks = values.map(|v| sorted[..unique].iter().position(|x| *x == v).unwrap() as u8);
    let reversed = ranks.map(|r| unique as u8 - 1 - r);
    ranks.min(reversed)
}

fn behavior(values: &[f64; PREFIX]) -> Behavior {
    if *signature(values).iter().max().unwrap() == 1 {
        Behavior::TwoLevels
    } else if values.windows(2).all(|p| p[1] >= p[0]) {
        Behavior::Nondecreasing
    } else if values.windows(2).all(|p| p[1] <= p[0]) {
        Behavior::Nonincreasing
    } else {
        Behavior::Varying
    }
}

fn group_key(sig: Signature) -> GroupKey {
    let levels = *sig.iter().max().unwrap() + 1;
    let priority = if levels == 2 {
        0
    } else if sig.windows(2).all(|p| p[1] >= p[0]) || sig.windows(2).all(|p| p[1] <= p[0]) {
        1
    } else {
        2
    };
    (
        priority,
        levels,
        mix(sig.iter().fold(0u64, |n, r| n * PREFIX as u64 + *r as u64)),
        sig,
    )
}

fn decode(prefix: &[&[u8]], kind: Kind, offset: usize) -> Option<[f64; PREFIX]> {
    let mut values = [0.; PREFIX];
    for (i, bytes) in prefix.iter().enumerate() {
        values[i] = kind.read(&bytes[offset..])?;
    }
    Some(values)
}

fn assess(selection: &Selection, later: &[Option<Vec<u8>>], offset: usize) -> Assessment {
    let mut missing_reads = 0;
    let mut invalid_numbers = 0;
    let values: Vec<_> = later
        .iter()
        .map(|bytes| match bytes {
            None => {
                missing_reads += 1;
                None
            }
            Some(bytes) => {
                let value = selection.kind.read(&bytes[offset..]);
                invalid_numbers += usize::from(value.is_none());
                value
            }
        })
        .collect();
    let mut checked = 0;
    let mut violations = 0;
    let mut previous = Some(selection.values[PREFIX - 1]);
    for value in &values {
        if let Some(value) = value {
            match selection.behavior {
                Behavior::TwoLevels => {
                    checked += 1;
                    violations += usize::from(!selection.values.contains(value));
                }
                Behavior::Nondecreasing | Behavior::Nonincreasing => {
                    if let Some(prior) = previous {
                        checked += 1;
                        violations +=
                            usize::from(if selection.behavior == Behavior::Nondecreasing {
                                *value < prior
                            } else {
                                *value > prior
                            });
                    }
                }
                Behavior::Varying => {}
            }
        }
        // Missing data never bridges a monotonicity comparison across the gap.
        previous = *value;
    }
    let verdict = if selection.behavior == Behavior::Varying {
        Verdict::NotPredicted
    } else if violations > 0 {
        Verdict::Contradicted
    } else if checked < 3 {
        Verdict::Insufficient
    } else {
        Verdict::Consistent
    };
    Assessment {
        values,
        missing_reads,
        invalid_numbers,
        checked,
        violations,
        verdict,
    }
}

/// Analyze an immutable archive. No LiveSession, process, image or model is used.
/// Prefix-only selection prioritizes two-level and monotonic groups, then fewer
/// levels and a stable hash. Representatives use bottom-hash address sampling
/// separately for each numeric type, so abundant integer views cannot erase floats.
/// These are generic behavioral priors, not gameplay relevance. Later samples cannot
/// change selection, group membership, ordering or candidate counts.
pub fn analyze(archive: &Archive) -> Result<Report> {
    let manifest = archive.manifest();
    let captures: Vec<_> = manifest
        .samples
        .iter()
        .filter(|s| s.memory_sample)
        .map(|s| s.number)
        .collect();
    if captures.len() < MIN_SAMPLES {
        return Err(
            "Memory trajectory research needs at least twelve memory samples in one frozen batch"
                .into(),
        );
    }
    let mut coverage = Coverage::default();
    let mut counts: BTreeMap<Signature, usize> = BTreeMap::new();
    let mut retained: BTreeMap<GroupKey, Representatives> = BTreeMap::new();
    archive.visit_memory_chunks(|history| {
        let chunk = &history.chunk;
        coverage.planned_bytes += chunk.length;
        let Some(prefix) = history.bytes[..PREFIX]
            .iter()
            .map(Option::as_deref)
            .collect::<Option<Vec<_>>>()
        else {
            coverage.missing_prefix_bytes += chunk.length;
            return Ok(());
        };
        coverage.complete_prefix_bytes += chunk.length;
        for (kind_code, kind) in [Kind::I32, Kind::F32, Kind::F64].into_iter().enumerate() {
            let width = kind.width();
            let first = ((width as u64 - chunk.address % width as u64) % width as u64) as usize;
            for offset in (first..chunk.length.saturating_sub(width - 1)).step_by(width) {
                coverage.typed_locations += 1;
                if prefix[1..]
                    .iter()
                    .all(|b| b[offset..offset + width] == prefix[0][offset..offset + width])
                {
                    coverage.unchanged_bytes_locations += 1;
                    continue;
                }
                let Some(values) = decode(&prefix, kind, offset) else {
                    coverage.invalid_numbers += 1;
                    continue;
                };
                if values.windows(2).filter(|p| p[0] != p[1]).count() < 2 {
                    coverage.fewer_than_two_changes += 1;
                    continue;
                }
                coverage.qualifying_candidates += 1;
                let sig = signature(&values);
                *counts.entry(sig).or_default() += 1;
                let key = group_key(sig);
                if !retained.contains_key(&key) && retained.len() == MAX_GROUPS {
                    if key > *retained.last_key_value().unwrap().0 {
                        continue;
                    }
                    retained.pop_last();
                }
                let members = &mut retained.entry(key).or_default()[kind_code];
                let address = chunk.address + offset as u64;
                let member_key = (
                    mix(address ^ (kind_code as u64).wrapping_mul(0x9e3779b97f4a7c15)),
                    address,
                );
                if members.len() == MAX_PER_KIND {
                    if member_key > *members.last_key_value().unwrap().0 {
                        continue;
                    }
                    members.pop_last();
                }
                let selection = Selection {
                    address,
                    kind,
                    values,
                    behavior: behavior(&values),
                };
                let assessment = assess(&selection, &history.bytes[PREFIX..], offset);
                members.insert(
                    member_key,
                    Member {
                        selection,
                        assessment,
                        chunk_address: chunk.address,
                        chunk_invalidated_in_recording: manifest
                            .invalid_chunks
                            .contains(&chunk.address),
                    },
                );
            }
        }
        Ok(())
    })?;
    coverage.distinct_groups = counts.len();
    coverage.omitted_groups = counts.len() - retained.len();
    let groups: Vec<_> = retained
        .into_iter()
        .map(|((_, _, _, order_signature), members)| {
            let candidate_count = counts[&order_signature];
            let members: Vec<_> = members
                .into_iter()
                .flat_map(BTreeMap::into_values)
                .collect();
            Group {
                order_signature,
                candidate_count,
                omitted_members: candidate_count - members.len(),
                members,
            }
        })
        .collect();
    coverage.retained_members = groups.iter().map(|g| g.members.len()).sum();
    coverage.omitted_candidates = coverage.qualifying_candidates - coverage.retained_members;
    Ok(Report {
        policy: POLICY.into(),
        manifest_sha256: archive.manifest_sha256().into(),
        investigation_id: manifest.state.investigation_id.clone(),
        batch: manifest.state.coverage.batch,
        selection_captures: captures[..PREFIX].to_vec(),
        assessment_captures: captures[PREFIX..].to_vec(),
        coverage,
        groups,
        limitation: LIMITATION.into(),
    })
}
