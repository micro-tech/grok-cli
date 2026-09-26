const fs = require('fs');

const mainPath = '.zed/task_list.json';
const archivePath = '.zed/archive/task_list_400-500.json';

// 1. Clean any stray task_list_411*.json
const stray = require('child_process').execSync('powershell -Command "Get-ChildItem -Path .zed -Recurse -Include *task_list_411* | ForEach-Object { $_.FullName }"', {encoding: 'utf8'}).trim();
if (stray) {
  console.log('Removing stray files:', stray);
  require('child_process').execSync('powershell -Command "Get-ChildItem -Path .zed -Recurse -Include *task_list_411* | Remove-Item -Force"', {encoding: 'utf8'});
} else {
  console.log('No stray task_list_411*.json files.');
}

// 2. Load main and ensure no 411
let main = JSON.parse(fs.readFileSync(mainPath, 'utf8'));
main.tasks = main.tasks.filter(t => Number(t.id) !== 411);
fs.writeFileSync(mainPath, JSON.stringify(main, null, 2));
console.log('Main task_list.json cleaned of any 411.');

// 3. Load archive (strip BOM)
let archiveText = fs.readFileSync(archivePath, 'utf8').replace(/^\uFEFF/, '');
let archive = JSON.parse(archiveText);

// 4. Remove any existing "Evaluation-Driven Self-Improvement Loop" or old id 411 self-improvement entry
archive.tasks = archive.tasks.filter(t => {
  const title = (t.title || '').toLowerCase();
  return !title.includes('evaluation-driven self-improvement') && Number(t.id) !== 411;
});

// 5. The task we want to archive (the completed one)
const selfImprovementTask = {
  "id": 411,
  "title": "Add Evaluation-Driven Self-Improvement Loop",
  "description": "Use evaluation results and task outcomes to suggest concrete improvements to prompts, heuristics, and agent behavior.",
  "status": "done",
  "dependencies": [361, 410],
  "priority": "high",
  "details": "Implemented (Task 411 + meta 434): Added SelfImprovementProposal struct + generate_self_improvement_proposals() in src/hoh/self_improvement.rs. Wired into outer_loop after evaluate_phase. Produces concrete reviewable proposals (test_strategy enforcement, patch quality heuristics, prompt injections, etc.) from evaluations + test_output + meta scores + patch metrics + improvement_suggestions. All human-review gated. Stored in HOHPlan.self_improvement_proposals. Updated state.rs, mod.rs, planner fallbacks. Tests present.",
  "testStrategy": "1. The system can produce 3-5 concrete improvement proposals from a batch of recent tasks.\n2. At least some proposals are accepted and show measurable lift.\n3. All proposals are logged with rationale and outcome.",
  "subtasks": []
};

// 6. Insert after the existing 410 (or at the end if not found)
let inserted = false;
for (let i = 0; i < archive.tasks.length; i++) {
  if (Number(archive.tasks[i].id) === 410) {
    archive.tasks.splice(i + 1, 0, selfImprovementTask);
    inserted = true;
    break;
  }
}
if (!inserted) {
  archive.tasks.push(selfImprovementTask);
}

fs.writeFileSync(archivePath, JSON.stringify(archive, null, 2));
console.log('SUCCESS: Task 411 (Evaluation-Driven Self-Improvement Loop) moved into task_list_400-500.json after id 410.');
console.log('Any task_list_411*.json deleted. Main task_list.json has no 411.');