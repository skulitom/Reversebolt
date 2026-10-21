//! Optional metadata review owned by the existing discovery job.
use crate::service::OperationOwner;
use reversebolt_core::{
    Result,
    engine::{Engine, View},
    runtime,
    runtime_hypotheses::Review,
};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::Path,
    sync::{Arc, RwLock, atomic::Ordering},
};

fn save_new(path: &Path, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("Metadata review exceeds its archive budget.".into());
    }
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(&bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| e.to_string())
}

pub(crate) fn publish(
    view: &mut runtime::View,
    catalog: &str,
    mut review: Review,
    cancelled: bool,
) -> Result<()> {
    if view.catalog_id != catalog
        || view
            .catalog_source
            .as_ref()
            .is_none_or(|s| s.document_sha256 != review.report_sha256)
    {
        return Err("Metadata hypotheses belong to a replaced catalogue.".into());
    }
    if cancelled {
        review.status = "cancelled".into();
        review.batch = None;
    } else if let Some(batch) = &review.batch
        && batch.report_sha256 != review.report_sha256
    {
        return Err("Hypothesis report binding changed.".into());
    }
    view.naming = Some(review);
    Ok(())
}

pub fn review(
    engine: &mut Engine,
    model: &str,
    owner: &OperationOwner,
    base: &Path,
    published: &Arc<RwLock<View>>,
) -> Result<Value> {
    let result = review_inner(engine, model, owner, base, published);
    if let Err(error) = &result {
        if let Some(review) = engine
            .live
            .as_mut()
            .and_then(|s| s.view.runtime.naming.as_mut())
            && review.status == "pending"
        {
            review.status = "failed".into();
            review.batch = None;
            review.error = Some(error.clone());
        }
        // Keep the failed archive/checkpoint visible; never publish partial hypotheses.
        let _ = crate::observation_pipeline::checkpoint(engine, published);
    }
    result
}

fn review_inner(
    engine: &mut Engine,
    model: &str,
    owner: &OperationOwner,
    base: &Path,
    published: &Arc<RwLock<View>>,
) -> Result<Value> {
    if owner.cancel.load(Ordering::SeqCst) {
        return Ok(json!({"cancelled":true}));
    }
    let session = engine
        .live
        .as_ref()
        .ok_or("The application disconnected.")?;
    let source = session
        .view
        .runtime
        .catalog_source
        .clone()
        .ok_or("No discovery report is available.")?;
    let catalog = session.view.runtime.catalog_id.clone();
    let report = crate::runtime_bridge::catalog_report(&session.view.runtime)?;
    let id = uuid::Uuid::new_v4().to_string();
    let directory = base.join("naming").join(&id);
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    save_new(
        &directory.join("intent.json"),
        &json!({"schema":1,"catalog_id":catalog,"report_sha256":source.document_sha256,
        "source":source.path,"job":owner.job,"model":model,"scope":{"pid":report["pid"],"created":report["created"],"session":report["session"]},"status":"pending"}),
    )?;
    let mut state = Review {
        evidence_id: id,
        report_sha256: source.document_sha256,
        model: model.into(),
        status: "pending".into(),
        batch: None,
        error: None,
    };
    publish(
        &mut engine.live.as_mut().unwrap().view.runtime,
        &catalog,
        state.clone(),
        false,
    )?;
    crate::observation_pipeline::checkpoint(engine, published)?;
    let inference = (|| -> Result<Value> {
        let inventory = reversebolt_core::runtime_inventory::build(&report)?;
        save_new(&directory.join("inventory.json"), &json!(inventory))?;
        if owner.cancel.load(Ordering::SeqCst) {
            return Ok(json!({"cancelled":true}));
        }
        let receipt = tokio::runtime::Runtime::new()
            .map_err(|e| e.to_string())?
            .block_on(async {
                let observer = reversebolt_observer::Observer::connect(model).await?;
                Ok::<_, String>(observer.runtime_hypotheses(&inventory, &owner.cancel).await)
            })?;
        state.batch = receipt.batch.clone();
        state.error = receipt.error.clone();
        Ok(json!(receipt))
    })();
    let result = match inference {
        Ok(value) => value,
        Err(error) => {
            state.error = Some(error.clone());
            json!({"error":error})
        }
    };
    // Persist the outcome before any hypothesis becomes visible, including failures.
    save_new(&directory.join("receipt.json"), &result)?;
    let session = engine.live.as_mut().unwrap();
    session.refresh_connection();
    let cancelled = owner.cancel.load(Ordering::SeqCst) || !session.view.connected;
    if !cancelled {
        crate::runtime_bridge::catalog_report(&session.view.runtime)?;
    }
    state.status = if state.error.is_some() {
        "unavailable"
    } else {
        "complete"
    }
    .into();
    if state.error.is_some() {
        state.batch = None;
    }
    let count = state.batch.as_ref().map_or(0, |b| b.accepted.len());
    let evidence_id = state.evidence_id.clone();
    publish(&mut session.view.runtime, &catalog, state, cancelled)?;
    engine.event("local-model",format!("Metadata review {evidence_id} retained {} provisional questions; cancelled: {cancelled}. No card meaning or edit authority changed.",if cancelled{0}else{count}));
    crate::observation_pipeline::checkpoint(engine, published)?;
    Ok(
        json!({"evidence_id":evidence_id,"hypotheses":if cancelled{0}else{count},"cancelled":cancelled}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view() -> runtime::View {
        runtime::View {
            catalog_id: "current".into(),
            catalog_source: Some(runtime::CatalogSource {
                path: "unused".into(),
                document_sha256: "a".repeat(64),
                observed_unix_ms: 0,
            }),
            ..Default::default()
        }
    }
    fn review() -> Review {
        Review {
            evidence_id: "review".into(),
            report_sha256: "a".repeat(64),
            model: "local".into(),
            status: "complete".into(),
            error: None,
            batch: Some(reversebolt_core::runtime_hypotheses::Batch {
                report_sha256: "a".repeat(64),
                selected_types: vec![],
                considered_samples: 0,
                omitted_samples: 0,
                accepted: vec![],
                rejected: vec![],
                limitation: "Unverified".into(),
            }),
        }
    }
    #[test]
    fn cancelled_or_rebound_reviews_never_publish_hypotheses() {
        let mut v = view();
        publish(&mut v, "current", review(), true).unwrap();
        assert_eq!(v.naming.as_ref().unwrap().status, "cancelled");
        assert!(v.naming.as_ref().unwrap().batch.is_none());
        assert!(publish(&mut v, "old", review(), false).is_err());
        let mut r = review();
        r.batch.as_mut().unwrap().report_sha256 = "b".repeat(64);
        assert!(publish(&mut v, "current", r, false).is_err());
        assert!(v.naming.as_ref().unwrap().batch.is_none());
        assert!(v.cards.is_empty());
        assert_eq!(v.revision, 0);
    }
}
