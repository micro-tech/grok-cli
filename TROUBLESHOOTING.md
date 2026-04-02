# Troubleshooting Guide

This guide helps resolve common issues with grok-cli installation and configuration.

## Table of Contents

- [Version Issues](#version-issues)
- [Configuration Issues](#configuration-issues)
- [Network Issues](#network-issues)
- [Installation Issues](#installation-issues)
- [Common Errors](#common-errors)

---

## Version Issues

### Problem: Wrong Version After Installation

**Symptoms:**
- After running the installer and rebooting, `grok --version` shows an old version
- `cargo run --bin grok` shows a different version than `grok` command

**Cause:**
Multiple installations of grok exist on your system, and PowerShell/terminal is finding the wrong one.

**Check Which Version is Running:**

```powershell
# Show which grok executable is being used
(Get-Command grok).Path

# Show version
grok --version
```

**Common Locations:**

1. **Old Cargo installation** (often version 0.1.3 or earlier):
   - `C:\Users\<username>\.cargo\bin\grok.exe`
   - Installed via `cargo install`

2. **New installer version** (current version):
   - `C:\Users\<username>\AppData\Local\grok-cli\bin\grok.exe`
   - Installed via the official installer

**Solution 1: Use the Cleanup Script (Recommended)**

```powershell
# Navigate to the project directory
cd H:\GitHub\grok-cli

# Run the cleanup script
.\scripts\cleanup_old_install.ps1
```

The script will:
- Detect old Cargo installations
- Show version information for both installations
- Offer to remove the old version
- Verify the correct version is being used

**Solution 2: Manual Removal**

```powershell
# Remove old Cargo installation
Remove-Item "$env:USERPROFILE\.cargo\bin\grok.exe" -Force

# Restart PowerShell or refresh environment
refreshenv
# OR close and reopen PowerShell

# Verify correct version
grok --version
```

**Solution 3: Reinstall with Automatic Cleanup**

The installer (version 0.1.4+) now automatically detects and offers to remove old Cargo installations:

```powershell
cd H:\GitHub\grok-cli
cargo run --bin installer
```

Follow the prompts to remove the old version.

**After Cleanup:**

1. Restart your terminal/PowerShell
2. Verify the correct version:
   ```powershell
   grok --version
   # Should show: grok-cli 0.1.4
   ```

---

## Configuration Issues

### Problem: Configuration Not Being Used from Project Directory

**Symptoms:**
- Running `grok` from `H:\GitHub\grok-cli` doesn't use the `.grok` folder in that directory
- Always uses `C:\Users\<username>\.grok` configuration

**Understanding Configuration Loading:**

Grok uses a **hierarchical configuration system** with the following priority (highest to lowest):

1. **Environment variables** (highest priority)
2. **Project-local**: `.grok/.env` in current directory or parent directories
3. **System-level**: `~/.grok/.env` (user home directory)
4. **Built-in defaults** (lowest priority)

**How Project Configuration is Found:**

Grok walks up the directory tree from your current location looking for:
- `.grok/config.toml` or `.grok/.env`
- Project markers: `.git`, `Cargo.toml`, `package.json`, or `.grok` directory

It stops at the first project root it finds.

**Verify Configuration Loading:**

```powershell
# Navigate to your project
cd H:\GitHub\grok-cli

# Run grok with debug output
$env:RUST_LOG="debug"; grok config show

# Look for this line in output:
# INFO grok_cli::config: Using project-local configuration from: "H:\\GitHub\\grok-cli\\.grok\\.env"
```

**Common Issues:**

1. **Wrong Version Running**
   - Old versions (pre-0.1.4) may not support hierarchical configuration
   - **Solution:** Update to version 0.1.4+ (see Version Issues above)

2. **No `.grok` Directory**
   ```powershell
   # Check if .grok exists
   Test-Path H:\GitHub\grok-cli\.grok
   ```
   - **Solution:** Create it if missing:
     ```powershell
     mkdir H:\GitHub\grok-cli\.grok
     cp H:\GitHub\grok-cli\.grok\.env.example H:\GitHub\grok-cli\.grok\.env
     ```

3. **Wrong Current Directory**
   - If you're not in the project directory or a subdirectory, grok won't find the project config
   - **Solution:** `cd` to the project directory first

**Configuration File Locations:**

```
Project Configuration (preferred for development):
H:\GitHub\grok-cli\.grok\.env
H:\GitHub\grok-cli\.grok\config.toml

System Configuration (global fallback):
C:\Users\<username>\.grok\.env
C:\Users\<username>\.grok\config.toml
```

**Best Practices:**

1. Keep API keys and sensitive data in `.env` files (NOT in config.toml)
2. Use project-local configuration for project-specific settings
3. Use system configuration for global defaults
4. Add `.grok/.env` to `.gitignore` (already done in this project)

### Problem: Changes to Configuration Not Taking Effect

**Solutions:**

1. **Verify the correct config file is being edited:**
   ```powershell
   # Show which config is being used
   grok config show
   # Look at the "Configuration Source" section
   ```

2. **Check for environment variable overrides:**
   ```powershell
   # Environment variables override config files
   # Check if variables are set:
   Get-ChildItem Env: | Where-Object { $_.Name -like "*GROK*" -or $_.Name -like "*XAI*" }
   ```

3. **Restart your terminal** after config changes

4. **Check syntax:**
   ```powershell
   # For .env files, use KEY=value format (no spaces around =)
   XAI_API_KEY=your-key-here
   
   # NOT:
   XAI_API_KEY = your-key-here
   ```

---

## Network Issues

### Problem: Network Timeouts or Connection Drops

**Cause:**
The system uses Starlink satellite internet which may experience intermittent drops.

**Built-in Protections:**

Grok-cli includes robust error handling for network issues:
- Automatic retry with exponential backoff
- Timeout detection
- Connection drop recovery

**Configuration:**

Check these settings in your `.env` file:

```env
# Request timeout (milliseconds)
HTTP_TIMEOUT=30000

# Maximum retries for failed requests
MAX_RETRIES=3

# Enable retry backoff
RETRY_BACKOFF=true
```

**Manual Testing:**

```powershell
# Test with shorter timeout
grok chat "test message" --timeout 10000

# Check network settings
grok config show | Select-String -Pattern "timeout|retry"
```

---

## Installation Issues

### Problem: Installer Fails

**Common Causes:**

1. **Cargo build fails:**
   ```powershell
   # Ensure Rust is installed and updated
   rustc --version
   cargo --version
   
   # Update if needed
   rustup update
   ```

2. **Permission errors:**
   - Run PowerShell as Administrator
   - Or choose a different installation directory

3. **File in use:**
   ```powershell
   # Close all grok instances
   Get-Process | Where-Object { $_.Name -like "*grok*" } | Stop-Process
   ```

4. **PATH not updated:**
   ```powershell
   # Check if grok-cli is in PATH
   $env:PATH -split ";" | Select-String "grok-cli"
   
   # If missing, the installer should have added it
   # Try: refreshenv or restart terminal
   ```

### Problem: Command Not Found After Installation

**Solutions:**

1. **Restart your terminal/PowerShell**

2. **Check PATH manually:**
   ```powershell
   $env:PATH -split ";" | Where-Object { $_ -like "*grok*" }
   ```

3. **Add to PATH manually (if needed):**
   ```powershell
   # Run as Administrator
   [Environment]::SetEnvironmentVariable(
       "Path",
       [Environment]::GetEnvironmentVariable("Path", "User") + ";$env:LOCALAPPDATA\grok-cli\bin",
       "User"
   )
   ```

4. **Use full path temporarily:**
   ```powershell
   & "$env:LOCALAPPDATA\grok-cli\bin\grok.exe" --version
   ```

---

## Common Errors

### Error: "Could not determine home directory"

**Cause:** Environment variables for user directory are not set.

**Solution:**
```powershell
# Check home directory
echo $env:USERPROFILE

# If empty, set it:
$env:USERPROFILE = "C:\Users\<your-username>"
```

### Error: "Failed to load config file"

**Causes:**
1. Syntax error in TOML or .env file
2. File permissions issue
3. File doesn't exist

**Solutions:**

1. **Validate TOML syntax:**
   ```powershell
   # Use example as template
   cp .grok\config.example.toml .grok\config.toml
   ```

2. **Check file permissions:**
   ```powershell
   Get-Acl H:\GitHub\grok-cli\.grok\.env
   ```

3. **Use default config:**
   ```powershell
   grok config reset
   ```

### Error: "API key not found"

**Solution:**

1. **Set in project `.env` file:**
   ```env
   XAI_API_KEY=your-key-here
   ```

2. **Or set as environment variable:**
   ```powershell
   $env:XAI_API_KEY = "your-key-here"
   ```

3. **Verify it's loaded:**
   ```powershell
   grok config show
   # Look for "xai_api_key" field (will show as "***" for security)
   ```

### Error: "Chat logger initialization failed"

**Cause:** Cannot create or write to log directory.

**Solution:**

```powershell
# Check if directory exists and is writable
Test-Path $env:USERPROFILE\.grok\logs\chat_sessions

# Create if missing
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.grok\logs\chat_sessions"

# Check permissions
Get-Acl "$env:USERPROFILE\.grok\logs"
```

---

## Getting Help

If you're still experiencing issues:

1. **Enable debug logging:**
   ```powershell
   $env:RUST_LOG = "debug"
   grok [command]
   ```

2. **Check version and configuration:**
   ```powershell
   grok --version
   grok config show
   ```

3. **Review logs:**
   ```powershell
   # View recent logs
   Get-ChildItem "$env:USERPROFILE\.grok\logs\chat_sessions" | Sort-Object LastWriteTime -Descending | Select-Object -First 5
   ```

4. **Report issues:**
   - GitHub: https://github.com/microtech/grok-cli/issues
   - Include: version, error message, steps to reproduce

---

## Quick Reference

### Verify Installation

```powershell
# Check which version is running
(Get-Command grok).Path
grok --version

# Should show:
# C:\Users\<username>\AppData\Local\grok-cli\bin\grok.exe
# grok-cli 0.1.4
```

### Verify Configuration

```powershell
# Check current configuration
cd H:\GitHub\grok-cli
grok config show

# Should show:
# "Using project-local configuration from: H:\GitHub\grok-cli\.grok\.env"
```

### Clean Slate

```powershell
# Remove all versions
Remove-Item "$env:USERPROFILE\.cargo\bin\grok.exe" -ErrorAction SilentlyContinue
Remove-Item "$env:LOCALAPPDATA\grok-cli" -Recurse -Force -ErrorAction SilentlyContinue

# Reinstall
cd H:\GitHub\grok-cli
cargo run --bin installer

# Restart PowerShell
```

---

## Configuration Hierarchy Diagram

```
┌─────────────────────────────────────┐
│   Environment Variables             │ ← Highest Priority
│   (XAI_API_KEY, GROK_MODEL, etc.)   │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│   Project .env                       │
│   H:\GitHub\grok-cli\.grok\.env     │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│   System .env                        │
│   C:\Users\john\.grok\.env          │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│   Built-in Defaults                  │ ← Lowest Priority
└─────────────────────────────────────┘
```

---

## Version History

- **0.1.4**: Added hierarchical configuration, automatic old version detection
- **0.1.3**: Earlier version with basic configuration
- **0.1.0**: Initial release

---

**Last Updated:** 2026-02-11