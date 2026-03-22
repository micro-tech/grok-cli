use crate::config::OperationalMode;
use crate::main::switch_mode;
use crate::settings::generate_prompt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_switching() {
        switch_mode(OperationalMode::Research);
        // Assert the mode is switched (in a real test, you'd check the global state)
        assert_eq!(*crate::main::CURRENT_MODE.lock().unwrap(), OperationalMode::Research);
    }

    #[test]
    fn test_prompt_integration() {
        let base_prompt = "Test prompt";
        switch_mode(OperationalMode::Coder);
        let coder_prompt = generate_prompt(base_prompt);
        assert!(coder_prompt.contains("Coder mode"));

        switch_mode(OperationalMode::Creative);
        let creative_prompt = generate_prompt(base_prompt);
        assert!(creative_prompt.contains("Creative mode"));
    }
}