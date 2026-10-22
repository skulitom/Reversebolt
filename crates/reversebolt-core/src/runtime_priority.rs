//! Evidence-bound comparison ordering. No candidate deletion or edit authority.
use crate::{
    Result, play_context::Context, runtime_hypotheses::Row, runtime_planning::ObservedActions,
    visual,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub const POLICY: &str = "observed_response_priority_v1";
pub const LIMITATION: &str = "A provisional model preference for testing suppression of the recorded rendered response. It does not validate a label, action meaning, safe range, controlling state or effect. Unranked candidates remain available; they are not disproven or unsuitable for other activities. Existing input, value and restoration guards still apply.";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub row: Row,
    pub proposed_value: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Input {
    pub report_sha256: String,
    pub activity: Context,
    pub observed_actions: ObservedActions,
    pub candidates: Vec<Candidate>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub order: Vec<usize>,
    pub reason: String,
    pub uncertainty: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Preference {
    pub policy: String,
    pub input_sha256: String,
    pub proposal: Proposal,
}

impl Input {
    pub fn from_choices(
        report: &str,
        activity: &Context,
        actions: &ObservedActions,
        choices: &[Value],
    ) -> Result<Self> {
        let input = Self {
            report_sha256: report.into(),
            activity: activity.clone(),
            observed_actions: actions.clone(),
            candidates: choices
                .iter()
                .map(|c| {
                    Ok(Candidate {
                        row: serde_json::from_value(c["selected_candidate"].clone())
                            .map_err(|e| e.to_string())?,
                        proposed_value: c["value"]
                            .as_str()
                            .ok_or("Missing comparison value")?
                            .into(),
                    })
                })
                .collect::<Result<_>>()?,
        };
        input.validate()?;
        Ok(input)
    }
    pub fn validate(&self) -> Result<()> {
        self.activity.validate()?;
        let hash = |s: &str| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit());
        if !hash(&self.report_sha256)
            || self.candidates.len() > 60
            || self.observed_actions.frame_metadata_sha256
                != self.activity.source.frame_metadata_sha256
            || self.observed_actions.actions.len() != 4
            || self.candidates.iter().any(|c| {
                !hash(&c.row.path_id) || !matches!(c.proposed_value.as_str(), "0" | "false")
            })
            || self
                .candidates
                .iter()
                .map(|c| &c.row.path_id)
                .collect::<BTreeSet<_>>()
                .len()
                != self.candidates.len()
        {
            return Err(
                "Comparison preference input has inconsistent scope, candidates or actions.".into(),
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
        // Anonymous symbols preserve repetition without assuming a key's meaning.
        let mut keys = vec![];
        let actions: Vec<_> = self
            .observed_actions
            .actions
            .iter()
            .map(|a| {
                let id = keys.iter().position(|k| k == &a.keys).unwrap_or_else(|| {
                    keys.push(a.keys.clone());
                    keys.len() - 1
                });
                json!({"symbol":id,"hold_ms":a.hold_ms})
            })
            .collect();
        let motion=self.activity.motion_comparison()?.map(|m|json!({"relation":m.relation,
            "pairs":m.pairs.iter().map(|p|json!({"displacement":p.displacement,"selected":p.selected,
                "matched":p.matched,"matched_cells":p.matched_cells,"error":p.error})).collect::<Vec<_>>()}));
        Ok(
            json!({"observation":self.activity.observation,"basis":self.activity.basis,
            "measured_image_difference":{"action_pairs":self.activity.source.pairs.each_ref().map(|p|p.difference),
                "quiet":self.activity.source.quiet_difference,"units":"image intensity difference, not physical motion"},
            "motion_comparison":motion,"repeat_sequence":actions,
            "reviewed_pair_symbol":keys.iter().position(|k|k==&[self.activity.source.key]),
            "candidates":self.candidates.iter().enumerate().map(|(i,c)|json!({"candidate":i,
                "field_name":c.row.field_name,"type_name":c.row.type_name,"object_text":c.row.object_text,
                "observed_value":c.row.value,"proposed_value":c.proposed_value})).collect::<Vec<_>>()}),
        )
    }
}

pub fn accept(input: &Input, proposal: Proposal) -> Result<Preference> {
    let valid_text = |s: &str| {
        !s.trim().is_empty() && s.chars().count() <= 400 && !s.chars().any(char::is_control)
    };
    if proposal.order.len() > 8
        || proposal.order.iter().collect::<BTreeSet<_>>().len() != proposal.order.len()
        || proposal.order.iter().any(|i| *i >= input.candidates.len())
        || !valid_text(&proposal.reason)
        || !valid_text(&proposal.uncertainty)
    {
        return Err("Comparison preference must cite distinct supplied candidates and retain bounded reasoning and uncertainty.".into());
    }
    Ok(Preference {
        policy: POLICY.into(),
        input_sha256: input.digest()?,
        proposal,
    })
}

pub fn apply(input: &Input, preference: &Preference, choices: &mut [Value]) -> Result<()> {
    if accept(input, preference.proposal.clone())? != *preference
        || Input::from_choices(
            &input.report_sha256,
            &input.activity,
            &input.observed_actions,
            choices,
        )? != *input
    {
        return Err("Comparison preference belongs to different evidence or choices.".into());
    }
    let preferred: Vec<_> = preference
        .proposal
        .order
        .iter()
        .map(|i| &input.candidates[*i].row.path_id)
        .collect();
    choices.sort_by_key(|c| {
        preferred
            .iter()
            .position(|id| c["selected_candidate"]["path_id"] == **id)
            .unwrap_or(preferred.len())
    });
    Ok(())
}

#[cfg(test)]
#[path = "runtime_priority_tests.rs"]
mod tests;
