//! Intent Validator for File Editing Skill
//!
//! Rejects ambiguous instructions and forces clarification.

pub struct IntentValidator;

impl IntentValidator {
    /// Returns Ok(()) if the request is clear, Err(clarification_question) otherwise
    pub fn validate_intent(request: &str, target_file: Option<&str>) -> Result<(), String> {
        let lower = request.to_lowercase();

        if lower.contains("fix the bug") && target_file.is_none() {
            return Err("Which file? What bug? Show me the error.".to_string());
        }

        if lower.contains("edit this file") && target_file.is_none() {
            return Err("Please specify the target file path.".to_string());
        }

        if lower.contains("make it better") {
            return Err("What specifically should be improved? (performance, readability, etc.)".to_string());
        }

        Ok(())
    }
}
