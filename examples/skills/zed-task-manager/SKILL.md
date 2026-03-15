---
name: zed-task-manager
description: Safely manages, appends, and updates tasks in JSON format (e.g. .zed/task_list.json) using standard parsing libraries instead of fragile shell regex.
version: 1.0.0
tags: [json, zed, task-management]
---

# Zed Task Manager

This skill provides the standard operating procedure for AI agents (and users) to update complex JSON configuration files like the Zed task list, completely avoiding the nightmare of command-line string escaping and regex replacement in PowerShell or inline `node -e`.

## <instructions>

1. **NEVER** use inline `node -e` or `powershell -Command` with regex/string replacement to edit `.zed/task_list.json` or other complex JSON files. It inevitably leads to escaping hell and corrupted JSON.
2. **ALWAYS** write a temporary script (e.g., `update_tasks.js` or `update_tasks.py`) using the `write_file` tool to the workspace root.
3. The script **MUST** structurally parse the JSON (`JSON.parse()` or `json.load()`), modify the in-memory object directly, and write it back nicely formatted (`JSON.stringify(data, null, 2)`).
4. Run the script using `run_shell_command` (e.g., `node update_tasks.js`).
5. After verifying the script executed successfully, ALWAYS clean up by deleting the temporary script.

## Example Node.js Workflow Template

```javascript
const fs = require('fs');
const file = '.zed/task_list.json';

try {
    // 1. Read and parse structurally
    const data = JSON.parse(fs.readFileSync(file, 'utf8'));
    
    // 2. Locate and modify the object directly
    // e.g., Mark task 34 as done
    const task = data.find(t => t.id === 34);
    if (task) {
        task.status = 'done';
        if (task.subtasks) {
            task.subtasks.forEach(st => st.status = 'done');
        }
    }
    
    // 3. Write back with proper formatting
    fs.writeFileSync(file, JSON.stringify(data, null, 2), 'utf8');
    console.log('Successfully updated task list.');
} catch (err) {
    console.error('Error updating task list:', err);
    process.exit(1);
}
```
</instructions>
