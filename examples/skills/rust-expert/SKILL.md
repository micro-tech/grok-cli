---
name: rust-expert
description: Expert guidance for Rust development including best practices, error handling, ownership patterns, and idiomatic Rust code. Use when working on Rust projects or when the user asks Rust-specific questions.
license: MIT
compatibility: Designed for Rust 2021 edition and later
metadata:
  author: grok-cli
  version: "1.0"
  category: programming
---

# Rust Expert Skill

## Overview

This skill provides expert-level guidance for Rust development, covering best practices, common patterns, and idiomatic approaches to writing safe, efficient Rust code.

## Core Principles

1. **Ownership and Borrowing**: Always explain ownership rules clearly
2. **Zero-Cost Abstractions**: Prefer abstractions that compile to efficient code
3. **Safety First**: Leverage the type system to prevent bugs at compile time
4. **Explicit Over Implicit**: Be clear about behavior, especially with error handling

## Best Practices

### Error Handling

- Use `Result<T, E>` for recoverable errors
- Use `Option<T>` for nullable values
- Prefer `?` operator for error propagation
- Create custom error types using `thiserror` for libraries
- Use `anyhow` for applications where error types don't need to be exposed

### Naming Conventions

- Use `snake_case` for variables, functions, and modules
- Use `CamelCase` for types and traits
- Use `SCREAMING_SNAKE_CASE` for constants
- Use `r#` prefix for raw identifiers when needed

### Memory Management

- Prefer borrowing (`&T`, `&mut T`) over ownership transfer
- Use `Rc<T>` for shared ownership in single-threaded contexts
- Use `Arc<T>` for shared ownership in multi-threaded contexts
- Use `Box<T>` for heap allocation when size is unknown at compile time
- Consider `Cow<T>` for clone-on-write semantics

### Collections

- `Vec<T>` for dynamic arrays
- `HashMap<K, V>` for key-value storage (use `BTreeMap` for sorted keys)
- `HashSet<T>` for unique values (use `BTreeSet` for sorted values)
- Pre-allocate with `with_capacity()` when size is known

### Concurrency

- Use `std::thread` for OS threads
- Use `tokio` or `async-std` for async/await patterns
- Protect shared state with `Mutex<T>` or `RwLock<T>`
- Use channels (`mpsc`) for message passing between threads
- Consider `rayon` for data parallelism

### Testing

- Write unit tests in the same file using `#[cfg(test)]`
- Write integration tests in `tests/` directory
- Use `#[test]` attribute for test functions
- Use `assert!`, `assert_eq!`, and `assert_ne!` macros
- Use `#[should_panic]` for testing error conditions
- Use `cargo test` to run all tests

## Common Patterns

### Builder Pattern

```rust
pub struct Config {
    host: String,
    port: u16,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

pub struct ConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
}

impl ConfigBuilder {
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }
    
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }
    
    pub fn build(self) -> Result<Config> {
        Ok(Config {
            host: self.host.ok_or("host is required")?,
            port: self.port.unwrap_or(8080),
        })
    }
}
```

### Newtype Pattern

```rust
pub struct UserId(u64);
pub struct Email(String);
```

### RAII (Resource Acquisition Is Initialization)

Resources are automatically cleaned up when they go out of scope through `Drop` trait.

## Clippy Lints

Always run `cargo clippy` before committing. Common lints to address:

- `clippy::needless_return` - Remove explicit returns when not needed
- `clippy::redundant_clone` - Remove unnecessary clones
- `clippy::large_enum_variant` - Box large enum variants
- `clippy::implicit_clone` - Use explicit `.clone()` calls

## Performance Tips

- Use `&str` instead of `String` when possible
- Avoid unnecessary allocations
- Use iterators instead of collecting into vectors when possible
- Profile with `cargo flamegraph` or `perf`
- Use `#[inline]` judiciously for hot paths
- Consider `#[repr(C)]` for FFI types

## When to Use This Skill

Activate this skill when:
- Writing or reviewing Rust code
- Debugging ownership or lifetime errors
- Optimizing Rust performance
- Setting up Rust project structure
- Choosing appropriate data structures or patterns
- Working with async/await or concurrency

## Additional Resources

- The Rust Book: https://doc.rust-lang.org/book/
- Rust by Example: https://doc.rust-lang.org/rust-by-example/
- The Cargo Book: https://doc.rust-lang.org/cargo/
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/