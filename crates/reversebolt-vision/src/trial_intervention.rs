//! Join already-measured regional response to the exact retained edit records.
use crate::trial_appearance;
use reversebolt_core::{Result, runtime_tests::Test, runtime_trials::Trial, visual};
use serde::Serialize;

pub const POLICY: &str = "recorded_regional_intervention_v1";
pub const LIMITATION: &str = "Recorded edit/readback/restoration evidence combined with a region saved before the edit. The result describes these image observations, not a verified gameplay name, object identity, delivered action, physical speed, complete recovery or exclusive causal explanation. Other state changes, camera/background motion and identical appearances can confound the result. No authority or saved scalar assessment is changed.";

#[derive(Debug, Serialize)]
pub struct Assessment {
    pub policy: &'static str,
    pub test_id: String,
    pub change_id: String,
    pub test_sha256: String,
    pub trial_sha256: Option<String>,
    pub appearance_report_sha256: String,
    pub appearance_status: &'static str,
    pub intervention_error: Option<String>,
    pub status: &'static str,
    pub reason: String,
    pub limitation: &'static str,
}

/// `appearance` must be the report just produced from engine-owned saved files.
/// No caller-supplied report is accepted by the application/MCP interface.
pub fn assess(
    test: &Test,
    trial: Option<&Trial>,
    appearance: &trial_appearance::Report,
) -> Result<Assessment> {
    let test_sha256 = visual::digest(&serde_json::to_vec(test).map_err(|e| e.to_string())?);
    if appearance.test_id != test.id
        || appearance.record_sha256 != test_sha256
        || appearance.policy != trial_appearance::POLICY
        || appearance.matcher_policy != crate::region_appearance::POLICY
        || appearance.retained_plan != test.appearance
        || test
            .appearance
            .as_ref()
            .is_some_and(|p| p.selection != appearance.selection)
    {
        return Err(
            "Regional measurements do not belong to this exact retained test and choice.".into(),
        );
    }
    let intervention_error = trial
        .ok_or_else(|| "The linked runtime edit record is unavailable.".to_string())
        .and_then(|trial| test.check_recorded_intervention(trial))
        .err();
    let (status, reason) = if let Some(error) = &intervention_error {
        ("inconclusive", error.clone())
    } else {
        match appearance.status {
            "appearance_displacement_changed_and_returned" => (
                "regional_change_and_return_observed",
                "The preselected region changed its action displacement after the recorded edit and returned after recorded restoration, with matching value readbacks.".into(),
            ),
            "similar_appearance_displacement" => (
                "regional_change_not_observed",
                "The recorded edit and restoration have matching value readbacks, but this region's action displacement stayed similar.".into(),
            ),
            _ => ("inconclusive", format!("The edit records are supported, but the image comparison is {}: {}", appearance.status, appearance.description)),
        }
    };
    Ok(Assessment {
        policy: POLICY,
        test_id: test.id.clone(),
        change_id: test.change_id.clone(),
        test_sha256,
        trial_sha256: trial
            .map(|t| serde_json::to_vec(t).map(|b| visual::digest(&b)))
            .transpose()
            .map_err(|e| e.to_string())?,
        appearance_report_sha256: visual::digest(
            &serde_json::to_vec(appearance).map_err(|e| e.to_string())?,
        ),
        appearance_status: appearance.status,
        intervention_error,
        status,
        reason,
        limitation: LIMITATION,
    })
}
