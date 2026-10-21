//! This library has no evaluator, target implementation, or ground-truth API.
//! Identification is conditional on the declared experiment scope and model family.
pub mod discovery;
pub mod validation;

use discovery::{Candidate, discover};

/// Expand the unchanged inherited affine discovery to every byte alignment.
/// No layout, preferred width, or canonical candidate ordering is supplied.
pub fn discover_bytewise(snapshots: &[Vec<u8>], track: &[Option<f64>]) -> Vec<Candidate> {
    let mut result = Vec::new();
    for shift in 0..8 {
        let shifted: Vec<_> = snapshots
            .iter()
            .map(|s| s.get(shift..).unwrap_or_default().to_vec())
            .collect();
        for mut c in discover(&shifted, track) {
            c.offset += shift;
            if !result
                .iter()
                .any(|p: &Candidate| p.offset == c.offset && p.kind == c.kind)
            {
                result.push(c);
            }
        }
    }
    result.sort_by_key(|c| (c.offset, c.kind.width(), format!("{:?}", c.kind)));
    result
}
