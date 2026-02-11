# Quick Fix Reference Card

## Version Mismatch Issue

### Symptoms
- `grok --version` shows old version (e.g., 0.1.3) after installing new version
- Different versions from `cargo run --bin grok` vs `grok` command

### Quick Fix (Choose One)

#### Option 1: Automated Script (Recommended)
```powershell
cd H:\GitHub\grok-cli
.\scripts\cleanup_old_install.ps1
```
Restart PowerShell after completion.

#### Option 2: One-Liner
```powershell
Remove-Item "$env:USERPROFILE\.cargo\bin\grok.exe" -Force; Write-Host "Restart PowerShell to use new version" -ForegroundColor Green
```

#### Option 3: Batch File
```cmd
.\scripts\cleanup_old_install.bat
```

### Verify Fix
```powershell
grok --version
# Should show: grok-cli 0.1.4
```

---

## Configuration Not Loading Issue

### Symptoms
- Project `.grok` folder not being used
- Always using system configuration from `C:\Users\<name>\.grok`

### Fix
1. **Ensure you're running the correct version** (see above)
2. **Navigate to project directory:**
   ```powershell
   cd H:\GitHub\grok-cli
   ```
3. **Verify configuration loading:**
   ```powershell
   grok config show
   ```
   Should show: `"Using project-local configuration from: H:\GitHub\grok-cli\.grok\.env"`

### If Still Not Working
```powershell
# Check if .grok exists
Test-Path .\.grok

# Create if missing
mkdir .grok
copy .grok\.env.example .grok\.env
```

---

## Emergency Reset

### Complete Clean Slate
```powershell
# Remove both installations
Remove-Item "$env:USERPROFILE\.cargo\bin\grok.exe" -ErrorAction SilentlyContinue
Remove-Item "$env:LOCALAPPDATA\grok-cli" -Recurse -Force -ErrorAction SilentlyContinue

# Reinstall
cd H:\GitHub\grok-cli
cargo run --bin installer

# Restart PowerShell
```

---

## Diagnostic Commands

```powershell
# Which executable is being used?
(Get-Command grok).Path

# What version?
grok --version

# Which config is loaded?
grok config show | Select-String "Using"

# All grok executables on system
where.exe grok
```

---

## Expected Results

### Version Check
```
C:\Users\johnm\AppData\Local\grok-cli\bin\grok.exe
grok-cli 0.1.4
```

### Config Check (from project dir)
```
INFO grok_cli::config: Using project-local configuration from: "H:\\GitHub\\grok-cli\\.grok\\.env"
```

---

## Need More Help?

See **[TROUBLESHOOTING.md](TROUBLESHOOTING.md)** for comprehensive guide.

---

**Print this page and keep it handy!**