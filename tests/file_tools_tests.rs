//! Top-level test harness for file-tools integration tests (subtasks 148.4 + 148.5).
//!
//! The actual test cases live in `tests/integration/file_tools_tests.rs` and
//! are pulled in via `#[path]`.  This wrapper exists solely because Cargo only
//! auto-discovers integration-test binaries from direct children of `tests/`.
//!
//! Run with:
//!   cargo test --test file_tools_tests -- --nocapture

#[path = "integration/file_tools_tests.rs"]
mod file_tools;
