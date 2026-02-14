# Installer Verification Checklist

## Pre-Installation Verification

### System Requirements
- [ ] Windows 10 or Windows 11
- [ ] Rust/Cargo installed (for building from source)
- [ ] At least 100 MB free disk space
- [ ] User has write permissions to:
  - [ ] `%LOCALAPPDATA%`
  - [ ] `%APPDATA%\Roaming`
  - [ ] `HKEY_CURRENT_USER\Environment`

### Build Verification
- [ ] Source code cloned/downloaded
- [ ] `Cargo.toml` present in project root
- [ ] `cargo build --release` completes successfully
- [ ] `target/release/grok.exe` exists
- [ ] Executable size is reasonable (~10-15 MB)

## Installation Process Verification

### Step 1: Binary Installation
- [ ] `grok.exe` copied to `%LOCALAPPDATA%\grok-cli\bin\`
- [ ] File is executable (not corrupted)
- [ ] Version matches expected: `grok --version`
- [ ] File permissions allow execution

### Step 2: Documentation Installation
- [ ] Docs directory created: `%LOCALAPPDATA%\grok-cli\docs\`
- [ ] Core documentation files present:
  - [ ] `README.md`
  - [ ] `CONFIGURATION.md`
  - [ ] `CHANGELOG.md`
  - [ ] `MAX_TOOL_LOOP_ITERATIONS.md`
- [ ] Additional docs from `Doc/docs/` copied:
  - [ ] `TOOLS.md`
  - [ ] `settings.md`
  - [ ] `ZED_INTEGRATION.md`
  - [ ] `WEB_TOOLS_SETUP.md`
  - [ ] `SKILLS_QUICK_START.md`
  - [ ] `SKILL_SECURITY.md`
  - [ ] `SKILL_SPECIFICATION.md`
- [ ] All markdown files are readable

### Step 3: License Installation
- [ ] `LICENSE` file copied to `%LOCALAPPDATA%\grok-cli\LICENSE`
- [ ] License is readable and correct

### Step 4: Configuration Setup
- [ ] Config directory created: `%APPDATA%\Roaming\grok-cli\`
- [ ] `config.example.toml` installed with all settings
- [ ] If first install: API key prompt displayed
- [ ] If first install: `config.toml` created with user's API key
- [ ] Config includes new `max_tool_loop_iterations` setting

### Step 5: Examples Installation
- [ ] Examples directory created: `%LOCALAPPDATA%\grok-cli\examples\`
- [ ] Skills directory present: `examples/skills/`
- [ ] Example skills copied:
  - [ ] `rust-expert/`
    - [ ] `SKILL.md`
    - [ ] All related files
  - [ ] `cli-design/`
    - [ ] `SKILL.md`
    - [ ] All related files
  - [ ] `README.md`
- [ ] All example files are readable

### Step 6: Global Context Setup
- [ ] `.grok` directory created in user home: `~/.grok/`
- [ ] `context.md` copied to `~/.grok/context.md`
- [ ] Context file is readable

### Step 7: PATH Configuration
- [ ] Installation directory added to user PATH
- [ ] PATH entry: `%LOCALAPPDATA%\grok-cli\bin`
- [ ] No duplicate PATH entries
- [ ] Verify with: `echo %PATH%` (should contain grok-cli)

### Step 8: Start Menu Integration
- [ ] Shortcut created in Start Menu
- [ ] Shortcut location: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Grok CLI.lnk`
- [ ] Shortcut targets correct executable
- [ ] Shortcut appears in Start Menu search
- [ ] Clicking shortcut opens command prompt with grok

### Step 9: Installation Summary
- [ ] Success message displayed
- [ ] Documentation path shown
- [ ] Reminder to restart terminal displayed

## Post-Installation Testing

### Basic Functionality
- [ ] Open NEW terminal/command prompt
- [ ] Run `grok --version` - shows version number
- [ ] Run `grok --help` - displays help text
- [ ] Run `grok health` - executes without error

### Configuration Testing
- [ ] Run `grok config validate` - no errors
- [ ] Run `grok config get api_key` - shows configured key (or prompts to set)
- [ ] Run `grok config get acp.max_tool_loop_iterations` - shows 25
- [ ] View `config.example.toml` - contains all settings with documentation

### Documentation Access
- [ ] Navigate to `%LOCALAPPDATA%\grok-cli\docs\`
- [ ] Open `README.md` - displays correctly
- [ ] Open `CONFIGURATION.md` - displays correctly
- [ ] Open `MAX_TOOL_LOOP_ITERATIONS.md` - displays correctly
- [ ] All documentation files open without errors

### Example Skills Access
- [ ] Navigate to `%LOCALAPPDATA%\grok-cli\examples\skills\`
- [ ] `rust-expert` directory present with all files
- [ ] `cli-design` directory present with all files
- [ ] README.md explains how to use examples

### API Connectivity (if key configured)
- [ ] Run `grok health --api` - connects successfully
- [ ] Run `grok chat "hello"` - gets response from API
- [ ] Verify API key works correctly

### ACP/Zed Integration (if using Zed)
- [ ] Run `grok acp capabilities` - lists capabilities
- [ ] Run `grok acp stdio` - enters ACP mode
- [ ] Can exit ACP mode with Ctrl+C
- [ ] Zed can connect to grok (if configured)

## File Size Verification

### Expected File Sizes
- [ ] `grok.exe`: ~10-15 MB
- [ ] Documentation: ~2-3 MB total
- [ ] Examples: ~500 KB
- [ ] Config files: ~50 KB
- [ ] **Total installation: ~15-20 MB**

### Disk Space
- [ ] Installation doesn't exceed 50 MB
- [ ] Enough space for future updates
- [ ] Logs directory has space to grow

## Security Verification

### File Integrity
- [ ] No unexpected files in installation directory
- [ ] All files are read from trusted source (project repository)
- [ ] No modifications to system files (only user environment)
- [ ] Registry changes limited to user hive only

### Permissions
- [ ] Installation only modifies user directories
- [ ] No admin/elevated permissions required
- [ ] User can read/write config files
- [ ] User can execute grok.exe

### API Key Security
- [ ] API key stored in `config.toml` only
- [ ] Config file has appropriate permissions
- [ ] API key not echoed to console during install
- [ ] No API key in registry or other locations

## Known Issues Check

### Windows Defender
- [ ] If SmartScreen warning appears, document for users
- [ ] Grok.exe not falsely flagged as malware
- [ ] Consider code signing for production releases

### PATH Issues
- [ ] PATH length doesn't exceed Windows limit (2048 chars)
- [ ] PATH separator is semicolon (;)
- [ ] Installation path has no spaces causing issues

### Terminal Detection
- [ ] Works in Command Prompt
- [ ] Works in PowerShell
- [ ] Works in Windows Terminal
- [ ] Works in Git Bash (if installed)

## Upgrade/Reinstall Testing

### Upgrade from Previous Version
- [ ] Existing config preserved
- [ ] New config options added to existing config
- [ ] Old binary backed up or replaced cleanly
- [ ] Documentation updated to new version
- [ ] Examples updated if needed

### Reinstall Over Existing
- [ ] Installer detects existing installation
- [ ] User data preserved
- [ ] No duplicate PATH entries
- [ ] Configuration merged correctly

## Uninstallation Verification (Manual)

Since no uninstaller exists yet, verify manual removal:

### Files to Remove
- [ ] Delete `%LOCALAPPDATA%\grok-cli\` directory
- [ ] Optionally keep `%APPDATA%\Roaming\grok-cli\config.toml`
- [ ] Optionally keep `~/.grok/context.md`
- [ ] Remove PATH entry manually from Environment Variables
- [ ] Delete Start Menu shortcut

### Cleanup Verification
- [ ] `grok` command no longer works in new terminals
- [ ] Installation directory removed
- [ ] PATH cleaned up
- [ ] Start Menu entry removed

## Regression Testing

### After Installer Changes
- [ ] Re-run all installation verification steps
- [ ] Test on clean Windows installation
- [ ] Test upgrade path from previous version
- [ ] Test with and without existing config

### Critical Paths
- [ ] Fresh installation works
- [ ] Upgrade preserves user data
- [ ] All files installed to correct locations
- [ ] Documentation accessible
- [ ] Examples functional

## Documentation Verification

### Installer Documentation
- [ ] `INSTALLER_REQUIREMENTS.md` up to date
- [ ] `INSTALLER_CHECKLIST.md` (this file) complete
- [ ] Installation instructions in README accurate
- [ ] Troubleshooting guide includes installer issues

### User-Facing Documentation
- [ ] README explains installation process
- [ ] CONFIGURATION.md explains setup
- [ ] Quick start guide available
- [ ] Common issues documented

## Comparison with Requirements

### From INSTALLER_REQUIREMENTS.md

#### Essential Files ✅
- [x] `grok.exe` - Installed
- [x] `config.toml` - Created during install
- [x] `context.md` - Copied to ~/.grok/
- [x] `LICENSE` - Installed
- [x] `README.md` - Installed
- [x] `CONFIGURATION.md` - Installed
- [x] `config.example.toml` - Installed
- [x] `MAX_TOOL_LOOP_ITERATIONS.md` - Installed

#### Documentation Files ✅
- [x] `TOOLS.md` - Installed
- [x] `settings.md` - Installed
- [x] `ZED_INTEGRATION.md` - Installed
- [x] `CHANGELOG.md` - Installed
- [x] Additional Doc/docs/ files - Installed

#### Example Files ✅
- [x] Example skills directory - Installed
- [x] `rust-expert` skill - Installed
- [x] `cli-design` skill - Installed
- [x] Skills README.md - Installed

#### Optional Files 🔄
- [ ] `grok.pdb` - NOT installed (optional debug symbols)
- [ ] `github_mcp.exe` - NOT installed (optional MCP server)
- [ ] Extensions examples - NOT installed (optional)

## Sign-Off

### Installer Version
- Version: 0.1.4
- Date: _______________
- Tested by: _______________

### Test Environment
- OS Version: Windows _______________
- Architecture: x64 / ARM64
- Rust Version: _______________
- Cargo Version: _______________

### Results Summary
- [ ] All critical checks passed
- [ ] All important checks passed
- [ ] Optional items documented
- [ ] Known issues documented
- [ ] Ready for release

### Issues Found
_List any issues discovered during testing:_

1. _Issue description..._
2. _Issue description..._
3. _Issue description..._

### Recommendations
_Improvements or fixes needed:_

1. _Recommendation..._
2. _Recommendation..._
3. _Recommendation..._

---

**Checklist Version:** 1.0  
**Last Updated:** 2025-01-XX  
**Author:** john mcconnell (john.microtech@gmail.com)  
**Repository:** https://github.com/microtech/grok-cli