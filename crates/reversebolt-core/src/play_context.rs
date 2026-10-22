//! Bounded activity evidence for investigation, never a semantic control.
use crate::{
    Result,
    action_patterns::Exposure,
    play_evidence::Evidence,
    visual::{self, CaptureInterval, FrameRef},
};
use serde::{Deserialize, Serialize};

pub const LIMITATION: &str = "Two measured image contrasts associated with a sampled key, with the model's interpretation and uncertainty retained separately. Measured contrast can justify investigating an ambiguous response; it does not overrule the model or establish a consistent semantic change. No key meaning, game delivery, persistent actor, physical quantity, causal relationship to an internal field or edit authority is established. Metadata questions are generated independently; no returned hypothesis is proven to explain this response.";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub id: String,
    pub sha256: String,
    pub number: usize,
    pub width: u32,
    pub height: u32,
    pub interval: CaptureInterval,
}
impl Frame {
    fn from(frame: &FrameRef) -> Result<Self> {
        Ok(Self {
            id: frame.id.clone(),
            sha256: frame.sha256.clone(),
            number: frame.number,
            width: frame.width,
            height: frame.height,
            interval: frame
                .capture_interval
                .clone()
                .ok_or("Missing capture clock")?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pair {
    pub episode: usize,
    pub before: Frame,
    pub after: Frame,
    pub input_sha256: String,
    pub difference: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub investigation_id: String,
    pub batch: usize,
    pub frame_metadata_sha256: String,
    pub key: u8,
    pub pairs: [Pair; 2],
    pub quiet_difference: f64,
}
fn hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}
impl Source {
    pub fn validate(&self) -> Result<()> {
        if self.investigation_id.is_empty()
            || self.batch == 0
            || !hash(&self.frame_metadata_sha256)
            || self.key == 0
            || self.key == 255
            || (16..=18).contains(&self.key)
            || !self.quiet_difference.is_finite()
            || !(0. ..=255.).contains(&self.quiet_difference)
            || self.pairs[0].episode >= self.pairs[1].episode
            || self.pairs[0].after.number >= self.pairs[1].before.number
            || self.pairs[0].after.interval.end_us >= self.pairs[1].before.interval.begin_us
        {
            return Err("Invalid activity context scope or pair order.".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for pair in &self.pairs {
            if !hash(&pair.input_sha256)
                || !pair.difference.is_finite()
                || pair.difference > 255.
                || pair.difference < 5_f64.max(5. * self.quiet_difference)
                || pair.before.number.checked_add(1) != Some(pair.after.number)
                || pair.before.interval.end_us >= pair.after.interval.begin_us
                || pair
                    .after
                    .interval
                    .end_us
                    .saturating_sub(pair.before.interval.begin_us)
                    > 4_000_000
            {
                return Err("Activity context lacks a bounded contrasting pair.".into());
            }
            for frame in [&pair.before, &pair.after] {
                if frame.id.is_empty()
                    || !ids.insert(&frame.id)
                    || !hash(&frame.sha256)
                    || frame.interval.clock_id != self.investigation_id
                    || frame.interval.begin_us > frame.interval.end_us
                    || frame.width == 0
                    || frame.height == 0
                    || u64::from(frame.width) * u64::from(frame.height) > 4_000_000
                    || (frame.width, frame.height)
                        != (self.pairs[0].before.width, self.pairs[0].before.height)
                {
                    return Err("Activity context image references are inconsistent.".into());
                }
            }
        }
        Ok(())
    }
    pub fn frames(&self) -> Vec<&Frame> {
        self.pairs
            .iter()
            .flat_map(|p| [&p.before, &p.after])
            .collect()
    }
}
/// Select at most one sampled key and its first two eligible separated episodes.
/// Single-interval episodes keep the complete key window with each image pair.
pub fn select(
    frames: &[FrameRef],
    evidence: &Evidence,
    investigation: &str,
    batch: usize,
) -> Result<Option<Source>> {
    let mut actions: Vec<_> = evidence.actions.iter().collect();
    actions.sort_by(|a, b| {
        b.median_difference
            .unwrap_or(0.)
            .total_cmp(&a.median_difference.unwrap_or(0.))
            .then(a.key.cmp(&b.key))
    });
    for action in actions {
        let Some(quiet) = action.quiet_median_difference else {
            continue;
        };
        let mut pairs = vec![];
        for &episode in &action.bounded_episodes {
            let Some(e) = evidence.episodes.get(episode) else {
                continue;
            };
            if e.first_interval != e.last_interval || !e.bounded_by_quiet {
                continue;
            }
            let Some(difference) = e.median_difference else {
                continue;
            };
            if difference < 5_f64.max(5. * quiet) {
                continue;
            }
            let Some(a) = frames.get(e.first_interval) else {
                continue;
            };
            let Some(b) = frames.get(e.first_interval + 1) else {
                continue;
            };
            if crate::action_patterns::exposure(a, b) != Ok(Exposure::Key(action.key)) {
                continue;
            }
            pairs.push(Pair {
                episode,
                before: Frame::from(a)?,
                after: Frame::from(b)?,
                difference,
                input_sha256: visual::digest(
                    &serde_json::to_vec(&b.input_context).map_err(|e| e.to_string())?,
                ),
            });
            if pairs.len() == 2 {
                break;
            }
        }
        if let Ok(pairs) = pairs.try_into() {
            let source = Source {
                investigation_id: investigation.into(),
                batch,
                key: action.key,
                pairs,
                quiet_difference: quiet,
                frame_metadata_sha256: visual::digest(
                    &serde_json::to_vec(frames).map_err(|e| e.to_string())?,
                ),
            };
            source.validate()?;
            return Ok(Some(source));
        }
    }
    Ok(None)
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub consistent_visible_change: bool,
    pub description: String,
    pub uncertainty: String,
}
impl Observation {
    pub fn validate(&self) -> Result<()> {
        if [&self.description, &self.uncertainty]
            .iter()
            .any(|s| s.trim().is_empty() || s.len() > 400 || s.chars().any(char::is_control))
        {
            return Err("Invalid activity interpretation text.".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    #[default]
    Interpreted,
    MeasuredContrastV1,
}
impl Basis {
    fn is_interpreted(&self) -> bool {
        *self == Self::Interpreted
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterpretationSource {
    pub review_id: String,
    /// Recorded producer of the image interpretation, independent of later hypothesis authors.
    pub model: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Context {
    /// Legacy positive interpretations retain their original serialized shape.
    #[serde(default, skip_serializing_if = "Basis::is_interpreted")]
    pub basis: Basis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpretation_source: Option<InterpretationSource>,
    /// Correspondences actually supplied to this interpretation, never semantic proof.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<crate::activity_motion::Summary>,
    pub source: Source,
    pub observation: Observation,
}
impl Context {
    pub fn motion_comparison(&self) -> Result<Option<crate::motion_comparison::Comparison>> {
        self.validate()?;
        self.motion
            .as_ref()
            .map(|motion| crate::motion_comparison::compare(&self.source, motion))
            .transpose()
    }

    pub fn from_observation(source: Source, observation: Observation) -> Result<Self> {
        let basis = if observation.consistent_visible_change {
            Basis::Interpreted
        } else {
            Basis::MeasuredContrastV1
        };
        let context = Self {
            basis,
            interpretation_source: None,
            motion: None,
            source,
            observation,
        };
        context.validate()?;
        Ok(context)
    }

    pub fn bind_interpretation(mut self, review_id: &str, model: &str) -> Result<Self> {
        if self.interpretation_source.is_some() {
            return Err("The image interpretation already has a recorded source.".into());
        }
        self.interpretation_source = Some(InterpretationSource {
            review_id: review_id.into(),
            model: model.into(),
        });
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<()> {
        self.source.validate()?;
        self.observation.validate()?;
        if let Some(motion) = &self.motion {
            motion.validate(&self.source)?;
        }
        if let Some(source) = &self.interpretation_source
            && (!uuid::Uuid::parse_str(&source.review_id)
                .is_ok_and(|id| id.to_string() == source.review_id)
                || source.model.trim().is_empty()
                || source.model.len() > 128
                || source.model.chars().any(char::is_control))
        {
            return Err("Invalid image interpretation source.".into());
        }
        if (self.basis == Basis::Interpreted) != self.observation.consistent_visible_change {
            return Err("The activity basis disagrees with the retained interpretation.".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        action_patterns::tests::fixture,
        play_evidence::{Measurement, analyze},
    };
    #[test]
    fn requires_two_separated_complete_contrasting_episodes() {
        let (mut frames, _) = fixture(&[0, 68, 0, 68, 0], &[0.; 5]);
        for f in &mut frames {
            f.sha256 = "a".repeat(64);
        }
        let evidence = analyze(
            &frames,
            [0., 12., 0., 15., 0.]
                .into_iter()
                .map(|v| Measurement {
                    difference: Some(v),
                    error: None,
                })
                .collect(),
        )
        .unwrap();
        let source = select(&frames, &evidence, "clock", 1).unwrap().unwrap();
        assert_eq!(source.key, 68);
        assert_eq!(source.pairs[0].before.id, "f1");
        assert_eq!(source.pairs[1].after.id, "f4");
        assert_eq!(source.frames().len(), 4);
        let mut invalid = source.clone();
        invalid.pairs[1].after.sha256 = "forged".into();
        assert!(invalid.validate().is_err());
        invalid = source;
        invalid.pairs[1].before.interval.clock_id = "other".into();
        assert!(invalid.validate().is_err());
        frames[2].input_context = None;
        assert!(select(&frames, &evidence, "clock", 1).unwrap().is_none());
    }
    #[test]
    fn sustained_activity_and_drift_do_not_supply_a_context() {
        for (keys, values) in [
            ([0, 68, 68, 68, 0], [0., 10., 10., 10., 0.]),
            ([0, 68, 0, 68, 0], [10.; 5]),
        ] {
            let (mut frames, _) = fixture(&keys, &[0.; 5]);
            for f in &mut frames {
                f.sha256 = "a".repeat(64);
            }
            let e = analyze(
                &frames,
                values
                    .into_iter()
                    .map(|v| Measurement {
                        difference: Some(v),
                        error: None,
                    })
                    .collect(),
            )
            .unwrap();
            assert!(select(&frames, &e, "clock", 1).unwrap().is_none());
        }
    }
}
