# Multi-Slot /replace Memory System — Formal Specification

**Task:** 450  
**Status:** Specification Complete (implementation pending)  
**Priority:** High  
**Version:** 0.1.0  
**Date:** 2025

---

## 1. Motivation & Goals

Current context management in Grok-CLI is monolithic. The LLM receives large, undifferentiated blocks of context, leading to:

- Rapid token bloat
- Loss of structure over long-horizon tasks
- Poor ability for the model to deliberately manage its own working state
- Difficult promotion of stable knowledge to long-term stores (OKF)

**Vision:** Give the agent **explicit, typed, addressable memory slots** that it can read from and write to using the existing `/replace` mechanism (or a lightweight variant).

This provides:
- Semantic separation of concerns (plan vs. working memory vs. errors)
- Hard token budgets per slot
- Deterministic compaction and eviction policies
- Clean promotion path to OKF (Open Knowledge Format) bundles
- Better observability and debugging of agent state

---

## 2. Core Concepts

### 2.1 Memory Slot

A **MemorySlot** is a named, bounded container for agent state.

```rust
pub struct MemorySlot {
    pub name: String,                    // e.g. "plan", "working", "mem.0"
    pub content: String,
    pub max_tokens: usize,
    pub last_updated: DateTime<Utc>,
    pub slot_type: SlotType,
    pub metadata: SlotMetadata,          // optional: importance, tags, etc.
}

pub enum SlotType {
    Plan,           // High-level goals and current plan
    Working,        // Current task state, scratchpad
    Context,        // Retrieved facts, code snippets, etc.
    Errors,         // Recent failures, diagnostics
    Indexed(u32),   // mem.0, mem.1, mem.2, ...
    Custom(String), // future extension
}
```

### 2.2 Named vs Indexed Slots

- **Named slots** (primary semantic roles):
  - `plan`
  - `working`
  - `context`
  - `errors`

- **Indexed slots** (rolling short-term memory):
  - `mem.0`, `mem.1`, `mem.2`, ... (configurable count, default 4–8)

Indexed slots act as a ring buffer / FIFO for recent observations.

### 2.3 Access Syntax (LLM-facing)

The LLM interacts with slots using the existing `/replace` tool with bracket notation:

```
/replace[plan]
/replace[working]
/replace[errors]
/replace[mem.0]
/replace[mem.3]
```

The prompt builder injects the current content of requested slots (or all by default) with clear delimiters.

---

## 3. Naming Rules & Conventions

| Slot Name     | Type          | Purpose                              | Default Max Tokens | Eviction Policy     |
|---------------|---------------|--------------------------------------|--------------------|---------------------|
| `plan`        | Named         | Current high-level plan & goals      | 800                | Never auto-evict    |
| `working`     | Named         | Active scratchpad / task state       | 1200               | Compact first       |
| `context`     | Named         | Retrieved knowledge & facts          | 1500               | Compact + promote   |
| `errors`      | Named         | Recent errors and diagnostics        | 600                | Rotate on overflow  |
| `mem.N`       | Indexed       | Rolling recent memory                | 400 each           | FIFO eviction       |

**Rules:**
- Slot names are lowercase, alphanumeric + dot for indexed (`mem.0`).
- The system **must** reject unknown slot names (except during very early bootstrap).
- `plan` and `working` should be treated as higher priority for preservation.

---

## 4. MemoryManager (Core Component)

```rust
pub struct MemoryManager {
    slots: HashMap<String, MemorySlot>,
    config: MemoryConfig,
}

impl MemoryManager {
    pub fn get_slot(&self, name: &str) -> Option<&MemorySlot>;
    pub fn update_slot(&mut self, name: &str, content: String) -> Result<()>;
    pub fn compact_all(&mut self) -> Result<()>;
    pub fn promote_all(&mut self) -> Result<Vec<OkfConcept>>;  // returns promoted items
    pub fn evict_oldest(&mut self, target_reduction_tokens: usize) -> Result<()>;
    pub fn serialize_for_prompt(&self, slot_names: &[&str]) -> String;
    pub fn total_tokens(&self) -> usize;
}
```

**Key Behaviors:**
- `update_slot` enforces `max_tokens`. If exceeded, it should either reject or immediately trigger compaction for that slot.
- Automatic compaction should be triggered before prompt construction when `total_tokens > threshold`.
- All mutations are timestamped.

---

## 5. Compaction Algorithm (Deterministic)

**Requirements:**
- Must be fully deterministic (same input → same output).
- Must be Rust-pure (no external LLM calls for core compaction in v1).
- Target: 50–80% token reduction while preserving meaning.

**Proposed Strategy (v1):**

1. **Priority order for compaction** (lowest first):
   - `errors` (oldest entries)
   - Indexed `mem.*` slots (oldest first)
   - `context`
   - `working` (more conservative)

2. **Techniques** (applied in order):
   - Remove duplicate / near-duplicate lines
   - Strip verbose logging / stack traces (keep summary)
   - Bulletize long paragraphs
   - Extract stable facts → candidate for OKF promotion
   - Replace repeated code blocks with references ("see mem.2")
   - Summarize using simple heuristics + optional small local model (future)

3. **Hard rules**:
   - Never drop content from `plan` unless explicitly instructed by user/LLM.
   - Always keep the most recent 3 entries in `errors`.

A `compact_slot(slot, target_tokens)` function must exist for fine-grained control.

---

## 6. OKF Promotion Pipeline

**Goal:** Move stable, high-value information out of short-term slots into structured OKF bundles.

**Trigger conditions:**
- Content survives N compaction cycles
- High repetition across slots or iterations
- Explicit LLM request (`promote this fact`)
- Content matches known high-value patterns (via OKF lookup or heuristics)

**Process:**
1. `MemoryManager::promote_all()` or per-slot `promote_to_okf()`
2. Extract candidate facts (title + body)
3. Call `okf_create(...)` (local + optional remote)
4. On success: replace the promoted content in the slot with a short reference:
   ```
   [Promoted to OKF: knowledge/architecture/decisions/2025-xxx]
   ```

**Safety:**
- Promotion is always reviewable (logged).
- LLM can be instructed to request promotion explicitly.

---

## 7. Prompt Injection Format

The prompt builder must produce clean, labeled sections:

```markdown
## Memory Slots

### [plan]
...content...

### [working]
...content...

### [context]
...content...

### [errors]
...content...

### [mem.0] ... [mem.3]
...
```

**LLM Instructions (to be added in task 457):**
- Explain each slot's purpose
- Instruct the model to use `/replace[slot]` to update
- Instruct when to compact vs. promote
- Provide examples

Token budgeting logic must be present so the total injected memory never exceeds a configured budget.

---

## 8. Integration Points

| Component              | Responsibility                              | Dependency |
|------------------------|---------------------------------------------|------------|
| `MemoryManager`        | Core state + compaction + promotion         | New        |
| Agent Loop / Outer Loop| Load, update after LLM/tool calls, compact  | 454        |
| Prompt Builder         | Serialize slots + inject with labels        | 455        |
| `/replace` tool        | Primary write path (extended syntax)        | Existing   |
| System Prompt          | Usage rules + examples                      | 457        |
| OKF layer              | Promotion target                            | Existing   |
| HOH / Evolution        | Can observe and optimize memory behavior    | Future     |

**Update Rules (agent loop):**
- After every LLM response → update `working` with new reasoning
- After tool results → update relevant slot (`context`, `errors`, or `working`)
- Before building prompt → run `compact_all()` if over budget
- On task completion or major milestone → attempt promotion

---

## 9. Configuration

Add to `config.toml` (under new `[memory]` section or extend existing):

```toml
[memory]
enabled = true
max_total_tokens = 6000
slots = [
  { name = "plan",    max_tokens = 800,  type = "plan" },
  { name = "working", max_tokens = 1200, type = "working" },
  ...
]
indexed_slots = 6
compaction_threshold = 4500
auto_promote = true
```

---

## 10. Internal JSON Schema (for persistence / serialization)

```json
{
  "version": "1",
  "slots": {
    "plan": {
      "content": "...",
      "max_tokens": 800,
      "last_updated": "2025-...",
      "slot_type": "plan"
    },
    "mem.0": { ... }
  },
  "metadata": {
    "total_tokens": 3120,
    "last_compaction": "..."
  }
}
```

The `MemoryManager` should be able to round-trip this cleanly.

---

## 11. Non-Goals (v1)

- Full LLM-powered summarization inside compaction (use heuristics first)
- Cross-session persistent memory (beyond OKF promotion)
- Automatic slot creation by the LLM (only predefined + indexed)
- Encryption or access control on slots

---

## 12. Success Criteria (for later tasks)

- LLM can reliably update specific slots using the documented syntax.
- Context size remains bounded even across 20+ step tasks.
- Important facts are promoted to OKF without manual intervention.
- The system is observable (`/context` or similar command shows slot state).
- No regression in existing prompt quality or tool behavior.

---

## 13. Open Questions (to be resolved during 450 → 451)

1. Should `/replace[slot]` be a new dedicated tool or reuse/extend the existing `replace` tool?
2. Exact token counting strategy (tiktoken? rough char*4?).
3. How much history should indexed `mem.*` slots retain vs. compact?
4. Should `plan` be allowed to grow or strictly capped?
5. Promotion should create OKF concepts of what type by default? (`Decision`, `Pattern`, `Fact`, etc.)

---

**End of Specification v0.1.0**

This document is the authoritative source for tasks 451–459. All implementation must conform to the naming, structure, and rules defined here.