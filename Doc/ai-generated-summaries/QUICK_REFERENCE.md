# Quick Reference: File Access Fixes

## Problem & Solution

**Problem:** grok-cli couldn't access files using relative paths like `./file.txt` or `src/main.rs`

**Solution:** Added path resolution to convert relative paths to absolute before checking security

## What Changed

### Before ❌
```bash
cd /home/user/project
grok query "read src/main.rs"
# Error: Access denied: Path is not in a trusted directory
```

### After ✅
```bash
cd /home/user/project
grok query "read src/main.rs"
# ✅ Works! Resolves to /home/user/project/src/main.rs
```

## Now Supported

| Path Type | Example | Status |
|-----------|---------|--------|
| Relative | `src/main.rs` | ✅ Works |
| Current Dir | `./README.md` | ✅ Works |
| Parent Dir | `../config.toml` | ✅ Works |
| Absolute | `/home/user/project/file.txt` | ✅ Works |
| Symlinks | `link.txt` → `/other/file.txt` | ✅ Resolves & checks |
| Outside Workspace | `/etc/passwd` | ❌ Correctly denied |

## Technical Details

### Files Modified
- `src/acp/security.rs` - Added `resolve_path()` method
- `src/acp/tools.rs` - Updated all file tools
- `src/acp/mod.rs` - Enhanced initialization

### Key Changes
1. **Working directory tracking** - SecurityPolicy stores current working directory
2. **Path resolution** - Converts relative → absolute before security checks
3. **Symlink following** - Uses `canonicalize()` to resolve symlinks
4. **Parent directory support** - Handles `..` components correctly

### Tests Added
- 9 new security tests covering all path scenarios
- All existing tests still pass
- Test coverage for relative paths, symlinks, parent dirs, etc.

## Common Use Cases

### Reading Files in Current Directory
```bash
grok query "read README.md"
grok query "read ./Cargo.toml"
```

### Reading Files in Subdirectories
```bash
grok query "read src/main.rs"
grok query "read docs/guide.md"
```

### Accessing Parent Directory
```bash
cd src
grok query "read ../README.md"
```

### Multiple Operations
```bash
grok query "read src/lib.rs and src/main.rs"
```

## Security

### Still Protected ✅
- Paths outside trusted directories are blocked
- No security regressions
- Deny-by-default policy maintained
- Symlinks are resolved and checked

### Trust Model
1. Current directory is trusted by default
2. Paths must resolve within trusted directories
3. Symlinks are followed and destination is checked
4. Relative paths work within trusted scope

## Configuration

### Model Selection
Create `.grok/.env` in your project:
```env
GROK_MODEL=grok-code-fast-1
```

Or system-wide at `~/.grok/.env`

### API Key (Required)
```env
GROK_API_KEY=xai-your-key-here
```

Store in system-wide `~/.grok/.env` for security

## Verification

Test that it works:
```bash
# Navigate to your project
cd /path/to/your/project

# Test relative path
grok query "list files in current directory"

# Test file reading
grok query "read README.md"

# Test subdirectory
grok query "read src/main.rs"
```

## ACP Protocol

The fix applies to:
- ✅ Direct CLI usage
- ✅ ACP protocol (Zed editor integration)
- ✅ Interactive chat mode
- ✅ All file operation tools

## Documentation

For detailed information:
- `.grok/FILE_ACCESS_ANALYSIS.md` - Technical deep dive
- `.grok/FILE_ACCESS_FIX_SUMMARY.md` - Complete summary
- `.grok/ENV_CONFIG_GUIDE.md` - Configuration guide

## Troubleshooting

### Still getting access denied?
1. Check you're in the project directory
2. Verify path is correct: `ls -la <path>`
3. Check if file exists
4. Try absolute path to confirm it's a path resolution issue

### Path not found?
1. Verify working directory: `pwd`
2. Check file exists: `ls <file>`
3. Use tab completion to verify path
4. Try `./<file>` prefix

### Outside workspace?
This is correct behavior - files outside your project are intentionally blocked for security.

## Quick Test

```bash
# Create test file
echo "Hello from grok-cli" > test.txt

# Test reading it
grok query "read test.txt"

# Should show: "Hello from grok-cli"
```

## Summary

✅ Relative paths now work correctly
✅ Symlinks are resolved properly  
✅ Parent directory access works
✅ Security is maintained
✅ All tests pass
✅ Ready for production use

The fix aligns grok-cli with gemini-cli's path handling approach while maintaining security.