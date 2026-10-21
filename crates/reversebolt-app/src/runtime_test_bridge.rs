//! Shared live response experiments. Input is supplied externally; capture and
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Prepare {
    test_id: String,
    change_id: String,
    investigation_id: String,
    catalog_id: String,
    after_revision: u64,
    candidate_id: String,
    value: String,
    hypothesis: String,
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
                    | "observe_runtime_test"
                    | "stop_runtime_test"
                    | "read_runtime_test"
                    | "status"
                    | "history"
                    | "list_processes"
                    | "list_memory_processes"
                    | "search_runtime_values"
                    | "read_capture"
                    | "read_sequence"
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
            let mut result = json!({"test":test,"limitation":runtime_tests::LIMITATION});
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
                    json!(base64::engine::general_purpose::STANDARD.encode(image));
                result["input"] = json!(input);
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
        let (condition, index) = test.begin_capture(revision)?;
        engine.runtime_tests.record(test.clone())?;
        checkpoint(engine, published)?;
        let result = self.capture(engine, &mut test, condition, index, base);
        if let Err(error) = &result {
            test.stop(error);
            self.cancel();
        }
        if !test.active() {
            self.cancel();
        }
        engine.runtime_tests.record(test.clone())?;
        result?;
        Ok(test.summary())
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
                || t.candidate.id != p.candidate_id
                || t.value != p.value
                || t.actions != p.actions
                || t.hypothesis != p.hypothesis
                || t.actor != c.source
            {
                return Err("This experiment identity belongs to a different plan.".into());
            }
            return Ok(t.summary());
        }
        if engine.runtime_tests.fault.is_some()
            || engine.runtime_tests.records.len() >= runtime_tests::MAX_TESTS
            || engine
                .runtime_tests
                .records
                .iter()
                .any(|t| t.active() || t.change_id == p.change_id)
            || engine.runtime_trials.get(&p.change_id).is_ok()
            || engine.view.freezes.iter().any(|f| f.active())
        {
            return Err("Finish the active test/Freeze first, and use new identities within the eight-test evidence budget.".into());
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
        let task:Task=serde_json::from_value(json!({"operation":"refresh","catalog_id":p.catalog_id,"after_revision":p.after_revision,"ids":[p.candidate_id]})).map_err(|e|e.to_string())?;
        let paths = session.view.runtime.paths(&task)?;
        let candidate = session
            .view
            .runtime
            .cards
            .iter()
            .find(|c| c.candidate.id == p.candidate_id)
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
            hypothesis: p.hypothesis,
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
    ) -> Result<()> {
        if self.abort.load(Ordering::SeqCst) || self.deadline.is_none_or(|d| Instant::now() >= d) {
            return Err("The desktop stopped this comparison.".into());
        }
        let (path, input_path) = engine.runtime_tests.reserve(test, condition, index)?;
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
