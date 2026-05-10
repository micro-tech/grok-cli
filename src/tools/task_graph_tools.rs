use anyhow::{anyhow, Result};
use crate::task_graph::TaskGraph;
use crate::tools::ToolContext;

pub async fn execute_task_graph(graph_json: &str, ctx: &ToolContext) -> Result<String> {
    let graph: TaskGraph = serde_json::from_str(graph_json)
        .map_err(|e| anyhow!("Invalid task graph JSON: {}", e))?;
    graph.execute(ctx).await
        .map_err(|e| anyhow!("Task graph execution failed: {}", e))?;
    Ok("Task graph executed successfully".to_string())
}