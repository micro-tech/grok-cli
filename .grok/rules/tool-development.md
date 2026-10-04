# Tool Development Process (Strict)

The tool system is the heart of the agent. Changes here have high blast radius.

**You MUST follow this process exactly:**

1. Add (or update) the **complete** schema entry in `src/tools/registry.rs` → `get_full_tool_definitions()`.
2. Implement the thin handler function: `handle_xxx(args: &Value, ctx: &ToolContext) -> Result<String>`.
3. Add **exactly one line** in the `execute_tool` match arm.
4. Use only the `require_*` and `optional_*` helpers for argument extraction.
5. **In the same change**, ensure the drift test `every_tool_schema_has_corresponding_handler` still passes (or add/update test cases as needed).
6. Add or update round-trip / helper tests in `registry.rs` (or the relevant `*_tools.rs` file) that would have caught the exact mistake you are fixing.
7. Verify with `cargo test` and `cargo clippy`.

**Never**:
- Add a schema without the handler
- Add a handler without updating the schema list
- Weaken the `unknown` arm
- Leave the drift test in a failing state

The `unknown` arm must remain a hard guard.

## Language Idiomatic Layouts (Required)

When generating or editing code, **always use the standard, conventional project layout for that language**.

- Do not create non-standard or "creative" directory structures.
- Match how mature, high-quality projects in that language are organized.

**Rust (primary language for this project):**
- Follow the standard Cargo layout:
  - `src/main.rs` (for binaries) or `src/lib.rs` (for libraries)
  - `src/` for internal modules (prefer `mod.rs` + subdirectory or `module.rs` style)
  - `src/bin/` for multiple binaries
  - `tests/` for integration tests
  - `examples/` for runnable examples
  - `benches/` for benchmarks
- Use proper module declarations (`mod`, `pub mod`, `pub use`).
- Keep `Cargo.toml` clean with clear sections.

**Other languages:**
- Python: standard `src/` + `tests/`, proper package structure (`__init__.py`, etc.)
- Go: standard `cmd/`, `internal/`, `pkg/`
- TypeScript/JavaScript: conventional `src/`, `tests/`, `dist/`
- Follow the language community's widely accepted conventions.

When working across languages in the same repo, respect each language's idiomatic structure.

## File Tools & Context

- Most file operations should go through `ToolContext` + security policy.
- Prefer using the registered tools (`read_file`, `write_file`, `replace`, etc.) even for internal operations when appropriate.
- Paths outside the working directory require explicit approval per the security model.

## Testing

- Tool changes **must** have tests in `registry.rs` (round-trips, drift guards, helper tests).
- Task tools have extensive tests in `task_tools.rs`.
- Run `cargo test + cargo clippy` before considering any work done.
- Prefer tests that would have caught the exact class of bug being fixed.

## Configuration

- System config: `~/.grok-cli/config.toml`
- Project config: `.grok/config.toml`
- Hierarchical loading lives in `src/config/`.
- The old monolithic `config/mod.rs` has been split into focused modules.