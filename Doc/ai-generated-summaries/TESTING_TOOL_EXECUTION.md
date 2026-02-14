# Testing Tool Execution in Interactive Mode

This document provides a quick test to verify that tool execution works correctly in the interactive terminal/PowerShell mode.

## Quick Test

### 1. Create Test Directory
```bash
mkdir test-tool-execution
cd test-tool-execution
```

### 2. Start Grok Interactive Mode
```bash
grok
```

### 3. Run Test Command
Once in interactive mode, type:
```
You: Create a test file called hello.txt with the content "Hello from Grok CLI tool execution!"
```

### Expected Result (WORKING)
You should see:
```
Grok is executing operations...
  ✓ Successfully wrote to hello.txt
All operations completed!
```

### Previous Behavior (BROKEN)
If tools are NOT working, you'll see:
```
🤖 Grok:

I'll help you create that file! Here's what you need to do:

1. Create a file called hello.txt
2. Add this content: "Hello from Grok CLI tool execution!"

You can use this command:
echo "Hello from Grok CLI tool execution!" > hello.txt
```

## Comprehensive Test Suite

### Test 1: File Creation
```
You: Create a file named test1.txt with "Test 1 passed"
```

**Expected**: File created, confirmation shown
**Verify**: `cat test1.txt` or `type test1.txt`

### Test 2: Directory Creation via Write
```
You: Create a file src/main.rs with a hello world Rust program
```

**Expected**: Directory `src/` created, file created
**Verify**: `ls src/` or `dir src/`

### Test 3: Read File
```
You: Read the contents of test1.txt
```

**Expected**: Displays file contents and byte count
**Verify**: Should show "Read X bytes from test1.txt"

### Test 4: List Directory
```
You: List all files in the current directory
```

**Expected**: Shows directory contents
**Verify**: Should see test1.txt, src/, etc.

### Test 5: Multiple Files at Once
```
You: Create a Rust project structure with:
- Cargo.toml
- src/main.rs
- README.md
```

**Expected**: All three files created
**Verify**: `ls` shows all files

### Test 6: Replace Text
```
You: Replace "Test 1" with "Test One" in test1.txt
```

**Expected**: Replacement confirmation
**Verify**: `cat test1.txt` should show "Test One passed"

## Troubleshooting

### Problem: Still Getting Instructions Instead of Execution

**Symptoms**:
- Grok responds with "Here's what you need to do..."
- No "✓ Successfully wrote to..." messages
- Files are not created

**Solutions**:

1. **Check Version**:
   ```bash
   grok --version
   ```
   Should be v0.1.2 or later

2. **Rebuild**:
   ```bash
   cd grok-cli
   cargo build --release
   ```

3. **Verify Binary**:
   Make sure you're running the correct binary:
   ```bash
   which grok  # Linux/Mac
   where grok  # Windows
   ```

4. **Check Logs**:
   Look for errors in the output when starting grok

### Problem: Permission Denied

**Cause**: Trying to access directories outside current directory

**Solution**: 
- Make sure you're in a directory where you want files created
- Don't use absolute paths or `../` 
- Stay within your project directory

### Problem: Tool Calls Not Parsed

**Check**: Make sure these imports are in `src/display/interactive.rs`:
```rust
use crate::acp::security::SecurityPolicy;
use crate::acp::tools;
```

## Debug Mode

To see what's happening behind the scenes:

1. Set debug logging:
   ```bash
   export RUST_LOG=debug  # Linux/Mac
   set RUST_LOG=debug     # Windows CMD
   $env:RUST_LOG="debug"  # Windows PowerShell
   ```

2. Run grok and look for:
   - "Tool definitions included: true"
   - "Tool calls received: X"
   - "Executing tool: write_file"

## Success Criteria

Tool execution is working correctly when:

✅ Files are created automatically when requested
✅ "✓ Successfully wrote to..." messages appear
✅ No need to manually run commands or create files
✅ Multiple files can be created in one request
✅ Directory structures are created automatically
✅ Read/list operations work

## Cleanup

After testing:
```bash
cd ..
rm -rf test-tool-execution  # Linux/Mac
rmdir /s test-tool-execution  # Windows
```

## Reporting Issues

If tool execution is not working:

1. Note your exact command
2. Copy the full output
3. Run with debug logging
4. Check the version: `grok --version`
5. Open an issue with all this information

## Architecture Notes

### Where Tool Execution Happens

- **Interactive Mode**: `src/display/interactive.rs` → `send_to_grok()` function
- **Chat Mode**: `src/cli/commands/chat.rs` → `handle_interactive_chat()` function
- **Single Query**: `src/cli/commands/chat.rs` → `handle_single_chat()` function

### Key Functions

1. `tools::get_tool_definitions()` - Returns tool schemas for API
2. `execute_tool_call_interactive()` - Executes a single tool
3. `SecurityPolicy::new()` - Sets up security validation

### Tool Flow

```
User Input
    ↓
send_to_grok()
    ↓
Include tool definitions in API request
    ↓
Receive response with tool_calls
    ↓
For each tool_call:
    - Parse function name and arguments
    - Validate security
    - Execute operation
    - Display result
    ↓
Continue conversation
```

## Version History

- **v0.1.1 and earlier**: Tool execution only in ACP/Zed mode
- **v0.1.2**: Tool execution added to interactive terminal mode
- **v0.1.2+**: This test guide created

---

**Status**: Tool execution should work in all modes (interactive, chat, query)
**Last Updated**: 2026-01-24
**Tested On**: Windows 11, PowerShell