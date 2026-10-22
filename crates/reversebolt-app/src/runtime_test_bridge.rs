//! Shared live response experiments. Input is external or explicitly planned; capture and
//! evidence are engine-owned. A screen contrast never promotes a semantic name.
use crate::{runtime_bridge, runtime_trial_bridge};
use reversebolt_core::{
    Command, Result,
    changes::now_ms,
    engine::{Engine, View},
    runtime::{Task, scalar_value},
    runtime_patch::{Patch, encode},
    runtime_tests::{self, Action, Frame, InputWitness, Phase, Test},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

fn appearance_suggestions(engine: &Engine, test: &Test, result: &mut Value) {
    match reversebolt_vision::trial_appearance_suggestions::analyze(
        test,
        |condition, frame| engine.runtime_tests.read_image(test, condition, frame),
        |condition, frame| engine.runtime_tests.read_input(test, condition, frame),
    ) {
        Ok(report) => result["appearance_suggestions"] = json!(report),
        Err(error) => {
            result["appearance_suggestions"] = Value::Null;
            result["appearance_suggestions_error"] = json!(error);
        }
    }
}

fn compare_appearance(
    engine: &Engine,
    test: &Test,
    selection: runtime_tests::AppearanceSelection,
) -> Result<(
    reversebolt_vision::trial_appearance::Report,
    reversebolt_vision::trial_intervention::Assessment,
)> {
    let report = reversebolt_vision::trial_appearance::analyze(
        test,
        selection,
        |condition, frame| engine.runtime_tests.read_image(test, condition, frame),
        |condition, frame| engine.runtime_tests.read_input(test, condition, frame),
    )?;
    let trial = engine.runtime_trials.get(&test.change_id).ok();
    let assessment = reversebolt_vision::trial_intervention::assess(test, trial.as_ref(), &report)?;
    Ok((report, assessment))
}

fn completed_regional_assessment(engine: &Engine, test: &Test, result: &mut Value) {
    if test.phase != Phase::Complete {
        return;
    }
    let Some(plan) = &test.appearance else {
        return;
    };
    match compare_appearance(engine, test, plan.selection.clone()) {
        Ok((_, assessment)) => result["regional_intervention_assessment"] = json!(assessment),
        Err(error) => {
            result["regional_intervention_assessment"] = Value::Null;
            result["trial_appearance_error"] = json!(error);
        }
    }
}

/// Add current regional outcomes to the exact prior tests selected by the core
/// planner. Never relabel an untried choice or mutate the retained experiment.
pub(crate) fn comparison_history(engine: &Engine, choices: &mut [Value]) -> Result<Value> {
    regional_history(&engine.runtime_tests.records, choices, |test| {
        let selection = test.appearance.as_ref().unwrap().selection.clone();
        let (_, assessment) = compare_appearance(engine, test, selection)?;
        Ok(json!(assessment))
    })
}

fn regional_history(
    tests: &[Test],
    choices: &mut [Value],
    mut evaluate: impl FnMut(&Test) -> Result<Value>,
) -> Result<Value> {
    let mut outcomes = std::collections::BTreeMap::<String, Result<Value>>::new();
    let mut checked = vec![];
    let mut unavailable = vec![];
    let mut omitted = std::collections::BTreeSet::new();
    for choice in choices {
        let prior = &choice["previous_test"];
        if prior.is_null() {
            continue;
        }
        let id = prior["id"]
            .as_str()
            .ok_or("Previous comparison lacks its saved identity.")?;
        let test = tests
            .iter()
            .find(|t| t.id == id)
            .ok_or("Previous comparison is no longer available.")?;
        if *prior != test.summary()
            || choice["hypothesis_ref"]["selected_path_id"] != test.candidate.id
            || choice["value"] != test.value
            || choice["actions"] != json!(test.actions)
        {
            return Err(
                "Previous comparison changed or belongs to a different proposed action/path."
                    .into(),
            );
        }
        if test.phase != Phase::Complete || test.appearance.is_none() {
            continue;
        }
        let id = id.to_string();
        for key in [
            "regional_intervention_assessment",
            "regional_intervention_error",
            "status_basis",
        ] {
            choice
                .as_object_mut()
                .ok_or("Comparison choice must be an object.")?
                .remove(key);
        }
        choice["whole_screen_status"] = json!(
            test.assessment
                .as_ref()
                .map_or("inconclusive", |a| a.status.as_str())
        );
        if !outcomes.contains_key(&id) {
            if outcomes.len() == runtime_tests::MAX_TESTS_PER_INVESTIGATION {
                omitted.insert(id);
                choice["status"] = json!("inconclusive");
                choice["status_basis"] = json!("regional_not_checked");
                choice["regional_intervention_error"] = json!(
                    "The regional history review budget is exhausted; this result was not rechecked."
                );
                continue;
            }
            let result = evaluate(test).and_then(|assessment| {
                if assessment["test_id"] != test.id
                    || assessment["change_id"] != test.change_id
                    || assessment["test_sha256"]
                        != reversebolt_core::visual::digest(
                            &serde_json::to_vec(test).map_err(|e| e.to_string())?,
                        )
                    || assessment["policy"] != reversebolt_vision::trial_intervention::POLICY
                    || !matches!(
                        assessment["status"].as_str(),
                        Some(
                            "regional_change_and_return_observed"
                                | "regional_change_not_observed"
                                | "inconclusive"
                        )
                    )
                {
                    return Err("Regional assessment does not match this comparison.".into());
                }
                Ok(assessment)
            });
            checked.push(id.clone());
            if result.is_err() {
                unavailable.push(id.clone());
            }
            outcomes.insert(id.clone(), result);
        }
        match &outcomes[&id] {
            Ok(assessment) => {
                choice["status"] = assessment["status"].clone();
                choice["status_basis"] = json!("regional_intervention");
                choice["regional_intervention_assessment"] = assessment.clone();
            }
            Err(error) => {
                choice["status"] = json!("inconclusive");
                choice["status_basis"] = json!("regional_unavailable");
                choice["regional_intervention_error"] =
                    json!(error.chars().take(512).collect::<String>());
            }
        }
    }
    Ok(json!({"policy":"regional_comparison_history_v1",
        "maximum_tests":runtime_tests::MAX_TESTS_PER_INVESTIGATION,
        "maximum_template_checks":runtime_tests::MAX_TESTS_PER_INVESTIGATION*18,
        "checked_test_ids":checked,"analysis_error_test_ids":unavailable,"omitted_test_ids":omitted}))
}

/// Called only after the engine has saved all six baseline images/input windows.
/// This also serves target-free protocol tests of the real post-capture path.
fn review_baseline(engine: &mut Engine, test: &mut Test, abort: &AtomicBool) -> Result<()> {
    if test.phase != Phase::BaselineReview {
        return Ok(());
    }
    if abort.load(Ordering::SeqCst) {
        return Err("The desktop stopped the baseline review.".into());
    }
    if engine
        .runtime_tests
        .get(&test.id)
        .is_ok_and(|saved| !matches!(saved.phase, Phase::Baseline | Phase::BaselineReview))
    {
        return Err("The saved test is no longer collecting or reviewing its baseline.".into());
    }
    if engine.runtime_trials.get(&test.change_id).is_ok()
        || engine.runtime_trials.holds(&test.scope)
    {
        return Err("The baseline cannot be selected after preparing an edit or while a write is unresolved.".into());
    }
    // Make the complete baseline durable before adding its immutable choice.
    // A restart here stops the test; it never resumes or guesses a selection.
    engine.runtime_tests.record(test.clone())?;
    let report = reversebolt_vision::trial_appearance_suggestions::analyze(
        test,
        |condition, frame| engine.runtime_tests.read_image(test, condition, frame),
        |condition, frame| engine.runtime_tests.read_input(test, condition, frame),
    )?;
    if abort.load(Ordering::SeqCst) {
        return Err("The desktop stopped the baseline review.".into());
    }
    test.finish_baseline_review(report.recommended, test.revision, now_ms())?;
    // Persist the exact acceptance before a late stop/expiry changes its phase.
    // The outer capture completion can then retain the choice while stopping.
    engine.runtime_tests.record(test.clone())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Prepare {
    test_id: String,
    change_id: String,
    investigation_id: String,
    catalog_id: String,
    after_revision: u64,
    candidate_id: Option<String>,
    value: String,
    hypothesis: Option<String>,
    hypothesis_ref: Option<reversebolt_core::runtime_hypotheses::Reference>,
    actions: Vec<Action>,
}
pub struct Session {
    pub abort: Arc<AtomicBool>,
    monitor: Option<Arc<AtomicBool>>,
    deadline: Option<Instant>,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.cancel();
    }
}
impl Session {
    pub fn new(abort: Arc<AtomicBool>) -> Self {
        Self {
            abort,
            monitor: None,
            deadline: None,
        }
    }
    fn cancel(&mut self) {
        if let Some(cancel) = self.monitor.take() {
            cancel.store(true, Ordering::SeqCst);
        }
        self.deadline = None;
    }
    pub fn stop(&mut self, engine: &mut Engine, reason: &str) -> Result<()> {
        self.cancel();
        let ids: Vec<_> = engine
            .runtime_tests
            .records
            .iter()
            .filter(|t| t.active())
            .map(|t| t.id.clone())
            .collect();
        for id in ids {
            let mut t = engine.runtime_tests.get(&id)?.clone();
            t.stop(reason);
            if let Err(error) = engine.runtime_tests.record(t.clone()) {
                if let Some(record) = engine.runtime_tests.records.iter_mut().find(|r| r.id == id) {
                    *record = t;
                }
                return Err(error);
            }
        }
        Ok(())
    }
    pub fn maintain(&mut self, engine: &mut Engine) -> Result<bool> {
        let Some(t) = engine.runtime_tests.records.iter().find(|t| t.active()) else {
            self.cancel();
            return Ok(false);
        };
        let reason = if self.abort.load(Ordering::SeqCst) {
            Some("The desktop stopped this comparison. The linked edit remains in history.")
        } else if self.monitor.is_none()
            || self.deadline.is_none_or(|d| Instant::now() >= d)
            || now_ms()
                .checked_sub(t.created_unix_ms)
                .is_none_or(|age| age > runtime_tests::LIFETIME_MS)
        {
            Some(
                "The experiment expired or its input sampler stopped. Check the linked edit for recovery.",
            )
        } else if engine.live.as_ref().is_none_or(|s| {
            !s.view.connected
                || s.view.investigation_id != t.investigation
                || s.view.runtime.catalog_id != t.catalog_id
                || s.view.process.window != t.window
        }) {
            Some("The application or catalogue changed. The comparison stopped.")
        } else if engine.view.freezes.iter().any(|f| f.active()) {
            Some("A Freeze operation can affect this comparison. The test stopped.")
        } else {
            None
        };
        if let Some(reason) = reason {
            self.stop(engine, reason)?;
            return Ok(true);
        }
        Ok(false)
    }
    pub fn before(&mut self, engine: &mut Engine, c: &Command) -> Result<()> {
        if matches!(
            c.action.as_str(),
            "set_runtime_value" | "apply_runtime_change"
        ) && let Some(t) = engine
            .runtime_tests
            .records
            .iter()
            .find(|t| c.args["change_id"] == t.change_id)
        {
            let replay = engine
                .runtime_trials
                .get(&t.change_id)
                .is_ok_and(|r| r.write.is_some());
            if !replay && (!t.active() || t.phase != Phase::ReadyToApply || t.pending_capture) {
                return Err(
                    "This experiment has no complete baseline ready for the planned edit.".into(),
                );
            }
            if c.action == "set_runtime_value"
                && (c.args["candidate_id"] != t.candidate.id
                    || c.args["investigation_id"] != t.investigation
                    || c.args["catalog_id"] != t.catalog_id
                    || c.args["value"]
                        .as_str()
                        .and_then(|v| encode(t.candidate.type_code, v).ok())
                        != encode(t.candidate.type_code, &t.value).ok())
            {
                return Err("The edit differs from the experiment's immutable plan.".into());
            }
        }
        let Some(t) = engine.runtime_tests.records.iter().find(|t| t.active()) else {
            return Ok(());
        };
        let linked = matches!(
            c.action.as_str(),
            "set_runtime_value" | "apply_runtime_change" | "restore_runtime_change"
        ) && c.args["change_id"] == t.change_id;
        if !linked
            && !matches!(
                c.action.as_str(),
                "prepare_runtime_test"
                    | "lock_runtime_test_appearance"
                    | "observe_runtime_test"
                    | "stop_runtime_test"
                    | "read_runtime_test"
                    | "status"
                    | "history"
                    | "list_processes"
                    | "list_memory_processes"
                    | "search_runtime_values"
                    | "plan_runtime_comparisons"
                    | "read_runtime_inventory"
                    | "read_capture"
                    | "read_sequence"
                    | "read_play_evidence"
                    | "read_change_evidence"
            )
        {
            self.stop(engine,"Another operation took over this investigation. The test stopped; its edit remains in history.")?;
        }
        Ok(())
    }
    pub fn after(&mut self, engine: &mut Engine) -> Result<()> {
        if engine.runtime_tests.fault.is_some() {
            self.stop(engine,"Experiment storage failed; sampling stopped. The edit remains independently undoable.")?;
        }
        if let Some(mut t) = engine
            .runtime_tests
            .records
            .iter()
            .find(|t| t.active())
            .cloned()
            && let Ok(trial) = engine.runtime_trials.get(&t.change_id)
        {
            if let Err(error) = t.sync_trial(&trial) {
                t.stop(&error);
            }
            let old = engine.runtime_tests.get(&t.id)?;
            if t.revision != old.revision || t.phase != old.phase {
                engine.runtime_tests.record(t)?;
            }
        }
        if !engine.runtime_tests.records.iter().any(|t| t.active()) {
            self.cancel();
        }
        Ok(())
    }
    pub fn command(
        &mut self,
        engine: &mut Engine,
        c: &Command,
        base: &Path,
        published: &Arc<RwLock<View>>,
    ) -> Result<Value> {
        if c.action == "prepare_runtime_test" {
            return self.prepare(engine, c, base, published);
        }
        let id = c.args["test_id"]
            .as_str()
            .ok_or("Choose an experiment from the shared session.")?;
        let mut test = engine.runtime_tests.get(id)?.clone();
        if c.action == "read_runtime_test" {
            if c.args
                .get("suggest_appearance")
                .is_some_and(|v| v.as_bool().is_none())
            {
                return Err("suggest_appearance must be true or false.".into());
            }
            let origin_motion = test
                .origin
                .as_ref()
                .and_then(|origin| origin.play_context.as_ref())
                .map(|context| context.motion_comparison())
                .transpose()?
                .flatten();
            let mut result = json!({"test":test,"origin_activity_motion_comparison":origin_motion,"executions":engine.runtime_tests.read_executions(&test)?,"limitation":runtime_tests::LIMITATION});
            if c.args
                .get("suggest_appearance")
                .and_then(Value::as_bool)
                .unwrap_or(test.phase == Phase::ReadyToApply && test.appearance.is_none())
            {
                appearance_suggestions(engine, &test, &mut result);
            }
            if c.args
                .get("include_motion")
                .is_some_and(|v| v.as_bool().is_none())
            {
                return Err("include_motion must be true or false.".into());
            }
            if c.args["include_motion"] == true {
                result["trial_motion_comparison"] =
                    serde_json::to_value(reversebolt_vision::trial_motion::analyze(
                        &test,
                        |condition, frame| engine.runtime_tests.read_image(&test, condition, frame),
                        |condition, frame| engine.runtime_tests.read_input(&test, condition, frame),
                    )?)
                    .map_err(|e| e.to_string())?;
            }
            let requested: Option<runtime_tests::AppearanceSelection> = c
                .args
                .get("appearance")
                .map(|selection| {
                    serde_json::from_value(selection.clone()).map_err(|_| {
                        "Supply the baseline frame 0 SHA-256 and an integer image region."
                            .to_string()
                    })
                })
                .transpose()?;
            let selection = test
                .appearance
                .as_ref()
                .map(|p| p.selection.clone())
                .or_else(|| requested.clone());
            if let Some(selection) = selection {
                if requested.as_ref().is_some_and(|r| *r != selection) {
                    return Err("The requested appearance differs from the retained plan.".into());
                }
                match compare_appearance(engine, &test, selection) {
                    Ok((report, assessment)) => {
                        result["regional_intervention_assessment"] = json!(assessment);
                        result["trial_appearance_comparison"] =
                            serde_json::to_value(report).map_err(|e| e.to_string())?
                    }
                    Err(error) if test.appearance.is_some() => {
                        result["regional_intervention_assessment"] = Value::Null;
                        result["trial_appearance_comparison"] = Value::Null;
                        result["trial_appearance_error"] = json!(error);
                    }
                    Err(error) => return Err(error),
                }
            }
            if c.args.get("condition").is_some() || c.args.get("frame").is_some() {
                let condition = c.args["condition"]
                    .as_u64()
                    .filter(|v| *v < 3)
                    .ok_or("Choose condition 0, 1 or 2.")? as usize;
                let frame = c.args["frame"]
                    .as_u64()
                    .filter(|v| *v < 6)
                    .ok_or("Choose frame 0 through 5.")? as usize;
                let image = engine.runtime_tests.read_image(&test, condition, frame)?;
                let input = engine.runtime_tests.read_input(&test, condition, frame)?;
                use base64::Engine as _;
                result["png_base64"] =
                    json!(base64::engine::general_purpose::STANDARD.encode(&image));
                result["input"] = json!(input);
                if frame > 0 {
                    let measured = (|| -> Result<_> {
                        let prior = engine
                            .runtime_tests
                            .read_image(&test, condition, frame - 1)?;
                        reversebolt_vision::patch_motion::measure(&prior, &image)
                    })();
                    let (measurement, error) = match measured {
                        Ok(report) => (Some(report), None),
                        Err(error) => (None, Some(error)),
                    };
                    result["patch_motion"] = json!({"condition":condition,"before_frame":frame-1,"after_frame":frame,
                        "before_sha256":test.conditions[condition].frames[frame-1].sha256,
                        "after_sha256":test.conditions[condition].frames[frame].sha256,
                        "measurement":measurement,"error":error});
                }
            }
            return Ok(result);
        }
        if c.action == "stop_runtime_test" {
            if test.active() {
                test.stop(
                    "Stopped by the user or agent. The linked edit remains available for Undo.",
                );
                self.cancel();
                engine.runtime_tests.record(test.clone())?;
            }
            return Ok(test.summary());
        }
        if c.source != test.actor && c.source != "desktop" {
            return Err("This experiment belongs to another controller.".into());
        }
        let revision = c.args["after_revision"]
            .as_u64()
            .ok_or("Supply the exact current experiment revision.")?;
        if c.action == "lock_runtime_test_appearance" {
            let selection: runtime_tests::AppearanceSelection =
                serde_json::from_value(c.args["appearance"].clone()).map_err(|_| {
                    "Supply the baseline frame 0 SHA-256 and an integer image region.".to_string()
                })?;
            let changed = test.lock_appearance(selection, revision, now_ms())?;
            if changed {
                if engine.runtime_trials.get(&test.change_id).is_ok()
                    || engine.runtime_trials.holds(&test.scope)
                {
                    return Err(
                        "Choose the appearance before preparing or applying the linked edit."
                            .into(),
                    );
                }
                let bytes = engine.runtime_tests.read_image(&test, 0, 0)?;
                let image = image::ImageReader::new(std::io::Cursor::new(bytes))
                    .with_guessed_format()
                    .map_err(|e| e.to_string())?
                    .into_dimensions()
                    .map_err(|e| e.to_string())?;
                let source = &test.conditions[0].frames[0];
                if image != (source.width, source.height) {
                    return Err("The baseline PNG dimensions disagree with its record.".into());
                }
                engine.runtime_tests.record(test.clone())?;
                checkpoint(engine, published)?;
            }
            return Ok(json!({"test_id":test.id,"appearance":test.appearance,"replayed":!changed}));
        }
        let perform_action = match c.args.get("perform_action") {
            None => false,
            Some(v) => v
                .as_bool()
                .ok_or("Choose whether to perform the planned action.")?,
        };
        let (condition, index) = test.begin_capture(revision)?;
        engine.runtime_tests.record(test.clone())?;
        checkpoint(engine, published)?;
        let result = self.capture(engine, &mut test, condition, index, base, perform_action);
        if let Err(error) = &result {
            test.stop(error);
            self.cancel();
        }
        if !test.active() {
            self.cancel();
        }
        engine.runtime_tests.record(test.clone())?;
        result?;
        let mut summary = test.summary();
        if test.phase == Phase::ReadyToApply && test.appearance.is_none() {
            appearance_suggestions(engine, &test, &mut summary);
        }
        completed_regional_assessment(engine, &test, &mut summary);
        Ok(summary)
    }
    fn prepare(
        &mut self,
        engine: &mut Engine,
        c: &Command,
        base: &Path,
        published: &Arc<RwLock<View>>,
    ) -> Result<Value> {
        let p: Prepare = serde_json::from_value(c.args.clone()).map_err(|e| e.to_string())?;
        if let Ok(t) = engine.runtime_tests.get(&p.test_id) {
            if t.change_id != p.change_id
                || t.investigation != p.investigation_id
                || t.catalog_id != p.catalog_id
                || t.prepared_revision != p.after_revision
                || p.candidate_id
                    .as_ref()
                    .is_some_and(|id| *id != t.candidate.id)
                || t.value != p.value
                || t.actions != p.actions
                || p.hypothesis.as_ref().is_some_and(|h| *h != t.hypothesis)
                || t.origin.as_ref().map(|o| &o.reference) != p.hypothesis_ref.as_ref()
                || (p.hypothesis_ref.is_none()
                    && (p.hypothesis.is_none() || p.candidate_id.is_none()))
                || t.actor != c.source
            {
                return Err("This experiment identity belongs to a different plan.".into());
            }
            return Ok(t.summary());
        }
        engine.runtime_tests.ensure_room(&p.investigation_id)?;
        if engine
            .runtime_tests
            .records
            .iter()
            .any(|t| t.active() || t.change_id == p.change_id)
            || engine.runtime_trials.get(&p.change_id).is_ok()
            || engine.view.freezes.iter().any(|f| f.active())
        {
            return Err("Finish the active test/Freeze first, and use new comparison and change identities.".into());
        }
        let session = engine
            .live
            .as_mut()
            .ok_or("Connect an application first.")?;
        session.refresh_connection();
        if !session.view.connected
            || !session.view.runtime.available
            || session.view.runtime.phase == "uncertain"
            || session.view.investigation_id != p.investigation_id
        {
            return Err("The application changed.".into());
        }
        if session.view.runtime.catalog_id != p.catalog_id
            || session.view.runtime.revision != p.after_revision
        {
            return Err("The runtime catalogue or revision changed.".into());
        }
        let mut staged = session.view.runtime.clone();
        let (candidate_id, hypothesis, origin) = if let Some(reference) = &p.hypothesis_ref {
            let report = runtime_bridge::catalog_report(&staged)?;
            let inventory = reversebolt_core::runtime_inventory::build(&report)?;
            let review = staged
                .naming
                .as_ref()
                .ok_or("No current hypothesis review is available.")?;
            let origin =
                reversebolt_core::runtime_hypotheses::resolve(review, reference, &inventory)?;
            let id = origin.selected()?.path_id.clone();
            let question = origin.hypothesis.proposal.question.clone();
            if p.candidate_id.as_ref().is_some_and(|given| *given != id)
                || p.hypothesis
                    .as_ref()
                    .is_some_and(|given| *given != question)
            {
                return Err(
                    "The supplied candidate or question conflicts with the selected hypothesis."
                        .into(),
                );
            }
            if !staged.cards.iter().any(|c| c.candidate.id == id) {
                let candidate = reversebolt_core::runtime::candidate(&report, &id)?;
                let previous_search = staged.search.take();
                staged.search = Some(reversebolt_core::runtime::SearchPage {
                    query: String::new(),
                    offset: 0,
                    matches: 1,
                    candidates: vec![candidate],
                });
                staged.show_values(&p.catalog_id, p.after_revision, std::slice::from_ref(&id))?;
                staged.search = previous_search;
            }
            (id, question, Some(origin))
        } else {
            (
                p.candidate_id
                    .clone()
                    .ok_or("Choose a shown candidate or a retained hypothesis.")?,
                p.hypothesis
                    .clone()
                    .ok_or("Supply a short question for this comparison.")?,
                None,
            )
        };
        let task = Task::Refresh {
            catalog_id: p.catalog_id.clone(),
            after_revision: staged.revision,
            ids: vec![candidate_id.clone()],
        };
        let paths = staged.paths(&task)?;
        let candidate = staged
            .cards
            .iter()
            .find(|c| c.candidate.id == candidate_id)
            .ok_or("Show this candidate as a card first.")?
            .candidate
            .clone();
        let info = session
            .view
            .memory_process
            .as_ref()
            .unwrap_or(&session.view.process)
            .clone();
        let mut test = Test {
            id: p.test_id,
            change_id: p.change_id,
            actor: c.source.clone(),
            investigation: p.investigation_id,
            catalog_id: p.catalog_id,
            scope: runtime_bridge::runtime_scope(&info),
            window: session.view.process.window,
            candidate,
            value: p.value,
            hypothesis,
            origin,
            appearance: None,
            actions: p.actions,
            prepared_revision: p.after_revision,
            created_unix_ms: now_ms(),
            preparation: None,
            metric: runtime_tests::METRIC_ID.into(),
            revision: 0,
            phase: Phase::Baseline,
            pending_capture: false,
            conditions: Default::default(),
            assessment: None,
            reason: "Collecting a baseline before the planned edit.".into(),
        };
        test.validate_plan()?;
        if engine.runtime_trials.holds(&test.scope) || engine.changes.holds_process(&info) {
            return Err("An unresolved edit holds this process.".into());
        }
        // Only an explicit, fully checked test choice adds an off-board candidate.
        // Its metadata label is unchanged; a model proposal never renames it.
        engine.live.as_mut().unwrap().view.runtime = staged;
        let start = now_ms();
        let outcome = runtime_bridge::run(&info, &paths, base, &AtomicBool::new(false))
            .and_then(runtime_trial_bridge::evidence);
        let evidence = match outcome {
            Ok(e) => e,
            Err(error) => {
                engine.live.as_mut().unwrap().view.runtime.phase = "uncertain".into();
                return Err(error);
            }
        };
        let now = now_ms();
        let patch = Patch::prepare(
            paths[0].clone(),
            &evidence.report["checks"][0],
            &test.value,
            now,
        )?;
        if patch.type_code != test.candidate.type_code {
            return Err("The candidate representation changed.".into());
        }
        test.candidate.bytes = patch.expected;
        test.candidate.value = scalar_value(patch.type_code, &test.candidate.bytes)?;
        engine.live.as_mut().unwrap().view.runtime.apply_refresh(
            &task,
            &evidence.report,
            [start, now],
            &evidence.path,
        )?;
        test.preparation = Some(evidence);
        engine.runtime_tests.record(test.clone())?;
        if self.abort.load(Ordering::SeqCst) {
            test.stop("The desktop took control while preparation settled. No sampling started.");
            engine.runtime_tests.record(test)?;
            return Err("Test preparation was interrupted by the desktop.".into());
        }
        let monitor = Arc::new(AtomicBool::new(false));
        if let Err(error) = engine.live.as_ref().unwrap().start_inputs(monitor.clone()) {
            test.stop(&error);
            engine.runtime_tests.record(test)?;
            return Err(error);
        }
        self.monitor = Some(monitor);
        self.deadline =
            Some(Instant::now() + Duration::from_millis(runtime_tests::LIFETIME_MS as u64));
        engine.event(
            &c.source,
            "Started a bounded input-response comparison. No gameplay value has been changed.",
        );
        if let Err(error) = checkpoint(engine, published) {
            let _ = self.stop(engine, "The experiment checkpoint could not be saved.");
            return Err(error);
        }
        Ok(test.summary())
    }
    fn capture(
        &mut self,
        engine: &mut Engine,
        test: &mut Test,
        condition: usize,
        index: usize,
        base: &Path,
        perform_action: bool,
    ) -> Result<()> {
        if self.abort.load(Ordering::SeqCst) || self.deadline.is_none_or(|d| Instant::now() >= d) {
            return Err("The desktop stopped this comparison.".into());
        }
        let (path, input_path) = engine.runtime_tests.reserve(test, condition, index)?;
        if perform_action {
            let session = engine
                .live
                .as_ref()
                .ok_or("The original application disconnected.")?;
            if !session.view.connected
                || session.view.investigation_id != test.investigation
                || session.view.runtime.catalog_id != test.catalog_id
                || session.view.process.window != test.window
            {
                return Err("The planned input target changed.".into());
            }
            crate::test_input::execute(
                &session.view.process,
                test,
                condition,
                index,
                &path,
                &self.abort,
            )?;
        }
        let capture = engine
            .live
            .as_ref()
            .ok_or("The original application disconnected.")?
            .capture_runtime_test(test, &path)?;
        let bytes = reversebolt_core::visual::image_bytes(&path)?;
        let input = serde_json::to_vec(&capture.input).map_err(|e| e.to_string())?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&input_path)
            .map_err(|e| e.to_string())?;
        file.write_all(&input).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        let difference = if index == 0 {
            None
        } else {
            let prior = engine
                .runtime_tests
                .read_image(test, condition, index - 1)?;
            Some(reversebolt_vision::response::difference(&prior, &bytes)?)
        };
        let frame = Frame {
            path: format!("{}/{condition}-{index}.png", test.id),
            input_path: format!("{}/{condition}-{index}.input.json", test.id),
            sha256: reversebolt_core::visual::digest(&bytes),
            input_sha256: reversebolt_core::visual::digest(&input),
            width: capture.screen.width,
            height: capture.screen.height,
            started_unix_ms: capture.started_unix_ms,
            completed_unix_ms: capture.completed_unix_ms,
            input: InputWitness::check(None, None, &capture.interval, capture.interval.begin_us),
            interval: capture.interval,
            sampling: capture.screen.sampling,
            difference,
        };
        test.finish_capture(frame, capture.input.as_ref())?;
        if self.abort.load(Ordering::SeqCst) {
            return Err(
                "The desktop stopped this comparison after the current image settled.".into(),
            );
        }
        review_baseline(engine, test, &self.abort)?;
        if self.abort.load(Ordering::SeqCst) || self.deadline.is_none_or(|d| Instant::now() >= d) {
            return Err(
                "The experiment stopped or expired while the baseline review settled.".into(),
            );
        }
        if index == 5 && condition > 0 {
            let session = engine.live.as_ref().unwrap();
            let task = Task::Refresh {
                catalog_id: session.view.runtime.catalog_id.clone(),
                after_revision: session.view.runtime.revision,
                ids: vec![test.candidate.id.clone()],
            };
            let started = now_ms();
            let info = session
                .view
                .memory_process
                .as_ref()
                .unwrap_or(&session.view.process)
                .clone();
            let result = runtime_bridge::run(
                &info,
                std::slice::from_ref(&test.candidate.locator),
                base,
                &AtomicBool::new(false),
            )
            .and_then(runtime_trial_bridge::evidence);
            match result {
                Ok(e) => {
                    let checked = now_ms();
                    engine.live.as_mut().unwrap().view.runtime.apply_refresh(
                        &task,
                        &e.report,
                        [started, checked],
                        &e.path,
                    )?;
                    test.conditions[condition].runtime_evidence = Some(e);
                    test.conditions[condition].runtime_checked_unix_ms = Some(checked);
                }
                Err(error) => {
                    engine.live.as_mut().unwrap().view.runtime.phase = "uncertain".into();
                    return Err(error);
                }
            }
            if self.abort.load(Ordering::SeqCst)
                || self.deadline.is_none_or(|d| Instant::now() >= d)
            {
                return Err(
                    "The experiment stopped or expired while the value check settled.".into(),
                );
            }
            if condition == 2 {
                test.complete(&engine.runtime_trials.get(&test.change_id)?, test.revision)?;
            }
        }
        Ok(())
    }
}
pub fn checkpoint(engine: &mut Engine, published: &Arc<RwLock<View>>) -> Result<()> {
    engine.view.runtime_tests = engine
        .runtime_tests
        .records
        .iter()
        .map(|t| t.summary())
        .collect();
    crate::observation_pipeline::checkpoint(engine, published)
}
pub fn check_patch(engine: &Engine, id: Option<&str>, patch: &Patch) -> Result<()> {
    if let Some(t) = engine
        .runtime_tests
        .records
        .iter()
        .find(|t| Some(t.change_id.as_str()) == id)
        && (t.phase != Phase::ReadyToApply
            || !t.active()
            || patch.path != t.candidate.locator
            || patch.type_code != t.candidate.type_code
            || patch.expected != t.candidate.bytes
            || patch.replacement != encode(t.candidate.type_code, &t.value)?)
    {
        return Err("The planned experiment value or reference changed before the edit.".into());
    }
    if let Some(t) = engine
        .runtime_tests
        .records
        .iter()
        .find(|t| Some(t.change_id.as_str()) == id)
    {
        let e = t
            .preparation
            .as_ref()
            .ok_or("Experiment baseline evidence is unavailable.")?;
        let expected = Patch::prepare(
            t.candidate.locator.clone(),
            &e.report["checks"][0],
            &t.value,
            patch.prepared_unix_ms,
        )?;
        if expected.id() != patch.id() {
            return Err("The reference or value changed after the experiment baseline.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "runtime_test_history_tests.rs"]
mod history_tests;
