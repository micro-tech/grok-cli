---
name: task-list
description: Expert guidance for creating, managing, and executing tasks in the task management system. Helps with task structure, dependencies, prioritization, breaking down complex tasks, and tracking progress. Use when working with task_list.json or when planning and organizing project tasks.
license: MIT
metadata:
  author: john mcconnell john.microtech@gmail.com
  version: "1.0"
  category: project-management
---

# Task List Management Skill

## Overview

This skill provides expert guidance for creating, managing, and executing tasks using the task management system. Tasks are stored in the `task_list.json` file located in the `.zed` folder and follow a structured format with dependencies, priorities, and detailed implementation instructions.

## Core Principles

1. **Clarity**: Every task should be clear, specific, and actionable
2. **Dependencies**: Respect task dependencies to ensure proper execution order
3. **Prioritization**: Focus on high-priority tasks that block progress
4. **Documentation**: Keep task status updated and document all changes
5. **Testing**: Always include a test strategy to verify completion

---

# PART 1: TASK BUILDER - Creating and Managing Tasks

## Task Structure

Each task in the `task_list.json` file follows this structure:

```json
{
  "id": 1,
  "title": "Task Title",
  "description": "Brief description of the task",
  "status": "pending",
  "dependencies": [2, 3],
  "priority": "high",
  "details": "Detailed implementation instructions...",
  "testStrategy": "How to verify the task is complete",
  "subtasks": []
}
```

## Task Fields Reference

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | Number | Yes | Unique identifier (can use decimals for subtasks: 5.1, 5.2) |
| `title` | String | Yes | Brief, descriptive title of the task |
| `description` | String | Yes | Concise description of what the task involves |
| `status` | String | Yes | Current state: "pending", "in_progress", "done", "deferred" |
| `dependencies` | Array | Yes | IDs of tasks that must be completed first (empty array if none) |
| `priority` | String | Yes | Importance level: "high", "medium", "low" |
| `details` | String | Yes | In-depth implementation instructions |
| `testStrategy` | String | Yes | How to verify the task is complete |
| `subtasks` | Array | Yes | List of smaller tasks (empty array if none) |

## Creating New Tasks

When adding a new task, follow these steps:

### Step 1: Determine Task ID
- Find the highest existing task ID
- Increment by 1 for the next task
- Use decimal notation for subtasks (e.g., 5.1, 5.2, 5.3)

### Step 2: Write Clear Title and Description
- **Title**: Action-oriented, concise (e.g., "Implement user authentication")
- **Description**: One sentence explaining what and why

### Step 3: Identify Dependencies
- List IDs of tasks that MUST be completed before this task can start
- Only include true blockers, not nice-to-haves
- Avoid circular dependencies

### Step 4: Set Priority
- **high**: Critical tasks that block other work
- **medium**: Important but not blocking
- **low**: Nice-to-have or future enhancements

### Step 5: Write Detailed Instructions
- Provide enough detail for someone to implement without asking questions
- Include technical specifications, file paths, function names
- List any required resources or dependencies

### Step 6: Define Test Strategy
- How will you know the task is complete?
- What should be tested?
- What are the success criteria?

### Step 7: Break Down Complex Tasks
- If a task takes more than a few hours, break it into subtasks
- Each subtask should be a focused, achievable unit of work

## Breaking Down Complex Tasks

### When to Use Subtasks

Create subtasks when:
- The main task is too large or complex
- Multiple steps need to happen in a specific order
- Different parts require different skills or time
- You want to track progress incrementally

### Subtask Example

```json
{
  "id": 5,
  "title": "Implement User Authentication System",
  "description": "Build complete authentication with login, registration, and password reset",
  "status": "pending",
  "dependencies": [3, 4],
  "priority": "high",
  "details": "Create authentication system using JWT tokens with refresh token support",
  "testStrategy": "Test login, registration, password reset, and token refresh flows",
  "subtasks": [
    {
      "id": 5.1,
      "title": "Design authentication database schema",
      "status": "pending",
      "dependencies": []
    },
    {
      "id": 5.2,
      "title": "Implement user registration endpoint",
      "status": "pending",
      "dependencies": [5.1]
    },
    {
      "id": 5.3,
      "title": "Implement login endpoint with JWT",
      "status": "pending",
      "dependencies": [5.1]
    },
    {
      "id": 5.4,
      "title": "Implement password reset flow",
      "status": "pending",
      "dependencies": [5.2, 5.3]
    },
    {
      "id": 5.5,
      "title": "Add integration tests",
      "status": "pending",
      "dependencies": [5.2, 5.3, 5.4]
    }
  ]
}
```

## Managing Dependencies

### Dependency Best Practices

✅ **DO:**
- Only add dependencies that are true blockers
- Keep dependency chains as short as possible
- Document why dependencies exist
- Check for circular dependencies before adding

❌ **DON'T:**
- Create circular dependencies (A depends on B, B depends on A)
- Add unnecessary dependencies "just in case"
- Create long chains that delay work unnecessarily

### Dependency Visualization

Use these indicators when viewing tasks:
- ✅ Dependency completed (status: "done")
- ⏱️ Dependency pending (status: "pending" or "in_progress")
- 🔶 Dependency deferred (status: "deferred")

### Checking Dependencies

Before starting a task, verify:
```
Task 7 depends on: [3, 5, 6]
  → Task 3: ✅ done
  → Task 5: ✅ done  
  → Task 6: ⏱️ in_progress
  
Status: NOT READY (Task 6 must complete first)
```

## Task Prioritization Guidelines

### High Priority
- Blocks other tasks from starting
- Critical bug fixes
- Security vulnerabilities
- Core functionality required for release
- Has many dependent tasks waiting

### Medium Priority
- Important features or improvements
- Affects user experience significantly
- Should be done soon but not blocking
- Moderate number of dependent tasks

### Low Priority
- Nice-to-have features
- Minor optimizations
- Technical debt that can wait
- Future enhancements
- No dependent tasks

## Task Status Lifecycle

```
pending → in_progress → done
                ↓
            deferred (if postponed)
```

### Status Definitions

- **pending**: Task is defined, ready to start when dependencies are met
- **in_progress**: Someone is actively working on this task
- **done**: Task is complete, verified, and tested
- **deferred**: Task is postponed for a future iteration (with reason documented)

## Best Practices for Task Creation

### 1. Be Specific and Actionable
❌ Bad: "Improve performance"
✅ Good: "Optimize database queries in user dashboard to load under 200ms"

### 2. Include All Necessary Context
```json
{
  "details": "Implement rate limiting middleware:\n- Use Redis for distributed rate limiting\n- Limit: 100 requests per minute per IP\n- Return 429 status when limit exceeded\n- Add X-RateLimit headers to all responses\n- File: src/middleware/rate_limiter.rs"
}
```

### 3. Define Clear Success Criteria
```json
{
  "testStrategy": "Success when:\n- Rate limiter blocks requests after 100/min\n- Correct HTTP headers are returned\n- Redis connection handles failures gracefully\n- All unit tests pass\n- Load test with 1000 concurrent requests succeeds"
}
```

### 4. Keep Dependencies Minimal
❌ Bad: Task 10 depends on [1, 2, 3, 4, 5, 6, 7, 8, 9]
✅ Good: Task 10 depends on [7, 9] (only true blockers)

### 5. Break Down Large Tasks
- If you can't complete it in 1-2 sessions, break it down
- Each subtask should be a clear, focused unit of work
- Subtasks should have their own dependencies

### 6. Update Regularly
- Change status as work progresses
- Document blockers immediately
- Update test strategy if requirements change
- Add notes about implementation decisions

## Task Validation Checklist

Before adding a task, verify:

- [ ] Unique ID assigned
- [ ] Clear, action-oriented title
- [ ] Concise description (what and why)
- [ ] Appropriate status set
- [ ] Dependencies listed (or empty array)
- [ ] Priority assigned (high/medium/low)
- [ ] Detailed implementation instructions
- [ ] Test strategy defined
- [ ] No circular dependencies
- [ ] Subtasks broken down (if complex)

---

# PART 2: TASK RUNNER - Executing Tasks

## Task Execution Overview

The task runner ensures tasks are completed in the correct order, properly documented, and thoroughly tested.

## Pre-Execution: Reading Tasks

Before starting work:

### 1. Load the Task List
```
Load: .zed/task_list.json
Parse: JSON structure
Validate: No corrupted data
```

### 2. Filter Available Tasks
```
Find all tasks where:
  - status = "pending"
  - all dependencies have status = "done"
```

### 3. Sort by Priority
```
Order: high → medium → low
Within same priority: by ID (oldest first)
```

### 4. Select Next Task
```
Choose: Highest priority task with satisfied dependencies
Verify: No blockers exist
```

## Task Execution Workflow

### Phase 1: Preparation

**1. Select the Task**
- Choose highest priority task with all dependencies complete
- Verify task status is "pending"
- Review all dependencies are actually complete

**2. Update Status to "in_progress"**
```json
{
  "id": 7,
  "status": "in_progress"  // Changed from "pending"
}
```

**3. Review Task Details**
- Read title, description, and details thoroughly
- Understand the test strategy
- Identify any unclear requirements
- Check for subtasks

### Phase 2: Execution

**For Tasks WITHOUT Subtasks:**

1. Read and understand the full task details
2. Implement according to specifications
3. Document any deviations or decisions
4. Apply the test strategy
5. Verify completion criteria met

**For Tasks WITH Subtasks:**

1. Execute each subtask in dependency order
2. Mark each subtask "done" when complete
3. Document results for each subtask
4. Only mark parent task "done" when ALL subtasks complete

**Implementation Guidelines:**

- Follow the details section exactly
- If something is unclear, document the interpretation
- Keep notes of challenges encountered
- Track time spent (optional but helpful)
- Test incrementally as you go

### Phase 3: Documentation

**Required Documentation (save in `.zed/` directory):**

1. **Implementation Summary**
   - What was implemented
   - How it was implemented
   - Any deviations from the plan

2. **Challenges and Solutions**
   - Problems encountered
   - How they were resolved
   - Lessons learned

3. **Configuration Changes**
   - Files modified
   - Settings changed
   - Dependencies added

4. **Test Results**
   - Test strategy applied
   - Results of testing
   - Any issues found and fixed

5. **Follow-up Items**
   - Remaining issues
   - Future improvements
   - New tasks to create

### Phase 4: Completion

**1. Apply Test Strategy**
- Execute all tests defined in testStrategy
- Verify all success criteria met
- Fix any issues found
- Re-test until all tests pass

**2. Update Task Status to "done"**
```json
{
  "id": 7,
  "status": "done"
}
```

**3. Check Dependent Tasks**
- Find all tasks that depend on this task
- Check if any are now unblocked
- Notify about newly available tasks

**4. Update CHANGELOG.md**
- Add entry for completed task
- Include task ID and title
- Summarize what was accomplished
- Note any significant decisions

## Handling Different Task Scenarios

### Scenario 1: Task is Blocked

```
Task 10 ready to start
Dependencies: [7, 8, 9]
  → Task 7: ✅ done
  → Task 8: ⏱️ in_progress
  → Task 9: ✅ done

Action: Cannot start Task 10. Wait for Task 8 to complete.
```

**Options:**
1. Work on Task 8 first (if possible)
2. Find another unblocked task
3. Consider breaking the dependency if not critical

### Scenario 2: Task Requirements Unclear

**DO:**
1. Document the unclear requirement
2. Make a reasonable interpretation
3. Proceed with implementation
4. Note the assumption in documentation
5. Flag for review

**DON'T:**
- Guess wildly without documentation
- Skip the requirement entirely
- Mark task as "done" if truly blocked

### Scenario 3: Task Needs Modification

**If requirements change during execution:**

1. Document the reason for change
2. Update task details in `task_list.json`
3. Create new subtasks if needed
4. Validate dependencies still correct
5. Update test strategy accordingly
6. Note changes in completion documentation

### Scenario 4: Task Should Be Deferred

**When to defer:**
- External dependency not available
- Requirements have changed
- Higher priority work identified
- Technical blocker cannot be resolved

**How to defer:**
```json
{
  "id": 15,
  "status": "deferred"
}
```

Add note in `.zed/` explaining:
- Why deferred
- What's blocking it
- When it might be reconsidered

## Progress Tracking and Reporting

### Daily Progress Check

**Morning:**
- Review active tasks (status: "in_progress")
- Check dependencies for pending tasks
- Identify highest priority available task
- Plan the day's work

**Evening:**
- Update status of all worked-on tasks
- Document progress and blockers
- Update CHANGELOG.md if tasks completed
- Note any new tasks discovered

### Progress Metrics

Track these metrics:
- **Total tasks**: Count all tasks
- **Completed**: Count status = "done"
- **In progress**: Count status = "in_progress"
- **Pending**: Count status = "pending"
- **Deferred**: Count status = "deferred"
- **Completion rate**: done / total

### Blocker Management

**When blocked:**

1. **Identify the blocker**
   - Missing information?
   - Technical limitation?
   - External dependency?
   - Resource unavailable?

2. **Document it**
   - Create note in `.zed/blockers.md`
   - Include task ID, date, description
   - Add possible solutions

3. **Take action**
   - Try to resolve if possible
   - Escalate if needed
   - Work on other tasks meanwhile
   - Consider deferring if long-term block

4. **Follow up**
   - Check blockers regularly
   - Update when resolved
   - Resume task when unblocked

## Best Practices for Task Execution

### ✅ DO

1. **Follow the dependency chain**
   - Never skip dependencies
   - Complete tasks in proper order
   - Verify dependencies are truly done

2. **Update status promptly**
   - Change to "in_progress" when starting
   - Change to "done" when complete
   - Keep task_list.json current

3. **Document thoroughly**
   - Write clear implementation notes
   - Record all decisions made
   - Note challenges and solutions
   - Save documentation in `.zed/`

4. **Verify completion**
   - Apply the test strategy fully
   - Don't skip testing
   - Re-test after fixes
   - Get review if needed

5. **Communicate blockers**
   - Report issues immediately
   - Document in detail
   - Propose solutions
   - Don't let blockers sit silently

6. **Respect priorities**
   - High priority first
   - Don't cherry-pick easy tasks
   - Focus on unblocking others
   - Balance urgency and importance

7. **Close the loop**
   - Update CHANGELOG.md
   - Notify affected parties
   - Update related documentation
   - Check for newly unblocked tasks

### ❌ DON'T

1. **Don't skip dependencies**
   - Even if you think you can
   - Dependencies exist for a reason
   - Skipping causes integration issues

2. **Don't leave status stale**
   - Update immediately when starting
   - Update immediately when done
   - Don't let tasks sit in wrong status

3. **Don't skip documentation**
   - Future you will thank you
   - Others need to understand your work
   - Documentation is part of the task

4. **Don't skip testing**
   - Untested code is broken code
   - Test strategy exists for a reason
   - Verify before marking done

5. **Don't work on multiple tasks simultaneously**
   - Finish one before starting another
   - Context switching is expensive
   - One task "in_progress" at a time

6. **Don't defer without reason**
   - Document why it's deferred
   - Set follow-up date
   - Don't use as excuse to skip hard tasks

## Project Completion Checklist

### When All Tasks Are Complete

1. **Final Verification**
   - [ ] All tasks have status = "done" or "deferred"
   - [ ] All deferred tasks have documented reasons
   - [ ] No tasks stuck in "in_progress"
   - [ ] All documentation is complete

2. **Documentation Review**
   - [ ] CHANGELOG.md fully updated
   - [ ] All `.zed/` notes organized
   - [ ] README updated if needed
   - [ ] API docs updated if needed

3. **Testing Review**
   - [ ] All test strategies executed
   - [ ] Integration tests pass
   - [ ] No known bugs remaining
   - [ ] Performance acceptable

4. **Completion Report**
   Create a summary in `.zed/completion_report.md`:
   - Total tasks completed
   - Total time spent (if tracked)
   - Major accomplishments
   - Outstanding issues (if any)
   - Future improvements identified
   - Lessons learned

5. **Future Planning**
   - [ ] Review deferred tasks
   - [ ] Create new task list if needed
   - [ ] Archive completed task_list.json
   - [ ] Plan next iteration

---

## Quick Reference Commands

### Task Status Updates
```bash
# Mark task as in progress
Update task_list.json: {"id": 5, "status": "in_progress"}

# Mark task as done
Update task_list.json: {"id": 5, "status": "done"}

# Defer a task
Update task_list.json: {"id": 5, "status": "deferred"}
```

### Finding Next Task
```bash
# Filter: status = "pending" AND all dependencies = "done"
# Sort: by priority (high → medium → low)
# Select: first task in sorted list
```

### Documentation
```bash
# Implementation notes
Save to: .zed/task_<id>_notes.md

# Blockers
Save to: .zed/blockers.md

# Completion summary
Save to: .zed/task_<id>_complete.md
```

---

## When to Activate This Skill

Use this skill when:
- Creating new tasks for the project
- Planning a feature or milestone
- Breaking down complex work into manageable pieces
- Deciding which task to work on next
- Updating task status or documentation
- Managing task dependencies
- Reviewing project progress
- Handling blockers or changes
- Completing tasks and verifying results

## Integration with Development Workflow

This skill integrates with:
- **Zed Editor**: Tasks stored in `.zed/task_list.json`
- **Version Control**: CHANGELOG.md tracks completed work
- **Documentation**: Notes saved in `.zed/` directory
- **Project Management**: Structured task tracking system
- **Quality Assurance**: Test strategies ensure completeness

---

## Example Task Templates

### Simple Task Template
```json
{
  "id": 1,
  "title": "Add logging to authentication module",
  "description": "Implement structured logging for auth events",
  "status": "pending",
  "dependencies": [],
  "priority": "medium",
  "details": "Add log statements for:\n- Login attempts (success/failure)\n- Registration events\n- Password reset requests\nUse the project's logging framework with appropriate levels.",
  "testStrategy": "Verify logs appear in output with correct format and level. Test each event type.",
  "subtasks": []
}
```

### Complex Task with Subtasks Template
```json
{
  "id": 10,
  "title": "Implement API documentation system",
  "description": "Add automated API documentation generation",
  "status": "pending",
  "dependencies": [8],
  "priority": "high",
  "details": "Set up automated API documentation using OpenAPI/Swagger",
  "testStrategy": "Verify docs generate correctly and all endpoints documented",
  "subtasks": [
    {
      "id": 10.1,
      "title": "Add OpenAPI dependency",
      "status": "pending",
      "dependencies": []
    },
    {
      "id": 10.2,
      "title": "Annotate API endpoints",
      "status": "pending",
      "dependencies": [10.1]
    },
    {
      "id": 10.3,
      "title": "Set up doc generation",
      "status": "pending",
      "dependencies": [10.2]
    },
    {
      "id": 10.4,
      "title": "Deploy docs to website",
      "status": "pending",
      "dependencies": [10.3]
    }
  ]
}
```

---

## Summary

This skill provides comprehensive guidance for:
1. **Creating** well-structured, clear tasks with proper dependencies
2. **Managing** tasks through their lifecycle with proper status tracking
3. **Executing** tasks in the correct order with thorough documentation
4. **Completing** tasks with proper testing and verification
5. **Tracking** progress and handling blockers effectively

**Remember**: Good task management is the foundation of successful project delivery. Clear tasks + proper execution = predictable results!