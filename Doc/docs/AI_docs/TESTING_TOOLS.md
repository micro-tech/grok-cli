# Testing Tools for Grok CLI

## Overview

This document provides information on the testing tools and methodologies used to ensure the quality and reliability of Grok CLI. It is intended for developers and contributors who want to understand or participate in the testing process.

## Testing Framework

Grok CLI uses the built-in Rust testing framework for unit and integration tests. Tests are located in the `tests/` directory and within individual modules under `src/`.

### Running Tests

To run the test suite:

```bash
cargo test
```

This command executes all unit and integration tests. For more detailed output, use:

```bash
cargo test -- --nocapture
```

## Types of Tests

- **Unit Tests**: Located within each module (e.g., `src/api/mod.rs`), these test individual functions and components in isolation.
- **Integration Tests**: Located in `tests/`, these test the interaction between components and the overall CLI behavior.
- **End-to-End Tests**: Simulate real user interactions with the CLI to ensure the application works as expected.

## Writing Tests

When contributing new features or bug fixes, please include corresponding tests:

1. **Unit Tests**: Add tests directly in the module using the `#[cfg(test)]` attribute.
2. **Integration Tests**: Create a new file in the `tests/` directory to test interactions between modules.

Example of a simple unit test in a module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function() {
        assert_eq!(function_under_test(), expected_result);
    }
}
```

## Mocking API Responses

Since Grok CLI interacts with the X API, tests often use mocked responses to avoid real API calls during testing. We use the `mockito` crate for creating mock servers.

To run tests with mocks:

1. Ensure `mockito` is included in `Cargo.toml` under `[dev-dependencies]`.
2. Set up mock responses in integration tests as needed.

## Continuous Integration (CI)

Grok CLI uses GitHub Actions for CI to automatically run tests on every pull request and push to the main branch. The configuration is in `.github/workflows/`.

## Code Coverage

To measure test coverage, use `cargo-tarpaulin`:

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --ignore-tests
```

Aim for high coverage, especially for critical components like API interactions and configuration parsing.

## Debugging Tests

If a test fails, you can debug it by setting the `RUST_LOG` environment variable to see detailed logs:

```bash
RUST_LOG=debug cargo test -- --nocapture
```

## Contributing to Testing

We welcome contributions to improve test coverage or add new test cases. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on submitting pull requests.

For more information on the project setup, refer to [SETUP.md](SETUP.md).