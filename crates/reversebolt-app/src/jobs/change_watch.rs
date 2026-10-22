use super::*;
use reversebolt_core::changes::Phase;

/// Compact journal-only field names keep full UUID/process identities inside
/// the existing 256-byte step budget. The engine API uses ObservationCursor.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Cursor {
    #[serde(rename = "id", alias = "change_id")]
    pub change_id: String,
    pub phase: Phase,
    #[serde(rename = "after", alias = "after_observation")]
    pub after_observation: usize,
}

impl Controller {
    pub(super) fn plan_change_watch(
        &self,
        command: &Command,
        binding: &job::Binding,
        request: u64,
        client: &str,
    ) -> Result<job::Action> {
        let id = command.args["change_id"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 80)
            .ok_or("Choose a trial to watch.")?;
        let phase: Phase = serde_json::from_value(command.args["phase"].clone())
            .map_err(|_| "Supply the trial's current written or restored phase.")?;
        if !matches!(phase, Phase::Written | Phase::Restored) {
            return Err("Only a settled write or restoration can be watched.".into());
        }
        let after = command.args["after_observation"]
            .as_u64()
            .filter(|n| *n <= 8)
            .ok_or("Supply the current observation count, from zero to eight.")?
            as usize;
        let count = command.args["count"]
            .as_u64()
            .filter(|n| (1..=8).contains(n))
            .ok_or("Choose one to eight result observations.")? as usize;
        let interval = command.args["interval_ms"]
            .as_u64()
            .filter(|n| (250..=60_000).contains(n))
            .ok_or("Choose an interval from 250 to 60,000 milliseconds.")?;
        let limit = if phase == Phase::Written { 6 } else { 8 };
        if after + count > limit {
            return Err("This watch exceeds the remaining observations. Two slots are kept for restoration checks while a trial is applied.".into());
        }
        // Replays must retain their original payload even after the trial advances.
        let parts: Vec<_> = binding.target.split('/').collect();
        if parts.len() != 4 {
            return Err("Invalid watch binding.".into());
        }
        let scope = Scope {
            investigation_id: parts[0].into(),
            batch: parts[1].parse().map_err(|_| "Invalid watch batch.")?,
            created: parts[2].into(),
            window: parts[3].parse().map_err(|_| "Invalid watch window.")?,
        };
        let history = self.jobs.snapshot();
        let fresh = request > history.clients.get(client).map_or(0, |c| c.high_water);
        if fresh && history.current.as_ref() == Some(binding) {
            if self.manual.is_some()
                || self.executor.flight.is_some()
                || history.jobs.values().any(|j| !j.phase.terminal())
            {
                return Err(
                    "Finish or stop the current observation job before watching a trial.".into(),
                );
            }
            let state = self.engine.view.read().unwrap();
            if state.analysis.is_some() {
                return Err("Wait for the current image review to finish.".into());
            }
            let live = state
                .live
                .as_ref()
                .filter(|l| l.connected)
                .ok_or("The original application must still be connected.")?;
            let change = state
                .changes
                .iter()
                .find(|c| c.id == id)
                .ok_or("That trial is no longer in history.")?;
            if change.phase != phase
                || change.observations.len() != after
                || change.scope.investigation != live.investigation_id
                || change.scope.batch != live.coverage.batch
                || change.scope.process.pid
                    != live.memory_process.as_ref().unwrap_or(&live.process).pid
                || change.scope.process.created
                    != live
                        .memory_process
                        .as_ref()
                        .unwrap_or(&live.process)
                        .created
            {
                return Err(
                    "The trial or application changed. Refresh its state before watching.".into(),
                );
            }
        }
        let steps = (0..count)
            .map(|index| {
                Ok(job::Step {
                    kind: job::Kind::Observe,
                    payload: serde_json::to_string(&CaptureStep {
                        memory_review: false,
                        runtime: None,
                        scope: scope.clone(),
                        delay_ms: interval,
                        source: command.source.clone(),
                        compare: false,
                        remaining: 0,
                        mode: Default::default(),
                        local_model: None,
                        background_model: false,
                        change: Some(Cursor {
                            change_id: id.into(),
                            phase: phase.clone(),
                            after_observation: after + index,
                        }),
                    })
                    .map_err(|e| e.to_string())?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(job::Action::Submit {
            steps,
            consent: false,
        })
    }
}
