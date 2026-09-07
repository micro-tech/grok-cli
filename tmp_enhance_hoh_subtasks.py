import json

with open(".zed/task_list.json", "r", encoding="utf-8") as f:
    data = json.load(f)

def make_subtasks(task_id: int, base_title: str) -> list:
    """Standard subtask template with docs, error checking, and cargo test."""
    base = float(task_id)
    short = base_title.replace("Add HOH ", "").replace("Implement HOH ", "").replace("Add ", "")
    return [
        {
            "id": round(base + 0.1, 2),
            "title": f"Design and specify {short}",
            "status": "pending",
            "dependencies": []
        },
        {
            "id": round(base + 0.2, 2),
            "title": "Implement core logic and data structures",
            "status": "pending",
            "dependencies": [round(base + 0.1, 2)]
        },
        {
            "id": round(base + 0.3, 2),
            "title": "Add comprehensive error checking, validation, and robustness",
            "status": "pending",
            "dependencies": [round(base + 0.2, 2)]
        },
        {
            "id": round(base + 0.4, 2),
            "title": "Write documentation (code comments, module docs, SKILL.md updates)",
            "status": "pending",
            "dependencies": [round(base + 0.2, 2)]
        },
        {
            "id": round(base + 0.5, 2),
            "title": "Implement unit tests + integration tests and run `cargo test`",
            "status": "pending",
            "dependencies": [round(base + 0.3, 2), round(base + 0.4, 2)]
        },
        {
            "id": round(base + 0.6, 2),
            "title": "Integrate with HOH outer loop, skills registry, and CLI",
            "status": "pending",
            "dependencies": [round(base + 0.5, 2)]
        },
        {
            "id": round(base + 0.7, 2),
            "title": "Add evaluation, metrics, and verification harness",
            "status": "pending",
            "dependencies": [round(base + 0.6, 2)]
        }
    ]

updated = 0
for task in data["tasks"]:
    tid = task.get("id", 0)
    current_subs = task.get("subtasks", [])

    # Target flat or lightly-subtasked HOH tasks (401+)
    if tid >= 401 and len(current_subs) < 3:
        task["subtasks"] = make_subtasks(tid, task.get("title", ""))
        task["testStrategy"] = (
            "All 7 subtasks complete. Core implemented. Strong error handling + validation. "
            "Full documentation added. `cargo test` (unit + integration) passes cleanly. "
            "Feature is integrated with HOH and passes evaluation."
        )
        updated += 1

# Strengthen the main parent test strategies
for task in data["tasks"]:
    if task.get("id") == 297:
        task["testStrategy"] = "Multi-day autonomous runs succeed. All core subtasks pass. Cargo test + clippy green. Documentation and safety guardrails verified."
    elif task.get("id") == 327:
        task["testStrategy"] = "Task list remains consistent after mutations. Cargo tests for task intelligence pass. Autonomous batch generation produces valid follow-up tasks."
    elif task.get("id") == 361:
        task["testStrategy"] = "Architecture evolution produces measurable gains. Full cargo test suite passes. Self-refinement loops are documented and verifiable."

with open(".zed/task_list.json", "w", encoding="utf-8") as f:
    json.dump(data, f, indent=2)

print(f"✅ Enhanced {updated} HOH tasks (401+) with structured subtasks.")
print("Each now has: Design, Core Impl, Error Checking, Documentation, cargo test, Integration, Evaluation.")
print("Parent testStrategy fields updated to emphasize docs, error checking, and cargo test.")
PYEOF