//! Explorer mode runner.

use anyhow::Result;
use std::path::PathBuf;

use crate::agent::explorer::evidence::{RepoEvidence, RepoEvidenceItem};
use crate::agent::prompts::explorer_prompt::explorer_system_prompt;
use crate::router::AppRouter;

/// Run the explorer agent and return structured `RepoEvidence`.
pub async fn run_explorer_mode(
    client: &AppRouter,
    query: &str,
    model: &str,
) -> Result<RepoEvidence> {
    let system = explorer_system_prompt();

    let messages = vec![
        serde_json::json!({ "role": "system", "content": system }),
        serde_json::json!({ "role": "user", "content": query }),
    ];

    // Only allow read/search tools
    let allowed_tools = ["fs_glob", "fs_read", "fs_grep", "list_directory", "search_file_content"];

    let all_tools = crate::acp::tools::get_available_tool_definitions();
    let filtered: Vec<serde_json::Value> = all_tools
        .into_iter()
        .filter(|t| {
            t.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(|name| allowed_tools.contains(&name))
                .unwrap_or(false)
        })
        .collect();

    let resp = client
        .chat_completion_with_history(&messages, 0.1, 4096, model, Some(filtered), None)
        .await?;

    if let Some(content) = resp.message.content {
        let text = crate::extract_text_content(&content);
        if let Ok(evidence) = serde_json::from_str::<RepoEvidence>(&text) {
            return Ok(evidence);
        }
        return Ok(RepoEvidence {
            items: vec![RepoEvidenceItem {
                path: PathBuf::from("raw"),
                line_start: 0,
                line_end: 0,
                summary: text,
            }],
        });
    }

    Ok(RepoEvidence { items: vec![] })
}
