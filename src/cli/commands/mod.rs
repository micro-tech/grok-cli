//! Command handlers for grok-cli
//!
//! This module contains all the command handler implementations for the various
//! CLI commands supported by grok-cli.
//!
//! Note: Individual command modules use deprecated I/O functions that will be
//! refactored in Phase 2. They have #![allow(deprecated)] to suppress warnings.

pub mod acp;
pub mod audit;
pub mod chat;
pub mod code;
pub mod config;
pub mod health;
pub mod history;
pub mod settings;
pub mod skills;

// Re-export all command handlers
