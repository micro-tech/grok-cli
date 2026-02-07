# Error Handling Guide for grok-cli

## Overview

This guide documents the error handling patterns and best practices used throughout the grok-cli project. Following these patterns ensures consistency, maintainability, and robustness, especially when dealing with network instability on satellite connections like Starlink.

## Table of Contents

1. [Core Principles](#core-principles)
2. [Error Types](#error-types)
3. [Error Handling Patterns](#error-handling-patterns)
4. [Network Error Handling](#network-error-handling)
5. [File I/O Error Handling](#file-io-error-handling)
6. [Testing Error Cases](#testing-error-cases)
7. [Common Pitfalls](#common-pitfalls)

## Core Principles

### 1. Use Result<T, E> for Fallible Operations

Always return `Result` for operations that can fail:

```rust
use anyhow::Result;

pub fn load_config(path: &Path) -> Result<Config> {
    let contents = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}
```

### 2. Propagate Errors with Context

Add context to errors as they bubble up:

```rust
use anyhow::Context;

pub fn save_session(session: &Session, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(session)
        .context("Failed to serialize session to JSON")?;
    
    fs::write(path, json)
        .with_context(|| format!("Failed to write session to {:?}", path))?;
    
    Ok(())
}
```

### 3. Never Unwrap in Production Code

Use `?` operator, `unwrap_or`, `unwrap_or_else`, or `expect` with descriptive messages:

```rust
// ❌ BAD - Can panic in production
let value = result.unwrap();

// ✅ GOOD - Propagates error
let value = result?;

// ✅ GOOD - Provides fallback
let value = result.unwrap_or_default();

// ✅ GOOD - Only for truly impossible cases
let value = mutex.lock()
    .expect("Mutex poisoned - this is a critical bug");
```

### 4. Log Errors Appropriately

Use structured logging to track errors:

```rust
use tracing::{error, warn, debug};

// Critical errors that prevent operation
error!("Failed to connect to API: {}", e);

// Recoverable issues
warn!("Failed to save cache, continuing without cache: {}", e);

// Diagnostic information
debug!("Retrying request after error: {}", e);
```

## Error Types

### Custom Error Types with thiserror

Define custom error types for domain-specific errors:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrokApiError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Authentication failed: Invalid API key")]
    Authentication,

    #[error("Rate limit exceeded. Please try again later")]
    RateLimit,

    #[error("Model not found: {model}")]
    ModelNotFound { model: String },

    #[error("Request timeout after {timeout_secs} seconds")]
    Timeout { timeout_secs: u64 },

    #[error("Connection dropped - Network instability detected")]
    NetworkDrop,

    #[error("Maximum retries ({max_retries}) exceeded")]
    MaxRetriesExceeded { max_retries: u32 },
}
```

### Using anyhow for Application Errors

Use `anyhow::Result` for application-level code where specific error types aren't needed:

```rust
use anyhow::{Result, anyhow};

pub fn validate_input(input: &str) -> Result<()> {
    if input.is_empty() {
        return Err(anyhow!("Input cannot be empty"));
    }
    Ok(())
}
```

## Error Handling Patterns

### Pattern 1: Retry with Exponential Backoff

For network operations that may fail temporarily:

```rust
pub async fn execute_with_retry<T, F, Fut>(
    &self,
    request_fn: F,
    max_retries: u32,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 1..=max_retries {
        debug!("Attempt {} of {}", attempt, max_retries);

        match request_fn().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                warn!("Attempt {} failed: {}", attempt, e);
                last_error = Some(e);

                if attempt < max_retries && should_retry(&e) {
                    let backoff = calculate_backoff(attempt);
                    debug!("Backing off for {:?}", backoff);
                    tokio::time::sleep(backoff).await;
                    continue;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("All retries failed")))
}

fn calculate_backoff(attempt: u32) -> Duration {
    let base_delay = 2_u64.pow(attempt - 1);
    let max_delay = 60;
    let jitter = rand::random::<u64>() % 1000;
    
    Duration::from_secs(base_delay.min(max_delay)) 
        + Duration::from_millis(jitter)
}
```

### Pattern 2: Graceful Degradation

Continue operation with reduced functionality when non-critical components fail:

```rust
pub async fn initialize(&mut self) -> Result<()> {
    // Critical: Must succeed
    self.load_config().await
        .context("Failed to load configuration")?;
    
    // Non-critical: Log and continue
    if let Err(e) = self.load_cache().await {
        warn!("Failed to load cache: {}. Continuing without cache.", e);
    }
    
    // Non-critical: Silent failure with fallback
    self.stats = UsageStats::load().unwrap_or_default();
    
    Ok(())
}
```

### Pattern 3: Resource Cleanup with Drop

Ensure resources are cleaned up even on error:

```rust
pub struct Session {
    id: String,
    // Drop automatically closes and saves
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Err(e) = self.save() {
            error!("Failed to save session {}: {}", self.id, e);
        }
    }
}
```

### Pattern 4: Error Conversion

Convert between error types explicitly:

```rust
// Automatic conversion with From trait
impl From<std::io::Error> for MyError {
    fn from(err: std::io::Error) -> Self {
        MyError::Io(err)
    }
}

// Using in code
let file = File::open(path)?; // io::Error -> MyError automatically
```

## Network Error Handling

### Detecting Network Drops

The project includes Starlink-specific network drop detection:

```rust
pub fn detect_network_drop(error: &Error) -> bool {
    let error_string = error.to_string().to_lowercase();

    // Check for connection issues
    error_string.contains("connection reset") ||
    error_string.contains("broken pipe") ||
    error_string.contains("network unreachable") ||
    error_string.contains("timeout") ||
    
    // Check for HTTP gateway errors
    error_string.contains("502") ||
    error_string.contains("503") ||
    error_string.contains("504") ||
    error_string.contains("520") ||
    error_string.contains("521") ||
    error_string.contains("522") ||
    error_string.contains("523") ||
    error_string.contains("524")
}
```

### HTTP Client Configuration

Configure reqwest client with appropriate timeouts for satellite connections:

```rust
let client = ClientBuilder::new()
    .timeout(Duration::from_secs(30))        // Total request timeout
    .connect_timeout(Duration::from_secs(10)) // Connection timeout
    .tcp_keepalive(Duration::from_secs(30))  // Keep connections alive
    .pool_idle_timeout(Duration::from_secs(90))
    .pool_max_idle_per_host(10)
    .build()?;
```

### Retry Strategy for Network Errors

Determine which errors should trigger retries:

```rust
fn should_retry(error: &Error) -> bool {
    let error_string = error.to_string().to_lowercase();

    // Network-related errors
    error_string.contains("connection") ||
    error_string.contains("timeout") ||
    error_string.contains("network") ||
    error_string.contains("dns") ||
    
    // Temporary server errors
    error_string.contains("502") ||
    error_string.contains("503") ||
    error_string.contains("504")
}
```

## File I/O Error Handling

### Safe File Writing

Always use atomic operations and error checking:

```rust
use std::io::Write;
use std::fs::OpenOptions;

pub fn save_to_file(path: &Path, content: &str) -> Result<()> {
    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {:?}", parent))?;
    }

    // Write atomically
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("Failed to open file {:?}", path))?;

    file.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write to file {:?}", path))?;

    file.sync_all()
        .with_context(|| "Failed to sync file to disk")?;

    Ok(())
}
```

### Reading Files with Validation

```rust
pub fn load_from_file(path: &Path) -> Result<Config> {
    // Check if file exists
    if !path.exists() {
        return Err(anyhow!("Configuration file not found: {:?}", path));
    }

    // Read and parse
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file {:?}", path))?;

    let config: Config = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse TOML in {:?}", path))?;

    // Validate
    config.validate()
        .context("Configuration validation failed")?;

    Ok(config)
}
```

### Handling Missing Home Directory

Always provide fallbacks for environment-dependent paths:

```rust
pub fn get_config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?;
    
    Ok(home.join(".grok"))
}

// Or with fallback
pub fn get_config_dir_with_fallback() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".grok")
}
```

## Testing Error Cases

### Unit Tests for Error Handling

Test both success and failure paths:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_temperature() {
        let config = Config {
            temperature: -1.0,  // Invalid
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_network_drop_detection() {
        assert!(detect_network_drop(&anyhow!("Connection reset by peer")));
        assert!(detect_network_drop(&anyhow!("HTTP 502 Bad Gateway")));
        assert!(!detect_network_drop(&anyhow!("Invalid API key")));
    }
}
```

### Integration Tests with Mocking

Use `mockito` for HTTP error simulation:

```rust
#[cfg(test)]
mod integration_tests {
    use mockito::{mock, server_url};

    #[tokio::test]
    async fn test_retry_on_timeout() {
        let _m = mock("POST", "/api/chat")
            .with_status(504)  // Gateway Timeout
            .expect_at_least(2) // Should retry
            .create();

        let client = GrokClient::new("test-key")
            .unwrap()
            .with_base_url(server_url());

        let result = client.chat_completion("test", None, 0.7, 100, "grok-2").await;
        
        // Should fail after retries
        assert!(result.is_err());
    }
}
```

## Common Pitfalls

### ❌ Pitfall 1: Silent Failures

```rust
// BAD: Error is ignored
let _ = save_to_file(&path, content);
```

```rust
// GOOD: Log the error
if let Err(e) = save_to_file(&path, content) {
    warn!("Failed to save to file: {}. Continuing without save.", e);
}
```

### ❌ Pitfall 2: Generic Error Messages

```rust
// BAD: No context
fs::read_to_string(path)?;
```

```rust
// GOOD: Descriptive error
fs::read_to_string(path)
    .with_context(|| format!("Failed to read config from {:?}", path))?;
```

### ❌ Pitfall 3: Unwrapping in Production

```rust
// BAD: Can panic
let config = load_config().unwrap();
```

```rust
// GOOD: Handle error gracefully
let config = load_config()
    .unwrap_or_else(|e| {
        warn!("Failed to load config: {}. Using defaults.", e);
        Config::default()
    });
```

### ❌ Pitfall 4: Not Retrying Network Errors

```rust
// BAD: Single attempt
let response = client.post(url).send().await?;
```

```rust
// GOOD: Retry with backoff
let response = execute_with_retry(|| async {
    client.post(url).send().await
}).await?;
```

### ❌ Pitfall 5: Mutex Unwrap Without Message

```rust
// BAD: Panic with no information
let data = mutex.lock().unwrap();
```

```rust
// GOOD: Descriptive message for debugging
let data = mutex.lock()
    .expect("Mutex poisoned - this indicates a panic in another thread");
```

## Checklist for New Code

When adding new error-prone code, verify:

- [ ] All fallible operations return `Result<T, E>`
- [ ] Errors have descriptive messages with context
- [ ] No `.unwrap()` calls in production code paths
- [ ] Network operations have retry logic
- [ ] File operations check for directory existence
- [ ] Errors are logged at appropriate levels
- [ ] Critical failures stop execution
- [ ] Non-critical failures degrade gracefully
- [ ] Tests cover both success and error paths
- [ ] Documentation mentions error conditions

## Resources

- [anyhow documentation](https://docs.rs/anyhow/)
- [thiserror documentation](https://docs.rs/thiserror/)
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- Project: `src/api/mod.rs` - Network error handling examples
- Project: `src/utils/chat_logger.rs` - File I/O error handling examples

---

**Last Updated:** 2025-01  
**Maintainers:** grok-cli development team  
**Questions:** Open an issue on GitHub