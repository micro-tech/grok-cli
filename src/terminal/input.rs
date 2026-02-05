//! Terminal input functions for grok-cli binary
//!
//! This module contains all functions that handle user input and interaction.
//! These are binary-only functions and should NOT be in the library.

use anyhow::Result;
use colored::*;
use std::io::{self, Write};

/// Prompt user for confirmation
pub fn confirm(message: &str) -> Result<bool> {
    print!("{} {} [y/N]: ", "?".cyan(), message);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}

/// Prompt user for text input
pub fn prompt(message: &str) -> Result<String> {
    print!("{} {}: ", "?".cyan(), message);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

/// Prompt user for text input with a default value
pub fn prompt_with_default(message: &str, default: &str) -> Result<String> {
    print!("{} {} [{}]: ", "?".cyan(), message, default.dimmed());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}
