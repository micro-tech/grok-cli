# Grok CLI Data Flow Map and Execution Path Visualization

## Overview

This document provides a comprehensive data flow map and visualization of the code execution paths in the Grok CLI application. The Grok CLI is a command-line interface for interacting with Grok AI, featuring chat capabilities, code operations, and various tools.

## Architecture Overview

The application follows a layered architecture:

1. **Entry Point**: `main.rs` - Logging setup and dispatch to CLI app
2. **CLI Layer**: `cli/app.rs` - Argument parsing and command routing
3. **Command Handlers**: `cli/commands/` - Specific command implementations
4. **Core Services**: API client, configuration, security
5. **Tools**: File operations, web access, shell commands

## Data Flow Map

### High-Level Data Flow

```
User Input
    ↓
CLI Arguments Parsing
    ↓
Configuration Loading
    ↓
API Key Resolution
    ↓
Command Dispatch
    ↓
Command Handler Execution
    ↓
API Communication (if needed)
    ↓
Tool Execution (if triggered)
    ↓
Result Processing & Display
    ↓
User Output
```

### Detailed Data Flow Components

#### 1. Input Processing Flow

```
CLI Args → Clap Parser → Cli Struct
    ↓
Config Path Resolution → Config::load_hierarchical()
    ↓
Environment Variables → API Key Resolution
    ↓
Model Selection (CLI override → config → default)
```

#### 2. Command Dispatch Flow

```
Parsed Command → Match on Commands enum
    ↓
Branch to Handler:
├── Chat → handle_chat()
├── Code → handle_code_action()
├── ACP → handle_acp_action()
├── Interactive → start_interactive_mode()
├── Query → handle_chat() (single message)
├── Config → handle_config_action()
├── Settings → handle_settings_action()
├── History → handle_history_action()
├── Audit → handle_audit_action()
├── Health → handle_health_check()
├── Skills → handle_skills_command()
└── Setup → handle_setup()
```

#### 3. Chat Command Data Flow

```
ChatOptions → initialize_client()
    ↓
Message Preparation (system + user messages)
    ↓
Tool Definitions Retrieval
    ↓
API Call: chat_completion_with_history()
    ↓
Response Processing
    ├── Text Response → Display
    └── Tool Calls → Tool Execution Loop
        ↓
        Security Validation → Tool Dispatch
        ↓
        Tool Result → Display
```

#### 4. Tool Execution Data Flow

```
ToolCall → Function Name + Arguments
    ↓
Security Policy Validation
    ↓
Path Access Control (for file operations)
    ↓
Tool-Specific Processing:
├── read_file → File Read → Content Return
├── write_file → File Write → Success Message
├── replace → Text Replacement → Update Count
├── list_directory → Directory Scan → File List
├── glob_search → Pattern Matching → File Paths
├── run_shell_command → Command Execution → Output
├── web_search → DuckDuckGo Query → Results
├── web_fetch → HTTP Request → Content
├── save_memory → Memory File Append → Confirmation
└── search_file_content → Regex Search → Matches
```

#### 5. Interactive Mode Data Flow

```
Interactive Config → Conversation History Init
    ↓
Input Loop:
    ├── User Input → Command Check
    │   ├── Special Commands (exit, help, etc.) → Handle
    │   └── Normal Input → Bayesian Router (if enabled)
    │       ↓
    │       Intent Classification → Skill/Tool Suggestion
    │       ↓
    │       Modified Input + System Prompts
    ↓
Message History Update
    ↓
API Call with Tools
    ↓
Response Processing → Tool Execution (if any)
    ↓
Display Response → Loop Continuation
```

## Execution Path Visualizations

### Main Execution Path

```
┌─────────────────┐
│     main()      │
│  setup_logging()│
└─────────┬───────┘
          │
          ▼
┌─────────────────┐
│ cli::app::run() │
│  Clap parsing   │
└─────────┬───────┘
          │
          ▼
┌─────────────────┐
│Config Loading   │
│API Key Resolve  │
└─────────┬───────┘
          │
          ▼
┌─────────────────┐    ┌─────────────────┐
│Command Dispatch │───▶│ Default:        │
│Based on CLI args│    │ Interactive Mode│
└─────────┬───────┘    └─────────────────┘
          │
          ▼
   Command Handlers
   (Chat, Code, etc.)
```

### Chat Execution Path

```
┌─────────────────┐
│ handle_chat()   │
└─────────┬───────┘
          │
          ▼
┌─────────────────┐
│Client Initialize│
│(API key, timeout│
│ retries, rate   │
│ limits)         │
└─────────┬───────┘
          │
          ▼
    ┌─────┴─────┐
    │ Interactive? │
    └─────┬─────┘
          │
    ┌─────┴─────┐
    │   Yes     │◄─────────────────┐
    └─────┬─────┘                  │
          │                        │
          ▼                        │
┌─────────────────┐                │
│Interactive Loop │                │
│- Input reading  │                │
│- History mgmt   │                │
│- API calls      │                │
│- Tool execution │                │
└─────────┬───────┘                │
          │                        │
          ▼                        │
    ┌─────┴─────┐                  │
    │    No     │                  │
    └─────┬─────┘                  │
          │                        │
          ▼                        │
┌─────────────────┐                │
│Single Message   │                │
│Processing       │                │
└─────────┬───────┘                │
          │                        │
          │                        │
          └────────────────────────┘
                   ▲
                   │
            Tool Calls?
                   │
                   ▼
          ┌─────────────────┐
          │Tool Execution   │
          │Security checks  │
          │Actual tool ops  │
          └─────────────────┘
```

### Tool Execution Path

```
┌─────────────────┐
│  Tool Call      │
│  (JSON args)    │
└─────────┬───────┘
          │
          ▼
┌─────────────────┐
│Security Policy  │
│Path validation  │
│Access control   │
└─────────┬───────┘
          │
          ▼
   Tool-Specific Path
   ┌─────┬─────┬─────┬─────┐
   │File │Shell│ Web │Memory│
   │Ops  │Cmd  │ Ops │Ops  │
   └─────┴─────┴─────┴─────┘
          │
          ▼
┌─────────────────┐
│Result Processing│
│Output formatting│
└─────────────────┘
```

### Data Structures Flow

```
Cli Struct
├── api_key: Option<String>
├── config: Option<PathBuf>
├── hide_banner: bool
├── model: Option<String>
└── command: Option<Commands>

Commands Enum
├── Chat { message, interactive, system, temperature, max_tokens }
├── Code { action: CodeAction }
├── Acp { action: AcpAction }
├── Interactive
├── Query { prompt }
├── Config { action: ConfigAction }
├── Settings { action: SettingsAction }
├── History { action: HistoryAction }
├── Audit { action: AuditAction }
├── Health { api, config, all }
├── Skills { action: SkillsCommand }
└── Setup

ChatOptions
├── message: Vec<String>
├── interactive: bool
├── system: Option<String>
├── temperature: f32
├── max_tokens: u32
├── api_key: &str
├── model: &str
├── timeout_secs: u64
├── max_retries: u32
├── rate_limit_config: RateLimitConfig
└── bayesian: BayesianConfig

Message Flow
User Input → Vec<String> → JSON Messages → API → Response → Tool Calls → Results
```

## Key Data Transformations

### 1. Input to API Messages

```
User String → JSON Message Object
{
    "role": "user",
    "content": "user input"
}

System Prompt → JSON Message Object
{
    "role": "system", 
    "content": "system prompt"
}
```

### 2. API Response to Display

```
API Response → MessageWithFinishReason
    ├── content: MessageContent::Text(text)
    └── tool_calls: Vec<ToolCall>

→ extract_text_content() → String
→ format_grok_response() → Colored Output
```

### 3. Tool Call Execution

```
ToolCall
├── function: FunctionCall
│   ├── name: "read_file"
│   └── arguments: JSON String
└── id: String

→ serde_json::from_str() → Value
→ match name → Tool Function
→ Security Check → Execution
→ Result String
```

## Security Data Flow

```
External Access Request
    ↓
Path Resolution → Canonical Path
    ↓
Security Policy Check
    ├── Trusted Directory?
    ├── External Access Enabled?
    └── Approval Required?
        ↓
    User Prompt (if needed)
        ↓
    Access Granted/Denied
        ↓
    Audit Logging
        ↓
    Operation Execution
```

## Configuration Data Flow

```
Config Loading Hierarchy:
1. Explicit --config path
2. Project config (./.grok/config.toml)
3. System config (~/.grok/config.toml)  
4. Default config

→ Config Struct
├── api_key_source: ConfigSource
├── default_model: String
├── timeout_secs: u64
├── max_retries: u32
├── rate_limits: RateLimitConfig
├── telemetry: TelemetryConfig
├── bayesian: BayesianConfig
└── external_access: ExternalAccessConfig
```

## Error Handling Flow

```
Operation Result
    ↓
Match on Result<T, E>
├── Ok(value) → Continue Processing
└── Err(error) → Error Handling
    ├── Logging (warn/error levels)
    ├── User Display (print_error!)
    └── Process Exit (if critical)
```

## Performance Considerations

### Data Flow Bottlenecks

1. **API Calls**: Network latency, rate limits
2. **File Operations**: Disk I/O, large files
3. **Tool Execution**: Security checks, external commands
4. **Web Operations**: Network timeouts, content size limits

### Optimization Points

1. **Caching**: Tool definitions, config loading
2. **Async Processing**: Non-blocking I/O operations  
3. **Streaming**: Large response handling
4. **Connection Pooling**: API client reuse

## Testing Data Flow

```
Unit Tests → Mock Data → Function Calls → Assertions
Integration Tests → Real Config → API Calls → Result Validation
E2E Tests → CLI Invocation → Full Pipeline → Output Verification
```

This data flow map provides a comprehensive view of how data moves through the Grok CLI system, from user input to final output, including all the processing steps, security checks, and error handling paths.

---

## Bayesian Belief Stabilization (Decay Step)

The Bayesian engine now includes a **decay / stabilization step** after every update to prevent extreme probability collapse.

### Problem
Without stabilization, repeated updates could drive one intent to 98–99% while crushing others to near-zero, making the router overly decisive and brittle.

### Solution
After the likelihood multiplication and floor, a decay pass is applied:

```rust
// --- DECAY STEP ---
for (intent, belief_value) in priors.iter_mut() {
    let prior = likelihoods.get(intent).copied().unwrap_or(0.0);
    *belief_value = *belief_value * decay_rate + prior * pull_rate;
}
```

- `decay_rate` (default `0.95`) — how much of the current belief is retained
- `pull_rate` (default `0.05`) — how strongly beliefs are pulled toward their long-term priors

### Configuration

```toml
[bayesian]
belief_decay_rate = 0.92   # stronger stabilization
prior_pull_rate   = 0.08
```

Higher `pull_rate` values produce more conservative, stable distributions. Lower values allow faster adaptation at the cost of occasional extreme spikes.

This mechanism is applied uniformly in:
- `update_from_text()`
- `update_from_model_confidence()`
- `update_from_tool_failure()`

---

## Reasoning Protocol Layer (RPL) Data Flow

The RPL is a **passive observability layer** that wraps the `CpuRouter::route_with_tools_traced()` call. It captures structured `ReasoningTrace` objects without influencing the control flow.

### RPL Lifecycle

```
CpuRouter::route_with_tools_traced()
    │
    ├─ RplLayer::on_pre_evaluate(goal, context)
    │      └─► Creates ReasoningTrace { phase: PreEvaluation, suppressed: true }
    │
    ├─ [Tool Execution Loop - per iteration]
    │      └─ RplLayer::on_tool_selection(trace, tool_name, selected, reason)
    │             └─► Appends ToolEvaluation { tool_name, relevance_score, selected }
    │                 Advances phase → ToolSelection
    │
    └─ RplLayer::on_complete(trace)
           ├─► Advances phase → Complete
           ├─► Calls validate(trace) — collects all ValidationErrors
           └─► Calls log_trace(trace, config.log_level)
```

### RPL Data Structures

```
ReasoningTrace
├── schema_version: u32          (= 1)
├── trace_id: String             (UUID v4 — correlation ID)
├── goal: Option<String>         (inferred from last user message)
├── context: Option<String>      (active skills, session summary)
├── tool_evaluations: Vec<ToolEvaluation>
│   └── { tool_name, relevance_score [0,1], reason, selected }
├── memory_considerations: Vec<MemoryConsideration>
│   └── { memory_key, relevance_score [0,1], summary }
├── plan: Option<String>         (planned action sequence)
├── uncertainty: f32             ([0,1] — 0=confident, 1=max uncertainty)
├── created_at: DateTime<Utc>
├── phase: ReasoningPhase        (PreEvaluation|ToolSelection|MemoryLookup|ActionPlanning|Complete)
└── suppressed: bool             (true by default — safe production default)
```

### Suppression Gate

```
ReasoningTrace (suppressed=true by default)
    │
    ▼
SuppressionLayer::guard(&trace)
    ├── suppressed=true  + production mode  → None  (trace blocked)
    ├── suppressed=true  + debug_mode=true  → Some  (debug override)
    └── suppressed=false + any mode         → Some  (explicitly exposed)
    │
    ▼
SuppressionLayer::redact(&trace)     (applied before any exposure)
    ├── RedactionConfig::apply_all(goal)
    ├── RedactionConfig::apply_all(context)
    ├── RedactionConfig::apply_all(plan)
    ├── RedactionConfig::apply_all(tool_evaluation.reason)
    └── RedactionConfig::apply_all(memory_consideration.summary)
    trace_id is NEVER redacted (needed for log correlation)
```

### RPL Log Levels

```
ReasoningLogLevel::Off     → no log events emitted
ReasoningLogLevel::Summary → tracing::info!  { trace_id, phase, uncertainty }
ReasoningLogLevel::Debug   → tracing::debug! { above + goal, plan, tool_count }
ReasoningLogLevel::Trace   → tracing::trace! { full JSON-serialised trace }
```

---

## Reasoning Engine Data Flow

The Reasoning Engine is an **active decision-making component** that runs alongside the CPU tool loop. Unlike the RPL (which only observes), the engine shapes what the CPU does.

### Engine Lifecycle per Turn

```
User Prompt
    │
    ▼
ReasoningEngineState::new()              phase: AnalyzeGoal
    │
    ├─ EngineBeliefs::update_from_evidence(UserText(prompt))
    │      └─► BayesianEngine::update_from_text(prompt)
    │          Adjusts hypothesis confidence and uncertainty score
    │
    ├─ state.transition(ExpandOptions)   phase: ExpandOptions
    │      └─► Add/update Hypothesis entries
    │
    ├─ state.transition(EvaluateOptions) phase: EvaluateOptions
    │      └─► EngineBeliefs::sync_to_state(&mut state)
    │          Sets state.uncertainty, updates hypothesis confidences
    │
    ├─ state.transition(CommitPlan)      phase: CommitPlan
    │      └─► PlanBuilder::build_plan(goal, available_tools)
    │          Returns Vec<PlanStep> with UseTool/QueryMemory/ModelCall/NoOp steps
    │
    ├─ [For each PlanStep]
    │      ├─ state.transition(ExecuteStep { step_index })
    │      │
    │      ├─ MemoryBridge::relevant_facts(&mut state, &long_term_memory)
    │      │      └─► Queries LongTermMemory by goal keywords
    │      │          Records IDs in state.memory_references
    │      │
    │      ├─ ArbitrationEngine::rank_tools(&state.plan, &rpl_trace)
    │      │      └─► Scores tools by: plan match (0.6 weight) + RPL trace (0.4 weight)
    │      │          Returns Vec<RankedTool> sorted descending by score
    │      │
    │      ├─ ArbitrationEngine::select_tool(&ranked, state.uncertainty)
    │      │      ├─ uncertainty < 0.7  → highest-score tool
    │      │      └─ uncertainty ≥ 0.7  → cheapest-cost tool (fallback)
    │      │
    │      └─ [Tool executes via CpuRouter]
    │             ├─ Success → state.mark_step_complete(result)
    │             └─ Failure → state.mark_step_failed(reason)
    │                    └─► CorrectionEngine::should_correct(state)
    │                        └─► Trigger? → apply_correction(state, trigger)
    │                                        Calls state.revise_plan(recovery_steps)
    │
    └─ state.transition(Complete)        phase: Complete
           ├─ MemoryBridge::should_write_memory(&state) → bool
           └─ EngineObserver::log_state_transition(engine_id, CommitPlan, Complete, uncertainty)
```

### Engine State Machine

```
         ┌───────────────┐
    ──►  │  AnalyzeGoal  │
         └───────┬───────┘
                 │
         ┌───────▼───────┐
         │ ExpandOptions │
         └───────┬───────┘
                 │
         ┌───────▼───────┐
         │EvaluateOptions│
         └───────┬───────┘
                 │
         ┌───────▼───────┐
         │  CommitPlan   │◄────────────┐
         └───────┬───────┘             │
                 │                     │
         ┌───────▼───────┐    ┌────────┴──────┐
         │ ExecuteStep(n)│───►│  RevisePlan   │
         └───────┬───────┘    └───────────────┘
         │       │
     ┌───▼───┐ ┌─▼──────┐
     │Complete│ │Failed  │   (terminal — no further transitions)
     └────────┘ └────────┘
```

### Self-Correction Loop

```
CorrectionEngine::should_correct(state)
    ├── state == Complete or Failed → None   (terminal, no correction)
    ├── plan is empty + goal set   → EmptyPlan trigger
    ├── any step Failed            → StepFailed { index, reason } trigger
    └── uncertainty > 0.75         → HighUncertainty trigger

CorrectionEngine::apply_correction(state, trigger)
    ├── StepFailed  → keep completed steps + ModelCall("Recover: {reason}") + pending steps
    ├── HighUncertainty → prepend ModelCall("Re-evaluate due to high uncertainty")
    ├── EmptyPlan   → single ModelCall("No plan; re-analyse goal: {goal}")
    └── ExternalFeedback → prepend ModelCall("User feedback: {msg}")
    All call state.revise_plan(recovery) — MaxRevisionsExceeded → stop

CorrectionEngine::correct_until_stable(state, max_rounds=10)
    Bounded loop: stops when should_correct → None OR MaxRevisionsReached
    (Double safeguard: max_rounds cap + state.max_revisions cap)
```

### Engine Data Structures

```
ReasoningEngineState
├── schema_version: u32            (= 1)
├── engine_id: String              (UUID v4 — links to RPL trace_id)
├── state: EngineState             (FSM variant)
├── goal: Option<String>           (inferred user intent)
├── hypotheses: Vec<Hypothesis>    ({ id, description, confidence: f32 })
├── plan: Vec<PlanStep>
│   └── { step_id, description, action: StepAction, status: StepStatus, result }
│       StepAction: UseTool{tool_name,args} | QueryMemory{query} | ModelCall{prompt} | NoOp
│       StepStatus: Pending | InProgress | Completed | Failed{reason} | Skipped
├── current_step_index: usize
├── selected_tools: Vec<String>
├── memory_references: Vec<String>
├── uncertainty: f32               ([0,1])
├── revision_count: u32
├── max_revisions: u32             (default 3 — self-correction safeguard)
└── created_at / updated_at: DateTime<Utc>
```

### Engine Observability Data Flow

```
EngineObserver::log_state_transition(engine_id, from, to, uncertainty)
    ├── Off     → nothing
    ├── Summary → tracing::info!  { engine_id, from, to }
    ├── Debug   → tracing::debug! { above + uncertainty }
    └── Trace   → tracing::trace! { above + JSON state }
    All fields pass through RedactionConfig::apply_all() before emission

EngineObserver::log_correction(engine_id, trigger, revision_count)
    └── Any active level → tracing::warn! (corrections always warrant warning)

is_safe_to_log(state) — returns false if goal or step descriptions contain
    patterns matching: api_key, secret, password (default RedactionConfig rules)
redact_state(state, redaction) — returns a clone with sensitive fields redacted
```

---

## Module Dependency Map (Updated)

```
src/main.rs
    └── src/lib.rs
            ├── src/cli/           (commands, argument parsing)
            ├── src/acp/           (Zed ACP protocol)
            ├── src/router/
            │       ├── CpuRouter::route_with_tools_traced()
            │       └── ── calls ──► src/rpl/ (RplLayer hooks)
            ├── src/rpl/           ◄── NEW (Tasks 86-92)
            │       ├── schema.rs  (ReasoningTrace)
            │       ├── layer.rs   (lifecycle hooks)
            │       ├── logging.rs (log_trace)
            │       ├── suppression.rs (SuppressionLayer)
            │       └── validation.rs
            ├── src/engine/        ◄── NEW (Tasks 93-101)
            │       ├── state.rs   (ReasoningEngineState FSM)
            │       ├── beliefs.rs (EngineBeliefs → src/bayes/)
            │       ├── planner.rs (PlanBuilder)
            │       ├── memory_bridge.rs (→ src/memory/)
            │       ├── arbitration.rs (→ src/skills/)
            │       ├── correction.rs (CorrectionEngine)
            │       └── observability.rs (EngineObserver)
            ├── src/bayes/         (BayesianEngine)
            ├── src/memory/        (MemoryStore, 4 tiers)
            ├── src/skills/        (AutoActivationEngine + RPL-aware scoring)
            └── src/tools/         (execute_tool registry)
```
