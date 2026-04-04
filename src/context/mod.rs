//! Context Engine 2.0
//!
//! Provides structured context layers for enhanced agent reasoning and prompt management.
//!
//! Context Layers:
//! - Session Context: Current user task, intent, recent commands
//! - Working Memory: Short-term facts inferred or extracted
//! - Tool Context: Recent tool calls, results, and errors
//! - Skill Context: Used skills with confidence and arbitration scores
//! - Belief State: Bayesian reasoning probabilities and uncertainty

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the current user session context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionContext {
    /// Current task the user is working on
    pub task: Option<String>,
    /// User's intent inferred from recent interactions
    pub intent: Option<String>,
    /// Recent commands or actions
    pub last_commands: Vec<String>,
    /// Session start time
    pub session_start: Option<std::time::SystemTime>,
}

/// Short-term working memory for inferred facts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkingMemory {
    /// Inferred facts with timestamps
    pub facts: Vec<MemoryFact>,
    /// Maximum number of facts to retain
    pub max_facts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFact {
    pub fact: String,
    pub timestamp: std::time::SystemTime,
    pub confidence: f64, // 0.0 to 1.0
}

/// Context from recent tool executions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolContext {
    /// Recent tool calls with results
    pub recent_calls: Vec<ToolCallRecord>,
    /// Maximum calls to retain
    pub max_calls: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: Option<String>,
    pub error: Option<String>,
    pub timestamp: std::time::SystemTime,
    pub duration_ms: u64,
}

/// Context from skill usage and arbitration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillContext {
    /// Recently used skills with metadata
    pub recent_skills: Vec<SkillUsage>,
    /// Current arbitration scores
    pub arbitration_scores: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUsage {
    pub skill_name: String,
    pub confidence: f64,
    pub arbitration_score: f64,
    pub timestamp: std::time::SystemTime,
    pub successful: bool,
}

/// Belief state from Bayesian reasoning
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeliefState {
    /// Probability distributions
    pub probabilities: HashMap<String, f64>,
    /// Uncertainty measures
    pub uncertainties: HashMap<String, f64>,
    /// Last update timestamp
    pub last_update: Option<std::time::SystemTime>,
}

/// Main context engine coordinating all layers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEngine {
    pub session: SessionContext,
    pub working_memory: WorkingMemory,
    pub tool_context: ToolContext,
    pub skill_context: SkillContext,
    pub belief_state: BeliefState,
}

impl Default for ContextEngine {
    fn default() -> Self {
        Self {
            session: Default::default(),
            working_memory: WorkingMemory {
                facts: Vec::new(),
                max_facts: 50,
            },
            tool_context: ToolContext {
                recent_calls: Vec::new(),
                max_calls: 20,
            },
            skill_context: Default::default(),
            belief_state: Default::default(),
        }
    }
}

impl ContextEngine {
    /// Create a new context engine
    pub fn new() -> Self {
        Default::default()
    }

    /// Update session context
    pub fn update_session(&mut self, task: Option<String>, intent: Option<String>) {
        self.session.task = task;
        self.session.intent = intent;
    }

    /// Add a fact to working memory
    pub fn add_fact(&mut self, fact: String, confidence: f64) {
        let memory_fact = MemoryFact {
            fact,
            timestamp: std::time::SystemTime::now(),
            confidence,
        };
        self.working_memory.facts.push(memory_fact);
        // Trim to max_facts
        if self.working_memory.facts.len() > self.working_memory.max_facts {
            self.working_memory.facts.remove(0);
        }
    }

    /// Record a tool call
    pub fn record_tool_call(&mut self, tool_name: String, arguments: serde_json::Value, result: Option<String>, error: Option<String>, duration_ms: u64) {
        let record = ToolCallRecord {
            tool_name,
            arguments,
            result,
            error,
            timestamp: std::time::SystemTime::now(),
            duration_ms,
        };
        self.tool_context.recent_calls.push(record);
        // Trim to max_calls
        if self.tool_context.recent_calls.len() > self.tool_context.max_calls {
            self.tool_context.recent_calls.remove(0);
        }
    }

    /// Record skill usage
    pub fn record_skill_usage(&mut self, skill_name: String, confidence: f64, arbitration_score: f64, successful: bool) {
        let usage = SkillUsage {
            skill_name: skill_name.clone(),
            confidence,
            arbitration_score,
            timestamp: std::time::SystemTime::now(),
            successful,
        };
        self.skill_context.recent_skills.push(usage);
        self.skill_context.arbitration_scores.insert(skill_name, arbitration_score);
    }

    /// Update belief state
    pub fn update_belief_state(&mut self, probabilities: HashMap<String, f64>, uncertainties: HashMap<String, f64>) {
        self.belief_state.probabilities = probabilities;
        self.belief_state.uncertainties = uncertainties;
        self.belief_state.last_update = Some(std::time::SystemTime::now());
    }

    /// Generate a summarized context string for prompt injection
    pub fn summarize_for_prompt(&self) -> String {
        let mut summary = String::new();

        // Session context
        if let Some(task) = &self.session.task {
            summary.push_str(&format!("Current Task: {}\n", task));
        }
        if let Some(intent) = &self.session.intent {
            summary.push_str(&format!("User Intent: {}\n", intent));
        }
        if !self.session.last_commands.is_empty() {
            summary.push_str(&format!("Recent Commands: {}\n", self.session.last_commands.join(", ")));
        }

        // Working memory (top 5 facts)
        if !self.working_memory.facts.is_empty() {
            summary.push_str("Working Memory Facts:\n");
            for fact in self.working_memory.facts.iter().rev().take(5) {
                summary.push_str(&format!("- {} (confidence: {:.2})\n", fact.fact, fact.confidence));
            }
        }

        // Tool context (recent errors/successes)
        let recent_tools: Vec<_> = self.tool_context.recent_calls.iter().rev().take(5).collect();
        if !recent_tools.is_empty() {
            summary.push_str("Recent Tool Usage:\n");
            for call in recent_tools {
                let status = if call.error.is_some() { "ERROR" } else { "SUCCESS" };
                summary.push_str(&format!("- {}: {} ({}ms)\n", call.tool_name, status, call.duration_ms));
            }
        }

        // Skill context
        if !self.skill_context.recent_skills.is_empty() {
            summary.push_str("Recent Skills Used:\n");
            for skill in self.skill_context.recent_skills.iter().rev().take(3) {
                summary.push_str(&format!("- {} (confidence: {:.2}, score: {:.2})\n", skill.skill_name, skill.confidence, skill.arbitration_score));
            }
        }

        // Belief state (top uncertainties)
        if !self.belief_state.uncertainties.is_empty() {
            summary.push_str("High Uncertainty Areas:\n");
            let mut uncertains: Vec<_> = self.belief_state.uncertainties.iter().collect();
            uncertains.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
            for (key, uncertainty) in uncertains.into_iter().take(3) {
                summary.push_str(&format!("- {}: {:.2}\n", key, uncertainty));
            }
        }

        summary
    }

    /// Get context status for debugging
    pub fn status(&self) -> String {
        format!(
            "Session: task={:?}, intent={:?}, commands={}\nWorking Memory: {} facts\nTool Context: {} calls\nSkill Context: {} skills\nBelief State: {} probabilities",
            self.session.task,
            self.session.intent,
            self.session.last_commands.len(),
            self.working_memory.facts.len(),
            self.tool_context.recent_calls.len(),
            self.skill_context.recent_skills.len(),
            self.belief_state.probabilities.len()
        )
    }
}