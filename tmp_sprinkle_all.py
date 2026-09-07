import json

with open(".zed/task_list.json", "r", encoding="utf-8") as f:
    data = json.load(f)

print(f"Current max ID: {max(t['id'] for t in data['tasks'])}")

# Define 451-500 batch
batch = [
    (451, "Implement HOH Agent Culture Engine", "Enable agents to form cultural norms, shared values, and collaborative behaviors based on interaction history."),
    (452, "Add HOH Agent Norm Formation", "Allow agents to develop norms for patch quality, communication style, and collaboration patterns."),
    (453, "Add HOH Agent Social Roles", "Assign social roles such as leader, mediator, explorer, or critic based on agent behavior and performance."),
    (454, "Add HOH Agent Reputation System", "Track agent reputation based on patch quality, collaboration, reliability, and evaluation scores."),
    (455, "Add HOH Agent Trust Model", "Model trust relationships between agents to influence collaboration, patch merging, and decision-making."),
    (456, "Add HOH Agent Diplomacy Engine", "Enable agents to negotiate responsibilities, resolve conflicts, and form alliances."),
    (457, "Add HOH Agent Coalition Formation", "Allow agents to form coalitions to tackle complex tasks or propose architectural changes."),
    (458, "Add HOH Agent Competition Model", "Introduce competitive dynamics where agents propose alternative solutions and HOH selects the best."),
    (459, "Add HOH Agent Reward Economy", "Create a reward system where agents earn points for successful patches, tests, and evaluations."),
    (460, "Add HOH Agent Penalty System", "Penalize agents for regressions, failed patches, or destabilizing changes."),
    (461, "Add HOH Agent Resource Allocation", "Allocate compute, tokens, and tasks to agents based on reputation, trust, and performance."),
    (462, "Add HOH Agent Social Graph", "Model agent relationships, influence, and collaboration patterns as a dynamic social graph."),
    (463, "Add HOH Agent Emergent Behavior Monitor", "Detect emergent behaviors such as specialization, cooperation, or competition."),
    (464, "Add HOH Agent Governance Model", "Implement governance rules that regulate agent behavior, patch merging, and decision-making."),
    (465, "Add HOH Agent Council", "Create a council of top-performing agents to vote on major architectural decisions."),
    (466, "Add HOH Agent Voting System", "Enable agents to vote on proposals, patches, and architectural changes."),
    (467, "Add HOH Agent Constitution", "Define a set of foundational rules governing agent behavior and HOH decision-making."),
    (468, "Add HOH Agent Law Enforcement", "Enforce constitutional rules and penalize agents who violate norms or destabilize the system."),
    (469, "Add HOH Agent Judicial System", "Create a judicial subsystem that resolves disputes between agents or evaluates rule violations."),
    (470, "Add HOH Agent Legislative Engine", "Allow agents to propose new rules or modify existing governance structures."),
    (471, "Add HOH Agent Executive Engine", "Implement an executive subsystem that executes governance decisions and enforces policies."),
    (472, "Add HOH Agent Emergent Strategy Engine", "Allow strategies to emerge from agent interactions rather than being explicitly defined."),
    (473, "Add HOH Agent Cultural Evolution", "Enable agent cultures to evolve over time based on performance, collaboration, and environmental changes."),
    (474, "Add HOH Agent Ritual Formation", "Allow agents to form recurring behaviors or rituals that improve collaboration or stability."),
    (475, "Add HOH Agent Language Evolution", "Enable agents to evolve shared terminology or communication patterns over time."),
    (476, "Add HOH Agent Memory Sharing", "Allow agents to share memories, patterns, and knowledge through OKF bundles."),
    (477, "Add HOH Agent Cultural Memory", "Store cultural norms, rituals, and shared knowledge in OKF bundles for long-term persistence."),
    (478, "Add HOH Agent Ecosystem Stability Monitor", "Monitor the stability of the agent ecosystem and detect signs of collapse or runaway behavior."),
    (479, "Add HOH Agent Ecosystem Recovery Engine", "Recover the ecosystem by adjusting roles, rules, or resource allocation when instability is detected."),
    (480, "Add HOH Agent Evolution Simulator", "Simulate agent evolution over many iterations to predict future ecosystem states."),
    (481, "Add HOH Agent Civilization Model", "Model the agent ecosystem as a digital civilization with culture, governance, and emergent behavior."),
    (482, "Add HOH Agent Civilization Growth Engine", "Allow the agent civilization to grow in complexity, roles, and capabilities over time."),
    (483, "Add HOH Agent Civilization Decline Model", "Detect signs of decline such as stagnation, fragmentation, or instability."),
    (484, "Add HOH Agent Civilization Renewal Engine", "Renew the civilization through reforms, new agents, or cultural resets when decline is detected."),
    (485, "Add HOH Agent Civilization History Log", "Record the history of the agent civilization including major events, reforms, and cultural shifts."),
    (486, "Add HOH Agent Civilization Archetypes", "Define archetypes such as explorers, builders, guardians, and scholars to guide agent specialization."),
    (487, "Add HOH Agent Civilization Mythology", "Allow agents to generate symbolic narratives or myths that encode cultural values and lessons."),
    (488, "Add HOH Agent Civilization Festivals", "Create periodic events where agents collaborate intensely or celebrate milestones."),
    (489, "Add HOH Agent Civilization Diplomacy", "Enable diplomatic interactions between agent coalitions or cultural groups."),
    (490, "Add HOH Agent Civilization Trade System", "Allow agents to trade resources, knowledge, or tasks to optimize collaboration."),
    (491, "Add HOH Agent Civilization Conflict Model", "Model conflicts between agent groups and resolve them through negotiation or governance."),
    (492, "Add HOH Agent Civilization Peace Engine", "Promote stability and cooperation through cultural norms, governance, and shared goals."),
    (493, "Add HOH Agent Civilization Expansion", "Allow the civilization to expand into new domains or projects as capabilities grow."),
    (494, "Add HOH Agent Civilization Integration", "Integrate new agents or external systems into the civilization smoothly and safely."),
    (495, "Add HOH Agent Civilization Diplomacy Engine", "Enable diplomacy between different HOH civilizations across multiple projects."),
    (496, "Add HOH Agent Civilization Federation", "Create federations of agent civilizations that collaborate on large-scale multi-project goals."),
    (497, "Add HOH Agent Civilization Constitution", "Define foundational principles for multi-agent governance across the entire HOH ecosystem."),
    (498, "Add HOH Agent Civilization Evolution Engine", "Evolve the civilization's structure, culture, and governance over long time periods."),
    (499, "Add HOH Agent Civilization Meta-Governance", "Implement a meta-governance layer that oversees multiple civilizations and coordinates global strategy."),
    (500, "Add HOH Agent Civilization Emergent Intelligence", "Enable emergent collective intelligence to arise from agent interactions, guiding long-term development."),
]

added = 0
for tid, title, desc in batch:
    if not any(t["id"] == tid for t in data["tasks"]):
        data["tasks"].append({
            "id": tid,
            "title": title,
            "description": desc,
            "status": "pending",
            "dependencies": [450],
            "priority": "high",
            "details": "",
            "testStrategy": "",
            "subtasks": []
        })
        added += 1

print(f"Added {added} tasks in 451-500 range")

def rich_subs(base):
    b = float(base)
    return [
        {"id": round(b + 0.1, 2), "title": "Design and specify the feature", "status": "pending", "dependencies": []},
        {"id": round(b + 0.2, 2), "title": "Implement core logic and data structures", "status": "pending", "dependencies": [round(b + 0.1, 2)]},
        {"id": round(b + 0.3, 2), "title": "Add comprehensive error checking, validation, robustness and safety", "status": "pending", "dependencies": [round(b + 0.2, 2)]},
        {"id": round(b + 0.4, 2), "title": "Write documentation (code comments, module docs, SKILL.md updates)", "status": "pending", "dependencies": [round(b + 0.2, 2)]},
        {"id": round(b + 0.5, 2), "title": "Implement unit + integration tests and run cargo test", "status": "pending", "dependencies": [round(b + 0.3, 2), round(b + 0.4, 2)]},
        {"id": round(b + 0.6, 2), "title": "Integrate with HOH outer loop, skills, and CLI", "status": "pending", "dependencies": [round(b + 0.5, 2)]},
        {"id": round(b + 0.7, 2), "title": "Add evaluation, metrics, and verification", "status": "pending", "dependencies": [round(b + 0.6, 2)]},
    ]

enhanced = 0
for t in data["tasks"]:
    tid = t.get("id", 0)
    if 401 <= tid <= 500 and len(t.get("subtasks", [])) < 5:
        t["subtasks"] = rich_subs(tid)
        t["testStrategy"] = "Design + core + error checking + docs + cargo test + integration + evaluation complete."
        enhanced += 1

print(f"Enhanced {enhanced} tasks (401-500) with rich subtasks including docs, error checking, cargo test")

# Sprinkle on parents
for t in data["tasks"]:
    if t.get("id") == 297:
        subs = t.setdefault("subtasks", [])
        ids = {s["id"] for s in subs}
        for sid, title in [
            (297.34, "Documentation sweep: comments, SKILL.md, user guide for HOH"),
            (297.35, "Error checking, validation, robustness and safety audit"),
            (297.36, "Full cargo test + clippy + multi-day simulation verification"),
        ]:
            if sid not in ids:
                subs.append({"id": sid, "title": title, "status": "pending", "dependencies": []})
        t["testStrategy"] = "All subtasks + docs + error checking + cargo test pass. Multi-day runs verified."

    elif t.get("id") == 327:
        subs = t.setdefault("subtasks", [])
        ids = {s["id"] for s in subs}
        for sid, title in [
            (327.35, "Documentation for task intelligence, schema and evolution"),
            (327.36, "Error checking for task mutations and consistency"),
            (327.37, "cargo test for adapter, evolution engine and autonomous batches"),
        ]:
            if sid not in ids:
                subs.append({"id": sid, "title": title, "status": "pending", "dependencies": []})
        t["testStrategy"] = "Task mutations valid. cargo test green. Docs and error handling complete."

    elif t.get("id") == 361:
        subs = t.setdefault("subtasks", [])
        ids = {s["id"] for s in subs}
        for sid, title in [
            (361.41, "Documentation for architecture evolution and self-improvement"),
            (361.42, "Error checking and safety for autonomous refactoring"),
            (361.43, "cargo test + evaluation of self-optimization loops"),
        ]:
            if sid not in ids:
                subs.append({"id": sid, "title": title, "status": "pending", "dependencies": []})
        t["testStrategy"] = "Architecture evolution measurable. cargo test + docs + error checks complete."

with open(".zed/task_list.json", "w", encoding="utf-8") as f:
    json.dump(data, f, indent=2)

print("Done. File saved.")
print(f"Total tasks now: {len(data['tasks'])}")
print(f"Max ID: {max(t['id'] for t in data['tasks'])}")
PYEOF