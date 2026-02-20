# External Directory Access - Quick Start Guide

**Get started with external file access in 5 minutes!**

---

## What Is This?

External Directory Access lets you read files outside your project directory while maintaining security. Perfect for:
- Shared configuration files across projects
- Reference documentation stored elsewhere
- API specifications in a central location
- Team-wide settings and standards

---

## Quick Setup (3 Steps)

### Step 1: Enable the Feature

**Option A: Using Environment Variables** (Easiest)

Create or edit `.grok/.env` in your project:

```bash
GROK_EXTERNAL_ACCESS_ENABLED=true
GROK_EXTERNAL_ACCESS_PATHS="H:\GitHub\shared-configs,H:\Documents\api-docs"
GROK_EXTERNAL_ACCESS_REQUIRE_APPROVAL=true
```

**Option B: Using Configuration File**

Create or edit `.grok/config.toml` in your project:

```toml
[security.external_access]
enabled = true
require_approval = true
logging = true
allowed_paths = [
    "H:\\GitHub\\shared-configs",
    "H:\\Documents\\api-docs"
]
```

> **Windows Users:** Use double backslashes (`\\`) in TOML files

---

### Step 2: Verify Configuration

```bash
grok config validate-external-access
```

Expected output:
```
✓ External access is enabled

Allowed Paths:
  1. H:\GitHub\shared-configs (directory, exists, readable)
  2. H:\Documents\api-docs (directory, exists, readable)

Security Settings:
  Approval required: ✓ Enabled (recommended)
  Logging enabled: ✓ Enabled (recommended)
```

---

### Step 3: Try It Out!

Start Grok and ask to read an external file:

```bash
grok
> Can you read H:\GitHub\shared-configs\eslint.config.js?
```

You'll see an approval prompt:

```
┌─────────────────────────────────────────────────────────────┐
│ 🔒 External File Access Request                             │
├─────────────────────────────────────────────────────────────┤
│ Path: H:\GitHub\shared-configs\eslint.config.js             │
│ Type: Read                                                   │
├─────────────────────────────────────────────────────────────┤
│ [A]llow Once  [T]rust Always  [D]eny  [V]iew Path           │
└─────────────────────────────────────────────────────────────┘

Your choice: 
```

**Choose an option:**
- **A** (Allow Once) - Allow this one time
- **T** (Trust Always) - Skip prompts for rest of session
- **D** (Deny) - Block access
- **V** (View Path) - See file details first

---

## Common Usage Scenarios

### Scenario 1: Shared Team Configuration

**Setup:**
```bash
# .grok/.env
GROK_EXTERNAL_ACCESS_ENABLED=true
GROK_EXTERNAL_ACCESS_PATHS="H:\team\configs"
```

**Usage:**
```
> Can you read our team's ESLint config at H:\team\configs\eslint.js?
```

Choose **T** (Trust Always) to skip future prompts for this session.

---

### Scenario 2: API Documentation Reference

**Setup:**
```bash
# .grok/.env
GROK_EXTERNAL_ACCESS_ENABLED=true
GROK_EXTERNAL_ACCESS_PATHS="H:\docs\api-specs"
```

**Usage:**
```
> Can you explain the OpenAPI spec at H:\docs\api-specs\openapi.yaml?
```

---

### Scenario 3: Cross-Project References

**Setup:**
```bash
# .grok/.env
GROK_EXTERNAL_ACCESS_ENABLED=true
GROK_EXTERNAL_ACCESS_PATHS="H:\GitHub\other-project"
```

**Usage:**
```
> Can you compare this file with H:\GitHub\other-project\src\utils.ts?
```

---

## Understanding the Approval Options

### [A] Allow Once
- Reads the file this one time
- Will prompt again if you access the same file later
- **Best for:** Trying things out, one-time checks

### [T] Trust Always (Recommended)
- Adds path to session-trusted list
- No more prompts for this file during your session
- Trust is **not saved** - resets when you restart
- **Best for:** Files you'll access multiple times

### [D] Deny
- Blocks access to this file
- AI receives "access denied" error
- You can try again later if you change your mind
- **Best for:** Sensitive files, mistakes

### [V] View Path
- Shows file details before deciding:
  - Canonical path (resolved symlinks)
  - Parent directory
  - File size and type
  - Existence check
- Returns to approval prompt after viewing
- **Best for:** Verifying you're accessing the right file

---

## Security: What's Protected?

These files are **automatically blocked** even if in allowed paths:

- `**/.env` - Environment variables
- `**/.ssh/**` - SSH keys
- `**/*.key`, `**/*.pem` - Private keys
- `**/password*`, `**/secret*` - Credentials
- `**/.aws/**`, `**/.azure/**` - Cloud credentials

You'll see: `"Path matches excluded pattern (security protection)"`

---

## Troubleshooting

### Issue: "External access is disabled"

**Solution:** Enable it in `.grok/.env`:
```bash
GROK_EXTERNAL_ACCESS_ENABLED=true
```

---

### Issue: "Path is not in allowed external paths"

**Solution:** Add the directory to allowed paths:
```bash
GROK_EXTERNAL_ACCESS_PATHS="H:\path1,H:\path2,H:\path3"
```

> **Tip:** Use the parent directory, not individual files

---

### Issue: "Access denied: Path matches excluded pattern"

This is **intentional security protection** for sensitive files.

**Solution:** 
- If it's a false positive, remove the pattern from config
- If it's actually sensitive, use a different approach (copy file temporarily)

---

### Issue: No prompt appears

**Check:**
1. Is `require_approval = false` in your config?
2. Was this path already trusted this session?

**Solution:**
- Set `GROK_EXTERNAL_ACCESS_REQUIRE_APPROVAL=true` to enable prompts
- Restart Grok to clear session trust

---

### Issue: Symlink on Windows requires admin

**Solutions:**
1. **Enable Developer Mode** (no admin needed):
   - Settings → Update & Security → For Developers
   - Enable "Developer Mode"
   - Restart terminal

2. **Use Directory Junctions** (no admin needed):
   ```cmd
   mklink /J link-name H:\path\to\directory
   ```

3. **Just copy the file** (simplest):
   ```cmd
   copy H:\external\file.txt .\temp-file.txt
   ```

---

## Audit Your Usage

### View Recent Access

```bash
grok audit external-access --count 20
```

### See Statistics

```bash
grok audit external-access --summary
```

Output:
```
Overall Statistics:
  Total Requests:     15
  Allowed:            12 (80.0%)
  Denied:             3 (20.0%)

Most Accessed Paths:
  1. H:\shared\eslint.config.js (8 times)
  2. H:\docs\api.yaml (4 times)
```

### Export Monthly Report

```bash
grok audit external-access \
  --from 2024-01-01 \
  --to 2024-01-31 \
  --export january_report.csv
```

---

## Configuration Reference

### All Environment Variables

```bash
# Feature toggle
GROK_EXTERNAL_ACCESS_ENABLED=true

# Allowed directories (comma-separated)
GROK_EXTERNAL_ACCESS_PATHS="H:\path1,H:\path2"

# Require approval prompts (true/false)
GROK_EXTERNAL_ACCESS_REQUIRE_APPROVAL=true

# Enable audit logging (true/false)
GROK_EXTERNAL_ACCESS_LOGGING=true
```

### All TOML Options

```toml
[security.external_access]
enabled = true
require_approval = true
logging = true

allowed_paths = [
    "H:\\GitHub\\shared-configs",
    "H:\\Documents\\api-docs"
]

# Optional: Override default excluded patterns
excluded_patterns = [
    "**/.env",
    "**/.ssh/**",
    "**/*.key"
]
```

---

## Advanced Usage

### Auto-Approve (No Prompts)

**Use with caution!** Only for trusted paths.

```bash
GROK_EXTERNAL_ACCESS_REQUIRE_APPROVAL=false
```

Now access is automatic without prompts.

---

### Custom Exclusion Patterns

Add your own sensitive patterns:

```toml
[security.external_access]
excluded_patterns = [
    "**/.env",           # Default
    "**/.ssh/**",        # Default
    "**/company-secrets/**",  # Custom
    "**/internal-only/**"     # Custom
]
```

---

### Filter Audit Logs

**By date range:**
```bash
grok audit external-access --from 2024-01-01 --to 2024-01-31
```

**By specific path:**
```bash
grok audit external-access --path "H:\shared\config.toml"
```

**Last 50 entries:**
```bash
grok audit external-access --count 50
```

---

## Best Practices

### ✅ Do This

- **Start with `require_approval = true`** - Stay aware of what's accessed
- **Use specific parent directories** - Don't allow entire drives
- **Enable logging** - Keep audit trail for compliance
- **Use "Trust Always"** - Save time on repeated access
- **Review audit logs** - Check what's being accessed monthly
- **Document your setup** - Add comment to .grok/.env explaining why

### ❌ Avoid This

- **Don't disable approval without understanding risks**
- **Don't add root directories** (C:\, /home, etc.)
- **Don't add sensitive directories** (.ssh, .aws, etc.)
- **Don't commit .grok/.env to git** - Keep it local
- **Don't ignore denied access** - Understand why it was blocked

---

## What's Next?

### Learn More
- **Complete Guide:** `Doc/EXTERNAL_FILE_REFERENCE.md`
- **Security Details:** `Doc/PROPOSAL_EXTERNAL_ACCESS.md`
- **Decision Tree:** `Doc/EXTERNAL_ACCESS_DECISION_TREE.md`
- **Test Plan:** `.zed/EXTERNAL_ACCESS_TEST_PLAN.md`

### Share Feedback
- GitHub Issues: https://github.com/microtech/grok-cli/issues
- Discussions: https://github.com/microtech/grok-cli/discussions
- Email: john.microtech@gmail.com

### Support the Project
- ⭐ Star on GitHub
- 📝 Report bugs or suggest improvements
- ☕ Buy me a coffee: https://buymeacoffee.com/micro.tech

---

## Quick Commands Cheat Sheet

```bash
# Setup
echo 'GROK_EXTERNAL_ACCESS_ENABLED=true' > .grok/.env
echo 'GROK_EXTERNAL_ACCESS_PATHS="H:\shared"' >> .grok/.env

# Validate
grok config validate-external-access

# Use
grok
> Can you read H:\shared\file.txt?

# Audit
grok audit external-access --summary
grok audit external-access --count 20
grok audit external-access --export report.csv

# Clear logs
grok audit clear --confirm
```

---

## Example: Complete Workflow

### 1. Project Setup
```bash
cd H:\GitHub\my-project

# Create configuration
cat > .grok/.env << EOF
GROK_EXTERNAL_ACCESS_ENABLED=true
GROK_EXTERNAL_ACCESS_PATHS="H:\GitHub\shared-configs,H:\team\docs"
GROK_EXTERNAL_ACCESS_REQUIRE_APPROVAL=true
GROK_EXTERNAL_ACCESS_LOGGING=true
EOF

# Validate
grok config validate-external-access
```

### 2. First Use
```bash
grok
> Can you read our team's ESLint config at H:\GitHub\shared-configs\eslint.js?

[Approval prompt]
Your choice: T  # Trust Always

✓ Path trusted for this session
AI: [Shows ESLint configuration]
```

### 3. Continue Working
```bash
> Now check the TypeScript config in the same directory

[No prompt - automatically approved via session trust]
AI: [Shows TypeScript configuration]
```

### 4. Weekly Audit
```bash
# See what was accessed this week
grok audit external-access --from 2024-01-15 --summary

# Export for records
grok audit external-access --from 2024-01-15 --export weekly_audit.csv
```

---

## Success! 🎉

You're now set up to access external files securely!

**Remember:**
- Configuration in `.grok/.env` or `.grok/config.toml`
- Use "Trust Always" for files you access often
- Review audit logs periodically
- Keep sensitive files excluded

**Need help?** Check the troubleshooting section or full documentation.

---

**Author:** john mcconnell (john.microtech@gmail.com)  
**Repository:** https://github.com/microtech/grok-cli  
**Version:** 1.0 - Feature Complete  
**Last Updated:** 2024