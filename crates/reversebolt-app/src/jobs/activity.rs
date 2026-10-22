use super::*;
use crate::activity_experiment::Step;
use reversebolt_core::activity_experiment::{self as activity, Plan};
#[cfg(test)]
mod tests;

impl Controller {
    pub(super) fn plan_activity(
        &self,
        command: &Command,
        binding: &job::Binding,
        request: u64,
        boot: u64,
        actor: &job::Actor,
    ) -> Result<job::Action> {
        if command.args.as_object().is_none_or(|m| {
            m.keys()
                .any(|k| !matches!(k.as_str(), "key" | "request_id" | "boot" | "binding"))
        }) {
            return Err("Input experiments accept only a sampled key and the job envelope.".into());
        }
        let key = command.args["key"]
            .as_u64()
            .filter(|k| *k < 255)
            .ok_or("Choose a previously sampled key.")? as u8;
        let history = self.jobs.snapshot();
        if let Some(client) = history.clients.get(&actor.client)
            && request <= client.high_water
        {
            let receipt = client
                .receipts
                .iter()
                .find(|r| r.command.request == request)
                .ok_or("This old request is no longer replayable.")?;
            let job::Action::Submit { steps, .. } = &receipt.command.action else {
                return Err("Request ID belongs to another operation".into());
            };
            if steps
                .first()
                .and_then(|s| serde_json::from_str::<Step>(&s.payload).ok())
                .is_none_or(|s| s.key != key)
            {
                return Err("Request ID was already used with different arguments".into());
            }
            return Ok(receipt.command.action.clone());
        }
        if history.current.as_ref() != Some(binding)
            || boot != history.boot
            || self.manual.is_some()
            || self.executor.flight.is_some()
            || history.jobs.values().any(|j| !j.phase.terminal())
        {
            return Err("Finish the current work and use the current application binding before testing an input.".into());
        }
        if actor.role == job::Role::Agent && history.human_hold.is_some() {
            return Err("The desktop has control.".into());
        }
        let view = self.engine.view.read().unwrap();
        if view.analysis.is_some()
            || view.runtime_tests.iter().any(|t| t["active"] == true)
            || view
                .freezes
                .iter()
                .any(|f| f.active() || f.phase == "uncertain")
        {
            return Err("Finish active image reviews, comparisons and freezes first.".into());
        }
        let live = view.live.as_ref().ok_or("Connect an application first.")?;
        if live
            .change_searches
            .last()
            .is_some_and(|s| s.status == "collecting")
        {
            return Err("Finish the current change search first.".into());
        }
        let plan = Plan::new(live, key, format!("{boot}:{}:{request}", actor.client))?;
        if plan.binding() != binding.target {
            return Err("Application binding changed".into());
        }
        let reference = crate::activity_experiment::store(&self.runtime_directory, &plan)?;
        let steps = (0..activity::STEPS)
            .map(|index| {
                Ok(job::Step {
                    kind: if activity::action(index, key).is_some() {
                        job::Kind::Effect
                    } else {
                        job::Kind::Observe
                    },
                    payload: serde_json::to_string(&Step {
                        activity: reference.clone(),
                        index,
                        key,
                    })
                    .map_err(|e| e.to_string())?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(job::Action::Submit {
            steps,
            consent: actor.role == job::Role::Human,
        })
    }
}
impl CaptureExecutor {
    pub(super) fn begin_activity(&mut self, operation: &job::Operation, step: Step) -> job::Poll {
        let effect = operation.instruction.kind == job::Kind::Effect;
        let rejected = |note: &str| {
            job::Poll::Ready(job::Outcome {
                success: false,
                effect: if effect {
                    job::Knowledge::NotApplied
                } else {
                    job::Knowledge::NotApplicable
                },
                data: vec![],
                note: note.into(),
            })
        };
        if self.flight.is_some()
            || step.index >= activity::STEPS
            || effect != activity::action(step.index, step.key).is_some()
            || Scope::from_view(&self.engine.view.read().unwrap())
                .map(|s| s.key())
                .as_ref()
                != Some(&operation.binding.target)
        {
            return rejected("Input experiment changed scope or step kind.");
        }
        if self
            .owner
            .as_ref()
            .is_none_or(|o| o.job != operation.job || o.cancel.load(Ordering::SeqCst))
        {
            if step.index != 0 {
                return rejected("Input experiment ownership ended; this sequence cannot resume.");
            }
            self.owner = Some(OperationOwner {
                job: operation.job,
                cancel: Arc::new(AtomicBool::new(false)),
            });
        }
        let owner = self.owner.as_ref().unwrap().clone();
        let command = Command {
            action: "_activity_step".into(),
            args: json!(step),
            source: "input-experiment".into(),
        };
        match self
            .engine
            .start_owned_command(command, Some(owner.clone()))
        {
            Ok(response) => {
                self.flight = Some(Flight {
                    id: operation.id,
                    response,
                    cancel: owner.cancel,
                    poll_interval: Duration::from_secs(1),
                    runtime: false,
                    effect,
                });
                self.next_poll = Instant::now() + Duration::from_secs(1);
                job::Poll::Pending
            }
            Err(error) => rejected(&error),
        }
    }
}
