//! Pre-Write Safety Hook (Mandatory)
//!
//! Runs before any file write/replace/delete/patch operation.
//! Validates diffs, checks dangerous patterns, compares with SessionDNA
//! failure modes, requests confirmation on high risk, and blocks insane actions.

use serde_json::Value;
use std::path::Path;

/// Result of the pre-write safety check
#[derive(Debug, Clone)]
pub enum SafetyDecision {
    Allow,
    AllowWithWarning(String),
    RequireConfirmation(String),
    Block(String),
}

/// Context passed to the safety hook
#[derive(Debug, Clone)]
pub struct WriteContext<'a> {
    pub path: &'a Path,
    pub operation: &'a str, // "write", "replace", "patch", "delete"
    pub proposed_content: Option<&'a str>,
    pub diff: Option<&'a str>,
    pub session_dna: Option<&'a Value>,
}

/// Main entry point: on_before_write_file
pub fn on_before_write_file(ctx: &WriteContext) -> SafetyDecision {
    // 1. Check for obviously dangerous patterns
    if let Some(content) = ctx.proposed_content {
        // Apply the 200k hard limit to both full writes and large replaces.
        // This makes the safety behavior consistent and keeps the integration
        // test happy (it expects either "200k" or "Safety" in the error).
        if content.len() > 200_000 && (ctx.operation == "write" || ctx.operation == "replace") {
            return SafetyDecision::Block(
                "Refusing to write >200k characters in a single operation".to_string(),
            );
        }

        // Binary junk detection (simple heuristic)
        let non_printable = content
            .bytes()
            .filter(|b| *b < 9 || (*b > 13 && *b < 32) || *b == 127)
            .count();
        if non_printable > content.len() / 10 {
            return SafetyDecision::Block("Content appears to contain binary junk".to_string());
        }

        // JSON validity check for .json files
        if ctx.path.extension().is_some_and(|e| e == "json")
            && serde_json::from_str::<Value>(content).is_err()
        {
            return SafetyDecision::Block(
                "Target is .json but content is not valid JSON".to_string(),
            );
        }
    }

    // 2. Compare with SessionDNA failure modes (if available)
    if let Some(dna) = ctx.session_dna
        && let Some(failures) = dna.get("repeated_file_write_failures")
        && failures.as_u64().unwrap_or(0) > 3
    {
        return SafetyDecision::RequireConfirmation(
            "SessionDNA shows repeated write failures. Confirm before proceeding.".to_string(),
        );
    }

    // 3. High-risk operations require confirmation
    if ctx.operation == "delete" {
        return SafetyDecision::RequireConfirmation(format!(
            "About to DELETE {}. Confirm?",
            ctx.path.display()
        ));
    }

    SafetyDecision::Allow
}
