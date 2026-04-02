# Project Context and Configuration Guide

**Date:** 2025-02-11  
**Version:** 0.1.4+  
**Status:** Implementation Complete

---

## Overview

This guide explains how grok-cli discovers and loads:
1. **Configuration files** (`.env`, `config.toml`) from `.grok/` directories
2. **Context files** (rules, guidelines) for the AI agent
3. **Project root detection** for proper file discovery

---

## Table of Contents

- [Configuration Loading](#configuration-loading)
- [Context File Discovery](#context-file-discovery)
- [Project Root Detection](#project-root-detection)
- [Common Issues](#common-issues)
- [Testing and Verification](#testing-and-verification)

---

## Configuration Loading

### How It Works

Grok uses **hierarchical configuration** with the following priority (highest to lowest):

```
┌─────────────────────────────────────┐
│   Environment Variables             │ ← Highest Priority
│   XAI_API_KEY, GROK_MODEL, etc.     │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│   Project .env                       │
│   .grok/.env in current dir tree    │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│   System .env                        │
│   ~/.grok/.env (home directory)     │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│   Built-in Defaults                  │ ← Lowest Priority
└─────────────────────────────────────┘
```

### Configuration File Discovery

When you run `grok` from **any directory**, it walks **up** the directory tree looking for:

1. `.grok/.env` or `.grok/config.toml`
2. Project markers: `.git/`, `Cargo.toml`, `package.json`, or `.grok/` directory

**Example:**

```
H:\GitHub\grok-cli\           ← Project root (has Cargo.toml)
  ├── .grok\
  │   ├── .env                ← Project config
  │   └── config.toml
  ├── src\
  │   ├── main.rs
  │   └── utils\
  │       └── deep\
  │           └── nested\      ← You run grok here
  └── Cargo.toml
```

**Running from `H:\GitHub\grok-cli\src\utils\deep\nested\`:**
- ✓ Grok walks up to find `.grok/.env` in project root
- ✓ Project config is loaded successfully

### Important Notes

#### ⚠️ Current Working Directory Matters

Configuration is discovered based on where you **run the command**, not where grok is installed.

```powershell
# Example 1: Running from project root
cd H:\GitHub\grok-cli
grok config show
# → Uses H:\GitHub\grok-cli\.grok\.env

# Example 2: Running from subdirectory
cd H:\GitHub\grok-cli\src
grok config show
# → Still uses H:\GitHub\grok-cli\.grok\.env (walks up)

# Example 3: Running from home directory
cd C:\Users\johnm
grok config show
# → Uses C:\Users\johnm\.grok\.env (system config)
```

#### ⚠️ Version Requirement

The hierarchical configuration system requires **version 0.1.4 or later**.

**Check your version:**
```powershell
grok --version
# Should show: grok-cli 0.1.4
```

If you see an older version, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

---

## Context File Discovery

### What Are Context Files?

Context files provide **project-specific rules and guidelines** to the AI agent. They help grok understand:
- Project conventions and coding standards
- Preferred patterns and practices
- Domain-specific knowledge
- Tool configurations and workflows

### Supported Context Files

Grok searches for context files in **priority order**:

| Priority | File Path              | Purpose                    |
|----------|------------------------|----------------------------|
| 1        | `GEMINI.md`            | Google Gemini conventions  |
| 2        | `.gemini.md`           | Hidden Gemini config       |
| 3        | `.claude.md`           | Anthropic Claude rules     |
| 4        | `.zed/rules`           | Zed editor conventions     |
| 5        | `.grok/context.md`     | Grok-specific context      |
| 6        | `.ai/context.md`       | General AI context         |
| 7        | `CONTEXT.md`           | Uppercase context file     |
| 8        | `.gemini/context.md`   | Gemini directory config    |
| 9        | `.cursor/rules`        | Cursor editor rules        |
| 10       | `AI_RULES.md`          | AI rules and guidelines    |
| 11       | `.grok/memory.md`      | Persistent facts/memory    |

**Global Fallback:**
- `~/.grok/context.md`
- `~/.grok/CONTEXT.md`
- `~/.grok/memory.md`

### Context Loading Behavior

#### Single Context Mode (Default)

By default, grok uses the **first context file found** in priority order:

```rust
load_project_context(project_root)
// Returns first file found, or None
```

#### Multi-Context Mode (Merging)

You can load and merge **all available context files**:

```rust
load_and_merge_project_context(project_root)
// Returns merged content from all files
```

**Used in:** Interactive mode (`grok` command without arguments)

### Context Discovery Process

When you start grok in interactive mode:

1. **Detect current directory**: `env::current_dir()`
2. **Search for context files** in current directory (priority order)
3. **Stop at first match** OR **merge all matches** (depending on mode)
4. **Load and format** context for AI prompt
5. **Display loaded files** to user

**Example Output:**

```
✓ Loaded and merged 3 context files
  • .zed/rules
  • .grok/context.md
  • context.md
```

### Important: Current Directory Rules

Context files are discovered in the **directory where you run grok**, not where grok is installed.

```powershell
# Scenario 1: Running from project root
cd H:\GitHub\grok-cli
grok
# → Finds .zed/rules, .grok/context.md, context.md

# Scenario 2: Running from subdirectory
cd H:\GitHub\grok-cli\src
grok
# → Does NOT walk up for context files (only checks src/)
# → No context files found (unless you have src/.zed/rules, etc.)

# Scenario 3: Running from home directory
cd C:\Users\johnm
grok
# → Checks C:\Users\johnm\ for context files
# → Falls back to ~/.grok/context.md if nothing found
```

### ⚠️ Critical Difference: Config vs Context

| Feature              | Configuration (.env)     | Context (rules, guidelines) |
|----------------------|--------------------------|----------------------------|
| **Walks up tree?**   | ✅ Yes                   | ❌ No                      |
| **Current dir only?**| ❌ No                    | ✅ Yes                     |
| **Project root?**    | Searches to root         | Only current directory     |
| **Example**          | .grok/.env               | .zed/rules                 |

**This is the key difference you noticed!**

---

## Project Root Detection

### What Is Project Root?

The **project root** is the top-level directory of your project, typically containing:
- `.git/` directory
- `Cargo.toml` (Rust)
- `package.json` (Node.js)
- `.grok/` directory

### How Grok Finds Project Root

#### For Configuration Files

Grok **walks up** from current directory until it finds:
1. A `.grok/` directory with `.env` or `config.toml`
2. OR a project marker (`.git`, `Cargo.toml`, etc.)

```rust
fn find_project_env() -> Result<PathBuf> {
    let mut current_dir = env::current_dir()?;
    
    loop {
        let env_path = current_dir.join(".grok").join(".env");
        if env_path.exists() {
            return Ok(env_path);
        }
        
        // Check for project markers
        let has_project_marker = current_dir.join(".git").exists()
            || current_dir.join("Cargo.toml").exists()
            || current_dir.join("package.json").exists()
            || current_dir.join(".grok").exists();
        
        if has_project_marker && !env_path.exists() {
            return Err(anyhow!("No project .env found"));
        }
        
        // Move to parent directory
        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            return Err(anyhow!("No project .env found"));
        }
    }
}
```

#### For Context Files

Context files are **only searched in the current directory**, not in parent directories.

```rust
pub fn load_project_context<P: AsRef<Path>>(project_root: P) -> Result<Option<String>> {
    let project_root = project_root.as_ref();
    
    // Check project directory ONLY (no walking up)
    if project_root.exists() && project_root.is_dir() {
        for file_name in CONTEXT_FILE_NAMES {
            let file_path = project_root.join(file_name);
            if file_path.exists() && file_path.is_file() {
                return Ok(Some(fs::read_to_string(&file_path)?));
            }
        }
    }
    
    // Fallback to global directory
    // ...
}
```

---

## Common Issues

### Issue 1: Configuration Not Loading from .grok/

**Symptom:**
```
Running grok from H:\GitHub\grok-cli but .grok/.env is not being used
```

**Diagnosis:**

1. **Check version:**
   ```powershell
   grok --version
   # Must be 0.1.4+
   ```

2. **Verify file exists:**
   ```powershell
   Test-Path H:\GitHub\grok-cli\.grok\.env
   # Should return: True
   ```

3. **Check if loaded:**
   ```powershell
   cd H:\GitHub\grok-cli
   grok config show | Select-String "Using project-local"
   # Should show: Using project-local configuration from: "H:\GitHub\grok-cli\.grok\.env"
   ```

**Solutions:**

- **Old version:** Upgrade to 0.1.4+ (see [TROUBLESHOOTING.md](TROUBLESHOOTING.md))
- **Wrong directory:** Ensure `.grok/.env` exists in project root
- **File permissions:** Check file is readable
- **Syntax errors:** Validate `.env` format (KEY=value, no spaces around =)

### Issue 2: Context Files Not Loading

**Symptom:**
```
Running grok from project root but .zed/rules or .grok/context.md not being used
```

**Diagnosis:**

Context files are **only searched in the current directory**, not parent directories.

**Example Problem:**

```powershell
# Your project structure
H:\GitHub\grok-cli\
  ├── .grok\
  │   └── context.md        ← Context file here
  └── src\
      └── main.rs

# Running grok from subdirectory
cd H:\GitHub\grok-cli\src
grok
# ❌ Does NOT find .grok/context.md (doesn't walk up)
```

**Solutions:**

1. **Run from project root:**
   ```powershell
   cd H:\GitHub\grok-cli
   grok
   # ✓ Finds .grok/context.md
   ```

2. **Use absolute paths** (not currently supported - feature request)

3. **Proposed Enhancement:** Make context discovery walk up like config does

### Issue 3: Wrong Version Still Running

**Symptom:**
```
Installed 0.1.4 but grok --version shows 0.1.3
```

**Solution:**

See [QUICK_FIX.md](QUICK_FIX.md) or run:
```powershell
.\scripts\cleanup_old_install.ps1
```

### Issue 4: Multiple Context Files, Only One Loading

**Symptom:**
```
I have .zed/rules AND .grok/context.md but only one is being used
```

**Explanation:**

By default, only the **first file found** (by priority) is loaded.

**Solutions:**

1. **Interactive mode automatically merges** all context files
2. **Merge manually** using `load_and_merge_project_context()`
3. **Prioritize one file** by renaming (e.g., use GEMINI.md as primary)

---

## Testing and Verification

### Verify Configuration Loading

```powershell
# Test from project root
cd H:\GitHub\grok-cli
grok config show

# Look for:
# INFO grok_cli::config: Using project-local configuration from: "H:\\GitHub\\grok-cli\\.grok\\.env"
# Configuration Source:
#   project (H:\GitHub\grok-cli\.grok\.env) + system (C:\Users\johnm\.grok\.env)
```

### Verify Context Loading

```powershell
# Start interactive mode from project root
cd H:\GitHub\grok-cli
grok

# Look for:
# ✓ Loaded and merged 3 context files
#   • .zed/rules
#   • .grok/context.md
#   • context.md
```

### Test from Subdirectories

```powershell
# Test config (should work)
cd H:\GitHub\grok-cli\src\utils
grok config show | Select-String "Using project-local"
# ✓ Should find H:\GitHub\grok-cli\.grok\.env

# Test context (won't work unless context files in src/utils/)
cd H:\GitHub\grok-cli\src\utils
grok
# ❌ Won't find .grok/context.md (doesn't walk up)
```

### Debug Mode

Enable debug logging to see exactly what's happening:

```powershell
# Windows PowerShell
$env:RUST_LOG = "debug"
grok config show

# Or inline
$env:RUST_LOG="debug"; grok config show; Remove-Item Env:RUST_LOG
```

**Look for these debug messages:**

```
DEBUG grok_cli::config: Loading configuration with hierarchical priority
DEBUG grok_cli::config: ✓ Loaded built-in defaults
DEBUG grok_cli::config: Loading system .env from: "C:\\Users\\johnm\\.grok\\.env"
DEBUG grok_cli::config: Loading project .env from: "H:\\GitHub\\grok-cli\\.grok\\.env"
INFO grok_cli::config: Using project-local configuration from: "H:\\GitHub\\grok-cli\\.grok\\.env"
```

---

## Proposed Enhancements

### 1. Make Context Discovery Walk Up Directory Tree

**Current Behavior:**
```rust
// Context: Only checks current directory
let context = load_project_context(env::current_dir())?;
```

**Proposed Behavior:**
```rust
// Context: Walk up to find project root first
let project_root = find_project_root(env::current_dir())?;
let context = load_project_context(project_root)?;
```

**Benefits:**
- Consistent with config file discovery
- Works from any subdirectory
- More intuitive for users

### 2. Add Project Root Indicator

Show detected project root in interactive mode:

```
Project: H:\GitHub\grok-cli
Context: .zed/rules, .grok/context.md
Config:  .grok/.env
```

### 3. Context File Path Display

Show full paths of loaded context files:

```
✓ Loaded context from:
  • H:\GitHub\grok-cli\.zed\rules
  • H:\GitHub\grok-cli\.grok\context.md
```

---

## Summary

### Key Takeaways

1. **Configuration (.env)** walks up directory tree → Works from subdirectories ✓
2. **Context files** only check current directory → Must run from project root ⚠️
3. **Version 0.1.4+** required for hierarchical config
4. **Run from project root** for best context discovery

### Quick Reference

| Feature          | Walks Up? | Works from Subdir? | File Location        |
|------------------|-----------|-------------------|----------------------|
| Config (.env)    | ✅ Yes    | ✅ Yes            | .grok/.env           |
| Context (rules)  | ❌ No     | ❌ No             | .zed/rules, etc.     |

### Best Practices

1. **Always run grok from project root** for full context loading
2. **Use .grok/.env** for project-specific configuration
3. **Use .zed/rules or GEMINI.md** for AI context and guidelines
4. **Check version** if config isn't loading: `grok --version`
5. **Enable debug logs** to troubleshoot: `$env:RUST_LOG="debug"`

---

## Related Documentation

- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Comprehensive troubleshooting guide
- [QUICK_FIX.md](QUICK_FIX.md) - Quick fixes for common issues
- [VERSION_CONFIG_FIX_SUMMARY.md](VERSION_CONFIG_FIX_SUMMARY.md) - Technical details
- [CONFIGURATION.md](CONFIGURATION.md) - Configuration system details

---

**Last Updated:** 2025-02-11  
**Version:** 0.1.4  
**Status:** ✅ Complete