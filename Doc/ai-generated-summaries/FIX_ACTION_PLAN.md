# Fix Action Plan - Version & Configuration Issues

**Date:** 2025-02-11  
**Current Status:** Two installations detected, old version taking precedence  
**Estimated Fix Time:** 2-5 minutes

---

## Current Situation

### Detected Issues

✗ **Two grok installations exist:**
  - `C:\Users\johnm\.cargo\bin\grok.exe` - Version 0.1.3 (OLD)
  - `C:\Users\johnm\AppData\Local\grok-cli\bin\grok.exe` - Version 0.1.4 (NEW)

✗ **PowerShell is using the old version:**
  - Command: `grok --version` → Shows 0.1.3
  - Should be: Version 0.1.4

✗ **Configuration features not available:**
  - Old version doesn't support hierarchical configuration
  - Project-local `.grok` folders not being detected

---

## Immediate Action Required

### Step 1: Remove Old Version

Choose **ONE** of the following methods:

#### Method A: Automated Cleanup Script (RECOMMENDED)

```powershell
# Navigate to project
cd H:\GitHub\grok-cli

# Run cleanup script
.\scripts\cleanup_old_install.ps1

# Follow the prompts, type 'yes' when asked
# Restart PowerShell after completion
```

**Advantages:**
- Fully automated
- Shows detailed status
- Verifies success
- Safe with confirmations

---

#### Method B: Quick Manual Removal

```powershell
# Delete old version
Remove-Item "$env:USERPROFILE\.cargo\bin\grok.exe" -Force

# Verify removal
Test-Path "$env:USERPROFILE\.cargo\bin\grok.exe"
# Should return: False

# Restart PowerShell
```

**Advantages:**
- Fastest method
- One command

---

#### Method C: Batch File (No PowerShell Issues)

```cmd
cd H:\GitHub\grok-cli
.\scripts\cleanup_old_install.bat
```

**Advantages:**
- Works without PowerShell execution policy
- Simple CMD interface

---

### Step 2: Restart PowerShell

**IMPORTANT:** You must restart PowerShell for PATH changes to take effect.

```powershell
# Close this window and open a new PowerShell session
```

---

### Step 3: Verify Fix

```powershell
# Check which version is now being used
grok --version

# Expected output:
# grok-cli 0.1.4

# Check which executable
(Get-Command grok).Path

# Expected output:
# C:\Users\johnm\AppData\Local\grok-cli\bin\grok.exe
```

---

### Step 4: Verify Configuration (Optional)

```powershell
# Navigate to project
cd H:\GitHub\grok-cli

# Check configuration loading
grok config show

# Look for this line in output:
# INFO grok_cli::config: Using project-local configuration from: "H:\\GitHub\\grok-cli\\.grok\\.env"
```

---

## What If It Doesn't Work?

### Troubleshooting

#### Issue: Old version still showing after restart

**Cause:** PATH cache or multiple terminal windows

**Solution:**
```powershell
# Close ALL PowerShell windows
# Open a fresh one
grok --version
```

#### Issue: "grok: command not found"

**Cause:** PATH not updated

**Solution:**
```powershell
# Use full path temporarily
& "$env:LOCALAPPDATA\grok-cli\bin\grok.exe" --version

# Add to PATH manually (run as Administrator)
[Environment]::SetEnvironmentVariable(
    "Path",
    [Environment]::GetEnvironmentVariable("Path", "User") + ";$env:LOCALAPPDATA\grok-cli\bin",
    "User"
)
```

#### Issue: Configuration still not loading

**Cause:** Wrong directory or missing .grok folder

**Solution:**
```powershell
# Ensure you're in the project directory
cd H:\GitHub\grok-cli

# Check if .grok exists
Test-Path .\.grok

# Create if missing
if (-not (Test-Path .\.grok)) {
    mkdir .grok
    copy .grok\.env.example .grok\.env
}
```

---

## Complete Reset (Last Resort)

If nothing else works, perform a complete reinstall:

```powershell
# 1. Remove everything
Remove-Item "$env:USERPROFILE\.cargo\bin\grok.exe" -ErrorAction SilentlyContinue
Remove-Item "$env:LOCALAPPDATA\grok-cli" -Recurse -Force -ErrorAction SilentlyContinue

# 2. Reinstall
cd H:\GitHub\grok-cli
cargo run --bin installer

# 3. Follow installer prompts
# 4. Restart PowerShell
# 5. Verify: grok --version
```

---

## Prevention for Future

The installer has been updated to automatically detect and remove old Cargo installations.

For future installs:
1. Run `cargo run --bin installer`
2. Installer will detect old versions
3. Choose 'yes' when prompted to remove old version
4. No manual cleanup needed!

---

## Timeline

| Step | Time | Status |
|------|------|--------|
| Run cleanup script | 1 min | Pending |
| Restart PowerShell | 30 sec | Pending |
| Verify version | 30 sec | Pending |
| Test configuration | 1 min | Pending |
| **Total** | **~3 min** | |

---

## Success Criteria

✓ `grok --version` shows 0.1.4  
✓ `(Get-Command grok).Path` shows AppData\Local\grok-cli\bin\grok.exe  
✓ `grok config show` shows project-local configuration when in project directory  
✓ Only one grok.exe found when running `where.exe grok`

---

## Documentation References

- **Quick Fix:** [QUICK_FIX.md](QUICK_FIX.md) - One-page reference
- **Detailed Troubleshooting:** [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Comprehensive guide
- **Technical Details:** [VERSION_CONFIG_FIX_SUMMARY.md](VERSION_CONFIG_FIX_SUMMARY.md) - Complete analysis

---

## Next Steps After Fix

Once the fix is complete:

1. **Test basic functionality:**
   ```powershell
   grok chat "Hello, test message"
   ```

2. **Test project configuration:**
   ```powershell
   cd H:\GitHub\grok-cli
   grok config show
   ```

3. **Explore new features in 0.1.4:**
   - Hierarchical configuration
   - Project-local settings
   - Enhanced context discovery
   - Improved error messages

---

## Need Help?

If you encounter any issues not covered here:

1. Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for comprehensive solutions
2. Run with debug output:
   ```powershell
   $env:RUST_LOG = "debug"
   grok config show
   ```
3. Report issue with:
   - Output of `grok --version`
   - Output of `where.exe grok`
   - Output of `grok config show`

---

## Summary

**Problem:** Old version (0.1.3) taking precedence over new version (0.1.4)  
**Solution:** Remove old version from `~\.cargo\bin\grok.exe`  
**Method:** Run cleanup script or manual removal  
**Time:** 2-5 minutes  
**Status:** Ready to fix

**Action Now:** Choose a method from Step 1 above and execute!

---

**Created:** 2025-02-11  
**Version:** 0.1.4  
**Status:** Ready for User Action