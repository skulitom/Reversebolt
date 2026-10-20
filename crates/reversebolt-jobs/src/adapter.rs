//! Optional bounded ingress for the existing serial worker. No thread/server.
use crate::{Action, Command, Role};
use std::collections::VecDeque;

pub struct Inbox {
    queue: VecDeque<Command>,
    normal: usize,
    control: usize,
}
impl Inbox {
    pub fn new(normal: usize, control: usize) -> Result<Self, String> {
        if normal == 0 || normal > 16 || control == 0 || control > 8 {
            return Err("invalid ingress budget".into());
        }
        Ok(Self {
            queue: VecDeque::new(),
            normal,
            control,
        })
    }
    fn control(c: &Command) -> bool {
        c.actor.role == Role::Human
            && matches!(
                c.action,
                Action::Pause { .. } | Action::Cancel { .. } | Action::TakeControl
            )
    }
    /// Admission only, NOT a durable job acknowledgement. Caller must await the
    /// worker's Jobs::command receipt (or retry the same ID if the reply is lost).
    pub fn try_send(&mut self, c: Command) -> Result<(), String> {
        if serde_json::to_vec(&c).map_err(|e| e.to_string())?.len() > 8192 {
            return Err("envelope budget".into());
        }
        let control = Self::control(&c);
        let count = self
            .queue
            .iter()
            .filter(|m| Self::control(m) == control)
            .count();
        if count >= if control { self.control } else { self.normal } {
            return Err("ingress lane full; request not accepted".into());
        }
        self.queue.push_back(c);
        Ok(())
    }
    /// Human controls lead other clients. Preserve FIFO for the same client,
    /// including a preceding submission that a later cancel refers to.
    pub fn pop(&mut self) -> Option<Command> {
        if let Some(index) = self.queue.iter().position(Self::control) {
            let client = &self.queue[index].actor.client;
            let first = self
                .queue
                .iter()
                .position(|c| &c.actor.client == client)
                .unwrap();
            self.queue.remove(first)
        } else {
            self.queue.pop_front()
        }
    }
    pub fn len(&self) -> usize {
        self.queue.len()
    }
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
