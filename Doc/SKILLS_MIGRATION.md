# Skills System Migration Guide

## Overview

This guide helps you transition from the old "all-skills-loaded" behavior to the new progressive disclosure system with on-demand skill activation.

## What Changed?

### Before (v0.1.2 and earlier)

```bash
grok interactive
# ALL skills loaded automatically at startup
# Skills always present in context
# Higher token usage
# No control over which skills are active
```

### After (v0.1.3+)

```bash
grok interactive
# NO skills loaded at startup
# Activate skills on-demand with /activate
# Lower token usage
# Full control over active skills
```

## Breaking Changes

### 1. Skills No Longer Auto-Load

**Old Behavior:**
- All skills in `~/.grok/skills/` loaded automatically
- Instructions added to system prompt at startup
- Always present in every conversation

**New Behavior:**
- Skills discovered but NOT loaded at startup
- Must explicitly activate with `/activate <skill-name>`
- Only active skills included in context

**Migration:**
```bash
# If you want old behavior (all skills active):
grok interactive
> /activate skill-1
> /activate skill-2
> /activate skill-3
> /save default-session
```

### 2. Session Structure Changed

**Impact:** Sessions saved with v0.1.2 are compatible but won't have `active_skills` field

**Old Session Format:**
```json
{
  "session_id": "grok-...",
  "model": "grok-3",
  "conversation_history": [...],
  "system_prompt": "..."
}
```

**New Session Format:**
```json
{
  "session_id": "grok-...",
  "model": "grok-3",
  "conversation_history": [...],
  "system_prompt": "...",
  "active_skills": ["rust-expert"]
}
```

**Migration:** Old sessions load fine, just with no skills active. Activate skills after loading.

## Migration Steps

### Step 1: Identify Your Workflow

**If you never used skills before:**
- ✅ No migration needed
- Continue using Grok as normal
- Explore skills when ready: `/skills`

**If you had skills but rarely used them:**
- ✅ Minimal migration needed
- You'll see lower token usage automatically
- Activate skills when you need them

**If you relied on skills being always active:**
- ⚠️ Action required
- Follow Step 2 to create activation workflow

### Step 2: Create Your Activation Workflow

#### Option A: Activate Per Session

```bash
# Start each session with needed skills
grok interactive

> /activate rust-expert
> /activate cli-design
# Now work as before
```

#### Option B: Save Default Sessions

```bash
# Create session templates for different work
grok interactive

> /activate rust-expert
> /save rust-work

> /reset
> /activate cli-design
> /save cli-work

# Later, load the appropriate one
> /load rust-work  # Rust skills already active
```

#### Option C: Quick Toggle Script

Create a shell alias or script:

```bash
# In ~/.bashrc or ~/.zshrc
alias grok-rust='grok interactive --load rust-work'
alias grok-cli='grok interactive --load cli-work'
```

### Step 3: Update Your Habits

**Old way:**
```bash
grok interactive
# Skills already there, just start asking
> How do I handle errors in Rust?
```

**New way:**
```bash
grok interactive
> /activate rust-expert  # Explicitly activate
> How do I handle errors in Rust?
```

**Or:**
```bash
grok interactive
> /load rust-work  # Load session with skills pre-activated
> How do I handle errors in Rust?
```

## Common Migration Scenarios

### Scenario 1: Developer with Project-Specific Skills

**Before:**
- Had 5-10 skills in `~/.grok/skills/`
- All loaded automatically
- Only used 1-2 per project

**After:**
```bash
# For Rust project
cd ~/projects/my-rust-app
grok interactive
> /activate rust-expert
> /save rust-project

# For CLI project  
cd ~/projects/my-cli-tool
grok interactive
> /activate cli-design
> /save cli-project

# Each project loads with relevant skills only
```

### Scenario 2: General Purpose Usage

**Before:**
- Few skills, used occasionally
- Always loaded "just in case"

**After:**
```bash
# Start clean, activate as needed
grok interactive
> [working normally]

> Actually, I need Rust help now
> /activate rust-expert
> How do I fix this lifetime error?

> /deactivate rust-expert
> [continue other work]
```

### Scenario 3: Heavy Skill User

**Before:**
- Many skills (10+)
- Different combinations for different tasks
- High token usage

**After:**
```bash
# Create named sessions for different task types
grok interactive

# Backend work
> /activate rust-expert
> /activate database-expert
> /activate api-design
> /save backend-work

# Frontend work
> /reset
> /activate react-expert
> /activate css-expert
> /save frontend-work

# DevOps work
> /reset
> /activate docker-expert
> /activate kubernetes-expert
> /save devops-work

# Later: load appropriate session
> /load backend-work  # Relevant skills pre-activated
```

## New Capabilities

### 1. Fine-Grained Control

```bash
> /activate rust-expert
> [Work on Rust code]

> Now I need CLI guidance too
> /activate cli-design

> Done with Rust specifics
> /deactivate rust-expert
> [Continue with just CLI guidance]
```

### 2. Token Optimization

**Savings Example:**
- 3 skills × 3000 tokens each = 9000 tokens always loaded
- New: Only load what you need, when you need it
- Typical savings: 50-80% on skill-related tokens

### 3. Session Clarity

```bash
> /status
Session: my-work
Model: grok-3
Skills: 10 available, 2 active
  Active: rust-expert, cli-design
```

You always know exactly what's active.

## Troubleshooting

### "I preferred the old behavior"

**Solution:** Create a startup session with all skills:

```bash
grok interactive
> /activate skill-1
> /activate skill-2
> /activate skill-3
# ... activate all your skills
> /save all-skills

# Add to shell config:
alias grok='grok interactive --load all-skills'
```

### "Skills disappeared from my sessions"

**Explanation:** Old sessions don't have `active_skills` field, so no skills load automatically.

**Solution:** After loading old session, activate needed skills:

```bash
> /load my-old-session
> /activate rust-expert
> /save my-old-session  # Overwrites with active_skills included
```

### "I keep forgetting to activate skills"

**Tips:**
1. Use `/skills` at start of session to see what's available
2. Create session templates with skills pre-activated
3. Save sessions after activating skills
4. Use `/status` to check what's currently active

### "Performance is slower"

**Check:**
- How many skills are active? `/skills`
- Each active skill adds to context
- Deactivate unused skills: `/deactivate <name>`

**Actually faster:**
- With fewer skills, responses should be FASTER
- Lower token usage = quicker API calls

## Rollback (If Needed)

If you need to temporarily revert to old behavior:

### Option 1: Use Release v0.1.2

```bash
# Check out previous version
git checkout v0.1.2
cargo build --release
```

### Option 2: Modify Code (Not Recommended)

In `src/display/interactive.rs`, around line 165, restore:

```rust
// Load skills context at startup
if let Some(skills_dir) = crate::skills::get_default_skills_dir() {
    if let Ok(skills_context) = crate::skills::get_skills_context(&skills_dir) {
        if !skills_context.is_empty() {
            let ctx = project_context.get_or_insert_with(String::new);
            ctx.push_str(&skills_context);
        }
    }
}
```

## Best Practices Going Forward

### ✅ DO

- **Activate skills at session start** for your current task
- **Save sessions** with your typical skill combinations
- **Deactivate unused skills** to keep context lean
- **Check `/status`** periodically to see what's active
- **Use `/skills`** to discover new skills

### ❌ DON'T

- **Don't activate all skills** unless you need them all
- **Don't forget to save** after setting up your skill set
- **Don't leave skills active** when switching topics
- **Don't assume skills are loaded** - check with `/skills`

## Questions?

### How do I know which skills to activate?

The skill descriptions tell you when to use them:

```bash
> /skills
Available Skills:
  [○] rust-expert - Use when working on Rust projects or Rust-specific questions
  [○] cli-design - Use when building CLI tools or improving command-line UX
```

### Can I have multiple skills active?

Yes! Activate as many as you need:

```bash
> /activate rust-expert
> /activate cli-design
> /activate database-expert
```

### Will old skills still work?

Yes! The skill format hasn't changed, only how they're loaded. All existing skills work exactly the same once activated.

### Do I need to recreate my skills?

No! Existing skill files work as-is. Just activate them with `/activate`.

## Summary

| Aspect | Before | After |
|--------|--------|-------|
| **Loading** | Automatic | On-demand |
| **Control** | None | Full control |
| **Token Usage** | High (all skills) | Low (active only) |
| **Commands** | None | `/skills`, `/activate`, `/deactivate` |
| **Sessions** | Saved | Saved with skill state |

**Bottom Line:** The change gives you more control and better performance. Activate skills when you need them, deactivate when you don't.

## Need Help?

- Check the [Quick Start Guide](SKILLS_QUICK_START.md)
- Review [Example Skills](../examples/skills/)
- See [Full Documentation](SKILL_SPECIFICATION.md)
- Report issues on GitHub