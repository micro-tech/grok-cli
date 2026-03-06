# Quick Fix Summary: Max Tool Loop Iterations Issue

## The Problem You Encountered

You're getting this error:
```
Max tool loop iterations reached (100 iterations)
```

Even though your config file at `%AppData%\Roaming\grok-cli\config.toml` shows `max_tool_loop_iterations = 1000`.

## Root Cause

The hierarchical config loader was **only reading `.env` files** and **ignoring `config.toml` files**. This meant your `config.toml` settings were never being loaded when you started Grok normally.

## What Was Fixed

Modified `src/config/mod.rs` in the `load_hierarchical()` function to:
- Now loads both `config.toml` AND `.env` files
- Proper priority ordering: project → system → defaults
- Environment variables still have the highest priority

## Immediate Solution (No Rebuild Required)

While your code is being rebuilt, you can fix this immediately by using an environment variable:

### Windows Command Prompt:
```cmd
set GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=1000
grok chat
```

### Windows PowerShell:
```powershell
$env:GROK_ACP_MAX_TOOL_LOOP_ITERATIONS = 1000
grok chat
```

### Permanent Fix (Alternative):
Create a `.env` file at `%AppData%\Roaming\grok-cli\.env` with:
```
GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=1000
```

This will work with your current Grok installation (no rebuild needed).

## After Rebuild

Once you rebuild and install the fixed version:

1. Your existing `config.toml` at `%AppData%\Roaming\grok-cli\config.toml` will be properly loaded
2. The `max_tool_loop_iterations = 1000` setting will be respected
3. You won't need the environment variable workaround anymore

## Configuration Priority (After Fix)

From highest to lowest priority:

1. **Environment Variable**: `GROK_ACP_MAX_TOOL_LOOP_ITERATIONS`
2. **Project .env**: `.grok/.env` in current project
3. **Project config.toml**: `.grok/config.toml` in current project
4. **System .env**: `%AppData%\Roaming\grok-cli\.env`
5. **System config.toml**: `%AppData%\Roaming\grok-cli\config.toml` ← Your file here!
6. **Built-in default**: 25 iterations

## To Rebuild and Install

```bash
cd grok-cli
cargo build --release
```

The executable will be at: `target/release/grok.exe`

Copy it to your installation directory or run the installer.

## Verify It's Working

After rebuild, check which config is loaded:
```cmd
set RUST_LOG=grok_cli=debug
grok chat
```

Look for log messages showing:
- `✓ Loaded system config.toml from: ...`
- This confirms your config.toml is being read

## More Details

See `Doc/FIX_MAX_TOOL_LOOP_ITERATIONS.md` for comprehensive documentation.

## Files Modified

- `src/config/mod.rs` - Fixed `load_hierarchical()` function
- `Doc/FIX_MAX_TOOL_LOOP_ITERATIONS.md` - Detailed documentation

## Recommended Settings

- **25** (default): Simple tasks
- **100-250**: Moderate complexity
- **500-1000**: Complex workflows (your setting)
- **1000+**: Very complex automation

Your setting of 1000 is perfect for complex multi-step tasks!