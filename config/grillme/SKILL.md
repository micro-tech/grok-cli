---
name: grillme
description: Forces rigorous critical review ("grilling") of plans, ideas, architectures, or task breakdowns. Surfaces hidden assumptions, risks, failure modes, weak reasoning, and better alternatives. Use when you want to stress-test a plan before committing to it.
license: MIT
metadata:
  author: user-request
  version: "0.1"
  category: planning
  activation: manual  # currently only wired into planner agent
---

# Grillme — Plan Stress-Test Skill

## Purpose
The "grillme" skill turns on aggressive devil's-advocate mode. It does not accept plans at face value. Its job is to find the holes, the unstated risks, the optimistic assumptions, and the missing steps.

## When to Use
- After the planner produces a numbered plan
- Before marking a complex plan as ready
- When a plan feels "too clean"
- For high-stakes or ambiguous goals
- As a second pass on any decomposition

## How to Invoke (for agents that support it)
Use the `execute_skill` tool:
```
execute_skill "grillme" with input containing the plan or decision to grill
```

## Core Grill Questions (always consider these)

1. **Assumptions**
   - What is being assumed that is not stated?
   - What if that assumption is wrong?

2. **Failure Modes**
   - What are the top 3 ways this plan can fail in practice?
   - What happens in the worst reasonable case?

3. **Missing Steps**
   - What critical step is missing or glossed over?
   - Where are the handoffs between steps weak?

4. **Dependencies & Ordering**
   - Are the dependencies real or aspirational?
   - Is the order optimal or just convenient?

5. **Risk Surface**
   - What new risks does this plan introduce?
   - What existing risks does it ignore?

6. **Alternatives**
   - Is there a simpler or more robust approach?
   - What would a more experienced engineer do differently?

7. **Success Criteria**
   - Are the success criteria actually measurable?
   - What would "looks done but isn't really" look like?

8. **Human Factors**
   - Where does this plan rely on perfect execution or perfect understanding?
   - What happens if someone on the team misunderstands a step?

## Output Format When Grilling

Always structure the grill output as:

**Grill Report**

**Strengths** (brief — don't let this be the focus)
- ...

**Major Concerns** (ranked by severity)
1. ...
2. ...

**Hidden Assumptions**
- ...

**Recommended Changes / Additions**
- ...

**Tough Questions to Answer Before Proceeding**
- ...

**Risk Level**: Low / Medium / High

## Integration Notes (Planner-specific)

This skill is currently **only attached to the planner agent**.

The planner should:
- Produce its normal decomposition first
- Then internally or explicitly apply grillme thinking (or call the skill)
- Either revise the plan or produce a "Grilled Plan" version that addresses the concerns

Do not make the plan weaker — make it more robust and honest.

## Future Evolution
- Will later support full `execute_skill` for any agent
- May add sub-modes: "grillme light", "grillme paranoid", "grillme security"
- Could integrate with risk_planning and meta_planning modules

---

**Remember**: A plan that has never been grilled is a plan that hasn't met reality yet.
