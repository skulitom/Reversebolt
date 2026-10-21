//! Bounded sampled input context. Polls are not game-delivered events or causal evidence.
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const MAX_POLLS: usize = 4096;
pub const MAX_WINDOW_POLLS: usize = 256;
pub const LIMITATION: &str = "Sampled Windows virtual-key states while the selected window was foreground. Keys are not translated into text. Zero can mean up or unavailable; short taps and changes between polls may be missed. Focus checks are best-effort. These are not delivered game inputs, continuous holds, button meanings or causal effects. Mouse motion, wheel, gamepads and audio are not sampled.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Focus {
    SelectedForeground,
    OutsideSelected,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Poll {
    pub begin_us: u64,
    pub end_us: u64,
    pub focus: Focus,
    /// Sorted unique virtual-key codes; no characters, titles or other target identities.
    pub down: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Window {
    pub clock_id: String,
    pub generation: String,
    pub requested_begin_us: u64,
    pub requested_end_us: u64,
    pub monitor_started_us: u64,
    pub retained_from_us: u64,
    pub omitted_polls: usize,
    /// Includes the requested start/end edges, not only gaps between retained polls.
    pub largest_unsampled_us: u64,
    pub polls: Vec<Poll>,
    pub limitation: String,
}

pub struct Buffer {
    clock_id: String,
    generation: String,
    started_us: u64,
    discarded_until_us: Option<u64>,
    polls: VecDeque<Poll>,
}

impl Buffer {
    pub fn new(clock_id: String, generation: String, started_us: u64) -> Self {
        Self {
            clock_id,
            generation,
            started_us,
            discarded_until_us: None,
            polls: VecDeque::new(),
        }
    }

    pub fn push(&mut self, poll: Poll) -> Result<()> {
        if poll.begin_us < self.started_us
            || poll.end_us < poll.begin_us
            || self.polls.back().is_some_and(|p| poll.begin_us <= p.end_us)
            || poll.down.len() > 254
            || poll.down.iter().any(|k| *k == 0 || *k == 255)
            || poll.down.windows(2).any(|p| p[0] >= p[1])
            || (poll.focus != Focus::SelectedForeground && !poll.down.is_empty())
        {
            return Err("Invalid input poll chronology, key set or foreground scope.".into());
        }
        if self.polls.len() == MAX_POLLS {
            self.discarded_until_us = self.polls.pop_front().map(|p| p.end_us);
        }
        self.polls.push_back(poll);
        Ok(())
    }

    pub fn window(&self, begin_us: u64, end_us: u64) -> Option<Window> {
        if begin_us > end_us {
            return None;
        }
        let selected: Vec<_> = self
            .polls
            .iter()
            .filter(|p| p.begin_us >= begin_us && p.end_us <= end_us)
            .collect();
        let omitted_polls = selected.len().saturating_sub(MAX_WINDOW_POLLS);
        let polls: Vec<_> = selected.into_iter().skip(omitted_polls).cloned().collect();
        let mut cursor = begin_us;
        let mut largest_unsampled_us = 0;
        for poll in &polls {
            largest_unsampled_us = largest_unsampled_us.max(poll.begin_us - cursor);
            cursor = poll.end_us;
        }
        largest_unsampled_us = largest_unsampled_us.max(end_us - cursor);
        Some(Window {
            clock_id: self.clock_id.clone(),
            generation: self.generation.clone(),
            requested_begin_us: begin_us,
            requested_end_us: end_us,
            monitor_started_us: self.started_us,
            retained_from_us: self
                .discarded_until_us
                .map_or(self.started_us, |t| t.saturating_add(1)),
            omitted_polls,
            largest_unsampled_us,
            polls,
            limitation: LIMITATION.into(),
        })
    }
}

pub fn key_name(code: u8) -> String {
    match code {
        1 => "Mouse left".into(),
        2 => "Mouse right".into(),
        4 => "Mouse middle".into(),
        5 => "Mouse X1".into(),
        6 => "Mouse X2".into(),
        8 => "Backspace".into(),
        9 => "Tab".into(),
        13 => "Enter".into(),
        27 => "Escape".into(),
        32 => "Space".into(),
        37 => "Left".into(),
        38 => "Up".into(),
        39 => "Right".into(),
        40 => "Down".into(),
        160 => "Left Shift".into(),
        161 => "Right Shift".into(),
        162 => "Left Ctrl".into(),
        163 => "Right Ctrl".into(),
        164 => "Left Alt".into(),
        165 => "Right Alt".into(),
        48..=57 | 65..=90 => char::from(code).to_string(),
        112..=135 => format!("F{}", code - 111),
        _ => format!("Key {code:02X}"),
    }
}

impl Window {
    pub fn brief(&self) -> String {
        let keys: std::collections::BTreeSet<_> = self
            .polls
            .iter()
            .flat_map(|p| p.down.iter().copied())
            .collect();
        if keys.is_empty() {
            if self
                .polls
                .iter()
                .any(|p| p.focus == Focus::SelectedForeground)
            {
                "No pressed keys detected in these samples".into()
            } else {
                "Input context unavailable for the selected window".into()
            }
        } else {
            let names = keys
                .iter()
                .take(8)
                .map(|k| key_name(*k))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "Sampled controls: {names}{}",
                if keys.len() > 8 { "…" } else { "" }
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn poll(begin_us: u64, focus: Focus, down: Vec<u8>) -> Poll {
        Poll {
            begin_us,
            end_us: begin_us + 1,
            focus,
            down,
        }
    }
    #[test]
    fn foreign_keys_and_bad_polls_are_rejected_atomically() {
        let mut b = Buffer::new("clock".into(), "generation".into(), 5);
        assert!(b.push(poll(4, Focus::SelectedForeground, vec![])).is_err());
        assert!(b.push(poll(10, Focus::OutsideSelected, vec![65])).is_err());
        assert!(
            b.push(poll(10, Focus::SelectedForeground, vec![65, 65]))
                .is_err()
        );
        b.push(poll(10, Focus::SelectedForeground, vec![65]))
            .unwrap();
        assert!(b.push(poll(10, Focus::SelectedForeground, vec![])).is_err());
        let w = b.window(0, 30).unwrap();
        assert_eq!(w.polls.len(), 1);
        assert_eq!(w.largest_unsampled_us, 19);
        assert_eq!(w.brief(), "Sampled controls: A");
    }
    #[test]
    fn focus_gaps_and_empty_windows_never_invent_idle_or_key_releases() {
        let mut b = Buffer::new("clock".into(), "generation".into(), 0);
        b.push(poll(10, Focus::SelectedForeground, vec![32]))
            .unwrap();
        b.push(poll(30, Focus::OutsideSelected, vec![])).unwrap();
        b.push(poll(50, Focus::SelectedForeground, vec![32]))
            .unwrap();
        let w = b.window(20, 60).unwrap();
        assert_eq!(w.polls.len(), 2);
        assert_eq!(w.largest_unsampled_us, 19);
        assert_eq!(w.polls[0].focus, Focus::OutsideSelected);
        let absent = b.window(60, 90).unwrap();
        assert_eq!(absent.largest_unsampled_us, 30);
        assert!(absent.brief().contains("unavailable"));
        assert!(b.window(90, 60).is_none());
    }
    #[test]
    fn bounded_history_and_window_truncation_are_explicit() {
        let mut b = Buffer::new("clock".into(), "generation".into(), 0);
        for i in 0..MAX_POLLS + 20 {
            b.push(poll(i as u64 * 10, Focus::SelectedForeground, vec![]))
                .unwrap();
        }
        let w = b.window(0, 50_000).unwrap();
        assert_eq!(w.polls.len(), MAX_WINDOW_POLLS);
        assert_eq!(w.omitted_polls, MAX_POLLS - MAX_WINDOW_POLLS);
        assert_eq!(w.retained_from_us, 192);
        assert!(w.largest_unsampled_us > 30_000);
    }
}
