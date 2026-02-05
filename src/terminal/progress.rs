//! Terminal progress indicators for grok-cli binary
//!
//! This module contains all functions that create and manage progress bars and spinners.
//! These are binary-only functions and should NOT be in the library.

use indicatif::{ProgressBar, ProgressStyle};

/// Create a progress spinner with the given message
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Create a progress bar with the given length and message
pub fn create_progress_bar(len: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
            .unwrap()
            .progress_chars("█▓▒░"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Create a simple spinner without styling
pub fn create_simple_spinner() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Finish a progress indicator with a success message
pub fn finish_progress_success(pb: &ProgressBar, message: &str) {
    pb.finish_with_message(format!("✓ {}", message));
}

/// Finish a progress indicator with an error message
pub fn finish_progress_error(pb: &ProgressBar, message: &str) {
    pb.finish_with_message(format!("✗ {}", message));
}

/// Finish a progress indicator and clear it
pub fn finish_progress_clear(pb: &ProgressBar) {
    pb.finish_and_clear();
}
