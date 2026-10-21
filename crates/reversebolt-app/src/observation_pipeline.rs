//! One background interpretation per owning capture job; publication stays serialized.
use crate::{
    local_observer::{Prepared, Worker},
    service::OperationOwner,
};
use reversebolt_core::{
    Result,
    engine::{Engine, View},
};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, RwLock, atomic::Ordering},
    time::Duration,
};

#[derive(Default)]
pub struct Pipeline {
    active: Option<Worker>,
    failure: Option<(u64, String)>,
    last: Option<(u64, Value)>,
}

pub fn checkpoint(engine: &mut Engine, published: &Arc<RwLock<View>>) -> Result<()> {
    engine.view.live = engine.live.as_ref().map(|s| s.view.clone());
    let result = engine.save();
    if let Err(error) = &result {
        engine.view.error = Some(format!("Observation checkpoint failed: {error}"));
    }
    *published.write().unwrap() = engine.view.clone();
    result
}

impl Pipeline {
    #[cfg(test)]
    pub(crate) fn test_active(worker: Worker) -> Self {
        Self {
            active: Some(worker),
            ..Self::default()
        }
    }
    pub fn busy(&self) -> bool {
        self.active.is_some()
    }
    pub fn stop(&self) {
        if let Some(worker) = &self.active {
            worker.cancel.store(true, Ordering::SeqCst);
        }
    }
    pub fn check(&self, owner: &OperationOwner) -> Result<()> {
        if let Some((job, error)) = &self.failure
            && *job == owner.job
        {
            return Err(error.clone());
        }
        if self
            .active
            .as_ref()
            .is_some_and(|w| w.status.job != owner.job)
        {
            return Err("An earlier observation worker is still settling.".into());
        }
        Ok(())
    }
    pub fn start(
        &mut self,
        engine: &mut Engine,
        model: &str,
        owner: &OperationOwner,
        directory: &Path,
        published: &Arc<RwLock<View>>,
    ) -> Result<Value> {
        self.check(owner)?;
        if self.active.is_some() {
            return Err("Local observation already has an active worker.".into());
        }
        let prepared = Prepared::from_session(
            engine
                .live
                .as_ref()
                .ok_or("Connect an application first.")?,
        )?;
        let worker = Worker::start(
            prepared,
            model.into(),
            owner.job,
            owner.cancel.clone(),
            directory.into(),
        )?;
        let result =
            json!({"evidence_id":worker.status.evidence_id,"started":true,"background":true});
        engine.view.analysis = Some(worker.status.clone());
        engine.event(
            "local-model",
            format!(
                "Started local image review {} for observation job {}.",
                worker.status.evidence_id, owner.job
            ),
        );
        self.active = Some(worker);
        if let Err(error) = checkpoint(engine, published) {
            self.stop();
            self.failure = Some((owner.job, error.clone()));
            return Err(error);
        }
        Ok(result)
    }
    pub fn poll(&mut self, engine: &mut Engine, published: &Arc<RwLock<View>>) {
        let Some(worker) = self.active.as_mut() else {
            return;
        };
        if !engine.live.as_ref().is_some_and(|s| s.view.connected) {
            worker.cancel.store(true, Ordering::SeqCst);
        }
        let stopping = worker.cancel.load(Ordering::SeqCst);
        if let Some(status) = engine.view.analysis.as_mut()
            && status.stopping != stopping
        {
            status.stopping = stopping;
            *published.write().unwrap() = engine.view.clone();
        }
        let Some(result) = worker.poll() else {
            return;
        };
        let worker = self.active.take().unwrap();
        let result = result.and_then(|completed| {
            completed.publish(
                engine
                    .live
                    .as_mut()
                    .ok_or("The observation target is no longer available.")?,
                &worker.cancel,
            )
        });
        engine.view.analysis = None;
        match result {
            Ok(value) => {
                if let Some(warning) = value["warning"].as_str() {
                    // Publish and persist the notice in the same serialized engine
                    // checkpoint as the result. This does not cancel the job.
                    let notices = &mut engine
                        .live
                        .as_mut()
                        .unwrap()
                        .view
                        .local_observation_warnings;
                    if notices.len() >= 32 {
                        notices.remove(0);
                    }
                    notices.push(reversebolt_core::live::LocalObservationWarning {
                        evidence_id: worker.status.evidence_id.clone(),
                        job: worker.status.job,
                        frame_ids: worker.status.frame_ids.clone(),
                        message: warning.into(),
                    });
                    engine.event(
                        "local-model",
                        format!(
                            "Local review {} unavailable: {warning} Other observation continues.",
                            worker.status.evidence_id
                        ),
                    );
                } else {
                    engine.event(
                        "local-model",
                        format!(
                            "Local review {} recorded {} provisional items; cancelled: {}.",
                            worker.status.evidence_id, value["recorded"], value["cancelled"]
                        ),
                    );
                }
                self.last = Some((worker.status.job, value));
            }
            Err(error) => {
                worker.cancel.store(true, Ordering::SeqCst);
                engine.event(
                    "local-model",
                    format!(
                        "Local review {} stopped: {error}",
                        worker.status.evidence_id
                    ),
                );
                engine.view.error = Some(error.clone());
                self.failure = Some((worker.status.job, error));
            }
        }
        if let Err(error) = checkpoint(engine, published) {
            worker.cancel.store(true, Ordering::SeqCst);
            self.failure = Some((worker.status.job, error));
        }
    }
    /// The final capture still owns its ledger operation until every model call settles.
    pub fn drain(
        &mut self,
        engine: &mut Engine,
        owner: &OperationOwner,
        published: &Arc<RwLock<View>>,
    ) -> Result<Value> {
        while self.busy() {
            if let Some(live) = engine.live.as_mut()
                && live.refresh_connection()
            {
                self.stop();
                if let Err(error) = checkpoint(engine, published) {
                    self.failure = Some((owner.job, error));
                }
            }
            self.poll(engine, published);
            if self.busy() {
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        self.check(owner)?;
        Ok(self
            .last
            .as_ref()
            .filter(|(job, _)| *job == owner.job)
            .map_or(Value::Null, |(_, v)| v.clone()))
    }
}
