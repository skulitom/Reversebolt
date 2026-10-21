//! Compare concrete actions without selecting a canonical representation.
use reversebolt_causal_validation::discovery::Candidate;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Patch {
    pub offset: usize,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Group {
    /// None is a genuine byte-for-byte no-op, never an enabled edit.
    pub patch: Option<Patch>,
    pub interpretations: Vec<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Blocked {
    pub interpretation: usize,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Plan {
    pub groups: Vec<Group>,
    /// An unavailable interpretation stays unresolved; it cannot vote by omission.
    pub blocked: Vec<Blocked>,
    pub error: Option<String>,
}

impl Plan {
    /// The normalized write is a NEW action, requiring its own effect/recovery
    /// test. This says nothing about atomicity, authority, or hidden effects.
    pub fn agreement(&self) -> Option<&Patch> {
        if self.error.is_none() && self.blocked.is_empty() && self.groups.len() == 1 {
            self.groups[0].patch.as_ref()
        } else {
            None
        }
    }
}

/// Recompute for every request and baseline. Agreement at 60 does not imply
/// agreement at 64, even when both are inside an apparently useful slider range.
pub fn plan(
    bytes: &[u8],
    candidates: &[Candidate],
    observed: f64,
    desired: f64,
    tolerance: f64,
) -> Plan {
    let mut plan = Plan {
        groups: vec![],
        blocked: vec![],
        error: None,
    };
    if candidates.is_empty()
        || !observed.is_finite()
        || !desired.is_finite()
        || !tolerance.is_finite()
        || tolerance <= 0.0
    {
        plan.error = Some("missing interpretations or invalid numeric request".into());
        return plan;
    }
    for (index, c) in candidates.iter().enumerate() {
        let proposal = (|| -> Result<Option<Patch>, &str> {
            let end = c
                .offset
                .checked_add(c.kind.width())
                .ok_or("address overflow")?;
            let before = bytes.get(c.offset..end).ok_or("incomplete baseline")?;
            if !c.slope.is_finite() || c.slope.abs() < 1e-12 || !c.intercept.is_finite() {
                return Err("invalid mapping");
            }
            let estimate = c
                .kind
                .read(before)
                .map(|v| v * c.slope + c.intercept)
                .filter(|v| v.is_finite())
                .ok_or("unreadable baseline")?;
            if (estimate - observed).abs() > tolerance {
                return Err("baseline mismatch");
            }
            let after = c
                .encode_observation(desired)
                .ok_or("request cannot be encoded")?;
            let predicted = c
                .kind
                .read(&after)
                .map(|v| v * c.slope + c.intercept)
                .filter(|v| v.is_finite())
                .ok_or("unreadable encoding")?;
            if (predicted - desired).abs() > tolerance {
                return Err("encoding misses request");
            }
            // Canonicalize the resulting bytes, NOT the type or storage footprint.
            let Some(first) = before.iter().zip(&after).position(|(a, b)| a != b) else {
                return Ok(None);
            };
            let last = before
                .iter()
                .zip(&after)
                .rposition(|(a, b)| a != b)
                .unwrap();
            Ok(Some(Patch {
                offset: c.offset + first,
                before: before[first..=last].to_vec(),
                after: after[first..=last].to_vec(),
            }))
        })();
        match proposal {
            Ok(patch) => {
                if let Some(group) = plan.groups.iter_mut().find(|g| g.patch == patch) {
                    group.interpretations.push(index);
                } else {
                    plan.groups.push(Group {
                        patch,
                        interpretations: vec![index],
                    });
                }
            }
            Err(reason) => plan.blocked.push(Blocked {
                interpretation: index,
                reason: reason.into(),
            }),
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use reversebolt_causal_validation::discovery::Kind;
    fn candidate(offset: usize, kind: Kind, slope: f64, intercept: f64) -> Candidate {
        Candidate {
            offset,
            kind,
            slope,
            intercept,
            train_inliers: 10,
            validation_inliers: 10,
            exact: true,
        }
    }
    fn twins() -> Vec<Candidate> {
        vec![
            candidate(0, Kind::F32, 1., 0.),
            candidate(0, Kind::I32, 1. / 262144., 32. - 1107296256. / 262144.),
        ]
    }
    #[test]
    fn float_and_integer_agree_only_within_this_exponent_bin() {
        let bytes = 50f32.to_le_bytes();
        let same = plan(&bytes, &twins(), 50., 55., 0.01);
        assert_eq!(same.groups[0].interpretations, [0, 1]);
        assert_eq!(
            same.agreement().unwrap(),
            &Patch {
                offset: 2,
                before: vec![72],
                after: vec![92]
            }
        );
        let different = plan(&bytes, &twins(), 50., 70., 0.01);
        assert_eq!(different.groups.len(), 2);
        assert!(different.agreement().is_none());
    }
    #[test]
    fn shifted_footprints_can_encode_one_new_action() {
        let bytes = [0, 0, 4, 0, 0, 0];
        let models = [
            candidate(0, Kind::I32, 1. / 65536., 0.),
            candidate(2, Kind::I32, 1., 0.),
        ];
        let result = plan(&bytes, &models, 4., 7., 0.01);
        assert_eq!(
            result.agreement().unwrap(),
            &Patch {
                offset: 2,
                before: vec![4],
                after: vec![7]
            }
        );
        assert_eq!(result.groups[0].interpretations.len(), 2);
    }
    #[test]
    fn missing_or_unencodable_rivals_cannot_be_dropped() {
        let bytes = 50f32.to_le_bytes();
        let mut models = twins();
        models.push(candidate(usize::MAX, Kind::I32, 1., 0.));
        let result = plan(&bytes, &models, 50., 55., 0.01);
        assert_eq!(result.blocked.len(), 1);
        assert!(result.agreement().is_none());
        let result = plan(
            &50i32.to_le_bytes(),
            &[candidate(0, Kind::I32, 1., 0.)],
            50.,
            50.3,
            0.01,
        );
        assert_eq!(result.blocked.len(), 1);
    }
    #[test]
    fn no_op_and_stale_baseline_never_offer_an_edit() {
        let bytes = 50f32.to_le_bytes();
        let result = plan(&bytes, &twins(), 50., 50., 0.01);
        assert!(result.agreement().is_none());
        assert_eq!(result.groups.len(), 1);
        assert!(result.groups[0].patch.is_none());
        assert!(plan(&bytes, &twins(), 49., 55., 0.01).agreement().is_none());
        assert!(plan(&bytes, &twins(), 50., f64::NAN, 0.01).error.is_some());
    }
}
