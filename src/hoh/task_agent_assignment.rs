//! HOH Task Agent Assignment (Task 327.23)
//!
//! Matches tasks to agent profiles based on keyword capability scoring.

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilityProfile {
    pub name: String,
    pub keywords: Vec<String>,
    pub max_concurrent: usize,
}

impl AgentCapabilityProfile {
    pub fn default_profiles() -> Vec<Self> {
        vec![
            Self { name: "Architect".into(),
                keywords: vec!["design","architecture","module","layer","interface","api","refactor","trait","struct"].into_iter().map(String::from).collect(),
                max_concurrent: 2 },
            Self { name: "Coder".into(),
                keywords: vec!["implement","add","create","write","build","integrate","fix","parse","serialize"].into_iter().map(String::from).collect(),
                max_concurrent: 4 },
            Self { name: "Tester".into(),
                keywords: vec!["test","coverage","assert","benchmark","validate","verify","regression","integration"].into_iter().map(String::from).collect(),
                max_concurrent: 3 },
            Self { name: "Researcher".into(),
                keywords: vec!["research","analyze","investigate","explore","study","document","literature","survey"].into_iter().map(String::from).collect(),
                max_concurrent: 2 },
            Self { name: "Governor".into(),
                keywords: vec!["security","safety","governance","policy","risk","compliance","audit","guardrail"].into_iter().map(String::from).collect(),
                max_concurrent: 1 },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub task_id: u64,
    pub agent_name: String,
    pub confidence: f32,
    pub matched_keywords: Vec<String>,
}

/// Score how well a profile matches a task.
fn profile_score(profile: &AgentCapabilityProfile, task: &Task) -> (f32, Vec<String>) {
    let text = format!("{} {} {}", task.title, task.description, task.details).to_lowercase();
    let mut matched = Vec::new();
    for kw in &profile.keywords {
        if text.contains(kw.as_str()) { matched.push(kw.clone()); }
    }
    let score = if profile.keywords.is_empty() { 0.0 }
    else { matched.len() as f32 / profile.keywords.len() as f32 };
    (score, matched)
}

/// Assign each task to the best-matching agent profile.
pub fn assign_tasks(tasks: &[Task], profiles: &[AgentCapabilityProfile]) -> Vec<TaskAssignment> {
    tasks.iter().map(|task| {
        let best = profiles.iter()
            .map(|p| { let (s, m) = profile_score(p, task); (p, s, m) })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        match best {
            Some((profile, score, matched)) if score > 0.0 => TaskAssignment {
                task_id: task.id,
                agent_name: profile.name.clone(),
                confidence: score,
                matched_keywords: matched,
            },
            _ => TaskAssignment {
                task_id: task.id,
                agent_name: "Coder".to_string(), // default fallback
                confidence: 0.1,
                matched_keywords: Vec::new(),
            },
        }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u64, title: &str) -> Task {
        Task { id, title: title.to_string(), ..Default::default() }
    }

    #[test]
    fn test_security_task_assigned_to_governor() {
        let profiles = AgentCapabilityProfile::default_profiles();
        let tasks = vec![t(1, "Add security guardrails and audit logging")];
        let assignments = assign_tasks(&tasks, &profiles);
        assert_eq!(assignments[0].agent_name, "Governor");
    }

    #[test]
    fn test_test_task_assigned_to_tester() {
        let profiles = AgentCapabilityProfile::default_profiles();
        let tasks = vec![t(2, "Add integration test coverage for all modules")];
        let assignments = assign_tasks(&tasks, &profiles);
        assert_eq!(assignments[0].agent_name, "Tester");
    }
}
