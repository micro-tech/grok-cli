//! Diff-Based Editing Validator
//!
//! Enforces that agents only use unified diffs, patch hunks, or line-range edits.
//! Rejects full file replacements >200 lines or >40% file removal.

use crate::safety::error::SafetyError;

pub struct DiffValidator;

impl DiffValidator {
    /// Returns Ok(()) if the edit is acceptable, Err(SafetyError) otherwise.
    pub fn validate_edit(
        original_lines: usize,
        new_lines: usize,
        is_full_replacement: bool,
    ) -> Result<(), SafetyError> {
        if is_full_replacement && new_lines > 200 {
            return Err(SafetyError::FullReplacementTooLarge);
        }

        let removal_ratio = if original_lines > 0 {
            (original_lines.saturating_sub(new_lines)) as f32 / original_lines as f32
        } else {
            0.0
        };

        if removal_ratio > 0.40 {
            return Err(SafetyError::ExcessiveRemoval(removal_ratio * 100.0));
        }

        Ok(())
    }
}
