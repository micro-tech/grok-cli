//! Explorer module – full Mode::Explorer implementation + planner helper.

pub mod evidence;
pub mod json;
pub mod runner;
pub mod tools;

pub use runner::run_explorer_mode;
