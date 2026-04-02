# Proposal: Configurable External Directory Access

**Status:** Draft  
**Author:** john mcconnell (john.microtech@gmail.com)  
**Repository:** https://github.com/microtech/grok-cli  
**Date:** 2024

---

## Executive Summary

This proposal outlines a feature to allow controlled, read-only access to files and directories outside the current project boundaries. This enhancement maintains security while providing flexibility for users who need to reference shared configurations, documentation, or resources stored in external locations.

---

## Problem Statement

### Current Limitation

Grok CLI restricts file access to project root directories for security reasons. While this is a sound default policy, it creates friction for legitimate use cases:

1. **Shared Configuration Files**: Teams often maintain shared ESLint, TypeScript, or other configuration files in a central location
2. **Reference Documentation**: API specifications, internal wikis, or documentation stored outside project boundaries
3. **Multi-Project Workflows**: Developers working on related projects need to reference files across project boundaries
4. **Monorepo Scenarios**: Large codebases with multiple sub-projects that need cross-references

### Current Workarounds

Users currently must resort to:
- **Symlinks**: Requires administrator privileges on Windows; not always obvious to team members
- **File Copying**: Creates duplication and sync issues
- **Terminal Commands**: Bypasses restrictions but lacks structure and safety
- **Manual Copy-Paste**: Error-prone and not scalable

### User Pain Points

From user feedback:
> "I'm working on a project and want to reference files outside of the project. Grok can't read files outside the project (it's confined to the project). Is there a way we can add read-only OR approve tool use outside the project?"

---

## Proposed Solution

### Overview

Add a configuration system that allows users to explicitly grant read-only access to specific external directories. This feature must:

1. **Be opt-in**: Disabled by default, requiring explicit user configuration
2. **Be auditable**: All external access attempts are logged
3. **Be controllable**: Users can require approval for each access or auto-approve trusted paths
4. **Be secure**: Read-only access only; no write, execute, or delete operations
5. **Be transparent**: Clear indication when external files are accessed

### User Experience

#### Configuration

Users can configure external access in their `.grok/.env` or `config.toml` files:

**Option 1: Environment Variables (.env)**
```bash
# Enable external directory access
GROK_EXTERNAL_ACCESS_ENABLED=true

# Comma-separated list of allowed paths
GROK_EXTERNAL_ACCESS_PATHS="H:\GitHub\shared-configs,H:\Documents\api-docs"

# Require approval for each external file access
GROK_EXTERNAL_ACCESS_REQUIRE_APPROVAL=true

# Log all external access attempts
GROK_EXTERNAL_ACCESS_LOGGING=true
```

**Option 2: TOML Configuration**
```toml
[security.external_access]
# Enable external directory access feature
enabled = true

# Require approval prompt for each external file access
require_approval = true

# Log all external access attempts
logging = true

# Read-only access to these directories
allowed_paths = [
    "H:\\GitHub\\shared-configs",
    "H:\\Documents\\api-docs",
    "C:\\Users\\YourName\\reference-materials"
]

# Optional: Patterns to exclude even within allowed paths
excluded_patterns = [
    "**/.env",           # Never read .env files
    "**/.git/**",        # Never read git internals
    "**/node_modules/**", # Skip node_modules
    "**/*.key",          # Skip key files
    "**/*.pem"           # Skip certificates
]
```

#### Interactive Approval

When `require_approval = true`, users see:

```
┌─────────────────────────────────────────────────────────────┐
│ 🔒 External File Access Request                             │
├─────────────────────────────────────────────────────────────┤
│ Path: H:\GitHub\shared-configs\eslint.config.js             │
│ Type: Read                                                   │
│ Reason: Requested by AI assistant                           │
├─────────────────────────────────────────────────────────────┤
│ This path is OUTSIDE your project directory.                │
│ External access is configured in: .grok/.env                │
├─────────────────────────────────────────────────────────────┤
│ [A]llow Once  [T]rust Always  [D]eny  [V]iew Path           │
└─────────────────────────────────────────────────────────────┘
```

Options:
- **Allow Once**: Permit this specific access, ask again next time
- **Trust Always**: Add to trusted paths (for this session)
- **Deny**: Reject the access
- **View Path**: Show resolved canonical path and verify location

---

## Technical Implementation

### Phase 1: Core Infrastructure (v0.2.0)

#### 1.1 Configuration Schema

**File:** `src/config.rs`

Add new configuration structure:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAccessConfig {
    /// Enable external directory access feature
    #[serde(default)]
    pub enabled: bool,

    /// Require user approval for each external file access
    #[serde(default = "default_require_approval")]
    pub require_approval: bool,

    /// Log all external access attempts
    #[serde(default = "default_true")]
    pub logging: bool,

    /// List of allowed external paths (absolute paths)
    #[serde(default)]
    pub allowed_paths: Vec<PathBuf>,

    /// Patterns to exclude even within allowed paths
    #[serde(default)]
    pub excluded_patterns: Vec<String>,

    /// Session-only trusted paths (not persisted)
    #[serde(skip)]
    pub session_trusted_paths: Arc<Mutex<Vec<PathBuf>>>,
}

fn default_require_approval() -> bool { true }
fn default_true() -> bool { true }
```

#### 1.2 Security Policy Enhancement

**File:** `src/acp/security.rs`

Extend `SecurityPolicy` to handle external access:

```rust
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    trusted_directories: Vec<PathBuf>,
    working_directory: PathBuf,
    external_access_config: ExternalAccessConfig,
}

impl SecurityPolicy {
    /// Check if a path is within project boundaries
    pub fn is_internal_path<P: AsRef<Path>>(&self, path: P) -> bool {
        let resolved = match self.resolve_path(path) {
            Ok(p) => p,
            Err(_) => return false,
        };

        self.trusted_directories
            .iter()
            .any(|trusted| resolved.starts_with(trusted))
    }

    /// Check if external access is allowed for a path
    pub fn is_external_access_allowed<P: AsRef<Path>>(&self, path: P) -> ExternalAccessResult {
        // If external access is disabled, deny
        if !self.external_access_config.enabled {
            return ExternalAccessResult::Denied(
                "External access is disabled".to_string()
            );
        }

        let resolved = match self.resolve_path(&path) {
            Ok(p) => p,
            Err(e) => return ExternalAccessResult::Denied(
                format!("Cannot resolve path: {}", e)
            ),
        };

        // Check if path matches excluded patterns
        if self.is_path_excluded(&resolved) {
            return ExternalAccessResult::Denied(
                "Path matches excluded pattern".to_string()
            );
        }

        // Check if path is in allowed external paths
        let is_allowed = self.external_access_config.allowed_paths
            .iter()
            .any(|allowed| resolved.starts_with(allowed));

        if !is_allowed {
            return ExternalAccessResult::Denied(
                "Path is not in allowed external paths".to_string()
            );
        }

        // Check if approval is required
        if self.external_access_config.require_approval {
            ExternalAccessResult::RequiresApproval(resolved)
        } else {
            ExternalAccessResult::Allowed(resolved)
        }
    }

    /// Check if path matches any excluded pattern
    fn is_path_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.external_access_config.excluded_patterns
            .iter()
            .any(|pattern| {
                // Use glob matching
                glob::Pattern::new(pattern)
                    .map(|p| p.matches(&path_str))
                    .unwrap_or(false)
            })
    }

    /// Combined path validation (internal or external)
    pub fn validate_path_access<P: AsRef<Path>>(
        &self, 
        path: P
    ) -> Result<PathAccessType> {
        let path_ref = path.as_ref();

        // First check if it's internal (project paths)
        if self.is_internal_path(path_ref) {
            return Ok(PathAccessType::Internal(self.resolve_path(path_ref)?));
        }

        // Not internal, check external access
        match self.is_external_access_allowed(path_ref) {
            ExternalAccessResult::Allowed(resolved) => {
                Ok(PathAccessType::External(resolved))
            }
            ExternalAccessResult::RequiresApproval(resolved) => {
                Ok(PathAccessType::ExternalRequiresApproval(resolved))
            }
            ExternalAccessResult::Denied(reason) => {
                Err(anyhow!("Access denied: {}", reason))
            }
        }
    }
}

#[derive(Debug)]
pub enum ExternalAccessResult {
    Allowed(PathBuf),
    RequiresApproval(PathBuf),
    Denied(String),
}

#[derive(Debug)]
pub enum PathAccessType {
    Internal(PathBuf),
    External(PathBuf),
    ExternalRequiresApproval(PathBuf),
}
```

#### 1.3 Approval UI

**File:** `src/cli/approval.rs` (new)

```rust
use std::io::{self, Write};
use std::path::Path;
use console::{style, Term};
use anyhow::Result;

pub enum ApprovalDecision {
    AllowOnce,
    TrustAlways,
    Deny,
}

pub fn prompt_external_access_approval<P: AsRef<Path>>(
    path: P,
    config_source: &str,
) -> Result<ApprovalDecision> {
    let term = Term::stdout();
    let path = path.as_ref();

    term.clear_screen()?;
    
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ {} External File Access Request                             │", 
        style("🔒").yellow());
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ Path: {}│", 
        style(path.display()).cyan());
    println!("│ Type: {}                                                   │", 
        style("Read").green());
    println!("│ Reason: {}                          │", 
        style("Requested by AI assistant").dim());
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ This path is OUTSIDE your project directory.                │");
    println!("│ External access is configured in: {}                 │", 
        style(config_source).cyan());
    println!("├─────────────────────────────────────────────────────────────┤");
    print!("│ [{}]llow Once  [{}]rust Always  [{}]eny  [{}]iew Path           │\n", 
        style("A").green().bold(),
        style("T").green().bold(),
        style("D").red().bold(),
        style("V").blue().bold());
    println!("└─────────────────────────────────────────────────────────────┘");
    
    loop {
        print!("\nYour choice: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        match input.trim().to_lowercase().as_str() {
            "a" | "allow" => return Ok(ApprovalDecision::AllowOnce),
            "t" | "trust" => return Ok(ApprovalDecision::TrustAlways),
            "d" | "deny" => return Ok(ApprovalDecision::Deny),
            "v" | "view" => {
                println!("\n{}", style("Canonical Path:").bold());
                println!("  {}", path.canonicalize()?.display());
                println!("\n{}", style("Press any key to continue...").dim());
                let _ = term.read_char()?;
                continue;
            }
            _ => {
                println!("{}", style("Invalid choice. Please enter A, T, D, or V.").red());
                continue;
            }
        }
    }
}
```

#### 1.4 Integration with ACP Tools

**File:** `src/acp/tools/read_file.rs`

Update the read_file tool to handle external access:

```rust
use crate::acp::security::{SecurityManager, PathAccessType};
use crate::cli::approval::{prompt_external_access_approval, ApprovalDecision};

pub async fn handle_read_file(
    params: &ReadFileParams,
    security: &SecurityManager,
) -> Result<String> {
    let policy = security.get_policy();
    
    // Validate path access
    match policy.validate_path_access(&params.path)? {
        PathAccessType::Internal(resolved_path) => {
            // Normal internal file access
            read_file_internal(resolved_path).await
        }
        PathAccessType::External(resolved_path) => {
            // External access allowed without approval
            if policy.external_access_config.logging {
                log_external_access(&resolved_path, "allowed");
            }
            read_file_external(resolved_path).await
        }
        PathAccessType::ExternalRequiresApproval(resolved_path) => {
            // External access requires user approval
            let config_source = detect_config_source();
            let decision = prompt_external_access_approval(
                &resolved_path,
                &config_source
            )?;
            
            match decision {
                ApprovalDecision::AllowOnce => {
                    if policy.external_access_config.logging {
                        log_external_access(&resolved_path, "approved_once");
                    }
                    read_file_external(resolved_path).await
                }
                ApprovalDecision::TrustAlways => {
                    // Add to session trusted paths
                    security.add_session_trusted_path(&resolved_path);
                    if policy.external_access_config.logging {
                        log_external_access(&resolved_path, "approved_always");
                    }
                    read_file_external(resolved_path).await
                }
                ApprovalDecision::Deny => {
                    if policy.external_access_config.logging {
                        log_external_access(&resolved_path, "denied");
                    }
                    Err(anyhow!("External file access denied by user"))
                }
            }
        }
    }
}

fn log_external_access(path: &Path, decision: &str) {
    info!(
        "External file access: path={}, decision={}",
        path.display(),
        decision
    );
}
```

### Phase 2: Enhanced Features (v0.3.0)

#### 2.1 Access Logging and Audit Trail

**File:** `src/security/audit.rs` (new)

```rust
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalAccessLog {
    pub timestamp: DateTime<Utc>,
    pub path: PathBuf,
    pub operation: String, // "read", "list", etc.
    pub decision: String,  // "allowed", "approved_once", "denied"
    pub user: String,
    pub session_id: String,
}

pub struct AuditLogger {
    log_file: PathBuf,
}

impl AuditLogger {
    pub fn new() -> Result<Self> {
        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".grok")
            .join("audit");
        
        std::fs::create_dir_all(&log_dir)?;
        
        Ok(Self {
            log_file: log_dir.join("external_access.jsonl"),
        })
    }

    pub fn log_access(&self, log: ExternalAccessLog) -> Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)?;

        let json = serde_json::to_string(&log)?;
        writeln!(file, "{}", json)?;
        
        Ok(())
    }

    pub fn get_recent_logs(&self, count: usize) -> Result<Vec<ExternalAccessLog>> {
        use std::io::{BufRead, BufReader};
        use std::fs::File;

        let file = File::open(&self.log_file)?;
        let reader = BufReader::new(file);
        
        let logs: Vec<ExternalAccessLog> = reader
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();

        Ok(logs.into_iter().rev().take(count).collect())
    }
}
```

#### 2.2 Configuration Validation

Add validation command:

```bash
grok config validate-external-access

# Output:
✓ External access is enabled
✓ 3 allowed paths configured:
  • H:\GitHub\shared-configs (exists, readable)
  • H:\Documents\api-docs (exists, readable)
  • C:\Users\YourName\reference (⚠️  does not exist)

✓ 5 excluded patterns configured
✓ Approval required: Yes
✓ Logging enabled: Yes

Recommendations:
  ⓘ Remove non-existent path: C:\Users\YourName\reference
  ⓘ Consider adding: **/.ssh/** to excluded patterns
```

#### 2.3 Access Statistics

```bash
grok audit external-access --summary

# Output:
External Access Summary (Last 30 Days)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total Requests:     45
Allowed:            32 (71%)
Denied:             13 (29%)

Most Accessed Paths:
  1. H:\GitHub\shared-configs\eslint.config.js (12 times)
  2. H:\Documents\api-docs\openapi.yaml (8 times)
  3. H:\GitHub\shared-configs\tsconfig.json (7 times)

Recent Denials:
  • H:\Users\YourName\.ssh\id_rsa (excluded pattern)
  • H:\Windows\System32\config (not in allowed paths)
```

### Phase 3: Advanced Features (v0.4.0)

#### 3.1 Path Aliases

```toml
[security.external_access]
enabled = true

# Define aliases for commonly used external paths
[security.external_access.aliases]
shared = "H:\\GitHub\\shared-configs"
docs = "H:\\Documents\\api-docs"
vendor = "C:\\vendor\\libraries"

# Use aliases in allowed_paths
allowed_paths = ["${shared}", "${docs}", "${vendor}"]
```

Usage:
```
Can you read the config from @shared/eslint.config.js?
```

#### 3.2 Temporary Access Grants

```bash
# Grant temporary access (expires after session or time limit)
grok grant-access H:\GitHub\temp-project --duration 1h

# Output:
✓ Temporary access granted
  Path: H:\GitHub\temp-project
  Expires: 2024-01-15 14:30:00
  Session: abc123
```

#### 3.3 Team Sharing

`.grok/.env.shared` (checked into version control):
```bash
# Shared external access configuration for team
GROK_EXTERNAL_ACCESS_PATHS="./external-libs,../shared-configs"
GROK_EXTERNAL_ACCESS_REQUIRE_APPROVAL=true
```

---

## Security Considerations

### Threat Model

**Threats Mitigated:**
1. ✅ Accidental exposure of sensitive files (via excluded patterns)
2. ✅ Unauthorized file system traversal (explicit allow-list)
3. ✅ Silent file access (approval prompts and logging)
4. ✅ Persistent access without review (session-only trusted paths)

**Threats Remaining:**
1. ⚠️  User misconfiguration (allowing sensitive directories)
2. ⚠️  Social engineering (AI convincing user to approve)
3. ⚠️  Symlink attacks (mitigated by path canonicalization)

### Default-Deny Philosophy

- Feature is **disabled by default**
- Approval is **required by default**
- No paths are **trusted by default**
- All access is **logged by default**

### Excluded Patterns (Recommended Defaults)

```toml
excluded_patterns = [
    "**/.env",
    "**/.env.*",
    "**/.git/**",
    "**/.ssh/**",
    "**/*.key",
    "**/*.pem",
    "**/*.p12",
    "**/*.pfx",
    "**/id_rsa*",
    "**/password*",
    "**/secret*",
    "**/.aws/**",
    "**/.azure/**",
]
```

### Read-Only Enforcement

- File operations limited to: `read`, `list`, `grep`
- Write operations explicitly blocked: `write`, `edit`, `delete`, `move`
- Execute operations blocked: `terminal` with external file paths

---

## User Documentation

### Quick Start Guide

**File:** `Doc/EXTERNAL_ACCESS_QUICK_START.md`

```markdown
# External Access Quick Start

## Enable External Access

1. Create or edit `.grok/.env`:
   ```bash
   GROK_EXTERNAL_ACCESS_ENABLED=true
   GROK_EXTERNAL_ACCESS_PATHS="H:\GitHub\shared,C:\docs"
   ```

2. Test the configuration:
   ```bash
   grok config validate-external-access
   ```

3. Try accessing an external file:
   ```bash
   grok
   > Can you read H:\GitHub\shared\config.toml?
   ```

## Security Tips

- Use specific paths, not root directories
- Always use `require_approval=true` initially
- Review audit logs periodically
- Add sensitive patterns to `excluded_patterns`
```

---

## Implementation Timeline

### Milestone 1: MVP (4-6 weeks)
- [ ] Configuration schema
- [ ] Security policy updates
- [ ] Basic approval UI
- [ ] Integration with read_file tool
- [ ] Unit tests
- [ ] Documentation

### Milestone 2: Production Ready (6-8 weeks)
- [ ] Audit logging
- [ ] Configuration validation
- [ ] Access statistics
- [ ] Integration tests
- [ ] User testing
- [ ] Performance optimization

### Milestone 3: Advanced Features (8-12 weeks)
- [ ] Path aliases
- [ ] Temporary access grants
- [ ] Team sharing support
- [ ] Advanced UI/UX
- [ ] Comprehensive test coverage

---

## Success Metrics

### Adoption Metrics
- Number of users enabling external access
- Number of external paths configured per user
- Feature usage frequency

### Security Metrics
- Zero reported security incidents
- Average denied access requests
- Audit log coverage

### User Satisfaction
- Reduced symlink-related support requests
- Positive feedback on approval UX
- Feature request completion

---

## Alternatives Considered

### Alternative 1: No Feature, Use Symlinks
**Pros:** No code changes needed  
**Cons:** Requires admin rights on Windows; confusing for teams

### Alternative 2: Always-On with No Configuration
**Pros:** Simple to use  
**Cons:** Security nightmare; violates least-privilege principle

### Alternative 3: Temporary Directory Mounting
**Pros:** Clear mental model  
**Cons:** Complex implementation; doesn't match user workflows

### Alternative 4: AI-Only Approval (No Config)
**Pros:** Zero configuration  
**Cons:** Users approve without understanding; audit trail unclear

**Selected Approach:** Configuration + Approval provides best balance of security, flexibility, and usability.

---

## Open Questions

1. **Should approval persist across restarts?**
   - Proposal: Session-only by default, with opt-in persistence

2. **How to handle network paths (\\server\share)?**
   - Proposal: Support with explicit warning about network access

3. **Should we limit file size for external reads?**
   - Proposal: Yes, 10MB default limit with configuration option

4. **Integration with existing folder trust feature?**
   - Proposal: Separate but complementary features

---

## Conclusion

This feature provides a secure, auditable, and user-friendly way to access external files while maintaining the security guarantees of the sandbox model. By requiring explicit configuration and providing approval mechanisms, we balance flexibility with safety.

The phased implementation approach allows us to:
1. Validate core functionality quickly (MVP)
2. Gather user feedback early
3. Iterate on UX and security
4. Add advanced features based on real usage patterns

---

## References

- [Current Security Implementation](../src/acp/security.rs)
- [External File Reference Guide](EXTERNAL_FILE_REFERENCE.md)
- [Configuration Guide](CONFIGURATION.md)
- [Issue #XXX: External File Access Request](https://github.com/microtech/grok-cli/issues/XXX)

---

**Feedback & Discussion:** https://github.com/microtech/grok-cli/discussions/XXX

**Author:** john mcconnell (john.microtech@gmail.com)  
**Buy me a coffee:** https://buymeacoffee.com/micro.tech