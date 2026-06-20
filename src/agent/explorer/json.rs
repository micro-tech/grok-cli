//! JSON parsing helpers for Explorer mode output.

use anyhow::{anyhow, Result};
use crate::agent::explorer::evidence::{RepoEvidence, RepoEvidenceItem};

/// Parse raw LLM text into `RepoEvidence`.
/// Accepts either a clean JSON object or a fenced ```json block.
pub fn parse_explorer_json(text: &str) -> Result<RepoEvidence> {
    // Strip common markdown fences
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    match serde_json::from_str::<RepoEvidence>(cleaned) {
        Ok(ev) => Ok(ev),
        Err(e) => {
            // Try to extract a JSON object heuristically
            if let Some(start) = cleaned.find('{') {
                if let Some(end) = cleaned.rfind('}') {
                    let candidate = &cleaned[start..=end];
                    if let Ok(ev) = serde_json::from_str::<RepoEvidence>(candidate) {
                        return Ok(ev);
                    }
                }
            }
            Err(anyhow!("Failed to parse explorer JSON: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean_json() {
        let input = r#"{"items":[{"path":"src/main.rs","line_start":1,"line_end":10,"summary":"entry point"}]}"#;
        let ev = parse_explorer_json(input).unwrap();
        assert_eq!(ev.items.len(), 1);
        assert_eq!(ev.items[0].path.to_string_lossy(), "src/main.rs");
    }

    #[test]
    fn parses_fenced_json() {
        let input = "```json\n{\"items\":[]}\n```";
        let ev = parse_explorer_json(input).unwrap();
        assert!(ev.items.is_empty());
    }

    #[test]
    fn handles_malformed() {
        let input = "not json at all";
        assert!(parse_explorer_json(input).is_err());
    }
}
