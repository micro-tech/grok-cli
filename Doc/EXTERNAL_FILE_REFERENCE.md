# External File Reference Guide

## The Problem

When working with AI assistants like Grok CLI, you may encounter situations where you need to reference files outside your current project directory. Unfortunately, AI assistants are confined to reading files only within the project's root directories for security reasons.

**What doesn't work:**
```bash
# From H:\GitHub\grok-cli, trying to read a file in H:\GitHub\another-project
read ../another-project/config.toml  ❌ Access denied
```

This guide provides **workarounds** and **solutions** to help you reference external files.

---

## Why This Limitation Exists

### Security & Safety
- **Prevents accidental exposure** of sensitive files (SSH keys, credentials, personal documents)
- **Sandbox isolation** ensures the AI can't roam freely through your filesystem
- **Controlled context** helps the AI focus on relevant project files only
- **Protection from path traversal** attacks and unintended file access

### Design Philosophy
The restriction is intentional and follows the principle of "least privilege" - the AI only has access to what it absolutely needs.

---

## Solutions & Workarounds

### Solution 1: Use Symbolic Links (Symlinks) ⭐ **Recommended**

Create a symlink inside your project to reference external files/directories.

#### Windows (PowerShell as Administrator)
```powershell
# Link a single file
New-Item -ItemType SymbolicLink -Path ".\external-config.toml" -Target "H:\GitHub\another-project\config.toml"

# Link an entire directory
New-Item -ItemType SymbolicLink -Path ".\external-docs" -Target "H:\Documents\reference-docs"
```

#### Windows (Command Prompt as Administrator)
```cmd
# Link a single file
mklink "external-config.toml" "H:\GitHub\another-project\config.toml"

# Link a directory
mklink /D "external-docs" "H:\Documents\reference-docs"
```

#### Linux/macOS
```bash
# Link a single file
ln -s /path/to/external/file.txt ./external-file.txt

# Link a directory
ln -s /path/to/external/directory ./external-dir
```

#### Benefits
✅ Files stay in their original location  
✅ Changes sync automatically  
✅ No duplication of data  
✅ Works seamlessly with version control (add symlinks to `.gitignore`)  

#### Gitignore Example
```gitignore
# Ignore symlinked external files
external-config.toml
external-docs/
reference-files/
```

---

### Solution 2: Copy Files into Project

Copy external files into your project temporarily.

```bash
# Windows
copy H:\GitHub\another-project\config.toml .\temp-config.toml

# Linux/macOS
cp /path/to/external/file.txt ./temp-file.txt
```

#### Benefits
✅ Simple and straightforward  
✅ No special permissions needed  
✅ Files are fully accessible to AI  

#### Drawbacks
❌ Creates duplication  
❌ Changes don't sync  
❌ Need to manage cleanup  

**Remember to add to `.gitignore`:**
```gitignore
# Temporary external files
temp-*.toml
temp-*.txt
external-copy/
```

---

### Solution 3: Use Multiple Project Roots

If you're working with Zed editor, you can open multiple project folders simultaneously.

#### In Zed Editor
1. Open your main project: `File > Open Folder` → `H:\GitHub\grok-cli`
2. Add another folder: `File > Add Folder to Project` → `H:\GitHub\another-project`
3. The AI can now access files in both directories

#### Benefits
✅ Access multiple projects naturally  
✅ No file copying or symlinking  
✅ Clean workspace organization  

---

### Solution 4: Manual Copy-Paste

For small snippets or one-off references, just paste the content directly into your chat.

**Example:**
```
I need help understanding this config from another project:

[api]
endpoint = "https://api.example.com"
timeout = 60
```

#### Benefits
✅ Works immediately  
✅ No file system changes  
✅ Good for small snippets  

#### Drawbacks
❌ Not suitable for large files  
❌ Manual and error-prone  
❌ No file path context  

---

### Solution 5: Use the Terminal Tool

Ask the AI to use the terminal to read external files.

**Example prompt:**
```
Can you use the terminal to read the contents of H:\GitHub\another-project\config.toml?
```

The AI will execute:
```bash
cat H:\GitHub\another-project\config.toml
# or on Windows
type H:\GitHub\another-project\config.toml
```

#### Benefits
✅ Bypasses read_file restrictions  
✅ Can access any readable file  
✅ Useful for quick checks  

#### Drawbacks
❌ Less structured than `read_file`  
❌ May hit output length limits  
❌ Requires explicit permission each time  

---

### Solution 6: Configure Trusted Directories (Advanced)

**Note:** This feature may require modification of grok-cli source code or future configuration support.

The grok-cli has a `SecurityManager` that controls trusted directories. In theory, you could:

1. **Modify the security policy** to add additional trusted directories
2. **Set environment variables** that specify additional allowed paths
3. **Add configuration options** to allow-list specific directories

#### Current Status
As of the current version, there's **no built-in configuration** to add arbitrary trusted directories outside the project root. This would require:

- Code modifications to `src/security/mod.rs`
- Adding configuration options to `config.toml`
- Security review to prevent abuse

#### Future Feature Request
Consider opening a GitHub issue requesting:
```markdown
Feature Request: Read-Only External Directory Access

Add configuration option to specify read-only directories:

[security.external_access]
allow_read_only = [
    "H:\\GitHub\\shared-configs",
    "H:\\Documents\\reference-docs"
]
```

---

## Best Practices

### 1. Use Project-Relative Structures
Organize your projects to minimize external dependencies:

```
H:\GitHub\
├── shared-configs/          ← Shared resources
│   ├── eslint.config.js
│   └── tsconfig.base.json
│
├── project-a/
│   ├── .config -> ../shared-configs/  ← Symlink
│   └── src/
│
└── project-b/
    ├── .config -> ../shared-configs/  ← Symlink
    └── src/
```

### 2. Document External Dependencies
Create a `EXTERNAL_DEPS.md` in your project:

```markdown
# External Dependencies

This project references the following external files:

- `shared-config.toml` → Symlink to `H:\GitHub\shared\config.toml`
- `reference-docs/` → Symlink to `H:\Documents\api-reference\`

To set up symlinks:
\`\`\`bash
mklink shared-config.toml H:\GitHub\shared\config.toml
mklink /D reference-docs H:\Documents\api-reference
\`\`\`
```

### 3. Add Setup Scripts
Create a `setup-symlinks.ps1` (Windows) or `setup-symlinks.sh` (Linux/macOS):

```powershell
# setup-symlinks.ps1
Write-Host "Creating symlinks for external dependencies..."

New-Item -ItemType SymbolicLink -Path ".\shared-config.toml" -Target "H:\GitHub\shared\config.toml" -Force
New-Item -ItemType SymbolicLink -Path ".\reference-docs" -Target "H:\Documents\api-reference" -Force

Write-Host "Done! External files are now accessible."
```

### 4. Keep External References Minimal
The fewer external dependencies, the easier your project is to work with:

- Prefer copying small, stable files into your project
- Use symlinks for large or frequently-changing files
- Document why each external dependency is needed

---

## Comparison Table

| Solution | Ease of Use | Sync Changes | Security | Best For |
|----------|-------------|--------------|----------|----------|
| **Symlinks** | ⭐⭐⭐ | ✅ Yes | ⭐⭐⭐ | Regular references |
| **Copy Files** | ⭐⭐⭐⭐ | ❌ No | ⭐⭐⭐⭐ | One-time needs |
| **Multiple Roots** | ⭐⭐⭐⭐ | ✅ Yes | ⭐⭐⭐⭐ | Zed editor users |
| **Copy-Paste** | ⭐⭐⭐⭐⭐ | ❌ No | ⭐⭐⭐⭐⭐ | Small snippets |
| **Terminal Tool** | ⭐⭐ | ✅ Yes | ⭐⭐ | Quick checks |
| **Config Trusted** | ⭐ | ✅ Yes | ⭐ | Future feature |

---

## Troubleshooting

### Symlinks Not Working?

**Windows:** You need administrator privileges to create symlinks.
```powershell
# Check if you have symlink privileges
whoami /priv | findstr SeCreateSymbolicLinkPrivilege

# Run PowerShell as Administrator
Start-Process powershell -Verb RunAs
```

**Enable Developer Mode** (Windows 10/11):
1. Settings → Update & Security → For Developers
2. Enable "Developer Mode"
3. Restart your terminal

### Symlink Shows as Regular File?

```bash
# Windows PowerShell - Check if it's a symlink
Get-Item .\external-config.toml | Select-Object LinkType, Target

# Linux/macOS
ls -la external-config.toml
```

### Symlink Breaks After Moving Project?

Symlinks use absolute paths. If you move the project or target files:
```powershell
# Windows - Recreate the symlink
Remove-Item .\external-config.toml
New-Item -ItemType SymbolicLink -Path ".\external-config.toml" -Target "NEW-PATH\config.toml"
```

### AI Still Can't Read External Files via Symlink?

This might be a security policy issue. Try:
1. Verify the symlink works: `type .\external-config.toml`
2. Check if the target file is readable
3. Ensure the symlink is inside the project root
4. Report as a bug if it should work but doesn't

---

## Feature Request Template

If you'd like built-in support for external file access, open a GitHub issue:

```markdown
## Feature Request: Configurable External Directory Access

### Problem
Currently, grok-cli restricts file access to project root directories only. 
This makes it difficult to reference shared configuration files or documentation 
stored outside the project.

### Proposed Solution
Add configuration option to allow read-only access to specified directories:

\`\`\`toml
[security.external_access]
# Allow read-only access to these directories
read_only_paths = [
    "H:\\GitHub\\shared-configs",
    "H:\\Documents\\api-docs"
]

# Require explicit approval for each external file access
require_approval = true
\`\`\`

### Use Cases
1. Shared configuration files across multiple projects
2. Reference documentation stored centrally
3. Reading API specifications from external sources

### Security Considerations
- Only read-only access (no write/execute)
- Explicit user configuration required
- Optional approval prompt for each access
- Logged for audit purposes

### Alternatives Considered
- Symlinks (current workaround, but requires admin on Windows)
- Copying files (creates duplication)
- Terminal tool (bypasses restrictions but less structured)
```

---

## Summary

While AI assistants are restricted to project directories for security reasons, you have several practical workarounds:

1. **Use symlinks** for regular external references (best option)
2. **Copy files** temporarily for one-off needs
3. **Use multiple project roots** in Zed editor
4. **Paste content** directly for small snippets
5. **Request terminal commands** to read external files
6. **Request a feature** for configurable external access

The symlink approach is the most elegant solution for most use cases, providing seamless access while maintaining security boundaries.

---

## Additional Resources

- [Grok CLI Configuration Guide](CONFIGURATION.md)
- [Security & Trust Documentation](../CONTRIBUTING.md#security)
- [Troubleshooting Guide](../README.md#troubleshooting)
- [Windows Symlink Documentation](https://docs.microsoft.com/en-us/windows/win32/fileio/symbolic-links)
- [Linux/macOS ln Command](https://man7.org/linux/man-pages/man1/ln.1.html)

---

**Author:** john mcconnell (john.microtech@gmail.com)  
**Repository:** https://github.com/microtech/grok-cli  
**Buy me a coffee:** https://buymeacoffee.com/micro.tech