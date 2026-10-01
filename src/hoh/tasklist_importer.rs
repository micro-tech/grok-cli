//! HOH TaskList Importer (Task 327.32)

use crate::hoh::tasklist_adapter::Task;

/// Parse a Markdown checklist into Task structs.
///
/// Supported format:
/// ```md
/// - [ ] Task title (priority)
///   > Description line
///   - [ ] Subtask title
/// ```
pub fn import_from_markdown(content: &str) -> Vec<Task> {
    let mut tasks = Vec::new();
    let mut next_id: u64 = 1000; // Start IDs high to avoid collisions

    for line in content.lines() {
        let trimmed = line.trim();

        // Top-level task: `- [ ] Title` or `- [x] Title`
        if let Some(rest) = trimmed.strip_prefix("- [") {
            let done = rest.starts_with("x]") || rest.starts_with("X]");
            let title_part = rest.trim_start_matches(|c: char| c != ']')
                .trim_start_matches(']').trim();

            // Extract priority from trailing `(priority)`
            let (title, priority) = if let Some(idx) = title_part.rfind('(') {
                let pr = title_part[idx+1..].trim_end_matches(')').trim().to_string();
                let ti = title_part[..idx].trim().to_string();
                (ti, if pr.is_empty() { "medium".to_string() } else { pr })
            } else {
                (title_part.to_string(), "medium".to_string())
            };

            if !title.is_empty() {
                tasks.push(Task {
                    id: next_id,
                    title,
                    status: if done { "done".to_string() } else { "pending".to_string() },
                    priority,
                    ..Default::default()
                });
                next_id += 1;
            }
        }

        // Description line: `  > description`
        if trimmed.starts_with("> ") {
            if let Some(last) = tasks.last_mut() {
                last.description = trimmed.trim_start_matches("> ").to_string();
            }
        }
    }
    tasks
}

/// Parse a simple CSV (id, title, status, priority).
pub fn import_from_csv(content: &str) -> Vec<Task> {
    let mut tasks = Vec::new();
    let mut lines = content.lines();
    let _header = lines.next(); // skip header

    for (i, line) in lines.enumerate() {
        let cols: Vec<&str> = line.splitn(4, ',').collect();
        if cols.len() < 2 { continue; }
        let id = cols[0].trim().parse::<u64>().unwrap_or(2000 + i as u64);
        let title = cols[1].trim().trim_matches('"').to_string();
        let status = cols.get(2).map(|s| s.trim().to_string()).unwrap_or_else(|| "pending".to_string());
        let priority = cols.get(3).map(|s| s.trim().to_string()).unwrap_or_else(|| "medium".to_string());
        if !title.is_empty() {
            tasks.push(Task { id, title, status, priority, ..Default::default() });
        }
    }
    tasks
}

/// Merge imported tasks into an existing task list (dedup by title).
pub fn merge_imported(existing: &mut Vec<Task>, imported: Vec<Task>) {
    let existing_titles: std::collections::HashSet<String> =
        existing.iter().map(|t| t.title.to_lowercase()).collect();
    for t in imported {
        if !existing_titles.contains(&t.title.to_lowercase()) {
            existing.push(t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_markdown_checklist() {
        let md = "- [ ] Implement cache (high)\n  > Cache LLM responses\n- [x] Write tests (medium)\n";
        let tasks = import_from_markdown(md);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].priority, "high");
        assert_eq!(tasks[0].status, "pending");
        assert_eq!(tasks[1].status, "done");
        assert_eq!(tasks[0].description, "Cache LLM responses");
    }

    #[test]
    fn test_import_csv() {
        let csv = "id,title,status,priority\n1,Do stuff,pending,high\n2,Fix bug,done,low\n";
        let tasks = import_from_csv(csv);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "Do stuff");
        assert_eq!(tasks[1].status, "done");
    }

    #[test]
    fn test_merge_deduplicates() {
        let mut existing = vec![Task { id: 1, title: "Existing task".to_string(), ..Default::default() }];
        let imported = vec![
            Task { id: 2, title: "Existing task".to_string(), ..Default::default() },
            Task { id: 3, title: "New task".to_string(), ..Default::default() },
        ];
        merge_imported(&mut existing, imported);
        assert_eq!(existing.len(), 2); // deduped
        assert!(existing.iter().any(|t| t.title == "New task"));
    }
}
