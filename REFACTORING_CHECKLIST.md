# Refactoring Checklist - Library/Binary Separation

**Status:** Phase 1 Complete ✅  
**Branch:** `fix`  
**Date:** 2025-01-XX

## ✅ Completed Work

### 1. Terminal Module Structure
- [x] Created `src/terminal/` directory (binary-only, not in lib.rs)
- [x] Created `src/terminal/mod.rs` with exports and terminal size functions
- [x] Created `src/terminal/display.rs` with print functions (success, error, warning, info)
- [x] Created `src/terminal/input.rs` with user input functions (confirm, prompt)
- [x] Created `src/terminal/progress.rs` with progress bars and spinners
- [x] Added terminal module declaration to `src/main.rs`

### 2. Deprecation Warnings
- [x] Marked `create_spinner()` in `src/cli/mod.rs` as deprecated
- [x] Marked `print_success()` in `src/cli/mod.rs` as deprecated
- [x] Marked `print_error()` in `src/cli/mod.rs` as deprecated
- [x] Marked `print_warning()` in `src/cli/mod.rs` as deprecated
- [x] Marked `print_info()` in `src/cli/mod.rs` as deprecated
- [x] Marked `confirm()` in `src/cli/mod.rs` as deprecated
- [x] Marked `get_terminal_width()` in `src/cli/mod.rs` as deprecated
- [x] Marked `format_table()` (I/O version) in `src/cli/mod.rs` as deprecated
- [x] Marked `clear_screen()` in `src/display/mod.rs` as deprecated
- [x] Marked `print_separator()` in `src/display/mod.rs` as deprecated
- [x] Marked `print_centered()` in `src/display/mod.rs` as deprecated

### 3. Pure Function Alternatives
- [x] Created `format_table_with_width()` - pure function that returns String
- [x] Created `format_separator()` - returns String instead of printing
- [x] Created `format_centered()` - returns String instead of printing
- [x] Created `format_list()` - formats bullet lists without I/O
- [x] Created `format_key_value()` - formats key-value pairs without I/O
- [x] Created `format_key_value_list()` - formats multiple pairs without I/O

### 4. Documentation
- [x] Added comprehensive architecture notes to `src/lib.rs`
- [x] Documented library/binary separation principles
- [x] Documented current violations and migration path
- [x] Created `docs/architecture-refactor.md` with full details
- [x] Added inline comments explaining deprecations
- [x] Created this checklist

### 5. Code Quality
- [x] Fixed missing imports (rand::Rng in tips.rs)
- [x] Fixed YAML indentation errors in `.github/workflows/release.yml`
- [x] Ensured all code compiles successfully
- [x] Verified binary runs correctly (`cargo run --bin grok -- --help`)
- [x] Suppressed 177 deprecation warnings with `#![allow(deprecated)]` in internal modules
- [x] Build produces zero warnings while maintaining deprecation markers for external users

### 6. Exports and Public API
- [x] Re-exported `print_grok_logo` from display module
- [x] Re-exported `clear_current_line` from banner module
- [x] Re-exported `print_welcome_banner` from banner module
- [x] Re-exported `print_directory_recommendation` from banner module
- [x] Made `get_random_tips()` public in tips module
- [x] Added `get_random_tip()` helper function in tips module

### 7. Warning Suppression
- [x] Added `#![allow(deprecated)]` to `src/cli/mod.rs`
- [x] Added `#![allow(deprecated)]` to `src/display/interactive.rs`
- [x] Added `#![allow(deprecated)]` to all command modules:
  - [x] `src/cli/commands/acp.rs`
  - [x] `src/cli/commands/chat.rs`
  - [x] `src/cli/commands/code.rs`
  - [x] `src/cli/commands/config.rs`
  - [x] `src/cli/commands/health.rs`
  - [x] `src/cli/commands/history.rs`
  - [x] `src/cli/commands/settings.rs`
- [x] Added explanatory comments to all suppressions
- [x] Created `docs/warning-suppression.md` documentation

## ⏳ TODO - Phase 2: Command Handler Refactoring

### Command Handlers to Refactor
- [ ] `src/cli/commands/acp.rs` - Remove direct printing, return data structures
- [ ] `src/cli/commands/chat.rs` - Remove direct printing, return data structures
- [ ] `src/cli/commands/code.rs` - Remove direct printing, return data structures
- [ ] `src/cli/commands/config.rs` - Remove direct printing, return data structures
- [ ] `src/cli/commands/health.rs` - Remove direct printing, return data structures
- [ ] `src/cli/commands/history.rs` - Remove direct printing, return data structures
- [ ] `src/cli/commands/settings.rs` - Remove direct printing, return data structures

### Data Structure Design
- [ ] Define `CommandOutput` enum for different output types
- [ ] Create output types for each command category
- [ ] Define traits for command execution
- [ ] Implement serialization for outputs (JSON, TOML, etc.)

### Presentation Layer
- [ ] Create `src/presentation/` module in binary
- [ ] Implement output formatters/presenters
- [ ] Move `src/cli/app.rs` logic to binary
- [ ] Create command dispatcher in binary
- [ ] Wire presenters to terminal module

## ⏳ TODO - Phase 3: Complete Separation

### Source Tree Restructuring
- [ ] Move binary-specific code to `src/bin/grok/`
- [ ] Create `src/bin/grok/main.rs`
- [ ] Create `src/bin/grok/presentation.rs`
- [ ] Create `src/bin/grok/terminal.rs` (move from src/terminal)
- [ ] Update `Cargo.toml` binary configuration
- [ ] Ensure library exports only pure code

### Display Module Cleanup
- [ ] Make `print_banner()` pure (return String)
- [ ] Make `print_welcome_banner()` pure (return String)
- [ ] Make `print_grok_logo()` pure (return String)
- [ ] Make all banner functions pure
- [ ] Make all tip printing functions pure
- [ ] Move ASCII art printing to binary

### Feature Flags
- [ ] Add `cli` feature flag in `Cargo.toml`
- [ ] Make CLI dependencies optional
- [ ] Conditionally compile `Subcommand` derives
- [ ] Document feature flag usage

### Testing
- [ ] Add unit tests for pure functions
- [ ] Add integration tests for I/O operations
- [ ] Add tests for command execution (no I/O)
- [ ] Add tests for presentation layer
- [ ] Update CI/CD pipeline

## 📊 Current Metrics

- **Build Status:** ✅ Success
- **Test Status:** ✅ Pass
- **Deprecation Warnings:** 0 (suppressed internally, active for external users)
- **Build Errors:** 0
- **Files Modified:** 13 (5 core + 8 warning suppressions)
- **Files Created:** 9 (terminal module + 5 docs)
- **Breaking Changes:** 0

## 🚀 How to Use This Checklist

1. **For New Features:**
   - Implement business logic in library (pure functions)
   - Add I/O operations using `src/terminal/` module in binary
   - Return data structures, don't print directly

2. **For Bug Fixes:**
   - Check if function is marked deprecated
   - Consider refactoring while fixing
   - Use pure alternatives where possible

3. **For Code Review:**
   - Verify no new I/O operations in library code
   - Check for use of deprecated functions
   - Ensure new code follows architecture guidelines

## 📝 Notes

- All deprecated functions still work (backwards compatible)
- Deprecation warnings suppressed internally but active for external users
- Terminal module in `src/terminal/` is **NOT** exposed in `src/lib.rs` (binary-only)
- Pure alternatives exist for most formatting operations
- See `docs/architecture-refactor.md` for detailed information
- See `docs/warning-suppression.md` for warning suppression details

## ✅ Acceptance Criteria for Complete Refactoring

Phase 2 Complete When:
- [ ] All command handlers return data structures
- [ ] No command handlers perform direct I/O
- [ ] Presentation layer exists in binary
- [ ] All deprecated CLI functions removed

Phase 3 Complete When:
- [ ] All library code is pure (no I/O)
- [ ] Binary code isolated in `src/bin/`
- [ ] Library can be used without terminal dependencies
- [ ] Feature flags properly isolate CLI concerns
- [ ] All tests pass without warnings
- [ ] Documentation updated

## 🔗 References

- **Architecture Document:** `docs/architecture-refactor.md`
- **Terminal Module:** `src/terminal/`
- **Library Entry:** `src/lib.rs` (see architecture notes)
- **Main Binary:** `src/main.rs`

---

**Last Updated:** 2025-01-XX  
**Maintained By:** John McConnell (john.microtech@gmail.com)  
**Repository:** https://github.com/microtech/grok-cli