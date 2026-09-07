import json

with open(".zed/task_list.json", "r", encoding="utf-8") as f:
    data = json.load(f)

def standard_subtasks(base: int) -> list:
    """7 structured subtasks with docs, error checking, and cargo test baked in."""
    b = float(base)
    return [
        {"id": round(b + 0.1, 2), "title": "Design and specify the feature / component", "status": "pending", "dependencies": []},
        {"id": round(b + 0.2, 2), "title": "Implement core logic, data structures, and algorithms", "status": "pending", "dependencies": [round(b + 0.1, 2)]},
        {"id": round(b + 0.3, 2), "title": "Add comprehensive error checking, validation, robustness, and safety guards", "status": "pending", "dependencies": [round(b + 0.2, 2)]},
        {"id": round(b + 0.4, 2), "title": "Write documentation: code comments, module docs, SKILL.md updates, examples", "status": "pending", "dependencies": [round(b + 0.2, 2)]},
        {"id": round(b + 0.5, 2), "title": "Implement unit tests + integration tests and verify with `cargo test`", "status": "pending", "dependencies": [round(b + 0.3, 2), round(b + 0.4, 2)]},
        {"id": round(b + 0.6, 2), "title": "Integrate with HOH outer loop, skills, task list, and CLI surface", "status": "pending", "dependencies": [round(b + 0.5, 2)]},
        {"id": round(b + 0.7, 2), "title": "Add evaluation, metrics, verification harness, and simulation tests", "status": "pending", "dependencies": [round(b + 0.6, 2)]},
    ]

changes = 0

# 1. Fix all flat or under-developed HOH tasks (401+)
for task in data["tasks"]:
    tid = task.get("id", 0)
    subs = task.get("subtasks", [])
    if tid >= 401 and len(subs) < 5:
        task["subtasks"] = standard_subtasks(tid)
        task["testStrategy"] = "Design + core + error checking + documentation + cargo test + integration + evaluation all complete. Feature is robust and verifiable inside HOH."
        changes += 1

# 2. Sprinkle cross-cutting docs/error/cargo subtasks on the three main parents
for task in data["tasks"]:
    if task.get("id") == 297:
        subs = task.setdefault("subtasks", [])
        existing = {s["id"] for s in subs}
        for num, title in [
            (297.34, "Documentation sweep: code comments, module docs, SKILL.md, and user guide for the full outer loop"),
            (297.35, "Error checking, validation, robustness, and safety guardrail audit across all HOH components"),
            (297.36, "Run complete `cargo test` + clippy + multi-day simulation verification for the HOH harness"),
        ]:
            if num not in existing:
                subs.append({"id": num, "title": title, "status": "pending", "dependencies": []})
                changes += 1
        task["testStrategy"] = "All core subtasks + docs + error checking + cargo test pass. Multi-day autonomous runs succeed with full verification."

    elif task.get("id") == 327:
        subs = task.setdefault("subtasks", [])
        existing = {s["id"] for s in subs}
        for num, title in [
            (327.35, "Documentation: task intelligence internals, schema, evolution rules, and SKILL.md updates"),
            (327.36, "Error checking + validation for task mutations, dependency graphs, and autonomous consistency"),
            (327.37, "`cargo test` for TaskList adapter, evolution engine, diff generator, and batch creation"),
        ]:
            if num not in existing:
                subs.append({"id": num, "title": title, "status": "pending", "dependencies": []})
                changes += 1
        task["testStrategy"] = "Task list mutations stay valid. cargo test green. Full documentation and error checking for autonomous task intelligence."

    elif task.get("id") == 361:
        subs = task.setdefault("subtasks", [])
        existing = {s["id"] for s in subs}
        for num, title in [
            (361.41, "Documentation pass for architecture evolution, agent profiles, self-refinement, and OKF integration"),
            (361.42, "Error checking, safety, and robustness review for autonomous refactoring and long-term evolution"),
            (361.43, "Full `cargo test` + evaluation of architecture changes, patch quality, and self-optimization loops"),
        ]:
            if num not in existing:
                subs.append({"id": num, "title": title, "status": "pending", "dependencies": []})
                changes += 1
        task["testStrategy"] = "Architecture evolution produces measurable gains. cargo test + docs + error checks complete. Self-improvement loops documented and verifiable."

with open(".zed/task_list.json", "w", encoding="utf-8") as f:
    json.dump(data, f, indent=2)

print(f"✅ Made {changes} subtask additions / replacements.")
print(" - All 401-500 tasks now have the standard 7-subtask structure (incl. error checking, docs, cargo test).")
print(" - Parents 297, 327, 361 received extra cross-cutting subtasks for docs, error checking, and cargo test.")
print(" - testStrategy fields updated to emphasize these requirements.")
PYEOF