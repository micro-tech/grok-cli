//! Terminal I/O module for grok-cli binary
//!
//! This module contains all terminal I/O operations including printing,
//! progress bars, user interaction, and terminal manipulation.
//! These functions are for the binary crate only and should NOT be in the library.

pub mod display;
pub mod input;
pub mod progress;

pub use display::{
    clear_screen, print_centered, print_error, print_info, print_separator, print_success,
    print_warning,
};
pub use input::confirm;
pub use progress::create_spinner;

use terminal_size::{Height, Width, terminal_size};

/// Get terminal dimensions
pub fn get_terminal_size() -> (u16, u16) {
    if let Some((Width(w), Height(h))) = terminal_size() {
        (w, h)
    } else {
        (80, 24) // Default fallback
    }
}

/// Get terminal width, defaulting to 80 if unable to determine
pub fn get_terminal_width() -> usize {
    terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80)
}
