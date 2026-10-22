use super::*;
use crate::activity_experiment::Step;
use reversebolt_core::activity_experiment::{self as activity, Plan};
#[cfg(test)]
mod tests;

impl Controller {
    pub(super) fn activity_suggestion(&self) -> Result<activity::selection::Suggestion> {
        let view = self.engine.view.read().unwrap();
        let live = view.live.as_ref().ok_or("Connect an application first.")?;
        let binding = Scope::from_view(&view)
            .ok_or("Application disconnected")?
            .key();
        let history = self.jobs.snapshot();
        let mut attempted: Vec<_> = history
            .jobs
            .values()
            .filter(|j| j.binding.target == binding)
            .filter_map(|j| j.steps.first())
            .filter_map(|s| serde_json::from_str::<Step>(&s.payload).ok())
            .map(|s| s.key)
            .collect();
        attempted.extend(crate::activity_experiment::selection_history::keys(
            &self.runtime_directory,
            &binding,
        )?);
        activity::selection::suggest(live, &attempted)
    }

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
        let requested_key = command
            .args
            .get("key")
            .map(|value| {
                value
                    .as_u64()
                    .filter(|k| (1..255).contains(k))
                    .map(|k| k as u8)
                    .ok_or("Supply a sampled key or omit key for automatic selection.")
            })
            .transpose()?;
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
            let first = steps
                .first()
                .and_then(|s| serde_json::from_str::<Step>(&s.payload).ok())
                .ok_or("Request ID belongs to another operation")?;
            let original =
                crate::activity_experiment::load(&self.runtime_directory, &first.activity)?;
            if requested_key.is_none() != original.selection.is_some()
                || requested_key.is_some_and(|k| k != first.key)
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
        let selection = requested_key
            .is_none()
            .then(|| self.activity_suggestion())
            .transpose()?;
        let key = requested_key
            .or_else(|| {
                selection
                    .as_ref()
                    .and_then(|s| s.next.as_ref())
                    .map(|c| c.key)
            })
            .ok_or_else(|| {
                selection
                    .as_ref()
                    .and_then(|s| s.unavailable.clone())
                    .unwrap_or_else(|| "No input experiment is available.".into())
            })?;
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
        let mut plan = Plan::new(live, key, format!("{boot}:{}:{request}", actor.client))?;
        plan.observer_model = view.observer_model.clone();
        plan.selection = selection;
        plan.check(live, 0)?;
        if plan.binding() != binding.target {
            return Err("Application binding changed".into());
        }
        let reference = crate::activity_experiment::store(&self.runtime_directory, &plan)?;
        crate::activity_experiment::selection_history::reserve(
            &self.runtime_directory,
            &plan,
            &reference,
        )?;
        let steps = (0..=activity::STEPS)
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
            || step.index > activity::STEPS
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
            action: if step.index == activity::STEPS {
                "_review_activity"
            } else {
                "_activity_step"
            }
            .into(),
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
