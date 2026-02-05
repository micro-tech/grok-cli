# Build Script Quick Reference

## Overview

The `build.ps1` PowerShell script provides comprehensive build, test, and release automation for grok-cli. It can build locally, run tests, create Git tags, and trigger GitHub Actions release builds.

## Quick Start

```powershell
# Basic build
.\scripts\build.ps1

# Release build
.\scripts\build.ps1 -Release

# Build and test
.\scripts\build.ps1 -Release -Test

# Full validation (format, clippy, build, test, docs)
.\scripts\build.ps1 -All
```

## Common Usage Patterns

### Local Development

```powershell
# Quick build (debug mode)
.\scripts\build.ps1

# Build with tests
.\scripts\build.ps1 -Test

# Clean build
.\scripts\build.ps1 -Clean -Release
```

### Release Workflow

```powershell
# 1. Create release build and tag manually
.\scripts\build.ps1 -Release -Tag "v0.1.4"

# 2. Push tag to GitHub (triggers release build)
git push origin v0.1.4

# OR do it all in one step:
.\scripts\build.ps1 -Release -Tag "v0.1.4" -Push
```

### Automated Version Bump

```powershell
# Auto-increment patch version, commit, tag, and push
.\scripts\build.ps1 -Release -Auto -Push

# What this does:
# - Reads current version from Cargo.toml (e.g., 0.1.3)
# - Increments patch version (becomes 0.1.4)
# - Updates Cargo.toml
# - Commits the change
# - Creates tag v0.1.4
# - Pushes commits and tag to GitHub
# - Triggers GitHub Actions release build
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `-Release` | switch | Build in release mode (optimized) |
| `-Clean` | switch | Clean build artifacts first |
| `-Test` | switch | Run tests after building |
| `-Clippy` | switch | Run Clippy linter |
| `-Doc` | switch | Build documentation |
| `-All` | switch | Run all checks (format, clippy, build, test, docs) |
| `-Verbose` | switch | Enable verbose cargo output |
| `-Push` | switch | Push commits and tags to GitHub |
| `-Tag` | string | Create a Git tag (e.g., "v0.1.4" or "0.1.4") |
| `-Auto` | switch | Auto-increment version and create tag |
| `-Target` | string | Build for specific target (e.g., "x86_64-pc-windows-gnu") |
| `-Remote` | string | Git remote name (default: "origin") |
| `-Branch` | string | Git branch to push (default: "main") |

## Examples

### Example 1: Local Release Build
```powershell
.\scripts\build.ps1 -Release -Test
```
- Builds in release mode
- Runs tests
- Binary: `target/release/grok.exe`

### Example 2: Create Tag (No Push)
```powershell
.\scripts\build.ps1 -Release -Tag "v0.1.4"
```
- Builds release
- Creates local tag `v0.1.4`
- Does NOT push to GitHub
- Use `git push origin v0.1.4` to push later

### Example 3: Full Release (Automated)
```powershell
.\scripts\build.ps1 -Release -Auto -Push
```
- Builds in release mode
- Auto-increments version (0.1.3 → 0.1.4)
- Updates Cargo.toml
- Commits version change
- Creates tag v0.1.4
- Pushes to GitHub
- Triggers GitHub Actions release build

### Example 4: Pre-Release Validation
```powershell
.\scripts\build.ps1 -All -Release
```
- Checks code formatting
- Runs Clippy linter
- Builds in release mode
- Runs tests
- Builds documentation
- Perfect for CI validation

### Example 5: Cross-Platform Build
```powershell
.\scripts\build.ps1 -Release -Target "x86_64-unknown-linux-gnu"
```
- Builds for Linux target
- Requires target installed: `rustup target add x86_64-unknown-linux-gnu`

## GitHub Actions Integration

The script works seamlessly with GitHub Actions:

### When You Push a Tag

```powershell
.\scripts\build.ps1 -Release -Tag "v0.1.4" -Push
```

**What happens:**
1. Script builds and tests locally
2. Creates tag `v0.1.4`
3. Pushes tag to GitHub
4. GitHub Actions detects tag (matches `v*` pattern)
5. `.github/workflows/release.yml` triggers
6. Builds binaries for:
   - Windows (grok-cli-windows-latest.exe)
   - macOS (grok-cli-macos-latest)
   - Linux (grok-cli-ubuntu-latest)
7. Creates GitHub Release
8. Uploads binaries as release assets

### Monitoring the Release

After pushing a tag:
- View builds: https://github.com/microtech/grok-cli/actions
- View releases: https://github.com/microtech/grok-cli/releases

## Error Handling

### Uncommitted Changes
```
✗ Cannot create tag with uncommitted changes
```
**Solution:** Commit your changes first or run without `-Tag`

### Tag Already Exists
```
✗ Tag 'v0.1.4' already exists
```
**Solution:** Use a different tag or delete existing:
```powershell
git tag -d v0.1.4
git push origin :refs/tags/v0.1.4
```

### Build Failed
```
✗ Build failed
```
**Solution:** Fix compilation errors, then run again

### Test Failed
```
✗ Tests failed
```
**Solution:** Fix failing tests, then run again

## Version Management

### Current Version
```powershell
# Read from Cargo.toml
$version = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages[0].version
Write-Host "Current version: $version"
```

### Manual Version Bump
Edit `Cargo.toml`:
```toml
[package]
version = "0.1.4"  # Update this line
```

### Automated Version Bump
```powershell
.\scripts\build.ps1 -Auto
```

## Best Practices

### Before Release
1. **Test thoroughly:**
   ```powershell
   .\scripts\build.ps1 -All
   ```

2. **Update CHANGELOG.md:**
   - Document all changes
   - Include breaking changes
   - Note bug fixes

3. **Commit everything:**
   ```powershell
   git add .
   git commit -m "chore: prepare for release v0.1.4"
   ```

### Creating Release
1. **Use semantic versioning:**
   - MAJOR.MINOR.PATCH (e.g., 1.2.3)
   - Increment MAJOR for breaking changes
   - Increment MINOR for new features
   - Increment PATCH for bug fixes

2. **Create and push tag:**
   ```powershell
   .\scripts\build.ps1 -Release -Tag "v0.1.4" -Push
   ```

3. **Verify release:**
   - Check GitHub Actions build status
   - Test downloaded binaries
   - Update release notes on GitHub

### After Release
1. **Verify downloads:**
   - Test binary on each platform
   - Verify version: `grok --version`

2. **Announce release:**
   - Update README if needed
   - Notify users

## Troubleshooting

### Script Won't Execute
```
.\scripts\build.ps1 : File cannot be loaded because running scripts is disabled
```
**Solution:** Enable script execution:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Cargo Not Found
```
✗ cargo: command not found
```
**Solution:** Install Rust:
```powershell
winget install Rustlang.Rust.MSVC
```

### Git Not Found
```
✗ Git is not installed or not in PATH
```
**Solution:** Install Git:
```powershell
winget install Git.Git
```

## Advanced Usage

### Custom Remote
```powershell
.\scripts\build.ps1 -Release -Tag "v0.1.4" -Push -Remote "upstream"
```

### Custom Branch
```powershell
.\scripts\build.ps1 -Release -Tag "v0.1.4" -Push -Branch "develop"
```

### Dry Run (No Push)
```powershell
# Create tag locally, don't push
.\scripts\build.ps1 -Release -Tag "v0.1.4"

# Inspect the tag
git show v0.1.4

# Push manually if satisfied
git push origin v0.1.4
```

### Clean Slate Build
```powershell
# Remove all artifacts and rebuild
.\scripts\build.ps1 -Clean -Release -All
```

## CI/CD Integration

### Local Pre-Push Check
```powershell
# Run same checks as CI
.\scripts\build.ps1 -All
```

### GitHub Actions (Already Configured)
The project has two workflows:
- `ci.yml` - Runs on every push/PR
- `release.yml` - Runs on tag push (v*)

## Output Files

### Debug Build
- Binary: `target/debug/grok.exe` (Windows)
- Binary: `target/debug/grok` (Unix)
- Size: ~50-100 MB (with debug symbols)

### Release Build
- Binary: `target/release/grok.exe` (Windows)
- Binary: `target/release/grok` (Unix)
- Size: ~5-10 MB (optimized, stripped)

### Documentation
- Path: `target/doc/grok_cli/index.html`
- Open: `cargo doc --open`

## FAQ

**Q: Can I create a pre-release?**
A: Yes, use a tag like `v0.1.4-beta.1`:
```powershell
.\scripts\build.ps1 -Release -Tag "v0.1.4-beta.1" -Push
```

**Q: How do I skip tests?**
A: Don't use the `-Test` flag:
```powershell
.\scripts\build.ps1 -Release
```

**Q: Can I build without tagging?**
A: Yes, just omit `-Tag` and `-Auto`:
```powershell
.\scripts\build.ps1 -Release -Test
```

**Q: What if GitHub Actions fails?**
A: Check the logs, fix the issue, and push a new tag:
```powershell
git tag -d v0.1.4
git push origin :refs/tags/v0.1.4
# Fix issues, then:
.\scripts\build.ps1 -Release -Tag "v0.1.4" -Push
```

**Q: How do I build for multiple platforms?**
A: Use GitHub Actions (automatic) or cross-compile:
```powershell
rustup target add x86_64-unknown-linux-gnu
.\scripts\build.ps1 -Release -Target "x86_64-unknown-linux-gnu"
```

## Support

- **Repository:** https://github.com/microtech/grok-cli
- **Issues:** https://github.com/microtech/grok-cli/issues
- **Email:** john.microtech@gmail.com
- **Buy Me a Coffee:** https://buymeacoffee.com/micro.tech

---

**Last Updated:** January 2025  
**Script Version:** 2.0  
**Author:** John McConnell