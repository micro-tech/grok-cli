//! Planner integration for Mode::Explorer (Task 162)
//!
//! Provides a helper that the reasoning/planner layer can call
//! to run an explorer-mode query and obtain compact RepoEvidence.

use anyhow::Result;
use serde_json::Value;

use crate::agent::Mode;
use crate::router::AppRouter;

/// Run the explorer agent with a focused query and return parsed JSON evidence.
///
/// This is intended to be called by the planner before starting a complex
/// coding task so the main agent receives compact evidence instead of raw files.
pub async fn run_explorer(
    client: &AppRouter,
    query: &str,
    model: &str,
) -> Result<Value> {
    let system = Mode::Explorer.system_prompt_additions();

    let messages = vec![
        serde_json::json!({ "role": "system", "content": system }),
        serde_json::json!({ "role": "user", "content": query }),
    ];

    // Only allow read/search tools
    let allowed = ["fs_glob", "fs_read", "fs_grep", "list_directory", "search_file_content"];
    // In a real implementation we would filter the full tool list here.
    // For now we pass None and rely on the system prompt + model behaviour.

    let resp = client
        .chat_completion_with_history(&messages, 0.1, 4096, model, None, None)
        .await?;

    if let Some(content) = resp.message.content {
        let text = crate::extract_text_content(&content);
        if let Ok(json) = serde_json::from_str::<Value>(&text) {
            return Ok(json);
        }
        // Fallback: wrap raw text
        return Ok(serde_json::json!({ "raw": text }));
    }

    Ok(serde_json::json!({ "error": "no content" }))
}
