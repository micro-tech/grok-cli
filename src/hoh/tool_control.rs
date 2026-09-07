//! Tool Exposure Control (Task 297.9)

use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct ToolControl {
    allowed: HashSet<String>,
    denied: HashSet<String>,
}

impl ToolControl {
    pub fn allow(&mut self, tool: &str) {
        self.allowed.insert(tool.to_string());
    }

    pub fn deny(&mut self, tool: &str) {
        self.denied.insert(tool.to_string());
    }

    pub fn is_allowed(&self, tool: &str) -> bool {
        if self.denied.contains(tool) {
            return false;
        }
        if !self.allowed.is_empty() {
            return self.allowed.contains(tool);
        }
        true // default open for now
    }
}