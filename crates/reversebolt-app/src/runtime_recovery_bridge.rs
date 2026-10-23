//! Reuse checked historical evidence through the same desktop/agent engine.
use crate::{runtime_bridge, runtime_test_bridge};
use reversebolt_core::{
    Command, Result,
    engine::Engine,
    runtime,
    runtime_recovery::{self, RecoveredControl},
    runtime_tests::{Phase, Test},
    visual,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};

fn verify(engine: &Engine, test: &Test) -> Result<()> {
    if test.phase != Phase::Complete {
        return Err("Complete and restore this comparison before remembering it.".into());
    }
    test.recovery_recipe
        .as_ref()
        .ok_or("This older comparison has no retained route metadata; run a new comparison.")?
        .check_candidate(&test.candidate)?;
    let trial = engine.runtime_trials.get(&test.change_id)?;
    if test
        .assessment
        .as_ref()
        .is_some_and(|a| a.status == "suppression_observed")
    {
        reversebolt_vision::response::verify_recorded_response(
            test,
            &trial,
            |c, f| engine.runtime_tests.read_image(test, c, f),
            |c, f| engine.runtime_tests.read_input(test, c, f),
        )
    } else if let Some(plan) = &test.appearance {
        let (_, assessment) =
            runtime_test_bridge::compare_appearance(engine, test, plan.selection.clone())?;
        if assessment.status != "regional_change_and_return_observed" {
            return Err("The earlier comparison did not establish a change and return.".into());
        }
        Ok(())
    } else {
        Err("The earlier comparison did not establish a change and return.".into())
    }
}

fn scope<'a>(engine: &'a Engine, catalog: &str, revision: u64) -> Result<&'a runtime::View> {
    let live = engine
        .live
        .as_ref()
        .ok_or("Connect an application first.")?;
    let view = &live.view.runtime;
    if !live.view.connected
        || !view.available
        || view.phase != "ready"
        || catalog != view.catalog_id
        || revision != view.revision
    {
        return Err("The application or current catalogue changed.".into());
    }
    Ok(view)
}

fn publish(
    view: &mut runtime::View,
    test: &Test,
    result: &runtime_recovery::Reacquisition,
) -> Result<bool> {
    let Some(candidate) = &result.candidate else {
        return Ok(false);
    };
    let source = view
        .catalog_source
        .as_ref()
        .ok_or("Discovery evidence unavailable.")?;
    if source.document_sha256 != result.current_report_sha256
        || candidate.locator.scope != result.current_scope
        || candidate.id != candidate.locator.id()
        || result.status != "candidate_for_revalidation"
        || result.edit_authority
    {
        return Err("Recovered candidate does not belong to this catalogue.".into());
    }
    let hint = RecoveredControl {
        source_test_id: test.id.clone(),
        source_test_sha256: visual::digest(&serde_json::to_vec(test).map_err(|e| e.to_string())?),
        recipe_sha256: result.recipe_sha256.clone(),
        catalog_id: view.catalog_id.clone(),
        report_sha256: result.current_report_sha256.clone(),
        candidate_id: candidate.id.clone(),
        name: test.candidate.name.clone(),
        description:
            "Needs a fresh check: previously changed the response to a recorded action sequence."
                .into(),
        needs_revalidation: true,
    };
    let prior = view.search.take();
    view.search = Some(runtime::SearchPage {
        query: String::new(),
        offset: 0,
        matches: 1,
        candidates: vec![candidate.clone()],
    });
    let result = view.show_values(
        &view.catalog_id.clone(),
        view.revision,
        std::slice::from_ref(&candidate.id),
    );
    view.search = prior;
    result?;
    view.recovered_controls.retain(|h| {
        h.candidate_id != candidate.id
            && view.cards.iter().any(|c| c.candidate.id == h.candidate_id)
    });
    view.recovered_controls.push(hint);
    Ok(true)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    test_id: String,
    catalog_id: String,
    after_revision: u64,
}

pub(crate) fn command(engine: &mut Engine, command: &Command) -> Result<Value> {
    let request: Request =
        serde_json::from_value(command.args.clone()).map_err(|e| e.to_string())?;
    if engine.runtime_tests.records.iter().any(|t| t.active())
        || engine.view.freezes.iter().any(|f| f.active())
    {
        return Err("Finish the active comparison or Freeze before recovering a control.".into());
    }
    let view = scope(engine, &request.catalog_id, request.after_revision)?;
    let report = runtime_bridge::catalog_report(view)?;
    let live = &engine.live.as_ref().unwrap().view;
    let info = live.memory_process.as_ref().unwrap_or(&live.process);
    let test = engine.runtime_tests.get(&request.test_id)?.clone();
    verify(engine, &test)?;
    let mut results = runtime_recovery::reacquire(
        std::slice::from_ref(test.recovery_recipe.as_ref().unwrap()),
        &report,
    )?;
    let result = results.remove(0);
    if result.current_scope != runtime_bridge::runtime_scope(info) {
        return Err("Discovery belongs to another process.".into());
    }
    let mut staged = view.clone();
    let shown = publish(&mut staged, &test, &result)?;
    let preparation=result.candidate.as_ref().map(|c| json!({"candidate_id":c.id,
        "hypothesis":test.hypothesis,"value":test.value,"actions":test.actions,
        "investigation_id":live.investigation_id,"catalog_id":staged.catalog_id,"after_revision":staged.revision}));
    engine.live.as_mut().unwrap().view.runtime = staged;
    Ok(
        json!({"source_test_id":test.id,"source_test_sha256":visual::digest(&serde_json::to_vec(&test).map_err(|e|e.to_string())?),
        "recovery":result,"shown":shown,"preparation":preparation,"edit_authority":false}),
    )
}

/// Discovery calls this once, before any automatic naming publication. The old
/// comparison is rechecked from retained files; input and memory are untouched.
pub(crate) fn automatic(engine: &mut Engine, report: &Value, cancel: &AtomicBool) -> Result<Value> {
    if engine.runtime_tests.records.iter().any(|t| t.active())
        || engine.view.freezes.iter().any(|f| f.active())
    {
        return Err("Finish the active comparison or Freeze before recovering a control.".into());
    }
    let live = engine
        .live
        .as_ref()
        .ok_or("Connect an application first.")?;
    let module = report["runtime"]
        .as_str()
        .ok_or("Missing runtime identity.")?
        .replace('\\', "/")
        .to_ascii_lowercase();
    let mut tests = vec![];
    let mut unavailable = 0;
    let mut checked = 0;
    let mut routes = std::collections::BTreeSet::new();
    for test in engine.runtime_tests.records.iter().rev().filter(|t| {
        t.recovery_recipe
            .as_ref()
            .is_some_and(|r| r.runtime == module)
    }) {
        if checked == runtime_recovery::MAX_RECOVERY_RECIPES || cancel.load(Ordering::SeqCst) {
            break;
        }
        // A newer stopped, negative or unavailable test prevents automatic
        // fallback to an older positive result for the same declared route.
        let route = serde_json::to_string(&test.recovery_recipe.as_ref().unwrap().route)
            .map_err(|e| e.to_string())?;
        if !routes.insert(route) {
            continue;
        }
        checked += 1;
        if verify(engine, test).is_err() {
            unavailable += 1;
            continue;
        }
        tests.push(test.clone());
    }
    if tests.is_empty() || cancel.load(Ordering::SeqCst) {
        return Ok(json!({"checked":checked,"unavailable":unavailable,"shown":0}));
    }
    let recipes = tests
        .iter()
        .map(|t| t.recovery_recipe.clone().unwrap())
        .collect::<Vec<_>>();
    let results = runtime_recovery::reacquire(&recipes, report)?;
    let expected = runtime_bridge::runtime_scope(
        live.view
            .memory_process
            .as_ref()
            .unwrap_or(&live.view.process),
    );
    let mut staged = live.view.runtime.clone();
    let mut shown = 0;
    for (test, result) in tests.iter().zip(&results) {
        if result.current_scope != expected {
            return Err("Recovery discovery belongs to another process.".into());
        }
        shown += usize::from(publish(&mut staged, test, result)?);
    }
    if !cancel.load(Ordering::SeqCst) {
        engine.live.as_mut().unwrap().view.runtime = staged;
    } else {
        shown = 0;
    }
    Ok(json!({"checked":checked,"unavailable":unavailable,"shown":shown,"edit_authority":false}))
}
