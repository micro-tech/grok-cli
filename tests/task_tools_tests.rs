//! Top-level test harness for task-tools integration tests.
//!
//! The actual test cases live in `tests/integration/task_tools_tests.rs`
//! and are pulled in via `#[path]`.  This file exists solely because Cargo
//! only discovers integration-test binaries from direct children of `tests/`.
//!
//! Run with:
//!   cargo test --test task_tools_tests -- --nocapture

// Pull in the shared helpers module so that `integration/task_tools_tests.rs`
// can reference it as `helpers::*` via its own `#[path]` include.
#[path = "integration/task_tools_tests.rs"]
mod task_tools;
