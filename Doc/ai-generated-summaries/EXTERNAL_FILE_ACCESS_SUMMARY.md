# External File Access - Master Summary

**Author:** john mcconnell (john.microtech@gmail.com)  
**Repository:** https://github.com/microtech/grok-cli  
**Last Updated:** 2024

---

## TL;DR - The Problem

🚫 **AI assistants (like Grok) can only read files within the project directory**  
🔒 Files outside the project are blocked for security reasons  
✅ **Multiple workarounds available** (see below)

---

## Quick Solutions (Pick One)

### 1. ⭐ Symbolic Links (Recommended)

Create links inside your project pointing to external files.

**Windows PowerShell (as Admin):**
```powershell
New-Item -ItemType SymbolicLink -Path ".\external-config.toml" -Target "H:\Other\config.toml"
```

**Linux/macOS:**
```bash
ln -s /path/to/external/file.txt ./external-file.txt
```

**Pros:** Auto-syncs, no duplication, stays in original location  
**Cons:** Requires admin on Windows (or Developer Mode)

---

### 2. 📋 Copy Files Temporarily

```bash
copy H:\Other\file.txt .\temp-file.txt
```

**Pros:** Simple, works immediately  
**Cons:** Manual sync, need to clean up

---

### 3. 💬 Paste Content in Chat

Just paste the file content directly:
```
Can you help with this config?

[api]
endpoint = "https://example.com"
```

**Pros:** Zero setup  
**Cons:** Only for small snippets

---

### 4. 💻 Use Terminal Commands

Ask the AI to read via terminal:
```
Can you run "type H:\Other\file.txt" to read that file?
```

**Pros:** Bypasses restrictions  
**Cons:** Less structured output

---

### 5. 📂 Multiple Project Roots (Zed Editor)

In Zed: `File > Add Folder to Project`

**Pros:** Natural workspace, no file system changes  
**Cons:** Zed-specific feature

---

## Documentation Reference

### For Quick Help
📄 **`.zed/EXTERNAL_FILES_QUICK_REF.md`**
- One-page cheat sheet
- All solutions with examples
- Troubleshooting tips

### For Comprehensive Guide
📄 **`Doc/EXTERNAL_FILE_REFERENCE.md`**
- Detailed explanations
- Step-by-step instructions
- Best practices
- Comparison tables
- Full troubleshooting section

### For Future Feature Details
📄 **`Doc/PROPOSAL_EXTERNAL_ACCESS.md`**
- Technical design proposal
- Configuration-based external access
- Implementation timeline
- Security considerations

---

## Why This Limitation Exists

### Security First
- Prevents accidental exposure of sensitive files (SSH keys, .env files, passwords)
- Sandbox isolation - AI can't roam freely through filesystem
- Protection against path traversal attacks
- Controlled context helps AI focus on relevant files

### Design Philosophy
Follows "least privilege" principle - AI only accesses what it needs.

---

## Windows-Specific: Symlinks Without Admin

### Option 1: Enable Developer Mode
1. **Settings → Update & Security → For Developers**
2. Enable "Developer Mode"
3. Restart terminal
4. Create symlinks without admin: ✅

### Option 2: Use Directory Junctions
```cmd
mklink /J link-name H:\path\to\directory
```
- Works for directories only
- No admin required
- Windows Vista and newer

---

## Real-World Example

### Project Structure
```
H:\GitHub\
├── shared-configs/          ← Shared across projects
│   ├── eslint.config.js
│   ├── tsconfig.json
│   └── prettier.config.js
│
├── project-a/
│   ├── .config/            ← Symlinks to shared-configs
│   │   ├── eslint.config.js -> ../../shared-configs/eslint.config.js
│   │   └── tsconfig.json -> ../../shared-configs/tsconfig.json
│   └── src/
│
└── project-b/
    ├── .config/            ← Symlinks to shared-configs
    │   ├── eslint.config.js -> ../../shared-configs/eslint.config.js
    │   └── tsconfig.json -> ../../shared-configs/tsconfig.json
    └── src/
```

### Setup Script (`setup-links.ps1`)
```powershell
# Create symlinks for external dependencies
Write-Host "Setting up external file links..."

$links = @{
    ".\shared-eslint.config.js" = "H:\GitHub\shared-configs\eslint.config.js"
    ".\shared-tsconfig.json" = "H:\GitHub\shared-configs\tsconfig.json"
    ".\docs" = "H:\Documents\api-reference"
}

foreach ($link in $links.GetEnumerator()) {
    New-Item -ItemType SymbolicLink -Path $link.Key -Target $link.Value -Force
    Write-Host "  ✓ Linked: $($link.Key)"
}

Write-Host "Done! External files are now accessible."
```

---

## Gitignore Best Practices

Add to `.gitignore`:
```gitignore
# External file symlinks and temporary copies
external-*
temp-*
*.external.*
reference-docs/
shared-config/
ext-*/

# But document them in README
# See EXTERNAL_DEPS.md for setup instructions
```

Create `EXTERNAL_DEPS.md`:
```markdown
# External Dependencies

This project uses symlinks to reference external files:

- `shared-eslint.config.js` → `H:\GitHub\shared-configs\eslint.config.js`
- `docs/` → `H:\Documents\api-reference\`

## Setup
Run `.\setup-links.ps1` to create symlinks.
```

---

## Solution Comparison

| Solution | Ease | Sync | Admin Required | Best For |
|----------|------|------|----------------|----------|
| **Symlinks** | ⭐⭐⭐ | ✅ Auto | Windows: Yes* | Regular use |
| **Copy Files** | ⭐⭐⭐⭐ | ❌ Manual | No | One-time |
| **Paste Content** | ⭐⭐⭐⭐⭐ | ❌ Manual | No | Small snippets |
| **Terminal Tool** | ⭐⭐ | ✅ Auto | No | Quick checks |
| **Multi Roots** | ⭐⭐⭐⭐ | ✅ Auto | No | Zed editor |

*Can enable Developer Mode to avoid admin requirement

---

## Future Feature: Configurable External Access

🚀 **Status:** Proposed (see `Doc/PROPOSAL_EXTERNAL_ACCESS.md`)

### What's Planned

Configuration-based external directory access:

```toml
[security.external_access]
enabled = true
require_approval = true
logging = true

allowed_paths = [
    "H:\\GitHub\\shared-configs",
    "H:\\Documents\\api-docs"
]

excluded_patterns = [
    "**/.env",
    "**/.ssh/**",
    "**/*.key"
]
```

### Features
- ✅ Read-only access to configured directories
- ✅ User approval prompts for external files
- ✅ Audit logging of all external access
- ✅ Exclude sensitive patterns automatically
- ✅ Session-based trusted paths
- ✅ Path validation and canonicalization

### Timeline
- **Phase 1 (MVP):** 4-6 weeks - Basic functionality
- **Phase 2:** 6-8 weeks - Audit logging, validation
- **Phase 3:** 8-12 weeks - Advanced features (aliases, temp access)

### Open Feature Request
Want this feature? Add your vote/comments:
- GitHub Issue: [#XXX - External Directory Access](https://github.com/microtech/grok-cli/issues/XXX)
- Discussions: [Feature Requests](https://github.com/microtech/grok-cli/discussions)

---

## Troubleshooting

### Symlink Creation Fails

**Error:** "You do not have sufficient privilege..."

**Solutions:**
1. Run PowerShell as Administrator
2. Enable Developer Mode (Settings → For Developers)
3. Use junctions for directories: `mklink /J name target`

---

### AI Can't Read Symlinked File

**Check:**
1. Symlink is inside project directory: `ls -la` or `dir`
2. Target file exists: `type .\symlink-name`
3. Symlink is valid: `Get-Item .\symlink-name | Select-Object LinkType, Target`

**If still failing:**
- Try copying file instead
- Or use terminal command workaround
- Report as bug if it should work

---

### Symlink Breaks After Moving Project

Symlinks use absolute paths. After moving:

```powershell
# Delete old symlink
Remove-Item .\external-file.txt

# Recreate with new path
New-Item -ItemType SymbolicLink -Path ".\external-file.txt" -Target "NEW_PATH\file.txt"
```

**Better approach:** Use relative paths when possible:
```powershell
New-Item -ItemType SymbolicLink -Path ".\config" -Target "..\shared\config"
```

---

### Multiple Copies Getting Out of Sync

**Problem:** Copied files manually, now have different versions

**Solution:** Switch to symlinks:
1. Delete all copies
2. Create symlinks to single source of truth
3. Update `.gitignore` to ignore symlinks
4. Document in `EXTERNAL_DEPS.md`

---

## Common Use Cases

### 1. Shared Team Configuration
```
H:\team-configs\
├── .eslintrc.json
├── .prettierrc
└── tsconfig.base.json

→ Symlink in each project
```

### 2. API Documentation Reference
```
H:\docs\api-specs\
├── openapi.yaml
├── graphql-schema.graphql
└── rest-endpoints.md

→ Symlink docs/ in project
```

### 3. Shared Libraries/Modules
```
H:\vendor\custom-libs\
├── auth-lib/
├── utils/
└── constants/

→ Symlink into src/external/
```

### 4. Cross-Project References
```
Working on: H:\projects\api-server
Need to reference: H:\projects\shared-types

→ Symlink or use Zed multi-root
```

---

## Best Practices Checklist

- [ ] Document external dependencies in `EXTERNAL_DEPS.md`
- [ ] Add symlinks to `.gitignore`
- [ ] Create setup script for team members
- [ ] Use relative paths for symlinks when possible
- [ ] Test symlinks after project moves
- [ ] Prefer symlinks over copying for frequently-updated files
- [ ] Keep external references to minimum necessary
- [ ] Use descriptive names for symlinks (e.g., `shared-config-eslint.js`)

---

## Security Notes

### Safe Practices
✅ Symlink to configuration files (read-only)  
✅ Link to documentation and reference materials  
✅ Use excluded patterns for sensitive files  
✅ Audit external file usage periodically

### Avoid
❌ Symlinking entire home directory  
❌ Linking to `.env` or credential files  
❌ Linking system directories  
❌ Granting write access to external files  
❌ Linking `.ssh`, `.aws`, `.azure` directories

### If Using Future External Access Feature
1. Start with `require_approval = true`
2. Review audit logs monthly
3. Keep `excluded_patterns` comprehensive
4. Use most specific paths possible
5. Don't add sensitive directories to `allowed_paths`

---

## Support & Resources

### Documentation
- **Quick Reference:** `.zed/EXTERNAL_FILES_QUICK_REF.md`
- **Complete Guide:** `Doc/EXTERNAL_FILE_REFERENCE.md`
- **Feature Proposal:** `Doc/PROPOSAL_EXTERNAL_ACCESS.md`
- **Configuration:** `Doc/CONFIG_QUICK_START.md`

### Community
- **GitHub Issues:** https://github.com/microtech/grok-cli/issues
- **Discussions:** https://github.com/microtech/grok-cli/discussions
- **Contributing:** `CONTRIBUTING.md`

### Author
- **Name:** john mcconnell
- **Email:** john.microtech@gmail.com
- **Buy Me a Coffee:** https://buymeacoffee.com/micro.tech

---

## Summary

While AI assistants are confined to project directories for security, multiple practical workarounds exist:

1. **Use symlinks** for regular external file access (best solution)
2. **Copy files temporarily** for one-off needs
3. **Paste content directly** for small snippets
4. **Use terminal commands** to bypass restrictions
5. **Use Zed multi-root** for related projects

The **symbolic link** approach provides the best balance of convenience, security, and maintainability for most use cases.

A configuration-based external access feature is proposed for future implementation, which will provide native support with proper security controls.

---

**Last Updated:** 2024  
**Version:** 1.0  
**Status:** Current workarounds documented; future feature planned