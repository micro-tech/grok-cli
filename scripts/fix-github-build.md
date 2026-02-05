# GitHub Actions Build Fix Guide

## Problem Summary

The GitHub Actions build is failing due to type mismatches with `MessageContent` from the `grok_api` crate (v0.1.1). The `grok_api` library changed the `Message.content` field from `Option<String>` to `Option<MessageContent>` where `MessageContent` is an enum that doesn't implement `Default` or `Display`.

## Error Examples

```
error[E0277]: the trait bound `MessageContent: std::default::Default` is not satisfied
error[E0277]: `MessageContent` doesn't implement `std::fmt::Display`
error[E0308]: mismatched types - expected `String`, found `MessageContent`
```

## Root Cause

The `grok_api` crate version 0.1.1 introduced a breaking change:
- **Old:** `content: Option<String>`
- **New:** `content: Option<MessageContent>`

Where `MessageContent` is defined as:
```rust
pub enum MessageContent {
    Text(String),
    MultiModal(Vec<Value>),
}
```

But it doesn't derive `Default` or `Display`, causing compilation errors.

## Solutions

### Option 1: Pin grok_api to 0.1.0 (Quick Fix)

**File:** `Cargo.toml`

```toml
[dependencies]
grok_api = "=0.1.0"  # Pin to exact version
```

Then run:
```bash
cargo clean
cargo update
cargo build
```

### Option 2: Implement Wrapper Type (Current Approach)

We've defined our own `MessageContent` in `src/lib.rs` with proper trait implementations:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    MultiModal(Vec<serde_json::Value>),
}

impl Default for MessageContent {
    fn default() -> Self {
        MessageContent::Text(String::new())
    }
}

impl std::fmt::Display for MessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageContent::Text(text) => write!(f, "{}", text),
            MessageContent::MultiModal(parts) => {
                let text: String = parts
                    .iter()
                    .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n");
                write!(f, "{}", text)
            }
        }
    }
}
```

**Helper functions provided:**
- `extract_text_content(&MessageContent) -> String`
- `content_to_string(Option<&MessageContent>) -> String`
- `text_content(impl Into<String>) -> MessageContent`

### Option 3: Fork grok_api (Long-term)

1. Fork the `grok_api` repository
2. Add trait implementations:
   ```rust
   #[derive(Default)]  // Add this
   pub enum MessageContent { ... }
   
   impl Display for MessageContent { ... }  // Add this
   ```
3. Update Cargo.toml to use forked version:
   ```toml
   [dependencies]
   grok_api = { git = "https://github.com/YOUR_USERNAME/grok-api-rust.git", branch = "main" }
   ```

### Option 4: Use Patch in Cargo.toml

If you have a local fix, use cargo's patch feature:

```toml
[patch.crates-io]
grok_api = { path = "../grok-api-rust" }
```

Or from a fork:
```toml
[patch.crates-io]
grok_api = { git = "https://github.com/YOUR_USERNAME/grok-api-rust.git" }
```

## Files That Need Updates

When fixing MessageContent issues, update these files:

1. **`src/lib.rs`**
   - Define MessageContent type or helpers
   - Export helper functions

2. **`src/grok_client_ext.rs`**
   - Fix `response.content()` usage
   - Change from `.map(|s| s.to_string())` to `.cloned()`

3. **`src/acp/mod.rs`**
   - Replace `content.unwrap_or_default()` with `content_to_string(content.as_ref())`
   - Add import: `use crate::{content_to_string, extract_text_content};`

4. **`src/cli/commands/chat.rs`**
   - Replace `&content` with `&extract_text_content(&content)`
   - Replace `content.unwrap_or_default()` with `content_to_string(content.as_ref())`
   - Add import: `use crate::{content_to_string, extract_text_content};`

5. **`src/display/interactive.rs`**
   - Replace string operations with helper functions
   - Add import: `use crate::{content_to_string, extract_text_content, text_content};`

## GitHub Actions Workflow Fix

Update `.github/workflows/ci.yml` and `.github/workflows/release.yml`:

### Add Dependency Caching

```yaml
- name: Cache cargo dependencies
  uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/bin/
      ~/.cargo/registry/index/
      ~/.cargo/registry/cache/
      ~/.cargo/git/db/
      target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      ${{ runner.os }}-cargo-
```

### Use Specific Rust Version

```yaml
- name: Install Rust
  uses: dtolnay/rust-toolchain@stable
  with:
    toolchain: stable
    components: rustfmt, clippy
```

### Build with Verbose Output

```yaml
- name: Build
  run: cargo build --release --verbose
  env:
    RUST_BACKTRACE: 1
```

## Testing Locally Before Push

```powershell
# Clean build to match CI environment
cargo clean

# Update dependencies
cargo update

# Check for errors
cargo check --all-targets

# Run clippy
cargo clippy --all-targets -- -D warnings

# Run tests
cargo test --all

# Build release
cargo build --release

# Test binary
./target/release/grok.exe --version
```

## Quick Fix Script

Save as `scripts/quick-fix.ps1`:

```powershell
#!/usr/bin/env pwsh
Write-Host "🔧 Quick Fix for GitHub Build Issues" -ForegroundColor Cyan

# Clean everything
Write-Host "→ Cleaning build artifacts..." -ForegroundColor Yellow
cargo clean

# Remove Cargo.lock to force fresh resolution
if (Test-Path "Cargo.lock") {
    Write-Host "→ Removing Cargo.lock..." -ForegroundColor Yellow
    Remove-Item "Cargo.lock"
}

# Update dependencies
Write-Host "→ Updating dependencies..." -ForegroundColor Yellow
cargo update

# Try to build
Write-Host "→ Building project..." -ForegroundColor Yellow
cargo build --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Build successful!" -ForegroundColor Green
} else {
    Write-Host "✗ Build failed. Check errors above." -ForegroundColor Red
    Write-Host ""
    Write-Host "Try these steps:" -ForegroundColor Yellow
    Write-Host "1. Pin grok_api to 0.1.0 in Cargo.toml" -ForegroundColor Yellow
    Write-Host "2. Run: cargo update -p grok_api" -ForegroundColor Yellow
    Write-Host "3. Run: cargo build" -ForegroundColor Yellow
    exit 1
}
```

## Environment Variables for CI

Add these to GitHub Actions secrets:

- `GROK_API_KEY` - Optional, for tests
- `RUST_BACKTRACE=1` - Better error messages
- `CARGO_TERM_COLOR=always` - Colored output

## Debugging CI Build

View full error logs:
```bash
# In GitHub Actions
cargo build --verbose 2>&1 | tee build.log

# Check specific package version
cargo tree | grep grok_api

# Show what changed
git diff Cargo.lock
```

## Contact

If issues persist:
1. Check grok_api changelog: https://crates.io/crates/grok_api
2. File issue: https://github.com/microtech/grok-cli/issues
3. Email: john.microtech@gmail.com

## Status

- **Last Updated:** January 2025
- **grok_api Version:** 0.1.1 (breaking changes)
- **Workaround:** Custom MessageContent wrapper with trait impls
- **Permanent Fix:** Pending grok_api update or fork

---

**TL;DR:** 
- The build fails because `grok_api` 0.1.1 changed API
- Quick fix: Pin `grok_api = "=0.1.0"` in Cargo.toml
- Better fix: Use our MessageContent wrapper (already in src/lib.rs)
- Long-term: Fork grok_api and add missing traits