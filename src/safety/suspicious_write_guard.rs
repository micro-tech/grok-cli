//! Suspicious Write Guard
//!
//! Final line of defense inside the tool itself.

use crate::safety::error::SafetyError;

pub struct SuspiciousWriteGuard;

impl SuspiciousWriteGuard {
    /// Returns Ok(()) if the write looks safe, Err(SafetyError) otherwise.
    pub fn check(
        original_len: usize,
        new_len: usize,
        content: &str,
        file_ext: Option<&str>,
    ) -> Result<(), SafetyError> {
        // Empty file check
        if new_len == 0 && original_len > 0 {
            return Err(SafetyError::EmptyFileOverwrite);
        }

        // 10x size increase
        if original_len > 0 && new_len > original_len * 10 {
            return Err(SafetyError::FileSizeExplosion);
        }

        // Binary junk (simple heuristic)
        let non_printable = content.bytes().filter(|b| *b < 32 && *b != 9 && *b != 10 && *b != 13).count();
        if non_printable > content.len() / 8 {
            return Err(SafetyError::BinaryJunk);
        }

        // Parse checks for known formats
        if let Some(ext) = file_ext {
            match ext {
                "json" => {
                    if serde_json::from_str::<serde_json::Value>(content).is_err() {
                        return Err(SafetyError::InvalidSyntax { format: "JSON".into() });
                    }
                }
                "toml" | "yaml" | "yml" => {
                    if content.trim().is_empty() {
                        return Err(SafetyError::InvalidSyntax { format: ext.to_string() });
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
