//! HOH TaskList Exporter (Task 327.31)

use crate::hoh::tasklist_adapter::Task;

/// Export task list to Markdown checklist format.
pub fn export_markdown(tasks: &[Task]) -> String {
    let mut out = String::from("# Task List\n\n");
    for t in tasks {
        let check = if t.status == "done" { "x" } else { " " };
        out.push_str(&format!("- [{}] **[{}]** {} ({})\n", check, t.id, t.title, t.priority));
        if !t.description.is_empty() {
            out.push_str(&format!("  > {}\n", t.description.lines().next().unwrap_or("")));
        }
        for st in &t.subtasks {
            let sc = if st.status == "done" { "x" } else { " " };
            out.push_str(&format!("  - [{}] {}\n", sc, st.title));
        }
    }
    out
}

/// Export to CSV.
pub fn export_csv(tasks: &[Task]) -> String {
    let mut out = String::from("id,title,status,priority,dependencies,subtask_count\n");
    for t in tasks {
        let deps = t.dependencies.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(";");
        let title_esc = t.title.replace('"', "\"\"");
        out.push_str(&format!(
            "{},\"{}\",{},{},\"{}\",{}\n",
            t.id, title_esc, t.status, t.priority, deps, t.subtasks.len()
        ));
    }
    out
}

/// Export to a simple JSON summary (lighter than full task_list.json).
pub fn export_json_summary(tasks: &[Task]) -> String {
    let entries: Vec<serde_json::Value> = tasks.iter().map(|t| serde_json::json!({
        "id": t.id,
        "title": t.title,
        "status": t.status,
        "priority": t.priority,
        "dependencies": t.dependencies,
        "subtasks_done": t.subtasks.iter().filter(|s| s.status == "done").count(),
        "subtasks_total": t.subtasks.len(),
    })).collect();
    serde_json::to_string_pretty(&entries).unwrap_or_default()
}

/// Save export to file.
pub async fn save_export(tasks: &[Task], project_root: &std::path::Path, format: &str) -> std::io::Result<()> {
    let content = match format {
        "csv"  => export_csv(tasks),
        "json" => export_json_summary(tasks),
        _      => export_markdown(tasks),
    };
    let ext = match format { "csv" => "csv", "json" => "json", _ => "md" };
    let out_dir = project_root.join(".grok/hoh/exports");
    tokio::fs::create_dir_all(&out_dir).await?;
    tokio::fs::write(out_dir.join(format!("task_list.{}", ext)), content).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u64, title: &str, status: &str) -> Task {
        Task { id, title: title.to_string(), status: status.to_string(), priority: "medium".to_string(), ..Default::default() }
    }

    #[test]
    fn test_markdown_export_contains_tasks() {
        let tasks = vec![t(1,"Implement cache","done"), t(2,"Add tests","pending")];
        let md = export_markdown(&tasks);
        assert!(md.contains("[x]") && md.contains("[ ]"));
        assert!(md.contains("Implement cache"));
    }

    #[test]
    fn test_csv_export_has_header_and_rows() {
        let tasks = vec![t(1,"Do X","pending")];
        let csv = export_csv(&tasks);
        assert!(csv.starts_with("id,title"));
        assert!(csv.contains("Do X"));
    }
}
