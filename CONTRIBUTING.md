# Contributing to Grok CLI

## Welcome!

Thank you for your interest in contributing to Grok CLI, a powerful command-line interface for interacting with Grok AI. We value contributions from the community and are excited to collaborate with you to improve this project.

## How Can I Contribute?

There are many ways to contribute to Grok CLI, including:

- **Reporting Bugs**: If you find a bug, please open an issue on our [GitHub Issues page](https://github.com/microtech/grok-cli/issues) with detailed steps to reproduce it.
- **Suggesting Features**: Have an idea for a new feature or improvement? Share it in [GitHub Discussions](https://github.com/microtech/grok-cli/discussions).
- **Code Contributions**: Submit pull requests with bug fixes, new features, or enhancements.
- **Documentation**: Help improve our documentation by correcting typos, clarifying instructions, or adding examples.
- **Testing**: Test new features or bug fixes and provide feedback.

## Development Setup

To set up the project for development:

```bash
git clone https://github.com/microtech/grok-cli
cd grok-cli
cargo test
cargo clippy
```

Ensure you have Rust 1.70+ installed. For more setup details, refer to [SETUP.md](SETUP.md).

## Coding Guidelines

We aim to maintain a high-quality, consistent codebase:

- **Rust Style**: Follow the [Rust Style Guide](https://github.com/rust-dev-tools/fmt-rfcs/blob/master/guide/guide.md) and use `cargo fmt` for formatting.
- **Linting**: Run `cargo clippy` to catch potential issues.
- **Testing**: Add tests for new functionality and ensure existing tests pass with `cargo test`.
- **Documentation**: Document public APIs and add comments for complex logic.

## Pull Request Process

1. **Fork or Clone** the repository and create a branch for your changes.
2. **Make Changes** and commit them with descriptive messages following the [Conventional Commits](https://www.conventionalcommits.org/) format if possible.
3. **Test Your Changes** to ensure they work as expected.
4. **Submit a Pull Request** to the `main` branch with a clear description of your changes and reference any related issues.
5. **Code Review**: Address feedback from maintainers and make necessary revisions.

## Community

Join the conversation in our [GitHub Discussions](https://github.com/microtech/grok-cli/discussions) or reach out via email at john.microtech@gmail.com for direct support.

## License

By contributing to Grok CLI, you agree that your contributions will be licensed under the MIT License. See [LICENSE](LICENSE) for details.

Thank you for helping make Grok CLI better!