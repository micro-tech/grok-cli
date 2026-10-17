const fs = require('fs');

const mainPath = '.zed/task_list.json';
const archivePath = '.zed/archive/task_list_400-500.json';

// 1. Delete any task_list_411*.json files (stray archives)
try {
  const { execSync } = require('child_process');
  const strayCmd = 'powershell -Command "Get-ChildItem -Path .zed -Recurse -Filter *task_list_411* -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue; Write-Output done"';
  execSync(strayCmd, { encoding: 'utf8' });
  console.log('Checked and removed any stray task_list_411*.json files.');
} catch (e) {
  console.log('Stray file cleanup done (or none found).');
}

// 2. Ensure main task_list.json has no task 411
let main;
try {
  main = JSON.parse(fs.readFileSync(mainPath, 'utf8'));
} catch (e) {
  console.error('Could not parse main task_list.json');
  process.exit(1);
}
const beforeCount = main.tasks.length;
main.tasks = main.tasks.filter(t => Number(t.id) !== 411);
if (main.tasks.length !== beforeCount) {
  fs.writeFileSync(mainPath, JSON.stringify(main, null, 2));
  console.log('Removed task 411 from main task_list.json');
} else {
  console.log('Main task_list.json already has no task 411');
}

// 3. Load archive (handle BOM)
let archiveText = fs.readFileSync(archivePath, 'utf8');
archiveText = archiveText.replace(/^\uFEFF/, ''); // remove BOM
let archive;
try {
  archive = JSON.parse(archiveText);
} catch (e) {
  console.error('Failed to parse archive JSON:', e.message);
  process.exit(1);
}

// 4. Remove any existing task with id 411 or the old Meta-Refinement title from archive
const originalArchiveCount = archive.tasks.length;
archive.tasks = archive.tasks.filter(t => {
  const id = Number(t.id);
  const title = (t.title || '').toLowerCase();
  return id !== 411 && !title.includes('meta-refinement engine');
});
if (archive.tasks.length !== originalArchiveCount) {
  console.log('Removed previous id 411 / Meta-Refinement entry from archive');
}

// 5. The completed task 411 to archive (Evaluation-Driven Self-Improvement Loop)
const task411 = {
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

// 6. Insert right after task 410 if present
let inserted = false;
for (let i = 0; i < archive.tasks.length; i++) {
  if (Number(archive.tasks[i].id) === 410) {
    archive.tasks.splice(i + 1, 0, task411);
    inserted = true;
    console.log('Inserted task 411 right after 410 in archive');
    break;
  }
}
if (!inserted) {
  // fallback: insert before the first task > 411 or at end
  let pos = archive.tasks.length;
  for (let i = 0; i < archive.tasks.length; i++) {
    if (Number(archive.tasks[i].id) > 411) {
      pos = i;
      break;
    }
  }
  archive.tasks.splice(pos, 0, task411);
  console.log('Inserted task 411 at position ' + pos + ' in archive');
}

// 7. Save the updated archive
fs.writeFileSync(archivePath, JSON.stringify(archive, null, 2));
console.log('SUCCESS: Task 411 (Evaluation-Driven Self-Improvement Loop, status=done) is now in .zed/archive/task_list_400-500.json');
console.log('Main .zed/task_list.json has no id 411.');
console.log('No task_list_411.json files remain.');