//! Provisional, per-candidate experiment design; never execution authority.
use crate::{Result, runtime_hypotheses::Citation, runtime_priority, visual};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const POLICY: &str = "separate_measurement_and_metadata_v1";
pub const LIMITATION: &str = "A model-proposed experiment, not a verified effect or executable plan. Metadata can suggest a mechanism but cannot establish that its event occurred. Measurement compatibility and quoted observations do not prove causal relevance. Missing measurements require further investigation; candidates remain available.";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Input {
    pub evidence: runtime_priority::Input,
    pub question: String,
    pub explanation: String,
    pub selection_role: String,
}
impl Input {
    pub fn from_choice(plan: &Value, choice: &Value) -> Result<Self> {
        let input = Self {
            evidence: runtime_priority::Input::from_choices(
                plan["report_sha256"]
                    .as_str()
                    .ok_or("Missing report digest")?,
                &serde_json::from_value(plan["activity_context"].clone())
                    .map_err(|e| e.to_string())?,
                &serde_json::from_value(plan["observed_actions"].clone())
                    .map_err(|e| e.to_string())?,
                std::slice::from_ref(choice),
            )?,
            question: choice["question"]
                .as_str()
                .ok_or("Missing question")?
                .into(),
            explanation: choice["explanation"]
                .as_str()
                .ok_or("Missing explanation")?
                .into(),
            selection_role: choice["selection_role"]
                .as_str()
                .ok_or("Missing role")?
                .into(),
        };
        input.validate()?;
        Ok(input)
    }
    pub fn validate(&self) -> Result<()> {
        self.evidence.validate()?;
        if self.evidence.candidates.len() != 1
            || !text(&self.question, 1200)
            || !text(&self.explanation, 1200)
            || !matches!(self.selection_role.as_str(), "primary" | "alternative")
        {
            return Err(
                "Test design needs one exact candidate and its provisional question.".into(),
            );
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        Ok(visual::digest(
            &serde_json::to_vec(self).map_err(|e| e.to_string())?,
        ))
    }
    pub fn prompt_data(&self) -> Result<Value> {
        self.validate()?;
        let source = self.evidence.prompt_data()?;
        Ok(json!({
            "measured":{"image_contrast":source["measured_image_difference"],
                "sampled_input":source["repeat_sequence"],"motion_comparison":source["motion_comparison"]},
            "fallible_visual_interpretation":source["observation"],
            "unexecuted_metadata_clues":source["candidates"][0],
            "unverified_question":{"question":self.question,"explanation":self.explanation,
                "selected_path_role":self.selection_role,
                "alternative_warning":"An alternative path does not inherit the primary question's meaning."},
            "available_experiment":{"measurement":"whole-image response suppression after input, followed by return after restoration",
                "conditions":["unchanged","proposed value","original value restored"],
                "unavailable":["audio recording","event trace","physical motion measurement","identified object tracking"],
                "scope":"No new activity, timing, range or input meaning has been established."}
        }))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub status: String,
    pub readout: String,
    pub prediction: String,
    pub required_condition: String,
    pub condition_support: String,
    pub observation_quote: Option<String>,
    pub metadata: Citation,
    pub reason: String,
    pub next_observation: String,
    pub uncertainty: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Design {
    pub policy: String,
    pub input_sha256: String,
    pub proposal: Proposal,
}
fn text(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.chars().count() <= max && !s.chars().any(char::is_control)
}
pub fn accept(input: &Input, p: Proposal) -> Result<Design> {
    input.validate()?;
    let row = &input.evidence.candidates[0].row;
    let quoted = match (p.metadata.source.as_str(), p.metadata.index) {
        ("field_name", None) => Some(&row.field_name),
        ("type_name", None) => Some(&row.type_name),
        ("object_text", Some(i)) => row.object_text.get(i),
        _ => None,
    };
    if quoted != Some(&p.metadata.quote)
        || !matches!(
            p.status.as_str(),
            "current_probe"
                | "needs_other_observation"
                | "needs_other_activity"
                | "insufficient_evidence"
        )
        || !matches!(
            p.readout.as_str(),
            "rendered_response"
                | "local_visual_feature"
                | "audio_events"
                | "internal_events"
                | "unknown"
        )
        || !matches!(
            p.prediction.as_str(),
            "decreases" | "increases" | "changes" | "unknown"
        )
        || !matches!(
            p.condition_support.as_str(),
            "visual_interpretation" | "not_observed" | "uncertain"
        )
        || [
            &p.required_condition,
            &p.reason,
            &p.next_observation,
            &p.uncertainty,
        ]
        .iter()
        .any(|s| !text(s, 400))
        || p.observation_quote
            .as_ref()
            .is_some_and(|q| q != &input.evidence.activity.observation.description)
        || (p.condition_support == "visual_interpretation" && p.observation_quote.is_none())
        || (p.status == "current_probe"
            && (p.readout != "rendered_response"
                || p.prediction != "decreases"
                || p.condition_support != "visual_interpretation"
                || p.observation_quote.is_none()))
    {
        return Err(
            "Design has unsupported citations, categories or current-test compatibility.".into(),
        );
    }
    Ok(Design {
        policy: POLICY.into(),
        input_sha256: input.digest()?,
        proposal: p,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Input, Proposal) {
        let (context, frames, evidence) = crate::runtime_planning::tests::observed();
        let actions = crate::runtime_planning::actions(&context, &frames, &evidence).unwrap();
        let choice = json!({"selected_candidate":{"id":0,"type_id":0,"field_id":0,"path_id":"a".repeat(64),
            "type_name":"T","field_name":"F","value":2,"object_text":["event label, not an observation"]},
            "value":"0","question":"Does this affect the pattern?","explanation":"Unverified metadata clue.","selection_role":"primary"});
        let input = Input::from_choice(&json!({"report_sha256":"b".repeat(64),"activity_context":context,"observed_actions":actions}), &choice).unwrap();
        let p = Proposal {
            status: "current_probe".into(),
            readout: "rendered_response".into(),
            prediction: "decreases".into(),
            required_condition: "The pattern shifts.".into(),
            condition_support: "visual_interpretation".into(),
            observation_quote: Some(input.evidence.activity.observation.description.clone()),
            metadata: Citation {
                source: "field_name".into(),
                index: None,
                quote: "F".into(),
            },
            reason: "A possible discriminating probe.".into(),
            next_observation: "Repeat the same bounded input.".into(),
            uncertainty: "Meaning is unverified.".into(),
        };
        (input, p)
    }
    #[test]
    fn metadata_cannot_be_cited_as_an_observed_event() {
        let (input, p) = fixture();
        assert!(accept(&input, p.clone()).is_ok());
        let mut bad = p.clone();
        bad.observation_quote = Some(input.evidence.candidates[0].row.object_text[0].clone());
        assert!(accept(&input, bad).is_err());
        let mut bad = p;
        bad.metadata.index = Some(0);
        assert!(accept(&input, bad).is_err());
    }
    #[test]
    fn current_probe_requires_matching_measurement_direction_and_condition() {
        let (input, p) = fixture();
        for (readout, prediction, support) in [
            ("audio_events", "decreases", "visual_interpretation"),
            ("rendered_response", "increases", "visual_interpretation"),
            ("rendered_response", "decreases", "not_observed"),
        ] {
            let mut bad = p.clone();
            bad.readout = readout.into();
            bad.prediction = prediction.into();
            bad.condition_support = support.into();
            assert!(accept(&input, bad.clone()).is_err());
            bad.status = "needs_other_observation".into();
            assert!(accept(&input, bad).is_ok());
        }
    }
    #[test]
    fn design_binds_question_values_and_path_without_reordering_candidates() {
        let (input, p) = fixture();
        let design = accept(&input, p.clone()).unwrap();
        for mutate in 0..3 {
            let mut changed = input.clone();
            match mutate {
                0 => changed.question.push('?'),
                1 => changed.evidence.candidates[0].row.path_id = "c".repeat(64),
                _ => changed.evidence.candidates[0].row.value = json!(3),
            };
            assert_ne!(
                accept(&changed, p.clone()).unwrap().input_sha256,
                design.input_sha256
            );
        }
    }
}
