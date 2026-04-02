use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Operational mode for the Grok CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Coder,
    Research,
    Shell,
    Creative,
}

impl Mode {
    /// Get the system prompt additions for this mode
    pub fn system_prompt_additions(&self) -> &'static str {
        match self {
            Mode::Coder => {
                "CODER MODE: Be concise, structured, and prioritize tool usage. Focus on writing, fixing, or analyzing code. Provide clear implementations."
            }
            Mode::Research => {
                "RESEARCH MODE: Provide long-form reasoning, detailed explanations, and cite sources or files when appropriate. Focus on deep understanding rather than immediate action."
            }
            Mode::Shell => {
                "SHELL MODE: Be terse and command-first. Prioritize providing shell commands and scripts to solve the user's problem. Minimize explanations unless asked."
            }
            Mode::Creative => {
                "CREATIVE MODE: Be open-ended, generative, and brainstorming-focused. Explore multiple solutions and out-of-the-box ideas."
            }
        }
    }

    /// Get a brief description of the mode
    pub fn description(&self) -> &'static str {
        match self {
            Mode::Coder => "Concise, structured, tool-heavy",
            Mode::Research => "Long-form reasoning, citations",
            Mode::Shell => "Terse, command-first",
            Mode::Creative => "Open-ended, generative",
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Coder => write!(f, "Coder"),
            Mode::Research => write!(f, "Research"),
            Mode::Shell => write!(f, "Shell"),
            Mode::Creative => write!(f, "Creative"),
        }
    }
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "coder" => Ok(Mode::Coder),
            "research" => Ok(Mode::Research),
            "shell" => Ok(Mode::Shell),
            "creative" => Ok(Mode::Creative),
            _ => Err(format!(
                "Unknown mode: {}. Valid modes: coder, research, shell, creative",
                s
            )),
        }
    }
}
