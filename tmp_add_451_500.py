import json

with open('.zed/task_list.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

new_batch = [
    {"id": 451, "title": "Implement HOH Agent Culture Engine", "description": "Enable agents to form cultural norms, shared values, and collaborative behaviors based on interaction history."},
    {"id": 452, "title": "Add HOH Agent Norm Formation", "description": "Allow agents to develop norms for patch quality, communication style, and collaboration patterns."},
    {"id": 453, "title": "Add HOH Agent Social Roles", "description": "Assign social roles such as leader, mediator, explorer, or critic based on agent behavior and performance."},
    {"id": 454, "title": "Add HOH Agent Reputation System", "description": "Track agent reputation based on patch quality, collaboration, reliability, and evaluation scores."},
    {"id": 455, "title": "Add HOH Agent Trust Model", "description": "Model trust relationships between agents to influence collaboration, patch merging, and decision-making."},
    {"id": 456, "title": "Add HOH Agent Diplomacy Engine", "description": "Enable agents to negotiate responsibilities, resolve conflicts, and form alliances."},
    {"id": 457, "title": "Add HOH Agent Coalition Formation", "description": "Allow agents to form coalitions to tackle complex tasks or propose architectural changes."},
    {"id": 458, "title": "Add HOH Agent Competition Model", "description": "Introduce competitive dynamics where agents propose alternative solutions and HOH selects the best."},
    {"id": 459, "title": "Add HOH Agent Reward Economy", "description": "Create a reward system where agents earn points for successful patches, tests, and evaluations."},
    {"id": 460, "title": "Add HOH Agent Penalty System", "description": "Penalize agents for regressions, failed patches, or destabilizing changes."},
    {"id": 461, "title": "Add HOH Agent Resource Allocation", "description": "Allocate compute, tokens, and tasks to agents based on reputation, trust, and performance."},
    {"id": 462, "title": "Add HOH Agent Social Graph", "description": "Model agent relationships, influence, and collaboration patterns as a dynamic social graph."},
    {"id": 463, "title": "Add HOH Agent Emergent Behavior Monitor", "description": "Detect emergent behaviors such as specialization, cooperation, or competition."},
    {"id": 464, "title": "Add HOH Agent Governance Model", "description": "Implement governance rules that regulate agent behavior, patch merging, and decision-making."},
    {"id": 465, "title": "Add HOH Agent Council", "description": "Create a council of top-performing agents to vote on major architectural decisions."},
    {"id": 466, "title": "Add HOH Agent Voting System", "description": "Enable agents to vote on proposals, patches, and architectural changes."},
    {"id": 467, "title": "Add HOH Agent Constitution", "description": "Define a set of foundational rules governing agent behavior and HOH decision-making."},
    {"id": 468, "title": "Add HOH Agent Law Enforcement", "description": "Enforce constitutional rules and penalize agents who violate norms or destabilize the system."},
    {"id": 469, "title": "Add HOH Agent Judicial System", "description": "Create a judicial subsystem that resolves disputes between agents or evaluates rule violations."},
    {"id": 470, "title": "Add HOH Agent Legislative Engine", "description": "Allow agents to propose new rules or modify existing governance structures."},
    {"id": 471, "title": "Add HOH Agent Executive Engine", "description": "Implement an executive subsystem that executes governance decisions and enforces policies."},
    {"id": 472, "title": "Add HOH Agent Emergent Strategy Engine", "description": "Allow strategies to emerge from agent interactions rather than being explicitly defined."},
    {"id": 473, "title": "Add HOH Agent Cultural Evolution", "description": "Enable agent cultures to evolve over time based on performance, collaboration, and environmental changes."},
    {"id": 474, "title": "Add HOH Agent Ritual Formation", "description": "Allow agents to form recurring behaviors or rituals that improve collaboration or stability."},
    {"id": 475, "title": "Add HOH Agent Language Evolution", "description": "Enable agents to evolve shared terminology or communication patterns over time."},
    {"id": 476, "title": "Add HOH Agent Memory Sharing", "description": "Allow agents to share memories, patterns, and knowledge through OKF bundles."},
    {"id": 477, "title": "Add HOH Agent Cultural Memory", "description": "Store cultural norms, rituals, and shared knowledge in OKF bundles for long-term persistence."},
    {"id": 478, "title": "Add HOH Agent Ecosystem Stability Monitor", "description": "Monitor the stability of the agent ecosystem and detect signs of collapse or runaway behavior."},
    {"id": 479, "title": "Add HOH Agent Ecosystem Recovery Engine", "description": "Recover the ecosystem by adjusting roles, rules, or resource allocation when instability is detected."},
    {"id": 480, "title": "Add HOH Agent Evolution Simulator", "description": "Simulate agent evolution over many iterations to predict future ecosystem states."},
    {"id": 481, "title": "Add HOH Agent Civilization Model", "description": "Model the agent ecosystem as a digital civilization with culture, governance, and emergent behavior."},
    {"id": 482, "title": "Add HOH Agent Civilization Growth Engine", "description": "Allow the agent civilization to grow in complexity, roles, and capabilities over time."},
    {"id": 483, "title": "Add HOH Agent Civilization Decline Model", "description": "Detect signs of decline such as stagnation, fragmentation, or instability."},
    {"id": 484, "title": "Add HOH Agent Civilization Renewal Engine", "description": "Renew the civilization through reforms, new agents, or cultural resets when decline is detected."},
    {"id": 485, "title": "Add HOH Agent Civilization History Log", "description": "Record the history of the agent civilization including major events, reforms, and cultural shifts."},
    {"id": 486, "title": "Add HOH Agent Civilization Archetypes", "description": "Define archetypes such as explorers, builders, guardians, and scholars to guide agent specialization."},
    {"id": 487, "title": "Add HOH Agent Civilization Mythology", "description": "Allow agents to generate symbolic narratives or myths that encode cultural values and lessons."},
    {"id": 488, "title": "Add HOH Agent Civilization Festivals", "description": "Create periodic events where agents collaborate intensely or celebrate milestones."},
    {"id": 489, "title": "Add HOH Agent Civilization Diplomacy", "description": "Enable diplomatic interactions between agent coalitions or cultural groups."},
    {"id": 490, "title": "Add HOH Agent Civilization Trade System", "description": "Allow agents to trade resources, knowledge, or tasks to optimize collaboration."},
    {"id": 491, "title": "Add HOH Agent Civilization Conflict Model", "description": "Model conflicts between agent groups and resolve them through negotiation or governance."},
    {"id": 492, "title": "Add HOH Agent Civilization Peace Engine", "description": "Promote stability and cooperation through cultural norms, governance, and shared goals."},
    {"id": 493, "title": "Add HOH Agent Civilization Expansion", "description": "Allow the civilization to expand into new domains or projects as capabilities grow."},
    {"id": 494, "title": "Add HOH Agent Civilization Integration", "description": "Integrate new agents or external systems into the civilization smoothly and safely."},
    {"id": 495, "title": "Add HOH Agent Civilization Diplomacy Engine", "description": "Enable diplomacy between different HOH civilizations across multiple projects."},
    {"id": 496, "title": "Add HOH Agent Civilization Federation", "description": "Create federations of agent civilizations that collaborate on large-scale multi-project goals."},
    {"id": 497, "title": "Add HOH Agent Civilization Constitution", "description": "Define foundational principles for multi-agent governance across the entire HOH ecosystem."},
    {"id": 498, "title": "Add HOH Agent Civilization Evolution Engine", "description": "Evolve the civilization's structure, culture, and governance over long time periods."},
    {"id": 499, "title": "Add HOH Agent Civilization Meta-Governance", "description": "Implement a meta-governance layer that oversees multiple civilizations and coordinates global strategy."},
    {"id": 500, "title": "Add HOH Agent Civilization Emergent Intelligence", "description": "Enable emergent collective intelligence to arise from agent interactions, guiding long-term development."}
]

for item in new_batch:
    data['tasks'].append({
        "id": item["id"],
        "title": item["title"],
        "description": item["description"],
        "status": "pending",
        "dependencies": [450],
        "priority": "high",
        "details": "",
        "testStrategy": "",
        "subtasks": []
    })

# Update task 297 details
for t in data['tasks']:
    if t.get('id') == 297:
        t['details'] = """Core HOH outer loop tasks (297-326).

All original numbers are shown in subtask titles for easy reference.

Sibling batches (IDs match title ranges):
- Task 327 = HOH TaskList Intelligence and Autonomous Evolution (327-360)
- Task 361 = HOH Advanced Autonomy, Architecture Evolution and Self-Improvement (361-400)
- Tasks 401-450 = Meta-HOH, Creativity, Governance
- Tasks 451-500 = Agent Culture, Civilization, Emergent Governance"""

with open('.zed/task_list.json', 'w', encoding='utf-8') as f:
    json.dump(data, f, indent=2)

print("SUCCESS: Added tasks 451-500 following SKILL.md rules")
print("Total tasks:", len(data['tasks']))
print("All new tasks depend on 450")
PYEOF