//! Settings command handler for grok-cli
//!
//! Provides an interactive settings interface similar to Gemini CLI's `/settings` command.
//! Allows users to browse and modify configuration settings in categories.

// Allow deprecated warnings in this module since these I/O functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users and documentation purposes.
#![allow(deprecated)]

use anyhow::{Result, anyhow};
use colored::*;
use std::io::{self, Write};

use crate::SettingsAction;
use crate::cli::{confirm, print_error, print_info, print_success, print_warning};
use crate::config::Config;

/// Setting definition for interactive display
#[derive(Debug, Clone)]
pub struct SettingDefinition {
    pub key: String,
    pub label: String,
    pub description: String,
    pub category: String,
    pub setting_type: SettingType,
    pub default_value: String,
    pub requires_restart: bool,
    pub current_value: String,
}

#[derive(Debug, Clone)]
pub enum SettingType {
    Boolean,
    String,
    Number,
    #[allow(dead_code)]
    Array,
    #[allow(dead_code)]
    Object,
}

/// Handle settings-related commands
pub async fn handle_settings_action(action: SettingsAction, config: &Config) -> Result<()> {
    match action {
        SettingsAction::Show | SettingsAction::Edit => {
            let settings = get_all_settings(config);
            crate::display::components::settings_list::run_settings_tui(config, settings).await
        }
        SettingsAction::Reset { category } => reset_settings(category).await,
        SettingsAction::Export { path } => export_settings(config, path).await,
        SettingsAction::Import { path } => import_settings(path).await,
    }
}

/// Show interactive settings UI (Legacy - now forwarded to TUI)
#[allow(dead_code)]
async fn show_settings_ui(config: &Config) -> Result<()> {
    println!("{}", "⚙️  Grok CLI Settings".cyan().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━".cyan());
    println!();

    let settings = get_all_settings(config);
    let categories = get_categories(&settings);

    loop {
        println!("{}", "Categories:".green().bold());
        for (i, category) in categories.iter().enumerate() {
            println!("  {}. {}", (i + 1).to_string().yellow(), category);
        }
        println!();
        println!("  {}. Quit", "q".red());
        println!();

        print!("{} ", "Select category (1-{}, q): ".blue().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "q" || input == "quit" {
            break;
        }

        if let Ok(choice) = input.parse::<usize>() {
            if choice > 0 && choice <= categories.len() {
                let category = &categories[choice - 1];
                show_category_settings(category, &settings, config).await?;
            } else {
                print_warning("Invalid selection. Please try again.");
            }
        } else {
            print_warning("Invalid input. Please enter a number or 'q'.");
        }
        println!();
    }

    print_info("Settings browser closed.");
    Ok(())
}

/// Show settings for a specific category
async fn show_category_settings(
    category: &str,
    settings: &[SettingDefinition],
    config: &Config,
) -> Result<()> {
    let category_settings: Vec<_> = settings.iter().filter(|s| s.category == category).collect();

    println!();
    println!("{}", format!("⚙️  {} Settings", category).cyan().bold());
    println!("{}", "━".repeat(50).cyan());
    println!();

    for setting in &category_settings {
        display_setting(setting);
        println!();
    }

    loop {
        println!("{}", "Options:".green().bold());
        println!("  1. Edit a setting");
        println!("  2. Reset category to defaults");
        println!("  3. Back to categories");
        println!();

        print!("{} ", "Choose option (1-3): ".blue().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" => {
                edit_category_setting(&category_settings, config).await?;
            }
            "2" => {
                if confirm(&format!("Reset all {} settings to defaults?", category))? {
                    reset_settings(Some(category.to_lowercase())).await?;
                    // Note: This won't refresh the current view's values unless we reload config,
                    // but for simplicity we just return to category menu which is acceptable.
                    break;
                }
            }
            "3" => break,
            _ => print_warning("Invalid selection. Please try again."),
        }
        println!();
    }

    Ok(())
}

/// Display a single setting
fn display_setting(setting: &SettingDefinition) {
    println!("{}", setting.label.green().bold());
    println!("  Key: {}", setting.key.yellow());
    println!(
        "  Current: {}",
        format_setting_value(&setting.current_value, &setting.setting_type)
    );
    println!(
        "  Default: {}",
        format_setting_value(&setting.default_value, &setting.setting_type)
    );
    println!("  Description: {}", setting.description.dimmed());

    if setting.requires_restart {
        println!("  {}", "⚠️  Requires restart".yellow());
    }
}

/// Format setting value for display
pub fn format_setting_value(value: &str, setting_type: &SettingType) -> colored::ColoredString {
    match setting_type {
        SettingType::Boolean => {
            if value == "true" {
                "✓ Enabled".green()
            } else {
                "✗ Disabled".red()
            }
        }
        SettingType::String => {
            if value.is_empty() {
                "\"\" (empty)".dimmed()
            } else {
                format!("\"{}\"", value).cyan()
            }
        }
        SettingType::Number => value.blue(),
        SettingType::Array => {
            if value == "[]" {
                "[] (empty)".dimmed()
            } else {
                value.magenta()
            }
        }
        SettingType::Object => {
            if value == "{{}}" {
                "{{}} (empty)".dimmed()
            } else {
                value.magenta()
            }
        }
    }
}

/// Edit a setting in a category
pub async fn edit_category_setting(settings: &[&SettingDefinition], config: &Config) -> Result<()> {
    println!();
    println!("{}", "Select setting to edit:".green().bold());

    for (i, setting) in settings.iter().enumerate() {
        println!("  {}. {}", (i + 1).to_string().yellow(), setting.label);
    }
    println!();

    print!("{} ", "Select setting (1-{}): ".blue().bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if let Ok(choice) = input.trim().parse::<usize>() {
        if choice > 0 && choice <= settings.len() {
            let setting = settings[choice - 1];
            edit_single_setting(setting, config).await?;
        } else {
            print_warning("Invalid selection.");
        }
    }

    Ok(())
}

/// Edit a single setting
pub async fn edit_single_setting(setting: &SettingDefinition, config: &Config) -> Result<()> {
    println!();
    println!("{}", format!("Editing: {}", setting.label).green().bold());
    println!(
        "Current value: {}",
        format_setting_value(&setting.current_value, &setting.setting_type)
    );
    println!("Description: {}", setting.description.dimmed());
    println!();

    let new_value = match setting.setting_type {
        SettingType::Boolean => edit_boolean_setting(setting)?,
        SettingType::String => edit_string_setting(setting)?,
        SettingType::Number => edit_number_setting(setting)?,
        SettingType::Array => edit_array_setting(setting)?,
        SettingType::Object => {
            print_warning("Object settings must be edited manually in the config file.");
            return Ok(());
        }
    };

    if new_value != setting.current_value {
        // Apply the setting
        let mut updated_config = config.clone();
        updated_config
            .set_value(&setting.key, &new_value)
            .map_err(|e| anyhow!("Failed to set value: {}", e))?;

        // Validate the change
        updated_config.validate()?;

        // Save the config
        updated_config.save(None).await?;

        print_success(&format!("Updated {} = {}", setting.key, new_value));

        if setting.requires_restart {
            print_warning("⚠️  This setting requires a restart to take effect.");
        }
    } else {
        print_info("No changes made.");
    }

    Ok(())
}

/// Edit a boolean setting
fn edit_boolean_setting(setting: &SettingDefinition) -> Result<String> {
    let _current = setting.current_value == "true";

    println!("Options:");
    println!("  1. Enable (true)");
    println!("  2. Disable (false)");
    println!("  3. Keep current value");
    println!();

    print!("{} ", "Choose option (1-3): ".blue().bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim() {
        "1" => Ok("true".to_string()),
        "2" => Ok("false".to_string()),
        "3" => Ok(setting.current_value.clone()),
        _ => {
            print_warning("Invalid selection, keeping current value.");
            Ok(setting.current_value.clone())
        }
    }
}

/// Edit a string setting
fn edit_string_setting(setting: &SettingDefinition) -> Result<String> {
    print!("Enter new value (press Enter to keep current): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        Ok(setting.current_value.clone())
    } else {
        Ok(input.to_string())
    }
}

/// Edit a number setting
fn edit_number_setting(setting: &SettingDefinition) -> Result<String> {
    print!("Enter new value (press Enter to keep current): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        Ok(setting.current_value.clone())
    } else {
        // Validate it's a number
        if input.parse::<f64>().is_ok() {
            Ok(input.to_string())
        } else {
            print_warning("Invalid number format, keeping current value.");
            Ok(setting.current_value.clone())
        }
    }
}

/// Edit an array setting
fn edit_array_setting(setting: &SettingDefinition) -> Result<String> {
    println!("Array editing:");
    println!("Enter comma-separated values (e.g., 'value1,value2,value3')");
    println!("Press Enter to keep current value");
    print!("New values: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        Ok(setting.current_value.clone())
    } else {
        // Convert comma-separated values to JSON array format
        let values: Vec<String> = input
            .split(',')
            .map(|s| format!("\"{}\"", s.trim()))
            .collect();
        Ok(format!(
            "[{}]
",
            values.join(",")
        ))
    }
}

/// Launch interactive settings editor
async fn edit_settings_interactive(config: &Config) -> Result<()> {
    println!("{}", "🔧 Interactive Settings Editor".cyan().bold());
    println!("{}", "Press Ctrl+C at any time to exit".dimmed());
    println!();

    show_settings_ui(config).await
}

/// Reset settings to defaults
async fn reset_settings(category: Option<String>) -> Result<()> {
    match category {
        Some(cat) => {
            print_info(&format!("Resetting {} settings to defaults...", cat));

            if !confirm(&format!(
                "This will reset all {} settings to their default values. Continue?",
                cat
            ))? {
                print_info("Reset cancelled.");
                return Ok(());
            }

            // Load default config and save only the specified category
            let default_config = Config::default();
            let mut current_config = Config::load(None).await?;

            // Reset the specific category
            match cat.to_lowercase().as_str() {
                "general" => current_config.general = default_config.general,
                "ui" => current_config.ui = default_config.ui,
                "model" => current_config.model = default_config.model,
                "context" => current_config.context = default_config.context,
                "tools" => current_config.tools = default_config.tools,
                "security" => current_config.security = default_config.security,
                "experimental" => current_config.experimental = default_config.experimental,
                "acp" => current_config.acp = default_config.acp,
                "network" => current_config.network = default_config.network,
                "logging" => current_config.logging = default_config.logging,
                _ => {
                    print_error(&format!("Unknown category: {}", cat));
                    return Err(anyhow!("Unknown category: {}", cat));
                }
            }

            current_config.save(None).await?;
            print_success(&format!("{} settings reset to defaults.", cat));
        }
        None => {
            print_info("Resetting ALL settings to defaults...");

            if !confirm("This will reset ALL settings to their default values. Continue?")? {
                print_info("Reset cancelled.");
                return Ok(());
            }

            Config::init(true).await?;
            print_success("All settings reset to defaults.");
        }
    }

    Ok(())
}

/// Export settings to a file
async fn export_settings(config: &Config, path: Option<String>) -> Result<()> {
    let export_path = path.unwrap_or_else(|| "grok-settings-export.toml".to_string());

    print_info(&format!("Exporting settings to: {}", export_path));

    let toml_content =
        toml::to_string_pretty(config).map_err(|e| anyhow!("Failed to serialize config: {}", e))?;

    std::fs::write(&export_path, toml_content)
        .map_err(|e| anyhow!("Failed to write export file: {}", e))?;

    print_success(&format!("Settings exported to: {}", export_path));
    Ok(())
}

/// Import settings from a file
async fn import_settings(path: String) -> Result<()> {
    print_info(&format!("Importing settings from: {}", path));

    if !std::path::Path::new(&path).exists() {
        return Err(anyhow!("Import file not found: {}", path));
    }

    let content =
        std::fs::read_to_string(&path).map_err(|e| anyhow!("Failed to read import file: {}", e))?;

    let imported_config: Config =
        toml::from_str(&content).map_err(|e| anyhow!("Failed to parse import file: {}", e))?;

    // Validate imported config
    imported_config.validate()?;

    if confirm("This will replace your current settings. Continue?")? {
        imported_config.save(None).await?;
        print_success("Settings imported successfully.");
    } else {
        print_info("Import cancelled.");
    }

    Ok(())
}

/// Get all settings definitions
pub fn get_all_settings(config: &Config) -> Vec<SettingDefinition> {
    let mut settings = Vec::new();

    // General settings
    settings.extend(vec![
        SettingDefinition {
            key: "general.preview_features".to_string(),
            label: "Preview Features".to_string(),
            description: "Enable preview features (e.g., preview models)".to_string(),
            category: "General".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.general.preview_features.to_string(),
        },
        SettingDefinition {
            key: "general.vim_mode".to_string(),
            label: "Vim Mode".to_string(),
            description: "Enable Vim keybindings".to_string(),
            category: "General".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: true,
            current_value: config.general.vim_mode.to_string(),
        },
        SettingDefinition {
            key: "general.disable_auto_update".to_string(),
            label: "Disable Auto Update".to_string(),
            description: "Disable automatic updates".to_string(),
            category: "General".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.general.disable_auto_update.to_string(),
        },
        SettingDefinition {
            key: "general.disable_update_nag".to_string(),
            label: "Disable Update Nag".to_string(),
            description: "Hide update notification messages".to_string(),
            category: "General".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.general.disable_update_nag.to_string(),
        },
        SettingDefinition {
            key: "general.enable_prompt_completion".to_string(),
            label: "Prompt Completion".to_string(),
            description: "Enable AI-powered prompt completion while typing".to_string(),
            category: "General".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.general.enable_prompt_completion.to_string(),
        },
        SettingDefinition {
            key: "general.retry_fetch_errors".to_string(),
            label: "Retry Fetch Errors".to_string(),
            description: "Automatically retry failed network requests".to_string(),
            category: "General".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.general.retry_fetch_errors.to_string(),
        },
        SettingDefinition {
            key: "general.debug_keystroke_logging".to_string(),
            label: "Debug Keystroke".to_string(),
            description: "Log keystrokes for debugging purposes".to_string(),
            category: "General".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.general.debug_keystroke_logging.to_string(),
        },
    ]);

    // UI settings
    settings.extend(vec![
        SettingDefinition {
            key: "ui.theme".to_string(),
            label: "Theme".to_string(),
            description: "Color theme for the UI".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::String,
            default_value: "default".to_string(),
            requires_restart: false,
            current_value: config.ui.theme.clone(),
        },
        SettingDefinition {
            key: "ui.colors".to_string(),
            label: "Enable Colors".to_string(),
            description: "Enable colored output in terminal".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.ui.colors.to_string(),
        },
        SettingDefinition {
            key: "ui.progress_bars".to_string(),
            label: "Progress Bars".to_string(),
            description: "Show progress indicators during operations".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.ui.progress_bars.to_string(),
        },
        SettingDefinition {
            key: "ui.verbose_errors".to_string(),
            label: "Verbose Errors".to_string(),
            description: "Display detailed error information".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.verbose_errors.to_string(),
        },
        SettingDefinition {
            key: "ui.terminal_width".to_string(),
            label: "Terminal Width".to_string(),
            description: "Terminal width override (0 = auto-detect)".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Number,
            default_value: "0".to_string(),
            requires_restart: false,
            current_value: config.ui.terminal_width.to_string(),
        },
        SettingDefinition {
            key: "ui.unicode".to_string(),
            label: "Unicode Support".to_string(),
            description: "Enable Unicode characters and emojis".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.ui.unicode.to_string(),
        },
        SettingDefinition {
            key: "ui.hide_window_title".to_string(),
            label: "Hide Window Title".to_string(),
            description: "Hide the window title bar".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.hide_window_title.to_string(),
        },
        SettingDefinition {
            key: "ui.show_status_in_title".to_string(),
            label: "Show Status in Title".to_string(),
            description: "Show Grok CLI status in terminal title".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.show_status_in_title.to_string(),
        },
        SettingDefinition {
            key: "ui.hide_tips".to_string(),
            label: "Hide Tips".to_string(),
            description: "Hide helpful tips in the UI".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.hide_tips.to_string(),
        },
        SettingDefinition {
            key: "ui.hide_banner".to_string(),
            label: "Hide Banner".to_string(),
            description: "Hide the application banner".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.hide_banner.to_string(),
        },
        SettingDefinition {
            key: "ui.hide_context_summary".to_string(),
            label: "Hide Context Summary".to_string(),
            description: "Hide context summary above input".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.hide_context_summary.to_string(),
        },
        SettingDefinition {
            key: "ui.hide_footer".to_string(),
            label: "Hide Footer".to_string(),
            description: "Hide the status footer".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.hide_footer.to_string(),
        },
        SettingDefinition {
            key: "ui.show_memory_usage".to_string(),
            label: "Show Memory Usage".to_string(),
            description: "Display memory usage information".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.show_memory_usage.to_string(),
        },
        SettingDefinition {
            key: "ui.show_line_numbers".to_string(),
            label: "Show Line Numbers".to_string(),
            description: "Show line numbers in the chat".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.ui.show_line_numbers.to_string(),
        },
        SettingDefinition {
            key: "ui.show_citations".to_string(),
            label: "Show Citations".to_string(),
            description: "Show citations for generated content".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.show_citations.to_string(),
        },
        SettingDefinition {
            key: "ui.show_model_info_in_chat".to_string(),
            label: "Show Model Info".to_string(),
            description: "Display model name in chat responses".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.show_model_info_in_chat.to_string(),
        },
        SettingDefinition {
            key: "ui.use_full_width".to_string(),
            label: "Use Full Width".to_string(),
            description: "Use the entire width of the terminal for output".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.ui.use_full_width.to_string(),
        },
        SettingDefinition {
            key: "ui.use_alternate_buffer".to_string(),
            label: "Use Alternate Buffer".to_string(),
            description: "Use alternate screen buffer (preserves history)".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.use_alternate_buffer.to_string(),
        },
        SettingDefinition {
            key: "ui.incremental_rendering".to_string(),
            label: "Incremental Rendering".to_string(),
            description: "Enable incremental text rendering".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.incremental_rendering.to_string(),
        },
        SettingDefinition {
            key: "ui.accessibility.disable_loading_phrases".to_string(),
            label: "Disable Loading Phrases".to_string(),
            description: "Disable witty loading phrases".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.accessibility.disable_loading_phrases.to_string(),
        },
        SettingDefinition {
            key: "ui.accessibility.screen_reader".to_string(),
            label: "Screen Reader Mode".to_string(),
            description: "Optimize output for screen readers".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.accessibility.screen_reader.to_string(),
        },
        SettingDefinition {
            key: "ui.footer.hide_cwd".to_string(),
            label: "Hide Footer CWD".to_string(),
            description: "Hide current working directory in footer".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.footer.hide_cwd.to_string(),
        },
        SettingDefinition {
            key: "ui.footer.hide_sandbox_status".to_string(),
            label: "Hide Footer Sandbox".to_string(),
            description: "Hide sandbox status indicator".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.footer.hide_sandbox_status.to_string(),
        },
        SettingDefinition {
            key: "ui.footer.hide_model_info".to_string(),
            label: "Hide Footer Model Info".to_string(),
            description: "Hide model information in footer".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.ui.footer.hide_model_info.to_string(),
        },
        SettingDefinition {
            key: "ui.footer.hide_context_percentage".to_string(),
            label: "Hide Context Percentage".to_string(),
            description: "Hide context usage percentage".to_string(),
            category: "UI".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.ui.footer.hide_context_percentage.to_string(),
        },
    ]);

    // Model settings
    settings.extend(vec![
        SettingDefinition {
            key: "default_model".to_string(),
            label: "Default Model".to_string(),
            description: "Default model to use for requests".to_string(),
            category: "Model".to_string(),
            setting_type: SettingType::String,
            default_value: "grok-4-1-fast-reasoning".to_string(),
            requires_restart: false,
            current_value: config.default_model.clone(),
        },
        SettingDefinition {
            key: "default_temperature".to_string(),
            label: "Default Temperature".to_string(),
            description: "Default temperature for responses (0.0-2.0)".to_string(),
            category: "Model".to_string(),
            setting_type: SettingType::Number,
            default_value: "0.7".to_string(),
            requires_restart: false,
            current_value: config.default_temperature.to_string(),
        },
        SettingDefinition {
            key: "default_max_tokens".to_string(),
            label: "Default Max Tokens".to_string(),
            description: "Default maximum tokens per response".to_string(),
            category: "Model".to_string(),
            setting_type: SettingType::Number,
            default_value: "256000".to_string(),
            requires_restart: false,
            current_value: config.default_max_tokens.to_string(),
        },
        SettingDefinition {
            key: "model.max_session_turns".to_string(),
            label: "Max Session Turns".to_string(),
            description: "Maximum conversation turns (-1 = unlimited)".to_string(),
            category: "Model".to_string(),
            setting_type: SettingType::Number,
            default_value: "-1".to_string(),
            requires_restart: false,
            current_value: config.model.max_session_turns.to_string(),
        },
        SettingDefinition {
            key: "model.compression_threshold".to_string(),
            label: "Compression Threshold".to_string(),
            description: "Context compression threshold (0.1-1.0)".to_string(),
            category: "Model".to_string(),
            setting_type: SettingType::Number,
            default_value: "0.2".to_string(),
            requires_restart: false,
            current_value: config.model.compression_threshold.to_string(),
        },
        SettingDefinition {
            key: "model.skip_next_speaker_check".to_string(),
            label: "Skip Speaker Check".to_string(),
            description: "Skip next speaker validation".to_string(),
            category: "Model".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.model.skip_next_speaker_check.to_string(),
        },
    ]);

    // Context Settings
    settings.extend(vec![
        SettingDefinition {
            key: "context.discovery_max_dirs".to_string(),
            label: "Discovery Max Dirs".to_string(),
            description: "Maximum directories to search for context".to_string(),
            category: "Context".to_string(),
            setting_type: SettingType::Number,
            default_value: "200".to_string(),
            requires_restart: false,
            current_value: config.context.discovery_max_dirs.to_string(),
        },
        SettingDefinition {
            key: "context.load_memory_from_include_directories".to_string(),
            label: "Load Memory from Includes".to_string(),
            description: "Load memory from included directories".to_string(),
            category: "Context".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config
                .context
                .load_memory_from_include_directories
                .to_string(),
        },
        SettingDefinition {
            key: "context.file_filtering.respect_git_ignore".to_string(),
            label: "Respect .gitignore".to_string(),
            description: "Respect .gitignore files".to_string(),
            category: "Context".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.context.file_filtering.respect_git_ignore.to_string(),
        },
        SettingDefinition {
            key: "context.file_filtering.respect_grok_ignore".to_string(),
            label: "Respect .grokignore".to_string(),
            description: "Respect .grokignore files".to_string(),
            category: "Context".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config
                .context
                .file_filtering
                .respect_grok_ignore
                .to_string(),
        },
        SettingDefinition {
            key: "context.file_filtering.enable_recursive_file_search".to_string(),
            label: "Recursive File Search".to_string(),
            description: "Enable recursive file search".to_string(),
            category: "Context".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config
                .context
                .file_filtering
                .enable_recursive_file_search
                .to_string(),
        },
        SettingDefinition {
            key: "context.file_filtering.disable_fuzzy_search".to_string(),
            label: "Disable Fuzzy Search".to_string(),
            description: "Disable fuzzy file matching".to_string(),
            category: "Context".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config
                .context
                .file_filtering
                .disable_fuzzy_search
                .to_string(),
        },
    ]);

    // Tools Settings
    settings.extend(vec![
        SettingDefinition {
            key: "tools.shell.enable_interactive_shell".to_string(),
            label: "Interactive Shell".to_string(),
            description: "Enable interactive shell mode".to_string(),
            category: "Tools".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.tools.shell.enable_interactive_shell.to_string(),
        },
        SettingDefinition {
            key: "tools.shell.show_color".to_string(),
            label: "Shell Colors".to_string(),
            description: "Show colors in shell output".to_string(),
            category: "Tools".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.tools.shell.show_color.to_string(),
        },
        SettingDefinition {
            key: "tools.auto_accept".to_string(),
            label: "Auto Accept Tools".to_string(),
            description: "Automatically accept safe tool executions".to_string(),
            category: "Tools".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.tools.auto_accept.to_string(),
        },
        SettingDefinition {
            key: "tools.use_ripgrep".to_string(),
            label: "Use Ripgrep".to_string(),
            description: "Use ripgrep for faster file searches".to_string(),
            category: "Tools".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.tools.use_ripgrep.to_string(),
        },
        SettingDefinition {
            key: "tools.enable_tool_output_truncation".to_string(),
            label: "Truncate Tool Output".to_string(),
            description: "Truncate large tool outputs".to_string(),
            category: "Tools".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.tools.enable_tool_output_truncation.to_string(),
        },
        SettingDefinition {
            key: "tools.truncate_tool_output_threshold".to_string(),
            label: "Truncation Threshold".to_string(),
            description: "Truncation threshold in characters".to_string(),
            category: "Tools".to_string(),
            setting_type: SettingType::Number,
            default_value: "10000".to_string(),
            requires_restart: false,
            current_value: config.tools.truncate_tool_output_threshold.to_string(),
        },
        SettingDefinition {
            key: "tools.truncate_tool_output_lines".to_string(),
            label: "Truncation Lines".to_string(),
            description: "Lines to keep when truncating".to_string(),
            category: "Tools".to_string(),
            setting_type: SettingType::Number,
            default_value: "100".to_string(),
            requires_restart: false,
            current_value: config.tools.truncate_tool_output_lines.to_string(),
        },
        SettingDefinition {
            key: "tools.enable_message_bus_integration".to_string(),
            label: "Message Bus".to_string(),
            description: "Enable message bus integration".to_string(),
            category: "Tools".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.tools.enable_message_bus_integration.to_string(),
        },
    ]);

    // Security settings
    settings.extend(vec![
        SettingDefinition {
            key: "security.disable_yolo_mode".to_string(),
            label: "Disable YOLO Mode".to_string(),
            description: "Disable YOLO mode even if flagged".to_string(),
            category: "Security".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.security.disable_yolo_mode.to_string(),
        },
        SettingDefinition {
            key: "security.enable_permanent_tool_approval".to_string(),
            label: "Permanent Approval".to_string(),
            description: "Allow permanent tool approvals".to_string(),
            category: "Security".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.security.enable_permanent_tool_approval.to_string(),
        },
        SettingDefinition {
            key: "security.block_git_extensions".to_string(),
            label: "Block Git Extensions".to_string(),
            description: "Block Git-based extensions".to_string(),
            category: "Security".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.security.block_git_extensions.to_string(),
        },
        SettingDefinition {
            key: "security.folder_trust.enabled".to_string(),
            label: "Folder Trust".to_string(),
            description: "Enable folder trust system".to_string(),
            category: "Security".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.security.folder_trust.enabled.to_string(),
        },
        SettingDefinition {
            key: "security.environment_variable_redaction.enabled".to_string(),
            label: "Env Var Redaction".to_string(),
            description: "Enable env var redaction".to_string(),
            category: "Security".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config
                .security
                .environment_variable_redaction
                .enabled
                .to_string(),
        },
    ]);

    // Experimental settings
    settings.extend(vec![
        SettingDefinition {
            key: "experimental.enable_agents".to_string(),
            label: "Enable Agents".to_string(),
            description: "Enable experimental agent features".to_string(),
            category: "Experimental".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.experimental.enable_agents.to_string(),
        },
        SettingDefinition {
            key: "experimental.extension_management".to_string(),
            label: "Extension Management".to_string(),
            description: "Enable extension management".to_string(),
            category: "Experimental".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.experimental.extension_management.to_string(),
        },
        SettingDefinition {
            key: "experimental.jit_context".to_string(),
            label: "JIT Context".to_string(),
            description: "Enable just-in-time context loading".to_string(),
            category: "Experimental".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.experimental.jit_context.to_string(),
        },
        SettingDefinition {
            key: "experimental.codebase_investigator_settings.enabled".to_string(),
            label: "Codebase Investigator".to_string(),
            description: "Enable codebase investigator".to_string(),
            category: "Experimental".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config
                .experimental
                .codebase_investigator_settings
                .enabled
                .to_string(),
        },
        SettingDefinition {
            key: "experimental.codebase_investigator_settings.max_num_turns".to_string(),
            label: "Investigator Max Turns".to_string(),
            description: "Max investigator turns".to_string(),
            category: "Experimental".to_string(),
            setting_type: SettingType::Number,
            default_value: "10".to_string(),
            requires_restart: false,
            current_value: config
                .experimental
                .codebase_investigator_settings
                .max_num_turns
                .to_string(),
        },
    ]);

    // ACP settings
    settings.extend(vec![
        SettingDefinition {
            key: "acp.enabled".to_string(),
            label: "ACP Enabled".to_string(),
            description: "Enable Agent Client Protocol".to_string(),
            category: "ACP".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: true,
            current_value: config.acp.enabled.to_string(),
        },
        SettingDefinition {
            key: "acp.bind_host".to_string(),
            label: "ACP Bind Host".to_string(),
            description: "ACP server bind address".to_string(),
            category: "ACP".to_string(),
            setting_type: SettingType::String,
            default_value: "127.0.0.1".to_string(),
            requires_restart: true,
            current_value: config.acp.bind_host.clone(),
        },
        SettingDefinition {
            key: "acp.default_port".to_string(),
            label: "ACP Default Port".to_string(),
            description: "Default ACP server port".to_string(),
            category: "ACP".to_string(),
            setting_type: SettingType::Number,
            default_value: "".to_string(), // None
            requires_restart: true,
            current_value: config
                .acp
                .default_port
                .map(|p| p.to_string())
                .unwrap_or_default(),
        },
        SettingDefinition {
            key: "acp.protocol_version".to_string(),
            label: "ACP Protocol Version".to_string(),
            description: "ACP protocol version".to_string(),
            category: "ACP".to_string(),
            setting_type: SettingType::String,
            default_value: "1.0".to_string(),
            requires_restart: true,
            current_value: config.acp.protocol_version.clone(),
        },
        SettingDefinition {
            key: "acp.dev_mode".to_string(),
            label: "ACP Dev Mode".to_string(),
            description: "Enable development mode".to_string(),
            category: "ACP".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: true,
            current_value: config.acp.dev_mode.to_string(),
        },
    ]);

    // Network settings
    settings.extend(vec![
        SettingDefinition {
            key: "network.starlink_optimizations".to_string(),
            label: "Starlink Optimizations".to_string(),
            description: "Enable Starlink satellite optimizations".to_string(),
            category: "Network".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: false,
            current_value: config.network.starlink_optimizations.to_string(),
        },
        SettingDefinition {
            key: "network.base_retry_delay".to_string(),
            label: "Base Retry Delay".to_string(),
            description: "Base retry delay in seconds".to_string(),
            category: "Network".to_string(),
            setting_type: SettingType::Number,
            default_value: "1".to_string(),
            requires_restart: false,
            current_value: config.network.base_retry_delay.to_string(),
        },
        SettingDefinition {
            key: "network.max_retry_delay".to_string(),
            label: "Max Retry Delay".to_string(),
            description: "Maximum retry delay in seconds".to_string(),
            category: "Network".to_string(),
            setting_type: SettingType::Number,
            default_value: "30".to_string(),
            requires_restart: false,
            current_value: config.network.max_retry_delay.to_string(),
        },
        SettingDefinition {
            key: "network.health_monitoring".to_string(),
            label: "Health Monitoring".to_string(),
            description: "Enable network health monitoring".to_string(),
            category: "Network".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "true".to_string(),
            requires_restart: false,
            current_value: config.network.health_monitoring.to_string(),
        },
        SettingDefinition {
            key: "network.connect_timeout".to_string(),
            label: "Connect Timeout".to_string(),
            description: "Connection timeout in seconds".to_string(),
            category: "Network".to_string(),
            setting_type: SettingType::Number,
            default_value: "10".to_string(),
            requires_restart: false,
            current_value: config.network.connect_timeout.to_string(),
        },
        SettingDefinition {
            key: "network.read_timeout".to_string(),
            label: "Read Timeout".to_string(),
            description: "Read timeout in seconds".to_string(),
            category: "Network".to_string(),
            setting_type: SettingType::Number,
            default_value: "30".to_string(),
            requires_restart: false,
            current_value: config.network.read_timeout.to_string(),
        },
    ]);

    // Logging settings
    settings.extend(vec![
        SettingDefinition {
            key: "logging.level".to_string(),
            label: "Log Level".to_string(),
            description: "Log level (trace/debug/info/warn/error)".to_string(),
            category: "Logging".to_string(),
            setting_type: SettingType::String,
            default_value: "info".to_string(),
            requires_restart: true,
            current_value: config.logging.level.clone(),
        },
        SettingDefinition {
            key: "logging.file_logging".to_string(),
            label: "File Logging".to_string(),
            description: "Enable logging to file".to_string(),
            category: "Logging".to_string(),
            setting_type: SettingType::Boolean,
            default_value: "false".to_string(),
            requires_restart: true,
            current_value: config.logging.file_logging.to_string(),
        },
        SettingDefinition {
            key: "logging.max_file_size_mb".to_string(),
            label: "Max Log Size (MB)".to_string(),
            description: "Maximum log file size in MB".to_string(),
            category: "Logging".to_string(),
            setting_type: SettingType::Number,
            default_value: "10".to_string(),
            requires_restart: true,
            current_value: config.logging.max_file_size_mb.to_string(),
        },
        SettingDefinition {
            key: "logging.rotation_count".to_string(),
            label: "Log Rotation Count".to_string(),
            description: "Number of rotated log files to keep".to_string(),
            category: "Logging".to_string(),
            setting_type: SettingType::Number,
            default_value: "5".to_string(),
            requires_restart: true,
            current_value: config.logging.rotation_count.to_string(),
        },
    ]);

    settings
}

/// Get unique categories from settings
fn get_categories(settings: &[SettingDefinition]) -> Vec<String> {
    let mut categories: Vec<String> = settings
        .iter()
        .map(|s| s.category.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    categories.sort();
    categories
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_setting_value() {
        assert_eq!(
            format_setting_value("true", &SettingType::Boolean).to_string(),
            "✓ Enabled".green().to_string()
        );
        assert_eq!(
            format_setting_value("false", &SettingType::Boolean).to_string(),
            "✗ Disabled".red().to_string()
        );
    }

    #[test]
    fn test_get_categories() {
        let settings = vec![
            SettingDefinition {
                key: "test1".to_string(),
                label: "Test 1".to_string(),
                description: "Test".to_string(),
                category: "UI".to_string(),
                setting_type: SettingType::Boolean,
                default_value: "false".to_string(),
                requires_restart: false,
                current_value: "true".to_string(),
            },
            SettingDefinition {
                key: "test2".to_string(),
                label: "Test 2".to_string(),
                description: "Test".to_string(),
                category: "General".to_string(),
                setting_type: SettingType::String,
                default_value: "default".to_string(),
                requires_restart: false,
                current_value: "custom".to_string(),
            },
        ];

        let categories = get_categories(&settings);
        assert_eq!(categories, vec!["General", "UI"]);
    }
}
