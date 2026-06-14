## Safety Hooks Data Flow (Tasks 154–160)

```
Tool Call (write_file / replace)
    │
    ▼
WriteContext { path, operation, proposed_content, diff, session_dna }
    │
    ▼
on_before_write_file()
    ├── Binary junk detection
    ├── Massive overwrite guard (>200k)
    ├── JSON validity check
    ├── SessionDNA failure pattern check
    └── High-risk operation confirmation
    │
    ▼
SuspiciousWriteGuard::check()
    ├── Empty overwrite rejection
    ├── 10× size explosion rejection
    ├── Binary junk heuristic
    └── Format-specific parse validation (JSON/TOML/YAML)
    │
    ▼
DryRunContext::should_simulate()
    ├── dry_run = true  → return diff only, no write
    └── dry_run = false → proceed to disk
    │
    ▼
DiffValidator::validate_edit()
    ├── Full rewrite >200 lines → reject
    └── >40% content removal → reject
    │
    ▼
IntentValidator::validate_intent()
    └── Ambiguous request → request clarification
    │
    ▼
DnaSafetyController::should_enter_safe_mode()
    └── Repeated failures/hallucinations → force dry-run + confirmation
    │
    ▼
ToolHealthMonitor
    └── Record success/failure → potentially disable unhealthy tool
    │
    ▼
Actual File Operation (or dry-run response)
```

All safety modules live in `src/safety/` and are re-exported from `crate::safety`.
