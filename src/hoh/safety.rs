//! HOH Safety Guardrails (Task 297.14)

#[derive(Debug, Clone, Default)]
pub struct SafetyGuardrails {
    pub max_changes_per_iteration: u32,
    pub forbidden_paths: Vec<String>,
    pub require_approval_for: Vec<String>,
}

impl SafetyGuardrails {
    pub fn is_action_allowed(&self, _action: &str, path: Option<&str>) -> bool {
        if let Some(p) = path {
            for forbidden in &self.forbidden_paths {
                if p.contains(forbidden) {
                    return false;
                }
            }
        }
        true // MVP permissive
    }
}