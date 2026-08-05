use crate::task_graph::TaskGraph;
use crate::tools::ToolContext;
use anyhow::{Result, anyhow};

pub async fn execute_task_graph(graph_json: &str, ctx: &ToolContext) -> Result<String> {
    let graph: TaskGraph =
        serde_json::from_str(graph_json).map_err(|e| anyhow!("Invalid task graph JSON: {}", e))?;
    let results = graph
        .execute(ctx)
        .await
        .map_err(|e| anyhow!("Task graph execution failed: {}", e))?;

    // Return structured results so callers can see per-node output
    let json = serde_json::to_string_pretty(&results)
        .unwrap_or_else(|_| "{}".to_string());
    Ok(format!("Task graph executed successfully.\nResults:\n{}", json))
}
