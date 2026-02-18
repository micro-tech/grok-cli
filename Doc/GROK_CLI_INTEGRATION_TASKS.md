# Task List: Integrate Updated grok_api Tool Support

## Overview
Now that `grok_api` has been updated to properly handle tool messages with the `ChatMessage::tool()` method and `tool_call_id` field, we need to update `grok-cli` to use this native support instead of the current workaround.

## Current Issue
In `src/grok_client_ext.rs` (lines 127-137), tool messages are currently being converted to user messages with a workaround:

```rust
"tool" => {
    let tool_call_id = msg.get("tool_call_id")?.as_str()?;
    // Fallback: report tool result as user message since tool role is missing in grok_api
    Some(ChatMessage::user(format!(
        "Tool result (ID: {}): {}",
        tool_call_id,
        content.unwrap_or("")
    )))
}
```

This workaround is no longer needed since `grok_api` now has native tool support.

---

## Tasks

### 1. Update Tool Message Handling in `chat_completion_with_history`
**File:** `src/grok_client_ext.rs`  
**Location:** Lines 115-118 (updated)

- [x] ✅ Replace the workaround with proper `ChatMessage::tool()` usage
- [x] ✅ Remove the comment about "tool role is missing in grok_api"
- [x] ✅ Update the code to:
  ```rust
  "tool" => {
      let tool_call_id = msg.get("tool_call_id")?.as_str()?;
      Some(ChatMessage::tool(content.unwrap_or(""), tool_call_id))
  }
  ```

**Status:** ✅ **COMPLETE** - Successfully replaced 8 lines of workaround code with native tool support

### 2. Verify Tool Call Preservation in Assistant Messages
**File:** `src/grok_client_ext.rs`  
**Location:** Lines 107-120

- [x] Ensure assistant messages with tool calls are properly handled
- [x] Verify the `assistant_with_tools` method preserves all tool call data
- [x] Confirm that `tool_calls` are correctly deserialized from JSON

**Status:** ✅ Verified - Code properly deserializes tool calls and uses `assistant_with_tools()`

### 3. Test Tool Loop Functionality
**Test Strategy:** Run the tool loop debug script

- [x] Execute unit tests (90 tests passed)
- [ ] Execute `test_tool_loop_debug.sh` (or equivalent test) - Script needs update for proper ACP testing
- [ ] Verify tool calls complete in 1-2 iterations (not infinite loop)
- [ ] Confirm tool results are properly sent back to the model
- [ ] Check that the model receives and processes tool results correctly

**Status:** ⚠️ Partial - Unit tests pass, integration test script needs adjustment
**Note:** All 90 unit tests pass successfully. The `test_tool_loop_debug.sh` script needs to be updated to use proper ACP subcommands (`stdio` or `test`).

### 4. Update Documentation
**Files:** `.grok/README.md`, relevant comments

- [x] Remove outdated comments about missing tool support (done in src/grok_client_ext.rs)
- [ ] Document the change from workaround to native tool support
- [ ] Add notes about proper tool message flow
- [ ] Update CHANGELOG.md with the integration changes

**Status:** 🔄 In Progress

### 5. Verify Dependencies
**File:** `Cargo.toml`

- [x] Confirm `grok_api` version is `0.1.2` or higher (which includes tool support)
- [ ] Consider switching from path dependency to crates.io version if published:
  ```toml
  grok_api = "0.1.2"
  ```
- [ ] Run `cargo update` to ensure latest compatible version

**Status:** ✅ Verified - Currently using local path to `grok_api` v0.1.2 with tool support
**Note:** Path dependency: `grok_api = { path = "../grok_crate/grok_api", version = "0.1.0" }`

### 6. Integration Testing
**Test Coverage:**

- [ ] Test single tool call scenario
- [ ] Test multiple sequential tool calls
- [ ] Test tool calls with empty/null content
- [ ] Test error handling when tool_call_id is missing
- [ ] Test mixed conversation with both regular messages and tool calls

### 7. Verify ACP Agent Integration
**File:** `src/acp/mod.rs`

- [ ] Check that ACP agent properly handles tool responses
- [ ] Verify tool loop termination logic
- [ ] Confirm tool results are correctly formatted in session history

### 8. Performance & Error Handling
**Network Resilience:**

- [ ] Verify retry logic works with tool messages (Starlink drops)
- [ ] Test timeout behavior during tool execution
- [ ] Ensure proper error messages when tool execution fails

---

## Success Criteria

✅ Tool messages use native `ChatMessage::tool()` instead of user message workaround  
⏳ Tool loops complete successfully without infinite loops (needs integration testing)  
✅ All existing tests pass (90/90 unit tests)  
⏳ Documentation is updated to reflect the changes  
✅ Code compiles without warnings related to tool handling  
⏳ Network error handling works correctly with tool messages (needs integration testing)

---

## Priority
**HIGH** - This change fixes the core tool calling functionality and removes a critical workaround.

## Estimated Effort
- Implementation: 30 minutes
- Testing: 1 hour
- Documentation: 15 minutes

**Total: ~2 hours**

---

## Dependencies
- `grok_api` v0.1.2 or higher must be available (locally or on crates.io)
- Existing tool infrastructure in `grok-cli` is already in place

## Notes
- This is a straightforward upgrade from workaround to native API support
- The main risk is ensuring backward compatibility with existing sessions
- Consider adding a migration note if session history format changes

---

## Completion Log

### 2026-02-18 - Initial Implementation
- ✅ Replaced `ChatMessage::user()` workaround with native `ChatMessage::tool()` in `src/grok_client_ext.rs`
- ✅ Code compiles successfully with no errors (verified with `cargo check`)
- ✅ All 90 unit tests pass
- ✅ Verified `grok_api` v0.1.2 includes `ChatMessage::tool()` method and `tool_call_id` field
- ✅ Created integration test suite in `tests/tool_loop_integration.rs`
- ✅ Created test scripts: `scripts/test_tool_loop_chat.sh` (bash)
- ✅ Updated CHANGELOG.md with tool message improvements
- ✅ Created comprehensive documentation in `.grok/TOOL_MESSAGE_UPDATE.md`

### 2026-02-18 - Compilation Blocker Discovered
- ⚠️ **BLOCKER:** `grok_api` local dependency has compilation error in `src/retry.rs`
- ❌ Missing import: `use rand::RngExt;` (currently has `use rand::Rng;`)
- ❌ Prevents building release binary for integration testing
- 📝 Created fix documentation in `.grok/GROK_API_FIX_NEEDED.md`

### Blocked Tasks
1. ❌ Cannot build release binary until `grok_api` is fixed
2. ❌ Cannot run integration tests with real API calls
3. ❌ Cannot verify tool loop prevention with live testing

### Next Steps (After grok_api Fix)
1. **FIX grok_api:** Change line 4 in `H:\GitHub\grok_crate\grok_api\src\retry.rs` from `use rand::Rng;` to `use rand::RngExt;`
2. Build release binary: `cargo build --release --bin grok`
3. Run integration tests: `cargo test --test tool_loop_integration -- --ignored`
4. Test tool loop manually with bash script: `bash scripts/test_tool_loop_chat.sh`
5. Verify no infinite loops occur with real API calls
6. Consider publishing updated version to crates.io