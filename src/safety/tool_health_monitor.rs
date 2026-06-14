//! Tool Health Monitor
//!
//! Tracks per-tool metrics and can disable unhealthy tools.

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub struct ToolMetrics {
    pub success_count: u64,
    pub failure_count: u64,
    pub hallucination_count: u64,
    pub invalid_diff_count: u64,
}

pub struct ToolHealthMonitor {
    metrics: Mutex<HashMap<String, ToolMetrics>>,
}

impl ToolHealthMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Mutex::new(HashMap::new()),
        }
    }

    pub fn record_success(&self, tool: &str) {
        let mut map = self.metrics.lock().unwrap();
        map.entry(tool.to_string()).or_default().success_count += 1;
    }

    pub fn record_failure(&self, tool: &str) {
        let mut map = self.metrics.lock().unwrap();
        map.entry(tool.to_string()).or_default().failure_count += 1;
    }

    pub fn is_healthy(&self, tool: &str) -> bool {
        let map = self.metrics.lock().unwrap();
        if let Some(m) = map.get(tool) {
            let total = m.success_count + m.failure_count;
            if total < 5 {
                return true;
            }
            let failure_rate = m.failure_count as f32 / total as f32;
            failure_rate < 0.35
        } else {
            true
        }
    }
}
