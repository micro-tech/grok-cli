//! Multi-Day Scheduler (Task 297.12)

use crate::hoh::state::IterationState;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Scheduler {
    pub last_run: Option<u64>,
    pub interval_hours: u32,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            last_run: None,
            interval_hours: 24,
        }
    }
}

impl Scheduler {
    pub fn should_run(&self, now: u64) -> bool {
        match self.last_run {
            None => true,
            Some(last) => now > last + (self.interval_hours as u64 * 3600),
        }
    }

    pub async fn schedule_next(&mut self, state: &IterationState) {
        self.last_run = Some(state.started_at);
        // TODO: persist schedule
    }
}