//! Suspicious Write Guard
//!
//! Final line of defense inside the tool itself.

pub struct SuspiciousWriteGuard;

impl SuspiciousWriteGuard {
    pub fn check(
        original_len: usize,
        new_len: usize,
        content: &str,
        file_ext: Option<&str>,
    ) -> Result<(), String> {
        // Empty file check
        if new_len == 0 && original_len > 0 {
            return Err("Refusing to write empty file over existing content".to_string());
        }

        // 10x size increase
        if original_len > 0 && new_len > original_len * 10 {
            return Err("Refusing write that would make file >10x larger".to_string());
        }

        // Binary junk (simple heuristic)
        let non_printable = content.bytes().filter(|b| *b < 32 && *b != 9 && *b != 10 && *b != 13).count();
        if non_printable > content.len() / 8 {
            return Err("Content contains binary junk".to_string());
        }

        // Parse checks for known formats
        if let Some(ext) = file_ext {
            match ext {
                "json" => {
                    if serde_json::from_str::<serde_json::Value>(content).is_err() {
                        return Err("Invalid JSON syntax".to_string());
                    }
                }
                "toml" | "yaml" | "yml" => {
                    // lightweight check only
                    if content.trim().is_empty() {
                        return Err("Empty config file".to_string());
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
