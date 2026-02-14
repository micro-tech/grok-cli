# Fix for Max Tool Loop Iterations Issue

## Problem

You may encounter this error:
```
Max tool loop iterations reached (100 iterations). Consider increasing 'acp.max_tool_loop_iterations' in config or breaking task into smaller steps.
```

Even though your `config.toml` at `%AppData%\Roaming\grok-cli\config.toml` shows:
```toml
[acp]
max_tool_loop_iterations = 1000
```

## Root Cause

When Grok CLI starts without an explicit `--config` flag, it uses **hierarchical configuration loading** which prioritizes:

1. **Project-local config** (`.grok/config.toml` in current directory tree)
2. **System-level config** (`%AppData%\Roaming\grok-cli\config.toml`)
3. **Environment variables** (`.env` files and system environment)
4. **Built-in defaults** (25 iterations)

**Previously**, the hierarchical loader only read `.env` files and ignored `config.toml` files. This meant your `config.toml` settings were never loaded unless you explicitly specified `--config` flag.

## Solution (After Fix)

The hierarchical loader now properly loads both `config.toml` and `.env` files in the correct priority order. Your existing `config.toml` at `%AppData%\Roaming\grok-cli\config.toml` should now be respected.

## Configuration Priority (Highest to Lowest)

1. **Environment variables** - `GROK_ACP_MAX_TOOL_LOOP_ITERATIONS`
2. **Project `.env` file** - `.grok/.env` in project directory
3. **Project `config.toml`** - `.grok/config.toml` in project directory
4. **System `.env` file** - `%AppData%\Roaming\grok-cli\.env`
5. **System `config.toml`** - `%AppData%\Roaming\grok-cli\config.toml`
6. **Built-in defaults** - 25 iterations

## How to Configure

### Option 1: System-Level config.toml (Recommended)

Edit or create: `%AppData%\Roaming\grok-cli\config.toml`

```toml
[acp]
max_tool_loop_iterations = 1000
```

This applies to all Grok CLI sessions unless overridden by project-specific config.

### Option 2: System-Level .env File

Create: `%AppData%\Roaming\grok-cli\.env`

```env
GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=1000
```

This has higher priority than `config.toml` but lower than project-specific settings.

### Option 3: Project-Level config.toml

For specific projects, create: `.grok/config.toml` in your project directory

```toml
[acp]
max_tool_loop_iterations = 1000
```

This overrides system-level settings for this project only.

### Option 4: Project-Level .env File

Create: `.grok/.env` in your project directory

```env
GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=1000
```

This has the highest priority among file-based configs.

### Option 5: Environment Variable (Temporary)

Set a Windows environment variable (highest priority):

```cmd
set GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=1000
grok chat
```

Or in PowerShell:
```powershell
$env:GROK_ACP_MAX_TOOL_LOOP_ITERATIONS = 1000
grok chat
```

## Verification

To verify which config is being loaded:

```bash
# Run with debug logging
set RUST_LOG=grok_cli=debug
grok chat
```

Look for log messages like:
- `✓ Loaded system config.toml from: ...`
- `✓ Loaded project config.toml from: ...`
- `✓ Loaded system .env from: ...`
- `✓ Loaded project .env from: ...`

## Troubleshooting

### Config Still Not Loading?

1. **Check file locations:**
   ```cmd
   echo %AppData%\Roaming\grok-cli\config.toml
   type %AppData%\Roaming\grok-cli\config.toml
   ```

2. **Verify TOML syntax:**
   Make sure your `config.toml` is valid TOML format. Common issues:
   - Missing `[acp]` section header
   - Typos in `max_tool_loop_iterations`
   - Invalid values (must be a positive integer)

3. **Check for project overrides:**
   If you're in a project directory, check if there's a `.grok/config.toml` or `.grok/.env` that might be overriding your system config.

4. **Enable debug logging:**
   ```cmd
   set RUST_LOG=debug
   grok chat
   ```
   This will show exactly which config files are being loaded.

### Still Hitting the Limit?

If you're still hitting the 100-iteration limit after configuration:

1. **Verify the config value is actually applied:**
   ```bash
   grok config get acp.max_tool_loop_iterations
   ```

2. **Try the environment variable method** (highest priority):
   ```cmd
   set GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=1000
   ```

3. **Rebuild/reinstall** if you made the code changes:
   ```bash
   cargo build --release
   ```

## Recommended Values

- **Default (25)**: Good for simple, focused tasks
- **100-250**: Suitable for moderate complexity tasks
- **500-1000**: Complex multi-step workflows
- **1000+**: Very complex automation or research tasks

**Note**: Higher values allow longer AI tool loops but may increase costs and time. Consider breaking very complex tasks into smaller subtasks instead of just increasing the limit.

## Implementation Details

The fix was implemented in `src/config/mod.rs` in the `load_hierarchical()` function to:

1. Load system-level `config.toml` first (as a baseline)
2. Apply system-level `.env` overrides
3. Load project-level `config.toml` (if exists)
4. Apply project-level `.env` overrides
5. Apply environment variable overrides (highest priority)

This ensures proper cascading of configuration values with intuitive priority ordering.