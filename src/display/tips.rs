//! Tips display module for Grok CLI
//!
//! Handles display of helpful tips and getting started information

use colored::*;
use rand::prelude::IndexedMutRandom;
use rand::seq::SliceRandom;

/// Collection of helpful tips for users
pub static GETTING_STARTED_TIPS: &[&str] = &[
    "Ask questions, edit files, or run commands.",
    "Be specific for the best results.",
    "/help for more information.",
    "Use 'grok code explain <file>' to understand code.",
    "Use 'grok code review <file>' for code improvements.",
    "Use 'grok chat --interactive' for ongoing conversations.",
    "Set GROK_API_KEY environment variable to avoid typing --api-key.",
    "Use 'grok config show' to view your current configuration.",
    "Create a config.toml file to customize default settings.",
];

/// Advanced usage tips
pub static ADVANCED_TIPS: &[&str] = &[
    "Use system prompts with 'grok chat --system \"You are a...\"' for specialized assistance.",
    "Adjust temperature with --temperature for more creative or focused responses.",
    "Use 'grok acp server' to integrate with Zed editor.",
    "Pipe input to Grok CLI: 'cat file.py | grok chat \"Explain this code\"'.",
    "Use 'grok health --all' to diagnose connectivity issues.",
    "Set custom models with --model flag (e.g., grok-3, grok-2-latest).",
    "Use 'grok code generate' to create new code from descriptions.",
];

/// Productivity tips
pub static PRODUCTIVITY_TIPS: &[&str] = &[
    "Create shell aliases: alias gc='grok chat' for faster access.",
    "Use tab completion in your shell for Grok CLI commands.",
    "Combine with other CLI tools: 'git diff | grok chat \"Review these changes\"'.",
    "Set up your preferred editor integration for seamless workflow.",
    "Use 'grok config set' to save frequently used settings.",
];

/// Troubleshooting tips
pub static TROUBLESHOOTING_TIPS: &[&str] = &[
    "Having connection issues? Check 'grok health --api' for diagnostics.",
    "API key not working? Verify it's set correctly with 'grok config show'.",
    "Slow responses? Try reducing --max-tokens or adjusting --timeout.",
    "Use --verbose flag to see detailed debug information.",
    "Check your internet connection - Starlink users may experience drops.",
];

/// Configuration for tip display
#[derive(Debug, Clone)]
pub struct TipConfig {
    pub show_tips: bool,
    pub randomize: bool,
    pub max_tips: usize,
    pub width: Option<u16>,
}

impl Default for TipConfig {
    fn default() -> Self {
        Self {
            show_tips: true,
            randomize: true,
            max_tips: 3,
            width: None,
        }
    }
}

/// Print getting started tips
pub fn print_getting_started_tips(config: &TipConfig) {
    if !config.show_tips {
        return;
    }

    println!("{}", "Tips for getting started:".bright_cyan());

    let tips = if config.randomize {
        get_random_tips(GETTING_STARTED_TIPS, config.max_tips)
    } else {
        GETTING_STARTED_TIPS
            .iter()
            .take(config.max_tips)
            .cloned()
            .collect()
    };

    for (i, tip) in tips.iter().enumerate() {
        println!("{}. {}", (i + 1).to_string().bright_white(), tip);
    }
    println!();
}

/// Print advanced tips for experienced users
pub fn print_advanced_tips(config: &TipConfig) {
    if !config.show_tips {
        return;
    }

    println!("{}", "Advanced tips:".bright_magenta());

    let tips = if config.randomize {
        get_random_tips(ADVANCED_TIPS, config.max_tips)
    } else {
        ADVANCED_TIPS
            .iter()
            .take(config.max_tips)
            .cloned()
            .collect()
    };

    for tip in tips {
        println!("• {}", tip);
    }
    println!();
}

/// Print productivity tips
pub fn print_productivity_tips(config: &TipConfig) {
    if !config.show_tips {
        return;
    }

    println!("{}", "Productivity tips:".bright_green());

    let tips = if config.randomize {
        get_random_tips(PRODUCTIVITY_TIPS, config.max_tips)
    } else {
        PRODUCTIVITY_TIPS
            .iter()
            .take(config.max_tips)
            .cloned()
            .collect()
    };

    for tip in tips {
        println!("💡 {}", tip);
    }
    println!();
}

/// Print troubleshooting tips
pub fn print_troubleshooting_tips(config: &TipConfig) {
    if !config.show_tips {
        return;
    }

    println!("{}", "Troubleshooting tips:".bright_yellow());

    let tips = if config.randomize {
        get_random_tips(TROUBLESHOOTING_TIPS, config.max_tips)
    } else {
        TROUBLESHOOTING_TIPS
            .iter()
            .take(config.max_tips)
            .cloned()
            .collect()
    };

    for tip in tips {
        println!("🔧 {}", tip);
    }
    println!();
}

/// Get random tips from a collection
fn get_random_tips<'a>(tips: &'a [&'a str], count: usize) -> Vec<&'a str> {
    let mut rng = rand::rng();
    let mut selected_tips: Vec<&str> = tips.to_vec();
    selected_tips.shuffle(&mut rng);
    selected_tips.into_iter().take(count).collect()
}

/// Print a tip of the day
pub fn print_tip_of_the_day() {
    let mut all_tips: Vec<&str> = GETTING_STARTED_TIPS
        .iter()
        .chain(ADVANCED_TIPS.iter())
        .chain(PRODUCTIVITY_TIPS.iter())
        .chain(TROUBLESHOOTING_TIPS.iter())
        .cloned()
        .collect();

    if let Some(tip) = all_tips.choose_mut(&mut rand::rng()) {
        println!("{} {}", "💡 Tip of the day:".bright_yellow(), tip);
        println!();
    }
}

/// Print contextual tips based on current operation
pub fn print_contextual_tips(context: &str, config: &TipConfig) {
    if !config.show_tips {
        return;
    }

    match context {
        "first-run" => print_getting_started_tips(config),
        "error" => print_troubleshooting_tips(config),
        "config" => print_config_tips(config),
        "code" => print_code_tips(config),
        "chat" => print_chat_tips(config),
        _ => print_getting_started_tips(config),
    }
}

/// Print configuration-specific tips
fn print_config_tips(config: &TipConfig) {
    println!("{}", "Configuration tips:".bright_blue());
    let tips = vec![
        "Use 'grok config init' to create a default configuration file",
        "Set your preferred model with 'grok config set default_model grok-3'",
        "Configure API settings with 'grok config set api_key <your-key>'",
        "Use 'grok config validate' to check your configuration",
    ];

    let display_tips = if config.randomize {
        get_random_tips(&tips, config.max_tips)
    } else {
        tips.iter().take(config.max_tips).copied().collect()
    };

    for tip in display_tips {
        println!("⚙️ {}", tip);
    }
    println!();
}

/// Print code-specific tips
fn print_code_tips(config: &TipConfig) {
    println!("{}", "Code assistance tips:".bright_purple());
    let tips = vec![
        "Use 'grok code explain' to understand complex code",
        "Get code reviews with 'grok code review --focus security'",
        "Generate code with 'grok code generate --language rust'",
        "Fix issues with 'grok code fix <file> \"description of problem\"'",
    ];

    let display_tips = if config.randomize {
        get_random_tips(&tips, config.max_tips)
    } else {
        tips.iter().take(config.max_tips).copied().collect()
    };

    for tip in display_tips {
        println!("💻 {}", tip);
    }
    println!();
}

/// Print chat-specific tips
fn print_chat_tips(config: &TipConfig) {
    println!("{}", "Chat tips:".bright_cyan());
    let tips = vec![
        "Use --interactive for ongoing conversations",
        "Set system prompts with --system for specialized help",
        "Adjust creativity with --temperature (0.1 = focused, 1.5 = creative)",
        "Use --max-tokens to control response length",
    ];

    let display_tips = if config.randomize {
        get_random_tips(&tips, config.max_tips)
    } else {
        tips.iter().take(config.max_tips).copied().collect()
    };

    for tip in display_tips {
        println!("💬 {}", tip);
    }
    println!();
}

/// Print a formatted help section
pub fn print_help_section(title: &str, items: &[(&str, &str)]) {
    println!("{}", title.bright_cyan().bold());
    println!();

    for (command, description) in items {
        println!("  {:<25} {}", command.bright_white(), description);
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_random_tips() {
        let tips = ["tip1", "tip2", "tip3", "tip4", "tip5"];
        let random_tips = get_random_tips(&tips, 3);
        assert_eq!(random_tips.len(), 3);

        // All returned tips should be from the original set
        for tip in random_tips {
            assert!(tips.contains(&tip));
        }
    }

    #[test]
    fn test_tip_config_default() {
        let config = TipConfig::default();
        assert!(config.show_tips);
        assert!(config.randomize);
        assert_eq!(config.max_tips, 3);
        assert!(config.width.is_none());
    }

    #[test]
    fn test_tips_not_empty() {
        assert!(!GETTING_STARTED_TIPS.is_empty());
        assert!(!ADVANCED_TIPS.is_empty());
        assert!(!PRODUCTIVITY_TIPS.is_empty());
        assert!(!TROUBLESHOOTING_TIPS.is_empty());
    }
}
