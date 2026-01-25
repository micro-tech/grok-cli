//! Shell command permission and security system
//!
//! This module provides a permission system for shell commands executed in interactive mode,
//! similar to Gemini CLI's approach. It includes:
//! - Dangerous command detection (blocklist)
//! - User prompts for confirmation
//! - Session-level allowlist ("Always allow")
//! - Persistent policy storage

use anyhow::{anyhow, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Approval mode for shell commands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum ApprovalMode {
    /// Ask for confirmation (default)
    #[default]
    Default,
    /// Always allow without asking (DANGEROUS!)
    Yolo,
}


/// Permission decision for a command
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Allow execution
    Allow,
    /// Deny execution
    Deny,
    /// Allow and add to session allowlist
    AllowAlways,
    /// Blocked by policy (dangerous command)
    Blocked(String),
}

/// Shell command permissions manager
#[derive(Debug, Clone)]
pub struct ShellPermissions {
    /// Approval mode
    approval_mode: ApprovalMode,
    /// Session-level allowlist (commands allowed for this session)
    session_allowlist: HashSet<String>,
    /// Persistent allowlist (saved to disk)
    persistent_allowlist: HashSet<String>,
    /// Blocklist of dangerous commands
    blocklist: HashSet<String>,
    /// Path to persistent policy file
    policy_path: Option<PathBuf>,
}

impl Default for ShellPermissions {
    fn default() -> Self {
        Self::new(ApprovalMode::Default)
    }
}

impl ShellPermissions {
    /// Create a new permissions manager
    pub fn new(approval_mode: ApprovalMode) -> Self {
        let mut permissions = Self {
            approval_mode,
            session_allowlist: HashSet::new(),
            persistent_allowlist: HashSet::new(),
            blocklist: Self::default_blocklist(),
            policy_path: Self::get_policy_path().ok(),
        };

        // Load persistent allowlist from disk
        if let Some(path) = &permissions.policy_path
            && let Ok(policy) = Self::load_policy(path) {
                permissions.persistent_allowlist = policy.allowed_commands;
            }

        permissions
    }

    /// Get the default blocklist of dangerous commands
    fn default_blocklist() -> HashSet<String> {
        let mut blocklist = HashSet::new();

        // Destructive file operations
        blocklist.insert("rm".to_string());
        blocklist.insert("del".to_string());
        blocklist.insert("rmdir".to_string());
        blocklist.insert("rd".to_string());
        blocklist.insert("format".to_string());
        blocklist.insert("fdisk".to_string());
        blocklist.insert("mkfs".to_string());

        // System operations
        blocklist.insert("shutdown".to_string());
        blocklist.insert("reboot".to_string());
        blocklist.insert("halt".to_string());
        blocklist.insert("poweroff".to_string());
        blocklist.insert("init".to_string());

        // Dangerous shell operations
        blocklist.insert(":(){ :|:& };:".to_string()); // Fork bomb
        blocklist.insert("dd".to_string()); // Can overwrite disks

        // Package management (can modify system)
        blocklist.insert("apt-get".to_string());
        blocklist.insert("yum".to_string());
        blocklist.insert("dnf".to_string());
        blocklist.insert("pacman".to_string());

        blocklist
    }

    /// Get the path to the persistent policy file
    fn get_policy_path() -> Result<PathBuf> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
        Ok(home_dir.join(".grok").join("shell_policy.json"))
    }

    /// Load policy from disk
    fn load_policy(path: &PathBuf) -> Result<ShellPolicy> {
        let contents = fs::read_to_string(path)?;
        let policy: ShellPolicy = serde_json::from_str(&contents)?;
        Ok(policy)
    }

    /// Save policy to disk
    fn save_policy(&self) -> Result<()> {
        if let Some(path) = &self.policy_path {
            // Create parent directory if it doesn't exist
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            let policy = ShellPolicy {
                allowed_commands: self.persistent_allowlist.clone(),
            };

            let contents = serde_json::to_string_pretty(&policy)?;
            fs::write(path, contents)?;
        }
        Ok(())
    }

    /// Extract the root command from a command string
    fn extract_root_command(command: &str) -> String {
        let trimmed = command.trim();

        // Handle shell operators
        let command_part = trimmed
            .split(&['|', '&', ';', '>', '<'][..])
            .next()
            .unwrap_or(trimmed)
            .trim();

        // Extract just the command name (first word)
        command_part
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    }

    /// Check if a command is blocked by policy
    pub fn is_blocked(&self, command: &str) -> Option<String> {
        let root_command = Self::extract_root_command(command);

        if self.blocklist.contains(&root_command) {
            return Some(format!(
                "Command '{}' is blocked for security reasons",
                root_command
            ));
        }

        // Check for dangerous patterns
        if command.contains("rm -rf /") || command.contains("del /s /q") {
            return Some("Dangerous recursive delete pattern detected".to_string());
        }

        if command.contains(":(){ :|:& };:") {
            return Some("Fork bomb pattern detected".to_string());
        }

        None
    }

    /// Check if a command is in the allowlist (session or persistent)
    pub fn is_allowed(&self, command: &str) -> bool {
        let root_command = Self::extract_root_command(command);

        self.session_allowlist.contains(&root_command)
            || self.persistent_allowlist.contains(&root_command)
    }

    /// Add a command to the session allowlist
    pub fn add_to_session_allowlist(&mut self, command: &str) {
        let root_command = Self::extract_root_command(command);
        self.session_allowlist.insert(root_command);
    }

    /// Add a command to the persistent allowlist and save to disk
    pub fn add_to_persistent_allowlist(&mut self, command: &str) -> Result<()> {
        let root_command = Self::extract_root_command(command);
        self.persistent_allowlist.insert(root_command);
        self.save_policy()
    }

    /// Prompt the user for permission to execute a command
    pub fn prompt_for_permission(&mut self, command: &str) -> Result<PermissionDecision> {
        // In YOLO mode, always allow
        if self.approval_mode == ApprovalMode::Yolo {
            return Ok(PermissionDecision::Allow);
        }

        // Check if blocked
        if let Some(reason) = self.is_blocked(command) {
            return Ok(PermissionDecision::Blocked(reason));
        }

        // Check if already allowed
        if self.is_allowed(command) {
            return Ok(PermissionDecision::Allow);
        }

        // Prompt user
        println!();
        println!(
            "{} {}",
            "⚠️  Shell command requires permission:".yellow().bold(),
            command.bright_yellow()
        );
        println!();
        println!("  {} Allow once", "a)".bright_cyan());
        println!("  {} Allow always (this session)", "s)".bright_cyan());
        println!("  {} Allow always (save permanently)", "p)".bright_cyan());
        println!("  {} Deny", "d)".bright_cyan());
        println!();
        print!("{} ", "Choose [a/s/p/d]:".bright_white().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        match choice.as_str() {
            "a" | "allow" => Ok(PermissionDecision::Allow),
            "s" | "session" => {
                self.add_to_session_allowlist(command);
                println!("{}", "✓ Added to session allowlist".bright_green());
                Ok(PermissionDecision::AllowAlways)
            }
            "p" | "permanent" => {
                self.add_to_persistent_allowlist(command)?;
                println!("{}", "✓ Added to permanent allowlist".bright_green());
                Ok(PermissionDecision::AllowAlways)
            }
            "d" | "deny" | "n" | "no" => {
                println!("{}", "✗ Command denied".bright_red());
                Ok(PermissionDecision::Deny)
            }
            "" => {
                // Default to deny on empty input
                println!("{}", "✗ Command denied (no input)".bright_red());
                Ok(PermissionDecision::Deny)
            }
            _ => {
                println!("{}", "Invalid choice, defaulting to deny".bright_red());
                Ok(PermissionDecision::Deny)
            }
        }
    }

    /// Check if a command should be executed (main entry point)
    pub fn should_execute(&mut self, command: &str) -> Result<bool> {
        let decision = self.prompt_for_permission(command)?;

        match decision {
            PermissionDecision::Allow | PermissionDecision::AllowAlways => Ok(true),
            PermissionDecision::Deny => Ok(false),
            PermissionDecision::Blocked(reason) => {
                eprintln!("{} {}", "✗ Blocked:".bright_red().bold(), reason.red());
                Ok(false)
            }
        }
    }

    /// Get approval mode
    pub fn approval_mode(&self) -> ApprovalMode {
        self.approval_mode
    }

    /// Set approval mode
    pub fn set_approval_mode(&mut self, mode: ApprovalMode) {
        self.approval_mode = mode;
    }

    /// Clear session allowlist
    pub fn clear_session_allowlist(&mut self) {
        self.session_allowlist.clear();
    }

    /// Get session allowlist
    pub fn get_session_allowlist(&self) -> &HashSet<String> {
        &self.session_allowlist
    }

    /// Get persistent allowlist
    pub fn get_persistent_allowlist(&self) -> &HashSet<String> {
        &self.persistent_allowlist
    }

    /// Remove a command from persistent allowlist
    pub fn remove_from_persistent_allowlist(&mut self, command: &str) -> Result<()> {
        self.persistent_allowlist.remove(command);
        self.save_policy()
    }

    /// Reset persistent allowlist (clear all saved permissions)
    pub fn reset_persistent_allowlist(&mut self) -> Result<()> {
        self.persistent_allowlist.clear();
        self.save_policy()
    }
}

/// Persistent policy stored on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShellPolicy {
    allowed_commands: HashSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_root_command() {
        assert_eq!(
            ShellPermissions::extract_root_command("ls -la"),
            "ls".to_string()
        );
        assert_eq!(
            ShellPermissions::extract_root_command("git status | grep modified"),
            "git".to_string()
        );
        assert_eq!(
            ShellPermissions::extract_root_command("echo hello && echo world"),
            "echo".to_string()
        );
        assert_eq!(
            ShellPermissions::extract_root_command("  pwd  "),
            "pwd".to_string()
        );
    }

    #[test]
    fn test_blocked_commands() {
        let perms = ShellPermissions::new(ApprovalMode::Default);

        assert!(perms.is_blocked("rm -rf /").is_some());
        assert!(perms.is_blocked("shutdown now").is_some());
        assert!(perms.is_blocked("format c:").is_some());
        assert!(perms.is_blocked("ls -la").is_none());
        assert!(perms.is_blocked("git status").is_none());
    }

    #[test]
    fn test_allowlist() {
        let mut perms = ShellPermissions::new(ApprovalMode::Default);

        assert!(!perms.is_allowed("git status"));

        perms.add_to_session_allowlist("git status");
        assert!(perms.is_allowed("git status"));
        assert!(perms.is_allowed("git commit")); // Same root command
    }

    #[test]
    fn test_yolo_mode() {
        let mut perms = ShellPermissions::new(ApprovalMode::Yolo);

        // In YOLO mode, non-blocked commands are always allowed
        let decision = perms.prompt_for_permission("ls -la").unwrap();
        assert_eq!(decision, PermissionDecision::Allow);
    }
}
