use crate::config::OperationalMode;  // Import from config
use crate::main::CURRENT_MODE;  // Assuming this is defined in main.rs

pub fn generate_prompt(base_prompt: &str) -> String {
    let mode = CURRENT_MODE.lock().unwrap().clone();  // Get current mode
    match mode {
        OperationalMode::Coder => format!("{} (Coder mode: Keep it concise and tool-focused)", base_prompt),
        OperationalMode::Research => format!("{} (Research mode: Expand with citations and depth)", base_prompt),
        OperationalMode::Shell => format!("{} (Shell mode: Terse and command-oriented)", base_prompt),
        OperationalMode::Creative => format!("{} (Creative mode: Open-ended and exploratory)", base_prompt),
    }
}

// Example integration in existing functions
pub fn handle_user_input(input: &str) {
    let prompt = generate_prompt(input);  // Use mode-adjusted prompt
    // Proceed with prompt usage in your system
}