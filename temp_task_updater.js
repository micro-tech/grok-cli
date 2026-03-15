const fs = require('fs');
const file = '.zed/task_list.json';

try {
  const data = JSON.parse(fs.readFileSync(file, 'utf8'));

  data.push({
    id: 35,
    title: "Create Zed Task Manager Skill",
    description: "Added a robust Zed Task Manager skill and system memory rule to ensure AI agents never use inline shell regex manipulation to edit complex JSON files again.",
    status: "done",
    dependencies: [],
    priority: "medium",
    subtasks: [
      {
        id: 35.1,
        title: "Create SKILL.md template",
        status: "done",
        dependencies: []
      },
      {
        id: 35.2,
        title: "Add persistent system rule to agent memory",
        status: "done",
        dependencies: [35.1]
      }
    ]
  });

  fs.writeFileSync(file, JSON.stringify(data, null, 2), 'utf8');
  console.log('Task 35 successfully appended using proper JSON parsing!');
} catch (err) {
  console.error('Failed to update task list:', err);
  process.exit(1);
}
