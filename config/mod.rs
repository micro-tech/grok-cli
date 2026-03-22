use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum OperationalMode {
    Coder,
    Research,
    Shell,
    Creative,
}

#[derive(Debug, Clone)]
pub struct ModeConfig {
    pub verbosity: u8,  // 0 = low, 1 = medium, 2 = high
    pub tools: Vec<String>,  // List of preferred tools
    pub reasoning_depth: u8,  // 0 = shallow, 1 = deep
}

pub fn get_mode_configs() -> HashMap<OperationalMode, ModeConfig> {
    let mut configs = HashMap::new();
    configs.insert(OperationalMode::Coder, ModeConfig {
        verbosity: 1,
        tools: vec!["code_editor".to_string(), "debugger".to_string()],
        reasoning_depth: 1,
    });
    configs.insert(OperationalMode::Research, ModeConfig {
        verbosity: 2,
        tools: vec!["web_search".to_string(), "web_fetch".to_string()],
        reasoning_depth: 2,
    });
    configs.insert(OperationalMode::Shell, ModeConfig {
        verbosity: 0,
        tools: vec!["run_shell_command".to_string()],
        reasoning_depth: 0,
    });
    configs.insert(OperationalMode::Creative, ModeConfig {
        verbosity: 2,
        tools: vec!["web_search".to_string()],
        reasoning_depth: 2,
    });
    configs
}