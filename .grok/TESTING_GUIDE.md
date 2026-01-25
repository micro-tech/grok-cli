# Testing Guide: File Access & Zed Integration Fixes

## Quick Start Testing

### Prerequisites
```bash
# Build the latest version
cd H:\GitHub\grok-cli
cargo build --release

# Verify build
target\release\grok.exe --version
```

### Test 1: Basic File Access (Command Line)

```bash
# Navigate to your project
cd H:\GitHub\grok-cli

# Test relative path
grok query "read README.md"

# Test with ./ prefix
grok query "read .\Cargo.toml"

# Test subdirectory
grok query "read src\main.rs"

# Test parent directory (from subdirectory)
cd src
grok query "read ..\README.md"
cd ..
```

**Expected:** All commands should successfully read and display file contents

**If fails:** Check `.grok/.env` has `GROK_API_KEY` set

### Test 2: Configuration Check

```bash
# Verify model configuration
grok config show

# Should show:
# Model: grok-code-fast-1 (or your configured model)
# Configuration: Project (.grok/.env) or Hierarchical
```

**If shows wrong model:**
```bash
# Check .grok/.env exists
type .grok\.env

# Should contain:
# GROK_MODEL=grok-code-fast-1

# If not, create it
echo GROK_MODEL=grok-code-fast-1 > .grok\.env
```

### Test 3: ACP STDIO Mode (Direct)

```bash
# Start ACP in stdio mode with logging
set RUST_LOG=info
grok acp stdio
```

**Send test JSON-RPC requests:**

1. Initialize:
```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"1.0","client_info":{"name":"test","version":"1.0"}}}
```

2. Create session:
```json
{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"workspaceRoot":"H:\\GitHub\\grok-cli"}}
```

**Expected log output:**
```
INFO Adding workspace root to trusted directories: "H:\\GitHub\\grok-cli"
INFO Initialized new ACP session: <session-id>
```

3. Test file read (replace <session-id> with actual ID from response):
```json
{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"<session-id>","prompt":[{"type":"text","text":"read README.md"}]}}
```

**Expected:** Should return file contents in response

Press `Ctrl+C` to stop when done.

### Test 4: Zed Editor Integration

#### A. Configure Zed

Create or edit `%APPDATA%\Zed\settings.json`:

```json
{
  "language_models": {
    "grok": {
      "version": "1",
      "provider": "agent",
      "default_model": "grok-code-fast-1",
      "agent": {
        "command": "H:\\GitHub\\grok-cli\\target\\release\\grok.exe",
        "args": ["acp", "stdio"],
        "env": {
          "GROK_API_KEY": "xai-your-key-here",
          "GROK_MODEL": "grok-code-fast-1"
        }
      }
    }
  }
}
```

**Important:** Replace `xai-your-key-here` with your actual API key!

#### B. Test in Zed

1. Open Zed editor
2. Open a project (File → Open Folder)
3. Press `Ctrl+Shift+A` to open Assistant
4. Select "grok" as the model (if multiple configured)
5. Try these prompts:

```
read README.md
```

```
list files in the current directory
```

```
read src/main.rs and explain what it does
```

**Expected:** Should see file contents and AI responses

#### C. Debug Zed Integration

If it's not working, check logs:

**Option 1: Zed Output Panel**
- View → Output Panel → Assistant
- Look for errors

**Option 2: Run grok-cli with logging manually**

1. Stop Zed
2. Run manually:
```bash
set RUST_LOG=debug
H:\GitHub\grok-cli\target\release\grok.exe acp stdio
```
3. Leave this running
4. Check what messages Zed sends
5. Look for:
   - "Adding workspace root to trusted directories: ..."
   - "Resolved path to: ..."
   - Any errors

### Test 5: Security Verification

Test that security is still working:

```bash
# Should fail - outside workspace
grok query "read C:\Windows\System32\cmd.exe"

# Expected: Error: Access denied: Path is not in a trusted directory
```

```bash
# Should fail - sensitive file
grok query "read C:\Users\<username>\ntuser.dat"

# Expected: Error or Access denied
```

## Common Issues & Solutions

### Issue 1: "API key not configured"

**Solution:**
```bash
# Check API key
grok config get api_key

# If empty, set it:
grok config set api_key xai-your-key-here

# Or use environment variable:
set GROK_API_KEY=xai-your-key-here
```

### Issue 2: "Access denied: Path is not in a trusted directory"

**Cause:** Path resolution issue or file outside workspace

**Solution:**
```bash
# Verify you're in the project directory
cd

# Check the file exists
dir README.md

# Try absolute path
grok query "read H:\GitHub\grok-cli\README.md"

# If that works, it's a relative path issue - check this was fixed:
grok --version  # Should be latest version
```

### Issue 3: Model not changing

**Solution:**
```bash
# Check config priority
grok config show

# Create project .env (higher priority than system)
echo GROK_MODEL=grok-code-fast-1 > .grok\.env

# Verify
grok config show
```

### Issue 4: Zed can't find grok.exe

**Solution:**
```json
{
  "agent": {
    "command": "H:\\GitHub\\grok-cli\\target\\release\\grok.exe"
  }
}
```

Use full path with escaped backslashes!

### Issue 5: Workspace not detected in Zed

**Check logs:**
```bash
# Look for this line when session starts:
INFO Adding workspace root to trusted directories: ...
```

**If missing:**
- Zed may not be sending workspaceRoot
- Try setting environment variable in Zed config:
```json
{
  "env": {
    "WORKSPACE_ROOT": "${workspaceFolder}"
  }
}
```

## Test Results Checklist

Use this to verify all fixes are working:

### File Access (Direct CLI)
- [ ] Read file with relative path (`read README.md`)
- [ ] Read file with ./ prefix (`read ./Cargo.toml`)
- [ ] Read file in subdirectory (`read src/main.rs`)
- [ ] Read file with parent dir (`read ../file.txt`)
- [ ] Security blocks outside files (`read C:\Windows\...`)

### Configuration
- [ ] Config shows correct model (`grok config show`)
- [ ] Project .env overrides system config
- [ ] API key is set

### ACP STDIO
- [ ] Initialize request succeeds
- [ ] Session/new with workspaceRoot logs "Adding workspace root..."
- [ ] Session/prompt with file operations works

### Zed Integration
- [ ] Zed launches grok-cli successfully
- [ ] Assistant panel opens
- [ ] File operations return results
- [ ] AI responses appear in Zed

## Performance Tests

```bash
# Test response time
Measure-Command { grok query "read README.md" }

# Should be: < 5 seconds for normal connections
# May be: 10-30 seconds with Starlink optimizations
```

## Debugging Commands

```bash
# Full debug output
set RUST_LOG=debug
grok acp stdio

# Test network
grok test-network --timeout 10

# Health check
grok health --all

# Validate config
grok config validate
```

## Success Criteria

✅ All file operations work with relative paths
✅ Zed integration extracts workspace context
✅ Security still blocks unauthorized access
✅ All tests pass
✅ Response times acceptable

## Next Steps After Testing

1. **If all tests pass:** Ready for production use!

2. **If file access works but Zed doesn't show results:**
   - Issue is in response/notification flow
   - Check `session/prompt` response format
   - Debug tool result → SessionUpdate → Zed flow

3. **If workspace context missing in Zed:**
   - Verify Zed sends workspaceRoot in session/new
   - Try environment variable fallback
   - Check Zed version compatibility

## Getting Help

**Enable debug logging:**
```bash
set RUST_LOG=debug
grok acp stdio > debug.log 2>&1
```

**Check documentation:**
- `docs/ZED_INTEGRATION.md` - Zed setup
- `.grok/FILE_ACCESS_FIX_SUMMARY.md` - Technical details
- `.grok/ZED_WORKSPACE_ISSUE.md` - Workspace context
- `.grok/QUICK_REFERENCE.md` - Quick commands

**Report issues:**
- Include debug.log
- Zed version
- grok-cli version (`grok --version`)
- Steps to reproduce

## Test Script (PowerShell)

Save this as `test-grok.ps1`:

```powershell
# Test grok-cli fixes
Write-Host "Testing grok-cli file access and Zed integration..." -ForegroundColor Cyan

# Test 1: Version
Write-Host "`n[Test 1] Version check" -ForegroundColor Yellow
& .\target\release\grok.exe --version

# Test 2: Config
Write-Host "`n[Test 2] Configuration" -ForegroundColor Yellow
& .\target\release\grok.exe config show

# Test 3: File read
Write-Host "`n[Test 3] File read (README.md)" -ForegroundColor Yellow
& .\target\release\grok.exe query "read README.md"

Write-Host "`n✅ All tests completed!" -ForegroundColor Green
```

Run with: `.\test-grok.ps1`

---

**Testing completed!** If all tests pass, the fixes are working correctly.