# Security Guide

This document outlines the security features and best practices for Grok CLI, with a focus on shell command execution safety.

## Overview

Grok CLI implements a comprehensive permission system for shell commands executed through the `!` prefix in interactive mode. This system is inspired by Gemini CLI's security model and provides multiple layers of protection against accidental or malicious command execution.

## Shell Command Permission System

### How It Works

When you execute a shell command using `!` in interactive mode, the following security checks are performed:

1. **Blocklist Check** - Commands are checked against a blocklist of dangerous operations
2. **Allowlist Check** - Previously approved commands are automatically allowed
3. **User Prompt** - New commands require explicit user approval
4. **Persistent Policy** - Approved commands can be saved permanently

### Approval Modes

#### Default Mode (Recommended)

```bash
GROK_SHELL_APPROVAL_MODE=default
```

In default mode, every new command requires explicit user approval:

```
⚠️  Shell command requires permission: git status

  a) Allow once
  s) Allow always (this session)
  p) Allow always (save permanently)
  d) Deny

Choose [a/s/p/d]:
```

**Options:**
- **a (Allow once)** - Execute this command once without saving permission
- **s (Session)** - Allow this command for the current session only
- **p (Permanent)** - Save permission permanently to `~/.grok/shell_policy.json`
- **d (Deny)** - Reject the command

#### YOLO Mode (⚠️ DANGEROUS!)

```bash
GROK_SHELL_APPROVAL_MODE=yolo
```

**NOT RECOMMENDED** - All non-blocked commands execute without prompts. Use only in trusted, isolated environments.

### Blocklist

The following commands are automatically blocked for safety:

#### Destructive File Operations
- `rm` - Remove files/directories
- `del` - Delete files (Windows)
- `rmdir` / `rd` - Remove directories
- `format` - Format drives
- `fdisk` - Disk partitioning
- `mkfs` - Make filesystem
- `dd` - Disk operations (can overwrite data)

#### System Operations
- `shutdown` - Shut down the system
- `reboot` - Reboot the system
- `halt` - Halt the system
- `poweroff` - Power off the system
- `init` - Change system runlevel

#### Package Management
- `apt-get` - APT package manager
- `yum` - YUM package manager
- `dnf` - DNF package manager
- `pacman` - Pacman package manager

#### Dangerous Patterns
- `rm -rf /` - Recursive root deletion
- `del /s /q` - Recursive Windows deletion
- `:(){ :|:& };:` - Fork bomb

**Note:** These commands are blocked even in YOLO mode.

### Allowlist Management

#### Session Allowlist

Temporary permissions that last only for the current session:
- Stored in memory
- Cleared when you exit Grok CLI
- Useful for frequently used commands during development

#### Persistent Allowlist

Permanent permissions saved to disk:
- Stored in `~/.grok/shell_policy.json`
- Persists across sessions
- Automatically loaded on startup

**View your persistent allowlist:**
```bash
cat ~/.grok/shell_policy.json
```

**Example policy file:**
```json
{
  "allowed_commands": [
    "git",
    "ls",
    "cat",
    "cargo"
  ]
}
```

#### Managing Allowlist

**Clear session allowlist:**
Exit and restart Grok CLI

**Clear persistent allowlist:**
Delete or edit `~/.grok/shell_policy.json`:
```bash
rm ~/.grok/shell_policy.json
```

### Command Parsing

The permission system extracts the root command from complex shell expressions:

```bash
!git status | grep modified     # Root command: "git"
!ls -la && cat file.txt          # Root command: "ls"
!echo "test" > output.txt        # Root command: "echo"
```

Once approved, all variations of that root command are allowed:
```bash
!git status      # Approved
!git commit      # Also allowed (same root: "git")
!git push        # Also allowed (same root: "git")
```

## Best Practices

### 1. Use Default Mode

Always use `default` approval mode unless you have a specific reason not to:

```bash
# In .env
GROK_SHELL_APPROVAL_MODE=default
```

### 2. Review Commands Carefully

Before approving a command:
- Read the full command carefully
- Verify the command does what you expect
- Check for typos or unexpected arguments
- Be cautious with commands that modify files or system state

### 3. Use Session Allowlist for Development

During active development:
- Use "s" (session) for temporary convenience
- Avoids permanent allowlist bloat
- Resets when you start a new session

### 4. Be Selective with Permanent Allowlist

Only add to permanent allowlist:
- Commands you use frequently
- Commands you fully trust
- Commands with predictable, safe behavior

**Good candidates:**
```bash
!git status
!ls -la
!cat somefile.txt
!cargo check
```

**Poor candidates:**
```bash
!rm -rf build/      # Destructive
!sudo systemctl     # Requires elevated privileges
!curl | bash        # Arbitrary code execution
```

### 5. Regular Allowlist Audits

Periodically review your persistent allowlist:

```bash
cat ~/.grok/shell_policy.json
```

Remove commands you no longer use:
```bash
# Edit the file manually or delete and rebuild
rm ~/.grok/shell_policy.json
```

### 6. Project-Specific Policies

For projects with specific security requirements:

**Option 1:** Use session allowlist only
```
Don't use "p" (permanent), only use "s" (session)
```

**Option 2:** Document approved commands
Create `.grok/APPROVED_COMMANDS.md` in your project:
```markdown
# Approved Shell Commands

This project has approved these commands:
- git (version control)
- cargo (build system)
- npm test (testing)
```

## Threat Model

### What This Protects Against

✅ **Accidental dangerous commands**
- Typos in destructive commands
- Copy-paste errors
- Muscle memory mistakes

✅ **Novice user errors**
- Running unfamiliar commands
- Not understanding command implications
- Learning without risk

✅ **AI suggestion mistakes**
- AI recommending unsafe commands
- Misunderstanding context
- Edge case failures

### What This Does NOT Protect Against

❌ **Malicious intent**
- A determined attacker can bypass these controls
- User can still approve dangerous commands
- Not a substitute for proper system security

❌ **Complex command exploitation**
- Sophisticated command injection
- Shell escape sequences
- Command obfuscation

❌ **Social engineering**
- Users can be tricked into approving bad commands
- No protection against user approval

❌ **System-level vulnerabilities**
- OS-level security issues
- Kernel vulnerabilities
- Hardware exploits

## Configuration Examples

### Development Environment

```bash
# .grok/.env (project root)
GROK_SHELL_APPROVAL_MODE=default
```

**Workflow:**
- Review each command
- Use session allowlist (s) for current work
- Don't save permanently during experimentation

### Production/CI Environment

```bash
# .env
GROK_SHELL_APPROVAL_MODE=yolo
```

**⚠️ Only if:**
- Running in isolated container
- Commands are pre-vetted
- No interactive input possible
- Failures are acceptable/recoverable

**Better approach for CI:**
Use explicit commands instead of Grok CLI interactive mode.

### Trusted Development Machine

```bash
# ~/.grok/.env (system-wide)
GROK_SHELL_APPROVAL_MODE=default

# Plus persistent allowlist at ~/.grok/shell_policy.json
{
  "allowed_commands": [
    "git",
    "ls",
    "cat",
    "grep",
    "find",
    "cargo",
    "npm",
    "yarn"
  ]
}
```

**Rationale:**
- Frequent commands pre-approved
- Unusual commands still prompt
- Balance of convenience and safety

## Troubleshooting

### Command Always Blocked

**Problem:** A safe command is being blocked

**Solutions:**

1. Check if it's actually on the blocklist
2. Use a different command variant
3. If absolutely needed, modify the source code (not recommended)

**Example:**
```bash
# Instead of using blocked "rm"
!mv file.txt ~/.trash/    # Move to trash instead
!rm file.txt              # Blocked

# Or use safe alternatives
!trash file.txt           # If trash-cli is installed
```

### Permission Prompt Not Appearing

**Problem:** Commands execute without prompting

**Check:**
1. Verify approval mode: `echo $GROK_SHELL_APPROVAL_MODE`
2. Check if command is already in allowlist
3. Ensure you're in interactive mode

### Reset All Permissions

**To start fresh:**

```bash
# Remove persistent allowlist
rm ~/.grok/shell_policy.json

# Restart Grok CLI (clears session allowlist)
exit
grok
```

### Allowlist Not Persisting

**Problem:** Approved commands don't save

**Check:**
1. Ensure you used "p" (permanent), not "s" (session)
2. Verify write permissions: `ls -la ~/.grok/`
3. Check for filesystem errors
4. Try creating the file manually:
   ```bash
   mkdir -p ~/.grok
   echo '{"allowed_commands":[]}' > ~/.grok/shell_policy.json
   ```

## Reporting Security Issues

If you discover a security vulnerability in Grok CLI:

1. **DO NOT** open a public GitHub issue
2. Email security concerns to: john.microtech@gmail.com
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We take security seriously and will respond promptly.

## Additional Security Measures

### Beyond Shell Commands

1. **API Key Protection**
   - Never commit `.env` files
   - Use `.env.example` for templates
   - Set restrictive file permissions: `chmod 600 .env`

2. **Network Security**
   - All API calls use HTTPS
   - Built-in retry logic for network failures
   - Timeout protection against hanging requests

3. **Data Privacy**
   - Conversations are not logged by default
   - Telemetry is opt-in only
   - API keys are redacted from logs

4. **Dependency Security**
   - Regular `cargo audit` runs
   - Minimal dependency footprint
   - Vetted crates only

### System Permissions

Grok CLI runs with your user permissions. To limit risk:

```bash
# Create a restricted user for testing (Linux)
sudo useradd -m -s /bin/bash groktest
sudo su - groktest

# Run Grok CLI as limited user
grok
```

## Security Checklist

Before using Grok CLI in production:

- [ ] Review and understand approval modes
- [ ] Configure `GROK_SHELL_APPROVAL_MODE` appropriately
- [ ] Audit persistent allowlist regularly
- [ ] Protect `.env` files (`.gitignore`, file permissions)
- [ ] Use session allowlist for temporary needs
- [ ] Document approved commands for your project
- [ ] Train team members on security features
- [ ] Set up monitoring for unexpected command execution
- [ ] Test in isolated environment first
- [ ] Have rollback plan for accidents

## References

- [Grok CLI Configuration Guide](CONFIGURATION.md)
- [Interactive Mode Guide](docs/INTERACTIVE.md)
- [Quick Start Guide](QUICKSTART.md)

## License

This software is provided as-is with no warranty. Use at your own risk.

---

**Remember:** The best security is informed, careful usage. No system can protect against user approval of dangerous commands. Always think before you approve!