# Installer Requirements for Grok CLI

## Overview

This document specifies all files and resources that should be included in the Grok CLI installer for Windows 11. It also documents what the current installer includes and identifies any gaps.

## Current Installer Status

### What the Current Installer Does

The `installer.rs` currently performs these operations:

1. ✅ **Builds the release binary** - `cargo build --release --bin grok`
2. ✅ **Copies grok.exe** - To `%LOCALAPPDATA%\grok-cli\bin\`
3. ✅ **Updates PATH** - Adds installation directory to user PATH
4. ✅ **Creates Start Menu shortcut** - `Grok CLI.lnk` in Start Menu
5. ✅ **Sets up config directory** - Creates `%APPDATA%\Roaming\grok-cli\`
6. ✅ **Interactive API key setup** - Prompts for X API key and creates basic config
7. ✅ **Copies global context** - Copies `context.md` to `~/.grok/context.md`

### Files Currently Installed

| File/Directory | Source | Destination | Status |
|----------------|--------|-------------|--------|
| `grok.exe` | `target/release/grok.exe` | `%LOCALAPPDATA%\grok-cli\bin\` | ✅ Installed |
| `context.md` | `context.md` (project root) | `~/.grok/context.md` | ✅ Installed |
| `config.toml` | Generated during install | `%APPDATA%\Roaming\grok-cli\config.toml` | ✅ Created |

## Required Files for Complete Installation

### Essential Files (Must Have)

#### 1. Executable
- **File:** `grok.exe`
- **Source:** `target/release/grok.exe`
- **Destination:** `%LOCALAPPDATA%\grok-cli\bin\grok.exe`
- **Size:** ~10-15 MB
- **Status:** ✅ Currently installed
- **Purpose:** Main CLI executable

#### 2. Debug Symbols (Optional but Recommended)
- **File:** `grok.pdb`
- **Source:** `target/release/grok.pdb`
- **Destination:** `%LOCALAPPDATA%\grok-cli\bin\grok.pdb`
- **Size:** ~20-30 MB
- **Status:** ❌ NOT currently installed
- **Purpose:** Debugging information for crash reports
- **Recommendation:** Include for better error reporting

### Configuration Files

#### 3. Example Configuration
- **File:** `config.example.toml`
- **Source:** `config.example.toml`
- **Destination:** `%APPDATA%\Roaming\grok-cli\config.example.toml`
- **Status:** ❌ NOT currently installed
- **Purpose:** Reference configuration with all available options
- **Priority:** HIGH - Users need this for configuration reference

#### 4. Default Configuration Template
- **File:** `config.toml` (generated)
- **Source:** Generated during installation
- **Destination:** `%APPDATA%\Roaming\grok-cli\config.toml`
- **Status:** ✅ Created during install
- **Purpose:** Active user configuration

### Documentation Files

#### 5. README
- **File:** `README.md`
- **Source:** `README.md`
- **Destination:** `%LOCALAPPDATA%\grok-cli\docs\README.md`
- **Status:** ❌ NOT currently installed
- **Priority:** HIGH - Primary documentation

#### 6. Configuration Guide
- **File:** `CONFIGURATION.md`
- **Source:** `CONFIGURATION.md`
- **Destination:** `%LOCALAPPDATA%\grok-cli\docs\CONFIGURATION.md`
- **Status:** ❌ NOT currently installed
- **Priority:** HIGH - Essential for setup

#### 7. Max Tool Loop Iterations Guide
- **File:** `MAX_TOOL_LOOP_ITERATIONS.md`
- **Source:** `Doc/MAX_TOOL_LOOP_ITERATIONS.md`
- **Destination:** `%LOCALAPPDATA%\grok-cli\docs\MAX_TOOL_LOOP_ITERATIONS.md`
- **Status:** ❌ NOT currently installed
- **Priority:** HIGH - Common error resolution

#### 8. Changelog
- **File:** `CHANGELOG.md`
- **Source:** `CHANGELOG.md`
- **Destination:** `%LOCALAPPDATA%\grok-cli\docs\CHANGELOG.md`
- **Status:** ❌ NOT currently installed
- **Priority:** MEDIUM - Version information

#### 9. License
- **File:** `LICENSE`
- **Source:** `LICENSE`
- **Destination:** `%LOCALAPPDATA%\grok-cli\LICENSE`
- **Status:** ❌ NOT currently installed
- **Priority:** HIGH - Legal requirement

#### 10. Full Documentation Directory
- **Directory:** `Doc/docs/`
- **Files:**
  - `TOOLS.md`
  - `settings.md`
  - `ZED_INTEGRATION.md`
  - `WEB_TOOLS_SETUP.md`
  - `SKILLS_QUICK_START.md`
  - `SKILL_SECURITY.md`
  - `SKILL_SPECIFICATION.md`
- **Destination:** `%LOCALAPPDATA%\grok-cli\docs\`
- **Status:** ❌ NOT currently installed
- **Priority:** MEDIUM - Reference documentation

### Context and Examples

#### 11. Global Context File
- **File:** `context.md`
- **Source:** `context.md`
- **Destination:** `~/.grok/context.md`
- **Status:** ✅ Currently installed
- **Purpose:** Global AI context for all projects

#### 12. Example Skills
- **Directory:** `examples/skills/`
- **Contents:**
  - `rust-expert/`
  - `cli-design/`
  - `README.md`
- **Destination:** `%LOCALAPPDATA%\grok-cli\examples\skills\`
- **Status:** ❌ NOT currently installed
- **Priority:** MEDIUM - Example implementations

#### 13. Example Extensions
- **Directory:** `examples/extensions/`
- **Destination:** `%LOCALAPPDATA%\grok-cli\examples\extensions\`
- **Status:** ❌ NOT currently installed
- **Priority:** LOW - Advanced feature examples

### Additional Executables (Optional)

#### 14. Helper Tools
- **File:** `docgen.exe`
- **Source:** `target/release/docgen.exe`
- **Destination:** `%LOCALAPPDATA%\grok-cli\bin\docgen.exe`
- **Status:** ❌ NOT currently installed
- **Priority:** LOW - Development tool

- **File:** `banner_demo.exe`
- **Source:** `target/release/banner_demo.exe`
- **Status:** Not needed in production installer
- **Priority:** NONE - Demo/testing only

- **File:** `github_mcp.exe`
- **Source:** `target/release/github_mcp.exe`
- **Destination:** `%LOCALAPPDATA%\grok-cli\bin\github_mcp.exe`
- **Status:** ❌ NOT currently installed
- **Priority:** MEDIUM - MCP server functionality

## Installation Directory Structure

### Recommended Structure

```
%LOCALAPPDATA%\grok-cli\
├── bin\
│   ├── grok.exe                    ✅ Installed
│   ├── grok.pdb                    ❌ Missing (optional)
│   └── github_mcp.exe              ❌ Missing (optional)
├── docs\
│   ├── README.md                   ❌ Missing
│   ├── CONFIGURATION.md            ❌ Missing
│   ├── MAX_TOOL_LOOP_ITERATIONS.md ❌ Missing
│   ├── CHANGELOG.md                ❌ Missing
│   ├── TOOLS.md                    ❌ Missing
│   ├── settings.md                 ❌ Missing
│   ├── ZED_INTEGRATION.md          ❌ Missing
│   ├── WEB_TOOLS_SETUP.md          ❌ Missing
│   ├── SKILLS_QUICK_START.md       ❌ Missing
│   ├── SKILL_SECURITY.md           ❌ Missing
│   └── SKILL_SPECIFICATION.md      ❌ Missing
├── examples\
│   ├── skills\
│   │   ├── rust-expert\            ❌ Missing
│   │   ├── cli-design\             ❌ Missing
│   │   └── README.md               ❌ Missing
│   └── extensions\                 ❌ Missing
├── LICENSE                         ❌ Missing
└── config.example.toml             ❌ Missing

%APPDATA%\Roaming\grok-cli\
└── config.toml                     ✅ Created

~/.grok\
└── context.md                      ✅ Installed
```

## Missing Files Summary

### Critical (Must Add)

1. ❌ `LICENSE` - Legal requirement
2. ❌ `README.md` - Essential documentation
3. ❌ `CONFIGURATION.md` - Setup guide
4. ❌ `config.example.toml` - Configuration reference
5. ❌ `MAX_TOOL_LOOP_ITERATIONS.md` - Common error resolution

### Important (Should Add)

6. ❌ `TOOLS.md` - Tool documentation
7. ❌ `ZED_INTEGRATION.md` - Editor integration guide
8. ❌ `CHANGELOG.md` - Version history
9. ❌ Example skills directory - User examples

### Optional (Nice to Have)

10. ❌ `grok.pdb` - Debug symbols
11. ❌ `github_mcp.exe` - MCP server
12. ❌ Full docs directory - Complete documentation
13. ❌ Example extensions - Advanced examples

## Installer Enhancement Recommendations

### Phase 1: Critical Files (Immediate)

```rust
// Add to installer.rs after copying grok.exe:

// Copy LICENSE
let license_src = root_dir.join("LICENSE");
let license_dst = install_dir.parent().unwrap().join("LICENSE");
if license_src.exists() {
    fs::copy(&license_src, &license_dst)?;
}

// Copy example config
let example_config_src = root_dir.join("config.example.toml");
let example_config_dst = config_dir.join("config.example.toml");
if example_config_src.exists() {
    fs::copy(&example_config_src, &example_config_dst)?;
}

// Copy essential documentation
let docs = vec![
    "README.md",
    "CONFIGURATION.md",
    "Doc/MAX_TOOL_LOOP_ITERATIONS.md",
    "CHANGELOG.md"
];

let docs_dir = install_dir.parent().unwrap().join("docs");
fs::create_dir_all(&docs_dir)?;

for doc in docs {
    let src = root_dir.join(doc);
    let filename = Path::new(doc).file_name().unwrap();
    let dst = docs_dir.join(filename);
    if src.exists() {
        fs::copy(&src, &dst)?;
    }
}
```

### Phase 2: Documentation (Next Priority)

```rust
// Copy all Doc/docs/ files
let doc_source_dir = root_dir.join("Doc").join("docs");
if doc_source_dir.exists() {
    for entry in fs::read_dir(doc_source_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            let filename = path.file_name().unwrap();
            let dst = docs_dir.join(filename);
            fs::copy(&path, &dst)?;
        }
    }
}
```

### Phase 3: Examples (Optional Enhancement)

```rust
// Copy example skills
let skills_src = root_dir.join("examples").join("skills");
let skills_dst = install_dir.parent().unwrap().join("examples").join("skills");
if skills_src.exists() {
    copy_dir_recursive(&skills_src, &skills_dst)?;
}

// Helper function to copy directories recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());
        
        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}
```

## Uninstaller Requirements

### Files to Remove During Uninstall

1. ✅ `%LOCALAPPDATA%\grok-cli\` (entire directory)
2. ⚠️ `%APPDATA%\Roaming\grok-cli\config.toml` (ask user to keep/remove)
3. ⚠️ `~/.grok/context.md` (ask user to keep/remove)
4. ✅ Start Menu shortcut
5. ✅ PATH environment variable entry

### User Data Preservation

Prompt user during uninstall:
- "Keep your configuration file?" (config.toml)
- "Keep your global context?" (context.md)
- "Keep your chat history?" (if any)

## Installation Size Estimate

### Current Installation
- `grok.exe`: ~12 MB
- `config.toml`: ~1 KB
- `context.md`: ~50 KB
- **Total: ~12 MB**

### With All Recommended Files
- Executables: ~15 MB
- Documentation: ~2 MB
- Examples: ~500 KB
- Debug symbols: ~25 MB (optional)
- **Total: ~17.5 MB** (42.5 MB with debug symbols)

## Registry Changes

### Current Registry Modifications

1. ✅ `HKEY_CURRENT_USER\Environment\Path` - Added installation directory

### Recommended Additional Registry Entries

```
HKEY_CURRENT_USER\Software\grok-cli\
├── InstallPath (REG_SZ) = %LOCALAPPDATA%\grok-cli
├── Version (REG_SZ) = 0.1.3
├── InstallDate (REG_SZ) = 2025-01-XX
└── ConfigPath (REG_SZ) = %APPDATA%\Roaming\grok-cli
```

Purpose: Help with version checking, updates, and clean uninstallation.

## Startup Configuration

### Post-Installation Actions

After installation completes, the installer should:

1. ✅ Display success message
2. ✅ Prompt to restart terminal
3. ❌ Offer to run `grok config init` (recommended addition)
4. ❌ Offer to open documentation (recommended addition)
5. ❌ Offer to test API key (recommended addition)

### Recommended Post-Install Dialog

```
Installation Complete!

Grok CLI has been installed to:
  %LOCALAPPDATA%\grok-cli\

Documentation available at:
  %LOCALAPPDATA%\grok-cli\docs\README.md

Next steps:
  [ ] Restart your terminal
  [ ] Run: grok config init --force
  [ ] Run: grok health --api
  [ ] View docs: grok --help

Would you like to:
  [Y] Test your API key now
  [N] Exit installer
```

## Update/Upgrade Considerations

### Version Detection

```rust
fn check_existing_installation(install_dir: &Path) -> Option<String> {
    let version_file = install_dir.join("VERSION");
    if version_file.exists() {
        fs::read_to_string(version_file).ok()
    } else {
        None
    }
}
```

### Preserve User Data During Updates

1. ✅ Keep existing `config.toml`
2. ✅ Keep existing chat history
3. ✅ Backup old executable as `grok.exe.backup`
4. ⚠️ Merge new config options with existing config
5. ⚠️ Update documentation files

## Validation Checklist

### Pre-Installation Checks

- [ ] Verify Rust/Cargo installed
- [ ] Check Windows version (Windows 10/11)
- [ ] Verify write permissions to installation directories
- [ ] Check available disk space (50 MB minimum)
- [ ] Detect existing installation

### Post-Installation Verification

- [ ] Executable exists and is accessible
- [ ] PATH updated correctly
- [ ] Config directory created
- [ ] Start Menu shortcut works
- [ ] `grok --version` runs successfully
- [ ] Documentation files accessible

## Platform-Specific Notes

### Windows 11 Specific

- ✅ Uses `%LOCALAPPDATA%` for binaries (Windows standard)
- ✅ Uses `%APPDATA%\Roaming` for config (Windows standard)
- ✅ Updates user PATH (not system PATH)
- ✅ Creates Start Menu shortcut via PowerShell
- ⚠️ Consider Windows Store packaging (future)
- ⚠️ Consider winget package (future)

### Windows Defender Considerations

Some files may trigger Windows Defender SmartScreen:
- First-time executables require "Run anyway" permission
- Code signing certificate recommended for production
- PDB files improve crash reporting without security issues

## Comparison with Other Installers

### npm installer (install.js)

**Purpose:** Post-npm-install hook
**Method:** Uses `cargo install grok-cli`
**Pros:** 
- Simple
- Leverages existing cargo infrastructure
**Cons:**
- Requires Rust toolchain
- No customization
- No documentation installation

### Rust installer (installer.rs)

**Purpose:** Direct installation from source
**Method:** Copies built binary and sets up environment
**Pros:**
- Full control
- Can include documentation
- No external dependencies after build
**Cons:**
- Windows-only currently
- Manual PATH management

## Recommendations Summary

### Immediate Actions (Critical)

1. **Add LICENSE file** to installation
2. **Add config.example.toml** to config directory
3. **Add core documentation** (README, CONFIGURATION, MAX_TOOL_LOOP_ITERATIONS)
4. **Create VERSION file** for update detection

### Next Priority (Important)

5. **Add all Doc/docs/ files** to installation
6. **Add example skills** for user reference
7. **Implement update detection** to preserve user data
8. **Add post-install verification** to ensure everything works

### Future Enhancements (Nice to Have)

9. **Add uninstaller** with user data preservation options
10. **Add debug symbols** (grok.pdb) as optional component
11. **Add MCP server executable** (github_mcp.exe)
12. **Create proper installer package** (MSI or NSIS)
13. **Implement auto-update mechanism**
14. **Add code signing certificate**

## Testing Plan

### Manual Testing Checklist

- [ ] Fresh install on clean Windows 11 system
- [ ] Install over existing installation
- [ ] Verify all files copied correctly
- [ ] Test grok.exe launches
- [ ] Test PATH configuration
- [ ] Test config creation
- [ ] Open documentation files
- [ ] Test Start Menu shortcut
- [ ] Uninstall and verify cleanup

### Automated Testing

```bash
# Test installation
cargo run --bin installer

# Verify installation
grok --version
grok health
grok config validate

# Check files exist
test -f $LOCALAPPDATA/grok-cli/bin/grok.exe
test -f $APPDATA/grok-cli/config.toml
test -f $HOME/.grok/context.md
```

## Conclusion

The current installer covers the essential functionality (executable + basic config), but is missing important documentation and example files that would significantly improve the user experience. 

**Priority order for improvements:**
1. Add LICENSE and core documentation
2. Add example configuration file
3. Add tool and integration documentation
4. Add example skills and extensions
5. Implement update/upgrade handling

**Estimated effort:**
- Phase 1 (Critical files): 2-3 hours
- Phase 2 (Documentation): 2-3 hours
- Phase 3 (Examples): 1-2 hours
- Testing: 2-3 hours
- **Total: 7-11 hours** for complete installer enhancement

---

**Document Version:** 1.0  
**Last Updated:** 2025-01-XX  
**Author:** john mcconnell (john.microtech@gmail.com)  
**Repository:** https://github.com/microtech/grok-cli